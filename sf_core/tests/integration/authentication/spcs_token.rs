use crate::common::mocks::password;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;
use sf_core::rest::snowflake::spcs_token::test_overrides::set_spcs_token_path;
use std::io::Write;
use wiremock::matchers::{body_partial_json, method, path_regex};
use wiremock::{Match, Mock, Request};

struct SpcsTestFixture {
    mock: MockServerWithTls,
    client: SnowflakeTestClient,
}

impl SpcsTestFixture {
    fn new() -> Self {
        let mock = MockServerWithTls::start();
        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("password", "test_password"); // pragma: allowlist secret
        Self { mock, client }
    }
}

struct SpcsTokenFieldAbsent;

impl Match for SpcsTokenFieldAbsent {
    fn matches(&self, request: &Request) -> bool {
        let Ok(body) = serde_json::from_slice::<serde_json::Value>(&request.body) else {
            return false;
        };
        body.get("data").and_then(|d| d.get("SPCS_TOKEN")).is_none()
    }
}

#[test]
fn should_not_include_spcs_token_when_env_var_is_not_set() {
    let fixture = SpcsTestFixture::new();
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/session/v1/login-request"))
            .and(SpcsTokenFieldAbsent)
            .respond_with(password::success_login_response()),
    );

    temp_env::with_var_unset("SNOWFLAKE_RUNNING_INSIDE_SPCS", || {
        let result = fixture.client.connect();
        assert!(
            result.is_ok(),
            "Expected login without SPCS_TOKEN to succeed, got: {result:?}"
        );
    });
}

#[test]
fn should_include_spcs_token_when_env_var_is_set_and_file_exists() {
    let fixture = SpcsTestFixture::new();
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/session/v1/login-request"))
            .and(body_partial_json(serde_json::json!({
                "data": {
                    "SPCS_TOKEN": "my-spcs-token"
                }
            })))
            .respond_with(password::success_login_response()),
    );

    temp_env::with_var("SNOWFLAKE_RUNNING_INSIDE_SPCS", Some("true"), || {
        let mut token_file = tempfile::NamedTempFile::new().unwrap();
        writeln!(token_file, "my-spcs-token").unwrap();
        let _token_path_guard = set_spcs_token_path(token_file.path().to_path_buf());

        let result = fixture.client.connect();
        assert!(
            result.is_ok(),
            "Expected login with SPCS_TOKEN to succeed, got: {result:?}"
        );
    });
}
