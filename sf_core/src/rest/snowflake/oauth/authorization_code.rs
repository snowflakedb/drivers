//! OAuth 2.0 Authorization Code flow with PKCE (S256).
//!
//! Owns the end-to-end orchestration: PKCE verifier/challenge, CSRF state,
//! browser launch, loopback HTTP redirect handling, token exchange, and
//! refresh-token rotation. See `analysis_feature_oauth.md` §3 for the
//! per-driver state machine and gotchas (notably §3.5 on 127.0.0.1
//! binding and §14 #11 on rejecting Node's `0.0.0.0`).
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
//!
//! HTTP transport, body encoding, CSRF generation, and PKCE generation
//! are owned by the `oauth2` crate; we only orchestrate the moving parts
//! via [`OAuthHttpClient`], [`webbrowser`], and [`loopback_server`].

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use oauth2::basic::{BasicClient, BasicErrorResponse, BasicErrorResponseType};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, RequestTokenError, Scope,
    StandardErrorResponse, TokenResponse, TokenUrl,
};
use snafu::{IntoError, ResultExt};
use url::Url;

use super::dpop::{self, DPoPKey};
use super::error::{
    EndpointUrlParseSnafu, IdpSnafu, MissingAccessTokenSnafu, OAuthError,
    RefreshTokenExchangeSnafu, StateMismatchSnafu, TokenResponseDecodeSnafu,
};
use super::http_client::{DPoPContext, OAuthHttpClient};
use super::loopback_server::{self, RedirectResult};
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
/// [`webbrowser::open`] with a stderr paste fallback; tests can
/// substitute a deterministic driver that pokes the loopback directly.
type BrowserLaunchFn = Box<dyn FnOnce(Url, Url) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

fn default_browser_launch() -> BrowserLaunchFn {
    Box::new(|authorize_url, _redirect_uri| {
        Box::pin(async move {
            tracing::debug!(
                authority = %authorize_url.authority(),
                path = %authorize_url.path(),
                "Opening system browser for OAuth authorization"
            );
            if let Err(e) = webbrowser::open(authorize_url.as_str()) {
                tracing::warn!(error = %e, "Failed to launch system browser; printing paste fallback");
                eprintln!("Open this URL in your browser to continue: {authorize_url}");
            }
        })
    })
}

#[tracing::instrument(
    skip(client, config, token_cache),
    fields(server_url = %server_url, username = %config.username),
)]
pub(crate) async fn acquire_authorization_code(
    // TODO(SNOW-XXX): build a no-redirect sibling reqwest client for OAuth
    // token calls (see https://docs.rs/oauth2/5.0.0/oauth2/#security-warning).
    client: &reqwest::Client,
    server_url: &Url,
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

#[tracing::instrument(skip(client, config, token_cache, launch_browser), fields(username = %config.username))]
async fn acquire_authorization_code_inner(
    client: &reqwest::Client,
    server_url: &Url,
    config: &OAuthAuthorizationCodeConfig,
    token_cache: Option<&dyn TokenCache>,
    launch_browser: BrowserLaunchFn,
) -> Result<AcquiredOAuthToken, OAuthError> {
    tracing::info!("Starting OAuth authorization code flow");

    let token_url = resolve_token_url(server_url, config.token_url.as_ref())?;
    let cache_host_url = token_url.as_str();

    // 1. Cache short-circuit (analysis §3.2 + §7).
    if let Some(cached) =
        try_cache_short_circuit(client, &token_url, cache_host_url, config, token_cache).await
    {
        return Ok(cached);
    }

    // 2. Interactive flow.
    let result =
        run_interactive_flow(client, server_url, &token_url, config, launch_browser).await?;

    persist_access_token(config, cache_host_url, token_cache, &result);
    persist_refresh_token(config, cache_host_url, token_cache, &result);
    tracing::info!("OAuth authorization code flow completed");
    Ok(result)
}

#[tracing::instrument(skip(client, token_url, config, token_cache), fields(username = %config.username))]
async fn try_cache_short_circuit(
    client: &reqwest::Client,
    token_url: &Url,
    cache_host_url: &str,
    config: &OAuthAuthorizationCodeConfig,
    token_cache: Option<&dyn TokenCache>,
) -> Option<AcquiredOAuthToken> {
    if !config.client_store_temporary_credential {
        tracing::debug!("OAuth token cache disabled; skipping cache lookup");
        return None;
    }
    let Some(_cache) = token_cache else {
        tracing::debug!("No OAuth token cache available; skipping cache lookup");
        return None;
    };

    if let Some(cached) =
        token::try_get_cached_oauth_access_token(cache_host_url, &config.username, token_cache)
    {
        tracing::debug!("OAuth access token served from cache");
        return Some(AcquiredOAuthToken {
            access_token: cached,
            refresh_token: None,
            dpop_jwk_json: None,
            expires_in: None,
        });
    }

    if let Some(refresh) =
        token::try_get_cached_oauth_refresh_token(cache_host_url, &config.username, token_cache)
    {
        tracing::debug!("Cache short-circuit hit on OAuth refresh token; attempting exchange");
        match refresh_access_token(client, token_url, config, refresh.reveal()).await {
            Ok(refreshed) => {
                tracing::info!("Refreshed OAuth access token");
                persist_access_token(config, cache_host_url, token_cache, &refreshed);
                persist_refresh_token(config, cache_host_url, token_cache, &refreshed);
                return Some(refreshed);
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Cached OAuth refresh token failed to exchange; evicting and falling back to full flow"
                );
                token::remove_oauth_refresh_token(cache_host_url, &config.username, token_cache);
            }
        }
    } else {
        tracing::debug!("Cache short-circuit missed; no OAuth refresh token cached");
    }

    None
}

