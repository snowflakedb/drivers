use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_forward_unrecognized_connection_option_as_session_parameter() {
    // Given Snowflake client is logged in with connection option QUERY_TAG set to "session_param_e2e_test"
    let client = SnowflakeTestClient::with_default_jwt_auth_params();
    client.set_connection_option("QUERY_TAG", "session_param_e2e_test");
    client.connect().unwrap();

    // When Query "SELECT CURRENT_QUERY_TAG()" is executed
    let result = client.execute_query("SELECT CURRENT_QUERY_TAG()");

    // Then the result should contain value "session_param_e2e_test"
    let mut helper = ArrowResultHelper::from_result(result);
    let rows = helper.transform_into_array::<String>().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "session_param_e2e_test");
}
