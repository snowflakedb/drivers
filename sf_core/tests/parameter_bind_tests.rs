pub mod common;
extern crate sf_core;

use arrow::datatypes::Int32Type;

use crate::common::test_utils::{
    ArrowResultHelper, SnowflakeTestClient, create_param_bindings, setup_logging,
};

#[test]
fn test_statement_bind() {
    setup_logging();
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stmt = client.new_statement();
    client
        .driver
        .statement_set_sql_query(stmt.clone(), "SELECT ? as value".to_string())
        .unwrap();
    let (schema, array) = create_param_bindings::<Int32Type>(&[42]);

    client
        .driver
        .statement_bind(stmt.clone(), schema, array)
        .unwrap();

    let result = client.driver.statement_execute_query(stmt.clone()).unwrap();
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper.assert_equals_single_value(42);
    client.driver.statement_release(stmt).unwrap();
}

#[test]
fn test_statement_bind_multiple_params() {
    setup_logging();
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stmt = client.new_statement();
    client
        .driver
        .statement_set_sql_query(stmt.clone(), "SELECT ?, ? as value".to_string())
        .unwrap();
    let (schema, array) = create_param_bindings::<Int32Type>(&[42, 1]);
    client
        .driver
        .statement_bind(stmt.clone(), schema, array)
        .unwrap();
    let result = client.driver.statement_execute_query(stmt.clone()).unwrap();
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper.assert_equals_array(vec![vec![42, 1]]);
    client.driver.statement_release(stmt).unwrap();
}
