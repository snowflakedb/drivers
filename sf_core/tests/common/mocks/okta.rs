//! Native Okta authentication mock helpers.
//!
//! Replaces the JSON mapping files previously loaded into Java WireMock.
//! Each function returns a `wiremock::Mock` for one logical step of the Okta
//! SAML authentication flow. The caller mounts them via `MockServerWithTls::mount`.

use serde_json::json;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, Respond, ResponseTemplate};

// ─── Snowflake Authenticator Request ─────────────────────────────────────────

pub fn authenticator_request(okta_base_url: &str) -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/authenticator-request"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "tokenUrl": format!("{okta_base_url}/api/v1/authn"),
                "ssoUrl":   format!("{okta_base_url}/app/sso/saml"),
            }
        })))
}

pub fn authenticator_request_mismatched_token_url(okta_base_url: &str) -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/authenticator-request"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "tokenUrl": "https://attacker.evil.com/api/v1/authn",
                "ssoUrl":   format!("{okta_base_url}/app/sso/saml"),
            }
        })))
}

pub fn authenticator_request_mismatched_sso_url(okta_base_url: &str) -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/authenticator-request"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "tokenUrl": format!("{okta_base_url}/api/v1/authn"),
                "ssoUrl":   "https://attacker.evil.com/app/sso/saml",
            }
        })))
}

// ─── Okta Token Endpoint (/api/v1/authn) ─────────────────────────────────────

pub fn okta_token_success() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/api/v1/authn"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sessionToken": "okta-session-token",
            "relayState": "relay-state"
        })))
}

pub fn okta_token_cookie_token() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/api/v1/authn"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "cookieToken": "okta-cookie-token",
            "relayState": "relay-state"
        })))
}

pub fn okta_token_401() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/api/v1/authn"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "errorCode": "E0000004",
            "errorSummary": "Authentication failed",
            "errorLink": "E0000004",
            "errorId": "oaeWq9j-VQ1v3cP2_VkAg",
            "errorCauses": []
        })))
}

pub fn okta_token_403() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/api/v1/authn"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "errorCode": "E0000006",
            "errorSummary": "You do not have permission to perform the requested action",
            "errorLink": "E0000006",
            "errorId": "oaeWq9j-VQ1v3cP2_VkAg",
            "errorCauses": []
        })))
}

pub fn okta_token_mfa_required() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/api/v1/authn"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "MFA_REQUIRED",
            "stateToken": "00abc123",
            "_embedded": {
                "factors": [{
                    "id": "factor123",
                    "factorType": "sms",
                    "provider": "OKTA"
                }]
            }
        })))
}

// ─── Okta SSO SAML Endpoint (/app/sso/saml) ─────────────────────────────────

fn saml_html(postback_url: &str) -> String {
    format!(
        r#"<html><body><form method="post" action="{postback_url}"><input type="hidden" name="SAMLResponse" value="AAAA" /></form></body></html>"#
    )
}

pub fn okta_sso_success(snowflake_base_url: &str) -> Mock {
    let body = saml_html(&format!("{snowflake_base_url}/fed/login"));
    Mock::given(method("GET"))
        .and(path_regex(r"/app/sso/saml"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/html")
                .set_body_string(body),
        )
}

pub fn okta_sso_mismatched_postback() -> Mock {
    let body = saml_html("https://attacker.evil.com/fed/login");
    Mock::given(method("GET"))
        .and(path_regex(r"/app/sso/saml"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/html")
                .set_body_string(body),
        )
}

pub fn okta_sso_missing_form_action() -> Mock {
    let body = r#"<html><body><form method="post"><input type="hidden" name="SAMLResponse" value="AAAA" /></form></body></html>"#;
    Mock::given(method("GET"))
        .and(path_regex(r"/app/sso/saml"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/html")
                .set_body_string(body),
        )
}

// ─── Okta SSO Retry Scenario ─────────────────────────────────────────────────
//
// Simulates a transient 503 on the first SSO attempt, then success on retry.
// Register the 503 mock with `up_to_n_times(1)`.  Once exhausted, subsequent
// requests fall through to a second (success) mock on the same path.

pub fn okta_sso_503_once() -> Mock {
    Mock::given(method("GET"))
        .and(path_regex(r"/app/sso/saml"))
        .respond_with(Sso503Responder)
        .up_to_n_times(1)
}

struct Sso503Responder;

impl Respond for Sso503Responder {
    fn respond(&self, _request: &wiremock::Request) -> ResponseTemplate {
        ResponseTemplate::new(503).set_body_string("Service Temporarily Unavailable")
    }
}

// ─── Snowflake Login Request ─────────────────────────────────────────────────

pub fn login_success() -> Mock {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "token": "mock_token",
                "masterToken": "mock_master_token",
                "sessionId": 12345
            }
        })))
}
