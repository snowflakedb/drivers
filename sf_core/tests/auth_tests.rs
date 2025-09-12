pub mod common;

use common::arrow_result_helper::ArrowResultHelper;
use common::test_utils::*;

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
