use crate::common::private_key_helper;
use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_authenticate_using_private_file_with_password() {
    //Given Authentication is set to JWT and private file with password is provided
    let client = SnowflakeTestClient::with_default_jwt_auth_params();

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    client.verify_simple_query(result);
}

#[test]
fn should_fail_jwt_authentication_when_invalid_private_key_provided() {
    //Given Authentication is set to JWT and invalid private key file is provided
    let mut client = SnowflakeTestClient::with_default_params();
    client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
    set_invalid_private_key_file(&mut client);

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned
    client.assert_login_error(result);
}

fn set_invalid_private_key_file(client: &mut SnowflakeTestClient) {
    let temp_key_file = private_key_helper::get_test_private_key_file()
        .expect("Failed to create test private key file");
    client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());
    client.set_temp_key_file(temp_key_file);
}
