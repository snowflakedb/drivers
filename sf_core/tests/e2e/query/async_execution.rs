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
