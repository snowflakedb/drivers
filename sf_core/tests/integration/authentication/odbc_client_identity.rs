//! End-to-end wire-level test for the ODBC wrapper's CLIENT_APP_ID /
//! CLIENT_ENVIRONMENT.APPLICATION contract.
//!
//! The ODBC wrapper feeds two pieces of identity to sf_core:
//!
//! 1. `client_app_id="ODBC"` — supplied via `wrapper_identity` injection at
//!    `connection_init` time.
//! 2. `application=<user value>` — supplied via `connection_set_options`
//!    from the `APPLICATION` connection-string key (or
//!    `SQL_SF_CONN_ATTR_APPLICATION`).
//!
//! These tests stub the Snowflake login endpoint via wiremock and assert on
//! the captured request body, mirroring what the Python wiremock test does
//! one layer up. They cover the contract from settings → on-the-wire login
//! payload; the wrapper-identity injection mechanism itself is covered by
//! `tests/integration/telemetry/wrapper_identity.rs`.
use crate::common::mocks::password;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;
use wiremock::Mock;
use wiremock::matchers::{body_partial_json, method, path_regex};

/// A SnowflakeTestClient configured the way the ODBC wrapper does on the
/// wire: client_app_id="ODBC", password auth, no application yet.
fn odbc_client(mock: &MockServerWithTls) -> SnowflakeTestClient {
    let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
    client.set_connection_option("password", "test_password"); // pragma: allowlist secret
    client.set_connection_option("client_app_id", "ODBC");
    client
}

#[test]
fn odbc_login_sends_client_app_id_odbc_and_user_application() {
    // User passed APPLICATION=Tableau on the connection string. CLIENT_APP_ID
    // must be "ODBC", CLIENT_ENVIRONMENT.APPLICATION must carry the user value.
    let mock = MockServerWithTls::start();
    let client = odbc_client(&mock);
    client.set_connection_option("application", "Tableau");

    mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/session/v1/login-request"))
            .and(body_partial_json(serde_json::json!({
                "data": {
                    "CLIENT_APP_ID": "ODBC",
                    "CLIENT_ENVIRONMENT": {
                        "APPLICATION": "Tableau"
                    }
                }
            })))
            .respond_with(password::success_login_response()),
    );

    let result = client.connect();

    assert!(
        result.is_ok(),
        "Expected ODBC login to succeed (mock matches CLIENT_APP_ID=ODBC + APPLICATION=Tableau), got: {result:?}"
    );
}

#[test]
fn odbc_login_without_application_falls_back_to_client_app_id() {
    // No APPLICATION provided. CLIENT_ENVIRONMENT.APPLICATION must fall back
    // to client_app_id ("ODBC") — the behavior in ClientInfo::from_settings.
    let mock = MockServerWithTls::start();
    let client = odbc_client(&mock);

    mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/session/v1/login-request"))
            .and(body_partial_json(serde_json::json!({
                "data": {
                    "CLIENT_APP_ID": "ODBC",
                    "CLIENT_ENVIRONMENT": {
                        "APPLICATION": "ODBC"
                    }
                }
            })))
            .respond_with(password::success_login_response()),
    );

    let result = client.connect();

    assert!(
        result.is_ok(),
        "Expected ODBC login without APPLICATION to fall back to CLIENT_APP_ID, got: {result:?}"
    );
}

#[test]
fn odbc_application_does_not_override_client_app_id() {
    // Regression guard: the user's APPLICATION value (now routed to the
    // canonical ``application`` setting) must NEVER bleed into CLIENT_APP_ID.
    // Same regression PR #1175 fixed for Python.
    let mock = MockServerWithTls::start();
    let client = odbc_client(&mock);
    client.set_connection_option("application", "SNOWCLI.STAGE.COPY");

    mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/session/v1/login-request"))
            .and(body_partial_json(serde_json::json!({
                "data": {
                    "CLIENT_APP_ID": "ODBC",
                    "CLIENT_ENVIRONMENT": {
                        "APPLICATION": "SNOWCLI.STAGE.COPY"
                    }
                }
            })))
            .respond_with(password::success_login_response()),
    );

    let result = client.connect();

    assert!(
        result.is_ok(),
        "Expected APPLICATION to land in CLIENT_ENVIRONMENT.APPLICATION while CLIENT_APP_ID stays ODBC, got: {result:?}"
    );
}

#[test]
fn odbc_login_with_release_type_sends_release_type_in_client_environment() {
    // When client_release_type is set (e.g. for a PuPr RC build), it must
    // appear as CLIENT_ENVIRONMENT.RELEASE_TYPE in the login request body.
    let mock = MockServerWithTls::start();
    let client = odbc_client(&mock);
    client.set_connection_option("client_release_type", "rc1");

    mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/session/v1/login-request"))
            .and(body_partial_json(serde_json::json!({
                "data": {
                    "CLIENT_APP_ID": "ODBC",
                    "CLIENT_ENVIRONMENT": {
                        "RELEASE_TYPE": "rc1"
                    }
                }
            })))
            .respond_with(password::success_login_response()),
    );

    let result = client.connect();

    assert!(
        result.is_ok(),
        "Expected ODBC login to send CLIENT_ENVIRONMENT.RELEASE_TYPE=rc1, got: {result:?}"
    );
}

#[test]
fn odbc_login_without_release_type_omits_release_type_field() {
    // When client_release_type is not set (GA build), CLIENT_ENVIRONMENT must
    // not contain RELEASE_TYPE at all.
    let mock = MockServerWithTls::start();
    let client = odbc_client(&mock);

    mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/session/v1/login-request"))
            .and(ReleaseTypeFieldAbsent)
            .respond_with(password::success_login_response()),
    );

    let result = client.connect();

    assert!(
        result.is_ok(),
        "Expected ODBC login without release_type to omit CLIENT_ENVIRONMENT.RELEASE_TYPE, got: {result:?}"
    );
}

/// Matches only requests whose `CLIENT_ENVIRONMENT` does **not** contain a
/// `RELEASE_TYPE` key. Used to assert GA builds send no release type.
struct ReleaseTypeFieldAbsent;

impl wiremock::Match for ReleaseTypeFieldAbsent {
    fn matches(&self, request: &wiremock::Request) -> bool {
        let Ok(body) = serde_json::from_slice::<serde_json::Value>(&request.body) else {
            return false;
        };
        body.get("data")
            .and_then(|d| d.get("CLIENT_ENVIRONMENT"))
            .and_then(|e| e.get("RELEASE_TYPE"))
            .is_none()
    }
}
