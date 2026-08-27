use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_authenticate_using_pat_as_password() {
    //Given Authentication is set to password and valid PAT token is provided
    let client = SnowflakeTestClient::with_default_params();
    let pat_secret = client
        .parameters
        .pat
        .clone()
        .expect("SNOWFLAKE_TEST_PAT must be set in parameters.json");
    set_pat_as_password(&client, &pat_secret);

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    client.verify_simple_query(result);
}

#[test]
#[cfg_attr(target_os = "windows", ignore)]
fn flaky_should_authenticate_using_pat_as_token() {
    //Given Authentication is set to Programmatic Access Token and valid PAT token is provided
    let client = SnowflakeTestClient::with_default_params();
    let pat_secret = client
        .parameters
        .pat
        .clone()
        .expect("SNOWFLAKE_TEST_PAT must be set in parameters.json");
    set_auth_to_programmatic_access_token(&client);
    set_pat_token(&client, &pat_secret);

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    client.verify_simple_query(result);
}

#[test]
#[cfg_attr(target_os = "windows", ignore)]
fn flaky_should_authenticate_using_pat_as_token_with_lowercase_authenticator() {
    //Given Authentication is set to lowercase programmatic_access_token and valid PAT token is provided
    let client = SnowflakeTestClient::with_default_params();
    let pat_secret = client
        .parameters
        .pat
        .clone()
        .expect("SNOWFLAKE_TEST_PAT must be set in parameters.json");
    client.set_connection_option("authenticator", "programmatic_access_token");
    set_pat_token(&client, &pat_secret);

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    client.verify_simple_query(result);
}

#[test]
#[cfg_attr(target_os = "windows", ignore)]
fn flaky_should_authenticate_using_pat_token_from_token_file_path() {
    use std::io::Write;

    //Given Authentication is set to Programmatic Access Token and a valid PAT token is stored in a file
    let client = SnowflakeTestClient::with_default_params();
    let pat_secret = client
        .parameters
        .pat
        .clone()
        .expect("SNOWFLAKE_TEST_PAT must be set in parameters.json");
    let mut token_file = tempfile::NamedTempFile::new().expect("temp token file");
    token_file
        .write_all(pat_secret.as_bytes())
        .expect("write PAT token");
    token_file.flush().expect("flush PAT token");
    set_auth_to_programmatic_access_token(&client);
    client.set_connection_option(
        "token_file_path",
        token_file.path().to_str().expect("token file path"),
    );

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
