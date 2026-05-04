//! OAuth 2.0 Authorization Code flow with PKCE (S256).
//!
//! Owns the end-to-end orchestration: PKCE verifier/challenge, state
//! parameter, browser launch, loopback HTTP redirect handling, token
//! exchange, and refresh-token rotation. See `analysis_feature_oauth.md`
//! §3 for the per-driver state machine and gotchas (notably §3.5 on
//! 127.0.0.1 binding and §14 #11 on rejecting Node's `0.0.0.0`).
//!
//! Defaults follow JDBC/Python (analysis §9):
//! * `authorization_url` ⇒ `https://{server_host}/oauth/authorize`
//! * `token_url` ⇒ `https://{server_host}/oauth/token-request`
//! * `redirect_uri` ⇒ ephemeral `http://127.0.0.1:<port>/`
//!
//! Caching obeys the cross-driver convention: when
//! `client_store_temporary_credential` is enabled and a [`TokenCache`] is
//! provided, the access token is consulted first, then the refresh token
//! is exchanged, and only if both fall through do we drive the
//! interactive flow (analysis §3.2 state machine).

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STD;
use reqwest::header::{self, HeaderMap, HeaderValue};
use reqwest::{Method, StatusCode};
use serde::Deserialize;
use snafu::ResultExt;
use url::Url;

use super::browser;
use super::dpop::{self, DPoPKey};
use super::error::{
    DPoPNonceRequiredSnafu, EndpointUrlParseSnafu, IdpSnafu, MissingAccessTokenSnafu, OAuthError,
    RefreshTokenExchangeSnafu, TokenExchangeSnafu, TokenResponseDecodeSnafu,
};
use super::loopback_server::{self, RedirectResult};
use super::pkce;
use super::state;
use super::token;
use crate::config::rest_parameters::OAuthAuthorizationCodeConfig;
use crate::sensitive::SensitiveString;
use crate::token_cache::TokenCache;

/// What an OAuth flow returns to the caller (step 2.3 will hand this to
/// the Snowflake login-request).
#[derive(Debug)]
pub(crate) struct AcquiredOAuthToken {
    pub(crate) access_token: SensitiveString,
    pub(crate) refresh_token: Option<SensitiveString>,
    /// Present iff DPoP was negotiated. Carries the JWK JSON so step 2.3
    /// can reuse the same key when signing the Snowflake login-request
    /// (analysis §5.1).
    pub(crate) dpop_jwk_json: Option<String>,
    pub(crate) expires_in: Option<Duration>,
}

/// Closure-shaped browser launcher. Production callers go through
/// [`browser::open`] with [`browser::print_paste_instructions`] as the
/// fallback; tests can substitute a deterministic driver that pokes the
/// loopback directly.
type BrowserLaunchFn = Box<dyn FnOnce(Url, Url) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

fn default_browser_launch() -> BrowserLaunchFn {
    Box::new(|authorize_url, _redirect_uri| {
        Box::pin(async move {
            if let Err(e) = browser::open(&authorize_url) {
                tracing::warn!(error = %e, "Failed to launch system browser; printing paste fallback");
                browser::print_paste_instructions(&authorize_url);
            }
        })
    })
}

#[tracing::instrument(
    skip(client, config, token_cache),
    fields(server_url, username = %config.username),
)]
pub(crate) async fn acquire_authorization_code(
    client: &reqwest::Client,
    server_url: &str,
    config: &OAuthAuthorizationCodeConfig,
    token_cache: Option<&dyn TokenCache>,
) -> Result<AcquiredOAuthToken, OAuthError> {
    acquire_authorization_code_inner(
        client,
        server_url,
        config,
        token_cache,
        default_browser_launch(),
    )
    .await
}

