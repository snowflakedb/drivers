use crate::common::mocks::mfa;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;
use sf_core::token_cache::{KeyringTokenCache, TokenCache, TokenType};

// =============================================================================
// Test Fixture - Reduces boilerplate for MFA integration tests
// =============================================================================

struct MfaTestFixture {
    mock: MockServerWithTls,
    client: SnowflakeTestClient,
}

impl MfaTestFixture {
    fn new() -> Self {
        Self::with_user("test_user")
    }

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
// Wiremock-based MFA tests - DUO push flow
// =============================================================================

#[test]
fn should_authenticate_with_mfa_duo_push_via_wiremock() {
    // Given Wiremock is running and Wiremock has MFA login success mapping with DUO push
    let fixture = MfaTestFixture::new();

    // And Snowflake client is configured for USERNAME_PASSWORD_MFA
    fixture.mock.mount(mfa::login_success_with_mfa_token());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login is successful
    fixture.expecting_success_result(result, "MFA DUO push login to succeed");
}

// =============================================================================
// Wiremock-based MFA tests - TOTP passcode flow
// =============================================================================

#[test]
fn should_authenticate_with_mfa_totp_passcode_via_wiremock() {
    // Given Wiremock is running and Wiremock has MFA login success mapping with passcode
    let fixture = MfaTestFixture::new();

    // And Snowflake client is configured for USERNAME_PASSWORD_MFA with passcode
    fixture.set_option("passcode", "123456");
    fixture.mock.mount(mfa::login_success_with_passcode());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login is successful
    fixture.expecting_success_result(result, "MFA TOTP passcode login to succeed");
}

// =============================================================================
// Wiremock-based MFA tests - passcode-in-password flow
// =============================================================================

#[test]
fn should_authenticate_with_mfa_passcode_in_password_via_wiremock() {
    // Given Wiremock is running and Wiremock has MFA login success mapping for passcode-in-password
    let fixture = MfaTestFixture::new();

    // And Snowflake client is configured with passcodeInPassword=true and passcode appended to password
    fixture.set_option("password", "test_password123456"); // pragma: allowlist secret
    fixture.set_option("passcodeInPassword", "true");
    fixture
        .mock
        .mount(mfa::login_success_passcode_in_password());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login is successful
    fixture.expecting_success_result(result, "MFA passcode-in-password login to succeed");
}

// =============================================================================
// Wiremock-based MFA tests - wrong password
// =============================================================================

#[test]
fn should_fail_mfa_authentication_when_wrong_password_is_provided_via_wiremock() {
    // Given Wiremock is running and Wiremock has MFA login failure mapping
    let fixture = MfaTestFixture::new();

    // And Snowflake client is configured for USERNAME_PASSWORD_MFA with invalid password
    fixture.set_option("password", "wrong_password"); // pragma: allowlist secret
    fixture.mock.mount(mfa::login_failure());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails with login error
    fixture.expecting_error_result(
        &[
            "login",
            "auth",
            "LoginError",
            "AuthError",
            "390100",
            "Incorrect",
        ],
        result,
        "login error for wrong password",
    );
}

// =============================================================================
// Wiremock-based MFA tests - cached MFA token flow
// =============================================================================

#[test]
fn should_authenticate_with_cached_mfa_token_via_wiremock() {
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

    let result = fixture.connect();

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
    assert_ext_authn_error_evicts_cached_mfa_token(
        mfa::login_failure_ext_authn_denied,
        "390120",
        "mfa_evict_denied",
    );
}

#[test]
fn should_evict_cached_mfa_token_on_ext_authn_locked() {
    assert_ext_authn_error_evicts_cached_mfa_token(
        mfa::login_failure_ext_authn_locked,
        "390123",
        "mfa_evict_locked",
    );
}

#[test]
fn should_evict_cached_mfa_token_on_ext_authn_timeout() {
    assert_ext_authn_error_evicts_cached_mfa_token(
        mfa::login_failure_ext_authn_timeout,
        "390126",
        "mfa_evict_timeout",
    );
}

#[test]
fn should_evict_cached_mfa_token_on_ext_authn_invalid() {
    assert_ext_authn_error_evicts_cached_mfa_token(
        mfa::login_failure_ext_authn_invalid,
        "390127",
        "mfa_evict_invalid",
    );
}

#[test]
fn should_evict_cached_mfa_token_on_ext_authn_exception() {
    assert_ext_authn_error_evicts_cached_mfa_token(
        mfa::login_failure_ext_authn_exception,
        "390129",
        "mfa_evict_exception",
    );
}

// =============================================================================
// Wiremock-based MFA tests - EXT_AUTHN retry with DUO push fallback
// =============================================================================

#[test]
fn should_retry_with_duo_push_when_cached_mfa_token_fails_ext_authn() {
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

    let result = fixture.connect();

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

    let result = fixture.connect();

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

// =============================================================================
// Parameter Validation - Missing user/password
// =============================================================================

#[test]
fn should_fail_authentication_when_user_is_not_provided() {
    //Given Authentication is set to username_password_mfa and user is not provided
    let client = SnowflakeTestClient::with_int_tests_params(None);
    client.set_connection_option("authenticator", "USERNAME_PASSWORD_MFA");
    client.set_connection_option("user", "");
    client.set_connection_option("password", "dummy_password"); // pragma: allowlist secret

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned
    client.assert_missing_parameter_error(result);
}

#[test]
fn should_fail_authentication_when_password_is_implicitly_unset() {
    //Given Authentication is set to username_password_mfa and password is not provided
    let client = SnowflakeTestClient::with_int_tests_params(None);
    client.set_connection_option("authenticator", "USERNAME_PASSWORD_MFA");

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned
    client.assert_missing_parameter_error(result);
}

#[test]
fn should_fail_authentication_when_password_is_explicitly_empty() {
    //Given Authentication is set to username_password_mfa and password is explicitly set to empty
    let client = SnowflakeTestClient::with_int_tests_params(None);
    client.set_connection_option("authenticator", "USERNAME_PASSWORD_MFA");
    client.set_connection_option("password", ""); // pragma: allowlist secret

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned
    client.assert_missing_parameter_error(result);
}

#[test]
fn should_fail_authentication_when_passcode_in_password_is_not_set_but_passcode_is_appended_to_password()
 {
    //Given Authentication is set to username_password_mfa and user, password with appended passcode are provided and passcodeInPassword is not set
    let fixture = MfaTestFixture::new();
    let password_with_passcode = "test_password123456"; // pragma: allowlist secret
    fixture.set_option("password", password_with_passcode);
    fixture.mock.mount(mfa::login_failure());

    //When Trying to Connect
    let result = fixture.connect();

    //Then There is error returned
    fixture.expecting_error_result(
        &[
            "login",
            "auth",
            "LoginError",
            "AuthError",
            "390100",
            "Incorrect",
        ],
        result,
        "login error when passcodeInPassword not set",
    );
}

#[test]
fn should_fail_authentication_when_passcode_in_password_is_set_but_passcode_is_not_appended_to_password()
 {
    //Given Authentication is set to username_password_mfa and user, password are provided and passcodeInPassword is set but passcode is not appended to password
    let fixture = MfaTestFixture::new();
    fixture.set_option("passcodeInPassword", "true");
    fixture.mock.mount(mfa::login_failure());

    //When Trying to Connect
    let result = fixture.connect();

    //Then There is error returned
    fixture.expecting_error_result(
        &[
            "login",
            "auth",
            "LoginError",
            "AuthError",
            "390100",
            "Incorrect",
        ],
        result,
        "login error when passcode not appended",
    );
}
