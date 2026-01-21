use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::wiremock_client::WiremockClient;
use std::collections::HashMap;

// =============================================================================
// Basic Authentication Flow - WireMock Integration
// =============================================================================

#[test]
fn should_login_with_native_okta_using_saml_flow() {
    // Given Wiremock is running
    let wiremock = WiremockClient::start();

    // And Wiremock has Snowflake and Okta mappings
    let mut placeholders = HashMap::new();
    placeholders.insert("{{OKTA_BASE_URL}}".to_string(), wiremock.https_url());
    placeholders.insert("{{SNOWFLAKE_BASE_URL}}".to_string(), wiremock.http_url());

    wiremock.add_mapping("auth/authenticator_request_okta.json", Some(&placeholders));
    wiremock.add_mapping("auth/okta_token_success.json", Some(&placeholders));
    wiremock.add_mapping("auth/okta_sso_success.json", Some(&placeholders));
    wiremock.add_mapping("auth/login_success_okta.json", Some(&placeholders));

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    let client = SnowflakeTestClient::with_int_tests_params(Some(&wiremock.http_url()));
    client.set_connection_option("authenticator", &wiremock.https_url());
    client.set_connection_option("user", "test_user");
    client.set_connection_option("password", "test_password");
    client.set_connection_option("verify_certificates", "false");
    client.set_connection_option("verify_hostname", "false");

    // When Trying to Connect
    let result = client.connect();

    // Then Login is successful
    assert!(
        result.is_ok(),
        "Expected Okta login to succeed, got: {result:?}"
    );
}

// =============================================================================
// Error Handling - Invalid Credentials
// =============================================================================

#[test]
fn should_fail_with_bad_credentials_when_okta_returns_401() {
    // Given Wiremock is running
    let wiremock = WiremockClient::start();

    // And Wiremock has Snowflake authenticator-request mapping
    let mut placeholders = HashMap::new();
    placeholders.insert("{{OKTA_BASE_URL}}".to_string(), wiremock.https_url());
    placeholders.insert("{{SNOWFLAKE_BASE_URL}}".to_string(), wiremock.http_url());

    wiremock.add_mapping("auth/authenticator_request_okta.json", Some(&placeholders));

    // And Wiremock has Okta token endpoint returning 401 Unauthorized
    wiremock.add_mapping("auth/okta_token_401.json", None);

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    let client = SnowflakeTestClient::with_int_tests_params(Some(&wiremock.http_url()));
    client.set_connection_option("authenticator", &wiremock.https_url());
    client.set_connection_option("user", "invalid_user");
    client.set_connection_option("password", "wrong_password");
    client.set_connection_option("verify_certificates", "false");
    client.set_connection_option("verify_hostname", "false");

    // When Trying to Connect
    let result = client.connect();

    // Then Connection fails with bad credentials error
    let error = result.expect_err("Expected connection to fail");
    assert!(
        error.contains("BadCredentials") || error.contains("401") || error.contains("credentials"),
        "Expected bad credentials error, got: {error}"
    );
}

#[test]
fn should_fail_with_bad_credentials_when_okta_returns_403() {
    // Given Wiremock is running
    let wiremock = WiremockClient::start();

    // And Wiremock has Snowflake authenticator-request mapping
    let mut placeholders = HashMap::new();
    placeholders.insert("{{OKTA_BASE_URL}}".to_string(), wiremock.https_url());
    placeholders.insert("{{SNOWFLAKE_BASE_URL}}".to_string(), wiremock.http_url());

    wiremock.add_mapping("auth/authenticator_request_okta.json", Some(&placeholders));

    // And Wiremock has Okta token endpoint returning 403 Forbidden
    wiremock.add_mapping("auth/okta_token_403.json", None);

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    let client = SnowflakeTestClient::with_int_tests_params(Some(&wiremock.http_url()));
    client.set_connection_option("authenticator", &wiremock.https_url());
    client.set_connection_option("user", "forbidden_user");
    client.set_connection_option("password", "forbidden_password");
    client.set_connection_option("verify_certificates", "false");
    client.set_connection_option("verify_hostname", "false");

    // When Trying to Connect
    let result = client.connect();

    // Then Connection fails with bad credentials error
    let error = result.expect_err("Expected connection to fail");
    assert!(
        error.contains("BadCredentials") || error.contains("403") || error.contains("credentials"),
        "Expected bad credentials error, got: {error}"
    );
}

