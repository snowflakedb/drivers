use crate::common::arrow_result_helper::{
    ArrowResultHelper, assert_record_batches_match, assert_schemas_match,
};
use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_return_arrow_even_if_json_result_set_is_returned_for_simple_types() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stmt = client.new_statement();

    // When Table json_result_set_simple_types (str_col STRING, tinyint_col TINYINT, smallint_col SMALLINT, int_col INT, bigint_col BIGINT, number_scale_0_col NUMBER(38, 0), number_scale_2_col NUMBER(38, 2)) is created
    client.set_sql_query(&stmt, "CREATE OR REPLACE TABLE json_result_set_simple_types (str_col STRING, tinyint_col TINYINT, smallint_col SMALLINT, int_col INT, bigint_col BIGINT, number_scale_0_col NUMBER(38, 0), number_scale_2_col NUMBER(38, 2))");
    client.execute_statement_query(&stmt);

    // And Row is inserted with INSERT INTO json_result_set_simple_types VALUES ('abc', 123, 12345, 1234567, 12345678901234567890, 12345678901234567890123456789012345678, 123.45)
    client.set_sql_query(&stmt, "INSERT INTO json_result_set_simple_types VALUES ('abc', 123, 12345, 1234567, 12345678901234567890, 12345678901234567890123456789012345678, 123.45)");
    client.execute_statement_query(&stmt);

    // And Query "SELECT * FROM json_result_set_simple_types" is executed
    client.set_sql_query(&stmt, "SELECT * FROM json_result_set_simple_types");
    let arrow_result = client.execute_statement_query(&stmt);

    // And Query result format is forced to JSON
    client.set_sql_query(
        &stmt,
        "ALTER SESSION SET PYTHON_CONNECTOR_QUERY_RESULT_FORMAT = JSON",
    );
    let result = client.execute_statement_query(&stmt);
    assert_eq!(result.rows_affected(), 1, "Cannot force JSON result set");

    // And Query "SELECT * FROM json_result_set_simple_types" is executed
    client.set_sql_query(&stmt, "SELECT * FROM json_result_set_simple_types");
    let json_result = client.execute_statement_query(&stmt);

    let mut arrow_result_helper = ArrowResultHelper::from_result(arrow_result);
    let mut json_result_helper = ArrowResultHelper::from_result(json_result);

    // Then Schema for both queries should match
    let arrow_schema = arrow_result_helper.schema();
    let json_schema = json_result_helper.schema();
    assert_schemas_match(&arrow_schema, &json_schema);

    let arrow_columns = arrow_result_helper.next_batch().unwrap();
    let json_columns = json_result_helper.next_batch().unwrap();

    // And the result for both queries should match
    assert_record_batches_match(&arrow_columns, &json_columns);

    // And Statement should be released
    client.release_statement(&stmt);
}
