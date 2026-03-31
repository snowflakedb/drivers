//! MFA (USERNAME_PASSWORD_MFA) authentication mock helpers.
//!
//! Each function returns a `wiremock::Mock` for a specific MFA login scenario.
//! The caller mounts them via `MockServerWithTls::mount`.

use serde_json::json;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::matchers::{body_partial_json, method, path_regex};

// ─── Successful MFA Login (DUO push, no passcode) ───────────────────────────

pub fn login_success_with_mfa_token() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "USERNAME_PASSWORD_MFA",
                "EXT_AUTHN_DUO_METHOD": "push"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "token": "mock_session_token",
                "masterToken": "mock_master_token",
                "sessionId": 12345,
                "mfaToken": "mock_mfa_token_from_server",
                "mfaTokenValidityInSeconds": 3600
            }
        })))
}

// ─── Successful MFA Login with TOTP passcode ────────────────────────────────

pub fn login_success_with_passcode() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "USERNAME_PASSWORD_MFA",
                "EXT_AUTHN_DUO_METHOD": "passcode",
                "PASSCODE": "123456"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "token": "mock_session_token",
                "masterToken": "mock_master_token",
                "sessionId": 12345
            }
        })))
}

// ─── Successful MFA Login with passcode-in-password ──────────────────────────

pub fn login_success_passcode_in_password() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "USERNAME_PASSWORD_MFA",
                "EXT_AUTHN_DUO_METHOD": "passcode"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "token": "mock_session_token",
                "masterToken": "mock_master_token",
                "sessionId": 12345
            }
        })))
}

// ─── Successful MFA Login with cached token ──────────────────────────────────

pub fn login_success_with_cached_token() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "USERNAME_PASSWORD_MFA",
                "TOKEN": "cached_mfa_token"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "token": "mock_session_token",
                "masterToken": "mock_master_token",
                "sessionId": 12345
            }
        })))
}

// ─── Generic successful MFA login (no specific field matching) ───────────────

pub fn login_success() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "USERNAME_PASSWORD_MFA"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "token": "mock_session_token",
                "masterToken": "mock_master_token",
                "sessionId": 12345
            }
        })))
}

// ─── MFA Login Failure with EXT_AUTHN error (cached token) ───────────────────

fn login_failure_with_ext_authn_error(code: &str, message: &str) -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "USERNAME_PASSWORD_MFA",
                "TOKEN": "cached_mfa_token"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "code": code,
            "message": message
        })))
}

pub fn login_failure_ext_authn_denied() -> Mock {
    login_failure_with_ext_authn_error("390120", "Authentication denied by external provider")
}

pub fn login_failure_ext_authn_locked() -> Mock {
    login_failure_with_ext_authn_error("390123", "Account locked by external provider")
}

pub fn login_failure_ext_authn_timeout() -> Mock {
    login_failure_with_ext_authn_error("390126", "External authentication timed out")
}

pub fn login_failure_ext_authn_invalid() -> Mock {
    login_failure_with_ext_authn_error("390127", "External authentication token is invalid")
}

pub fn login_failure_ext_authn_exception() -> Mock {
    login_failure_with_ext_authn_error("390129", "External authentication exception")
}

// ─── MFA Login Failure (DUO push, no passcode) ──────────────────────────────

pub fn login_failure_duo_push() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "USERNAME_PASSWORD_MFA",
                "EXT_AUTHN_DUO_METHOD": "push"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "code": "390100",
            "message": "Incorrect username or password was specified."
        })))
}

// ─── MFA Login Failure ───────────────────────────────────────────────────────

pub fn login_failure() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "USERNAME_PASSWORD_MFA"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "code": "390100",
            "message": "Incorrect username or password was specified."
        })))
}
