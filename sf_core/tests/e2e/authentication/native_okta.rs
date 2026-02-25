use crate::common::snowflake_test_client::SnowflakeTestClient;

// Native Okta E2E tests require VPN access to preprod Snowflake account.
// Skipped in GH Actions (no VPN), run on Jenkins or locally with:
//   PARAMETER_PATH=parameters.json cargo test -- --ignored vpn_

#[test]
#[ignore = "Requires VPN to access preprod account (run on Jenkins)"]
fn vpn_should_authenticate_using_native_okta() {
    // Given Okta authentication is configured with valid credentials
    let client = SnowflakeTestClient::with_default_params();
    let okta_url = client
        .parameters
        .okta_url
        .clone()
        .expect("SNOWFLAKE_TEST_OKTA_URL must be set for Okta E2E tests");
    let okta_user = client
        .parameters
        .okta_user
        .clone()
        .expect("SNOWFLAKE_TEST_OKTA_USER must be set for Okta E2E tests");
    let okta_password = client
        .parameters
        .okta_password
        .clone()
        .expect("SNOWFLAKE_TEST_OKTA_PASSWORD must be set for Okta E2E tests");
    let okta_account = client
        .parameters
        .okta_account
        .clone()
        .expect("SNOWFLAKE_TEST_OKTA_ACCOUNT must be set for Okta E2E tests");
    let okta_host = client
        .parameters
        .okta_host
        .clone()
        .expect("SNOWFLAKE_TEST_OKTA_HOST must be set for Okta E2E tests");

    // Use the Okta-enabled Snowflake account with its specific settings
    client.set_connection_option("account", &okta_account);
    client.set_connection_option("host", &okta_host);
    client.set_connection_option("authenticator", &okta_url);
    client.set_connection_option("user", &okta_user);
    client.set_connection_option("password", &okta_password);
    // Use PUBLIC role to avoid role mismatch errors
    client.set_connection_option("role", "PUBLIC");

    // When Trying to Connect
    let result = client.connect();

    // Then Login is successful and simple query can be executed
    client.verify_simple_query(result);
}

#[test]
#[ignore = "Requires VPN to access preprod account (run on Jenkins)"]
fn vpn_should_fail_native_okta_authentication_with_wrong_credentials() {
    // Given Okta authentication is configured with wrong password
    let client = SnowflakeTestClient::with_default_params();
    let okta_url = client
        .parameters
        .okta_url
        .clone()
        .expect("SNOWFLAKE_TEST_OKTA_URL must be set for Okta E2E tests");
    let okta_user = client
        .parameters
        .okta_user
        .clone()
        .expect("SNOWFLAKE_TEST_OKTA_USER must be set for Okta E2E tests");
    let okta_account = client
        .parameters
        .okta_account
        .clone()
        .expect("SNOWFLAKE_TEST_OKTA_ACCOUNT must be set for Okta E2E tests");
    let okta_host = client
        .parameters
        .okta_host
        .clone()
        .expect("SNOWFLAKE_TEST_OKTA_HOST must be set for Okta E2E tests");

    // Use the Okta-enabled Snowflake account with its specific settings
    client.set_connection_option("account", &okta_account);
    client.set_connection_option("host", &okta_host);
    client.set_connection_option("authenticator", &okta_url);
    client.set_connection_option("user", &okta_user);
    client.set_connection_option("password", "wrong_password_12345");
    client.set_connection_option("role", "PUBLIC");

    // When Trying to Connect
    let result = client.connect();

    // Then Connection fails with authentication error
    client.assert_login_error(result);
}

#[test]
#[ignore = "Requires VPN to access preprod account (run on Jenkins)"]
fn vpn_should_fail_native_okta_authentication_with_wrong_okta_url() {
    // Given Okta authentication is configured with invalid okta url
    let client = SnowflakeTestClient::with_default_params();
    let okta_user = client
        .parameters
        .okta_user
        .clone()
        .expect("SNOWFLAKE_TEST_OKTA_USER must be set for Okta E2E tests");
    let okta_password = client
        .parameters
        .okta_password
        .clone()
        .expect("SNOWFLAKE_TEST_OKTA_PASSWORD must be set for Okta E2E tests");
    let okta_account = client
        .parameters
        .okta_account
        .clone()
        .expect("SNOWFLAKE_TEST_OKTA_ACCOUNT must be set for Okta E2E tests");
    let okta_host = client
        .parameters
        .okta_host
        .clone()
        .expect("SNOWFLAKE_TEST_OKTA_HOST must be set for Okta E2E tests");

    // Use the Okta-enabled Snowflake account with invalid Okta URL
    client.set_connection_option("account", &okta_account);
    client.set_connection_option("host", &okta_host);
    client.set_connection_option("authenticator", "https://invalid.okta.com");
    client.set_connection_option("user", &okta_user);
    client.set_connection_option("password", &okta_password);
    client.set_connection_option("role", "PUBLIC");

    // When Trying to Connect
    let result = client.connect();

    // Then Connection fails with authentication error
    client.assert_login_error(result);
}
