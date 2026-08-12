use crate::common::mocks::auth;
use crate::common::private_key_helper;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;

#[test]
fn should_fail_jwt_authentication_when_no_private_file_provided() {
    //Given Authentication is set to JWT
    let client = SnowflakeTestClient::with_int_tests_params(None);
    client.set_connection_option("authenticator", "SNOWFLAKE_JWT");

    //When Trying to Connect with no private file provided
    let result = client.connect();

    //Then There is error returned
    client.assert_missing_parameter_error(result);
}

#[test]
fn should_surface_jwt_credential_rejection_code() {
    //Given Authentication is set to JWT and the backend is configured to reject the JWT as invalid
    let mock = MockServerWithTls::start();
    let mut client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
    client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
    let temp_key_file =
        private_key_helper::get_test_private_key_file().expect("Failed to create private key file");
    client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());
    client.set_temp_key_file(temp_key_file);
    mock.mount(auth::login_failure_jwt_token_invalid());

    //When Trying to Connect
    let result = client.connect();

    //Then the raw GS code surfaces in the error
    let error = result.expect_err("Expected JWT authentication to fail");
    assert!(
        error.contains("390144"),
        "Expected the raw GS code 390144 (JWT_TOKEN_INVALID) in the error, not a \
         generic/hardcoded login-failure code — got: {error}"
    );
}
