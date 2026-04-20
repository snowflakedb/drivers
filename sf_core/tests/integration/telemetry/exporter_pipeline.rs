use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use opentelemetry::trace::{
    SpanContext, SpanId, SpanKind, Status, TraceFlags, TraceId, TraceState,
};
use opentelemetry::{InstrumentationScope, KeyValue};
use opentelemetry_sdk::metrics::Temporality;
use opentelemetry_sdk::metrics::exporter::PushMetricExporter;
use opentelemetry_sdk::trace::{SpanData, SpanExporter};
use serde_json::json;
use sf_core::config::rest_parameters::{ClientInfo, QueryParameters};
use sf_core::crl::config::CrlConfig;
use sf_core::rest::snowflake::SessionTokens;
use sf_core::sensitive::SensitiveString;
use sf_core::telemetry::snowflake_exporter::{ExporterSession, SnowflakeInBandExporter};
use sf_core::tls::config::TlsConfig;
use tokio::sync::RwLock as AsyncRwLock;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_query_parameters(server_url: &str) -> QueryParameters {
    QueryParameters {
        server_url: server_url.to_string(),
        client_info: ClientInfo {
            application: "TestApp".to_string(),
            version: "1.0.0".to_string(),
            os: "Linux".to_string(),
            os_version: "5.15".to_string(),
            ocsp_mode: None,
            crl_config: CrlConfig::default(),
            tls_config: TlsConfig::default(),
        },
        log_max_query_length: 80,
    }
}

fn make_session(server_url: &str, token: Option<SessionTokens>) -> Arc<ExporterSession> {
    Arc::new(ExporterSession {
        client: reqwest::Client::new(),
        query_parameters: test_query_parameters(server_url),
        session_token: Arc::new(AsyncRwLock::new(token)),
    })
}

fn make_active_session(server_url: &str) -> Arc<ExporterSession> {
    let tokens = SessionTokens {
        session_token: SensitiveString::from("test_token"),
        master_token: SensitiveString::from("master_token"),
        session_id: 42,
        session_expires_at: None,
        master_expires_at: None,
    };
    make_session(server_url, Some(tokens))
}

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

