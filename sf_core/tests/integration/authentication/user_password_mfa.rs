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
        let mock = MockServerWithTls::start();

        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("authenticator", "USERNAME_PASSWORD_MFA");
        client.set_connection_option("user", "test_user");
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
    let fixture = MfaTestFixture::new();
    fixture.set_option("client_store_temporary_credential", "true");
    fixture.mock.mount(mfa::login_success_with_cached_token());

    let cache = KeyringTokenCache::new().expect("token cache should be available");
    let host = url::Url::parse(&fixture.mock.http_url())
        .unwrap()
        .host_str()
        .unwrap()
        .to_string();
    cache
        .add_token(&host, "test_user", TokenType::MfaToken, "cached_mfa_token")
        .expect("failed to seed token cache");

    let result = fixture.connect();

    fixture.expecting_success_result(result, "MFA cached token login to succeed");

    let _ = cache.remove_token(&host, "test_user", TokenType::MfaToken);
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
    let password = client.parameters.password.clone().unwrap();
    client.set_connection_option("password", &password); // pragma: allowlist secret

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned
    client.assert_missing_parameter_error(result);
}

#[test]
fn should_fail_authentication_when_password_is_not_provided() {
    //Given Authentication is set to username_password_mfa and password is not provided
    let client = SnowflakeTestClient::with_int_tests_params(None);
    client.set_connection_option("authenticator", "USERNAME_PASSWORD_MFA");

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
