use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_fail_authentication_when_user_is_not_provided() {
    //Given Authentication is set to username_password_mfa and user is not provided
    let mut client = SnowflakeTestClient::with_int_test_params();
    set_auth_to_user_password_mfa(&mut client);
    clear_user(&mut client);
    let password = client.parameters.password.clone().unwrap();
    set_password(&mut client, &password);

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned
    client.assert_missing_parameter_error(result);
}

#[test]
fn should_fail_authentication_when_password_is_not_provided() {
    //Given Authentication is set to username_password_mfa and password is not provided
    let mut client = SnowflakeTestClient::with_int_test_params();
    set_auth_to_user_password_mfa(&mut client);

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned
    client.assert_missing_parameter_error(result);
}

#[test]
fn should_fail_authentication_when_passcode_in_password_is_not_set_but_passcode_is_appended_to_password()
 {
    //Given Authentication is set to username_password_mfa and user, password with appended passcode are provided and passcodeInPassword is not set
    let mut client = SnowflakeTestClient::with_int_test_params();
    set_auth_to_user_password_mfa(&mut client);
    let password = client.parameters.password.clone().unwrap();
    let password_with_passcode = format!("{password}123456");
    set_password(&mut client, &password_with_passcode);

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned
    client.assert_login_error(result);
}

#[test]
fn should_fail_authentication_when_passcode_in_password_is_set_but_passcode_is_not_appended_to_password()
 {
    //Given Authentication is set to username_password_mfa and user, password are provided and passcodeInPassword is set but passcode is not appended to password
    let mut client = SnowflakeTestClient::with_int_test_params();
    set_auth_to_user_password_mfa(&mut client);
    let password = client.parameters.password.clone().unwrap();
    set_password(&mut client, &password);
    set_passcode_in_password(&mut client, true);

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned
    client.assert_login_error(result);
}

fn set_auth_to_user_password_mfa(client: &mut SnowflakeTestClient) {
    client.set_connection_option("authenticator", "USERNAME_PASSWORD_MFA");
}

fn set_password(client: &mut SnowflakeTestClient, password: &str) {
    client.set_connection_option("password", password); // pragma: allowlist secret
}

fn clear_user(client: &mut SnowflakeTestClient) {
    client.set_connection_option("user", "");
}

fn set_passcode_in_password(client: &mut SnowflakeTestClient, enabled: bool) {
    client.set_connection_option("passcodeInPassword", &enabled.to_string());
}
