//! GS query-request mock helpers for non-PUT/GET integration tests.
//!
//! `auth.rs` mounts the login endpoint; `put_get.rs` mounts UPLOAD/DOWNLOAD
//! responses. This module covers the regular query flow — a multistatement
//! response (parsed when `statementTypeId == 0xA000`) and a minimal
//! single-statement response.

use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path_regex};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

/// Negative-body matcher: the request body must NOT contain `needle`.
/// Use with `.and(...)` to make the mock's selectivity explicit rather
/// than relying on wiremock's mock-resolution order.
struct BodyStringDoesNotContain(&'static str);

impl Match for BodyStringDoesNotContain {
    fn matches(&self, request: &Request) -> bool {
        std::str::from_utf8(&request.body)
            .map(|body| !body.contains(self.0))
            .unwrap_or(false)
    }
}

/// Server-side multistatement marker. `data.statementTypeId == 0xA000`
/// triggers the `Multi` dispatch path (see
/// `sf_core/src/apis/database_driver_v1/multistatement.rs`).
const STATEMENT_TYPE_MULTI: i64 = 0xA000;

/// Mount a multistatement query response, matched by the presence of
/// `MULTI_STATEMENT_COUNT` in the request body. The response carries
/// `statementTypeId: 0xA000` and a comma-separated `resultIds` list, which
/// the parser splits into individual child query IDs.
///
/// `child_query_ids` becomes `data.resultIds`. `expected_calls` is enforced
/// via `.expect(N)`; passing `1` is the typical regression-pin shape, where
/// any leak of `MULTI_STATEMENT_COUNT` from a later request would push the
/// hit count above the expected value and fail the test on `MockServer`
/// drop.
pub async fn mount_multistatement_response_for_count_carrying_request(
    server: &MockServer,
    child_query_ids: &[&str],
    expected_calls: u64,
) {
    let result_ids = child_query_ids.join(",");
    let result_types = vec!["4096"; child_query_ids.len()].join(",");
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .and(body_string_contains("MULTI_STATEMENT_COUNT"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true,
                    "data": {
                        "statementTypeId": STATEMENT_TYPE_MULTI,
                        "resultIds": result_ids,
                        "resultTypes": result_types,
                        "queryId": "multi-parent-id",
                        "queryResultFormat": "json",
                        "rowtype": [],
                    }
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .expect(expected_calls)
        .mount(server)
        .await;
}

/// Mount a minimal single-statement response. Explicitly matches only
/// query-request POSTs whose body does NOT contain `MULTI_STATEMENT_COUNT`,
/// so routing between this and the multistatement mock doesn't depend on
/// wiremock's mock-resolution order — a leak of the count from a prior
/// statement would no longer match here either, surfacing as an
/// unmatched-request failure rather than silently routing to the wrong
/// response. Use `expected_calls` to pin the exact count of
/// single-statement executes.
pub async fn mount_single_statement_response(server: &MockServer, expected_calls: u64) {
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .and(BodyStringDoesNotContain("MULTI_STATEMENT_COUNT"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true,
                    "data": {
                        "queryId": "single-id",
                        "queryResultFormat": "json",
                        "rowset": [],
                        "rowtype": [],
                    }
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .expect(expected_calls)
        .mount(server)
        .await;
}

/// Mount a response for `POST /queries/{query_id}/abort-request`, returning
/// `{"success": success}`. Use `success: false` to simulate the server
/// declining the abort (e.g. an already-completed query).
pub async fn mount_abort_query_response(server: &MockServer, success: bool, expected_calls: u64) {
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/.*/abort-request"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "success": success }))
                .insert_header("Content-Type", "application/json"),
        )
        .expect(expected_calls)
        .mount(server)
        .await;
}