async fn acquire_authorization_code_inner(
    client: &reqwest::Client,
    server_url: &str,
    config: &OAuthAuthorizationCodeConfig,
    token_cache: Option<&dyn TokenCache>,
    launch_browser: BrowserLaunchFn,
) -> Result<AcquiredOAuthToken, OAuthError> {
    tracing::info!("Starting OAuth authorization code flow");

    let token_url = resolve_token_url(server_url, config.token_url.as_ref())?;
    let cache_host_url = token_url.as_str();

    // 1. Cache short-circuit (analysis §3.2 + §7).
    if config.client_store_temporary_credential && token_cache.is_some() {
        if let Some(cached) =
            token::try_get_cached_oauth_access_token(cache_host_url, &config.username, token_cache)
        {
            tracing::info!("OAuth authorization code flow served from cache");
            return Ok(AcquiredOAuthToken {
                access_token: cached,
                refresh_token: None,
                dpop_jwk_json: None,
                expires_in: None,
            });
        }
        if let Some(refresh) =
            token::try_get_cached_oauth_refresh_token(cache_host_url, &config.username, token_cache)
        {
            match refresh_access_token(client, &token_url, config, refresh.reveal()).await {
                Ok(refreshed) => {
                    tracing::info!("Refreshed OAuth access token");
                    persist_tokens(config, cache_host_url, token_cache, &refreshed);
                    return Ok(refreshed);
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Cached OAuth refresh token failed to exchange; evicting and falling back to full flow"
                    );
                    token::remove_oauth_refresh_token(
                        cache_host_url,
                        &config.username,
                        token_cache,
                    );
                }
            }
        }
    }

    // 2. Interactive flow.
    let result =
        run_interactive_flow(client, server_url, &token_url, config, launch_browser).await?;

    persist_tokens(config, cache_host_url, token_cache, &result);
    tracing::info!("OAuth authorization code flow completed");
    Ok(result)
}

async fn run_interactive_flow(
    client: &reqwest::Client,
    server_url: &str,
    token_url: &Url,
    config: &OAuthAuthorizationCodeConfig,
    launch_browser: BrowserLaunchFn,
) -> Result<AcquiredOAuthToken, OAuthError> {
    let authorize_url_base = resolve_authorize_url(server_url, config.authorization_url.as_ref())?;
    let binding = loopback_server::bind(config.redirect_uri.as_ref()).await?;
    let redirect_uri = binding.redirect_uri.clone();
    let pkce_material = pkce::generate();
    let csrf = state::generate();

    let dpop_key = if config.enable_dpop {
        Some(dpop::DPoPKey::generate()?)
    } else {
        None
    };

    let authorize_url = build_authorize_url(
        &authorize_url_base,
        &config.client_id,
        &redirect_uri,
        &pkce_material,
        &csrf,
        config.scope.as_deref(),
        dpop_key.as_ref(),
    )?;

    tracing::debug!(
        authority = %authorize_url.authority(),
        path = %authorize_url.path(),
        "Built authorize URL for browser leg"
    );

    // Run the browser launcher concurrently with the redirect listener so
    // we don't lose the redirect if the browser launcher takes its time.
    let timeout = Duration::from_secs(config.authentication_timeout_secs);
    let listener = binding;

    let (redirect_result, _) = tokio::join!(
        listener.wait_for_redirect(timeout),
        launch_browser(authorize_url, redirect_uri.clone()),
    );
    let redirect: RedirectResult = redirect_result?;
    state::validate(&csrf, &redirect.state)?;

    let mut acquired = exchange_authorization_code(
        client,
        token_url,
        config,
        &redirect_uri,
        redirect.code.reveal(),
        pkce_material.verifier.reveal(),
        dpop_key.as_ref(),
    )
    .await?;

    if let Some(key) = dpop_key.as_ref() {
        acquired.dpop_jwk_json = Some(key.to_jwk_json()?);
    }

    Ok(acquired)
}

