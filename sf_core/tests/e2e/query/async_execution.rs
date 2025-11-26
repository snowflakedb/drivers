use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_process_async_query_result() {
    // Given Snowflake client is logged in with async engine enabled
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stmt = client.new_statement();
    client.set_statement_async_execution(&stmt, true);

    // When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY id" is executed
    client.set_sql_query(
        &stmt,
        "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY id",
    );
    let result = client.execute_statement_query(&stmt);

    // Then there are 10000 numbered sequentially rows returned
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    let rows = arrow_helper.transform_into_array::<i64>().unwrap();
    assert_eq!(rows.len(), 10000);
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row[0], i as i64);
    }

    // And Statement should be released
    client.release_statement(&stmt);
}

#[test]
fn should_match_blocking_results_when_async_execution_enabled() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let blocking_stmt = client.new_statement();
    let async_stmt = client.new_statement();

    // And Statement A has async execution disabled
    client.set_statement_async_execution(&blocking_stmt, false);

    // And Statement B has async execution enabled
    client.set_statement_async_execution(&async_stmt, true);

    // When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000)) v ORDER BY id" is executed on both statements
    let sql = "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000)) v ORDER BY id";
    client.set_sql_query(&blocking_stmt, sql);
    client.set_sql_query(&async_stmt, sql);
    let blocking_result = client.execute_statement_query(&blocking_stmt);
    let async_result = client.execute_statement_query(&async_stmt);

    // Then both result sets have identical sequential ids
    let mut blocking_helper = ArrowResultHelper::from_result(blocking_result);
    let blocking_rows = blocking_helper.transform_into_array::<i64>().unwrap();
    let mut async_helper = ArrowResultHelper::from_result(async_result);
    let async_rows = async_helper.transform_into_array::<i64>().unwrap();
    assert_eq!(
        blocking_rows, async_rows,
        "async and blocking results must match"
    );
    assert_eq!(blocking_rows.len(), 1000);
    for (i, row) in blocking_rows.iter().enumerate() {
        assert_eq!(row[0], i as i64);
    }

    // And Both statements should be released
    client.release_statement(&blocking_stmt);
    client.release_statement(&async_stmt);
}

#[test]
fn should_use_async_by_default_when_no_execution_mode_specified() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // And Statement has no async execution setting
    let stmt = client.new_statement(); // no set_statement_async_execution call

    // When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 100)) v ORDER BY id" is executed
    client.set_sql_query(
        &stmt,
        "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 100)) v ORDER BY id",
    );
    let result = client.execute_statement_query(&stmt);

    // Then there are 100 numbered sequentially rows returned
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    let rows = arrow_helper.transform_into_array::<i64>().unwrap();
    assert_eq!(rows.len(), 100);
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row[0], i as i64);
    }

    // And Statement should be released
    client.release_statement(&stmt);
}
