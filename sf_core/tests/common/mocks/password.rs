//! Password (SNOWFLAKE) authentication mock helpers.
//!
//! Each function returns a `wiremock::Mock` for a specific password login scenario.
//! The caller mounts them via `MockServerWithTls::mount`.

use serde_json::json;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::matchers::{body_partial_json, method, path_regex};

/// Successful password login — matches a POST to login-request with LOGIN_NAME and PASSWORD
/// but WITHOUT the AUTHENTICATOR field (matching old driver behavior).
pub fn login_success() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "LOGIN_NAME": "test_user",
                "PASSWORD": "test_password"
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
                "parameters": [],
                "sessionInfo": {
                    "databaseName": "test_database",
                    "schemaName": "test_schema",
                    "warehouseName": "test_warehouse",
                    "roleName": "test_role"
                }
            }
        })))
}

/// Failed password login — wrong credentials.
pub fn login_failure_wrong_credentials() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "LOGIN_NAME": "test_user",
                "PASSWORD": "wrong_password"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "code": "390100",
            "message": "Incorrect username or password was specified."
        })))
}
