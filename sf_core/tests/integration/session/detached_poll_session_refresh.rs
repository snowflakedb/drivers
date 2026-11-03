//! A detached-query poll that returns SessionExpired must refresh the token
//! and retry only the poll, not re-POST the query.
//!
//! Two code paths in `read_response_json` produce `SessionExpired`:
//!   - HTTP 200 with `success:false, code:"390112"` (body-level signal)
//!   - HTTP 401 (status-level signal, no body parsing)
//!
//! The WireMock scenario sequences:
//!   1. POST /queries/v1/query-request → 333334 with getResultUrl (detached)
//!   2. GET  /queries/<id>/result      → SessionExpired (390112 or 401)
//!   3. POST /session/token-request    → 200 with new tokens
//!   4. GET  /queries/<id>/result      → 200 success=true with data

use crate::common::mocks::password;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, Respond, ResponseTemplate};

struct DetachedPollHarness {
    mock: MockServerWithTls,
    client: SnowflakeTestClient,
}

impl DetachedPollHarness {
    fn new(
        poll_responder: impl Respond + 'static,
        refresh_responder: impl Respond + 'static,
    ) -> Self {
        let mock = MockServerWithTls::start();
        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("password", "test_password"); // pragma: allowlist secret

        mock.mount(password::login_success());
        mock.mount(
            Mock::given(method("POST"))
                .and(path_regex(r"/queries/v1/query-request.*"))
                .respond_with(DetachedQueryResponder),
        );
        mock.mount(
            Mock::given(method("GET"))
                .and(path_regex(r"/queries/detached-test-qid/result.*"))
                .respond_with(poll_responder),
        );
        mock.mount(
            Mock::given(method("POST"))
                .and(path_regex(r"/session/token-request.*"))
                .respond_with(refresh_responder),
        );

        Self { mock, client }
    }

    fn connect_and_query(&self) -> Result<(), String> {
        self.client.connect()?;
        self.client.execute_query_no_unwrap("SELECT 1").map(|_| ())
    }

    fn query_post_count(&self) -> usize {
        self.mock
            .received_requests()
            .into_iter()
            .filter(|r| r.url.path().contains("query-request"))
            .count()
    }

    fn poll_get_count(&self) -> usize {
        self.mock
            .received_requests()
            .into_iter()
            .filter(|r| r.url.path().contains("detached-test-qid/result"))
            .count()
    }

    fn refresh_post_count(&self) -> usize {
        self.mock
            .received_requests()
            .into_iter()
            .filter(|r| r.url.path().contains("token-request"))
            .count()
    }

    fn poll_requests(&self) -> Vec<wiremock::Request> {
        self.mock
            .received_requests()
            .into_iter()
            .filter(|r| r.url.path().contains("detached-test-qid/result"))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Responders
// ---------------------------------------------------------------------------

struct DetachedQueryResponder;

impl Respond for DetachedQueryResponder {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "code": "333334",
            "data": {
                "queryId": "detached-test-qid",
                "getResultUrl": "/queries/detached-test-qid/result",
                "queryResultFormat": "json",
                "rowtype": null,
                "rowset": null,
                "rowsetBase64": null,
                "chunks": null,
                "parameters": []
            }
        }))
    }
}

fn success_poll_body() -> serde_json::Value {
    json!({
        "success": true,
        "data": {
            "queryId": "detached-test-qid",
            "queryResultFormat": "json",
            "rowtype": [{"name": "STATUS", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
            "rowset": [["Statement executed successfully."]],
            "total": 1,
            "returned": 1,
            "parameters": [],
            "statementTypeId": 16384
        }
    })
}

struct Poll390112ThenSuccess(AtomicUsize);

impl Poll390112ThenSuccess {
    fn new() -> Self {
        Self(AtomicUsize::new(0))
    }
}

impl Respond for Poll390112ThenSuccess {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        let n = self.0.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            ResponseTemplate::new(200).set_body_json(json!({
                "success": false,
                "code": "390112",
                "message": "Your session has expired. Please login again.",
                "data": {}
            }))
        } else {
            ResponseTemplate::new(200).set_body_json(success_poll_body())
        }
    }
}

struct Poll401ThenSuccess(AtomicUsize);

