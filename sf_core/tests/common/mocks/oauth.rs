//! OAuth 2.0 authentication mock helpers.
//!
//! Each function returns a `wiremock::Mock` for one logical step in the
//! OAuth Authorization Code (AC), Client Credentials (CC), or legacy
//! pre-acquired-token flow. These mirror the behavioral fixtures shipped
//! with the ODBC driver under
//! `Tests/UnitTests/UnitOAuthTest/wiremock/{idp_responses,snowflake_responses}`
//! and the cross-driver expectations (loopback binding, CC flow, legacy
//! OAUTH body shape, refresh-token rotation, refresh-on-failure,
//! login-request payload, error taxonomy). The caller mounts them via
//! `MockServerWithTls::mount`.
//!
//! Conventions:
//! * IdP token endpoint is at `POST /oauth/token-request` (the Snowflake
//!   default). External-IdP scenarios still hit this path
//!   because the harness routes the configured `oauth_token_request_url`
//!   to the same wiremock server.
//! * Snowflake login mocks match on `data.AUTHENTICATOR == "OAUTH"`
//!   (`AUTHENTICATOR` is always uppercase `OAUTH` in the login body, never
//!   the user-supplied authenticator string verbatim).

use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::json;
use wiremock::matchers::{body_partial_json, body_string_contains, method, path_regex};
use wiremock::{Mock, Respond, ResponseTemplate};

// ─── IdP token endpoint — success variants ──────────────────────────────────

/// Successful AC token exchange (`grant_type=authorization_code`).
///
/// Mirrors ODBC's `idp_auth_successful.json` (the canonical AC happy
/// path). Returns `access_token`, `refresh_token`, `token_type` and
/// `expires_in` so callers can also exercise the refresh-token rotation
/// persistence path.
pub fn idp_token_endpoint_success_authorization_code() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/oauth/token-request"))
        .and(body_string_contains("grant_type=authorization_code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "ac-access-token-success",
            "refresh_token": "ac-refresh-token-success",
            "token_type": "Bearer",
            "expires_in": 600,
            "refresh_token_expires_in": 86399
        })))
}

/// AC token exchange whose body asserts `enable_single_use_refresh_tokens=true`.
///
/// Mirrors ODBC's `idp_auth_successful_with_single_use_refresh_token.json`
/// The wiremock body matcher only fires when the
/// driver actually included the flag, so a missing flag fails the test
/// with a 404 from wiremock and the OAuth flow surfaces a token-exchange
/// error.
pub fn idp_token_endpoint_success_with_single_use_refresh_token() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/oauth/token-request"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains(
            "enable_single_use_refresh_tokens=true",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "ac-access-token-single-use",
            "refresh_token": "ac-refresh-token-rotated",
            "token_type": "Bearer",
            "expires_in": 600
        })))
}

/// Successful CC token exchange (`grant_type=client_credentials`).
///
/// Mirrors ODBC's `idp_client_successful.json`. CC tokens are
/// intentionally never cached (CC is stateless by design), so this fixture does
/// not include a `refresh_token`.
pub fn idp_token_endpoint_success_client_credentials() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/oauth/token-request"))
        .and(body_string_contains("grant_type=client_credentials"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "cc-access-token-success",
            "token_type": "Bearer",
            "expires_in": 900
        })))
}

/// Successful refresh-token exchange (`grant_type=refresh_token`).
///
/// Mirrors ODBC's `idp_refresh_successful.json`. Returns a fresh access
/// token AND a rotated refresh token so callers can also assert the
/// rotated RT is persisted to the cache (single-use refresh-token rotation).
pub fn idp_token_endpoint_success_refresh() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/oauth/token-request"))
        .and(body_string_contains("grant_type=refresh_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "ac-access-token-refreshed",
            "refresh_token": "ac-refresh-token-rotated",
            "token_type": "Bearer",
            "expires_in": 599
        })))
}

// ─── IdP token endpoint — error variants ────────────────────────────────────

/// IdP returns 400 with an `invalid_scope` body. Mirrors ODBC's
/// `idp_auth_invalid_scope.json`. The OAuth flow should surface
/// `OAuthError::IdpError { error: "invalid_scope", .. }`.
pub fn idp_token_endpoint_invalid_scope() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/oauth/token-request"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "invalid_scope",
            "error_description": "One or more scopes are not configured for the authorization server resource."
        })))
}

/// 200 OK with an empty/missing `access_token` — mirrors ODBC's
/// `idp_auth_missing_access_token.json` (the field is present but
/// empty, which our flow treats as missing). The flow
/// surfaces `OAuthError::MissingAccessToken`.
pub fn idp_token_endpoint_missing_access_token() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/oauth/token-request"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "",
            "refresh_token": "rt-but-no-at",
            "token_type": "Bearer",
            "expires_in": 600
        })))
}

