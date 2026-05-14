use std::io::{Read, Write};

use crate::common::mocks::external_browser;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;

// =============================================================================
// Test Fixture
// =============================================================================

struct ExternalBrowserTestFixture {
    mock: MockServerWithTls,
    client: SnowflakeTestClient,
}

impl ExternalBrowserTestFixture {
    fn new() -> Self {
        unsafe { std::env::set_var("SF_TEST_BROWSER_OPENER", "noop") };

        let mock = MockServerWithTls::start();

        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("authenticator", "EXTERNALBROWSER");
        client.set_connection_option("user", "test_user");
        client.set_connection_option("authentication_timeout", "10");

        Self { mock, client }
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
// Helper: simulate the browser callback
// =============================================================================

/// Extract the `BROWSER_MODE_REDIRECT_PORT` from the authenticator-request
/// body recorded by wiremock, then send a fake token to that port.
fn simulate_browser_callback(mock: &MockServerWithTls, token: &str) {
    let requests = mock.received_requests();
    let authn_req = requests
        .iter()
        .find(|r| r.url.path().contains("authenticator-request"))
        .expect("No authenticator-request was captured");

    let body: serde_json::Value =
        serde_json::from_slice(&authn_req.body).expect("Request body is not valid JSON");
    let port_str = body["data"]["BROWSER_MODE_REDIRECT_PORT"]
        .as_str()
        .expect("BROWSER_MODE_REDIRECT_PORT not found in request body");
    let port: u16 = port_str
        .parse()
        .expect("BROWSER_MODE_REDIRECT_PORT is not a valid port number");

    let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .expect("Failed to connect to callback listener");
    let request = format!("GET /?token={token} HTTP/1.1\r\nHost: localhost\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .expect("Failed to write to callback listener");

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .expect("Failed to read response from callback listener");
    let response_str = String::from_utf8_lossy(&response);
    assert!(
        response_str.contains("200 OK"),
        "Expected 200 OK response, got: {response_str}"
    );
}

// =============================================================================
// Happy Path
// =============================================================================

#[test]
fn should_login_with_external_browser_using_simulated_callback() {
    // Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
    let fixture = ExternalBrowserTestFixture::new();
    let proof_key = "test_proof_key_abc123";
    fixture.mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        proof_key,
    ));

    // And Login endpoint returns success
    fixture.mock.mount(external_browser::login_success());

    // When Trying to Connect with simulated browser callback delivering a token
    let mock_ref = &fixture.mock;
    let result = std::thread::scope(|s| {
        s.spawn(|| {
            for _ in 0..50 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let requests = mock_ref.received_requests();
                if requests
                    .iter()
                    .any(|r| r.url.path().contains("authenticator-request"))
                {
                    simulate_browser_callback(mock_ref, "browser_sso_token_12345");
                    return;
                }
            }
            panic!("Timed out waiting for authenticator-request");
        });

        fixture.connect()
    });

    // Then Login is successful
    ExternalBrowserTestFixture::assert_success(result, "external browser login to succeed");

    // And Login request contains EXTERNALBROWSER authenticator, token, proof key, and login name
    let requests = fixture.mock.received_requests();
    let login_req = requests
        .iter()
        .find(|r| r.url.path().contains("login-request"))
        .expect("No login-request was captured");
    let body: serde_json::Value = serde_json::from_slice(&login_req.body).unwrap();
    assert_eq!(body["data"]["AUTHENTICATOR"], "EXTERNALBROWSER");
    assert_eq!(body["data"]["TOKEN"], "browser_sso_token_12345");
    assert_eq!(body["data"]["PROOF_KEY"], proof_key);
    assert_eq!(body["data"]["LOGIN_NAME"], "test_user");
}

// =============================================================================
// Error Handling - Authenticator Request Failures
// =============================================================================

#[test]
fn should_fail_when_authenticator_request_returns_forbidden() {
    // Given Wiremock returns HTTP 403 for authenticator-request
    let fixture = ExternalBrowserTestFixture::new();
    fixture
        .mock
        .mount(external_browser::authenticator_request_forbidden());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails with authenticator error
    ExternalBrowserTestFixture::assert_error(
        result,
        &["403", "Forbidden", "ExternalBrowser", "authenticator"],
        "authenticator-request forbidden error",
    );
}

#[test]
fn should_fail_when_authenticator_request_returns_logical_failure() {
    // Given Wiremock returns success false for authenticator-request
    let fixture = ExternalBrowserTestFixture::new();
    fixture
        .mock
        .mount(external_browser::authenticator_request_logical_failure());

    // When Trying to Connect
    let result = fixture.connect();

    // Then Connection fails with authenticator error
    ExternalBrowserTestFixture::assert_error(
        result,
        &[
            "not enabled",
            "ExternalBrowser",
            "authenticator",
            "logical failure",
        ],
        "authenticator-request logical failure error",
    );
}

// =============================================================================
// Error Handling - Timeout (no browser callback)
// =============================================================================

#[test]
fn should_fail_with_timeout_when_no_browser_callback_arrives() {
    // Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
    let fixture = ExternalBrowserTestFixture::new();
    fixture.mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        "proof_key",
    ));

    // And Authentication timeout is set to 2 seconds
    fixture
        .client
        .set_connection_option("authentication_timeout", "2");

    // When Trying to Connect without any browser callback
    let result = fixture.connect();

    // Then Connection fails with timeout or browser error
    ExternalBrowserTestFixture::assert_error(
        result,
        &[
            "timeout",
            "Timeout",
            "browser",
            "Browser",
            "ExternalBrowser",
        ],
        "authentication timeout error",
    );
}

// =============================================================================
// Error Handling - Login Failure After Successful Browser Flow
// =============================================================================

#[test]
fn should_fail_when_login_request_is_rejected_after_browser_callback() {
    // Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
    let fixture = ExternalBrowserTestFixture::new();
    let proof_key = "test_proof_key";
    fixture.mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        proof_key,
    ));

    // And Login endpoint returns failure
    fixture.mock.mount(external_browser::login_failure());

    // When Trying to Connect with simulated browser callback delivering a token
    let mock_ref = &fixture.mock;
    let result = std::thread::scope(|s| {
        s.spawn(|| {
            for _ in 0..50 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let requests = mock_ref.received_requests();
                if requests
                    .iter()
                    .any(|r| r.url.path().contains("authenticator-request"))
                {
                    simulate_browser_callback(mock_ref, "valid_token");
                    return;
                }
            }
            panic!("Timed out waiting for authenticator-request");
        });

        fixture.connect()
    });

    // Then Connection fails with login error
    ExternalBrowserTestFixture::assert_error(
        result,
        &["login", "Login", "auth", "credentials", "Invalid"],
        "login failure after browser callback",
    );
}