// =============================================================================
// Error Handling - MFA Required
// =============================================================================

#[test]
fn should_fail_when_okta_returns_mfa_required_status() {
    // Given Wiremock is running
    let wiremock = WiremockClient::start();

    // And Wiremock has Snowflake authenticator-request mapping
    let mut placeholders = HashMap::new();
    placeholders.insert("{{OKTA_BASE_URL}}".to_string(), wiremock.https_url());
    placeholders.insert("{{SNOWFLAKE_BASE_URL}}".to_string(), wiremock.http_url());

    wiremock.add_mapping("auth/authenticator_request_okta.json", Some(&placeholders));

    // And Wiremock has Okta token endpoint returning MFA_REQUIRED status
    wiremock.add_mapping("auth/okta_token_mfa_required.json", None);

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    let client = SnowflakeTestClient::with_int_tests_params(Some(&wiremock.http_url()));
    client.set_connection_option("authenticator", &wiremock.https_url());
    client.set_connection_option("user", "mfa_user");
    client.set_connection_option("password", "mfa_password");
    client.set_connection_option("verify_certificates", "false");
    client.set_connection_option("verify_hostname", "false");

    // When Trying to Connect
    let result = client.connect();

    // Then Connection fails with MFA required error
    let error = result.expect_err("Expected connection to fail");
    assert!(
        error.contains("MfaRequired") || error.contains("MFA") || error.contains("mfa"),
        "Expected MFA required error, got: {error}"
    );
}

// =============================================================================
// IdP URL Validation
// =============================================================================

#[test]
fn should_fail_when_tokenurl_does_not_match_configured_okta_url_origin() {
    // Given Wiremock is running
    let wiremock = WiremockClient::start();

    // And Wiremock has Snowflake authenticator-request with mismatched tokenUrl
    let mut placeholders = HashMap::new();
    placeholders.insert("{{OKTA_BASE_URL}}".to_string(), wiremock.https_url());
    placeholders.insert("{{SNOWFLAKE_BASE_URL}}".to_string(), wiremock.http_url());

    wiremock.add_mapping(
        "auth/authenticator_request_okta_mismatched_token_url.json",
        Some(&placeholders),
    );

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    let client = SnowflakeTestClient::with_int_tests_params(Some(&wiremock.http_url()));
    client.set_connection_option("authenticator", &wiremock.https_url());
    client.set_connection_option("user", "test_user");
    client.set_connection_option("password", "test_password");
    client.set_connection_option("verify_certificates", "false");
    client.set_connection_option("verify_hostname", "false");

    // When Trying to Connect
    let result = client.connect();

    // Then Connection fails with IdP URL mismatch error
    let error = result.expect_err("Expected connection to fail");
    assert!(
        error.contains("IdpUrlMismatch")
            || error.contains("mismatch")
            || error.contains("does not match"),
        "Expected IdP URL mismatch error, got: {error}"
    );
}

#[test]
fn should_fail_when_ssourl_does_not_match_configured_okta_url_origin() {
    // Given Wiremock is running
    let wiremock = WiremockClient::start();

    // And Wiremock has Snowflake authenticator-request with mismatched ssoUrl
    let mut placeholders = HashMap::new();
    placeholders.insert("{{OKTA_BASE_URL}}".to_string(), wiremock.https_url());
    placeholders.insert("{{SNOWFLAKE_BASE_URL}}".to_string(), wiremock.http_url());

    wiremock.add_mapping(
        "auth/authenticator_request_okta_mismatched_sso_url.json",
        Some(&placeholders),
    );

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    let client = SnowflakeTestClient::with_int_tests_params(Some(&wiremock.http_url()));
    client.set_connection_option("authenticator", &wiremock.https_url());
    client.set_connection_option("user", "test_user");
    client.set_connection_option("password", "test_password");
    client.set_connection_option("verify_certificates", "false");
    client.set_connection_option("verify_hostname", "false");

    // When Trying to Connect
    let result = client.connect();

    // Then Connection fails with IdP URL mismatch error
    let error = result.expect_err("Expected connection to fail");
    assert!(
        error.contains("IdpUrlMismatch")
            || error.contains("mismatch")
            || error.contains("does not match"),
        "Expected IdP URL mismatch error, got: {error}"
    );
}

