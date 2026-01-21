use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::wiremock_client::WiremockClient;
use std::collections::HashMap;

// =============================================================================
// Test Fixture - Reduces boilerplate for Native Okta integration tests
// =============================================================================

/// Test fixture that encapsulates common setup for Native Okta integration tests.
/// Provides WireMock server with HTTPS support and a pre-configured Snowflake client.
struct OktaTestFixture {
    wiremock: WiremockClient,
    client: SnowflakeTestClient,
    placeholders: HashMap<String, String>,
}

impl OktaTestFixture {
    /// Create a new test fixture with WireMock running and client configured for Okta.
    fn new() -> Self {
        let wiremock = WiremockClient::start();

        let mut placeholders = HashMap::new();
        placeholders.insert("{{OKTA_BASE_URL}}".to_string(), wiremock.https_url());
        placeholders.insert("{{SNOWFLAKE_BASE_URL}}".to_string(), wiremock.http_url());

        let client = SnowflakeTestClient::with_int_tests_params(Some(&wiremock.http_url()));
        client.set_connection_option("authenticator", &wiremock.https_url());
        client.set_connection_option("user", "test_user");
        client.set_connection_option("password", "test_password");
        client.set_connection_option("verify_certificates", "false");
        client.set_connection_option("verify_hostname", "false");

        Self {
            wiremock,
            client,
            placeholders,
        }
    }

    /// Add the standard Okta authentication flow mappings (authenticator + token + SSO + login).
    fn with_successful_okta_flow(self) -> Self {
        self.wiremock.add_mapping(
            "auth/authenticator_request_okta.json",
            Some(&self.placeholders),
        );
        self.wiremock
            .add_mapping("auth/okta_token_success.json", Some(&self.placeholders));
        self.wiremock
            .add_mapping("auth/okta_sso_success.json", Some(&self.placeholders));
        self.wiremock
            .add_mapping("auth/login_success_okta.json", Some(&self.placeholders));
        self
    }

    /// Add authenticator-request mapping only (for tests that need custom token/SSO behavior).
    fn with_authenticator_request(self) -> Self {
        self.wiremock.add_mapping(
            "auth/authenticator_request_okta.json",
            Some(&self.placeholders),
        );
        self
    }

    /// Add a mapping with placeholder substitution.
    fn add_mapping(&self, path: &str) -> &Self {
        self.wiremock.add_mapping(path, Some(&self.placeholders));
        self
    }

    /// Add a mapping without placeholder substitution.
    fn add_mapping_raw(&self, path: &str) -> &Self {
        self.wiremock.add_mapping(path, None);
        self
    }

    /// Set a connection option on the client.
    fn set_option(&self, key: &str, value: &str) -> &Self {
        self.client.set_connection_option(key, value);
        self
    }

    /// Set WireMock scenario state (for stateful mock behavior).
    fn set_scenario_state(&self, scenario: &str, state: &str) -> &Self {
        self.wiremock.set_scenario_state(scenario, state);
        self
    }

    /// Connect and return the result.
    fn connect(&self) -> Result<(), String> {
        self.client.connect()
    }

    /// Connect and expect success.
    fn connect_expecting_success(&self, context: &str) {
        let result = self.connect();
        assert!(result.is_ok(), "Expected {context}, got: {result:?}");
    }

    /// Connect and expect failure containing any of the given patterns.
    fn connect_expecting_error(&self, patterns: &[&str], context: &str) {
        let error = self
            .connect()
            .expect_err(&format!("Expected {context} to fail"));
        let matches = patterns.iter().any(|p| error.contains(p));
        assert!(matches, "Expected {context}, got: {error}");
    }
}

// =============================================================================
// Basic Authentication Flow - WireMock Integration
// =============================================================================

#[test]
fn should_login_with_native_okta_using_saml_flow() {
    // Given Wiremock is running
    // And Wiremock has Snowflake and Okta mappings
    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    let fixture = OktaTestFixture::new().with_successful_okta_flow();

    // When Trying to Connect
    // Then Login is successful
    fixture.connect_expecting_success("Okta login to succeed");
}

// =============================================================================
// Error Handling - Invalid Credentials
// =============================================================================

#[test]
fn should_fail_with_bad_credentials_when_okta_returns_401() {
    // Given Wiremock is running
    // And Wiremock has Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token endpoint returning 401 Unauthorized
    fixture.add_mapping_raw("auth/okta_token_401.json");

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    fixture.set_option("user", "invalid_user");
    fixture.set_option("password", "wrong_password");

    // When Trying to Connect
    // Then Connection fails with bad credentials error
    fixture.connect_expecting_error(
        &["BadCredentials", "401", "credentials"],
        "bad credentials error",
    );
}

#[test]
fn should_fail_with_bad_credentials_when_okta_returns_403() {
    // Given Wiremock is running
    // And Wiremock has Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token endpoint returning 403 Forbidden
    fixture.add_mapping_raw("auth/okta_token_403.json");

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    fixture.set_option("user", "forbidden_user");
    fixture.set_option("password", "forbidden_password");

    // When Trying to Connect
    // Then Connection fails with bad credentials error
    fixture.connect_expecting_error(
        &["BadCredentials", "403", "credentials"],
        "bad credentials error",
    );
}

// =============================================================================
// Error Handling - MFA Required
// =============================================================================

