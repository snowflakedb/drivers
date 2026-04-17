use crate::common::mocks::password;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;

struct PlatformDetectionFixture {
    mock: MockServerWithTls,
    client: SnowflakeTestClient,
}

impl PlatformDetectionFixture {
    fn new() -> Self {
        let mock = MockServerWithTls::start();
        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("password", "test_password"); // pragma: allowlist secret
        mock.mount(password::login_success());
        Self { mock, client }
    }
}

fn login_request_body(mock: &MockServerWithTls) -> serde_json::Value {
    let reqs = mock.received_requests();
    let login = reqs
        .iter()
        .find(|r| r.url.path() == "/session/v1/login-request")
        .expect("no login-request captured");
    serde_json::from_slice(&login.body).expect("login body is not valid JSON")
}

#[test]
fn should_send_platform_disabled_when_detection_is_disabled_via_env_var() {
    //Given SNOWFLAKE_DISABLE_PLATFORM_DETECTION is set to "true"
    temp_env::with_var("SNOWFLAKE_DISABLE_PLATFORM_DETECTION", Some("true"), || {
        //And Wiremock is running with a password login-success mapping
        let fixture = PlatformDetectionFixture::new();

        //When Trying to Connect
        fixture.client.connect().expect("connect should succeed");

        //Then The login-request body contains CLIENT_ENVIRONMENT.PLATFORM equal to ["disabled"]
        let body = login_request_body(&fixture.mock);
        let platform = &body["data"]["CLIENT_ENVIRONMENT"]["PLATFORM"];
        assert_eq!(
            platform,
            &serde_json::json!(["disabled"]),
            "expected PLATFORM=[\"disabled\"], got body: {body}"
        );
    });
}

#[test]
fn should_send_empty_platform_array_when_detection_produces_no_platforms() {
    //Given SNOWFLAKE_DISABLE_PLATFORM_DETECTION is unset
    temp_env::with_var_unset("SNOWFLAKE_DISABLE_PLATFORM_DETECTION", || {
        //And Wiremock is running with a password login-success mapping
        let fixture = PlatformDetectionFixture::new();

        //When Trying to Connect
        fixture.client.connect().expect("connect should succeed");

        //Then The login-request body contains CLIENT_ENVIRONMENT.PLATFORM equal to []
        let body = login_request_body(&fixture.mock);
        let platform = &body["data"]["CLIENT_ENVIRONMENT"]["PLATFORM"];
        assert_eq!(
            platform,
            &serde_json::json!([]),
            "expected PLATFORM=[], got body: {body}"
        );
    });
}
