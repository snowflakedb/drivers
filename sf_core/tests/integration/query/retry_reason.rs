//! Wiremock integration tests for retry reason context on retried query requests.
//!
//! Implements scenarios from: tests/definitions/shared/query/retry_reason.feature
//!
//! The HTTP-level retry mechanics (retryCount/retryReason params) are tested in
//! `sf_core/tests/integration/http/sync_retry.rs`. This module provides the
//! feature-file-aligned test names that the CI validator expects.

use crate::common::mocks::password;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, Respond, ResponseTemplate};

// ---------------------------------------------------------------------------
// Scenario: should include retryReason with HTTP status code on server error retry
// ---------------------------------------------------------------------------

#[test]
fn should_include_retry_reason_with_http_status_code_on_server_error_retry() {
    for status_code in [503, 429] {
        // Given a wiremock server that returns <status_code> on the first query then succeeds
        let fixture = RetryReasonFixture::new();
        fixture.mount_login();
        fixture.mock.mount(
            Mock::given(method("POST"))
                .and(path_regex(r"/queries/v1/query-request.*"))
                .respond_with(ErrorCodeThenSuccess::new(status_code)),
        );

        // When the client executes a query
        fixture.client.connect().unwrap();
        fixture.client.execute_query_no_unwrap("SELECT 1").unwrap();

        // Then the retry request includes retryCount=1
        let requests = fixture.query_requests();
        assert_eq!(
            requests.len(),
            2,
            "Expected 2 query requests (1 initial + 1 retry)"
        );
        let request = &requests[1];
        assert!(
            request.contains("retryCount=1"),
            "Retry should include retryCount=1: {}",
            request
        );

        // And the retry request includes retryReason=<status_code>
        assert!(
            request.contains(&format!("retryReason={}", status_code)),
            "Retry should include retryReason={}: {}",
            status_code,
            request
        );
    }
}

// ---------------------------------------------------------------------------
// Scenario: should include retryReason with 503 on async retry
// ---------------------------------------------------------------------------

#[test]
fn should_include_retry_reason_with_503_on_async_retry() {
    // Given a wiremock server that returns 503 on the first query then succeeds
    let fixture = RetryReasonFixture::new();
    fixture.mount_login();
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/queries/v1/query-request.*"))
            .respond_with(AsyncErrorCodeThenSuccess::new(503)),
    );

    // When the client executes an async query
    fixture.client.connect().unwrap();
    let stmt = fixture.client.new_statement();
    fixture.client.set_statement_async_execution(&stmt, true);
    fixture.client.set_sql_query(&stmt, "SELECT 1");
    let query_id = fixture.client.execute_statement_async(&stmt);
    assert!(
        !query_id.is_empty(),
        "Async submit should return a query id"
    );

    // Then the retry request includes retryCount=1
    let requests = fixture.query_requests();
    assert_eq!(
        requests.len(),
        2,
        "Expected 2 query requests (1 initial + 1 retry)"
    );
    let second = &requests[1];
    assert!(
        second.contains("retryCount=1"),
        "Async retry should include retryCount=1: {}",
        second
    );

    // And the retry request includes retryReason=503
    assert!(
        second.contains("retryReason=503"),
        "Async retry should include retryReason=503: {}",
        second
    );
}

// ---------------------------------------------------------------------------
// Scenario: should suppress retryReason on async submit when disabled
// ---------------------------------------------------------------------------

#[test]
fn should_suppress_retry_reason_on_async_submit_when_disabled() {
    // Given a wiremock server that returns 503 on the first query then succeeds
    let fixture = RetryReasonFixture::new();
    fixture.mount_login();
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/queries/v1/query-request.*"))
            .respond_with(AsyncErrorCodeThenSuccess::new(503)),
    );

    // And the connection has includeRetryReason set to false
    fixture
        .client
        .set_connection_option_bool("include_retry_reason", false);

    // When the client executes an async query
    fixture.client.connect().unwrap();
    let stmt = fixture.client.new_statement();
    fixture.client.set_statement_async_execution(&stmt, true);
    fixture.client.set_sql_query(&stmt, "SELECT 1");
    let query_id = fixture.client.execute_statement_async(&stmt);
    assert!(
        !query_id.is_empty(),
        "Async submit should return a query id"
    );

    // Then the retry request includes retryCount=1
    let requests = fixture.query_requests();
    assert_eq!(
        requests.len(),
        2,
        "Expected 2 query requests (1 initial + 1 retry)"
    );
    let second = &requests[1];
    assert!(
        second.contains("retryCount=1"),
        "Async retry should still include retryCount=1: {}",
        second
    );

    // But the retry request does not include retryReason
    assert!(
        !second.contains("retryReason"),
        "Async retry should NOT include retryReason when disabled: {}",
        second
    );
}

