use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_connect_and_select_with_crl_enabled() {
    // Given Snowflake client is configured before connect
    let client = SnowflakeTestClient::with_default_jwt_auth_params();

    // And CRL is enabled before the connection is established
    client.set_connection_option("crl_mode", "ENABLED");

    // Given Snowflake client is logged in
    let connection_result = client.connect();
    connection_result.expect("Login failed");

    // When Query "SELECT 1" is executed
    let result = client.execute_query("SELECT 1");

    // Then the request attempt should be successful
    let rows = crate::common::arrow_result_helper::ArrowResultHelper::from_result(result)
        .transform_into_array::<i64>()
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], 1);
}
