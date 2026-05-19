//! Wiremock-driven integration tests for the OAuth login pipeline.
//!
//! Exercises the three OAuth-shaped login methods that the universal
//! driver exposes today:
//!
//! * legacy pre-acquired access token (`AUTHENTICATOR=OAUTH`, raw `token=`),
//! * authorization code with PKCE and token caching (AC), and
//! * client credentials (CC).
//!
//! Cross-driver conventions referenced in this module:
//!
//! * §6 / §10.1 — legacy `OAUTH` body shape.
//! * §3.2 / §7   — AC cache short-circuit + refresh-token exchange.
//! * §8 / §14 #9 — refresh-on-failure for `390303` / `390318`.
//! * §4 / §14 #12 — CC tokens are intentionally never cached.
//! * §13         — IdP-error / missing-access-token surface area.
//!
//! Approach (per the integration-test plan): the AC interactive leg
//! would otherwise hang on the loopback redirect, so happy-path tests
//! pre-seed `KeyringTokenCache` with a cached access token (and / or
//! refresh token) so `acquire_authorization_code` short-circuits before
//! the loopback ever binds. Tests that intentionally exercise the
//! interactive leg are gated by `#[ignore]`.
//!
//! Cache hygiene: each test uses a unique username and removes the
//! tokens it seeded as part of teardown (mirroring
//! `user_password_mfa_token_cache.rs`).

use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::mocks::oauth;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;
use sf_core::token_cache::{KeyringTokenCache, TokenCache, TokenType};

// The OAuth Authorization Code flow's interactive leg would otherwise
// pop a real browser window against the wiremock IdP. We rely on the
// `cfg(any(test, feature = "test-utils"))` default in
// `OAuthAuthorizationCodeConfig::from_settings`, which installs a no-op
// browser launcher automatically when `sf_core` is built with the
// `test-utils` feature (as it is for the integration test binaries).

// =============================================================================
// Test Fixture
// =============================================================================

/// Reduces boilerplate for OAuth integration tests.  Holds a wiremock
/// server, a configured [`SnowflakeTestClient`], a [`KeyringTokenCache`]
/// handle, and the host used as the token-cache key (always `127.0.0.1`
/// for the wiremock-backed flow because that's what
/// `host_from_token_url` extracts from the configured `oauth_token_request_url`).
///
/// On `Drop` the fixture removes every token type it could conceivably
/// have seeded, scoped to the unique username — avoids cross-test
/// pollution if a test panics before its explicit teardown.
struct OAuthTestFixture {
    mock: MockServerWithTls,
    client: SnowflakeTestClient,
    cache: KeyringTokenCache,
    /// Host used as the token-cache key (always `127.0.0.1` for the
    /// wiremock-backed flow, which is what `host_from_token_url`
    /// extracts from the configured `oauth_token_request_url`).
    cache_host: String,
    user: String,
}

impl OAuthTestFixture {
    /// Build a fixture configured for the OAuth Authorization Code flow.
    /// The wiremock server hosts both the IdP token endpoint
    /// (`/oauth/token-request`) and the Snowflake login endpoint
    /// (`/session/v1/login-request`).
    fn with_authorization_code(user: &str) -> Self {
        let mock = MockServerWithTls::start();
        let token_url = format!("{}/oauth/token-request", mock.http_url());
        let auth_url = format!("{}/oauth/authorize", mock.http_url());

        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("authenticator", "OAUTH_AUTHORIZATION_CODE");
        client.set_connection_option("user", user);
        client.set_connection_option("oauth_client_id", "test-oauth-client-id");
        client.set_connection_option("oauth_client_secret", "test-oauth-client-secret"); // pragma: allowlist secret
        client.set_connection_option("oauth_token_request_url", &token_url);
        client.set_connection_option("oauth_authorization_url", &auth_url);
        client.set_connection_option("oauth_scope", "session:role:test_role");
        client.set_connection_option("client_store_temporary_credential", "true");

        let cache = KeyringTokenCache::new().expect("token cache should be available");
        let cache_host = host_from(&mock.http_url());

        Self {
            mock,
            client,
            cache,
            cache_host,
            user: user.to_string(),
        }
    }

