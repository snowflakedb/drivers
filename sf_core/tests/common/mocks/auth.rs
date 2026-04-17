//! Authentication mock helpers.

use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path_regex};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

/// Asserts that `data.CLIENT_ENVIRONMENT.PLATFORM`, if present, is a JSON
/// array. `body_partial_json` cannot express "field exists AND is an array
/// of any shape", so the check is split out as a custom matcher.
///
/// The detected platform list is host-dependent (e.g. `is_github_action` on
/// CI, `is_ec2_instance` on self-hosted runners), so matching exact values
/// would be flaky. The serde rename (`platforms` ->  `PLATFORM`) and the
/// `Vec<String>` wire shape are the only invariants the driver guarantees.
struct PlatformFieldIsArrayWhenPresent;

impl Match for PlatformFieldIsArrayWhenPresent {
    fn matches(&self, request: &Request) -> bool {
        let Ok(body) = serde_json::from_slice::<serde_json::Value>(&request.body) else {
            return false;
        };
        match body
            .get("data")
            .and_then(|d| d.get("CLIENT_ENVIRONMENT"))
            .and_then(|env| env.get("PLATFORM"))
        {
            Some(value) => value.is_array(),
            None => true,
        }
    }
}

/// Mount a successful JWT authentication response.
///
/// Matches POST requests to `/session/v1/login-request.*` with SNOWFLAKE_JWT
/// authenticator, and verifies that `CLIENT_ENVIRONMENT.PLATFORM` (when
/// emitted) is a JSON array.
pub async fn mount_jwt_login_success(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request.*"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "SNOWFLAKE_JWT"
            }
        })))
        .and(PlatformFieldIsArrayWhenPresent)
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true,
                    "data": {
                        "token": "mock_token",
                        "masterToken": "mock_master_token",
                        "sessionId": 12345
                    }
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(server)
        .await;
}
