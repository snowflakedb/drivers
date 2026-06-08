//! Authentication mock helpers.

use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mount a successful JWT authentication response.
///
/// Matches POST requests to `/session/v1/login-request.*` with SNOWFLAKE_JWT authenticator.
pub async fn mount_jwt_login_success(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request.*"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "SNOWFLAKE_JWT"
            }
        })))
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

/// Mount a successful JWT login response that includes `CLIENT_SESSION_KEEP_ALIVE`
/// in the session parameters and a short `masterValidityInSeconds` so heartbeat
/// interval computation yields a testable value.
pub async fn mount_jwt_login_with_keep_alive(server: &MockServer, keep_alive: bool) {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request.*"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "SNOWFLAKE_JWT"
            }
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true,
                    "data": {
                        "token": "mock_token",
                        "masterToken": "mock_master_token",
                        "masterValidityInSeconds": 1,
                        "sessionId": 12345,
                        "parameters": [
                            {
                                "name": "CLIENT_SESSION_KEEP_ALIVE",
                                "value": keep_alive
                            }
                        ]
                    }
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(server)
        .await;
}