    /// Build a fixture configured for the OAuth Client Credentials flow.
    fn with_client_credentials(user: &str) -> Self {
        let mock = MockServerWithTls::start();
        let token_url = format!("{}/oauth/token-request", mock.http_url());

        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("authenticator", "OAUTH_CLIENT_CREDENTIALS");
        client.set_connection_option("user", user);
        client.set_connection_option("oauth_client_id", "test-oauth-client-id");
        client.set_connection_option("oauth_client_secret", "test-oauth-client-secret"); // pragma: allowlist secret
        client.set_connection_option("oauth_token_request_url", &token_url);
        client.set_connection_option("oauth_scope", "session:role:test_role");

        let cache = KeyringTokenCache::new().expect("token cache should be available");
        let cache_host = host_from(&mock.http_url());

        Self {
            mock,
            client,
            cache,
            cache_host,
            user: user.to_string(),
        }
    }

    /// Build a fixture configured for the legacy pre-acquired token
    /// flow (`AUTHENTICATOR=OAUTH` + raw `token=`).
    fn with_legacy_oauth(user: &str, token: &str) -> Self {
        let mock = MockServerWithTls::start();

        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("authenticator", "OAUTH");
        client.set_connection_option("user", user);
        client.set_connection_option("token", token);

        let cache = KeyringTokenCache::new().expect("token cache should be available");
        let cache_host = host_from(&mock.http_url());

        Self {
            mock,
            client,
            cache,
            cache_host,
            user: user.to_string(),
        }
    }

    /// Set an arbitrary connection option.
    fn set_oauth_options(&self, options: &[(&str, &str)]) {
        for (k, v) in options {
            self.client.set_connection_option(k, v);
        }
    }

    fn seed_access_token(&self, value: &str) {
        self.cache
            .add_token(
                &self.cache_host,
                &self.user,
                TokenType::OAuthAccessToken,
                value,
            )
            .expect("seed OAuth access token");
    }

    fn seed_refresh_token(&self, value: &str) {
        self.cache
            .add_token(
                &self.cache_host,
                &self.user,
                TokenType::OAuthRefreshToken,
                value,
            )
            .expect("seed OAuth refresh token");
    }

    fn cached_access_token(&self) -> Option<String> {
        self.cache
            .get_token(&self.cache_host, &self.user, TokenType::OAuthAccessToken)
            .expect("get OAuth access token")
    }

    fn cached_refresh_token(&self) -> Option<String> {
        self.cache
            .get_token(&self.cache_host, &self.user, TokenType::OAuthRefreshToken)
            .expect("get OAuth refresh token")
    }

    fn connect(&self) -> Result<(), String> {
        self.client.connect()
    }

    fn assert_success(result: Result<(), String>, context: &str) {
        assert!(result.is_ok(), "Expected {context}, got: {result:?}");
    }

    // TODO(SNOW-3549115): Replace string-pattern matching against error
    // messages with structured matching against an `AuthenticationError`
    // enum. Matching on substrings is brittle and risks getting
    // cargo-culted to other auth tests.
    fn assert_error(result: Result<(), String>, patterns: &[&str], context: &str) {
        let error = result.expect_err(&format!("Expected {context} to fail"));
        let matches = patterns.iter().any(|p| error.contains(p));
        assert!(matches, "Expected {context}, got: {error}");
    }
}

impl Drop for OAuthTestFixture {
    fn drop(&mut self) {
        // Remove every token type we could plausibly have seeded so a
        // panicking test does not pollute the OS keyring (mirrors
        // `user_password_mfa_token_cache.rs`).
        for &tt in &[
            TokenType::OAuthAccessToken,
            TokenType::OAuthRefreshToken,
            TokenType::DpopBundledAccessToken,
        ] {
            let _ = self.cache.remove_token(&self.cache_host, &self.user, tt);
        }
    }
}

/// Extract the bare host (no port) from a URL string. Mirrors
/// `host_from_token_url` in `sf_core::rest::snowflake::oauth::token`,
/// which the production code uses to derive the cache key (prefers
/// IdP token URL host, falls back to Snowflake host).
fn host_from(url: &str) -> String {
    url::Url::parse(url)
        .expect("valid mock URL")
        .host_str()
        .expect("mock URL must have a host")
        .to_string()
}

/// Build a unique username scoped to the running test, so concurrent
/// runs of the suite do not race on the OS keyring (mirrors the
/// uniqueness convention in `user_password_mfa_token_cache.rs`).
fn unique_user(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix}_{nanos}")
}

fn count_token_endpoint_requests(mock: &MockServerWithTls) -> usize {
    mock.received_requests()
        .iter()
        .filter(|r| r.url.path() == "/oauth/token-request" && r.method.as_str() == "POST")
        .count()
}

