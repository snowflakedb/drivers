use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_initialize_connection_with_tls_options() {
    // Given TLS certificate and hostname verification are enabled
    let client = SnowflakeTestClient::with_default_params();
    client.set_connection_option("verify_hostname", "true");
    client.set_connection_option("verify_certificates", "true");
    let password = client.parameters.password.clone().unwrap();

    // And connection parameters (account, user, password, host) are set
    client.set_connection_option("password", &password);

    // When Connection is initialized
    let result = client.connect();

    // Then Login should succeed
    client.verify_simple_query(result);
}