// ---------------------------------------------------------------------------
// Scenario: should suppress retryReason when includeRetryReason is disabled
// ---------------------------------------------------------------------------

#[test]
fn should_suppress_retry_reason_when_include_retry_reason_is_disabled() {
    // Given a wiremock server that returns 503 on the first query then succeeds
    let fixture = RetryReasonFixture::new();
    fixture.mount_login();
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/queries/v1/query-request.*"))
            .respond_with(ErrorCodeThenSuccess::new(503)),
    );

    // And the connection has includeRetryReason set to false
    fixture
        .client
        .set_connection_option_bool("include_retry_reason", false);

    // When the client executes a query
    fixture.client.connect().unwrap();
    fixture.client.execute_query_no_unwrap("SELECT 1").unwrap();

    // Then the retry request includes retryCount=1
    let requests = fixture.query_requests();
    assert_eq!(
        requests.len(),
        2,
        "Expected 2 query requests (1 initial + 1 retry)"
    );
    let second = &requests[1];
    assert!(
        second.contains("retryCount=1"),
        "Retry should still include retryCount=1: {}",
        second
    );

    // But the retry request does not include retryReason
    assert!(
        !second.contains("retryReason"),
        "Retry should NOT include retryReason when disabled: {}",
        second
    );
}

// ---------------------------------------------------------------------------
// Scenario: should not include retry params on non-query endpoints
// ---------------------------------------------------------------------------

#[test]
fn should_not_include_retry_params_on_non_query_endpoints() {
    // Given a wiremock server that returns 503 on the first login then succeeds
    let fixture = RetryReasonFixture::new();
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/session/v1/login-request"))
            .respond_with(LoginErrorThenSuccess::new(503)),
    );

    // When the client connects
    fixture.client.connect().unwrap();

    // Then the login retry request does not include retryCount
    let login_urls = fixture.login_requests();
    assert!(
        login_urls.len() >= 2,
        "Expected at least 2 login requests (1 failure + 1 retry), got {}",
        login_urls.len()
    );
    for url in &login_urls[1..] {
        assert!(
            !url.contains("retryCount"),
            "Login retry should not include retryCount: {}",
            url
        );
        // And the login retry request does not include retryReason
        assert!(
            !url.contains("retryReason"),
            "Login retry should not include retryReason: {}",
            url
        );
    }
}

// ---------------------------------------------------------------------------
// Scenario: should update retryReason on each subsequent retry
// ---------------------------------------------------------------------------

#[test]
fn should_update_retry_reason_on_each_subsequent_retry() {
    // Given a wiremock server that returns 429 then 503 then succeeds on query
    let fixture = RetryReasonFixture::new();
    fixture.mount_login();
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/queries/v1/query-request.*"))
            .respond_with(MultiErrorThenSuccess::new(vec![429, 503])),
    );

    // When the client executes a query
    fixture.client.connect().unwrap();
    fixture.client.execute_query_no_unwrap("SELECT 1").unwrap();

    let requests = fixture.query_requests();
    assert_eq!(
        requests.len(),
        3,
        "Expected 3 query requests (1 initial + 2 retries)"
    );

    // Then the second request includes retryCount=1 and retryReason=429
    assert!(
        requests[1].contains("retryCount=1"),
        "Second request should have retryCount=1: {}",
        requests[1]
    );
    assert!(
        requests[1].contains("retryReason=429"),
        "Second request should have retryReason=429: {}",
        requests[1]
    );

    // And the third request includes retryCount=2 and retryReason=503
    assert!(
        requests[2].contains("retryCount=2"),
        "Third request should have retryCount=2: {}",
        requests[2]
    );
    assert!(
        requests[2].contains("retryReason=503"),
        "Third request should have retryReason=503: {}",
        requests[2]
    );
}

// ---------------------------------------------------------------------------
// Scenario: should increment retryCount on each retry
// ---------------------------------------------------------------------------

#[test]
fn should_increment_retry_count_on_each_retry() {
    // Given a wiremock server that returns 503 twice then succeeds on query
    let fixture = RetryReasonFixture::new();
    fixture.mount_login();
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/queries/v1/query-request.*"))
            .respond_with(MultiErrorThenSuccess::new(vec![503, 503])),
    );

    // When the client executes a query
    fixture.client.connect().unwrap();
    fixture.client.execute_query_no_unwrap("SELECT 1").unwrap();

    let requests = fixture.query_requests();
    assert_eq!(
        requests.len(),
        3,
        "Expected 3 query requests (1 initial + 2 retries)"
    );

    // Then the second request includes retryCount=1
    assert!(
        requests[1].contains("retryCount=1"),
        "Second request should have retryCount=1: {}",
        requests[1]
    );

    // And the third request includes retryCount=2
    assert!(
        requests[2].contains("retryCount=2"),
        "Third request should have retryCount=2: {}",
        requests[2]
    );
}

