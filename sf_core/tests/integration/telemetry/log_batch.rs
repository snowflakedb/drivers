//! End-to-end tests for the raw log-telemetry lane: drive the real
//! [`LogBatcher`] against a wiremock `/telemetry/send` and assert the exact
//! bytes that leave the process. Faults are injected at the wire (real HTTP
//! statuses); assertions are on observable outcomes (payload, request count).

use serde_json::json;
use sf_core::telemetry::log_batch::LogBatcher;
use wiremock::matchers::{header, header_regex, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::common::{
    SESSION_ID, decompress_gzip_json, empty_registry, make_active_session, make_registry,
};

#[tokio::test]
async fn should_post_exact_wire_body_and_auth_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .and(header_regex("Authorization", r#"^Snowflake Token=".+"$"#))
        .and(header("Content-Encoding", "gzip"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;

    let batcher = LogBatcher::new(make_registry(
        SESSION_ID,
        make_active_session(&server.uri()),
    ));
    batcher.add_log(
        SESSION_ID,
        r#"{"type":"client_time_consume_last_result","query_id":"abc","value":42}"#.to_string(),
        1700000000123,
    );
    batcher.send_log_batch(SESSION_ID).await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let body = decompress_gzip_json(&requests[0].body);
    let logs = body["logs"].as_array().expect("logs array");
    assert_eq!(logs.len(), 1);
    assert_eq!(
        logs[0]["message"]["type"],
        "client_time_consume_last_result"
    );
    assert_eq!(logs[0]["message"]["query_id"], "abc");
    assert_eq!(logs[0]["message"]["value"], 42); // JSON number, not "42"
    assert_eq!(logs[0]["timestamp"], "1700000000123");
    assert!(
        logs[0]["timestamp"].is_string(),
        "timestamp must be a JSON string"
    );
}

#[tokio::test]
async fn should_send_all_buffered_entries_in_one_post() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1) // a single POST carries the whole batch
        .mount(&server)
        .await;

    let batcher = LogBatcher::new(make_registry(
        SESSION_ID,
        make_active_session(&server.uri()),
    ));
    batcher.add_log(SESSION_ID, r#"{"type":"a"}"#.to_string(), 1700000000000);
    batcher.add_log(SESSION_ID, r#"{"type":"b"}"#.to_string(), 1700000000001);
    batcher.send_log_batch(SESSION_ID).await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let logs = decompress_gzip_json(&requests[0].body)["logs"]
        .as_array()
        .expect("logs array")
        .clone();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0]["message"]["type"], "a");
    assert_eq!(logs[1]["message"]["type"], "b");
}

#[tokio::test]
async fn should_not_post_when_session_not_registered() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0) // telemetry disabled: nothing must leave the process
        .mount(&server)
        .await;

    // Empty registry == CLIENT_TELEMETRY_ENABLED off for every session.
    let batcher = LogBatcher::new(empty_registry());
    batcher.add_log(SESSION_ID, r#"{"type":"x"}"#.to_string(), 1);
    batcher.send_log_batch(SESSION_ID).await;

    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn should_drop_batch_without_retry_on_server_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1) // exactly one attempt — a failed send is not retried
        .mount(&server)
        .await;

    let batcher = LogBatcher::new(make_registry(
        SESSION_ID,
        make_active_session(&server.uri()),
    ));
    batcher.add_log(SESSION_ID, r#"{"type":"x"}"#.to_string(), 1);
    batcher.send_log_batch(SESSION_ID).await;

    // The batch was taken before the send, so a second flush sends nothing:
    // the failed batch is dropped, never requeued.
    batcher.send_log_batch(SESSION_ID).await;
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        1,
        "failed batch must be dropped, not retried or requeued"
    );
}

#[tokio::test]
async fn flush_session_posts_remaining_entries() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;

    // Mirrors connection-release: entries buffered below threshold are flushed
    // by flush_session (the close hook).
    let batcher = LogBatcher::new(make_registry(
        SESSION_ID,
        make_active_session(&server.uri()),
    ));
    batcher.add_log(
        SESSION_ID,
        r#"{"type":"on_close"}"#.to_string(),
        1700000000000,
    );
    batcher.flush_session(SESSION_ID).await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        decompress_gzip_json(&requests[0].body)["logs"][0]["message"]["type"],
        "on_close"
    );
}