// =============================================================================
// SAML Postback Validation
// =============================================================================

#[test]
fn should_fail_when_saml_postback_url_does_not_match_snowflake_server() {
    // Given Wiremock is running
    let wiremock = WiremockClient::start();

    // And Wiremock has Snowflake authenticator-request mapping
    let mut placeholders = HashMap::new();
    placeholders.insert("{{OKTA_BASE_URL}}".to_string(), wiremock.https_url());
    placeholders.insert("{{SNOWFLAKE_BASE_URL}}".to_string(), wiremock.http_url());

    wiremock.add_mapping("auth/authenticator_request_okta.json", Some(&placeholders));

    // And Wiremock has Okta token success mapping
    wiremock.add_mapping("auth/okta_token_success.json", Some(&placeholders));

    // And Wiremock has Okta SSO returning SAML with mismatched postback URL
    wiremock.add_mapping("auth/okta_sso_mismatched_postback.json", None);

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    let client = SnowflakeTestClient::with_int_tests_params(Some(&wiremock.http_url()));
    client.set_connection_option("authenticator", &wiremock.https_url());
    client.set_connection_option("user", "test_user");
    client.set_connection_option("password", "test_password");
    client.set_connection_option("verify_certificates", "false");
    client.set_connection_option("verify_hostname", "false");

    // When Trying to Connect
    let result = client.connect();

    // Then Connection fails with SAML destination mismatch error
    let error = result.expect_err("Expected connection to fail");
    assert!(
        error.contains("SamlDestinationMismatch")
            || error.contains("postback")
            || error.contains("destination"),
        "Expected SAML destination mismatch error, got: {error}"
    );
}

#[test]
fn should_succeed_with_mismatched_postback_when_disable_saml_url_check_is_true() {
    // Given Wiremock is running
    let wiremock = WiremockClient::start();

    // And Wiremock has Snowflake authenticator-request mapping
    let mut placeholders = HashMap::new();
    placeholders.insert("{{OKTA_BASE_URL}}".to_string(), wiremock.https_url());
    placeholders.insert("{{SNOWFLAKE_BASE_URL}}".to_string(), wiremock.http_url());

    wiremock.add_mapping("auth/authenticator_request_okta.json", Some(&placeholders));

    // And Wiremock has Okta token success mapping
    wiremock.add_mapping("auth/okta_token_success.json", Some(&placeholders));

    // And Wiremock has Okta SSO returning SAML with mismatched postback URL
    wiremock.add_mapping("auth/okta_sso_mismatched_postback.json", None);

    // And Wiremock has Snowflake login success for Okta
    wiremock.add_mapping("auth/login_success_okta.json", Some(&placeholders));

    // And Snowflake client is configured for native Okta with disable_saml_url_check
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    let client = SnowflakeTestClient::with_int_tests_params(Some(&wiremock.http_url()));
    client.set_connection_option("authenticator", &wiremock.https_url());
    client.set_connection_option("user", "test_user");
    client.set_connection_option("password", "test_password");
    client.set_connection_option("disable_saml_url_check", "true");
    client.set_connection_option("verify_certificates", "false");
    client.set_connection_option("verify_hostname", "false");

    // When Trying to Connect
    let result = client.connect();

    // Then Login is successful
    assert!(
        result.is_ok(),
        "Expected Okta login to succeed with disable_saml_url_check, got: {result:?}"
    );
}

#[test]
fn should_fail_when_saml_html_is_missing_form_action() {
    // Given Wiremock is running
    let wiremock = WiremockClient::start();

    // And Wiremock has Snowflake authenticator-request mapping
    let mut placeholders = HashMap::new();
    placeholders.insert("{{OKTA_BASE_URL}}".to_string(), wiremock.https_url());
    placeholders.insert("{{SNOWFLAKE_BASE_URL}}".to_string(), wiremock.http_url());

    wiremock.add_mapping("auth/authenticator_request_okta.json", Some(&placeholders));

    // And Wiremock has Okta token success mapping
    wiremock.add_mapping("auth/okta_token_success.json", Some(&placeholders));

    // And Wiremock has Okta SSO returning SAML HTML without form action
    wiremock.add_mapping("auth/okta_sso_missing_form_action.json", None);

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    let client = SnowflakeTestClient::with_int_tests_params(Some(&wiremock.http_url()));
    client.set_connection_option("authenticator", &wiremock.https_url());
    client.set_connection_option("user", "test_user");
    client.set_connection_option("password", "test_password");
    client.set_connection_option("verify_certificates", "false");
    client.set_connection_option("verify_hostname", "false");
    // Set a short timeout to avoid long waits on retry
    client.set_connection_option("authentication_timeout", "5");

    // When Trying to Connect
    let result = client.connect();

    // Then Connection fails with missing SAML postback error
    let error = result.expect_err("Expected connection to fail");
    assert!(
        error.contains("MissingSamlPostback")
            || error.contains("postback")
            || error.contains("form action"),
        "Expected missing SAML postback error, got: {error}"
    );
}