/// Persist access and refresh tokens (when caching is enabled and a
/// [`TokenCache`] is provided). DPoP-bundled cache writes go through the
/// dedicated `DpopBundledAccessToken` slot so the JWK survives across
/// process restarts (analysis §7.2).
fn persist_tokens(
    config: &OAuthAuthorizationCodeConfig,
    cache_host_url: &str,
    token_cache: Option<&dyn TokenCache>,
    acquired: &AcquiredOAuthToken,
) {
    if !config.client_store_temporary_credential || token_cache.is_none() {
        return;
    }

    if let Some(jwk_json) = acquired.dpop_jwk_json.as_deref() {
        token::store_oauth_dpop_bundled(
            cache_host_url,
            &config.username,
            acquired.access_token.reveal(),
            jwk_json,
            token_cache,
        );
    } else {
        token::store_oauth_access_token(
            cache_host_url,
            &config.username,
            acquired.access_token.reveal(),
            token_cache,
        );
    }

    if let Some(refresh) = acquired.refresh_token.as_ref() {
        token::store_oauth_refresh_token(
            cache_host_url,
            &config.username,
            refresh.reveal(),
            token_cache,
        );
    }
}

fn resolve_authorize_url(server_url: &str, override_url: Option<&Url>) -> Result<Url, OAuthError> {
    if let Some(url) = override_url {
        return Ok(url.clone());
    }
    let server = Url::parse(server_url).context(EndpointUrlParseSnafu {
        url: server_url.to_string(),
    })?;
    let host = server.host_str().unwrap_or("");
    let default = format!("https://{host}/oauth/authorize");
    Url::parse(&default).context(EndpointUrlParseSnafu { url: default })
}

fn resolve_token_url(server_url: &str, override_url: Option<&Url>) -> Result<Url, OAuthError> {
    if let Some(url) = override_url {
        return Ok(url.clone());
    }
    let server = Url::parse(server_url).context(EndpointUrlParseSnafu {
        url: server_url.to_string(),
    })?;
    let host = server.host_str().unwrap_or("");
    let default = format!("https://{host}/oauth/token-request");
    Url::parse(&default).context(EndpointUrlParseSnafu { url: default })
}

fn build_authorize_url(
    base: &Url,
    client_id: &str,
    redirect_uri: &Url,
    pkce_material: &pkce::PkceMaterial,
    csrf: &state::StateToken,
    scope: Option<&str>,
    dpop_key: Option<&DPoPKey>,
) -> Result<Url, OAuthError> {
    let mut url = base.clone();
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("response_type", "code");
        q.append_pair("client_id", client_id);
        q.append_pair("redirect_uri", redirect_uri.as_str());
        q.append_pair("state", csrf.expose());
        q.append_pair("code_challenge", &pkce_material.challenge);
        q.append_pair("code_challenge_method", pkce_material.method);
        if let Some(s) = scope {
            q.append_pair("scope", s);
        }
        if let Some(k) = dpop_key {
            q.append_pair("dpop_jkt", &dpop::jwk_thumbprint(k)?);
        }
    }
    Ok(url)
}

// ─── Token exchange ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TokenResponseBody {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

async fn exchange_authorization_code(
    client: &reqwest::Client,
    token_url: &Url,
    config: &OAuthAuthorizationCodeConfig,
    redirect_uri: &Url,
    code: &str,
    code_verifier: &str,
    dpop_key: Option<&DPoPKey>,
) -> Result<AcquiredOAuthToken, OAuthError> {
    let mut params: Vec<(&str, String)> = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", code.to_string()),
        ("redirect_uri", redirect_uri.as_str().to_string()),
        ("code_verifier", code_verifier.to_string()),
    ];
    if let Some(scope) = config.scope.as_deref() {
        params.push(("scope", scope.to_string()));
    }
    if config.enable_single_use_refresh_tokens {
        params.push(("enable_single_use_refresh_tokens", "true".to_string()));
    }

    let body = post_token_request(
        client,
        token_url,
        &config.client_id,
        config.client_secret.reveal(),
        &params,
        dpop_key,
    )
    .await?;

    let token_response = parse_success_body(&body, |status| TokenExchangeSnafu { status }.build())?;

    let access_token = token_response
        .access_token
        .filter(|s| !s.is_empty())
        .ok_or_else(|| MissingAccessTokenSnafu.build())?;
    Ok(AcquiredOAuthToken {
        access_token: SensitiveString::from(access_token),
        refresh_token: token_response
            .refresh_token
            .filter(|s| !s.is_empty())
            .map(SensitiveString::from),
        dpop_jwk_json: None,
        expires_in: token_response.expires_in.map(Duration::from_secs),
    })
}

