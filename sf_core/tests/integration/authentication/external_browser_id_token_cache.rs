use std::io::{Read, Write};

use crate::common::mocks::external_browser;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;
use sf_core::token_cache::{KeyringTokenCache, TokenCache, TokenType};

// =============================================================================
// Test Fixture
// =============================================================================

struct IdTokenCacheFixture {
    mock: MockServerWithTls,
    client: SnowflakeTestClient,
    cache: KeyringTokenCache,
    host: String,
    user: String,
}

impl IdTokenCacheFixture {
    fn new(user: &str) -> Self {
        unsafe { std::env::set_var("SF_TEST_BROWSER_OPENER", "noop") };

        let mock = MockServerWithTls::start();
        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("authenticator", "EXTERNALBROWSER");
        client.set_connection_option("user", user);
        client.set_connection_option("authentication_timeout", "10");
        client.set_connection_option("client_store_temporary_credential", "true");

        let cache = KeyringTokenCache::new().expect("token cache should be available");
        let host = url::Url::parse(&mock.http_url())
            .expect("mock URL should be valid")
            .host_str()
            .expect("mock URL should have a host")
            .to_string();

        Self {
            mock,
            client,
            cache,
            host,
            user: user.to_string(),
        }
    }

    fn seed_cached_id_token(&self) {
        self.cache
            .add_token(
                &self.host,
                &self.user,
                TokenType::IdToken,
                "cached_id_token",
            )
            .expect("failed to seed ID token in cache");
    }

    fn connect(&self) -> Result<(), String> {
        self.client.connect()
    }

    fn cached_id_token(&self) -> Option<String> {
        self.cache
            .get_token(&self.host, &self.user, TokenType::IdToken)
            .expect("get_token should not fail")
    }
}

impl Drop for IdTokenCacheFixture {
    fn drop(&mut self) {
        let _ = self
            .cache
            .remove_token(&self.host, &self.user, TokenType::IdToken);
    }
}

fn assert_success(result: Result<(), String>, context: &str) {
    assert!(result.is_ok(), "Expected {context}, got: {result:?}");
}

fn assert_error(result: Result<(), String>, patterns: &[&str], context: &str) {
    let error = result.expect_err(&format!("Expected {context} to fail"));
    let matches = patterns.iter().any(|p| error.contains(p));
    assert!(matches, "Expected {context}, got: {error}");
}

/// Simulate a browser callback delivering a token to the loopback server.
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
// Cached ID Token - Happy Path
// =============================================================================

#[test]
fn should_authenticate_with_cached_id_token() {
    let fixture = IdTokenCacheFixture::new("eb_cache_hit");
    fixture.seed_cached_id_token();
    fixture
        .mock
        .mount(external_browser::login_success_with_cached_id_token());

    let result = fixture.connect();

    assert_success(result, "login with cached ID token to succeed");

    let requests = fixture.mock.received_requests();
    assert!(
        !requests
            .iter()
            .any(|r| r.url.path().contains("authenticator-request")),
        "Should skip authenticator-request when using cached ID token"
    );
    let login_req = requests
        .iter()
        .find(|r| r.url.path().contains("login-request"))
        .expect("No login-request was captured");
    let body: serde_json::Value = serde_json::from_slice(&login_req.body).unwrap();
    assert_eq!(body["data"]["AUTHENTICATOR"], "ID_TOKEN");
    assert_eq!(body["data"]["TOKEN"], "cached_id_token");
    assert!(
        body["data"]["PROOF_KEY"].is_null(),
        "PROOF_KEY should not be sent when using cached ID token"
    );
}

// =============================================================================
// ID Token Stored After Successful Browser Flow
// =============================================================================

#[test]
fn should_store_id_token_after_successful_browser_login() {
    let fixture = IdTokenCacheFixture::new("eb_cache_store");
    fixture.mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        "proof_key_store_test",
    ));
    fixture
        .mock
        .mount(external_browser::login_success_with_id_token_in_response());

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
                    simulate_browser_callback(mock_ref, "browser_token_xyz");
                    return;
                }
            }
            panic!("Timed out waiting for authenticator-request");
        });
        fixture.connect()
    });

    assert_success(result, "browser login to succeed");

    let cached = fixture.cached_id_token();
    assert_eq!(
        cached.as_deref(),
        Some("server_issued_id_token"),
        "ID token from login response should be stored in cache"
    );
}

// =============================================================================
// EXT_AUTHN Error Evicts Cached ID Token
// =============================================================================

fn assert_ext_authn_error_evicts_cached_id_token(
    mount_failure: fn() -> wiremock::Mock,
    code: &str,
    user: &str,
) {
    let fixture = IdTokenCacheFixture::new(user);
    fixture.seed_cached_id_token();

    // First request with cached token fails with EXT_AUTHN error.
    fixture.mock.mount(mount_failure());
    // Retry goes through browser flow, which also fails (no callback → timeout).
    // We set a short timeout so the test doesn't hang.
    fixture
        .client
        .set_connection_option("authentication_timeout", "2");
    fixture.mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        "retry_proof_key",
    ));

    let result = fixture.connect();

    assert_error(
        result,
        &[
            "timeout",
            "Timeout",
            "browser",
            "Browser",
            "ExternalBrowser",
            "login",
            "Login",
        ],
        &format!("login to fail after evict-and-retry for EXT_AUTHN code {code}"),
    );

    let cached = fixture.cached_id_token();
    assert!(
        cached.is_none(),
        "Expected cached ID token to be removed after EXT_AUTHN error {code}, but it still exists"
    );
}

