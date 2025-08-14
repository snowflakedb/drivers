pub mod common;

use common::test_utils::*;

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

#[test]
fn test_private_key_auth() {
    setup_logging();
    let mut client = SnowflakeTestClient::with_default_params();
    let private_key_file = get_private_key_file(&client.parameters);

    client
        .driver
        .connection_set_option_string(
            client.conn_handle.clone(),
            "private_key_file".to_string(),
            private_key_file.path.clone(),
        )
        .unwrap();

    client
        .driver
        .connection_set_option_string(
            client.conn_handle.clone(),
            "private_key_password".to_string(),
            client.parameters.private_key_password.clone().unwrap(),
        )
        .unwrap();

    client
        .driver
        .connection_set_option_string(
            client.conn_handle.clone(),
            "authenticator".to_string(),
            "SNOWFLAKE_JWT".to_string(),
        )
        .unwrap();

    client
        .driver
        .connection_init(client.conn_handle.clone(), client.db_handle.clone())
        .unwrap();

    let result = client.execute_query("SELECT 1");

    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper.assert_equals_single_value(String::from("1"));
}

#[test]
fn test_private_key_auth_no_private_key_file() {
    setup_logging();
    let mut client = SnowflakeTestClient::with_default_params();

    client
        .driver
        .connection_set_option_string(
            client.conn_handle.clone(),
            "authenticator".to_string(),
            "SNOWFLAKE_JWT".to_string(),
        )
        .unwrap();

    let result = client
        .driver
        .connection_init(client.conn_handle.clone(), client.db_handle.clone());
    assert!(result.is_err());
}
