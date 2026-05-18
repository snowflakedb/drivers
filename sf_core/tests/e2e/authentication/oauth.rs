//! End-to-end OAuth tests against a real Snowflake account / IdP.
//!
//! These tests are feature-gated behind `auth_oauth_e2e` (the entire
//! module is `#[cfg(feature = "auth_oauth_e2e")] mod oauth;` in
//! `tests/e2e/authentication/mod.rs`) and `#[ignore]`d so they do not
//! run in CI by default — most need real OAuth client credentials and
//! at least one needs a live browser leg. They mirror the `vpn_*`
//! gating pattern from `native_okta.rs` and the feature-gating pattern
//! from `user_password_mfa.rs`, so a developer can opt in selectively.
//!
//! ## How to run
//!
//! ```bash
//! # 1. Make sure your parameters.json has the OAuth fields (see below).
//! export PARAMETER_PATH=/path/to/parameters.json
//!
//! # 2. Run all OAuth E2E tests (the `oauth_` prefix targets just this
//! #    module):
//! cargo test --package sf_core --features auth_oauth_e2e --release \
//!     --test e2e_tests -- --ignored oauth_
//!
//! # 3. Or pick a single test:
//! cargo test --package sf_core --features auth_oauth_e2e --release \
//!     --test e2e_tests -- --ignored \
//!     oauth_should_authenticate_with_pre_acquired_access_token
//! ```
//!
//! ## Required `parameters.json` fields (`testconnection` block)
//!
//! All `SNOWFLAKE_TEST_OAUTH_*` keys are optional; tests that need a
//! particular field either `expect()` it (hard fail when the parameter
//! is genuinely required) or `eprintln!` and early-return when the
//! parameter is only sometimes available (e.g. the CC-flow token URL,
//! which requires an external IdP). The cross-driver names follow
//! `analysis_feature_oauth.md` §9:
//!
//! | parameters.json key                           | drives                                       |
//! |-----------------------------------------------|----------------------------------------------|
//! | `SNOWFLAKE_TEST_OAUTH_CLIENT_ID`              | AC + CC: OAuth client id (required for both) |
//! | `SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET`          | AC + CC: OAuth client secret                 |
//! | `SNOWFLAKE_TEST_OAUTH_AUTHORIZATION_URL`      | AC: IdP authorization endpoint (optional;    |
//! |                                               | defaults to `https://{host}/oauth/authorize`)|
//! | `SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL`      | AC (optional) + CC (required): IdP token URL |
//! | `SNOWFLAKE_TEST_OAUTH_REDIRECT_URI`           | AC: loopback redirect URI (optional)         |
//! | `SNOWFLAKE_TEST_OAUTH_SCOPE`                  | AC + CC: requested OAuth scope (optional)    |
//! | `SNOWFLAKE_TEST_OAUTH_ACCESS_TOKEN`           | legacy `AUTHENTICATOR=OAUTH` flow            |

use crate::common::config::Parameters;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use sf_core::token_cache::{KeyringTokenCache, TokenCache, TokenType};

// =============================================================================
// Legacy `AUTHENTICATOR=OAUTH` (analysis §6 / §10.1)
// =============================================================================

#[test]
#[ignore = "OAuth E2E: requires SNOWFLAKE_TEST_OAUTH_ACCESS_TOKEN (run with --features auth_oauth_e2e)"]
fn oauth_should_authenticate_with_pre_acquired_access_token() {
    // Given Authentication is set to legacy OAUTH and a pre-acquired
    //       OAuth access token is supplied via `token=`
    let client = SnowflakeTestClient::with_default_params();
    let access_token = client
        .parameters
        .oauth_access_token
        .clone()
        .expect("SNOWFLAKE_TEST_OAUTH_ACCESS_TOKEN must be set for legacy OAUTH E2E test");

    set_legacy_oauth(&client);
    set_oauth_token(&client, &access_token);

    // When Trying to Connect
    let result = client.connect();

    // Then Login is successful and a simple query can be executed
    client.verify_simple_query(result);
}

#[test]
#[ignore = "OAuth E2E: legacy OAUTH negative path (run with --features auth_oauth_e2e)"]
fn oauth_should_fail_legacy_authentication_with_invalid_token() {
    // Given Authentication is set to legacy OAUTH and an invalid
    //       OAuth access token is supplied
    let client = SnowflakeTestClient::with_default_params();
    set_legacy_oauth(&client);
    set_oauth_token(&client, "invalid_oauth_token_12345");

    // When Trying to Connect
    let result = client.connect();

    // Then Connection fails with an authentication / login error
    client.assert_login_error(result);
}

// =============================================================================
// OAuth Authorization Code (AC) flow (analysis §3 / §10.2)
// =============================================================================

