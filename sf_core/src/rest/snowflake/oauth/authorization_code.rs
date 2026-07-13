//! OAuth 2.0 Authorization Code flow with PKCE (S256).
//!
//! Owns the end-to-end orchestration: PKCE verifier/challenge, CSRF state,
//! browser launch, loopback HTTP redirect handling, token exchange, and
//! refresh-token rotation. Loopback binds `127.0.0.1` explicitly (Node's
//! `0.0.0.0` bind is intentionally not replicated).
//!
//! Defaults follow JDBC/Python:
//! * `authorization_url` ⇒ `https://{server_host}/oauth/authorize`
//! * `token_url` ⇒ `https://{server_host}/oauth/token-request`
//! * `redirect_uri` ⇒ ephemeral `http://127.0.0.1:<port>/`
//!
//! Caching obeys the cross-driver convention: when
//! `client_store_temporary_credential` is enabled and a [`TokenCache`] is
//! provided, the access token is consulted first, then the refresh token
//! is exchanged, and only if both fall through do we drive the
//! interactive flow.
//!
//! HTTP transport, body encoding, CSRF generation, and PKCE generation
//! are owned by the `oauth2` crate; we only orchestrate the moving parts
//! via [`OAuthHttpClient`], the WSL-safe [`crate::rest::snowflake::browser`]
//! module, and [`loopback_server`].

use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

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
    AuthenticationTimeoutSnafu, EndpointUrlParseSnafu, IdpSnafu, MissingAccessTokenSnafu,
    OAuthError, RedirectUriParseSnafu, RefreshTokenExchangeSnafu, StateMismatchSnafu,
    TokenResponseDecodeSnafu,
};
use super::http_client::make_http_client;
use super::loopback_server::{self, RedirectResult};
use super::token;
use crate::config::rest_parameters::OAuthAuthorizationCodeConfig;
use crate::sensitive::SensitiveString;
use crate::token_cache::TokenCache;

/// Drift B.3: shared deadline for the AC flow. Tracks the start instant
/// and the configured `authentication_timeout` budget so that the
/// loopback wait, the IdP token exchange, and the refresh exchange
/// all share one end-to-end deadline rather than each getting its own
/// 120 s window. `remaining()` returns `None` once the budget is
/// exhausted, at which point [`run_with_deadline`] short-circuits with
/// [`OAuthError::AuthenticationTimeout`].
#[derive(Clone, Copy, Debug)]
struct FlowDeadline {
    start: Instant,
    budget: Duration,
}

impl FlowDeadline {
    fn new(budget_secs: u64) -> Self {
        Self {
            start: Instant::now(),
            budget: Duration::from_secs(budget_secs),
        }
    }

    fn remaining(&self) -> Option<Duration> {
        self.budget.checked_sub(self.start.elapsed())
    }

    fn elapsed_secs(&self) -> u64 {
        self.start.elapsed().as_secs()
    }
}

/// Run `fut` under the deadline; on timeout return
/// [`OAuthError::AuthenticationTimeout`] without dragging the caller's
/// own error type into the signature.
async fn run_with_deadline<F, T>(deadline: FlowDeadline, fut: F) -> Result<T, OAuthError>
where
    F: Future<Output = Result<T, OAuthError>>,
{
    let Some(remaining) = deadline.remaining() else {
        return AuthenticationTimeoutSnafu {
            elapsed_secs: deadline.elapsed_secs(),
        }
        .fail();
    };
    match tokio::time::timeout(remaining, fut).await {
        Ok(inner) => inner,
        Err(_) => AuthenticationTimeoutSnafu {
            elapsed_secs: deadline.elapsed_secs(),
        }
        .fail(),
    }
}

/// What an OAuth flow returns to the caller (the wiring layer hands
/// the access token, and optionally the DPoP JWK, to the Snowflake
/// login-request).
#[derive(Debug)]
pub(crate) struct AcquiredOAuthToken {
    pub(crate) access_token: SensitiveString,
    /// Refresh token returned by the IdP, when issued. Persisted by the
    /// flow itself; the wiring layer does not consume it directly.
    pub(crate) refresh_token: Option<SensitiveString>,
    /// Present iff DPoP was negotiated. Carries the JWK JSON so the
    /// Snowflake login-request can reuse the same key when signing.
    pub(crate) dpop_jwk_json: Option<String>,
    /// IdP-reported lifetime of the access token, when present. Snowflake
    /// drives session validity itself, so this is informational only.
    #[allow(dead_code)]
    pub(crate) expires_in: Option<Duration>,
}

/// Closure-shaped browser launcher. Production callers go through
/// [`crate::rest::snowflake::browser::open_url`] (WSL-safe; see
/// SNOW-3649282) with a stdout paste fallback (drift C.6 — the fallback
/// URL is stdout per `doc/oauth.md` §3), and only after the URL passes
/// [`crate::rest::snowflake::browser::validate_browser_url`]; tests can
/// substitute a deterministic driver that pokes the loopback directly.
///
/// Exposed at `pub(crate)` so [`OAuthAuthorizationCodeConfig`] can carry
/// an `Arc<dyn Fn() -> BrowserLaunchFn + …>` factory field — letting
/// tests inject deterministic launchers as configuration data rather
/// than through global mutable state.
pub(crate) type BrowserLaunchFn =
    Box<dyn FnOnce(Url, Url) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