#[test]
fn should_fail_when_okta_returns_mfa_required_status() {
    // Given Wiremock is running
    // And Wiremock has Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token endpoint returning MFA_REQUIRED status
    fixture.add_mapping_raw("auth/okta_token_mfa_required.json");

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    fixture.set_option("user", "mfa_user");
    fixture.set_option("password", "mfa_password");

    // When Trying to Connect
    // Then Connection fails with MFA required error
    fixture.connect_expecting_error(&["MfaRequired", "MFA", "mfa"], "MFA required error");
}

// =============================================================================
// IdP URL Validation
// =============================================================================

#[test]
fn should_fail_when_tokenurl_does_not_match_configured_okta_url_origin() {
    // Given Wiremock is running
    let fixture = OktaTestFixture::new();

    // And Wiremock has Snowflake authenticator-request with mismatched tokenUrl
    fixture.add_mapping("auth/authenticator_request_okta_mismatched_token_url.json");

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock

    // When Trying to Connect
    // Then Connection fails with IdP URL mismatch error
    fixture.connect_expecting_error(
        &["IdpUrlMismatch", "mismatch", "does not match"],
        "IdP URL mismatch error",
    );
}

#[test]
fn should_fail_when_ssourl_does_not_match_configured_okta_url_origin() {
    // Given Wiremock is running
    let fixture = OktaTestFixture::new();

    // And Wiremock has Snowflake authenticator-request with mismatched ssoUrl
    fixture.add_mapping("auth/authenticator_request_okta_mismatched_sso_url.json");

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock

    // When Trying to Connect
    // Then Connection fails with IdP URL mismatch error
    fixture.connect_expecting_error(
        &["IdpUrlMismatch", "mismatch", "does not match"],
        "IdP URL mismatch error",
    );
}

// =============================================================================
// SAML Postback Validation
// =============================================================================

#[test]
fn should_fail_when_saml_postback_url_does_not_match_snowflake_server() {
    // Given Wiremock is running
    // And Wiremock has Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token success mapping
    fixture.add_mapping("auth/okta_token_success.json");

    // And Wiremock has Okta SSO returning SAML with mismatched postback URL
    fixture.add_mapping_raw("auth/okta_sso_mismatched_postback.json");

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock

    // When Trying to Connect
    // Then Connection fails with SAML destination mismatch error
    fixture.connect_expecting_error(
        &["SamlDestinationMismatch", "postback", "destination"],
        "SAML destination mismatch error",
    );
}

#[test]
fn should_succeed_with_mismatched_postback_when_disable_saml_url_check_is_true() {
    // Given Wiremock is running
    // And Wiremock has Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token success mapping
    fixture.add_mapping("auth/okta_token_success.json");

    // And Wiremock has Okta SSO returning SAML with mismatched postback URL
    fixture.add_mapping_raw("auth/okta_sso_mismatched_postback.json");

    // And Wiremock has Snowflake login success for Okta
    fixture.add_mapping("auth/login_success_okta.json");

    // And Snowflake client is configured for native Okta with disable_saml_url_check
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    fixture.set_option("disable_saml_url_check", "true");

    // When Trying to Connect
    // Then Login is successful
    fixture.connect_expecting_success("Okta login to succeed with disable_saml_url_check");
}

#[test]
fn should_fail_when_saml_html_is_missing_form_action() {
    // Given Wiremock is running
    // And Wiremock has Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token success mapping
    fixture.add_mapping("auth/okta_token_success.json");

    // And Wiremock has Okta SSO returning SAML HTML without form action
    fixture.add_mapping_raw("auth/okta_sso_missing_form_action.json");

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    fixture.set_option("authentication_timeout", "5"); // Short timeout to avoid long waits on retry

    // When Trying to Connect
    // Then Connection fails with missing SAML postback error
    fixture.connect_expecting_error(
        &["MissingSamlPostback", "postback", "form action"],
        "missing SAML postback error",
    );
}

// =============================================================================
// Token Handling
// =============================================================================

#[test]
fn should_use_cookietoken_when_sessiontoken_is_missing() {
    // Given Wiremock is running
    // And Wiremock has Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token endpoint returning cookieToken instead of sessionToken
    fixture.add_mapping_raw("auth/okta_token_cookie_token.json");

    // And Wiremock has Okta SSO success mapping
    fixture.add_mapping("auth/okta_sso_success.json");

    // And Wiremock has Snowflake login success for Okta
    fixture.add_mapping("auth/login_success_okta.json");

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock

    // When Trying to Connect
    // Then Login is successful
    fixture.connect_expecting_success("Okta login with cookieToken to succeed");
}

// =============================================================================
// Retry Behavior - Token Refresh on Transient Errors
// =============================================================================

#[test]
fn should_retry_saml_fetch_with_fresh_token_on_transient_error() {
    // Given Wiremock is running
    // And Wiremock has Snowflake authenticator-request mapping
    let fixture = OktaTestFixture::new().with_authenticator_request();

    // And Wiremock has Okta token success mapping
    fixture.add_mapping("auth/okta_token_success.json");

    // And Wiremock has Okta SSO returning 503 on first attempt
    fixture.set_scenario_state("okta-sso-retry", "Retry Test Started");
    fixture.add_mapping_raw("auth/okta_sso_503_first_attempt.json");

    // And Wiremock has Okta SSO returning success on retry
    fixture.add_mapping("auth/okta_sso_success_after_retry.json");

    // And Wiremock has Snowflake login success for Okta
    fixture.add_mapping("auth/login_success_okta.json");

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock

    // When Trying to Connect
    // Then Login is successful
    fixture.connect_expecting_success("Okta login to succeed after retrying transient error");
}