#[tracing::instrument(skip(client, server_url, token_url, config, launch_browser), fields(username = %config.username))]
async fn run_interactive_flow(
    client: &reqwest::Client,
    server_url: &Url,
    token_url: &Url,
    config: &OAuthAuthorizationCodeConfig,
    launch_browser: BrowserLaunchFn,
) -> Result<AcquiredOAuthToken, OAuthError> {
    let authorize_url_base = resolve_authorize_url(server_url, config.authorization_url.as_ref())?;
    let binding = loopback_server::bind(config.redirect_uri.as_ref()).await?;
    let redirect_uri = binding.redirect_uri.clone();

    let dpop_key = if config.enable_dpop {
        Some(dpop::DPoPKey::generate()?)
    } else {
        None
    };
    let is_dpop_enabled = dpop_key.is_some();

    let oauth_client = build_oauth_client(
        config,
        authorize_url_base.clone(),
        token_url.clone(),
        redirect_uri.clone(),
    )?;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let mut request = oauth_client
        .authorize_url(CsrfToken::new_random)
        .set_pkce_challenge(pkce_challenge);
    if let Some(scope) = config.scope.as_deref() {
        request = request.add_scope(Scope::new(scope.to_string()));
    }
    if is_dpop_enabled {
        let thumbprint = dpop::jwk_thumbprint(dpop_key.as_ref().expect("dpop_key checked above"))?;
        request = request.add_extra_param("dpop_jkt", thumbprint);
    }
    let (authorize_url, state) = request.url();

    tracing::debug!(
        authority = %authorize_url.authority(),
        path = %authorize_url.path(),
        "Built OAuth authorization URL"
    );

    // Run the browser launcher concurrently with the redirect listener so
    // we don't lose the redirect if the browser launcher takes its time.
    let timeout = Duration::from_secs(config.authentication_timeout_secs);
    let listener = binding;

    let (redirect_result, _) = tokio::join!(
        listener.wait_for_redirect(timeout),
        launch_browser(authorize_url, redirect_uri),
    );
    let redirect: RedirectResult = redirect_result?;

    // CSRF check via timing-safe equality on `oauth2::CsrfToken`.
    let received = CsrfToken::new(redirect.state.clone());
    snafu::ensure!(received == state, StateMismatchSnafu);

    let http = make_http_client(client, dpop_key.as_ref(), token_url);

    let mut exchange = oauth_client
        .exchange_code(AuthorizationCode::new(redirect.code.reveal().to_string()))
        .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.into_secret()));
    if config.enable_single_use_refresh_tokens {
        exchange = exchange.add_extra_param("enable_single_use_refresh_tokens", "true");
    }

    tracing::debug!("Starting OAuth code exchange");
    let response = exchange
        .request_async(&http)
        .await
        .map_err(map_request_token_error)?;

    let access_token = SensitiveString::from(response.access_token().secret().clone());
    if access_token.reveal().is_empty() {
        return MissingAccessTokenSnafu.fail();
    }
    let refresh_token = response
        .refresh_token()
        .map(|rt| rt.secret().clone())
        .filter(|s| !s.is_empty())
        .map(SensitiveString::from);
    let expires_in = response.expires_in();

    let dpop_jwk_json = match dpop_key.as_ref() {
        Some(k) => Some(k.to_jwk_json()?),
        None => None,
    };

    Ok(AcquiredOAuthToken {
        access_token,
        refresh_token,
        dpop_jwk_json,
        expires_in,
    })
}

