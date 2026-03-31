use crate::common::mocks::okta;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;

// =============================================================================
// Test Fixture - Reduces boilerplate for Native Okta integration tests
// =============================================================================

struct OktaTestFixture {
    mock: MockServerWithTls,
    client: SnowflakeTestClient,
}

impl OktaTestFixture {
    fn new() -> Self {
        let mock = MockServerWithTls::start();

        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("authenticator", &mock.https_url());
        client.set_connection_option("user", "test_user");
        client.set_connection_option("password", "test_password");
        client.set_connection_option("verify_certificates", "false");
        client.set_connection_option("verify_hostname", "false");

        Self { mock, client }
    }

    /// Mount the full successful Okta authentication flow.
    fn with_successful_okta_flow(self) -> Self {
        self.mock
            .mount(okta::authenticator_request(&self.mock.https_url()));
        self.mock.mount(okta::okta_token_success());
        self.mock
            .mount(okta::okta_sso_success(&self.mock.http_url()));
        self.mock.mount(okta::login_success());
        self
    }

    /// Mount only the Snowflake authenticator-request.
    fn with_authenticator_request(self) -> Self {
        self.mock
            .mount(okta::authenticator_request(&self.mock.https_url()));
        self
    }

    fn set_option(&self, key: &str, value: &str) -> &Self {
        self.client.set_connection_option(key, value);
        self
    }

    fn connect(&self) -> Result<(), String> {
        self.client.connect()
    }

    fn assert_success(result: Result<(), String>, context: &str) {
        assert!(result.is_ok(), "Expected {context}, got: {result:?}");
    }

    fn assert_error(result: Result<(), String>, patterns: &[&str], context: &str) {
        let error = result.expect_err(&format!("Expected {context} to fail"));
        let matches = patterns.iter().any(|p| error.contains(p));
        assert!(matches, "Expected {context}, got: {error}");
    }
}

// =============================================================================
// Basic Authentication Flow
// =============================================================================

#[test]
fn should_login_with_native_okta_using_saml_flow() {
    // Given Wiremock is running with Snowflake and Okta mappings
    let fixture = OktaTestFixture::new().with_successful_okta_flow();

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login is successful
    OktaTestFixture::assert_success(result, "Okta login to succeed");
}

// =============================================================================
// Error Handling - Invalid Credentials
// =============================================================================

#[test]
fn should_fail_with_bad_credentials_when_okta_returns_401() {
    // Given Wiremock is running with Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token endpoint returning 401 Unauthorized
    fixture.mock.mount(okta::okta_token_401());
    fixture.set_option("user", "invalid_user");
    fixture.set_option("password", "wrong_password");

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails with bad credentials error
    OktaTestFixture::assert_error(
        result,
        &["BadCredentials", "401", "credentials"],
        "bad credentials error",
    );
}

#[test]
fn should_fail_with_bad_credentials_when_okta_returns_403() {
    // Given Wiremock is running with Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token endpoint returning 403 Forbidden
    fixture.mock.mount(okta::okta_token_403());
    fixture.set_option("user", "forbidden_user");
    fixture.set_option("password", "forbidden_password");

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails with bad credentials error
    OktaTestFixture::assert_error(
        result,
        &["BadCredentials", "403", "credentials"],
        "bad credentials error",
    );
}

// =============================================================================
// Error Handling - MFA Required
// =============================================================================

#[test]
fn should_fail_when_okta_returns_mfa_required_status() {
    // Given Wiremock is running with Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token endpoint returning MFA_REQUIRED status
    fixture.mock.mount(okta::okta_token_mfa_required());
    fixture.set_option("user", "mfa_user");
    fixture.set_option("password", "mfa_password");

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails with MFA required error
    OktaTestFixture::assert_error(result, &["MfaRequired", "MFA", "mfa"], "MFA required error");
}

// =============================================================================
// IdP URL Validation
// =============================================================================

#[test]
fn should_fail_when_tokenurl_does_not_match_configured_okta_url_origin() {
    // Given Wiremock is running
    let fixture = OktaTestFixture::new();

    // And Wiremock has Snowflake authenticator-request with mismatched tokenUrl
    fixture
        .mock
        .mount(okta::authenticator_request_mismatched_token_url(
            &fixture.mock.https_url(),
        ));

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails with IdP URL mismatch error
    OktaTestFixture::assert_error(
        result,
        &["IdpUrlMismatch", "mismatch", "does not match"],
        "IdP URL mismatch error",
    );
}

