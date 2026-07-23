//! Live proof that `ConnectionAbortQuery` (abort by Snowflake Query ID)
//! actually stops a running query, verified via `ConnectionGetQueryStatus`
//! (more deterministic than QUERY_HISTORY_BY_SESSION, which has history
//! latency). Complements Python's `test_abort_query_returns_true_for_running_query`
//! (`python/tests/integ/test_cursor.py`), which asserts on the 57014 result error.

use crate::common::snowflake_test_client::{SnowflakeTestClient, unwrap_single_query_id};
use sf_core::protobuf::generated::database_driver_v1::AbortQueryOutcome;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(200);
const POLL_TIMEOUT: Duration = Duration::from_secs(20);

fn poll_until<F: Fn(&str) -> bool>(
    client: &SnowflakeTestClient,
    query_id: &str,
    done: F,
) -> String {
    let deadline = Instant::now() + POLL_TIMEOUT;
    loop {
        let status = client.get_query_status(query_id);
        if done(&status) {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for query {query_id} status; last status: {status}"
        );
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[test]
fn should_abort_running_query_by_query_id() {
    // Given a long-running query submitted without waiting for completion
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stmt = client.new_statement();
    client.set_sql_query(&stmt, "SELECT SYSTEM$WAIT(30, 'SECONDS')");
    let query_id = client.execute_statement_async(&stmt);

    // And it has actually started running
    poll_until(&client, &query_id, |status| status == "RUNNING");

    // When it is aborted by its query id
    let outcome = client.abort_query(&query_id);

    // Then the abort is acknowledged
    assert_eq!(
        outcome,
        AbortQueryOutcome::Aborted,
        "abort_query should acknowledge a running query"
    );

    // And the query reaches a terminal, non-successful state
    let final_status = poll_until(&client, &query_id, |status| status != "RUNNING");
    assert_ne!(
        final_status, "SUCCESS",
        "an aborted query must not complete successfully"
    );

    client.release_statement(&stmt);
}

#[test]
fn should_return_not_running_when_aborting_completed_query() {
    // Given a query that has already completed
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stmt = client.new_statement();
    client.set_sql_query(&stmt, "SELECT 1");
    let result = client.execute_statement_query(&stmt);
    let query_id = unwrap_single_query_id(&result);

    // When it is aborted by its query id
    let outcome = client.abort_query(&query_id);

    // Then the server declines: no query was actually running to abort
    assert_eq!(
        outcome,
        AbortQueryOutcome::NotRunning,
        "abort_query should decline a query that is not running"
    );

    client.release_statement(&stmt);
}
