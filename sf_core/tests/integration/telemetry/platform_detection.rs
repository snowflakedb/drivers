use std::sync::Arc;

use serde_json::json;
use sf_core::fs_adapter::RealFs;
use sf_core::logging::LogManager;
use sf_core::protobuf::apis::database_driver_v1::DriverProviders;
use sf_core::telemetry::platform_detection::platform_detection_env_vars;
use wiremock::matchers::{body_partial_json, method, path, path_regex, query_param};
use wiremock::{Mock, ResponseTemplate};

use super::common::decompress_gzip_json;
use crate::common::mocks::password;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;

struct PlatformDetectionFixture {
    mock: MockServerWithTls,
    client: SnowflakeTestClient,
}

impl PlatformDetectionFixture {
    fn new() -> Self {
        Self::with(password::login_success())
    }

    fn with_telemetry_disabled() -> Self {
        Self::with(login_with_telemetry_disabled())
    }

    fn with(login: Mock) -> Self {
        let mock = MockServerWithTls::start();
        let providers = DriverProviders {
            log_manager: Some(LogManager::with_none_subscriber(Arc::new(RealFs))),
            ..Default::default()
        };
        let client =
            SnowflakeTestClient::with_int_tests_params_using(Some(&mock.http_url()), providers);
        client.set_connection_option("password", "test_password"); // pragma: allowlist secret
        mock.mount(login);
        mock.mount(
            Mock::given(method("POST"))
                .and(path("/telemetry/send"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true}))),
        );
        mock.mount(
            Mock::given(method("POST"))
                .and(path("/session"))
                .and(query_param("delete", "true"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true}))),
        );
        Self { mock, client }
    }

    fn connect_and_close(&self) {
        self.client.connect().expect("connect should succeed");
        self.client
            .connection_close_blocking()
            .expect("close should succeed");
    }
}

fn detection_disabled() -> Vec<(&'static str, Option<&'static str>)> {
    platform_detection_env_vars(&[("SNOWFLAKE_DISABLE_PLATFORM_DETECTION", "true")])
}

fn login_with_telemetry_disabled() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "LOGIN_NAME": "test_user",
                "PASSWORD": "test_password",
                "EXT_AUTHN_DUO_METHOD": "push"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "token": "mock_session_token",
                "masterToken": "mock_master_token",
                "sessionId": 12345,
                "validityInSeconds": 3600,
                "masterValidityInSeconds": 14400,
                "parameters": [
                    {"name": "CLIENT_TELEMETRY_ENABLED", "value": false}
                ],
                "sessionInfo": {
                    "databaseName": "test_database",
                    "schemaName": "test_schema",
                    "warehouseName": "test_warehouse",
                    "roleName": "test_role"
                }
            }
        })))
}

fn assert_logins_omit_platform(mock: &MockServerWithTls) {
    let logins: Vec<_> = mock
        .received_requests()
        .into_iter()
        .filter(|req| req.url.path() == "/session/v1/login-request")
        .collect();
    assert!(!logins.is_empty(), "no login-request captured");
    for login in logins {
        let body: serde_json::Value =
            serde_json::from_slice(&login.body).expect("login body is not valid JSON");
        let environment = &body["data"]["CLIENT_ENVIRONMENT"];
        assert!(
            environment.is_object() && environment.get("PLATFORM").is_none(),
            "login-request must carry CLIENT_ENVIRONMENT without PLATFORM, got body: {body}"
        );
    }
}

fn platform_detection_status_values(mock: &MockServerWithTls) -> Vec<serde_json::Value> {
    mock.received_requests()
        .iter()
        .filter(|req| req.url.path() == "/telemetry/send")
        .flat_map(|req| {
            decompress_gzip_json(&req.body)["logs"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .filter(|entry| entry["message"]["type"] == "platform_detection_status")
        .map(|entry| entry["message"]["value"].clone())
        .collect()
}

#[test]
fn should_report_the_cached_detection_result_once_per_session() {
    temp_env::with_vars(detection_disabled(), || {
        let mut fixture = PlatformDetectionFixture::new();

        fixture.connect_and_close();
        temp_env::with_vars(platform_detection_env_vars(&[]), || {
            fixture.client.open_another_connection();
            fixture
                .client
                .set_connection_option("password", "test_password"); // pragma: allowlist secret
            fixture.connect_and_close();
        });

        assert_eq!(
            platform_detection_status_values(&fixture.mock),
            vec![json!(["disabled"]), json!(["disabled"])]
        );
        assert_logins_omit_platform(&fixture.mock);
    });
}

#[test]
fn should_not_report_platform_detection_when_telemetry_is_disabled() {
    temp_env::with_vars(detection_disabled(), || {
        let fixture = PlatformDetectionFixture::with_telemetry_disabled();

        fixture.connect_and_close();

        assert!(platform_detection_status_values(&fixture.mock).is_empty());
    });
}

#[test]
fn should_include_detected_platforms_in_the_connection_diagnostic() {
    temp_env::with_vars(detection_disabled(), || {
        let log_dir = tempfile::tempdir().expect("diagnostic log dir");
        let fixture = PlatformDetectionFixture::new();
        fixture
            .client
            .set_connection_option_bool("enable_connection_diag", true);
        fixture.client.set_connection_option(
            "connection_diag_log_path",
            &log_dir.path().to_string_lossy(),
        );

        fixture.connect_and_close();

        let report =
            std::fs::read_to_string(log_dir.path().join("SnowflakeConnectionTestReport.txt"))
                .expect("diagnostic report");
        assert!(
            report.contains("Detected platforms: disabled"),
            "diagnostic report should name the detection result, got:\n{report}"
        );
    });
}