async fn refresh_access_token(
    client: &reqwest::Client,
    token_url: &Url,
    config: &OAuthAuthorizationCodeConfig,
    refresh_token: &str,
) -> Result<AcquiredOAuthToken, OAuthError> {
    tracing::debug!("Refreshing OAuth access token");
    // For the refresh leg we don't need authorize_url; reuse the same
    // token_url for both endpoints to satisfy the typestate bounds.
    let oauth_client = build_oauth_client(config, token_url.clone(), token_url.clone(), {
        // Refresh leg has no redirect URI requirement. Re-use the token
        // URL as a placeholder; `exchange_refresh_token` never reads it.
        token_url.clone()
    })?;

    let http = make_http_client(client, None, token_url);

    let refresh = RefreshToken::new(refresh_token.to_string());
    let mut request = oauth_client.exchange_refresh_token(&refresh);
    if let Some(scope) = config.scope.as_deref() {
        request = request.add_scope(Scope::new(scope.to_string()));
    }

    let response = request
        .request_async(&http)
        .await
        .map_err(map_refresh_token_error)?;

    let access_token = SensitiveString::from(response.access_token().secret().clone());
    if access_token.reveal().is_empty() {
        return MissingAccessTokenSnafu.fail();
    }
    let refresh_token = response
        .refresh_token()
        .map(|rt| rt.secret().clone())
        .filter(|s| !s.is_empty())
        .map(SensitiveString::from);

    Ok(AcquiredOAuthToken {
        access_token,
        refresh_token,
        dpop_jwk_json: None,
        expires_in: response.expires_in(),
    })
}

fn make_http_client<'a>(
    client: &'a reqwest::Client,
    dpop_key: Option<&'a DPoPKey>,
    token_url: &'a Url,
) -> OAuthHttpClient<'a> {
    let adapter = OAuthHttpClient::new(client);
    if let Some(key) = dpop_key {
        adapter.with_dpop(DPoPContext::new(key, token_url))
    } else {
        adapter
    }
}

fn build_oauth_client(
    config: &OAuthAuthorizationCodeConfig,
    authorize_url: Url,
    token_url: Url,
    redirect_uri: Url,
) -> Result<
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    OAuthError,
> {
    let _ = config.disable_pkce; // PKCE is always enabled in the rewrite (analysis §9: Python-only escape hatch).
    Ok(BasicClient::new(ClientId::new(config.client_id.clone()))
        .set_client_secret(ClientSecret::new(config.client_secret.reveal().to_string()))
        .set_auth_uri(AuthUrl::from_url(authorize_url))
        .set_token_uri(TokenUrl::from_url(token_url))
        .set_redirect_uri(RedirectUrl::from_url(redirect_uri)))
}

