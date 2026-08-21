use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use opentelemetry::trace::{
    SpanContext, SpanId, SpanKind, Status, TraceFlags, TraceId, TraceState,
};
use opentelemetry::{InstrumentationScope, KeyValue};
use opentelemetry_sdk::trace::SpanData;
use serde_json::json;
use sf_core::rest::snowflake::SessionTokens;
use sf_core::sensitive::SensitiveString;
use sf_core::telemetry::session_telemetry::SessionTelemetry;
use sf_core::telemetry::snowflake_exporter::ExporterSession;
use tokio::sync::RwLock as AsyncRwLock;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::common::{
    decompress_gzip_json, make_active_session, make_registry, test_query_parameters,
};

const SESSION_ID: i64 = 42;

fn make_span(name: &'static str, attributes: Vec<KeyValue>) -> SpanData {
    SpanData {
        span_context: SpanContext::new(
            TraceId::from_hex("0102030405060708090a0b0c0d0e0f10").unwrap(),
            SpanId::from_hex("0102030405060708").unwrap(),
            TraceFlags::default(),
            false,
            TraceState::default(),
        ),
        parent_span_id: SpanId::INVALID,
        span_kind: SpanKind::Internal,
        name: name.into(),
        start_time: UNIX_EPOCH + Duration::from_millis(1700000000000),
        end_time: UNIX_EPOCH + Duration::from_millis(1700000001000),
        attributes,
        dropped_attributes_count: 0,
        events: Default::default(),
        links: Default::default(),
        status: Status::Ok,
        instrumentation_scope: InstrumentationScope::builder("test").build(),
    }
}

/// Helper: create a span with the `snowflake.session.id` attribute set.
fn make_tagged_span(name: &'static str, extra_attrs: Vec<KeyValue>) -> SpanData {
    let mut attrs = vec![KeyValue::new(
        "snowflake.session.id",
        SESSION_ID.to_string(),
    )];
    attrs.extend(extra_attrs);
    make_span(name, attrs)
}

// ---------------------------------------------------------------------------
// SessionTelemetry span-lane egress tests
//
// The span lane is driven by pushing spans (as the OTel processor does) and
// flushing the session — the connection-release path. Both producers share one
// buffer and one POST per session, so these assertions also guard the raw-log
// lane's egress.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn telemetry_sends_span_attributes_in_snowflake_format() {
    let server = MockServer::start().await;
    let telemetry = SessionTelemetry::new(make_registry(
        SESSION_ID,
        make_active_session(&server.uri()),
    ));

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;

    telemetry.add_span(make_tagged_span(
        "session_init",
        vec![
            KeyValue::new("service.name", "snowflake-python"),
            KeyValue::new("os.type", "linux"),
        ],
    ));
    telemetry.flush_session(SESSION_ID).await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let body = decompress_gzip_json(&requests[0].body);
    assert_eq!(body["logs"][0]["message"]["type"], "session_init");
    assert_eq!(
        body["logs"][0]["message"]["service.name"],
        "snowflake-python"
    );
}

#[tokio::test]
async fn telemetry_sends_multiple_spans_in_single_post() {
    let server = MockServer::start().await;
    let telemetry = SessionTelemetry::new(make_registry(
        SESSION_ID,
        make_active_session(&server.uri()),
    ));

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1) // Single POST for the whole session's batch
        .mount(&server)
        .await;

    telemetry.add_span(make_tagged_span(
        "session_init",
        vec![KeyValue::new("service.name", "python")],
    ));
    telemetry.add_span(make_tagged_span(
        "driver_exception",
        vec![KeyValue::new("exception.type", "RuntimeError")],
    ));
    telemetry.add_span(make_tagged_span(
        "session_init",
        vec![KeyValue::new("service.name", "odbc")],
    ));
    telemetry.flush_session(SESSION_ID).await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let body = decompress_gzip_json(&requests[0].body);
    assert_eq!(
        body["logs"].as_array().unwrap().len(),
        3,
        "all three spans in one body"
    );
}

#[tokio::test]
async fn telemetry_sends_span_and_raw_log_in_one_post() {
    // The headline of the unification: a session's span-lane and raw-log-lane
    // entries share one buffer and leave in a single POST.
    let server = MockServer::start().await;
    let telemetry = SessionTelemetry::new(make_registry(
        SESSION_ID,
        make_active_session(&server.uri()),
    ));

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;

    telemetry.add_span(make_tagged_span(
        "session_init",
        vec![KeyValue::new("service.name", "python")],
    ));
    telemetry.add_log(
        SESSION_ID,
        r#"{"type":"snowpark_function_usage","data":{"func_name":"collect"}}"#.to_string(),
        1700000000123,
    );
    telemetry.flush_session(SESSION_ID).await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "span + raw-log ride one POST");
    let body = decompress_gzip_json(&requests[0].body);
    let logs = body["logs"].as_array().unwrap();
    assert_eq!(logs.len(), 2);
    let types: Vec<&str> = logs
        .iter()
        .filter_map(|l| l["message"]["type"].as_str())
        .collect();
    assert!(types.contains(&"session_init"), "span entry present");
    assert!(
        types.contains(&"snowpark_function_usage"),
        "raw-log entry present"
    );
}

