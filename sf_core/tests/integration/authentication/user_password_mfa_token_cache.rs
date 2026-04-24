use crate::common::mocks::mfa;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;
use sf_core::token_cache::{KeyringTokenCache, TokenCache, TokenType};

// =============================================================================
// Test Fixture - Reduces boilerplate for MFA token cache integration tests
// =============================================================================

struct MfaTestFixture {
    mock: MockServerWithTls,
    client: SnowflakeTestClient,
}

impl MfaTestFixture {
    fn with_user(user: &str) -> Self {
        let mock = MockServerWithTls::start();

        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("authenticator", "USERNAME_PASSWORD_MFA");
        client.set_connection_option("user", user);
        client.set_connection_option("password", "test_password"); // pragma: allowlist secret

        Self { mock, client }
    }

    fn set_option(&self, key: &str, value: &str) -> &Self {
        self.client.set_connection_option(key, value);
        self
    }

    fn connect(&self) -> Result<(), String> {
        self.client.connect()
    }

    fn expecting_success_result(&self, result: Result<(), String>, context: &str) {
        assert!(result.is_ok(), "Expected {context}, got: {result:?}");
    }

    fn expecting_error_result(&self, patterns: &[&str], result: Result<(), String>, context: &str) {
        let error = result.expect_err(&format!("Expected {context} to fail"));
        let matches = patterns.iter().any(|p| error.contains(p));
        assert!(matches, "Expected {context}, got: {error}");
    }
}

// =============================================================================
// Wiremock-based MFA tests - cached MFA token flow
// =============================================================================

#[test]
fn should_authenticate_with_cached_mfa_token_via_wiremock() {
    // Given Wiremock is running and Wiremock has MFA login success mapping with cached token and MFA token is pre-seeded in the token cache
    let user = "mfa_cache_user";
    let fixture = MfaTestFixture::with_user(user);
    fixture.set_option("client_store_temporary_credential", "true");
    fixture.mock.mount(mfa::login_success_with_cached_token());

    let cache = KeyringTokenCache::new().expect("token cache should be available");
    let host = url::Url::parse(&fixture.mock.http_url())
        .expect("mock URL should be valid")
        .host_str()
        .expect("mock URL should have a host")
        .to_string();
    cache
        .add_token(&host, user, TokenType::MfaToken, "cached_mfa_token")
        .expect("failed to seed token cache");

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login is successful
    fixture.expecting_success_result(result, "MFA cached token login to succeed");

    let _ = cache.remove_token(&host, user, TokenType::MfaToken);
}

// =============================================================================
// Wiremock-based MFA tests - EXT_AUTHN error codes evict cached MFA token
// =============================================================================

fn assert_ext_authn_error_evicts_cached_mfa_token(
    mount_mock: fn() -> wiremock::Mock,
    code: &str,
    user: &str,
) {
    let fixture = MfaTestFixture::with_user(user);
    fixture.set_option("client_store_temporary_credential", "true");
    fixture.mock.mount(mount_mock());
    fixture.mock.mount(mfa::login_failure_duo_push());

    let cache = KeyringTokenCache::new().expect("token cache should be available");
    let host = url::Url::parse(&fixture.mock.http_url())
        .expect("mock URL should be valid")
        .host_str()
        .expect("mock URL should have a host")
        .to_string();
    cache
        .add_token(&host, user, TokenType::MfaToken, "cached_mfa_token")
        .expect("failed to seed token cache");

    let result = fixture.connect();

    fixture.expecting_error_result(
        &["login", "auth", "LoginError", "AuthError", "390100"],
        result,
        &format!("login error after retry for EXT_AUTHN code {code}"),
    );

    let cached = cache
        .get_token(&host, user, TokenType::MfaToken)
        .expect("get_token should not fail");
    assert!(
        cached.is_none(),
        "Expected cached MFA token to be removed after EXT_AUTHN error {code}, but it still exists"
    );
}

#[test]
fn should_evict_cached_mfa_token_on_ext_authn_denied() {
    // Given Wiremock is running with EXT_AUTHN denied mapping and MFA token is pre-seeded in the token cache
    let mount_mock = mfa::login_failure_ext_authn_denied;
    // When Trying to Connect
    let code = "390120";
    // Then Connection fails with login error and Cached MFA token is evicted from the cache
    assert_ext_authn_error_evicts_cached_mfa_token(mount_mock, code, "mfa_evict_denied");
}

