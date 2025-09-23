use super::super::common::arrow_result_helper::ArrowResultHelper;
use super::super::common::test_utils::*;

#[test]
fn should_process_one_million_row_result_set() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id" is executed
    let result = client.execute_query(
        "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id",
    );

    // Then there are 1000000 numbered sequentially rows returned
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    let rows = arrow_helper.transform_into_array::<i64>().unwrap();
    assert_eq!(rows.len(), 1000000);
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row[0], i as i64);
    }
}
