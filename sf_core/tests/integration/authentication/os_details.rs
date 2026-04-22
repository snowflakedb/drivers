use std::sync::Arc;

use crate::common::mocks::password;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;
use sf_core::fs_adapter::mock::MockFs;
use sf_core::protobuf::apis::database_driver_v1::DriverProviders;
use wiremock::Mock;
use wiremock::matchers::{body_partial_json, method, path_regex};

const MOCK_OS_RELEASE: &str = r#"
NAME="Arch Linux"
PRETTY_NAME="Arch Linux"
ID=arch
BUILD_ID=rolling
VERSION_ID=20251019.0.436919
ANSI_COLOR="38;2;23;147;209"
HOME_URL="https://archlinux.org/"
"#;

#[test]
fn should_include_os_details_on_linux() {
    if !cfg!(target_os = "linux") {
        return;
    }

    let fs = Arc::new(MockFs::new().with_file("/etc/os-release", MOCK_OS_RELEASE));
    let mock = MockServerWithTls::start();
    let client = SnowflakeTestClient::with_int_tests_params_using(
        Some(&mock.http_url()),
        DriverProviders { fs: Some(fs) },
    );
    client.set_connection_option("password", "test_password"); // pragma: allowlist secret

    mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/session/v1/login-request"))
            .and(body_partial_json(serde_json::json!({
                "data": {
                    "CLIENT_ENVIRONMENT": {
                        "OS_DETAILS": {
                            "ID": "arch",
                            "NAME": "Arch Linux",
                            "PRETTY_NAME": "Arch Linux",
                            "BUILD_ID": "rolling",
                            "VERSION_ID": "20251019.0.436919"
                        }
                    }
                }
            })))
            .respond_with(password::success_login_response()),
    );

    let result = client.connect();
    assert!(
        result.is_ok(),
        "Expected login with OS_DETAILS to succeed, got: {result:?}"
    );
}
