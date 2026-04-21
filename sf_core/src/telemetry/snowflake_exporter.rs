use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::RwLock;
use std::time::Duration;

use opentelemetry::KeyValue;
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::trace::{SpanData, SpanExporter};
use tokio::sync::RwLock as AsyncRwLock;

use crate::config::rest_parameters::QueryParameters;
use crate::rest::snowflake::SessionTokens;
use crate::rest::snowflake::telemetry as rest;

use super::serialization;

/// Span attribute key used to route telemetry to the correct session.
const SESSION_ID_ATTR: &str = "snowflake.session.id";

/// Shared registry mapping session IDs to their exporter sessions.
/// Connections register on init, deregister on release.
pub type SessionRegistry = Arc<RwLock<HashMap<i64, Arc<ExporterSession>>>>;

/// Process-global session registry shared between the tracing exporter layer
/// (installed at logging init) and `DatabaseDriverV1` (which populates it
/// when connections open). The exporter is a no-op while the registry is empty.
static GLOBAL_SESSION_REGISTRY: LazyLock<SessionRegistry> = LazyLock::new(SessionRegistry::default);

/// Returns the process-global session registry.
pub fn global_session_registry() -> SessionRegistry {
    GLOBAL_SESSION_REGISTRY.clone()
}

/// Shared session context that the exporter uses to POST telemetry.
pub struct ExporterSession {
    pub client: reqwest::Client,
    pub query_parameters: QueryParameters,
    pub session_token: Arc<AsyncRwLock<Option<SessionTokens>>>,
}

impl std::fmt::Debug for ExporterSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExporterSession")
            .field("server_url", &self.query_parameters.server_url)
            .finish_non_exhaustive()
    }
}

/// Custom OTel exporter that sends telemetry to Snowflake's in-band `/telemetry/send` endpoint.
///
/// Uses a shared session registry to route spans to the correct connection based on
/// the `snowflake.session.id` span attribute. Spans without this attribute are silently
/// dropped — this is the security/privacy boundary ensuring only explicitly tagged spans
/// are sent to Snowflake.
///
/// All errors are treated as non-fatal — telemetry must never break the user's workflow.
#[derive(Debug, Clone)]
pub struct SnowflakeInBandExporter {
    sessions: SessionRegistry,
}

impl SnowflakeInBandExporter {
    pub fn new(sessions: SessionRegistry) -> Self {
        Self { sessions }
    }
}

/// Clone the session token under a short-lived read guard, then send
/// telemetry without holding the lock across the network call.
async fn send_with_token(session: &ExporterSession, payload: &serde_json::Value) -> OTelSdkResult {
    let token = {
        let guard = session.session_token.read().await;
        match guard.as_ref() {
            Some(tokens) => tokens.session_token.clone(),
            None => {
                tracing::debug!("No active session token, dropping telemetry");
                return Ok(());
            }
        }
    };

    if let Err(e) = rest::send_telemetry(
        &session.client,
        &session.query_parameters,
        token.reveal(),
        payload,
    )
    .await
    {
        tracing::warn!("Failed to export telemetry: {e}");
    }

    // Best-effort: always return Ok
    Ok(())
}

/// Extract the `snowflake.session.id` attribute value from a span's attributes.
/// Handles both I64 (from tracing i64 fields) and String representations.
fn extract_session_id(attrs: &[KeyValue]) -> Option<i64> {
    use opentelemetry::Value;
    attrs.iter().find_map(|kv| {
        if kv.key.as_str() == SESSION_ID_ATTR {
            match &kv.value {
                Value::I64(id) => Some(*id),
                Value::String(s) => s.as_str().parse::<i64>().ok(),
                _ => None,
            }
        } else {
            None
        }
    })
}

impl SpanExporter for SnowflakeInBandExporter {
    fn export(
        &self,
        batch: Vec<SpanData>,
    ) -> impl std::future::Future<Output = OTelSdkResult> + Send {
        let sessions = self.sessions.clone();
        async move {
            if batch.is_empty() {
                return Ok(());
            }

            // Group spans by session_id. Spans without the attribute are dropped.
            let mut by_session: HashMap<i64, Vec<SpanData>> = HashMap::new();
            for span in batch {
                if let Some(session_id) = extract_session_id(&span.attributes) {
                    by_session.entry(session_id).or_default().push(span);
                }
            }

            if by_session.is_empty() {
                return Ok(());
            }

            // Snapshot the sessions we need under a short-lived read lock.
            let session_map: HashMap<i64, Arc<ExporterSession>> = {
                let guard = sessions.read().unwrap_or_else(|e| e.into_inner());
                by_session
                    .keys()
                    .filter_map(|id| guard.get(id).map(|s| (*id, s.clone())))
                    .collect()
            };

            // Send each session's spans independently.
            for (session_id, spans) in &by_session {
                let Some(session) = session_map.get(session_id) else {
                    tracing::debug!(
                        session_id,
                        "No registered session for telemetry spans, dropping"
                    );
                    continue;
                };
                let payload = serialization::spans_to_snowflake_payload(spans.as_slice());
                let _ = send_with_token(session, &payload).await;
            }

            Ok(())
        }
    }

