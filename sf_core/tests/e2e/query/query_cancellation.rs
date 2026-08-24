//! Live proof that cancelling a query **stops it on the server**, not just
//! locally.
//!
//! The distinction matters and nothing covered it before: a driver that only
//! drops its in-flight request returns promptly to the caller while the query
//! keeps running and consuming credits. Every assertion here is therefore about
//! observable *server-side* state.
//!
//! Verification is by side effect rather than by query status, because the paths
//! under test never learn the server-assigned query id — cancellation is keyed on
//! the client-generated `requestId`, and the id only comes back in a response we
//! never receive. So each test runs an `INSERT ... SYSTEM$WAIT(n)`: if the abort
//! landed, the statement never commits and the table stays empty; if it did not,
//! the insert completes and the row appears. No query-history latency, no polling
//! for a status that may never be attributed to us.
//!
//! The two tests establish "the insert can no longer commit" differently, which is
//! why only one of them pays a wall-clock wait:
//!
//! * **Cancel** — the executing call blocks until the *server* fails the query and
//!   returns gsCode 604. Receiving that response is itself proof the query is
//!   terminated, so the row check can happen immediately.
//! * **Timeout** — the call returns on a *local* deadline, which says nothing about
//!   the server. There is no id to ask about, so the only available proof is
//!   outliving the query's own duration and finding no row.
//!
//! Complements [`super::abort_query`], which covers the abort-by-query-id
//! primitive (`ConnectionAbortQuery`) and can use `ConnectionGetQueryStatus`
//! because an async submission does return a query id.

use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::test_utils::{TableCleanupGuard, unique_table_name};
use sf_core::protobuf::generated::database_driver_v1::AbortQueryOutcome;
use std::time::{Duration, Instant};

/// Query duration for the cancel test. Never waited out — it only has to outlast
/// getting a cancel in flight, and the cancel is retried until it lands rather
/// than issued after a fixed sleep.
const CANCEL_QUERY_SECONDS: u64 = 10;

/// Query duration for the timeout test. This one *is* waited out, so it is kept
/// as short as it can be while still comfortably outlasting
/// [`TIMEOUT_SECONDS`] — it sets the floor on that test's runtime.
const TIMEOUT_QUERY_SECONDS: u64 = 5;

/// Client-side query timeout for the timeout test — well below
/// [`TIMEOUT_QUERY_SECONDS`], so the timeout is what ends the query.
const TIMEOUT_SECONDS: u64 = 2;

/// Extra margin waited past [`TIMEOUT_QUERY_SECONDS`] before asserting the table
/// is still empty, to absorb queue/compile/commit time on a cold warehouse.
///
/// **This is the one constant that must not be trimmed aggressively.** `started`
/// is taken before the query is submitted, so `started + TIMEOUT_QUERY_SECONDS`
/// is *earlier* than the moment a surviving insert would commit. Too small a
/// margin therefore makes the "table is empty" assertion pass merely because the
/// insert had not committed yet — a false pass that hides exactly the bug this
/// test exists to catch.
const SETTLE_MARGIN: Duration = Duration::from_secs(6);

/// Bound on how long to keep retrying the cancel while waiting for the query to
/// reach the server.
const CANCEL_ARRIVAL_BOUND: Duration = Duration::from_secs(8);

/// gsCode Snowflake returns on a query it has cancelled. Receiving it is what
/// proves the *server* terminated the query, as opposed to us giving up locally.
const QUERY_CANCELED_CODE: i32 = 604;

/// A statement that takes `seconds` and leaves a durable, observable trace
/// **only if it runs to completion**.
fn slow_insert(table: &str, seconds: u64) -> String {
    format!("INSERT INTO {table} SELECT SYSTEM$WAIT({seconds}, 'SECONDS')")
}

/// Drop `table` on scope exit even if an assertion panics. Bound as
/// `let _cleanup = …` (never `let _ = …`, which would drop it immediately).
fn cleanup_table<'a>(client: &'a SnowflakeTestClient, table: &str) -> TableCleanupGuard<'a> {
    TableCleanupGuard::new(table.to_string(), move |name| {
        client.execute_sql(&format!("DROP TABLE IF EXISTS {name}"));
    })
}

fn row_count(client: &SnowflakeTestClient, table: &str) -> i64 {
    let result = client.execute_query(&format!("SELECT COUNT(*) FROM {table}"));
    let mut helper = ArrowResultHelper::from_result(result);
    let rows = helper.transform_into_array::<i64>().unwrap();
    rows[0][0]
}

/// Sleep until `started + TIMEOUT_QUERY_SECONDS + SETTLE_MARGIN`, so the
/// assertion that follows is made after the point the query *would* have
/// committed.
fn wait_past_natural_completion(started: Instant) {
    let settle_at = started + Duration::from_secs(TIMEOUT_QUERY_SECONDS) + SETTLE_MARGIN;
    if let Some(remaining) = settle_at.checked_duration_since(Instant::now()) {
        std::thread::sleep(remaining);
    }
}