fn token_endpoint_bodies(mock: &MockServerWithTls) -> Vec<String> {
    mock.received_requests()
        .iter()
        .filter(|r| r.url.path() == "/oauth/token-request" && r.method.as_str() == "POST")
        .map(|r| String::from_utf8_lossy(&r.body).to_string())
        .collect()
}

// =============================================================================
// Legacy `AUTHENTICATOR=OAUTH` + raw `token=`
// =============================================================================

#[test]
fn should_login_with_legacy_oauth_using_pre_acquired_token() {
    // Given Wiremock is running and a fixture is configured with the
    // legacy OAUTH authenticator and a pre-acquired access token
    let user = unique_user("oauth_legacy_ok");
    let token = "legacy-pre-acquired-access-token";
    let fixture = OAuthTestFixture::with_legacy_oauth(&user, token);
    fixture
        .mock
        .mount(oauth::snowflake_login_success_oauth(token));

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login is successful
    OAuthTestFixture::assert_success(result, "legacy OAUTH login to succeed");
    // And the IdP token endpoint must NOT have been hit (legacy flow
    // forwards the caller-supplied token unchanged).
    assert_eq!(
        count_token_endpoint_requests(&fixture.mock),
        0,
        "legacy OAUTH must not call the IdP token endpoint"
    );
}