// ---------------------------------------------------------------------------
// Scenario: should not include retry params on first request
// ---------------------------------------------------------------------------

#[test]
fn should_not_include_retry_params_on_first_request() {
    // Given a wiremock server that succeeds on query
    let fixture = RetryReasonFixture::new();
    fixture.mount_login();
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/queries/v1/query-request.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-ok",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": []
                }
            }))),
    );

    // When the client executes a query
    fixture.client.connect().unwrap();
    fixture.client.execute_query_no_unwrap("SELECT 1").unwrap();

    // Then the request does not include retryCount
    let requests = fixture.query_requests();
    assert!(!requests.is_empty());
    assert!(
        !requests[0].contains("retryCount"),
        "First request should not include retryCount: {}",
        requests[0]
    );

    // And the request does not include retryReason
    assert!(
        !requests[0].contains("retryReason"),
        "First request should not include retryReason: {}",
        requests[0]
    );
}

// ---------------------------------------------------------------------------
// Test fixture
// ---------------------------------------------------------------------------

struct RetryReasonFixture {
    mock: MockServerWithTls,
    client: SnowflakeTestClient,
}

impl RetryReasonFixture {
    fn new() -> Self {
        let mock = MockServerWithTls::start();
        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("password", "test_password"); // pragma: allowlist secret
        Self { mock, client }
    }

    fn mount_login(&self) {
        self.mock.mount(password::login_success());
    }

    fn query_requests(&self) -> Vec<String> {
        self.mock
            .received_requests()
            .into_iter()
            .filter(|r| r.url.path().contains("query-request"))
            .map(|r| r.url.to_string())
            .collect()
    }

    fn login_requests(&self) -> Vec<String> {
        self.mock
            .received_requests()
            .into_iter()
            .filter(|r| r.url.path().contains("login-request"))
            .map(|r| r.url.to_string())
            .collect()
    }
}

fn query_success_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "success": true,
        "data": {
            "queryId": "qid-retry-success",
            "queryResultFormat": "json",
            "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
            "rowset": [["OK"]],
            "total": 1, "returned": 1,
            "parameters": []
        }
    }))
}

fn error_response(status: u16) -> ResponseTemplate {
    ResponseTemplate::new(status)
        .set_body_string("Error")
        .append_header("Retry-After", "0")
}

/// Counter-backed responder: returns an HTTP error on the first call, success after.
struct ErrorCodeThenSuccess {
    calls: AtomicUsize,
    error_code: u16,
}

impl ErrorCodeThenSuccess {
    fn new(error_code: u16) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            error_code,
        }
    }
}

impl Respond for ErrorCodeThenSuccess {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            error_response(self.error_code)
        } else {
            query_success_response()
        }
    }
}

/// Counter-backed responder: returns each error code in sequence, then success.
struct MultiErrorThenSuccess {
    calls: AtomicUsize,
    error_codes: Vec<u16>,
}

impl MultiErrorThenSuccess {
    fn new(error_codes: Vec<u16>) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            error_codes,
        }
    }
}

impl Respond for MultiErrorThenSuccess {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n < self.error_codes.len() {
            error_response(self.error_codes[n])
        } else {
            query_success_response()
        }
    }
}

fn async_submit_success_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "success": true,
        "data": {
            "queryId": "qid-async-retry-success",
            "queryResultFormat": "json",
            "parameters": []
        }
    }))
}

/// Counter-backed responder for async submit: returns an HTTP error on the first
/// call, then an async submit success response (queryId only, no rowset).
struct AsyncErrorCodeThenSuccess {
    calls: AtomicUsize,
    error_code: u16,
}

impl AsyncErrorCodeThenSuccess {
    fn new(error_code: u16) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            error_code,
        }
    }
}

impl Respond for AsyncErrorCodeThenSuccess {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            error_response(self.error_code)
        } else {
            async_submit_success_response()
        }
    }
}

/// Counter-backed responder for login: returns an HTTP error on the first call,
/// then the standard login success response.
struct LoginErrorThenSuccess {
    calls: AtomicUsize,
    error_code: u16,
}

impl LoginErrorThenSuccess {
    fn new(error_code: u16) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            error_code,
        }
    }
}

impl Respond for LoginErrorThenSuccess {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            error_response(self.error_code)
        } else {
            password::success_login_response()
        }
    }
}
