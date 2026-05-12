//! External browser authentication mock helpers.
//!
//! Each function returns a `wiremock::Mock` for a specific step of the
//! external browser SSO flow. The caller mounts them via
//! `MockServerWithTls::mount`.

use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path_regex};
use wiremock::{Mock, ResponseTemplate};

// ─── Snowflake Authenticator Request (EXTERNALBROWSER) ──────────────────────

/// Successful authenticator-request returning ssoUrl and proofKey.
///
/// `sso_url` is the URL the client would open in a browser; for integration
/// tests we won't actually open a browser — the test simulates the callback
/// directly.
pub fn authenticator_request(sso_url: &str, proof_key: &str) -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/authenticator-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "EXTERNALBROWSER"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "ssoUrl": sso_url,
                "proofKey": proof_key
            }
        })))
}

/// Authenticator-request that returns an HTTP 403 error.
pub fn authenticator_request_forbidden() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/authenticator-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "EXTERNALBROWSER"
            }
        })))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "success": false,
            "message": "Forbidden"
        })))
}

/// Authenticator-request that returns logical failure (HTTP 200, success=false).
pub fn authenticator_request_logical_failure() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/authenticator-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "EXTERNALBROWSER"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "message": "External browser authentication is not enabled for this account"
        })))
}

// ─── Snowflake Login Request (after browser callback) ───────────────────────

/// Successful login response for external browser auth.
pub fn login_success() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "EXTERNALBROWSER"
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

/// Login request that fails with an authentication error.
pub fn login_failure() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "EXTERNALBROWSER"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "code": "390100",
            "message": "Invalid credentials"
        })))
}
