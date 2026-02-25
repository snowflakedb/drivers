use crate::common::arrow_convert_row::ArrowConvertRow;
use crate::common::arrow_deserialize::RecordBatch;
use crate::common::arrow_extract_value::{ArrowExtractError, extract_arrow_value};
use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_return_arrow_even_if_json_result_set_is_returned_for_simple_types() {
    #[derive(Debug, PartialEq)]
    struct StringAndInt {
        str_col: String,
        int_col: i32,
    }

    impl ArrowConvertRow for StringAndInt {
        fn from_arrow_row(batch: &RecordBatch, row_idx: usize) -> Result<Self, ArrowExtractError> {
            Ok(StringAndInt {
                str_col: extract_arrow_value::<String>(batch.column(0).as_ref(), row_idx)?,
                int_col: extract_arrow_value::<i32>(batch.column(1).as_ref(), row_idx)?,
            })
        }
    }

    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stmt = client.new_statement();

    // When Query "SELECT 'abc', 123" is executed
    client.set_sql_query(&stmt, "SELECT 'abc', 123");
    let arrow_result = client.execute_statement_query(&stmt);

    // And Query result format is forced to JSON
    client.set_sql_query(
        &stmt,
        "ALTER SESSION SET PYTHON_CONNECTOR_QUERY_RESULT_FORMAT = JSON",
    );
    let result = client.execute_statement_query(&stmt);
    assert_eq!(result.rows_affected(), 1, "Cannot force JSON result set");

    // And Query "SELECT 'abc', 123" is executed
    client.set_sql_query(&stmt, "SELECT 'abc', 123");
    let json_result = client.execute_statement_query(&stmt);

    let mut arrow_result_helper = ArrowResultHelper::from_result(arrow_result);
    let mut json_result_helper = ArrowResultHelper::from_result(json_result);

    // Then Schema for both queries should match
    let arrow_schema = arrow_result_helper.schema();
    let json_schema = json_result_helper.schema();
    assert_eq!(arrow_schema, json_schema, "Schemas do not match");

    let arrow_records = arrow_result_helper
        .transform_rows::<StringAndInt>()
        .unwrap();
    let json_records = json_result_helper.transform_rows::<StringAndInt>().unwrap();

    // And the result for both queries should match
    assert_eq!(arrow_records, json_records, "Records do not match");

    // And Statement should be released
    client.release_statement(&stmt);
}
