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
//! Both tests therefore pay a wall-clock wait before checking the table: neither
//! ending proves on its own that the query has *stopped*, only that it was told
//! to.
//!
//! * **Cancel** — the core observes the cancellation, fires the abort-request, and
//!   unwinds; the acknowledgement it carries back (`cancellation_abort_outcome`)
//!   proves the abort reached the server and was accepted, which is a different
//!   claim from "the query has finished dying". This used to be provable
//!   immediately, back when the executing call stayed parked until the server
//!   failed the query and returned gsCode 604 — it no longer waits for that.
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
use sf_core::protobuf::generated::database_driver_v1::CancellationAbortOutcome;
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

/// How long the first attempt lets the query travel before cancelling it. Each
/// retry doubles it — see [`cancel_in_flight_query`].
const FIRST_CANCEL_DELAY: Duration = Duration::from_secs(1);

/// Bound on how long to keep retrying the dispatch-then-cancel attempt while
/// waiting for the query to reach the server.
const CANCEL_ARRIVAL_BOUND: Duration = Duration::from_secs(30);

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

/// Sleep until `started + query_seconds + SETTLE_MARGIN`, so the assertion that
/// follows is made after the point the query *would* have committed had nothing
/// stopped it.
fn wait_past_natural_completion(started: Instant, query_seconds: u64) {
    let settle_at = started + Duration::from_secs(query_seconds) + SETTLE_MARGIN;
    if let Some(remaining) = settle_at.checked_duration_since(Instant::now()) {
        std::thread::sleep(remaining);
    }
}

/// What a raw execute returns: the response, or the `DriverException` the two
/// readers below inspect.
type ExecuteResult = Result<
    sf_core::protobuf::generated::database_driver_v1::ExecuteQueryResponse,
    Box<proto_utils::ProtoError<sf_core::protobuf::generated::database_driver_v1::DriverException>>,
>;

/// The gsCode the server reported on the failed execute, or `None` if the failure
/// was not a server-reported application error.
fn server_error_code(result: &ExecuteResult) -> Option<i32> {
    match result {
        Err(boxed) => match &**boxed {
            proto_utils::ProtoError::Application(driver_exception) => driver_exception.vendor_code,
            proto_utils::ProtoError::Transport(_) => None,
        },
        Ok(_) => None,
    }
}

/// What the abort fired on cancellation achieved, as reported on the cancelled
/// execute's error. `None` when the failure was not a server-reported application
/// error, or when no abort was issued at all.
fn cancellation_abort_outcome(result: &ExecuteResult) -> Option<CancellationAbortOutcome> {
    match result {
        Err(boxed) => match &**boxed {
            proto_utils::ProtoError::Application(driver_exception) => driver_exception
                .cancellation_abort_outcome
                .and_then(|o| CancellationAbortOutcome::try_from(o).ok()),
            proto_utils::ProtoError::Transport(_) => None,
        },
        Ok(_) => None,
    }
}

/// Dispatch the statement and cancel it from another thread, retrying the whole
/// attempt until the cancel actually catches a query the server is running.
///
/// A bare sleep before the cancel cannot be trusted: on a loaded or cold-warehouse
/// runner the query-request may not have reached the server yet, and cancelling
/// then aborts nothing. There is also nothing to poll for "is it cancellable yet"
/// — cancelling a handle is fire-and-forget — so the *acknowledgement on the
/// returned error* is used as that signal instead: anything other than a
/// server-confirmed abort means the attempt was too early, so back off and try
/// again with a fresh statement and handle.
///
/// Returns the execute result of the attempt that landed. Panics on the bound
/// rather than returning, so the caller cannot mistake "never got a cancel in
/// flight" for a driver failure.
fn cancel_in_flight_query(client: &SnowflakeTestClient, table: &str) -> ExecuteResult {
    let deadline = Instant::now() + CANCEL_ARRIVAL_BOUND;
    let mut delay = FIRST_CANCEL_DELAY;

    loop {
        let stmt = client.new_statement();
        client.set_sql_query(&stmt, &slow_insert(table, CANCEL_QUERY_SECONDS));
        let operation = client.register_operation();

        // Scoped threads so the executor can borrow the client — the cancel must
        // come from a *different* thread than the one blocked in execute, which is
        // the whole shape being tested.
        let result = std::thread::scope(|scope| {
            let executor =
                scope.spawn(|| client.execute_statement_query_cancellable_raw(&stmt, operation));
            std::thread::sleep(delay);
            client.cancel_operation(operation);
            executor.join().expect("executor thread panicked")
        });
        client.release_statement(&stmt);

        if landed_on_the_server(&result) {
            return result;
        }
        assert!(
            Instant::now() < deadline,
            "no attempt got a cancel in flight within {CANCEL_ARRIVAL_BOUND:?}; \
             last result: {result:?}"
        );
        delay *= 2;
    }
}

/// Whether the execute ended in a way that proves the abort reached the server.
///
/// Two endings are legitimate and both prove it, so this accepts either rather
/// than pinning a race:
///
/// * the core observes the cancellation and unwinds, carrying `ABORTED` — the
///   acknowledgement that the server accepted the abort-request. This is the
///   common ending, because the cancelled execute stops waiting for the
///   query-response.
/// * the abort lands first and the query-request comes back canceled, carrying the
///   server's gsCode 604. Rare, but it is the *server* reporting termination,
///   which is strictly stronger.
fn landed_on_the_server(result: &ExecuteResult) -> bool {
    cancellation_abort_outcome(result) == Some(CancellationAbortOutcome::Aborted)
        || server_error_code(result) == Some(QUERY_CANCELED_CODE)
}

#[test]
fn should_abort_query_on_server_when_statement_is_cancelled() {
    // Given a table to observe, and a long-running insert into it
    let client = SnowflakeTestClient::connect_with_default_auth();
    let table = unique_table_name("cancel_server_side");
    client.execute_sql(&format!("CREATE OR REPLACE TABLE {table} (waited STRING)"));
    let _cleanup = cleanup_table(&client, &table);

    // When it is cancelled from another thread while in flight. `started` is taken
    // after the attempt that landed, so the settle wait below covers that query's
    // own duration rather than any earlier discarded attempt's.
    let exec_result = cancel_in_flight_query(&client, &table);
    let started = Instant::now();

    // Then the executing call proves the abort reached the server, which is what
    // `cancel_in_flight_query` retried until it saw. Re-asserted here so this test
    // states its own postcondition rather than relying on the helper's loop.
    assert!(
        landed_on_the_server(&exec_result),
        "a cancelled insert must either report an acknowledged server-side abort or fail with \
         the server's gsCode {QUERY_CANCELED_CODE}; got {exec_result:?}"
    );

    // And the insert left no row. This has to outlive the query's own duration: an
    // acknowledged abort means the server *accepted* the request, not that the
    // query has finished dying, so checking immediately could pass merely because
    // the insert had not committed yet.
    wait_past_natural_completion(started, CANCEL_QUERY_SECONDS);
    assert_eq!(
        row_count(&client, &table),
        0,
        "cancelled insert must leave no row: the query was not aborted server-side"
    );
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
    wait_past_natural_completion(started, TIMEOUT_QUERY_SECONDS);
    assert_eq!(
        row_count(&client, &table),
        0,
        "timed-out insert must leave no row: the query was not aborted server-side"
    );

    client.release_statement(&stmt);
}
