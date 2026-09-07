use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::{SnowflakeTestClient, unwrap_single_rs_handle};
use sf_core::protobuf::generated::database_driver_v1::StatementHandle;

const CONCURRENCY: usize = 8;

fn take<T>(handle: std::thread::ScopedJoinHandle<'_, T>) -> T {
    handle
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

fn join_n<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    std::thread::scope(|scope| {
        let f = &f;
        let handles: Vec<_> = (0..n).map(|i| scope.spawn(move || f(i))).collect();
        handles.into_iter().map(take).collect()
    })
}

fn execute_marker(client: &SnowflakeTestClient, stmt: &StatementHandle, marker: i64) -> i64 {
    client.set_sql_query(stmt, &format!("SELECT {marker} AS marker"));
    let result = client.execute_statement_query(stmt);
    let rs_handle = unwrap_single_rs_handle(&result);
    let stream = client.result_set_get_stream(&rs_handle);
    let rows = ArrowResultHelper::from_result(stream)
        .transform_into_array::<i64>()
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].len(), 1);
    rows[0][0]
}

fn assert_identity(markers: &[i64]) {
    let expected: Vec<i64> = (0..CONCURRENCY).map(|i| i as i64).collect();
    assert_eq!(markers, expected.as_slice());
}

#[test]
fn should_return_independent_correct_results_for_overlapping_queries_on_one_connection() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When 8 queries with distinct markers are executed concurrently on the same connection
    let stmts: Vec<StatementHandle> = (0..CONCURRENCY).map(|_| client.new_statement()).collect();
    let markers = join_n(CONCURRENCY, |i| {
        execute_marker(&client, &stmts[i], i as i64)
    });

    // Then each query returns its own marker
    assert_identity(&markers);
}

#[test]
fn should_return_independent_correct_results_for_overlapping_queries_on_distinct_connections() {
    // Given 8 Snowflake clients are logged in
    let clients = join_n(CONCURRENCY, |_| {
        SnowflakeTestClient::connect_with_default_auth()
    });

    // When 8 queries with distinct markers are executed concurrently on distinct connections
    let markers = join_n(CONCURRENCY, |i| {
        let stmt = clients[i].new_statement();
        execute_marker(&clients[i], &stmt, i as i64)
    });

    // Then each query returns its own marker
    assert_identity(&markers);
}
