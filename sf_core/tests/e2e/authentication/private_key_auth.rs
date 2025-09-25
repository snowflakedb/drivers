use crate::common::test_utils::*;
use std::fs;

#[test]
fn should_authenticate_using_private_file_with_password() {
    //Given Authentication is set to JWT and private file with password is provided
    let mut client = SnowflakeTestClient::with_default_params();
    set_auth_to_jwt(&mut client);
    let _private_key_file = set_private_key_file(&mut client);
    set_private_key_password(&mut client);

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    client.verify_simple_query(result);
}

#[test]
fn should_fail_jwt_authentication_when_invalid_private_key_provided() {
    //Given Authentication is set to JWT and invalid private key file is provided
    let mut client = SnowflakeTestClient::with_default_params();
    set_auth_to_jwt(&mut client);
    let _invalid_private_key_file = set_invalid_private_key_file(&mut client);

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned
    client.assert_login_error(result);
}

fn set_auth_to_jwt(client: &mut SnowflakeTestClient) {
    client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
}

fn set_private_key_file(client: &mut SnowflakeTestClient) -> PrivateKeyFile {
    let private_key_file = get_private_key_file(&client.parameters);
    client.set_connection_option("private_key_file", &private_key_file.path);
    private_key_file
}

fn set_invalid_private_key_file(client: &mut SnowflakeTestClient) -> PrivateKeyFile {
    let invalid_private_key_file = get_invalid_private_key_file();
    client.set_connection_option("private_key_file", &invalid_private_key_file.path);
    invalid_private_key_file
}

fn set_private_key_password(client: &mut SnowflakeTestClient) {
    client.set_connection_option(
        "private_key_password",
        &client.parameters.private_key_password.clone().unwrap(),
    );
}

struct PrivateKeyFile {
    path: String,
}

impl Drop for PrivateKeyFile {
    fn drop(&mut self) {
        std::fs::remove_file(self.path.clone()).unwrap();
    }
}

fn create_private_key_file(key_lines: Vec<String>, file_prefix: &str) -> PrivateKeyFile {
    let key_content = key_lines.join("\n") + "\n";
    let suffix = format!("{:x}", rand::random::<u32>());
    let key_path = format!("{file_prefix}_{suffix}.p8");

    std::fs::write(&key_path, key_content).expect("Failed to write private key file");

    PrivateKeyFile { path: key_path }
}

fn create_private_key_file_from_option(
    key_lines: Option<Vec<String>>,
    file_prefix: &str,
) -> PrivateKeyFile {
    let key_contents = key_lines.unwrap();
    create_private_key_file(key_contents, file_prefix)
}

fn get_private_key_file(parameters: &Parameters) -> PrivateKeyFile {
    create_private_key_file_from_option(parameters.private_key_contents.clone(), "rsa_key")
}

fn get_invalid_private_key_file() -> PrivateKeyFile {
    let invalid_key_path = repo_root()
        .join("tests")
        .join("test_data")
        .join("invalid_rsa_key.p8");

    let key_content = fs::read_to_string(&invalid_key_path).unwrap_or_else(|_| {
        panic!(
            "Failed to read invalid private key file: {}",
            invalid_key_path.display()
        )
    });

    let invalid_key_lines: Vec<String> = key_content.lines().map(|s| s.to_string()).collect();
    create_private_key_file(invalid_key_lines, "invalid_rsa_key")
}
