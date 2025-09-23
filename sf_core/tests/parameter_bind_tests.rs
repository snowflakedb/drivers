pub mod common;
extern crate sf_core;

use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::test_utils::{SnowflakeTestClient, create_param_bindings, setup_logging};
use arrow::datatypes::Int32Type;
use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::{
    StatementBindRequest, StatementExecuteQueryRequest, StatementReleaseRequest,
    StatementSetSqlQueryRequest,
};

#[test]
fn test_statement_bind() {
    setup_logging();
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stmt = client.new_statement();
    DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
        stmt_handle: Some(stmt),
        query: "SELECT ? as value".to_string(),
    })
    .unwrap();
    let (schema, array) = create_param_bindings::<Int32Type>(&[42]);

    DatabaseDriverClient::statement_bind(StatementBindRequest {
        stmt_handle: Some(stmt),
        schema: Some(schema),
        array: Some(array),
    })
    .unwrap();

    let result = DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
        stmt_handle: Some(stmt),
    })
    .unwrap()
    .result
    .unwrap();
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper.assert_equals_single_value(42);
    DatabaseDriverClient::statement_release(StatementReleaseRequest {
        stmt_handle: Some(stmt),
    })
    .unwrap();
}

#[test]
fn test_statement_bind_multiple_params() {
    setup_logging();
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stmt = client.new_statement();
    DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
        stmt_handle: Some(stmt),
        query: "SELECT ?, ? as value".to_string(),
    })
    .unwrap();
    let (schema, array) = create_param_bindings::<Int32Type>(&[42, 1]);
    DatabaseDriverClient::statement_bind(StatementBindRequest {
        stmt_handle: Some(stmt),
        schema: Some(schema),
        array: Some(array),
    })
    .unwrap();
    let result = DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
        stmt_handle: Some(stmt),
    })
    .unwrap()
    .result
    .unwrap();
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper.assert_equals_array(vec![vec![42, 1]]);
    DatabaseDriverClient::statement_release(StatementReleaseRequest {
        stmt_handle: Some(stmt),
    })
    .unwrap();
}