/// IdP returns a generic 500 (no parseable body). Mirrors ODBC's
/// `idp_auth_token_request_error.json` and `idp_client_token_request_error.json`.
/// The flow surfaces `OAuthError::TokenExchange { status: 500, .. }`.
pub fn idp_token_endpoint_token_request_error() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/oauth/token-request"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error from IdP"))
}

/// IdP returns 400 with an explicit `invalid_token` IdP-side error (the
/// body's `access_token` field is empty so even if the body were 200,
/// `MissingAccessToken` would fire). Mirrors ODBC's
/// `idp_auth_invalid_access_token.json` semantics — the IdP signals that
/// the token it would have minted is itself invalid.
pub fn idp_token_endpoint_invalid_access_token() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/oauth/token-request"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "invalid_token",
            "error_description": "Identity Provider rejected the access token"
        })))
}

/// Refresh-token exchange fails with `invalid_grant`. Mirrors ODBC's
/// `idp_refresh_failed.json`. After this fires the OAuth flow evicts the
/// cached refresh token and falls back to the full interactive flow
/// (evicts RT, then falls back to full interactive flow).
pub fn idp_token_endpoint_refresh_failed() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/oauth/token-request"))
        .and(body_string_contains("grant_type=refresh_token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "invalid_grant",
            "error_description": "Refresh token expired or revoked"
        })))
}

// ─── Snowflake login endpoint — OAuth variants ──────────────────────────────

/// Successful Snowflake login for OAuth, asserting the request body
/// carries `AUTHENTICATOR=OAUTH` *and* the supplied `expected_token`
/// value in `data.TOKEN`. The `expected_token` echo
/// lets callers prove that a specific cached / freshly-acquired access
/// token was actually forwarded to GS — particularly useful for the
/// cached-AT short-circuit assertion.
pub fn snowflake_login_success_oauth(expected_token: &str) -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "OAUTH",
                "TOKEN": expected_token
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "token": "mock_session_token_oauth",
                "masterToken": "mock_master_token_oauth",
                "sessionId": 12345,
                "validityInSeconds": 3600,
                "masterValidityInSeconds": 14400
            }
        })))
}

/// Snowflake login returns `390303` (`OAUTH_ACCESS_TOKEN_INVALID`).
/// Mirrors the cross-driver eviction trigger — drivers
/// remove the cached access token and replay the login.
pub fn snowflake_login_oauth_access_token_invalid_390303() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "OAUTH"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "code": "390303",
            "message": "OAuth access token is invalid."
        })))
}

/// Snowflake login returns `390318` (`OAUTH_ACCESS_TOKEN_EXPIRED`).
/// Same eviction-and-replay semantics as 390303.
pub fn snowflake_login_oauth_access_token_expired_390318() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "OAUTH"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "code": "390318",
            "message": "OAuth access token has expired."
        })))
}

/// Refresh-on-failure fixture: the first matching OAuth login returns
/// the supplied Snowflake error `code` (`390303` or `390318`);
/// subsequent matching OAuth logins succeed. Returning both responses
/// from a single counter-backed responder (instead of two
/// `up_to_n_times`-gated mocks) sidesteps wiremock's mount-ordering
/// ambiguity when two equal-priority mocks match the same request —
/// cross-driver expectation: eviction always runs exactly once before
/// the retry succeeds (390303/390318 refresh-on-failure semantics).
///
/// The retry success body uses distinct token values
/// (`mock_session_token_oauth_retry`, `mock_master_token_oauth_retry`)
/// so test assertions can unambiguously distinguish the retry leg from
/// the initial failed leg.
pub fn snowflake_login_oauth_then_success(code: &str) -> Mock {
    let message = match code {
        "390303" => "OAuth access token is invalid.",
        "390318" => "OAuth access token has expired.",
        _ => "OAuth access token error.",
    };
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "OAUTH"
            }
        })))
        .respond_with(ThenSuccessResponder::new(code, message))
}

/// Counter-backed responder that returns a failing OAuth login (JSON
/// body with `success=false` and the supplied Snowflake error `code`)
/// on the first match, and a success body on every subsequent match.
struct ThenSuccessResponder {
    calls: AtomicUsize,
    failure_code: String,
    failure_message: String,
}

impl ThenSuccessResponder {
    fn new(code: &str, message: &str) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            failure_code: code.to_string(),
            failure_message: message.to_string(),
        }
    }
}

impl Respond for ThenSuccessResponder {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            ResponseTemplate::new(200).set_body_json(json!({
                "success": false,
                "code": self.failure_code,
                "message": self.failure_message,
            }))
        } else {
            ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "token": "mock_session_token_oauth_retry",
                    "masterToken": "mock_master_token_oauth_retry",
                    "sessionId": 12345,
                    "validityInSeconds": 3600,
                    "masterValidityInSeconds": 14400,
                }
            }))
        }
    }
}