#[test]
fn should_fail_legacy_oauth_when_snowflake_returns_390303() {
    // Given Wiremock is running with a Snowflake login that returns 390303
    let user = unique_user("oauth_legacy_390303");
    let fixture = OAuthTestFixture::with_legacy_oauth(&user, "legacy-bad-token");
    fixture
        .mock
        .mount(oauth::snowflake_login_oauth_access_token_invalid_390303());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails with a login error (legacy flow has no
    // cache to evict, so the error is surfaced unchanged — legacy
    // flow has no refresh-on-failure semantics).
    OAuthTestFixture::assert_error(
        result,
        &["390303", "OAuth", "login", "auth"],
        "legacy OAUTH 390303 failure",
    );
}

// =============================================================================
// Authorization Code — cache short-circuit (preferred happy path)
// =============================================================================

#[test]
fn should_login_with_authorization_code_using_cached_access_token() {
    // Given Wiremock is running and an OAuth access token is already
    // cached for the user (so the AC flow short-circuits before binding
    // the loopback — AC state machine short-circuits on cache hit)
    let user = unique_user("oauth_ac_cached_at");
    let cached_at = "ac-cached-access-token-canary";
    let fixture = OAuthTestFixture::with_authorization_code(&user);
    fixture.seed_access_token(cached_at);
    fixture
        .mock
        .mount(oauth::snowflake_login_success_oauth(cached_at));

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login is successful and no IdP token request was issued
    OAuthTestFixture::assert_success(result, "AC cached-AT login to succeed");
    assert_eq!(
        count_token_endpoint_requests(&fixture.mock),
        0,
        "cached-AT short-circuit must NOT call the IdP token endpoint"
    );
}

#[test]
fn should_login_with_authorization_code_using_cached_refresh_token() {
    // Given Wiremock is running with the IdP refresh-token endpoint
    // mounted, only a refresh token is cached for the user, and the
    // Snowflake login accepts the refreshed access token
    let user = unique_user("oauth_ac_cached_rt");
    let fixture = OAuthTestFixture::with_authorization_code(&user);
    fixture.seed_refresh_token("ac-cached-refresh-token");
    fixture
        .mock
        .mount(oauth::idp_token_endpoint_success_refresh());
    fixture.mock.mount(oauth::snowflake_login_success_oauth(
        "ac-access-token-refreshed",
    ));

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login is successful, the IdP refresh endpoint was hit
    // exactly once, and the rotated tokens replaced the cached values
    // (refresh-token rotation persists rotated tokens).
    OAuthTestFixture::assert_success(result, "AC cached-RT login to succeed");
    assert_eq!(
        count_token_endpoint_requests(&fixture.mock),
        1,
        "expected exactly one IdP refresh request"
    );
    let stored_at = fixture.cached_access_token();
    assert_eq!(
        stored_at.as_deref(),
        Some("ac-access-token-refreshed"),
        "fresh access token must be persisted to the cache"
    );
    let stored_rt = fixture.cached_refresh_token();
    assert_eq!(
        stored_rt.as_deref(),
        Some("ac-refresh-token-rotated"),
        "rotated refresh token must replace the cached one"
    );
}

// =============================================================================
// Authorization Code — refresh-on-failure (390303 / 390318)
// =============================================================================

#[test]
fn should_evict_cached_at_and_retry_via_refresh_token_on_390303() {
    // Given Wiremock returns 390303 once, then accepts the retry, the
    // IdP refresh endpoint is mounted, and both AT + RT are pre-seeded
    let user = unique_user("oauth_ac_390303");
    let stale_at = "ac-stale-access-token";
    let fixture = OAuthTestFixture::with_authorization_code(&user);
    fixture.seed_access_token(stale_at);
    fixture.seed_refresh_token("ac-cached-refresh-token");
    fixture
        .mock
        .mount(oauth::snowflake_login_oauth_then_success("390303"));
    fixture
        .mock
        .mount(oauth::idp_token_endpoint_success_refresh());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login eventually succeeds (after eviction + refresh +
    // retry — 390303/390318 refresh-on-failure), and the originally seeded AT is
    // no longer present in the cache.
    OAuthTestFixture::assert_success(result, "AC 390303 retry to succeed");
    let stored_at = fixture.cached_access_token();
    assert_ne!(
        stored_at.as_deref(),
        Some(stale_at),
        "stale access token must be evicted after 390303"
    );
    assert_eq!(
        stored_at.as_deref(),
        Some("ac-access-token-refreshed"),
        "fresh access token must be persisted after the retry"
    );
}

#[test]
fn should_evict_cached_at_and_retry_via_refresh_token_on_390318() {
    // Given Wiremock returns 390318 once, then accepts the retry, the
    // IdP refresh endpoint is mounted, and both AT + RT are pre-seeded
    let user = unique_user("oauth_ac_390318");
    let stale_at = "ac-expired-access-token";
    let fixture = OAuthTestFixture::with_authorization_code(&user);
    fixture.seed_access_token(stale_at);
    fixture.seed_refresh_token("ac-cached-refresh-token");
    fixture
        .mock
        .mount(oauth::snowflake_login_oauth_then_success("390318"));
    fixture
        .mock
        .mount(oauth::idp_token_endpoint_success_refresh());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login eventually succeeds and the seeded AT was evicted
    OAuthTestFixture::assert_success(result, "AC 390318 retry to succeed");
    let stored_at = fixture.cached_access_token();
    assert_ne!(
        stored_at.as_deref(),
        Some(stale_at),
        "expired access token must be evicted after 390318"
    );
    assert_eq!(
        stored_at.as_deref(),
        Some("ac-access-token-refreshed"),
        "fresh access token must be persisted after the retry"
    );
}

// =============================================================================
// Authorization Code — single-use refresh tokens
// =============================================================================

#[test]
fn should_omit_single_use_refresh_flag_from_refresh_grant_body() {
    // Given the AC fixture has `oauth_enable_single_use_refresh_tokens=true`
    // configured, only a refresh token is cached, and the IdP refresh +
    // Snowflake login mocks are mounted
    let user = unique_user("oauth_ac_single_use_rt");
    let fixture = OAuthTestFixture::with_authorization_code(&user);
    fixture.set_oauth_options(&[("oauth_enable_single_use_refresh_tokens", "true")]);
    fixture.seed_refresh_token("ac-cached-refresh-token");
    fixture
        .mock
        .mount(oauth::idp_token_endpoint_success_refresh());
    fixture.mock.mount(oauth::snowflake_login_success_oauth(
        "ac-access-token-refreshed",
    ));

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login succeeds and the refresh-grant body did NOT carry the
    // single-use flag (the flag is only meaningful on the AC token
    // exchange — `authorization_code.rs` adds it to that grant only,
    // single-use flag is only meaningful on the AC token exchange).
    OAuthTestFixture::assert_success(result, "single-use refresh grant to succeed");
    let bodies = token_endpoint_bodies(&fixture.mock);
    assert_eq!(bodies.len(), 1, "expected exactly one IdP token request");
    assert!(
        bodies[0].contains("grant_type=refresh_token"),
        "refresh grant must carry grant_type=refresh_token, got: {body}",
        body = bodies[0]
    );
    assert!(
        !bodies[0].contains("enable_single_use_refresh_tokens"),
        "refresh grant must NOT include the single-use flag, got: {body}",
        body = bodies[0]
    );
}

// =============================================================================
// Authorization Code — IdP error responses
// =============================================================================

#[test]
fn should_evict_refresh_token_when_idp_returns_invalid_grant() {
    // Given a refresh token is cached, the IdP refuses with
    // `invalid_grant`, and the AC `authentication_timeout` is short
    // enough to surface BrowserTimeout fast when the flow falls
    // through to the interactive leg.
    let user = unique_user("oauth_ac_invalid_grant");
    let fixture = OAuthTestFixture::with_authorization_code(&user);
    fixture.set_oauth_options(&[("authentication_timeout", "1")]);
    fixture.seed_refresh_token("ac-cached-refresh-token");
    fixture
        .mock
        .mount(oauth::idp_token_endpoint_refresh_failed());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails (interactive leg times out without a
    // browser) and the refresh token is evicted from the cache. Per
    // `authorization_code.rs`, an IdP refresh-exchange failure evicts
    // the cached refresh token before falling through to the
    // interactive leg (evicts RT, then falls back to full flow).
    OAuthTestFixture::assert_error(
        result,
        &[
            "BrowserTimeout",
            "OAuthFlow",
            "OAuth",
            "timed out",
            "timeout",
        ],
        "AC interactive leg to time out after refresh failure",
    );
    let stored_rt = fixture.cached_refresh_token();
    assert!(
        stored_rt.is_none(),
        "refresh token must be evicted after invalid_grant; cache still holds {stored_rt:?}"
    );
}

// =============================================================================
// Client Credentials — happy path + error paths
// =============================================================================

#[test]
fn should_login_with_client_credentials_using_external_idp() {
    // Given the CC fixture is wired with mock IdP token endpoint and
    // Snowflake login mocks
    let user = unique_user("oauth_cc_ok");
    let fixture = OAuthTestFixture::with_client_credentials(&user);
    fixture
        .mock
        .mount(oauth::idp_token_endpoint_success_client_credentials());
    fixture.mock.mount(oauth::snowflake_login_success_oauth(
        "cc-access-token-success",
    ));

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login is successful and CC tokens are NOT cached
    // (CC is stateless by design — tokens are never cached).
    OAuthTestFixture::assert_success(result, "CC happy path to succeed");
    assert!(
        fixture.cached_access_token().is_none(),
        "CC must not cache access tokens"
    );
    assert!(
        fixture.cached_refresh_token().is_none(),
        "CC must not cache refresh tokens"
    );
}

#[test]
fn should_fail_client_credentials_when_idp_returns_500() {
    // Given the CC fixture is wired with an IdP that returns 500
    let user = unique_user("oauth_cc_500");
    let fixture = OAuthTestFixture::with_client_credentials(&user);
    fixture
        .mock
        .mount(oauth::idp_token_endpoint_token_request_error());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails with a transport / token-exchange error and
    // the cache is untouched (CC never writes to it)
    OAuthTestFixture::assert_error(
        result,
        &["TokenExchange", "OAuth", "500", "OAuthFlow"],
        "CC IdP 500 error",
    );
    assert!(
        fixture.cached_access_token().is_none(),
        "CC IdP failure must not have written anything to the cache"
    );
}

#[test]
fn should_fail_client_credentials_when_idp_returns_invalid_scope() {
    // Given the CC fixture is wired with an IdP that returns
    // `invalid_scope`
    let user = unique_user("oauth_cc_invalid_scope");
    let fixture = OAuthTestFixture::with_client_credentials(&user);
    fixture
        .mock
        .mount(oauth::idp_token_endpoint_invalid_scope());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails with a redacted IdP-error message that
    // names the OAuth error code but never echoes the client secret
    // (redacted IdP error; client secret must never leak). The error class is `OAuthFlow → IdpError`.
    let err = result.expect_err("CC invalid_scope must fail");
    let pattern_hit = [
        "IdpError",
        "invalid_scope",
        "Identity Provider",
        "OAuthFlow",
    ]
    .iter()
    .any(|p| err.contains(p));
    assert!(pattern_hit, "expected IdP error message, got: {err}");
    assert!(
        !err.contains("test-oauth-client-secret"),
        "client secret must never appear in an OAuth error: {err}"
    );
}

#[test]
fn should_fail_client_credentials_when_idp_response_is_missing_access_token() {
    // Given the CC fixture is wired with an IdP that returns 200 but no
    // `access_token`
    let user = unique_user("oauth_cc_missing_at");
    let fixture = OAuthTestFixture::with_client_credentials(&user);
    fixture
        .mock
        .mount(oauth::idp_token_endpoint_missing_access_token());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails with `MissingAccessToken` (
    // empty access tokens are treated as missing to avoid forwarding
    // a useless Bearer token to GS).
    OAuthTestFixture::assert_error(
        result,
        &["MissingAccessToken", "access_token", "OAuthFlow", "OAuth"],
        "CC missing access_token error",
    );
}