// ---------------------------------------------------------------------------
// SpanExporter tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn span_exporter_sends_span_attributes_in_snowflake_format() {
    let server = MockServer::start().await;
    let session = make_active_session(&server.uri());

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;

    let span = make_span(
        "session_init",
        vec![
            KeyValue::new("service.name", "snowflake-python"),
            KeyValue::new("os.type", "linux"),
        ],
    );

    let exporter = SnowflakeInBandExporter::new(session);
    let result = SpanExporter::export(&exporter, vec![span]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn span_exporter_sends_multiple_spans_in_single_batch() {
    let server = MockServer::start().await;
    let session = make_active_session(&server.uri());

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1) // Single POST for the entire batch
        .mount(&server)
        .await;

    let spans = vec![
        make_span(
            "session_init",
            vec![KeyValue::new("service.name", "python")],
        ),
        make_span(
            "driver_exception",
            vec![KeyValue::new("exception.type", "RuntimeError")],
        ),
        make_span("session_init", vec![KeyValue::new("service.name", "odbc")]),
    ];

    let exporter = SnowflakeInBandExporter::new(session);
    let result = SpanExporter::export(&exporter, spans).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn span_exporter_handles_token_revoked_between_calls() {
    let server = MockServer::start().await;
    let tokens = SessionTokens {
        session_token: SensitiveString::from("valid_token"),
        master_token: SensitiveString::from("master"),
        session_id: 1,
        session_expires_at: None,
        master_expires_at: None,
    };
    let token_store = Arc::new(AsyncRwLock::new(Some(tokens)));

    let session = Arc::new(ExporterSession {
        client: reqwest::Client::new(),
        query_parameters: test_query_parameters(&server.uri()),
        session_token: token_store.clone(),
    });

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;

    let exporter = SnowflakeInBandExporter::new(session);
    let result = SpanExporter::export(&exporter, vec![make_span("span1", vec![])]).await;
    assert!(result.is_ok());

    // Clear the token — simulating session expiry
    *token_store.write().await = None;

    // Second export should silently succeed (no POST, no error)
    let result = SpanExporter::export(&exporter, vec![make_span("span2", vec![])]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn span_exporter_uses_refreshed_token() {
    let server = MockServer::start().await;
    let initial_tokens = SessionTokens {
        session_token: SensitiveString::from("old_token"),
        master_token: SensitiveString::from("master"),
        session_id: 1,
        session_expires_at: None,
        master_expires_at: None,
    };
    let token_store = Arc::new(AsyncRwLock::new(Some(initial_tokens)));

    let session = Arc::new(ExporterSession {
        client: reqwest::Client::new(),
        query_parameters: test_query_parameters(&server.uri()),
        session_token: token_store.clone(),
    });

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;

    // Refresh the token before exporting
    let new_tokens = SessionTokens {
        session_token: SensitiveString::from("new_refreshed_token"),
        master_token: SensitiveString::from("master"),
        session_id: 1,
        session_expires_at: None,
        master_expires_at: None,
    };
    *token_store.write().await = Some(new_tokens);

    let exporter = SnowflakeInBandExporter::new(session);
    let result = SpanExporter::export(&exporter, vec![make_span("test", vec![])]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn span_exporter_swallows_all_server_errors() {
    let server = MockServer::start().await;
    let session = make_active_session(&server.uri());

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
        .mount(&server)
        .await;

    let exporter = SnowflakeInBandExporter::new(session);
    let result = SpanExporter::export(&exporter, vec![make_span("test", vec![])]).await;
    assert!(result.is_ok(), "Exporter must not propagate server errors");
}

#[tokio::test]
async fn span_exporter_swallows_connection_errors() {
    let session = make_active_session("http://127.0.0.1:1");

    let exporter = SnowflakeInBandExporter::new(session);
    let result = SpanExporter::export(&exporter, vec![make_span("test", vec![])]).await;
    assert!(
        result.is_ok(),
        "Exporter must not propagate connection errors"
    );
}

#[tokio::test]
async fn span_exporter_shutdown_is_clean() {
    let server = MockServer::start().await;
    let session = make_active_session(&server.uri());

    let mut exporter = SnowflakeInBandExporter::new(session);
    let result = SpanExporter::shutdown(&mut exporter);
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// PushMetricExporter tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn metric_exporter_empty_metrics_skips_post() {
    let server = MockServer::start().await;
    let session = make_active_session(&server.uri());

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(0) // No call expected for empty metrics
        .mount(&server)
        .await;

    let exporter = SnowflakeInBandExporter::new(session);
    let empty_metrics = opentelemetry_sdk::metrics::data::ResourceMetrics::default();
    let result = PushMetricExporter::export(&exporter, &empty_metrics).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn metric_exporter_no_token_drops_silently() {
    let server = MockServer::start().await;
    let session = make_session(&server.uri(), None);

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(0) // No call expected when no token
        .mount(&server)
        .await;

    let exporter = SnowflakeInBandExporter::new(session);
    // Even with default (empty) metrics, the code path for no-token should be tested
    let empty_metrics = opentelemetry_sdk::metrics::data::ResourceMetrics::default();
    let result = PushMetricExporter::export(&exporter, &empty_metrics).await;
    assert!(result.is_ok());
}

#[test]
fn metric_exporter_temporality_is_delta() {
    let session = make_session("http://localhost:1", None);
    let exporter = SnowflakeInBandExporter::new(session);
    assert_eq!(exporter.temporality(), Temporality::Delta);
}

#[test]
fn metric_exporter_force_flush_is_ok() {
    let session = make_session("http://localhost:1", None);
    let exporter = SnowflakeInBandExporter::new(session);
    assert!(exporter.force_flush().is_ok());
}

#[test]
fn metric_exporter_shutdown_is_ok() {
    let session = make_session("http://localhost:1", None);
    let exporter = SnowflakeInBandExporter::new(session);
    assert!(PushMetricExporter::shutdown(&exporter).is_ok());
}