fn default_browser_launch() -> BrowserLaunchFn {
    Box::new(|authorize_url, _redirect_uri| {
        Box::pin(async move {
            tracing::debug!(
                authority = %authorize_url.authority(),
                path = %authorize_url.path(),
                "Opening system browser for OAuth authorization"
            );
            // Validate the URL and route it through the WSL-safe launcher
            // (SNOW-3649282). The paste fallback is printed only when a
            // *validated* URL fails to launch.
            let url = authorize_url.to_string();
            launch_authorize_url(&url, crate::rest::snowflake::browser::open_url, || {
                println!("Open this URL in your browser to continue: {url}")
            });
        })
    })
}

/// Drive the "open the OAuth authorization URL" decision, factored out of
/// the launcher closure so its security-relevant control flow is unit-
/// testable without spawning a browser or capturing stdout.
///
/// Contract (SNOW-3649282):
/// * URL fails validation ⇒ refuse; `open` is **not** called and the paste
///   `fallback` is **not** shown (printing it would invite the user to
///   hand-open the very URL we rejected as a possible injection).
/// * URL is valid ⇒ hand it to `open`; only if that *launch* fails do we
///   show the manual-paste `fallback`.
fn launch_authorize_url(
    url: &str,
    open: impl FnOnce(&str) -> Result<(), String>,
    fallback: impl FnOnce(),
) {
    if let Err(e) = crate::rest::snowflake::browser::validate_browser_url(url) {
        tracing::error!(error = %e, "Refusing to open browser: authorization URL failed validation");
        return;
    }
    if let Err(e) = open(url) {
        tracing::warn!(error = %e, "Failed to launch system browser; printing paste fallback");
        fallback();
    }
}

#[tracing::instrument(
    skip(client, config, token_cache),
    fields(server_url = %server_url, username = %config.username),
)]
pub(crate) async fn run_oauth_authorization_code(
    // TODO(SNOW-XXX): build a no-redirect sibling reqwest client for OAuth
    // token calls (see https://docs.rs/oauth2/5.0.0/oauth2/#security-warning).
    client: &reqwest::Client,
    server_url: &Url,
    config: &OAuthAuthorizationCodeConfig,
    token_cache: Option<&dyn TokenCache>,
    disable_parallel_user_prompt: bool,
    prompt_locks: Option<&std::sync::Arc<super::super::prompt_lock::PromptLockMap>>,
) -> Result<AcquiredOAuthToken, OAuthError> {
    // The launcher rides on the config (Arc'd factory ⇒ each call mints
    // a fresh `FnOnce` so the retry-on-failure path can rebuild one).
    // Production builds default to `None` ⇒ open the system browser;
    // test/test-utils builds default to `Some(noop)` ⇒ never touch the
    // OS browser. See `OAuthAuthorizationCodeConfig::from_settings`.
    let launch_browser = config
        .browser_launcher
        .as_ref()
        .map(|factory| factory())
        .unwrap_or_else(default_browser_launch);
    run_authorization_code_flow(
        client,
        server_url,
        config,
        token_cache,
        launch_browser,
        disable_parallel_user_prompt,
        prompt_locks,
    )
    .await
}

