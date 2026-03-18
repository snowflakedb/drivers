use crate::common::snowflake_test_client::SnowflakeTestClient;

// MFA E2E tests require a real Snowflake account with DUO MFA enrolled.
// Cannot run in CI (needs interactive MFA push or real TOTP); run locally with:
//   PARAMETER_PATH=parameters.json cargo test --package sf_core --features auth_mfa_e2e -- --ignored user_password_mfa

#[test]
#[ignore = "E2E will be enabled within: SNOW-3252862"]
fn should_authenticate_using_username_password_and_duo_push() {
    //Given Authentication is set to username_password_mfa and user, password are provided and DUO push is enabled
    let client = SnowflakeTestClient::with_default_params();
    set_auth_to_user_password_mfa(&client);
    set_password(&client, &client.parameters.password.clone().unwrap());

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    client.verify_simple_query(result);
}

#[test]
#[ignore = "E2E will be enabled within: SNOW-3252862"]
fn should_authenticate_using_username_password_and_totp_passcode() {
    //Given Authentication is set to username_password_mfa and user, password and passcode are provided
    let client = SnowflakeTestClient::with_default_params();
    set_auth_to_user_password_mfa(&client);
    set_password(&client, &client.parameters.password.clone().unwrap());
    set_passcode(&client, "123456"); // TODO: use real TOTP passcode

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    client.verify_simple_query(result);
}

#[test]
#[ignore = "E2E will be enabled within: SNOW-3252862"]
fn should_authenticate_using_username_password_with_appended_totp_passcode() {
    //Given Authentication is set to username_password_mfa and user, password with appended passcode are provided and passcodeInPassword is set
    let client = SnowflakeTestClient::with_default_params();
    set_auth_to_user_password_mfa(&client);
    let password = client.parameters.password.clone().unwrap();
    let password_with_passcode = format!("{password}123456"); // TODO: use real TOTP passcode
    set_password(&client, &password_with_passcode);
    set_passcode_in_password(&client, true);

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    client.verify_simple_query(result);
}

#[test]
#[ignore = "E2E will be enabled within: SNOW-3252862"]
fn should_fail_authentication_when_wrong_password_is_provided() {
    //Given Authentication is set to username_password_mfa and user is provided but password is skipped or invalid
    let client = SnowflakeTestClient::with_default_params();
    set_auth_to_user_password_mfa(&client);
    set_invalid_password(&client);

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned
    client.assert_login_error(result);
}

fn set_auth_to_user_password_mfa(client: &SnowflakeTestClient) {
    client.set_connection_option("authenticator", "USERNAME_PASSWORD_MFA");
}

fn set_password(client: &SnowflakeTestClient, password: &str) {
    client.set_connection_option("password", password); // pragma: allowlist secret
}

fn set_passcode(client: &SnowflakeTestClient, passcode: &str) {
    client.set_connection_option("passcode", passcode);
}

fn set_passcode_in_password(client: &SnowflakeTestClient, enabled: bool) {
    client.set_connection_option("passcodeInPassword", &enabled.to_string());
}

fn set_invalid_password(client: &SnowflakeTestClient) {
    client.set_connection_option("password", "invalid_password_12345"); // pragma: allowlist secret
}