#[tokio::test]
async fn telemetry_handles_token_revoked_between_flushes() {
    let server = MockServer::start().await;
    let tokens = SessionTokens {
        session_token: SensitiveString::from("valid_token"),
        master_token: SensitiveString::from("master"),
        session_id: SESSION_ID,
        session_expires_at: None,
        master_expires_at: None,
        master_validity: None,
    };
    let token_store = Arc::new(AsyncRwLock::new(Some(tokens)));

    let session = Arc::new(ExporterSession {
        client: reqwest::Client::new(),
        query_parameters: test_query_parameters(&server.uri()),
        session_token: token_store.clone(),
    });
    let telemetry = SessionTelemetry::new(make_registry(SESSION_ID, session));

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;

    telemetry.add_span(make_tagged_span("span1", vec![]));
    telemetry.flush_session(SESSION_ID).await;

    // Clear the token — simulating session expiry.
    *token_store.write().await = None;

    // Second flush drops silently (no POST, no panic).
    telemetry.add_span(make_tagged_span("span2", vec![]));
    telemetry.flush_session(SESSION_ID).await;
}

#[tokio::test]
async fn telemetry_uses_refreshed_token() {
    let server = MockServer::start().await;
    let initial_tokens = SessionTokens {
        session_token: SensitiveString::from("old_token"),
        master_token: SensitiveString::from("master"),
        session_id: SESSION_ID,
        session_expires_at: None,
        master_expires_at: None,
        master_validity: None,
    };
    let token_store = Arc::new(AsyncRwLock::new(Some(initial_tokens)));

    let session = Arc::new(ExporterSession {
        client: reqwest::Client::new(),
        query_parameters: test_query_parameters(&server.uri()),
        session_token: token_store.clone(),
    });
    let telemetry = SessionTelemetry::new(make_registry(SESSION_ID, session));

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .and(wiremock::matchers::header_regex(
            "Authorization",
            r#"Snowflake Token="new_refreshed_token""#,
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;

    // Refresh the token before flushing.
    *token_store.write().await = Some(SessionTokens {
        session_token: SensitiveString::from("new_refreshed_token"),
        master_token: SensitiveString::from("master"),
        session_id: SESSION_ID,
        session_expires_at: None,
        master_expires_at: None,
        master_validity: None,
    });

    telemetry.add_span(make_tagged_span("test", vec![]));
    telemetry.flush_session(SESSION_ID).await;
}

#[tokio::test]
async fn telemetry_swallows_server_errors() {
    let server = MockServer::start().await;
    let telemetry = SessionTelemetry::new(make_registry(
        SESSION_ID,
        make_active_session(&server.uri()),
    ));

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
        .mount(&server)
        .await;

    telemetry.add_span(make_tagged_span("test", vec![]));
    // Must not panic or propagate — telemetry is best-effort.
    telemetry.flush_session(SESSION_ID).await;
}

#[tokio::test]
async fn telemetry_swallows_connection_errors() {
    // Dead port: the send fails fast and is swallowed.
    let telemetry = SessionTelemetry::new(make_registry(
        SESSION_ID,
        make_active_session("http://127.0.0.1:1"),
    ));
    telemetry.add_span(make_tagged_span("test", vec![]));
    telemetry.flush_session(SESSION_ID).await;
}

#[tokio::test]
async fn telemetry_drops_span_without_session_id() {
    let server = MockServer::start().await;
    let telemetry = SessionTelemetry::new(make_registry(
        SESSION_ID,
        make_active_session(&server.uri()),
    ));

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(0) // Untagged span never buffers, so nothing to flush.
        .mount(&server)
        .await;

    telemetry.add_span(make_span("test", vec![]));
    telemetry.flush_session(SESSION_ID).await;
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn telemetry_drops_spans_after_session_deregistered() {
    let server = MockServer::start().await;
    let registry = make_registry(SESSION_ID, make_active_session(&server.uri()));
    let telemetry = SessionTelemetry::new(registry.clone());

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1) // Only the first flush should POST.
        .mount(&server)
        .await;

    telemetry.add_span(make_tagged_span("before_close", vec![]));
    telemetry.flush_session(SESSION_ID).await;

    // Simulate connection close: remove the session from the registry.
    registry.write().unwrap().remove(&SESSION_ID);

    // Second flush drops silently — the session is gone.
    telemetry.add_span(make_tagged_span("after_close", vec![]));
    telemetry.flush_session(SESSION_ID).await;
}

#[tokio::test]
async fn telemetry_routes_each_session_to_its_own_endpoint() {
    let server1 = MockServer::start().await;
    let server2 = MockServer::start().await;

    // One registry with two sessions pointing at different servers.
    let registry = make_registry(1, make_active_session(&server1.uri()));
    registry
        .write()
        .unwrap()
        .insert(2, make_active_session(&server2.uri()));
    let telemetry = SessionTelemetry::new(registry);

    for server in [&server1, &server2] {
        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
            .expect(1)
            .mount(server)
            .await;
    }

    telemetry.add_span(make_span(
        "s1",
        vec![KeyValue::new("snowflake.session.id", "1")],
    ));
    telemetry.add_span(make_span(
        "s2",
        vec![KeyValue::new("snowflake.session.id", "2")],
    ));
    telemetry.flush_session(1).await;
    telemetry.flush_session(2).await;

    assert_eq!(server1.received_requests().await.unwrap().len(), 1);
    assert_eq!(server2.received_requests().await.unwrap().len(), 1);
}