#[tracing::instrument(skip(client, config, token_cache, launch_browser), fields(username = %config.username))]
async fn run_authorization_code_flow(
    client: &reqwest::Client,
    server_url: &Url,
    config: &OAuthAuthorizationCodeConfig,
    token_cache: Option<&dyn TokenCache>,
    launch_browser: BrowserLaunchFn,
    disable_parallel_user_prompt: bool,
    prompt_locks: Option<&std::sync::Arc<super::super::prompt_lock::PromptLockMap>>,
) -> Result<AcquiredOAuthToken, OAuthError> {
    tracing::info!("Starting OAuth authorization code flow");

    let token_url = resolve_token_url(server_url, config.token_url.as_ref())?;
    let cache_host_url = token_url.as_str();
    // Drift B.3: single end-to-end deadline shared by the loopback
    // wait, the IdP token exchange and any refresh exchange.
    let deadline = FlowDeadline::new(config.flow_options.authentication_timeout_secs);

    // Acquire the per-<user, idp-host> prompt-lock when serializing concurrent
    // interactive auth flows.  The lock is held across the cache re-check and
    // the entire interactive flow + persist so that waiters find the token in
    // the cache once the lock is released (double-checked locking).
    // Lock key uses the IdP token-URL host.
    let _prompt_guard = if let Some(locks) = prompt_locks {
        if super::super::prompt_lock::is_eligible(
            config.client_store_temporary_credential,
            disable_parallel_user_prompt,
            &config.username,
        ) {
            let host = token_url.host_str().unwrap_or_default();
            Some(
                super::super::prompt_lock::acquire(
                    locks,
                    host,
                    &config.username,
                    crate::token_cache::TokenType::OAuthAccessToken,
                )
                .await,
            )
        } else {
            None
        }
    } else {
        None
    };

    // 1. Cache short-circuit (also acts as the post-lock double-check for
    //    waiters: if another thread already acquired and persisted a token,
    //    this returns it without re-running the interactive flow).
    if let Some(cached) = try_cache_short_circuit(
        client,
        &token_url,
        cache_host_url,
        config,
        token_cache,
        deadline,
    )
    .await
    {
        return Ok(cached);
    }

    // 2. Interactive flow.
    let result = run_interactive_flow(
        client,
        server_url,
        &token_url,
        config,
        launch_browser,
        deadline,
    )
    .await?;

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
    deadline: FlowDeadline,
) -> Option<AcquiredOAuthToken> {
    if !config.client_store_temporary_credential {
        tracing::debug!("OAuth token cache disabled; skipping cache lookup");
        return None;
    }
    let Some(_cache) = token_cache else {
        tracing::debug!("No OAuth token cache available; skipping cache lookup");
        return None;
    };

    // Drift B.11: when DPoP is enabled, look up the bundled cache entry
    // first. The bundle carries both the access token and the JWK JSON
    // so the same DPoP key can sign the Snowflake login leg without
    // re-running the interactive flow.
    if config.flow_options.enable_dpop
        && let Some((cached_at, jwk_json)) =
            token::try_get_cached_oauth_dpop_bundled(cache_host_url, &config.username, token_cache)
    {
        // Validate the cached JWK before handing the bundle back to
        // the caller. A corrupt JWK would only surface later when
        // the Snowflake-login DPoP signing path tries to rehydrate
        // the key, so fail fast here and evict the bad entry.
        match dpop::DPoPKey::from_jwk_json(&jwk_json) {
            Ok(_) => {
                tracing::debug!("OAuth access token served from DPoP-bundled cache");
                return Some(AcquiredOAuthToken {
                    access_token: cached_at,
                    refresh_token: None,
                    dpop_jwk_json: Some(jwk_json),
                    expires_in: None,
                });
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Cached DPoP-bundled access token has unparseable JWK; evicting"
                );
                token::remove_oauth_dpop_bundled(cache_host_url, &config.username, token_cache);
            }
        }
    }

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
        // Drift B.12: when DPoP was negotiated for this connection, we
        // must reuse the same key on the refresh leg so the IdP can
        // verify the proof JWT and so we can rebundle the new access
        // token with the same JWK afterwards. Generating a fresh key
        // here would silently break the cached-bundle contract.
        let dpop_key = if config.flow_options.enable_dpop {
            match dpop::DPoPKey::generate() {
                Ok(k) => Some(k),
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to generate DPoP key for refresh leg");
                    None
                }
            }
        } else {
            None
        };
        match refresh_access_token(
            client,
            token_url,
            config,
            refresh.reveal(),
            dpop_key.as_ref(),
            deadline,
        )
        .await
        {
            Ok(refreshed) => {
                tracing::info!("Refreshed OAuth access token");
                persist_access_token(config, cache_host_url, token_cache, &refreshed);
                if refreshed.refresh_token.is_some() {
                    persist_refresh_token(config, cache_host_url, token_cache, &refreshed);
                } else {
                    // Cross-driver convention (.NET / Node): when the
                    // IdP omits a new refresh
                    // token, evict the cached one. It has either been
                    // single-use-rotated server-side (Snowflake-IdP with
                    // `enable_single_use_refresh_tokens=true`) or
                    // otherwise invalidated, and re-presenting it on the
                    // next refresh would fail.
                    tracing::debug!(
                        "Refresh response omitted refresh_token; evicting cached refresh token"
                    );
                    token::remove_oauth_refresh_token(
                        cache_host_url,
                        &config.username,
                        token_cache,
                    );
                }
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
    deadline: FlowDeadline,
) -> Result<AcquiredOAuthToken, OAuthError> {
    let authorize_url_base = resolve_authorize_url(server_url, config.authorization_url.as_ref())?;
    let binding = loopback_server::bind(config.redirect_uri.as_ref()).await?;
    let redirect_uri_advertised = binding.redirect_uri.as_configured().to_string();
    let redirect_uri = binding.redirect_uri.parsed().clone();

    let dpop_key = if config.flow_options.enable_dpop {
        Some(dpop::DPoPKey::generate()?)
    } else {
        None
    };
    let is_dpop_enabled = dpop_key.is_some();

    let oauth_client = build_oauth_client(
        config,
        authorize_url_base.clone(),
        token_url.clone(),
        redirect_uri_advertised,
    )?;

    // PKCE S256 is on by default across drivers; Python
    // is the only one with a `oauth_disable_pkce` escape hatch and we
    // mirror it here (drift A.2). When disabled we skip both the
    // `set_pkce_challenge` on the authorize URL and the
    // `set_pkce_verifier` on the token exchange.
    let pkce = if config.disable_pkce {
        None
    } else {
        Some(PkceCodeChallenge::new_random_sha256())
    };

    // 256-bit CSRF state (drift A.4). 32 random bytes base64url-encoded.
    let mut request = oauth_client.authorize_url(|| CsrfToken::new_random_len(32));
    if let Some((challenge, _)) = pkce.as_ref() {
        request = request.set_pkce_challenge(challenge.clone());
    }
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
    // Drift B.3: cap the loopback wait at the *remaining* end-to-end
    // budget, not the full `authentication_timeout` — otherwise a
    // delayed click leaves no time for the token exchange below.
    let listener = binding;
    let loopback_budget = deadline.remaining().ok_or_else(|| {
        AuthenticationTimeoutSnafu {
            elapsed_secs: deadline.elapsed_secs(),
        }
        .build()
    })?;

    let (redirect_result, _) = tokio::join!(
        listener.wait_for_redirect(loopback_budget),
        launch_browser(authorize_url, redirect_uri),
    );
    let redirect: RedirectResult = redirect_result?;

    // CSRF check via timing-safe equality on `oauth2::CsrfToken`.
    let received = CsrfToken::new(redirect.state.clone());
    snafu::ensure!(received == state, StateMismatchSnafu);

    let http = make_http_client(client, dpop_key.as_ref(), token_url);

    let mut exchange =
        oauth_client.exchange_code(AuthorizationCode::new(redirect.code.reveal().to_string()));
    if let Some((_, verifier)) = pkce {
        exchange = exchange.set_pkce_verifier(PkceCodeVerifier::new(verifier.into_secret()));
    }
    if config.enable_single_use_refresh_tokens {
        exchange = exchange.add_extra_param("enable_single_use_refresh_tokens", "true");
    }

    tracing::debug!("Starting OAuth code exchange");
    // Drift B.3: token exchange shares the same end-to-end budget.
    let response = run_with_deadline(deadline, async {
        exchange
            .request_async(&http)
            .await
            .map_err(map_request_token_error)
    })
    .await?;

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
    dpop_key: Option<&DPoPKey>,
    deadline: FlowDeadline,
) -> Result<AcquiredOAuthToken, OAuthError> {
    tracing::debug!("Refreshing OAuth access token");
    // For the refresh leg we don't need authorize_url; reuse the same
    // token_url for both endpoints to satisfy the typestate bounds.
    let oauth_client = build_oauth_client(
        config,
        token_url.clone(),
        token_url.clone(),
        // Refresh leg has no redirect URI requirement. Re-use the token
        // URL string as a placeholder; `exchange_refresh_token` never reads it.
        token_url.to_string(),
    )?;

    // Drift B.12: reuse the caller-supplied DPoP key on the refresh hop
    // so the IdP can verify the proof and so the new access token can be
    // rebundled under the same JWK below.
    let http = make_http_client(client, dpop_key, token_url);

    let refresh = RefreshToken::new(refresh_token.to_string());
    let mut request = oauth_client.exchange_refresh_token(&refresh);
    if let Some(scope) = config.scope.as_deref() {
        request = request.add_scope(Scope::new(scope.to_string()));
    }

    // Drift B.3: refresh exchange shares the AC flow's end-to-end
    // deadline.
    let response = run_with_deadline(deadline, async {
        request
            .request_async(&http)
            .await
            .map_err(map_refresh_token_error)
    })
    .await?;

    let access_token = SensitiveString::from(response.access_token().secret().clone());
    if access_token.reveal().is_empty() {
        return MissingAccessTokenSnafu.fail();
    }
    let refresh_token = response
        .refresh_token()
        .map(|rt| rt.secret().clone())
        .filter(|s| !s.is_empty())
        .map(SensitiveString::from);

    // Drift B.12: serialize the DPoP JWK alongside the new access token
    // so `persist_access_token` takes the bundled-cache write path
    // (DPoP key + access token lives under the bundled entry, not
    // `OAUTH_ACCESS_TOKEN`).
    let dpop_jwk_json = match dpop_key {
        Some(k) => Some(k.to_jwk_json()?),
        None => None,
    };

    Ok(AcquiredOAuthToken {
        access_token,
        refresh_token,
        dpop_jwk_json,
        expires_in: response.expires_in(),
    })
}

fn build_oauth_client(
    config: &OAuthAuthorizationCodeConfig,
    authorize_url: Url,
    token_url: Url,
    redirect_uri_advertised: String,
) -> Result<
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    OAuthError,
> {
    // Snowflake-as-IdP substitutes `LOCAL_APPLICATION` for missing
    // client_id/client_secret (drift B.1). We treat "user did not override
    // the token URL" as the
    // Snowflake-as-IdP signal, matching JDBC's `OAuthUtil` heuristic.
    let snowflake_idp = config.token_url.is_none();
    let (client_id, client_secret) = resolve_client_credentials(config, snowflake_idp);
    // Use `RedirectUrl::new` (not `from_url`) so the verbatim user-supplied
    // string is stored and emitted byte-for-byte by `oauth2`'s wire layer.
    // `from_url` re-serializes the `Url` struct, which adds a trailing slash
    // to bare origins (e.g. `http://localhost:12346` → `http://localhost:12346/`),
    // breaking IdPs that do RFC 6749 §3.1.2.3 exact-string `redirect_uri`
    // matching (e.g. Okta).
    let redirect_url = RedirectUrl::new(redirect_uri_advertised).context(RedirectUriParseSnafu)?;
    Ok(BasicClient::new(ClientId::new(client_id))
        .set_client_secret(ClientSecret::new(client_secret))
        .set_auth_uri(AuthUrl::from_url(authorize_url))
        .set_token_uri(TokenUrl::from_url(token_url))
        .set_redirect_uri(redirect_url))
}

/// Drift B.1: if no client credentials were provided and Snowflake is
/// the IdP, substitute the literal `LOCAL_APPLICATION` for both
/// `client_id` and `client_secret` (matches JDBC, ODBC, Python, .NET,
/// Go and Node). External IdPs require explicit
/// credentials, so the empty values fall through unchanged.
fn resolve_client_credentials(
    config: &OAuthAuthorizationCodeConfig,
    snowflake_idp: bool,
) -> (String, String) {
    const LOCAL_APPLICATION: &str = "LOCAL_APPLICATION";
    let client_id = if snowflake_idp && config.client_id.is_empty() {
        LOCAL_APPLICATION.to_string()
    } else {
        config.client_id.clone()
    };
    let client_secret = if snowflake_idp && config.client_secret.reveal().is_empty() {
        LOCAL_APPLICATION.to_string()
    } else {
        config.client_secret.reveal().to_string()
    };
    (client_id, client_secret)
}

/// Translate the `oauth2` crate's `RequestTokenError` into an
/// [`OAuthError`] for the authorization-code exchange leg.
///
/// The `'static` bound on `RE` is required because the `Request` variant
/// is boxed into `Box<dyn Error + Send + Sync>` and then downcast via
/// `Any` to recover an [`OAuthError`]; `Any`-based downcasting requires
/// `'static`.
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
/// [`run_oauth_authorization_code`] can match on a single variant.
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
    use crate::config::rest_parameters::{DEFAULT_AUTHENTICATION_TIMEOUT_SECS, OAuthFlowOptions};
    use crate::token_cache::TokenCacheError;
    use base64::Engine as _;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // ─── launch_authorize_url control flow (SNOW-3649282) ────────────────

    #[test]
    fn launch_rejects_malicious_url_without_opening_or_fallback() {
        use std::cell::Cell;
        let opened = Cell::new(false);
        let fell_back = Cell::new(false);
        launch_authorize_url(
            "https://idp.example.com/authorize?state=poc|calc",
            |_| {
                opened.set(true);
                Ok(())
            },
            || fell_back.set(true),
        );
        assert!(
            !opened.get(),
            "a rejected URL must never reach the launcher"
        );
        assert!(
            !fell_back.get(),
            "a rejected URL must NOT be printed as a paste fallback (would invite the user \
             to hand-open the injection payload)"
        );
    }

    #[test]
    fn launch_valid_url_opens_and_skips_fallback_on_success() {
        use std::cell::Cell;
        let opened = Cell::new(false);
        let fell_back = Cell::new(false);
        launch_authorize_url(
            "https://idp.example.com/authorize?client_id=abc&state=xyz",
            |_| {
                opened.set(true);
                Ok(())
            },
            || fell_back.set(true),
        );
        assert!(opened.get(), "a valid URL must be handed to the launcher");
        assert!(
            !fell_back.get(),
            "no paste fallback when the launch succeeds"
        );
    }

    #[test]
    fn launch_valid_url_shows_fallback_when_launch_fails() {
        use std::cell::Cell;
        let fell_back = Cell::new(false);
        launch_authorize_url(
            "https://idp.example.com/authorize?client_id=abc",
            |_| Err("no browser available".to_string()),
            || fell_back.set(true),
        );
        assert!(
            fell_back.get(),
            "the paste fallback must show when a validated URL fails to launch"
        );
    }

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
            client_store_temporary_credential: true,
            flow_options: OAuthFlowOptions {
                enable_dpop: false,
                authentication_timeout_secs: DEFAULT_AUTHENTICATION_TIMEOUT_SECS,
            },
            // Unit tests that drive the interactive leg pass their
            // launcher directly through `run_authorization_code_flow`
            // (see `loopback_redirect_with`). Cache / refresh-only paths
            // never invoke the launcher, so `None` is safe.
            browser_launcher: None,
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

    #[test]
    fn build_oauth_client_emits_redirect_uri_verbatim_without_trailing_slash() {
        // Regression guard for SNOW-3637145: a bare-origin redirect URI must
        // reach the IdP byte-for-byte. `RedirectUrl::from_url` re-serializes
        // the `Url` and appends a trailing slash
        // (`http://localhost:12346` → `http://localhost:12346/`), breaking
        // IdPs that do RFC 6749 §3.1.2.3 exact-string matching (Okta). This
        // asserts on the emitted authorize URL, so it fails if anyone swaps
        // `RedirectUrl::new` back to `from_url`.
        let config = cfg_with_token_url(server_url());
        let authorize_url = Url::parse("https://idp.example.com/authorize").unwrap();
        let token_url = Url::parse("https://idp.example.com/token").unwrap();

        let client = build_oauth_client(
            &config,
            authorize_url,
            token_url,
            "http://localhost:12346".to_string(),
        )
        .expect("client builds");

        let (url, _state) = client.authorize_url(|| CsrfToken::new_random_len(32)).url();
        let redirect = url
            .query_pairs()
            .find(|(k, _)| k == "redirect_uri")
            .map(|(_, v)| v.into_owned())
            .expect("authorize URL must carry a redirect_uri parameter");
        assert_eq!(
            redirect, "http://localhost:12346",
            "redirect_uri must be emitted verbatim (no trailing slash added)"
        );
    }

    #[tokio::test]
    async fn cached_access_token_short_circuits_full_flow() {
        let cache = StubTokenCache::new();
        let token_url = Url::parse("https://idp.example.com/oauth/token").unwrap();
        token::store_oauth_access_token(token_url.as_str(), "alice", "CACHED-AT", Some(&cache));

        let config = cfg_with_token_url(token_url);
        let client = reqwest::Client::new();
        let acquired = run_oauth_authorization_code(
            &client,
            &server_url(),
            &config,
            Some(&cache),
            false,
            None,
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
        let acquired = run_oauth_authorization_code(
            &client,
            &server_url(),
            &config,
            Some(&cache),
            false,
            None,
        )
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
        config_short.flow_options.authentication_timeout_secs = 1;
        let client = reqwest::Client::new();
        let result = run_authorization_code_flow(
            &client,
            &server_url(),
            &config_short,
            Some(&cache),
            launch,
            false,
            None,
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
            run_authorization_code_flow(&client, &server_url(), &config, None, launch, false, None)
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
        let err =
            run_authorization_code_flow(&client, &server_url(), &config, None, launch, false, None)
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
        let err =
            run_authorization_code_flow(&client, &server_url(), &config, None, launch, false, None)
                .await
                .expect_err("must fail with state mismatch");
        assert!(matches!(err, OAuthError::StateMismatch { .. }));
    }
    // ─── Step 2.4 coverage additions ─────────────────────────────────────
    // The block below extends the inline tests with cross-driver gaps:
    // token redaction in tracing, DPoP nonce retry, single-use refresh
    // token body opt-in, refresh-token rotation persistence, eviction
    // when the refresh response omits a new RT, scope-default behavior,
    // and the non-fatal-ness of token-cache failures.

    /// `TokenCache` whose mutating operations always fail. Used to prove
    /// that `OAuthError::Cache`-equivalent failures inside the cache
    /// callees are reduced to WARN logs rather than aborting the flow
    /// (cross-driver: token-cache I/O is best-effort).
    struct FaultingTokenCache;
    impl crate::token_cache::TokenCache for FaultingTokenCache {
        fn add_token(
            &self,
            _host: &str,
            _username: &str,
            _token_type: crate::token_cache::TokenType,
            _token_value: &str,
        ) -> Result<(), TokenCacheError> {
            Err(TokenCacheError::TokenStorage {
                source: "synthetic add_token failure".into(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })
        }
        fn remove_token(
            &self,
            _host: &str,
            _username: &str,
            _token_type: crate::token_cache::TokenType,
        ) -> Result<(), TokenCacheError> {
            Err(TokenCacheError::TokenRemoval {
                source: "synthetic remove_token failure".into(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })
        }
        fn get_token(
            &self,
            _host: &str,
            _username: &str,
            _token_type: crate::token_cache::TokenType,
        ) -> Result<Option<String>, TokenCacheError> {
            // Pretend the cache is empty so the flow runs the full
            // interactive path; we want add_token failures to dominate.
            Ok(None)
        }
    }

    /// Drive the loopback by connecting and writing the redirect line
    /// for the supplied `code` (extracting the state from the authorize
    /// URL the flow generated). Used by tests that need the interactive
    /// branch to complete deterministically.
    fn loopback_redirect_with(code: &'static str) -> BrowserLaunchFn {
        Box::new(move |authorize_url, redirect_uri| {
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
                let req =
                    format!("GET /?code={code}&state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
                let _ = s.write_all(req.as_bytes()).await;
                // Hold the stream open long enough for the server to flush
                // its response. Dropping immediately after write_all can
                // race with axum's handler invocation (mirrors the same
                // guard in `full_interactive_flow_drives_loopback_directly`).
                tokio::time::sleep(Duration::from_millis(100)).await;
            })
        })
    }

    #[tokio::test]
    async fn dpop_nonce_retry_happens_exactly_once_with_nonce_claim() {
        // First request without DPoP nonce returns 400 + use_dpop_nonce
        // and a `DPoP-Nonce` header. The flow must retry exactly once
        // with the supplied nonce embedded in the proof JWT.
        let server = MockServer::start().await;

        // Wiremock evaluates mounts in priority order; we use the
        // built-in expectation+priority knobs to model the one-shot
        // sequence: the first matching POST gets the 400 (priority 1),
        // subsequent ones get the 200 (priority 5).
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(
                ResponseTemplate::new(400)
                    .insert_header("DPoP-Nonce", "nonce-from-server")
                    .set_body_raw(r#"{"error":"use_dpop_nonce"}"#, "application/json"),
            )
            .up_to_n_times(1)
            .with_priority(1)
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"access_token":"AT-AFTER-NONCE","token_type":"Bearer"}"#,
                "application/json",
            ))
            .with_priority(5)
            .expect(1)
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
        let mut config = cfg_with_token_url(token_url.clone());
        config.client_store_temporary_credential = false;
        config.flow_options.enable_dpop = true;
        config.authorization_url =
            Some(Url::parse("https://idp.example.com/oauth/authorize").unwrap());

        let client = reqwest::Client::new();
        let acquired = run_authorization_code_flow(
            &client,
            &server_url(),
            &config,
            None,
            loopback_redirect_with("THE-CODE"),
            false,
            None,
        )
        .await
        .expect("flow succeeds after one DPoP-nonce retry");
        assert_eq!(acquired.access_token.reveal(), "AT-AFTER-NONCE");

        let received = server.received_requests().await.unwrap();
        // We should observe exactly two POSTs: the original request
        // (without nonce in the proof) and the retry (with nonce).
        let token_posts: Vec<_> = received
            .iter()
            .filter(|r| r.url.path() == "/oauth/token" && r.method.as_str() == "POST")
            .collect();
        assert_eq!(
            token_posts.len(),
            2,
            "expected exactly one retry, observed {}",
            token_posts.len()
        );

        let dpop_b = token_posts[1]
            .headers
            .get("DPoP")
            .expect("retry must carry a DPoP header");
        let proof = dpop_b.to_str().unwrap();
        let claims_b64 = proof.split('.').nth(1).expect("DPoP proof has 3 segments");
        let claims_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(claims_b64)
            .expect("DPoP proof claims b64");
        let claims: serde_json::Value = serde_json::from_slice(&claims_bytes).unwrap();
        assert_eq!(claims["nonce"], "nonce-from-server", "claims={claims:?}");
    }

    #[tokio::test]
    async fn enable_single_use_refresh_tokens_appears_in_body_only_when_enabled() {
        // Two parallel mock servers: one expects the flag in the body,
        // the other expects its absence. Each test arm runs its own AC
        // exchange and asserts on the IdP-side body the wiremock saw.
        for enable in [true, false] {
            let server = MockServer::start().await;
            let body_matcher: Box<dyn Fn(&str) -> bool + Send + Sync> = if enable {
                Box::new(|b: &str| b.contains("enable_single_use_refresh_tokens=true"))
            } else {
                Box::new(|b: &str| !b.contains("enable_single_use_refresh_tokens"))
            };
            // Matcher is encoded via a custom mount that returns 200 only
            // when the body satisfies the condition; otherwise wiremock
            // returns its default 404 and the flow surfaces an error,
            // which makes the assertion below crisp.
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .and(BodyMatcher(body_matcher))
                .respond_with(ResponseTemplate::new(200).set_body_raw(
                    r#"{"access_token":"AT-OK","refresh_token":"RT-OK","token_type":"Bearer"}"#,
                    "application/json",
                ))
                .mount(&server)
                .await;

            let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
            let mut config = cfg_with_token_url(token_url);
            config.client_store_temporary_credential = false;
            config.authorization_url =
                Some(Url::parse("https://idp.example.com/oauth/authorize").unwrap());
            config.enable_single_use_refresh_tokens = enable;

            let client = reqwest::Client::new();
            let acquired = run_authorization_code_flow(
                &client,
                &server_url(),
                &config,
                None,
                loopback_redirect_with("CODE-OK"),
                false,
                None,
            )
            .await
            .unwrap_or_else(|e| panic!("enable={enable} flow failed: {e:?}"));
            assert_eq!(acquired.access_token.reveal(), "AT-OK");
        }
    }

    /// Custom wiremock body matcher backed by a closure. Used to assert
    /// presence/absence of arbitrary substrings without polluting the
    /// shared module surface.
    struct BodyMatcher(Box<dyn Fn(&str) -> bool + Send + Sync>);
    impl wiremock::Match for BodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            let body = std::str::from_utf8(&request.body).unwrap_or("");
            (self.0)(body)
        }
    }

    #[tokio::test]
    async fn refresh_response_with_new_refresh_token_persists_rotated_rt_in_cache() {
        // Sibling to `cached_refresh_token_is_exchanged_for_fresh_access_token`
        // — that test only checks the AT got rotated; this one pins the
        // RT side of the rotation (single-use refresh-token rotation).
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=RT-OLD"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"access_token":"AT-NEW","refresh_token":"RT-NEW","token_type":"Bearer"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
        let cache = StubTokenCache::new();
        token::store_oauth_refresh_token(token_url.as_str(), "alice", "RT-OLD", Some(&cache));
        let config = cfg_with_token_url(token_url.clone());
        let client = reqwest::Client::new();
        let _ = run_oauth_authorization_code(
            &client,
            &server_url(),
            &config,
            Some(&cache),
            false,
            None,
        )
        .await
        .expect("refresh succeeds");

        let stored_rt =
            token::try_get_cached_oauth_refresh_token(token_url.as_str(), "alice", Some(&cache))
                .map(|s| s.reveal().to_string());
        assert_eq!(
            stored_rt.as_deref(),
            Some("RT-NEW"),
            "rotated refresh token must replace the cached one"
        );
    }

    #[tokio::test]
    async fn refresh_response_without_refresh_token_evicts_cached_rt() {
        // Cross-driver convention (.NET / Node): when the IdP omits a
        // new `refresh_token`, the old one must be
        // evicted because it has either been single-use-rotated server
        // side or otherwise invalidated.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=RT-STALE"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"access_token":"AT-NEW","token_type":"Bearer"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
        let cache = StubTokenCache::new();
        token::store_oauth_refresh_token(token_url.as_str(), "alice", "RT-STALE", Some(&cache));
        let config = cfg_with_token_url(token_url.clone());
        let client = reqwest::Client::new();
        let acquired = run_oauth_authorization_code(
            &client,
            &server_url(),
            &config,
            Some(&cache),
            false,
            None,
        )
        .await
        .expect("refresh without new RT still succeeds");
        assert_eq!(acquired.access_token.reveal(), "AT-NEW");
        assert!(acquired.refresh_token.is_none());

        let stored_rt =
            token::try_get_cached_oauth_refresh_token(token_url.as_str(), "alice", Some(&cache));
        assert!(
            stored_rt.is_none(),
            "stale refresh token must be evicted when the response omits a new one"
        );
    }

    #[tokio::test]
    async fn cache_add_token_failure_does_not_abort_the_flow() {
        // Wire a faulting cache into a successful AC exchange and assert
        // we still get the access token back. Mirrors the cross-driver
        // expectation that token-cache I/O is best-effort.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"access_token":"AT-DESPITE-CACHE","token_type":"Bearer"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
        let mut config = cfg_with_token_url(token_url);
        config.authorization_url =
            Some(Url::parse("https://idp.example.com/oauth/authorize").unwrap());
        config.client_store_temporary_credential = true;

        let cache = FaultingTokenCache;
        let client = reqwest::Client::new();
        let acquired = run_authorization_code_flow(
            &client,
            &server_url(),
            &config,
            Some(&cache),
            loopback_redirect_with("CODE"),
            false,
            None,
        )
        .await
        .expect("faulting cache must not abort the flow");
        assert_eq!(acquired.access_token.reveal(), "AT-DESPITE-CACHE");
    }

    #[tokio::test]
    async fn ac_token_exchange_omits_scope_when_config_scope_is_none() {
        // The AC code-exchange counterpart to the existing CC test
        // `scope_is_added_to_body_when_provided`. We pin the negative
        // case here: when scope is None, the body must NOT carry a
        // `scope=` field.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(BodyMatcher(Box::new(|b: &str| !b.contains("scope="))))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"access_token":"AT-SCOPELESS","token_type":"Bearer"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
        let mut config = cfg_with_token_url(token_url);
        config.authorization_url =
            Some(Url::parse("https://idp.example.com/oauth/authorize").unwrap());
        config.client_store_temporary_credential = false;
        // explicit None — already the cfg_with_token_url default but
        // re-asserted here for clarity of intent.
        config.scope = None;

        let client = reqwest::Client::new();
        let acquired = run_authorization_code_flow(
            &client,
            &server_url(),
            &config,
            None,
            loopback_redirect_with("CODE"),
            false,
            None,
        )
        .await
        .expect("scope-less AC succeeds");
        assert_eq!(acquired.access_token.reveal(), "AT-SCOPELESS");
    }

    #[tokio::test]
    async fn refresh_token_grant_omits_scope_when_config_scope_is_none() {
        // Symmetric to the above for the refresh-token grant — the
        // `scope` field is only optional and should be skipped when not
        // set. Pinned here to lock the body shape across refactors.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(BodyMatcher(Box::new(|b: &str| !b.contains("scope="))))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"access_token":"AT-NEW","token_type":"Bearer"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
        let cache = StubTokenCache::new();
        token::store_oauth_refresh_token(token_url.as_str(), "alice", "RT-OLD", Some(&cache));
        let config = cfg_with_token_url(token_url);
        assert!(config.scope.is_none());
        let client = reqwest::Client::new();
        let acquired = run_oauth_authorization_code(
            &client,
            &server_url(),
            &config,
            Some(&cache),
            false,
            None,
        )
        .await
        .expect("refresh without scope succeeds");
        assert_eq!(acquired.access_token.reveal(), "AT-NEW");
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn flow_does_not_log_secrets_at_any_level() {
        // tracing_test::traced_test installs a dedicated subscriber and
        // captures all tracing output for the lifetime of the test. We
        // run a full happy-path AC exchange with deliberately distinctive
        // secret-shaped strings on every spot a leak could plausibly
        // occur (client secret, authorization code, token-endpoint
        // response body) and assert NONE of them appear in the captured
        // logs (cross-driver redaction requirement).
        const ACCESS_TOKEN: &str = "AT-LEAK-CANARY-AAAAAAAAAAAA";
        const REFRESH_TOKEN: &str = "RT-LEAK-CANARY-BBBBBBBBBBBB";
        const CLIENT_SECRET: &str = "client-secret-leak-canary-CCCCCC";
        const AUTH_CODE: &str = "auth-code-leak-canary-DDDDDDDD";

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                format!(
                    r#"{{"access_token":"{ACCESS_TOKEN}","refresh_token":"{REFRESH_TOKEN}","token_type":"Bearer"}}"#
                ),
                "application/json",
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/oauth/token", server.uri())).unwrap();
        let mut config = cfg_with_token_url(token_url);
        config.authorization_url =
            Some(Url::parse("https://idp.example.com/oauth/authorize").unwrap());
        config.client_store_temporary_credential = false;
        config.client_secret = CLIENT_SECRET.into();

        let client = reqwest::Client::new();
        let acquired = run_authorization_code_flow(
            &client,
            &server_url(),
            &config,
            None,
            loopback_redirect_with(AUTH_CODE),
            false,
            None,
        )
        .await
        .expect("happy path");
        assert_eq!(acquired.access_token.reveal(), ACCESS_TOKEN);

        // `logs_contain` is injected into scope by `#[traced_test]` and
        // reads the thread-local buffer that the macro's dedicated
        // subscriber populates.
        for needle in [ACCESS_TOKEN, REFRESH_TOKEN, CLIENT_SECRET, AUTH_CODE] {
            assert!(
                !logs_contain(needle),
                "secret value {needle:?} leaked into tracing output"
            );
        }
    }
}