/// Translate the `oauth2` crate's `RequestTokenError` into an
/// [`OAuthError`] for the authorization-code exchange leg.
pub(super) fn map_request_token_error<RE>(
    err: RequestTokenError<RE, StandardErrorResponse<BasicErrorResponseType>>,
) -> OAuthError
where
    RE: std::error::Error + Send + Sync + 'static,
{
    match err {
        RequestTokenError::ServerResponse(resp) => IdpSnafu {
            error: error_type_as_str(resp.error()),
            description: resp.error_description().cloned().unwrap_or_default(),
        }
        .build(),
        RequestTokenError::Request(inner) => {
            // OAuthError itself implements std::error::Error, so when the
            // adapter surfaces an OAuthError (e.g. Transport), we re-wrap
            // it by downcasting through `Box<dyn Error>`.
            let any: Box<dyn std::error::Error + Send + Sync> = Box::new(inner);
            match any.downcast::<OAuthError>() {
                Ok(boxed) => *boxed,
                Err(other) => IdpSnafu {
                    error: "transport".to_string(),
                    description: other.to_string(),
                }
                .build(),
            }
        }
        RequestTokenError::Parse(e, _bytes) => {
            tracing::debug!(error = %e, "OAuth token response failed to parse");
            // serde_path_to_error wraps the serde_json error; pull the
            // inner one out so we can fill the TokenResponseDecode source.
            TokenResponseDecodeSnafu.into_error(e.into_inner())
        }
        RequestTokenError::Other(s) => {
            if s.contains("access_token") {
                MissingAccessTokenSnafu.build()
            } else {
                IdpSnafu {
                    error: "unknown".to_string(),
                    description: s,
                }
                .build()
            }
        }
    }
}

/// Refresh-leg specialization: surfaces [`OAuthError::RefreshTokenExchange`]
/// for any failure so cache-eviction logic in
/// [`acquire_authorization_code`] can match on a single variant.
pub(super) fn map_refresh_token_error<RE>(
    err: RequestTokenError<RE, StandardErrorResponse<BasicErrorResponseType>>,
) -> OAuthError
where
    RE: std::error::Error + Send + Sync + 'static,
{
    tracing::debug!(error = %err, "OAuth refresh-token exchange failed");
    let _ = err;
    RefreshTokenExchangeSnafu.build()
}

fn error_type_as_str(err: &BasicErrorResponseType) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = write!(out, "{err}");
    out
}