async fn refresh_access_token(
    client: &reqwest::Client,
    token_url: &Url,
    config: &OAuthAuthorizationCodeConfig,
    refresh_token: &str,
) -> Result<AcquiredOAuthToken, OAuthError> {
    let mut params: Vec<(&str, String)> = vec![
        ("grant_type", "refresh_token".to_string()),
        ("refresh_token", refresh_token.to_string()),
    ];
    if let Some(scope) = config.scope.as_deref() {
        params.push(("scope", scope.to_string()));
    }

    let body = post_token_request(
        client,
        token_url,
        &config.client_id,
        config.client_secret.reveal(),
        &params,
        None,
    )
    .await
    .map_err(|e| match e {
        OAuthError::TokenExchange { .. } => RefreshTokenExchangeSnafu.build(),
        other => other,
    })?;

    let token_response =
        parse_success_body(&body, |_| RefreshTokenExchangeSnafu.build()).map_err(|e| match e {
            OAuthError::IdpError { .. } => RefreshTokenExchangeSnafu.build(),
            other => other,
        })?;

    let access_token = token_response
        .access_token
        .filter(|s| !s.is_empty())
        .ok_or_else(|| MissingAccessTokenSnafu.build())?;

    Ok(AcquiredOAuthToken {
        access_token: SensitiveString::from(access_token),
        refresh_token: token_response
            .refresh_token
            .filter(|s| !s.is_empty())
            .map(SensitiveString::from),
        dpop_jwk_json: None,
        expires_in: token_response.expires_in.map(Duration::from_secs),
    })
}

/// POST `application/x-www-form-urlencoded` to the token endpoint with
/// HTTP Basic auth. Performs at most one DPoP-nonce retry per RFC 9449 §8
/// when `dpop_key` is supplied.
pub(super) async fn post_token_request(
    client: &reqwest::Client,
    token_url: &Url,
    client_id: &str,
    client_secret: &str,
    params: &[(&str, String)],
    dpop_key: Option<&DPoPKey>,
) -> Result<String, OAuthError> {
    match send_token_request(
        client,
        token_url,
        client_id,
        client_secret,
        params,
        dpop_key,
        None,
    )
    .await
    {
        Ok(body) => Ok(body),
        Err((OAuthError::DPoPNonceRequired { .. }, Some(nonce))) => {
            tracing::info!("Retrying OAuth token request with DPoP nonce");
            send_token_request(
                client,
                token_url,
                client_id,
                client_secret,
                params,
                dpop_key,
                Some(nonce.as_str()),
            )
            .await
            .map_err(|(e, _)| e)
        }
        Err((e, _)) => Err(e),
    }
}