#[test]
fn should_evict_cached_id_token_on_ext_authn_denied() {
    assert_ext_authn_error_evicts_cached_id_token(
        external_browser::login_failure_ext_authn_denied_cached_id,
        "390120",
        "eb_evict_denied",
    );
}

#[test]
fn should_evict_cached_id_token_on_ext_authn_locked() {
    assert_ext_authn_error_evicts_cached_id_token(
        external_browser::login_failure_ext_authn_locked_cached_id,
        "390123",
        "eb_evict_locked",
    );
}

#[test]
fn should_evict_cached_id_token_on_ext_authn_timeout() {
    assert_ext_authn_error_evicts_cached_id_token(
        external_browser::login_failure_ext_authn_timeout_cached_id,
        "390126",
        "eb_evict_timeout",
    );
}

#[test]
fn should_evict_cached_id_token_on_ext_authn_invalid() {
    assert_ext_authn_error_evicts_cached_id_token(
        external_browser::login_failure_ext_authn_invalid_cached_id,
        "390127",
        "eb_evict_invalid",
    );
}

#[test]
fn should_evict_cached_id_token_on_ext_authn_exception() {
    assert_ext_authn_error_evicts_cached_id_token(
        external_browser::login_failure_ext_authn_exception_cached_id,
        "390129",
        "eb_evict_exception",
    );
}

// =============================================================================
// EXT_AUTHN Retry Succeeds After Eviction
// =============================================================================

#[test]
fn should_retry_with_browser_flow_when_cached_id_token_fails() {
    let fixture = IdTokenCacheFixture::new("eb_retry_ok");
    fixture.seed_cached_id_token();

    // First request with cached token → EXT_AUTHN denied.
    fixture
        .mock
        .mount(external_browser::login_failure_ext_authn_denied_cached_id());
    // Retry → browser flow → success with a fresh ID token in response.
    fixture.mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        "retry_proof_key",
    ));
    fixture
        .mock
        .mount(external_browser::login_success_with_id_token_in_response());

    let mock_ref = &fixture.mock;
    let result = std::thread::scope(|s| {
        s.spawn(|| {
            // Wait for the retry's authenticator-request (the second one).
            // The first login-request uses the cached token (no authenticator-request).
            for _ in 0..80 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let requests = mock_ref.received_requests();
                if requests
                    .iter()
                    .any(|r| r.url.path().contains("authenticator-request"))
                {
                    simulate_browser_callback(mock_ref, "retry_browser_token");
                    return;
                }
            }
            panic!("Timed out waiting for authenticator-request on retry");
        });
        fixture.connect()
    });

    assert_success(result, "login retry via browser flow to succeed");

    let cached = fixture.cached_id_token();
    assert_eq!(
        cached.as_deref(),
        Some("server_issued_id_token"),
        "New ID token should be cached after successful retry"
    );
}

// =============================================================================
// Caching Disabled — Token Not Stored
// =============================================================================

#[test]
fn should_not_cache_id_token_when_caching_disabled() {
    unsafe { std::env::set_var("SF_TEST_BROWSER_OPENER", "noop") };

    let mock = MockServerWithTls::start();
    let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
    client.set_connection_option("authenticator", "EXTERNALBROWSER");
    client.set_connection_option("user", "eb_no_cache");
    client.set_connection_option("authentication_timeout", "10");
    // client_store_temporary_credential is NOT set (defaults to false)

    mock.mount(external_browser::authenticator_request(
        "https://idp.example.com/sso",
        "proof_key_no_cache",
    ));
    mock.mount(external_browser::login_success_with_id_token_in_response());

    let mock_ref = &mock;
    let result = std::thread::scope(|s| {
        s.spawn(|| {
            for _ in 0..50 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let requests = mock_ref.received_requests();
                if requests
                    .iter()
                    .any(|r| r.url.path().contains("authenticator-request"))
                {
                    simulate_browser_callback(mock_ref, "token_no_cache");
                    return;
                }
            }
            panic!("Timed out waiting for authenticator-request");
        });
        client.connect()
    });

    assert_success(result, "browser login to succeed without caching");

    let cache = KeyringTokenCache::new().expect("token cache should be available");
    let host = url::Url::parse(&mock.http_url())
        .expect("mock URL should be valid")
        .host_str()
        .expect("mock URL should have a host")
        .to_string();
    let cached = cache
        .get_token(&host, "eb_no_cache", TokenType::IdToken)
        .expect("get_token should not fail");
    assert!(
        cached.is_none(),
        "ID token should NOT be cached when client_store_temporary_credential is false"
    );
}
