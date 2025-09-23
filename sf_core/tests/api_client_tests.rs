pub mod common;
use common::arrow_result_helper::ArrowResultHelper;
use common::test_utils::*;

use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::*;

#[test]
fn test_snowflake_select_1() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stmt = client.new_statement();
    DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
        stmt_handle: Some(stmt),
        query: "select 1".to_string(),
    })
    .unwrap();
    let result = DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
        stmt_handle: Some(stmt),
    })
    .unwrap()
    .result
    .unwrap();

    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper.assert_equals_single_value(String::from("1"));
}

#[test]
fn test_create_temporary_stage() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE";
    let result = client.execute_query(&format!("create temporary stage {stage_name}"));

    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper
        .assert_equals_single_value(format!("Stage area {stage_name} successfully created."));
}
