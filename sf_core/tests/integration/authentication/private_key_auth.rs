use crate::common::snowflake_test_client::SnowflakeTestClient;

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
