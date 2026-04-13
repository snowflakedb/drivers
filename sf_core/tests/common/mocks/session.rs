//! Session management mock helpers.

use serde_json::json;
use wiremock::matchers::{header, method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mount a successful token refresh response.
///
/// Matches requests with valid-master-token in Authorization header.
///
/// Used together with [`mount_query_401_expired_token`] and
/// [`mount_query_success_after_refresh`] to simulate a full token-refresh cycle.
/// See [`mount_refresh_failure_master_expired`] for the master-token-also-expired case.
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
/// `times` controls how many consecutive 401 responses are returned before
/// this mock is exhausted. Use `1` for the typical scenario where a single
/// failed query triggers a token refresh.
///
/// See also: [`mount_token_refresh_success`], [`mount_query_success_after_refresh`].
pub async fn mount_query_401_expired_token(server: &MockServer, times: u64) {
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .and(header(
            "Authorization",
            "Snowflake Token=\"expired-session-token\"",
        ))
        .respond_with(ResponseTemplate::new(401).set_body_string("Session expired"))
        .up_to_n_times(times)
        .mount(server)
        .await;
}

/// Mount a successful query response for refreshed token.
///
/// See also: [`mount_query_401_expired_token`], [`mount_token_refresh_success`].
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
///
/// See [`mount_token_refresh_success`] for the success counterpart.
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
