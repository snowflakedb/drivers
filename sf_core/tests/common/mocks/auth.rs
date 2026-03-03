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