async fn send_token_request(
    client: &reqwest::Client,
    token_url: &Url,
    client_id: &str,
    client_secret: &str,
    params: &[(&str, String)],
    dpop_key: Option<&DPoPKey>,
    nonce: Option<&str>,
) -> Result<String, (OAuthError, Option<String>)> {
    let basic = format!("{client_id}:{client_secret}");
    let basic_b64 = BASE64_STD.encode(basic.as_bytes());
    let auth = format!("Basic {basic_b64}");

    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&auth).map_err(|e| {
            (
                OAuthError::Internal {
                    source: Box::new(e),
                    location: snafu::Location::new(file!(), line!(), column!()),
                },
                None,
            )
        })?,
    );
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));

    if let Some(key) = dpop_key {
        let proof =
            dpop::proof_jwt(key, Method::POST.as_str(), token_url, nonce).map_err(|e| (e, None))?;
        let value = HeaderValue::from_str(proof.reveal()).map_err(|e| {
            (
                OAuthError::Internal {
                    source: Box::new(e),
                    location: snafu::Location::new(file!(), line!(), column!()),
                },
                None,
            )
        })?;
        headers.insert("DPoP", value);
    }

    let body = form_urlencode(params);

    let resp = client
        .post(token_url.clone())
        .headers(headers)
        .body(body)
        .send()
        .await
        .map_err(|e| {
            (
                OAuthError::Transport {
                    source: e,
                    location: snafu::Location::new(file!(), line!(), column!()),
                },
                None,
            )
        })?;

    let status = resp.status();
    let response_headers = resp.headers().clone();
    let text = resp.text().await.map_err(|e| {
        (
            OAuthError::Transport {
                source: e,
                location: snafu::Location::new(file!(), line!(), column!()),
            },
            None,
        )
    })?;

    if status.is_success() {
        return Ok(text);
    }

    if dpop_key.is_some()
        && let Some(nonce) = dpop::check_use_dpop_nonce(&response_headers, &text)
    {
        return Err((DPoPNonceRequiredSnafu.build(), Some(nonce)));
    }

    // Surface a structured IdP error when the body parses as one;
    // otherwise fall back to the generic HTTP-status error.
    match try_extract_error_body(&text) {
        Some((error, description)) => Err((IdpSnafu { error, description }.build(), None)),
        None => Err((TokenExchangeSnafu { status }.build(), None)),
    }
}

