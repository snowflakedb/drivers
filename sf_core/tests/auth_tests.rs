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

struct Pat {
    token_name: String,
    token_secret: String,
}

impl Pat {
    fn acquire() -> Self {
        let name = format!("pat_{:x}", rand::random::<u32>());
        let mut client = SnowflakeTestClient::connect_with_default_auth();
        let user = client.parameters.user.clone().unwrap();
        let role = client.parameters.role.clone().unwrap();
        let result = client.execute_query(&format!("ALTER USER IF EXISTS {user} ADD PROGRAMMATIC ACCESS TOKEN {name} ROLE_RESTRICTION = {role}"));
        let mut arrow_helper = ArrowResultHelper::from_result(result);
        let result = arrow_helper.transform_into_array::<String>().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 2);
        let token_name = result[0][0].clone();
        let token_secret = result[0][1].clone();
        Self {
            token_name,
            token_secret,
        }
    }
}

impl Drop for Pat {
    fn drop(&mut self) {
        let mut client = SnowflakeTestClient::connect_with_default_auth();
        let user = client.parameters.user.clone().unwrap();
        client.execute_query(&format!(
            "ALTER USER IF EXISTS {user} REMOVE PROGRAMMATIC ACCESS TOKEN {}",
            self.token_name
        ));
    }
}

#[test]
fn test_pat_as_password() {
    setup_logging();
    let pat = Pat::acquire();
    let mut client = SnowflakeTestClient::with_default_params();
    client
        .driver
        .connection_set_option_string(
            client.conn_handle.clone(),
            "password".to_string(),
            pat.token_secret.clone(),
        )
        .unwrap();
    client
        .driver
        .connection_init(client.conn_handle.clone(), client.db_handle.clone())
        .unwrap();

    let result = client.execute_query("SELECT 1");
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper.assert_equals_single_value(1);
}

#[test]
fn test_pat_as_token() {
    setup_logging();
    let pat = Pat::acquire();
    let mut client = SnowflakeTestClient::with_default_params();
    client
        .driver
        .connection_set_option_string(
            client.conn_handle.clone(),
            "authenticator".to_string(),
            "PROGRAMMATIC_ACCESS_TOKEN".to_string(),
        )
        .unwrap();
    client
        .driver
        .connection_set_option_string(
            client.conn_handle.clone(),
            "token".to_string(),
            pat.token_secret.clone(),
        )
        .unwrap();
    client
        .driver
        .connection_init(client.conn_handle.clone(), client.db_handle.clone())
        .unwrap();
}
