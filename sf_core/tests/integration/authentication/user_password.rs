use crate::common::mocks::password;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;

struct PasswordTestFixture {
    mock: MockServerWithTls,
    client: SnowflakeTestClient,
}

impl PasswordTestFixture {
    fn new() -> Self {
        let mock = MockServerWithTls::start();
        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("password", "test_password"); // pragma: allowlist secret
        Self { mock, client }
    }
}

#[test]
fn should_authenticate_with_password_via_wiremock() {
    //Given Wiremock is running and has password login success mapping
    let fixture = PasswordTestFixture::new();
    //And Snowflake client is configured for password authentication
    fixture.mock.mount(password::login_success());

    //When Trying to Connect
    let result = fixture.client.connect();

    //Then Login is successful
    assert!(
        result.is_ok(),
        "Expected password login to succeed, got: {result:?}"
    );
}

#[test]
fn should_fail_authentication_when_wrong_credentials_are_provided() {
    //Given Wiremock is running and has password login failure mapping for wrong credentials
    let fixture = PasswordTestFixture::new();
    //And Snowflake client is configured for password authentication with wrong password
    fixture
        .client
        .set_connection_option("password", "wrong_password"); // pragma: allowlist secret
    fixture
        .mock
        .mount(password::login_failure_wrong_credentials());

    //When Trying to Connect
    let result = fixture.client.connect();

    //Then There is error returned
    let error = result.expect_err("Expected login to fail with wrong password");
    assert!(
        error.contains("Incorrect username or password"),
        "Expected credential error, got: {error}"
    );
}

#[test]
fn should_fail_authentication_when_user_is_not_provided() {
    //Given Wiremock is running and has password login success mapping
    let mock = MockServerWithTls::start();
    let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
    //And Snowflake client is configured for password authentication without user
    client.set_connection_option("user", "");
    client.set_connection_option("password", "test_password"); // pragma: allowlist secret
    mock.mount(password::login_success());

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned with missing parameter
    client.assert_missing_parameter_error(result);
}

#[test]
fn should_fail_authentication_when_password_is_not_provided() {
    //Given Wiremock is running and has password login success mapping
    let mock = MockServerWithTls::start();
    let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
    //And Snowflake client is configured for password authentication without password
    mock.mount(password::login_success());

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned with missing parameter
    client.assert_missing_parameter_error(result);
}

#[test]
fn should_fail_authentication_when_password_is_empty() {
    //Given Wiremock is running and has password login success mapping
    let mock = MockServerWithTls::start();
    let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
    //And Snowflake client is configured for password authentication with empty password
    client.set_connection_option("password", ""); // pragma: allowlist secret
    mock.mount(password::login_success());

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned with missing parameter
    client.assert_missing_parameter_error(result);
}