impl Poll401ThenSuccess {
    fn new() -> Self {
        Self(AtomicUsize::new(0))
    }
}

impl Respond for Poll401ThenSuccess {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        let n = self.0.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            ResponseTemplate::new(401)
        } else {
            ResponseTemplate::new(200).set_body_json(success_poll_body())
        }
    }
}

struct RefreshResponder;

impl Respond for RefreshResponder {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "sessionToken": "refreshed-session-token",
                "masterToken": "refreshed-master-token",
                "sessionId": 12345
            }
        }))
    }
}

struct RefreshFailureResponder;

impl Respond for RefreshFailureResponder {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "code": "390114",
            "message": "Master token expired.",
            "data": null
        }))
    }
}

// ---------------------------------------------------------------------------
// Tests — body-level 390112 triggers refresh
// ---------------------------------------------------------------------------

#[test]
fn poll_390112_retries_only_the_poll_not_the_query_post() {
    let h = DetachedPollHarness::new(Poll390112ThenSuccess::new(), RefreshResponder);

    h.connect_and_query().unwrap();

    assert_eq!(
        h.query_post_count(),
        1,
        "token renewal must retry only the poll, not resubmit the query"
    );
    assert_eq!(
        h.poll_get_count(),
        2,
        "expected 2 GETs (first returns 390112, second succeeds)"
    );
    assert_eq!(h.refresh_post_count(), 1, "expected 1 token refresh");
}

#[test]
fn second_poll_carries_the_refreshed_token() {
    let h = DetachedPollHarness::new(Poll390112ThenSuccess::new(), RefreshResponder);

    h.connect_and_query().unwrap();

    let polls = h.poll_requests();
    assert_eq!(polls.len(), 2, "expected 2 poll GETs");

    let first_auth = polls[0]
        .headers
        .get("authorization")
        .expect("first poll must have Authorization header")
        .to_str()
        .unwrap();
    let second_auth = polls[1]
        .headers
        .get("authorization")
        .expect("second poll must have Authorization header")
        .to_str()
        .unwrap();

    assert!(
        first_auth.contains("mock_session_token"),
        "first poll should carry the original session token, got {first_auth}"
    );
    assert!(
        second_auth.contains("refreshed-session-token"),
        "second poll should carry the refreshed token, got {second_auth}"
    );
}

#[test]
fn refresh_failure_during_poll_does_not_repost_the_query() {
    let h = DetachedPollHarness::new(Poll390112ThenSuccess::new(), RefreshFailureResponder);

    h.client.connect().unwrap();
    let err = h
        .client
        .execute_query_no_unwrap("SELECT 1")
        .expect_err("query should fail when token refresh fails");
    assert!(
        err.contains("390114"),
        "expected master-token terminal error (GS 390114), got: {err}"
    );

    assert_eq!(h.query_post_count(), 1, "the query must not be re-POSTed");
    assert_eq!(
        h.poll_get_count(),
        1,
        "only one poll GET before the refresh fails"
    );
}

// ---------------------------------------------------------------------------
// Tests — HTTP 401 triggers refresh (no 390112 in the body)
// ---------------------------------------------------------------------------

#[test]
fn poll_401_retries_only_the_poll_not_the_query_post() {
    let h = DetachedPollHarness::new(Poll401ThenSuccess::new(), RefreshResponder);

    h.connect_and_query().unwrap();

    assert_eq!(
        h.query_post_count(),
        1,
        "token renewal must retry only the poll, not resubmit the query"
    );
    assert_eq!(
        h.poll_get_count(),
        2,
        "expected 2 GETs (first returns 401, second succeeds)"
    );
    assert_eq!(h.refresh_post_count(), 1, "expected 1 token refresh");

    let polls = h.poll_requests();
    let first_auth = polls[0]
        .headers
        .get("authorization")
        .expect("first poll must have Authorization header")
        .to_str()
        .unwrap();
    let second_auth = polls[1]
        .headers
        .get("authorization")
        .expect("second poll must have Authorization header")
        .to_str()
        .unwrap();

    assert!(
        first_auth.contains("mock_session_token"),
        "first poll should carry the original session token, got {first_auth}"
    );
    assert!(
        second_auth.contains("refreshed-session-token"),
        "second poll should carry the refreshed token, got {second_auth}"
    );
}
