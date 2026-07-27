use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_forward_unrecognized_connection_option_as_session_parameter() {
    // Given Snowflake client is logged in with connection option TIMEZONE set to "Europe/Warsaw"
    let client = SnowflakeTestClient::with_default_jwt_auth_params();
    client.set_connection_option("TIMEZONE", "Europe/Warsaw");
    client.connect().unwrap();

    // When Query "SHOW PARAMETERS LIKE 'TIMEZONE'" is executed
    let result = client.execute_query("SHOW PARAMETERS LIKE 'TIMEZONE'");

    // Then the session parameter value should be "Europe/Warsaw"
    let mut helper = ArrowResultHelper::from_result(result);
    let rows = helper.transform_into_array::<String>().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][1], "Europe/Warsaw");
}
