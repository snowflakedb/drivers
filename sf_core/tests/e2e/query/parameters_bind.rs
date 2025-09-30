use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use arrow::datatypes::Int32Type;

#[test]
fn should_bind_single_parameter_to_statement() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // And A statement is created
    let stmt = client.new_statement();

    // When Query with single parameter is executed
    client.set_sql_query(&stmt, "SELECT ? as value");
    client.bind_parameters::<Int32Type>(&stmt, &[42]);
    let result = client.execute_statement_query(&stmt);

    // Then Query execution should return the bound parameter value
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper.assert_equals_single_value(42);

    // And Statement should be released
    client.release_statement(&stmt);
}

#[test]
fn should_bind_multiple_parameters_to_statement() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // And A statement is created
    let stmt = client.new_statement();

    // When Query with multiple parameters is executed
    client.set_sql_query(&stmt, "SELECT ?, ? as value");
    client.bind_parameters::<Int32Type>(&stmt, &[42, 1]);
    let result = client.execute_statement_query(&stmt);

    // Then Query execution should return the bound parameter values
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    let expected_array = vec![vec![42, 1]];
    arrow_helper.assert_equals_array(expected_array);

    // And Statement should be released
    client.release_statement(&stmt);
}
