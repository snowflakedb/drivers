//! Session management mock helpers.

use serde_json::json;
use wiremock::matchers::{header, method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mount a successful token refresh response.
///
/// Matches requests with valid-master-token in Authorization header.
pub async fn mount_token_refresh_success(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/token-request.*"))
        .and(header(
            "Authorization",
            "Snowflake Token=\"valid-master-token\"",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true,
                    "data": {
                        "sessionToken": "new-session-token",
                        "masterToken": "new-master-token",
                        "sessionId": 12345
                    }
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(server)
        .await;
}

/// Mount a 401 response for queries with expired session token.
///
/// This is meant to be used with `up_to_n_times(1)` in a scenario
/// where the first query fails, triggers refresh, then succeeds.
pub async fn mount_query_401_expired_token(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .and(header(
            "Authorization",
            "Snowflake Token=\"expired-session-token\"",
        ))
        .respond_with(ResponseTemplate::new(401).set_body_string("Session expired"))
        .up_to_n_times(1)
        .mount(server)
        .await;
}

/// Mount a successful query response for refreshed token.
pub async fn mount_query_success_after_refresh(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .and(header(
            "Authorization",
            "Snowflake Token=\"new-session-token\"",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true,
                    "data": {
                        "queryId": "test-query-id",
                        "rowtype": [],
                        "rowset": [[1]]
                    }
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(server)
        .await;
}

/// Mount a 401 response for token refresh when master token is also expired.
pub async fn mount_refresh_failure_master_expired(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path_regex(r"/session/token-request.*"))
        .and(header(
            "Authorization",
            "Snowflake Token=\"expired-master-token\"",
        ))
        .respond_with(ResponseTemplate::new(401).set_body_string("Master token expired"))
        .mount(server)
        .await;
}