// =============================================================================
// Token Handling
// =============================================================================

#[test]
fn should_use_cookietoken_when_sessiontoken_is_missing() {
    // Given Wiremock is running
    let wiremock = WiremockClient::start();

    // And Wiremock has Snowflake authenticator-request mapping
    let mut placeholders = HashMap::new();
    placeholders.insert("{{OKTA_BASE_URL}}".to_string(), wiremock.https_url());
    placeholders.insert("{{SNOWFLAKE_BASE_URL}}".to_string(), wiremock.http_url());

    wiremock.add_mapping("auth/authenticator_request_okta.json", Some(&placeholders));

    // And Wiremock has Okta token endpoint returning cookieToken instead of sessionToken
    wiremock.add_mapping("auth/okta_token_cookie_token.json", None);

    // And Wiremock has Okta SSO success mapping
    wiremock.add_mapping("auth/okta_sso_success.json", Some(&placeholders));

    // And Wiremock has Snowflake login success for Okta
    wiremock.add_mapping("auth/login_success_okta.json", Some(&placeholders));

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    let client = SnowflakeTestClient::with_int_tests_params(Some(&wiremock.http_url()));
    client.set_connection_option("authenticator", &wiremock.https_url());
    client.set_connection_option("user", "test_user");
    client.set_connection_option("password", "test_password");
    client.set_connection_option("verify_certificates", "false");
    client.set_connection_option("verify_hostname", "false");

    // When Trying to Connect
    let result = client.connect();

    // Then Login is successful
    assert!(
        result.is_ok(),
        "Expected Okta login with cookieToken to succeed, got: {result:?}"
    );
}

// =============================================================================
// Retry Behavior - Token Refresh on Transient Errors
// =============================================================================

#[test]
fn should_retry_saml_fetch_with_fresh_token_on_transient_error() {
    // Given Wiremock is running
    let wiremock = WiremockClient::start();

    // And Wiremock has Snowflake authenticator-request mapping
    let mut placeholders = HashMap::new();
    placeholders.insert("{{OKTA_BASE_URL}}".to_string(), wiremock.https_url());
    placeholders.insert("{{SNOWFLAKE_BASE_URL}}".to_string(), wiremock.http_url());

    wiremock.add_mapping("auth/authenticator_request_okta.json", Some(&placeholders));

    // And Wiremock has Okta token success mapping
    wiremock.add_mapping("auth/okta_token_success.json", Some(&placeholders));

    // And Wiremock has Okta SSO returning 503 on first attempt
    wiremock.set_scenario_state("okta-sso-retry", "Retry Test Started");

    // And Wiremock has Okta SSO returning success on retry
    wiremock.add_mapping(
        "auth/okta_sso_success_after_retry.json",
        Some(&placeholders),
    );

    // And Wiremock has Snowflake login success for Okta
    wiremock.add_mapping("auth/login_success_okta.json", Some(&placeholders));

    // And Snowflake client is configured for native Okta
    // And TLS certificate verification is disabled for the Okta HTTPS mock
    let client = SnowflakeTestClient::with_int_tests_params(Some(&wiremock.http_url()));
    client.set_connection_option("authenticator", &wiremock.https_url());
    client.set_connection_option("user", "test_user");
    client.set_connection_option("password", "test_password");
    client.set_connection_option("verify_certificates", "false");
    client.set_connection_option("verify_hostname", "false");

    // When Trying to Connect
    let result = client.connect();

    // Then Login is successful
    assert!(
        result.is_ok(),
        "Expected Okta login to succeed after retrying transient error, got: {result:?}"
    );
}
