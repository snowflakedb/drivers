pub mod common;

use common::test_utils::*;

#[test]
fn test_large_result() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let sql = "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id";
    let result = client.execute_query(sql);
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    let rows = arrow_helper.transform_into_array::<i64>().unwrap();
    assert_eq!(rows.len(), 1000000);
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row[0], i as i64);
    }
}