fn try_extract_error_body(text: &str) -> Option<(String, String)> {
    let parsed: serde_json::Value = serde_json::from_str(text).ok()?;
    let error = parsed.get("error").and_then(|v| v.as_str())?.to_string();
    let description = parsed
        .get("error_description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Some((error, description))
}

fn parse_success_body<F>(text: &str, _err: F) -> Result<TokenResponseBody, OAuthError>
where
    F: Fn(StatusCode) -> OAuthError,
{
    serde_json::from_str::<TokenResponseBody>(text)
        .context(TokenResponseDecodeSnafu)
        .and_then(|tr| {
            if let Some(error) = tr.error.as_deref() {
                IdpSnafu {
                    error: error.to_string(),
                    description: tr.error_description.clone().unwrap_or_default(),
                }
                .fail()
            } else {
                Ok(tr)
            }
        })
}

fn form_urlencode(params: &[(&str, String)]) -> String {
    let mut out = String::new();
    for (i, (k, v)) in params.iter().enumerate() {
        if i > 0 {
            out.push('&');
        }
        out.push_str(&urlencoding::encode(k));
        out.push('=');
        out.push_str(&urlencoding::encode(v));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rest_parameters::DEFAULT_AUTHENTICATION_TIMEOUT_SECS;
    use crate::token_cache::TokenCacheError;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct StubTokenCache {
        store: Mutex<HashMap<String, String>>,
    }
    impl StubTokenCache {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
            }
        }
        fn key(host: &str, username: &str, token_type: crate::token_cache::TokenType) -> String {
            format!("{host};{username};{}", token_type.as_str())
        }
    }
    impl crate::token_cache::TokenCache for StubTokenCache {
        fn add_token(
            &self,
            host: &str,
            username: &str,
            token_type: crate::token_cache::TokenType,
            token_value: &str,
        ) -> Result<(), TokenCacheError> {
            self.store.lock().unwrap().insert(
                Self::key(host, username, token_type),
                token_value.to_string(),
            );
            Ok(())
        }
        fn remove_token(
            &self,
            host: &str,
            username: &str,
            token_type: crate::token_cache::TokenType,
        ) -> Result<(), TokenCacheError> {
            self.store
                .lock()
                .unwrap()
                .remove(&Self::key(host, username, token_type));
            Ok(())
        }
        fn get_token(
            &self,
            host: &str,
            username: &str,
            token_type: crate::token_cache::TokenType,
        ) -> Result<Option<String>, TokenCacheError> {
            Ok(self
                .store
                .lock()
                .unwrap()
                .get(&Self::key(host, username, token_type))
                .cloned())
        }
    }

    fn cfg_with_token_url(token_url: Url) -> OAuthAuthorizationCodeConfig {
        OAuthAuthorizationCodeConfig {
            username: "alice".to_string(),
            client_id: "cid".to_string(),
            client_secret: "shh".into(),
            authorization_url: None,
            token_url: Some(token_url),
            redirect_uri: None,
            scope: None,
            enable_single_use_refresh_tokens: false,
            disable_pkce: false,
            enable_dpop: false,
            client_store_temporary_credential: true,
            authentication_timeout_secs: DEFAULT_AUTHENTICATION_TIMEOUT_SECS,
        }
    }

    #[test]
    fn build_authorize_url_includes_required_query_params() {
        let base = Url::parse("https://idp.example.com/oauth/authorize").unwrap();
        let redirect = Url::parse("http://127.0.0.1:1234/").unwrap();
        let pkce = pkce::generate();
        let csrf = state::generate();

        let url = build_authorize_url(
            &base,
            "client-x",
            &redirect,
            &pkce,
            &csrf,
            Some("session:role:DEV"),
            None,
        )
        .unwrap();

        let q: HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(q.get("response_type").map(|s| s.as_str()), Some("code"));
        assert_eq!(q.get("client_id").map(|s| s.as_str()), Some("client-x"));
        assert_eq!(
            q.get("redirect_uri").map(|s| s.as_str()),
            Some("http://127.0.0.1:1234/")
        );
        assert!(q.contains_key("state"));
        assert!(q.contains_key("code_challenge"));
        assert_eq!(
            q.get("code_challenge_method").map(|s| s.as_str()),
            Some("S256")
        );
        assert_eq!(q.get("scope").map(|s| s.as_str()), Some("session:role:DEV"));
        assert!(!q.contains_key("dpop_jkt"));
    }

    #[test]
    fn build_authorize_url_with_dpop_includes_thumbprint() {
        let base = Url::parse("https://idp.example.com/oauth/authorize").unwrap();
        let redirect = Url::parse("http://127.0.0.1:9000/").unwrap();
        let pkce = pkce::generate();
        let csrf = state::generate();
        let key = DPoPKey::generate().unwrap();
        let url =
            build_authorize_url(&base, "cid", &redirect, &pkce, &csrf, None, Some(&key)).unwrap();
        let q: HashMap<_, _> = url.query_pairs().into_owned().collect();
        let jkt = q.get("dpop_jkt").expect("dpop_jkt present");
        assert!(!jkt.is_empty());
        assert_eq!(*jkt, dpop::jwk_thumbprint(&key).unwrap());
    }

    #[test]
    fn resolve_default_urls_use_server_host() {
        let auth = resolve_authorize_url("https://acct.snowflakecomputing.com", None).unwrap();
        assert_eq!(
            auth.as_str(),
            "https://acct.snowflakecomputing.com/oauth/authorize"
        );
        let tok = resolve_token_url("https://acct.snowflakecomputing.com", None).unwrap();
        assert_eq!(
            tok.as_str(),
            "https://acct.snowflakecomputing.com/oauth/token-request"
        );
    }

    #[tokio::test]
    async fn cached_access_token_short_circuits_full_flow() {
        let cache = StubTokenCache::new();
        let token_url = Url::parse("https://idp.example.com/oauth/token").unwrap();
        token::store_oauth_access_token(token_url.as_str(), "alice", "CACHED-AT", Some(&cache));

        let config = cfg_with_token_url(token_url);
        let client = reqwest::Client::new();
        let acquired = acquire_authorization_code(
            &client,
            "https://acct.snowflakecomputing.com",
            &config,
            Some(&cache),
        )
        .await
        .expect("cache hit");
        assert_eq!(acquired.access_token.reveal(), "CACHED-AT");
        assert!(acquired.refresh_token.is_none());
        assert!(acquired.dpop_jwk_json.is_none());
    }

    #[tokio::test]
    async fn cached_refresh_token_is_exchanged_for_fresh_access_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(header("content-type", "application/x-www-form-urlencoded"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=RT-OLD"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(
                    r#"{"access_token":"AT-NEW","refresh_token":"RT-NEW","token_type":"Bearer","expires_in":3600}"#,
                ),
            )
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
        let cache = StubTokenCache::new();
        token::store_oauth_refresh_token(token_url.as_str(), "alice", "RT-OLD", Some(&cache));
        let config = cfg_with_token_url(token_url.clone());

        let client = reqwest::Client::new();
        let acquired = acquire_authorization_code(
            &client,
            "https://acct.snowflakecomputing.com",
            &config,
            Some(&cache),
        )
        .await
        .expect("refresh succeeds");

        assert_eq!(acquired.access_token.reveal(), "AT-NEW");
        assert_eq!(
            acquired.refresh_token.as_ref().map(|s| s.reveal().as_str()),
            Some("RT-NEW")
        );
        assert_eq!(acquired.expires_in, Some(Duration::from_secs(3600)));

        // The rotated tokens should be persisted.
        let stored_at =
            token::try_get_cached_oauth_access_token(token_url.as_str(), "alice", Some(&cache))
                .map(|s| s.reveal().to_string());
        assert_eq!(stored_at.as_deref(), Some("AT-NEW"));
    }

    #[tokio::test]
    async fn refresh_token_idp_failure_evicts_cache_and_propagates() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(400).set_body_string(
                r#"{"error":"invalid_grant","error_description":"refresh token expired"}"#,
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
        let cache = StubTokenCache::new();
        token::store_oauth_refresh_token(token_url.as_str(), "alice", "RT-OLD", Some(&cache));
        let config = cfg_with_token_url(token_url.clone());

        // Stub a no-op browser launcher so the test fails fast without real
        // browser interaction; the call into the interactive leg will then
        // surface BrowserTimeout when no redirect ever arrives.
        let launch: BrowserLaunchFn = Box::new(|_, _| Box::pin(async {}));
        let mut config_short =
            cfg_with_token_url(Url::parse(&format!("{}/oauth/token", server.uri())).unwrap());
        config_short.authentication_timeout_secs = 1;
        let client = reqwest::Client::new();
        let result = acquire_authorization_code_inner(
            &client,
            "https://acct.snowflakecomputing.com",
            &config_short,
            Some(&cache),
            launch,
        )
        .await;
        // We expect either a refresh exchange failure to fall through and then
        // hit BrowserTimeout (no redirect ever received), depending on timing.
        // The important assertion is that the refresh token was evicted.
        assert!(result.is_err());
        let stored_rt =
            token::try_get_cached_oauth_refresh_token(token_url.as_str(), "alice", Some(&cache));
        assert!(
            stored_rt.is_none(),
            "expired refresh token must be evicted from the cache"
        );
        // Suppress unused-variable warning about the dropped config.
        let _ = config;
    }

    #[tokio::test]
    async fn full_interactive_flow_drives_loopback_directly() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("grant_type=authorization_code"))
            .and(body_string_contains("code=THE-CODE"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"access_token":"AT-FRESH","refresh_token":"RT-FRESH","token_type":"Bearer","expires_in":600}"#,
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
        let mut config = cfg_with_token_url(token_url);
        config.authorization_url =
            Some(Url::parse("https://idp.example.com/oauth/authorize").unwrap());
        // Don't cache anything for this test to force the interactive branch.
        config.client_store_temporary_credential = false;

        let launch: BrowserLaunchFn = Box::new(|authorize_url, redirect_uri| {
            Box::pin(async move {
                let state = authorize_url
                    .query_pairs()
                    .find(|(k, _)| k == "state")
                    .map(|(_, v)| v.into_owned())
                    .unwrap_or_default();
                let mut s = tokio::net::TcpStream::connect((
                    redirect_uri.host_str().unwrap(),
                    redirect_uri.port().unwrap(),
                ))
                .await
                .expect("connect loopback");
                use tokio::io::AsyncWriteExt;
                let req = format!(
                    "GET /?code=THE-CODE&state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
                );
                let _ = s.write_all(req.as_bytes()).await;
            })
        });

        let client = reqwest::Client::new();
        let acquired = acquire_authorization_code_inner(
            &client,
            "https://acct.snowflakecomputing.com",
            &config,
            None,
            launch,
        )
        .await
        .expect("interactive flow succeeds");
        assert_eq!(acquired.access_token.reveal(), "AT-FRESH");
        assert_eq!(
            acquired.refresh_token.as_ref().map(|s| s.reveal().as_str()),
            Some("RT-FRESH")
        );
    }

    #[tokio::test]
    async fn idp_error_in_authorize_redirect_surfaces_idp_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .mount(&server)
            .await;
        let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
        let mut config = cfg_with_token_url(token_url);
        config.authorization_url =
            Some(Url::parse("https://idp.example.com/oauth/authorize").unwrap());
        config.client_store_temporary_credential = false;

        let launch: BrowserLaunchFn = Box::new(|_authorize_url, redirect_uri| {
            Box::pin(async move {
                let mut s = tokio::net::TcpStream::connect((
                    redirect_uri.host_str().unwrap(),
                    redirect_uri.port().unwrap(),
                ))
                .await
                .expect("connect loopback");
                use tokio::io::AsyncWriteExt;
                let req = "GET /?error=access_denied&error_description=denied HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
                let _ = s.write_all(req.as_bytes()).await;
            })
        });
        let client = reqwest::Client::new();
        let err = acquire_authorization_code_inner(
            &client,
            "https://acct.snowflakecomputing.com",
            &config,
            None,
            launch,
        )
        .await
        .expect_err("must fail with IdpError");
        assert!(matches!(err, OAuthError::IdpError { .. }), "got {err:?}");
    }

    #[tokio::test]
    async fn state_mismatch_in_redirect_surfaces_canonical_xss_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .mount(&server)
            .await;
        let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
        let mut config = cfg_with_token_url(token_url);
        config.authorization_url =
            Some(Url::parse("https://idp.example.com/oauth/authorize").unwrap());
        config.client_store_temporary_credential = false;

        let launch: BrowserLaunchFn = Box::new(|_authorize_url, redirect_uri| {
            Box::pin(async move {
                let mut s = tokio::net::TcpStream::connect((
                    redirect_uri.host_str().unwrap(),
                    redirect_uri.port().unwrap(),
                ))
                .await
                .expect("connect loopback");
                use tokio::io::AsyncWriteExt;
                let req = "GET /?code=THE-CODE&state=ATTACKER-SUPPLIED HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
                let _ = s.write_all(req.as_bytes()).await;
            })
        });
        let client = reqwest::Client::new();
        let err = acquire_authorization_code_inner(
            &client,
            "https://acct.snowflakecomputing.com",
            &config,
            None,
            launch,
        )
        .await
        .expect_err("must fail with state mismatch");
        let msg = err.to_string();
        assert!(
            msg.contains("It might indicate an XSS attack."),
            "unexpected error: {msg}"
        );
        assert!(matches!(err, OAuthError::StateMismatch { .. }));
    }

    #[test]
    fn form_urlencode_encodes_special_characters() {
        let params = vec![
            ("grant_type", "refresh_token".to_string()),
            ("refresh_token", "abc=def&ghi".to_string()),
        ];
        let encoded = form_urlencode(&params);
        assert!(encoded.contains("grant_type=refresh_token"));
        assert!(encoded.contains("refresh_token=abc%3Ddef%26ghi"));
    }
}
