use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_initialize_connection_with_tls_options() {
    // Given Connection parameters are set
    let client = SnowflakeTestClient::with_default_jwt_auth_params();

    // And TLS certificate and hostname verification are enabled
    client.set_connection_option("verify_hostname", "true");
    client.set_connection_option("verify_certificates", "true");

    // When Connection is initialized
    let result = client.connect();

    // Then Login should succeed
    client.verify_simple_query(result);
}
