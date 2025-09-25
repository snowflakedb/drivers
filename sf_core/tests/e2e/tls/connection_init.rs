use super::super::common::test_utils::*;

#[test]
fn connection_init_with_tls_options_succeeds() {
    // Given TLS certificate and hostname verification are enabled
    let client = SnowflakeTestClient::with_default_params();
    client.set_connection_option("verify_hostname", "true");
    client.set_connection_option("verify_certificates", "true");
    client.set_connection_option("custom_root_store_path", "true");
    let password = client.parameters.password.clone().unwrap();

    // And connection parameters (account, user, password, host) are set
    client.set_connection_option("password", &password);

    // When I initialize the connection
    let result = client.connect();

    // Then the login succeeds
    client.verify_simple_query(result);
}