fn persist_access_token(
    config: &OAuthAuthorizationCodeConfig,
    cache_host_url: &str,
    token_cache: Option<&dyn TokenCache>,
    acquired: &AcquiredOAuthToken,
) {
    if !config.client_store_temporary_credential {
        tracing::debug!("OAuth token caching disabled; skipping persist (access token)");
        return;
    }
    if token_cache.is_none() {
        tracing::debug!("No OAuth token cache available; skipping persist (access token)");
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
}

fn persist_refresh_token(
    config: &OAuthAuthorizationCodeConfig,
    cache_host_url: &str,
    token_cache: Option<&dyn TokenCache>,
    acquired: &AcquiredOAuthToken,
) {
    if !config.client_store_temporary_credential {
        tracing::debug!("OAuth token caching disabled; skipping persist (refresh token)");
        return;
    }
    if token_cache.is_none() {
        tracing::debug!("No OAuth token cache available; skipping persist (refresh token)");
        return;
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

fn resolve_authorize_url(server_url: &Url, override_url: Option<&Url>) -> Result<Url, OAuthError> {
    if let Some(url) = override_url {
        return Ok(url.clone());
    }
    let host = server_url.host_str().unwrap_or("");
    let default = format!("https://{host}/oauth/authorize");
    Url::parse(&default).context(EndpointUrlParseSnafu { url: default })
}

fn resolve_token_url(server_url: &Url, override_url: Option<&Url>) -> Result<Url, OAuthError> {
    if let Some(url) = override_url {
        return Ok(url.clone());
    }
    let host = server_url.host_str().unwrap_or("");
    let default = format!("https://{host}/oauth/token-request");
    Url::parse(&default).context(EndpointUrlParseSnafu { url: default })
}

// Silence the unused-type warning emitted by the lint rather than
// touching the visibility surface of `BasicErrorResponse`.
const _: fn() = || {
    let _ = std::marker::PhantomData::<BasicErrorResponse>;
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rest_parameters::DEFAULT_AUTHENTICATION_TIMEOUT_SECS;
    use crate::token_cache::TokenCacheError;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use wiremock::matchers::{body_string_contains, method, path};
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

    fn server_url() -> Url {
        Url::parse("https://acct.snowflakecomputing.com").unwrap()
    }

    #[test]
    fn resolve_default_urls_use_server_host() {
        let auth = resolve_authorize_url(&server_url(), None).unwrap();
        assert_eq!(
            auth.as_str(),
            "https://acct.snowflakecomputing.com/oauth/authorize"
        );
        let tok = resolve_token_url(&server_url(), None).unwrap();
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
        let acquired = acquire_authorization_code(&client, &server_url(), &config, Some(&cache))
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
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=RT-OLD"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(
                    r#"{"access_token":"AT-NEW","refresh_token":"RT-NEW","token_type":"Bearer","expires_in":3600}"#,
                    "application/json",
                ),
            )
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
        let cache = StubTokenCache::new();
        token::store_oauth_refresh_token(token_url.as_str(), "alice", "RT-OLD", Some(&cache));
        let config = cfg_with_token_url(token_url.clone());

        let client = reqwest::Client::new();
        let acquired = acquire_authorization_code(&client, &server_url(), &config, Some(&cache))
            .await
            .expect("refresh succeeds");

        assert_eq!(acquired.access_token.reveal(), "AT-NEW");
        assert_eq!(
            acquired.refresh_token.as_ref().map(|s| s.reveal().as_str()),
            Some("RT-NEW")
        );
        assert_eq!(acquired.expires_in, Some(Duration::from_secs(3600)));

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
            .respond_with(ResponseTemplate::new(400).set_body_raw(
                r#"{"error":"invalid_grant","error_description":"refresh token expired"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
        let cache = StubTokenCache::new();
        token::store_oauth_refresh_token(token_url.as_str(), "alice", "RT-OLD", Some(&cache));

        let launch: BrowserLaunchFn = Box::new(|_, _| Box::pin(async {}));
        let mut config_short =
            cfg_with_token_url(Url::parse(&format!("{}/oauth/token", server.uri())).unwrap());
        config_short.authentication_timeout_secs = 1;
        let client = reqwest::Client::new();
        let result = acquire_authorization_code_inner(
            &client,
            &server_url(),
            &config_short,
            Some(&cache),
            launch,
        )
        .await;
        assert!(result.is_err());
        let stored_rt =
            token::try_get_cached_oauth_refresh_token(token_url.as_str(), "alice", Some(&cache));
        assert!(
            stored_rt.is_none(),
            "expired refresh token must be evicted from the cache"
        );
    }

    #[tokio::test]
    async fn full_interactive_flow_drives_loopback_directly() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("grant_type=authorization_code"))
            .and(body_string_contains("code=THE-CODE"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"access_token":"AT-FRESH","refresh_token":"RT-FRESH","token_type":"Bearer","expires_in":600}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
        let mut config = cfg_with_token_url(token_url);
        config.authorization_url =
            Some(Url::parse("https://idp.example.com/oauth/authorize").unwrap());
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
                // Hold the stream open long enough for the server to flush
                // its response. Dropping immediately after write_all can
                // race with axum's handler invocation.
                tokio::time::sleep(Duration::from_millis(100)).await;
            })
        });

        let client = reqwest::Client::new();
        let acquired =
            acquire_authorization_code_inner(&client, &server_url(), &config, None, launch)
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
            .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
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
                tokio::time::sleep(Duration::from_millis(100)).await;
            })
        });
        let client = reqwest::Client::new();
        let err = acquire_authorization_code_inner(&client, &server_url(), &config, None, launch)
            .await
            .expect_err("must fail with IdpError");
        assert!(matches!(err, OAuthError::IdpError { .. }), "got {err:?}");
    }

    #[tokio::test]
    async fn state_mismatch_in_redirect_surfaces_state_mismatch_variant() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
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
                tokio::time::sleep(Duration::from_millis(100)).await;
            })
        });
        let client = reqwest::Client::new();
        let err = acquire_authorization_code_inner(&client, &server_url(), &config, None, launch)
            .await
            .expect_err("must fail with state mismatch");
        assert!(matches!(err, OAuthError::StateMismatch { .. }));
    }
}
