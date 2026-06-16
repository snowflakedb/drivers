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

/// Login request that fails for a specific browser callback token value.
///
/// Use when multiple concurrent connections are in flight and each carries a
/// distinct callback token so the failure stub targets only the intended
/// connection.
pub fn login_failure_with_token(token: &str) -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "EXTERNALBROWSER",
                "TOKEN": token
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "code": "390100",
            "message": "Invalid credentials"
        })))
}

/// Successful login response for a specific browser callback token value.
///
/// Use when multiple concurrent connections are in flight and each carries a
/// distinct callback token so the success stub targets only the intended
/// connection.
pub fn login_success_with_token(token: &str) -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "EXTERNALBROWSER",
                "TOKEN": token
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

// ─── Cached ID Token Login ──────────────────────────────────────────────────

/// Successful login using a cached SSO ID token (no PROOF_KEY in request).
pub fn login_success_with_cached_id_token() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "ID_TOKEN",
                "TOKEN": "cached_id_token"
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

/// Successful login that returns an idToken (for caching after browser flow).
pub fn login_success_with_id_token_in_response() -> Mock {
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
                "idToken": "server_issued_id_token",
                "idTokenValidityInSeconds": 3600,
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

// ─── Cached ID Token Login (generic — any token value) ──────────────────────

/// Successful login using a cached SSO ID token where the exact token value
/// was produced dynamically (e.g., returned in the `idToken` field of a
/// previous EB login response).  Unlike `login_success_with_cached_id_token`
/// this stub matches ANY `AUTHENTICATOR=ID_TOKEN` request so callers do not
/// need to hard-code the specific token string.
pub fn login_success_for_cached_id_token_flow() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "ID_TOKEN"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "token": "mock_session_token_cached",
                "masterToken": "mock_master_token_cached",
                "sessionId": 12346,
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

// ─── EXT_AUTHN Failure with Cached ID Token ─────────────────────────────────

fn login_failure_ext_authn_with_cached_id_token(code: &str, message: &str) -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "ID_TOKEN",
                "TOKEN": "cached_id_token"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "code": code,
            "message": message
        })))
}

pub fn login_failure_ext_authn_denied_cached_id() -> Mock {
    login_failure_ext_authn_with_cached_id_token(
        "390120",
        "Authentication denied by external provider",
    )
}

pub fn login_failure_ext_authn_locked_cached_id() -> Mock {
    login_failure_ext_authn_with_cached_id_token("390123", "Account locked by external provider")
}

pub fn login_failure_ext_authn_timeout_cached_id() -> Mock {
    login_failure_ext_authn_with_cached_id_token("390126", "External authentication timed out")
}

pub fn login_failure_ext_authn_invalid_cached_id() -> Mock {
    login_failure_ext_authn_with_cached_id_token(
        "390127",
        "External authentication token is invalid",
    )
}

pub fn login_failure_ext_authn_exception_cached_id() -> Mock {
    login_failure_ext_authn_with_cached_id_token("390129", "External authentication exception")
}
