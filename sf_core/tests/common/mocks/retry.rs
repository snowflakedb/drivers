//! Retry scenario mock helpers.

use serde_json::json;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mount a scenario where the first poll returns 503, then succeeds.
///
/// Uses `up_to_n_times(1)` to make the 503 response only match once,
/// then subsequent requests match the success response.
///
/// **Mount order matters**: wiremock matches mocks in registration order when
/// priorities are equal. The 503 mock must be mounted first so it fires on the
/// first matching request; once exhausted, the success mock takes over.
pub async fn mount_503_then_success(server: &MockServer) {
    // First request -> 503 (only matches once)
    Mock::given(method("GET"))
        .and(path_regex(r"/queries/.*/result.*"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .up_to_n_times(1)
        .mount(server)
        .await;

    // Subsequent requests -> 200
    Mock::given(method("GET"))
        .and(path_regex(r"/queries/.*/result.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "queryId": "test-query-id",
                "rowset": [["1", "test"]]
            }
        })))
        .mount(server)
        .await;
}

/// Mount a response that always returns 401 session expired.
pub async fn mount_401_session_expired(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path_regex(r"/queries/.*/result.*"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Session expired"))
        .mount(server)
        .await;
}
