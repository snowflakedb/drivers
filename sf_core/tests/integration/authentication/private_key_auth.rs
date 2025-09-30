use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_fail_jwt_authentication_when_no_private_file_provided() {
    //Given Authentication is set to JWT
    let mut client = SnowflakeTestClient::with_int_test_params();
    set_auth_to_jwt(&mut client);

    //When Trying to Connect with no private file provided
    let result = client.connect();

    //Then There is error returned
    client.assert_missing_parameter_error(result);
}

fn set_auth_to_jwt(client: &mut SnowflakeTestClient) {
    client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
}