/// The gsCode the server reported on the failed execute, or `None` if the failure
/// was not a server-reported application error.
fn server_error_code(
    result: &Result<
        sf_core::protobuf::generated::database_driver_v1::ExecuteQueryResponse,
        Box<
            proto_utils::ProtoError<
                sf_core::protobuf::generated::database_driver_v1::DriverException,
            >,
        >,
    >,
) -> Option<i32> {
    match result {
        Err(boxed) => match &**boxed {
            proto_utils::ProtoError::Application(driver_exception) => driver_exception.vendor_code,
            proto_utils::ProtoError::Transport(_) => None,
        },
        Ok(_) => None,
    }
}

#[test]
fn should_abort_query_on_server_when_statement_is_cancelled() {
    // Given a table to observe, and a long-running insert into it
    let client = SnowflakeTestClient::connect_with_default_auth();
    let table = unique_table_name("cancel_server_side");
    client.execute_sql(&format!("CREATE OR REPLACE TABLE {table} (waited STRING)"));
    let _cleanup = cleanup_table(&client, &table);

    let stmt = client.new_statement();
    client.set_sql_query(&stmt, &slow_insert(&table, CANCEL_QUERY_SECONDS));

    // When it is cancelled from another thread while in flight
    // Scoped threads so the executor can borrow the client — the cancel must come
    // from a *different* thread than the one blocked in execute, which is the
    // whole shape being tested.
    let exec_result = std::thread::scope(|scope| {
        // Blocks until the server fails the query.
        let executor = scope.spawn(|| client.execute_statement_query_raw(&stmt));

        // Retry the cancel until it actually claims a running query. A cancel that
        // lands before the query-request reaches the server finds an empty
        // in-flight slot and reports NOT_RUNNING without marking anything, so
        // retrying is safe — and it is what removes the need for a padded sleep
        // sized for a cold warehouse.
        let deadline = Instant::now() + CANCEL_ARRIVAL_BOUND;
        loop {
            let outcome = client
                .statement_cancel_blocking(&stmt)
                .expect("cancel RPC should not transport-fail")
                .outcome;
            if AbortQueryOutcome::try_from(outcome) == Ok(AbortQueryOutcome::Aborted) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "query never became cancellable within {CANCEL_ARRIVAL_BOUND:?}; \
                 last cancel outcome: {outcome:?}"
            );
            std::thread::sleep(Duration::from_millis(200));
        }

        executor.join().expect("executor thread panicked")
    });

    // Then the executing call fails with the *server's* cancellation code, not
    // some local give-up. This is the load-bearing assertion: had the abort not
    // reached the server, this call would have waited out CANCEL_QUERY_SECONDS and
    // returned a successful insert instead.
    assert_eq!(
        server_error_code(&exec_result),
        Some(QUERY_CANCELED_CODE),
        "cancelled insert must fail with the server's gsCode {QUERY_CANCELED_CODE}, \
         got {exec_result:?}"
    );

    // And the insert left no row. Sound to check immediately: the server having
    // reported the query cancelled means it is already terminated, so it can never
    // commit — no need to outlive its natural duration the way the timeout test does.
    assert_eq!(
        row_count(&client, &table),
        0,
        "cancelled insert must leave no row"
    );

    client.release_statement(&stmt);
}

#[test]
fn should_abort_query_on_server_when_client_side_query_timeout_fires() {
    // Given a connection whose client-side QUERY_TIMEOUT is shorter than the query
    let client = SnowflakeTestClient::with_default_jwt_auth_params();
    client.set_connection_option("QUERY_TIMEOUT", &TIMEOUT_SECONDS.to_string());
    client.connect().unwrap();

    let table = unique_table_name("timeout_server_side");
    client.execute_sql(&format!("CREATE OR REPLACE TABLE {table} (waited STRING)"));
    let _cleanup = cleanup_table(&client, &table);

    let stmt = client.new_statement();
    client.set_sql_query(&stmt, &slow_insert(&table, TIMEOUT_QUERY_SECONDS));

    // When the query outlives that timeout
    let started = Instant::now();
    let exec_result = client.execute_statement_query_raw(&stmt);

    // Then the call fails with a timeout, and does so at the timeout rather than
    // waiting out the query
    assert!(
        exec_result.is_err(),
        "a query exceeding QUERY_TIMEOUT must fail"
    );
    assert!(
        started.elapsed() < Duration::from_secs(TIMEOUT_QUERY_SECONDS),
        "QUERY_TIMEOUT must end the call early, not after the query's own duration"
    );

    // And the query is aborted server-side rather than left running to
    // completion — the regression this guards is a client-side timeout that
    // gives up locally while the server keeps burning credits
    wait_past_natural_completion(started);
    assert_eq!(
        row_count(&client, &table),
        0,
        "timed-out insert must leave no row: the query was not aborted server-side"
    );

    client.release_statement(&stmt);
}
