use crate::common::arrow_deserialize::ArrowDeserialize;
use crate::common::arrow_extract_value::extract_arrow_value;
use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::SnowflakeTestClient;

#[derive(Debug)]
struct AllTypes {
    s: String,
    i: i64,
}

impl ArrowDeserialize for AllTypes {
    fn deserialize_one(
        batch: &crate::common::arrow_deserialize::RecordBatch,
        row_index: usize,
    ) -> Result<Self, String> {
        Ok(AllTypes {
            s: extract_arrow_value::<String>(batch.column(0), row_index)
                .map_err(|e| e.to_string())?,
            i: extract_arrow_value::<i64>(batch.column(1), row_index).map_err(|e| e.to_string())?,
        })
    }
}

impl PartialEq for AllTypes {
    fn eq(&self, other: &Self) -> bool {
        assert_eq!(self.s, other.s);
        assert_eq!(self.i, other.i);
        true
    }
}

#[test]
fn should_return_arrow_even_if_json_result_set_is_returned_for_simple_types() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // And Query result format is forced to JSON
    let stmt = client.new_statement();
    client.set_sql_query(
        &stmt,
        "ALTER SESSION SET PYTHON_CONNECTOR_QUERY_RESULT_FORMAT = JSON",
    );
    let result = client.execute_statement_query(&stmt);
    assert_eq!(result.rows_affected(), 1, "Cannot force JSON result set");

    // When Query "SELECT 'abc', 123" is executed
    client.set_sql_query(&stmt, "SELECT 'abc', 123");
    let result = client.execute_statement_query(&stmt);

    // Then all values are deserialized correctly
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    let all_types: AllTypes = arrow_helper.fetch_one().unwrap();
    assert_eq!(
        all_types,
        AllTypes {
            s: "abc".to_string(),
            i: 123
        }
    );
}