    fn shutdown_with_timeout(&mut self, _timeout: Duration) -> OTelSdkResult {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::InstrumentationScope;
    use opentelemetry::trace::{
        SpanContext, SpanId, SpanKind, Status, TraceFlags, TraceId, TraceState,
    };
    use serde_json::json;
    use std::time::UNIX_EPOCH;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_query_parameters(server_url: &str) -> QueryParameters {
        use crate::config::rest_parameters::test_fixtures::test_client_info;
        QueryParameters {
            server_url: server_url.to_string(),
            client_info: test_client_info(),
            log_max_query_length: 80,
        }
    }

    fn make_registry_with_session(
        session_id: i64,
        session: Arc<ExporterSession>,
    ) -> SessionRegistry {
        let mut map = HashMap::new();
        map.insert(session_id, session);
        Arc::new(RwLock::new(map))
    }

    fn make_exporter_session(server_url: &str) -> Arc<ExporterSession> {
        use crate::sensitive::SensitiveString;

        let tokens = SessionTokens {
            session_token: SensitiveString::from("test_token"),
            master_token: SensitiveString::from("master_token"),
            session_id: 1,
            session_expires_at: None,
            master_expires_at: None,
        };

        Arc::new(ExporterSession {
            client: reqwest::Client::new(),
            query_parameters: test_query_parameters(server_url),
            session_token: Arc::new(AsyncRwLock::new(Some(tokens))),
        })
    }

    fn make_exporter_session_no_token(server_url: &str) -> Arc<ExporterSession> {
        Arc::new(ExporterSession {
            client: reqwest::Client::new(),
            query_parameters: test_query_parameters(server_url),
            session_token: Arc::new(AsyncRwLock::new(None)),
        })
    }

    fn make_test_span(session_id: Option<i64>) -> SpanData {
        let mut attributes = vec![];
        if let Some(id) = session_id {
            attributes.push(KeyValue::new(SESSION_ID_ATTR, id.to_string()));
        }

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
            name: "test_span".into(),
            start_time: UNIX_EPOCH + std::time::Duration::from_millis(1700000000000),
            end_time: UNIX_EPOCH + std::time::Duration::from_millis(1700000001000),
            attributes,
            dropped_attributes_count: 0,
            events: Default::default(),
            links: Default::default(),
            status: Status::Ok,
            instrumentation_scope: InstrumentationScope::builder("test").build(),
        }
    }

    #[tokio::test]
    async fn span_exporter_posts_to_telemetry_endpoint() {
        let server = MockServer::start().await;
        let session = make_exporter_session(&server.uri());
        let registry = make_registry_with_session(1, session);

        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
            .expect(1)
            .mount(&server)
            .await;

        let exporter = SnowflakeInBandExporter::new(registry);
        let result = SpanExporter::export(&exporter, vec![make_test_span(Some(1))]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn span_exporter_empty_batch_skips_post() {
        let registry: SessionRegistry = Arc::new(RwLock::new(HashMap::new()));

        let exporter = SnowflakeInBandExporter::new(registry);
        let result = SpanExporter::export(&exporter, vec![]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn span_exporter_drops_spans_without_session_id() {
        let server = MockServer::start().await;
        let session = make_exporter_session(&server.uri());
        let registry = make_registry_with_session(1, session);

        // No POST expected — span has no session_id attribute
        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
            .expect(0)
            .mount(&server)
            .await;

        let exporter = SnowflakeInBandExporter::new(registry);
        let result = SpanExporter::export(&exporter, vec![make_test_span(None)]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn span_exporter_drops_spans_for_unknown_session() {
        let server = MockServer::start().await;
        let session = make_exporter_session(&server.uri());
        let registry = make_registry_with_session(1, session);

        // Span has session_id=999, but only session 1 is registered
        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
            .expect(0)
            .mount(&server)
            .await;

        let exporter = SnowflakeInBandExporter::new(registry);
        let result = SpanExporter::export(&exporter, vec![make_test_span(Some(999))]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn span_exporter_no_session_token_drops_silently() {
        let server = MockServer::start().await;
        let session = make_exporter_session_no_token(&server.uri());
        let registry = make_registry_with_session(1, session);

        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;

        let exporter = SnowflakeInBandExporter::new(registry);
        let result = SpanExporter::export(&exporter, vec![make_test_span(Some(1))]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn span_exporter_server_error_returns_ok() {
        let server = MockServer::start().await;
        let session = make_exporter_session(&server.uri());
        let registry = make_registry_with_session(1, session);

        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(500).set_body_string("error"))
            .mount(&server)
            .await;

        let exporter = SnowflakeInBandExporter::new(registry);
        let result = SpanExporter::export(&exporter, vec![make_test_span(Some(1))]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn span_exporter_routes_to_multiple_sessions() {
        let server1 = MockServer::start().await;
        let server2 = MockServer::start().await;

        let session1 = make_exporter_session(&server1.uri());
        let session2 = make_exporter_session(&server2.uri());

        let registry: SessionRegistry = Arc::new(RwLock::new(HashMap::new()));
        registry.write().unwrap().insert(1, session1);
        registry.write().unwrap().insert(2, session2);

        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
            .expect(1)
            .mount(&server1)
            .await;
        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
            .expect(1)
            .mount(&server2)
            .await;

        let exporter = SnowflakeInBandExporter::new(registry);
        let result = SpanExporter::export(
            &exporter,
            vec![make_test_span(Some(1)), make_test_span(Some(2))],
        )
        .await;
        assert!(result.is_ok());
    }
}