#[test]
fn should_fail_when_ssourl_does_not_match_configured_okta_url_origin() {
    // Given Wiremock is running
    let fixture = OktaTestFixture::new();

    // And Wiremock has Snowflake authenticator-request with mismatched ssoUrl
    fixture
        .mock
        .mount(okta::authenticator_request_mismatched_sso_url(
            &fixture.mock.https_url(),
        ));

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails with IdP URL mismatch error
    OktaTestFixture::assert_error(
        result,
        &["IdpUrlMismatch", "mismatch", "does not match"],
        "IdP URL mismatch error",
    );
}

// =============================================================================
// SAML Postback Validation
// =============================================================================

#[test]
fn should_fail_when_saml_postback_url_does_not_match_snowflake_server() {
    // Given Wiremock is running with Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token success mapping
    fixture.mock.mount(okta::okta_token_success());

    // And Wiremock has Okta SSO returning SAML with mismatched postback URL
    fixture.mock.mount(okta::okta_sso_mismatched_postback());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails with SAML destination mismatch error
    OktaTestFixture::assert_error(
        result,
        &["SamlDestinationMismatch", "postback", "destination"],
        "SAML destination mismatch error",
    );
}

#[test]
fn should_succeed_with_mismatched_postback_when_disable_saml_url_check_is_true() {
    // Given Wiremock is running with Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token success mapping
    fixture.mock.mount(okta::okta_token_success());

    // And Wiremock has Okta SSO returning SAML with mismatched postback URL
    fixture.mock.mount(okta::okta_sso_mismatched_postback());

    // And Wiremock has Snowflake login success for Okta
    fixture.mock.mount(okta::login_success());

    // And Snowflake client is configured for native Okta with disable_saml_url_check
    fixture.set_option("disable_saml_url_check", "true");

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login is successful
    OktaTestFixture::assert_success(result, "Okta login to succeed with disable_saml_url_check");
}

#[test]
fn should_fail_when_saml_html_is_missing_form_action() {
    // Given Wiremock is running with Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token success mapping
    fixture.mock.mount(okta::okta_token_success());

    // And Wiremock has Okta SSO returning SAML HTML without form action
    fixture.mock.mount(okta::okta_sso_missing_form_action());
    fixture.set_option("authentication_timeout", "15");

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails with missing SAML postback error
    OktaTestFixture::assert_error(
        result,
        &["MissingSamlPostback", "postback", "form action"],
        "missing SAML postback error",
    );
}

// =============================================================================
// Token Handling
// =============================================================================

#[test]
fn should_use_cookietoken_when_sessiontoken_is_missing() {
    // Given Wiremock is running with Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token endpoint returning cookieToken instead of sessionToken
    fixture.mock.mount(okta::okta_token_cookie_token());

    // And Wiremock has Okta SSO success mapping
    fixture
        .mock
        .mount(okta::okta_sso_success(&fixture.mock.http_url()));

    // And Wiremock has Snowflake login success for Okta
    fixture.mock.mount(okta::login_success());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login is successful
    OktaTestFixture::assert_success(result, "Okta login with cookieToken to succeed");
}

// =============================================================================
// Retry Behavior - Token Refresh on Transient Errors
// =============================================================================

#[test]
fn should_retry_saml_fetch_with_fresh_token_on_transient_error() {
    // Given Wiremock is running with Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token success mapping
    fixture.mock.mount(okta::okta_token_success());

    // And Wiremock has Okta SSO returning 503 on first attempt
    fixture.mock.mount(okta::okta_sso_503_once());

    // And Wiremock has Okta SSO returning success on retry
    fixture
        .mock
        .mount(okta::okta_sso_success(&fixture.mock.http_url()));

    // And Wiremock has Snowflake login success for Okta
    fixture.mock.mount(okta::login_success());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Login is successful
    OktaTestFixture::assert_success(
        result,
        "Okta login to succeed after retrying transient error",
    );
}