#[test]
fn should_evict_cached_mfa_token_on_ext_authn_locked() {
    // Given Wiremock is running with EXT_AUTHN locked mapping and MFA token is pre-seeded in the token cache
    let mount_mock = mfa::login_failure_ext_authn_locked;
    // When Trying to Connect
    let code = "390123";
    // Then Connection fails with login error and Cached MFA token is evicted from the cache
    assert_ext_authn_error_evicts_cached_mfa_token(mount_mock, code, "mfa_evict_locked");
}

#[test]
fn should_evict_cached_mfa_token_on_ext_authn_timeout() {
    // Given Wiremock is running with EXT_AUTHN timeout mapping and MFA token is pre-seeded in the token cache
    let mount_mock = mfa::login_failure_ext_authn_timeout;
    // When Trying to Connect
    let code = "390126";
    // Then Connection fails with login error and Cached MFA token is evicted from the cache
    assert_ext_authn_error_evicts_cached_mfa_token(mount_mock, code, "mfa_evict_timeout");
}

#[test]
fn should_evict_cached_mfa_token_on_ext_authn_invalid() {
    // Given Wiremock is running with EXT_AUTHN invalid mapping and MFA token is pre-seeded in the token cache
    let mount_mock = mfa::login_failure_ext_authn_invalid;
    // When Trying to Connect
    let code = "390127";
    // Then Connection fails with login error and Cached MFA token is evicted from the cache
    assert_ext_authn_error_evicts_cached_mfa_token(mount_mock, code, "mfa_evict_invalid");
}

#[test]
fn should_evict_cached_mfa_token_on_ext_authn_exception() {
    // Given Wiremock is running with EXT_AUTHN exception mapping and MFA token is pre-seeded in the token cache
    let mount_mock = mfa::login_failure_ext_authn_exception;
    // When Trying to Connect
    let code = "390129";
    // Then Connection fails with login error and Cached MFA token is evicted from the cache
    assert_ext_authn_error_evicts_cached_mfa_token(mount_mock, code, "mfa_evict_exception");
}

// =============================================================================
// Wiremock-based MFA tests - EXT_AUTHN retry with DUO push fallback
// =============================================================================

#[test]
fn should_retry_with_duo_push_when_cached_mfa_token_fails_ext_authn() {
    // Given Wiremock is running with EXT_AUTHN denied mapping and DUO push success mapping and MFA token is pre-seeded in the token cache
    let user = "mfa_retry_success";
    let fixture = MfaTestFixture::with_user(user);
    fixture.set_option("client_store_temporary_credential", "true");

    fixture.mock.mount(mfa::login_failure_ext_authn_denied());
    fixture.mock.mount(mfa::login_success_with_mfa_token());

    let cache = KeyringTokenCache::new().expect("token cache should be available");
    let host = url::Url::parse(&fixture.mock.http_url())
        .expect("mock URL should be valid")
        .host_str()
        .expect("mock URL should have a host")
        .to_string();
    cache
        .add_token(&host, user, TokenType::MfaToken, "cached_mfa_token")
        .expect("failed to seed token cache");

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login is successful
    fixture.expecting_success_result(result, "MFA login retry via DUO push to succeed");

    let cached = cache
        .get_token(&host, user, TokenType::MfaToken)
        .expect("get_token should not fail");
    assert!(
        cached.is_some(),
        "Expected new MFA token to be cached after successful retry"
    );

    let _ = cache.remove_token(&host, user, TokenType::MfaToken);
}

#[test]
fn should_fail_with_retry_error_when_both_cached_token_and_duo_push_fail() {
    // Given Wiremock is running with EXT_AUTHN denied mapping and DUO push failure mapping and MFA token is pre-seeded in the token cache
    let user = "mfa_retry_fail";
    let fixture = MfaTestFixture::with_user(user);
    fixture.set_option("client_store_temporary_credential", "true");

    fixture.mock.mount(mfa::login_failure_ext_authn_denied());
    fixture.mock.mount(mfa::login_failure_duo_push());

    let cache = KeyringTokenCache::new().expect("token cache should be available");
    let host = url::Url::parse(&fixture.mock.http_url())
        .expect("mock URL should be valid")
        .host_str()
        .expect("mock URL should have a host")
        .to_string();
    cache
        .add_token(&host, user, TokenType::MfaToken, "cached_mfa_token")
        .expect("failed to seed token cache");

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails with login error
    fixture.expecting_error_result(
        &["login", "auth", "LoginError", "AuthError", "390100"],
        result,
        "login error from retry (not EXT_AUTHN code)",
    );

    let cached = cache
        .get_token(&host, user, TokenType::MfaToken)
        .expect("get_token should not fail");
    assert!(
        cached.is_none(),
        "Expected cached MFA token to be removed after EXT_AUTHN error and failed retry"
    );
}