#[test]
#[ignore = "OAuth E2E: AC flow needs a real browser leg unless an OAuth access token is already cached in the keyring (run with --features auth_oauth_e2e)"]
fn oauth_should_authenticate_using_authorization_code_flow() {
    // Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a
    //       valid client id / secret. `oauth_authorization_url` and
    //       `oauth_token_request_url` are forwarded from parameters
    //       when present (otherwise the driver falls back to the
    //       Snowflake-IdP defaults `https://{host}/oauth/authorize`
    //       and `https://{host}/oauth/token-request`).
    //       `client_store_temporary_credential=true` lets the AC flow
    //       short-circuit on subsequent runs by re-using the cached
    //       access / refresh token (analysis §3.2 / §7).
    let client = SnowflakeTestClient::with_default_params();
    let client_id = client
        .parameters
        .oauth_client_id
        .clone()
        .expect("SNOWFLAKE_TEST_OAUTH_CLIENT_ID must be set for OAuth AC E2E test");
    let client_secret = client
        .parameters
        .oauth_client_secret
        .clone()
        .expect("SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET must be set for OAuth AC E2E test");

    set_authorization_code(&client);
    client.set_connection_option("oauth_client_id", &client_id);
    client.set_connection_option("oauth_client_secret", &client_secret);
    set_optional_oauth_endpoints(&client);
    client.set_connection_option_bool("client_store_temporary_credential", true);

    // When Trying to Connect (this will spawn the local-loopback HTTP
    // listener and `xdg-open`/`open`/`ShellExecute` the IdP login URL
    // unless a previously cached access token short-circuits the leg)
    let result = client.connect();

    // Then Login is successful and a simple query can be executed
    client.verify_simple_query(result);
}

#[test]
#[ignore = "OAuth E2E: AC short-circuit via cached access token (run with --features auth_oauth_e2e)"]
fn oauth_should_short_circuit_authorization_code_flow_with_cached_access_token() {
    // Given Authentication is set to OAUTH_AUTHORIZATION_CODE and a
    //       valid OAuth access token is pre-seeded in the OS keyring
    //       under the (host, user, OAUTH_ACCESS_TOKEN) cache key. The
    //       host is derived from `oauth_token_request_url` — falling
    //       back to the Snowflake server URL — exactly like
    //       `host_from_token_url` in production code (analysis §7.3).
    let client = SnowflakeTestClient::with_default_params();
    let client_id = client
        .parameters
        .oauth_client_id
        .clone()
        .expect("SNOWFLAKE_TEST_OAUTH_CLIENT_ID must be set for OAuth AC short-circuit test");
    let client_secret =
        client.parameters.oauth_client_secret.clone().expect(
            "SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET must be set for OAuth AC short-circuit test",
        );
    let access_token =
        client.parameters.oauth_access_token.clone().expect(
            "SNOWFLAKE_TEST_OAUTH_ACCESS_TOKEN must be set for OAuth AC short-circuit test",
        );
    let user = client
        .parameters
        .user
        .clone()
        .expect("SNOWFLAKE_TEST_USER must be set for OAuth AC short-circuit test");
    let cache_host = derive_cache_host(&client.parameters)
        .expect("expected to derive a cache host from oauth_token_request_url or server URL");

    // Pre-seed the OAuth access token so the AC flow skips the
    // browser leg entirely (mirroring the wiremock-driven
    // short-circuit test in `tests/integration/authentication/oauth.rs`
    // and the keyring-based pattern from
    // `tests/integration/authentication/user_password_mfa_token_cache.rs`).
    let cache = KeyringTokenCache::new().expect("keyring token cache should be available");
    cache
        .add_token(
            &cache_host,
            &user,
            TokenType::OAuthAccessToken,
            &access_token,
        )
        .expect("seed OAuth access token");

    set_authorization_code(&client);
    client.set_connection_option("oauth_client_id", &client_id);
    client.set_connection_option("oauth_client_secret", &client_secret);
    set_optional_oauth_endpoints(&client);
    client.set_connection_option_bool("client_store_temporary_credential", true);

    // When Trying to Connect — should NOT spawn a browser; the
    // pre-seeded access token must satisfy the AC short-circuit.
    let result = client.connect();

    // Then Login is successful and a simple query can be executed
    client.verify_simple_query(result);

    // Cleanup: remove the cache entries we seeded so the keyring
    // doesn't accumulate test artefacts even if the test panicked
    // earlier (best-effort; we ignore errors).
    let _ = cache.remove_token(&cache_host, &user, TokenType::OAuthAccessToken);
    let _ = cache.remove_token(&cache_host, &user, TokenType::OAuthRefreshToken);
}

// =============================================================================
// OAuth Client Credentials (CC) flow (analysis §4 / §10.3)
// =============================================================================

