use super::super::common::test_utils::*;

#[test]
fn should_connect_and_select_with_crl_enabled() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::with_default_params();

    // And CRL is enabled
    client.set_connection_option("crl_mode", "ENABLED");

    // When Query "SELECT 1" is executed
    let result = client.execute_query("SELECT 1");

    // Then the request attempt should be successful
    let rows = crate::common::arrow_result_helper::ArrowResultHelper::from_result(result)
        .transform_into_array::<i64>()
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], 1);
}
