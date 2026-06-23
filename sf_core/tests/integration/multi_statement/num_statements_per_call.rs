//! Regression: `MULTI_STATEMENT_COUNT` is per-statement at the Rust core
//! layer. The Python wrapper PR (#171) restored per-call semantics by
//! routing the option per-call; that fix is necessary but not sufficient —
//! if the Rust core ever moved the option into a connection- or
//! session-scoped store, the wrapper-level fix would silently stop working.
//!
//! This test pins the Rust-core scope by driving two executes through a
//! shared `SnowflakeTestClient` against a wiremock GS server. The first
//! sets `MULTI_STATEMENT_COUNT`; the second does not. The mocks are routed
//! by body presence, and `expect(N)` on each pins exactly which side handled
//! each request. A second-call leak would either re-trigger the
//! multistatement mock (failing its `expect(1)`) or — if the dispatch path
//! diverged — leave the single-statement mock unmatched (failing *its*
//! `expect(1)`). Either way the test fails loudly. The first execute's
//! Multi dispatch is asserted inline, so a separate count-carrying-dispatch
//! test would be redundant.
//!
//! Scope: this is *not* a Python-wrapper-lifecycle regression — that lives
//! in `python/tests/unit/test_cursor.py::TestExecuteNumStatements`. This is
//! the Rust-side complement that pins the layer below.

use crate::common::mocks;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use sf_core::protobuf::generated::database_driver_v1::execute_query_response;
use wiremock::MockServer;

#[tokio::test(flavor = "multi_thread")]
async fn multi_statement_count_does_not_persist_across_executes() {
    let gs_server = MockServer::start().await;
    mocks::auth::mount_jwt_login_success(&gs_server).await;
    // First mock matches only when MULTI_STATEMENT_COUNT is in the body.
    // If the second call leaks the param, this mock matches twice and its
    // expect(1) fails on MockServer drop — that is the regression seam.
    mocks::query::mount_multistatement_response_for_count_carrying_request(
        &gs_server,
        &["child-q1", "child-q2"],
        /* expected_calls */ 1,
    )
    .await;
    // Second mock catches the single-statement execute. Any non-leaking
    // second call lands here; expect(1) pins the count.
    mocks::query::mount_single_statement_response(&gs_server, /* expected_calls */ 1).await;

    let gs_uri = gs_server.uri();
    tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&gs_uri));

        let multi_result = client.execute_multistatement("SELECT 1; SELECT 2", 2);
        assert!(
            matches!(multi_result, execute_query_response::Result::Multi(_)),
            "first execute must dispatch as Multi; got {multi_result:?}",
        );

        let single_result = client
            .execute_query_no_unwrap("SELECT 99")
            .expect("single-statement execute must succeed");
        assert!(
            matches!(single_result, execute_query_response::Result::Single(_)),
            "second execute must dispatch as Single (no leaked count); got {single_result:?}",
        );
    })
    .await
    .unwrap();

    // Belt-and-suspenders: read raw request bodies and pin per-call presence
    // directly. The mock-routing assertion above catches *behavioural*
    // regressions; this catches the case where a future refactor changes
    // wire format such that "MULTI_STATEMENT_COUNT" no longer appears in
    // request bodies even when the option is set, which would silently
    // disable the routing-based check.
    let requests = gs_server
        .received_requests()
        .await
        .expect("wiremock should record requests");
    let query_request_bodies: Vec<String> = requests
        .iter()
        .filter(|r| r.url.path().contains("/queries/v1/query-request"))
        .map(|r| String::from_utf8_lossy(&r.body).into_owned())
        .collect();
    assert_eq!(
        query_request_bodies.len(),
        2,
        "expected exactly two query-request POSTs, got {}: {query_request_bodies:?}",
        query_request_bodies.len(),
    );
    assert!(
        query_request_bodies[0].contains("MULTI_STATEMENT_COUNT"),
        "first query-request body must carry MULTI_STATEMENT_COUNT; body: {}",
        query_request_bodies[0],
    );
    assert!(
        !query_request_bodies[1].contains("MULTI_STATEMENT_COUNT"),
        "second query-request body must NOT carry MULTI_STATEMENT_COUNT (per-statement scope); body: {}",
        query_request_bodies[1],
    );
}