#[test]
#[ignore = "OAuth E2E: CC flow requires an external IdP and a configured oauth_token_request_url (run with --features auth_oauth_e2e)"]
fn oauth_should_authenticate_using_client_credentials_flow() {
    // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a
    //       valid client id / secret and an external IdP token URL.
    //       Snowflake's GS does not mint CC tokens (analysis §4), so
    //       `oauth_token_request_url` is required up-front.
    let client = SnowflakeTestClient::with_default_params();
    let client_id = client
        .parameters
        .oauth_client_id
        .clone()
        .expect("SNOWFLAKE_TEST_OAUTH_CLIENT_ID must be set for OAuth CC E2E test");
    let client_secret = client
        .parameters
        .oauth_client_secret
        .clone()
        .expect("SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET must be set for OAuth CC E2E test");
    let Some(token_url) = client.parameters.oauth_token_request_url.clone() else {
        eprintln!(
            "Skipping oauth_should_authenticate_using_client_credentials_flow: \
             SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL is not configured (CC requires an external IdP)"
        );
        return;
    };

    set_client_credentials(&client);
    client.set_connection_option("oauth_client_id", &client_id);
    client.set_connection_option("oauth_client_secret", &client_secret);
    client.set_connection_option("oauth_token_request_url", &token_url);
    if let Some(scope) = client.parameters.oauth_scope.clone() {
        client.set_connection_option("oauth_scope", &scope);
    }

    // When Trying to Connect
    let result = client.connect();

    // Then Login is successful and a simple query can be executed
    client.verify_simple_query(result);
}

// =============================================================================
// AC flow — bad client secret (analysis §13: IdP-error surface)
// =============================================================================

#[test]
#[ignore = "OAuth E2E: AC negative path with a bad client secret (run with --features auth_oauth_e2e)"]
fn oauth_should_fail_authorization_code_flow_with_bad_client_secret() {
    // Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a
    //       valid client id but a deliberately invalid client secret.
    //       The IdP token-exchange step must reject the credentials
    //       and the driver must surface an authentication / login
    //       error.
    let client = SnowflakeTestClient::with_default_params();
    let client_id = client
        .parameters
        .oauth_client_id
        .clone()
        .expect("SNOWFLAKE_TEST_OAUTH_CLIENT_ID must be set for OAuth AC negative E2E test");

    set_authorization_code(&client);
    client.set_connection_option("oauth_client_id", &client_id);
    client.set_connection_option("oauth_client_secret", "invalid_client_secret_12345"); // pragma: allowlist secret
    set_optional_oauth_endpoints(&client);
    client.set_connection_option_bool("client_store_temporary_credential", false);

    // When Trying to Connect
    let result = client.connect();

    // Then Connection fails with an authentication / login error
    client.assert_login_error(result);
}

// =============================================================================
// Helpers
// =============================================================================

fn set_legacy_oauth(client: &SnowflakeTestClient) {
    client.set_connection_option("authenticator", "OAUTH");
}

fn set_authorization_code(client: &SnowflakeTestClient) {
    client.set_connection_option("authenticator", "OAUTH_AUTHORIZATION_CODE");
}

fn set_client_credentials(client: &SnowflakeTestClient) {
    client.set_connection_option("authenticator", "OAUTH_CLIENT_CREDENTIALS");
}

fn set_oauth_token(client: &SnowflakeTestClient, token: &str) {
    client.set_connection_option("token", token);
}

/// Forward optional AC endpoints (`oauth_authorization_url`,
/// `oauth_token_request_url`, `oauth_redirect_uri`, `oauth_scope`) from
/// `parameters.json`. When the IdP is Snowflake itself the driver
/// derives the authorization / token URLs from the host, so leaving
/// these unset is valid.
fn set_optional_oauth_endpoints(client: &SnowflakeTestClient) {
    if let Some(url) = client.parameters.oauth_authorization_url.clone() {
        client.set_connection_option("oauth_authorization_url", &url);
    }
    if let Some(url) = client.parameters.oauth_token_request_url.clone() {
        client.set_connection_option("oauth_token_request_url", &url);
    }
    if let Some(uri) = client.parameters.oauth_redirect_uri.clone() {
        client.set_connection_option("oauth_redirect_uri", &uri);
    }
    if let Some(scope) = client.parameters.oauth_scope.clone() {
        client.set_connection_option("oauth_scope", &scope);
    }
}

/// Mirror of `sf_core::rest::snowflake::oauth::host_from_token_url`:
/// the OAuth token cache keys off the IdP token endpoint host when
/// available, otherwise the Snowflake server URL host (Python-style
/// `urlparse(token_request_url).hostname`, analysis §7.3). Kept private
/// to this test file so the production helper does not need to be
/// promoted to `pub`.
fn derive_cache_host(parameters: &Parameters) -> Option<String> {
    fn host_of(raw: &str) -> Option<String> {
        url::Url::parse(raw)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
    }
    if let Some(token_url) = parameters.oauth_token_request_url.as_deref()
        && let Some(host) = host_of(token_url)
    {
        return Some(host);
    }
    if let Some(server_url) = parameters.get_server_url()
        && let Some(host) = host_of(&server_url)
    {
        return Some(host);
    }
    parameters.host.clone()
}
