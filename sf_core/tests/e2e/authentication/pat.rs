use super::super::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_authenticate_using_pat_as_password() {
    //Given Authentication is set to password and valid PAT token is provided
    let pat = Pat::acquire();
    let client = SnowflakeTestClient::with_default_params();
    set_pat_as_password(&client, &pat.token_secret);

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    client.verify_simple_query(result);
}

#[test]
fn should_authenticate_using_pat_as_token() {
    //Given Authentication is set to Programmatic Access Token and valid PAT token is provided
    let pat = Pat::acquire();
    let client = SnowflakeTestClient::with_default_params();
    set_auth_to_programmatic_access_token(&client);
    set_pat_token(&client, &pat.token_secret);

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    client.verify_simple_query(result);
}

#[test]
fn should_fail_pat_authentication_when_invalid_token_provided() {
    //Given Authentication is set to Programmatic Access Token and invalid PAT token is provided
    let client = SnowflakeTestClient::with_default_params();
    set_auth_to_programmatic_access_token(&client);
    set_invalid_pat_token(&client);

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned
    client.assert_login_error(result);
}

struct Pat {
    token_name: String,
    token_secret: String,
}

impl Pat {
    fn acquire() -> Self {
        let name = format!("pat_{:x}", rand::random::<u32>());
        let client = SnowflakeTestClient::connect_with_default_auth();
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
        let client = SnowflakeTestClient::connect_with_default_auth();
        let user = client.parameters.user.clone().unwrap();
        client.execute_query(&format!(
            "ALTER USER IF EXISTS {user} REMOVE PROGRAMMATIC ACCESS TOKEN {}",
            self.token_name
        ));
    }
}

fn set_auth_to_programmatic_access_token(client: &SnowflakeTestClient) {
    client.set_connection_option("authenticator", "PROGRAMMATIC_ACCESS_TOKEN");
}

fn set_pat_as_password(client: &SnowflakeTestClient, token_secret: &str) {
    client.set_connection_option("password", token_secret);
}

fn set_pat_token(client: &SnowflakeTestClient, token_secret: &str) {
    client.set_connection_option("token", token_secret);
}

fn set_invalid_pat_token(client: &SnowflakeTestClient) {
    client.set_connection_option("token", "invalid_token_12345");
}
