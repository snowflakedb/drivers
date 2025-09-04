use super::super::common::test_utils::*;

#[test]
fn should_authenticate_using_private_file_with_password() {
    //Given Authentication is set to JWT
    let mut client = SnowflakeTestClient::with_default_params();
    set_auth_to_jwt(&mut client);

    //And Private file with password is provided
    let _private_key_file = set_private_key_file(&mut client);
    set_private_key_password(&mut client);

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    result.unwrap();
    client.verify_simple_query();
}

#[test]
fn should_fail_jwt_authentication_when_no_private_file_provided() {
    //Given Authentication is set to JWT
    let mut client = SnowflakeTestClient::with_default_params();
    set_auth_to_jwt(&mut client);

    //When Trying to Connect with no private file provided
    let result = client.connect();

    //Then There is error returned
    assert!(result.is_err());
}

fn set_auth_to_jwt(client: &mut SnowflakeTestClient) {
    client
        .driver
        .connection_set_option_string(
            client.conn_handle.clone(),
            "authenticator".to_string(),
            "SNOWFLAKE_JWT".to_string(),
        )
        .unwrap();
}

fn set_private_key_file(client: &mut SnowflakeTestClient) -> PrivateKeyFile {
    let private_key_file = get_private_key_file(&client.parameters);
    client
        .driver
        .connection_set_option_string(
            client.conn_handle.clone(),
            "private_key_file".to_string(),
            private_key_file.path.clone(),
        )
        .unwrap();
    private_key_file
}

fn set_private_key_password(client: &mut SnowflakeTestClient) {
    client
        .driver
        .connection_set_option_string(
            client.conn_handle.clone(),
            "private_key_password".to_string(),
            client.parameters.private_key_password.clone().unwrap(),
        )
        .unwrap();
}

struct PrivateKeyFile {
    path: String,
}

impl Drop for PrivateKeyFile {
    fn drop(&mut self) {
        std::fs::remove_file(self.path.clone()).unwrap();
    }
}

fn get_private_key_file(parameters: &Parameters) -> PrivateKeyFile {
    let private_key_contents = parameters.private_key_contents.clone().unwrap();
    let private_key_contents = private_key_contents.join("\n");
    let suffix = format!("{:x}", rand::random::<u32>());
    let private_key_path = format!("rsa_key_{suffix}.p8");
    std::fs::write(&private_key_path, private_key_contents).unwrap();
    PrivateKeyFile {
        path: private_key_path,
    }
}
