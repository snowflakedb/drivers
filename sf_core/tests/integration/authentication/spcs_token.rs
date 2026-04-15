use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;
use sf_core::config::rest_parameters::{ClientInfo, LoginMethod, LoginParameters};
use sf_core::rest::snowflake::{AuthContext, RuntimePaths, snowflake_login_with_client};
use std::io::Write;
use wiremock::matchers::method;
use wiremock::{Match, Mock, Request, ResponseTemplate};

struct SpcsTokenFieldAbsent;

impl Match for SpcsTokenFieldAbsent {
    fn matches(&self, request: &Request) -> bool {
        let Ok(body) = serde_json::from_slice::<serde_json::Value>(&request.body) else {
            return false;
        };
        body.get("data").and_then(|d| d.get("SPCS_TOKEN")).is_none()
    }
}

fn success_login_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "success": true,
        "data": {
            "token": "mock_session_token",
            "masterToken": "mock_master_token",
            "sessionId": 12345,
            "validityInSeconds": 3600,
            "masterValidityInSeconds": 14400,
            "parameters": [],
            "sessionInfo": {
                "databaseName": "test_database",
                "schemaName": "test_schema",
                "warehouseName": "test_warehouse",
                "roleName": "test_role"
            }
        }
    }))
}

fn test_client_info() -> ClientInfo {
    ClientInfo {
        application: "TestApp".to_string(),
        version: "1.0.0".to_string(),
        os: "Linux".to_string(),
        os_version: "5.15".to_string(),
        ocsp_mode: None,
        crl_config: Default::default(),
        tls_config: Default::default(),
    }
}

/*
Call topic - How do we mock files in a language which doesnt allow monkey patching/auto reflections?

This topic will return for /etc/os-version and libc family features.

------

Dependency injection requires lots of additional code in entire call chain = makes code harder to read and maintain.
DI isn't justified if we need dependencies only for tests and real app uses defaults

- solve this by dependency injection and calling snowflake_login_with_client with mocked auth context
  - we can't perform full client test
  - we can't perform test from host driver
- solve this by storing test-specific connection options on ConnectionConfig
 - lots of code to support this
- use thread_local! overwrites?
  pub mod test_overrides {
    thread_local! {
        pub static SPCS_TOKEN_PATH: RefCell<Option<PathBuf>> = RefCell::new(None);
    }
  }
  clean implementation that doesn't change anything except the module where we'd like mocking
*/

#[test]
fn should_not_include_spcs_token_when_env_var_is_not_set() {
    temp_env::with_var_unset("SNOWFLAKE_RUNNING_INSIDE_SPCS", || {
        // Given: WireMock rejects requests that contain SPCS_TOKEN
        let mock = MockServerWithTls::start();
        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("password", "test_password"); // pragma: allowlist secret

        let login_mock = Mock::given(method("POST"))
            .and(wiremock::matchers::path_regex(r"/session/v1/login-request"))
            .and(wiremock::matchers::body_partial_json(serde_json::json!({
                "data": {
                    "LOGIN_NAME": "test_user",
                    "PASSWORD": "test_password"
                }
            })))
            .and(SpcsTokenFieldAbsent)
            .respond_with(success_login_response());
        mock.mount(login_mock);

        // When: connecting
        let result = client.connect();

        // Then: login succeeds (the mock matched, meaning SPCS_TOKEN was absent)
        assert!(
            result.is_ok(),
            "Expected login without SPCS_TOKEN to succeed, got: {result:?}"
        );
    });
}

#[test]
fn should_include_spcs_token_when_env_var_is_set_and_file_exists() {
    // Given: a temp file containing the SPCS token
    let mut token_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(token_file, "  my-spcs-token").unwrap();

    let mock = MockServerWithTls::start();

    // WireMock expects SPCS_TOKEN (trimmed) in the request body
    let login_mock = Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex(r"/session/v1/login-request"))
        .and(wiremock::matchers::body_partial_json(serde_json::json!({
            "data": {
                "LOGIN_NAME": "test_user",
                "PASSWORD": "test_password",
                "SPCS_TOKEN": "my-spcs-token"
            }
        })))
        .respond_with(success_login_response());
    mock.mount(login_mock);

    let login_params = LoginParameters {
        account_name: "test_account".to_string(),
        login_method: LoginMethod::Password {
            username: "test_user".to_string(),
            password: "test_password".into(), // pragma: allowlist secret
        },
        server_url: mock.http_url(),
        database: None,
        schema: None,
        warehouse: None,
        role: None,
        client_info: test_client_info(),
        session_parameters: None,
    };

    let auth_context = AuthContext {
        runtime_paths: RuntimePaths {
            spcs_token_file: token_file.path().to_path_buf(),
        },
        token_cache: None,
    };

    // When: logging in with SNOWFLAKE_RUNNING_INSIDE_SPCS set
    let rt = tokio::runtime::Runtime::new().unwrap();
    let http_client = reqwest::Client::new();

    let result = temp_env::with_var("SNOWFLAKE_RUNNING_INSIDE_SPCS", Some("true"), || {
        rt.block_on(snowflake_login_with_client(
            &http_client,
            &login_params,
            None,
            &auth_context,
        ))
    });

    // Then: login succeeds (the mock matched, meaning SPCS_TOKEN was present and trimmed)
    assert!(
        result.is_ok(),
        "Expected login with SPCS_TOKEN to succeed, got: {result:?}"
    );
}
