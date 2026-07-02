use sf_core::telemetry::platform_detection::platform_detection_env_vars;

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
        .find(|req| req.url.path() == "/session/v1/login-request")
        .expect("no login-request captured");
    serde_json::from_slice(&login.body).expect("login body is not valid JSON")
}

#[test]
fn should_send_platform_disabled_when_detection_is_disabled_via_env_var() {
    //Given SNOWFLAKE_DISABLE_PLATFORM_DETECTION is set to "true"
    temp_env::with_vars(
        platform_detection_env_vars(&[("SNOWFLAKE_DISABLE_PLATFORM_DETECTION", "true")]),
        || {
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
        },
    );
}

#[test]
fn should_detect_aws_lambda() {
    //Given LAMBDA_TASK_ROOT is set to "/var/task"
    temp_env::with_vars(
        platform_detection_env_vars(&[("LAMBDA_TASK_ROOT", "/var/task")]),
        || {
            //And Wiremock is running with a password login-success mapping
            let fixture = PlatformDetectionFixture::new();

            //When Trying to Connect
            fixture.client.connect().expect("connect should succeed");

            //Then The login-request body contains CLIENT_ENVIRONMENT.PLATFORM containing "is_aws_lambda"
            let body = login_request_body(&fixture.mock);
            let platform = &body["data"]["CLIENT_ENVIRONMENT"]["PLATFORM"];
            let arr = platform
                .as_array()
                .expect("PLATFORM should be an array in login-request body");
            assert!(
                arr.iter().any(|v| v == "is_aws_lambda"),
                "expected PLATFORM to include \"is_aws_lambda\", got body: {body}"
            );
        },
    );
}
