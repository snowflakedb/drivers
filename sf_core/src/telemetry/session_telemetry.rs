//! Unified per-session telemetry buffering for both producers.
//!
//! The driver feeds `/telemetry/send` from two producers that differ in only one
//! step — how a buffered item becomes a log entry:
//!
//! - the **span lane** ([`super::snowflake_processor`]): a driver-instrumented
//!   `SpanData` → [`span_to_log_entries`] (message built from scalar attributes);
//! - the **raw-log lane** (caller JSON forwarded by wrappers, e.g. Snowpark): a
//!   `message_json` string → parsed and embedded as an object.
//!
//! Everything after that is identical — both build the same `{message, timestamp}`
//! shape via [`log_entry`]/[`logs_payload`] and POST through the same
//! [`send_with_token`] to the same endpoint. So both share one per-session buffer
//! here, keyed by `snowflake.session.id`. Conversion to JSON is deferred to send
//! time, so the span-end hot path only enqueues the `SpanData`.

use opentelemetry_sdk::trace::SpanData;
use serde_json::{Value, json};

use super::serialization::{log_entry, logs_payload, span_to_log_entries};
use super::session_batch::{SessionBuffer, flush_bounded, spawn_best_effort};
use super::snowflake_exporter::{SessionRegistry, extract_session_id, send_with_token};
use crate::utils::sync::RwLockRecoverExt;

/// Buffered records per session before an automatic flush. One record is one
/// span or one raw-log entry (a span may expand to several log entries at send).
/// Matches the legacy drivers' `DEFAULT_FORCE_FLUSH_SIZE`.
const FLUSH_THRESHOLD: usize = 100;

/// One caller-produced telemetry entry. `message_json` is the caller's `message`
/// already JSON-encoded by the wrapper; it is parsed only at send time.
#[derive(Debug)]
pub(crate) struct RawLogEntry {
    pub(crate) message_json: String,
    pub(crate) timestamp_ms: i64,
}

impl RawLogEntry {
    /// Parse the caller's `message_json` and wrap it as a single log entry.
    ///
    /// The `message` is always a JSON **object**. Consumers read payload fields
    /// as `message:data:<field>` against a VARIANT, and against a scalar or an
    /// array that yields NULL rather than an error — so a caller sending the
    /// wrong shape would produce rows that look present but carry no readable
    /// payload, with nothing reported on either side.
    ///
    /// Anything that is not an object is therefore nested under `message`
    /// rather than dropped, whether it parsed as valid JSON of the wrong shape
    /// or failed to parse at all: one bad entry must never sink the whole
    /// batch, but it also shouldn't silently vanish.
    fn into_log_entries(self) -> Vec<Value> {
        let parsed = serde_json::from_str::<Value>(&self.message_json);
        let message = match parsed {
            Ok(Value::Object(fields)) => Value::Object(fields),
            Ok(other) => {
                tracing::debug!("Raw log message is not a JSON object; nesting it under `message`");
                json!({ "message": other })
            }
            Err(_) => {
                tracing::debug!("Raw log message is not valid JSON; nesting it under `message`");
                json!({ "message": self.message_json })
            }
        };
        vec![log_entry(message, self.timestamp_ms)]
    }
}

/// One buffered telemetry record. Both variants serialize to the same
/// `{message, timestamp}` log-entry shape — only *how* differs, which is the
/// whole reason the two producers can share one buffer.
#[derive(Debug)]
pub(crate) enum TelemetryRecord {
    /// A driver-instrumented OTel span. Boxed so the enum stays pointer-sized: a
    /// `RawLog` slot doesn't carry `SpanData`'s footprint, and buffer moves/reallocs
    /// stay cheap. `SpanData` already heap-allocates its attributes, so the box is
    /// negligible next to span creation.
    Span(Box<SpanData>),
    /// A caller-produced raw log entry (e.g. Snowpark).
    RawLog(RawLogEntry),
}

impl TelemetryRecord {
    fn into_log_entries(self) -> Vec<Value> {
        match self {
            TelemetryRecord::Span(span) => span_to_log_entries(&span),
            TelemetryRecord::RawLog(entry) => entry.into_log_entries(),
        }
    }
}

/// Per-session telemetry buffer shared by the span lane and the raw-log lane.
/// Owns the one [`SessionBuffer`] both producers push into and the
/// [`SessionRegistry`] used to look up per-session auth at send time. Cloneable;
/// every clone shares the same buffer and registry.
#[derive(Debug, Clone)]
pub struct SessionTelemetry {
    buffer: SessionBuffer<TelemetryRecord>,
    sessions: SessionRegistry,
}

impl SessionTelemetry {
    /// Create a telemetry buffer over `sessions`. Registry membership is the
    /// `CLIENT_TELEMETRY_ENABLED` gate — sessions are registered only when
    /// telemetry is enabled, so gating rides on membership with no second flag.
    pub fn new(sessions: SessionRegistry) -> Self {
        Self {
            buffer: SessionBuffer::new(FLUSH_THRESHOLD),
            sessions,
        }
    }

    /// The shared session registry. Connections register on init and deregister
    /// on release.
    pub(crate) fn sessions(&self) -> &SessionRegistry {
        &self.sessions
    }

    /// Buffer one caller-produced entry. Drops silently when the session is not
    /// registered (telemetry disabled / login incomplete). On threshold overflow
    /// the drained batch's POST is spawned best-effort.
    pub fn add_log(&self, session_id: i64, message_json: String, timestamp_ms: i64) {
        {
            let sessions = self.sessions.read_recover();
            if !sessions.contains_key(&session_id) {
                return;
            }
        }
        let record = TelemetryRecord::RawLog(RawLogEntry {
            message_json,
            timestamp_ms,
        });
        if let Some(batch) = self.buffer.push(session_id, record) {
            self.spawn_send(session_id, batch);
        }
    }

    /// Buffer one driver-instrumented span, routed by its `snowflake.session.id`
    /// attribute. A span without it doesn't belong to a Snowflake session and is
    /// dropped. On threshold overflow the drained batch's POST is spawned. No
    /// JSON work here — serialization is deferred to [`Self::send_records`], so
    /// the span-end hot path stays cheap.
    pub fn add_span(&self, span: SpanData) {
        let Some(session_id) = extract_session_id(&span.attributes) else {
            return;
        };
        if let Some(batch) = self
            .buffer
            .push(session_id, TelemetryRecord::Span(Box::new(span)))
        {
            self.spawn_send(session_id, batch);
        }
    }

    /// Connection-release hook: flush this session's buffer, bounded by
    /// `FLUSH_TIMEOUT` so a hung endpoint cannot stall `connection_close`. The
    /// buffer is drained synchronously before the awaited send, so it is empty by
    /// the time this returns even on timeout.
    pub async fn flush_session(&self, session_id: i64) {
        let records = self.buffer.take(session_id);
        flush_bounded(session_id, self.send_records(session_id, records)).await;
    }

    /// Drain every session best-effort. Backs the span processor's `force_flush`
    /// (tracer-provider shutdown); the authoritative, awaited flush is
    /// [`Self::flush_session`] on connection release.
    pub(crate) fn force_flush_all(&self) {
        for (session_id, records) in self.buffer.drain_all() {
            if !records.is_empty() {
                self.spawn_send(session_id, records);
            }
        }
    }

    /// Serialize a session's records into one `/telemetry/send` body and POST it.
    /// Empty batch (or all entries dropped as malformed) → no HTTP. Records are
    /// converted here, at send — not at enqueue. The batch is dropped if the
    /// session is unregistered or has no token: best-effort, no retry.
    async fn send_records(&self, session_id: i64, records: Vec<TelemetryRecord>) {
        if records.is_empty() {
            return;
        }
        let session = {
            let sessions = self.sessions.read_recover();
            sessions.get(&session_id).cloned()
        };
        let Some(session) = session else {
            tracing::debug!(
                session_id,
                "No registered session for telemetry; dropping batch"
            );
            return;
        };
        let logs: Vec<Value> = records
            .into_iter()
            .flat_map(TelemetryRecord::into_log_entries)
            .collect();
        if logs.is_empty() {
            return;
        }
        tracing::debug!(session_id, entries = logs.len(), "flushing telemetry batch");
        let payload = logs_payload(logs);
        let _ = send_with_token(session.as_ref(), &payload).await;
    }

    /// Fire-and-forget the POST of an already-drained batch (no-op outside a
    /// tokio runtime). Threshold overflow / shutdown must not block the caller.
    fn spawn_send(&self, session_id: i64, records: Vec<TelemetryRecord>) {
        let this = self.clone();
        spawn_best_effort(async move {
            this.send_records(session_id, records).await;
        });
    }

    #[cfg(test)]
    pub(crate) fn buffer_len(&self, session_id: i64) -> usize {
        self.buffer.len(session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rest_parameters::QueryParameters;
    use crate::rest::snowflake::SessionTokens;
    use crate::sensitive::SensitiveString;
    use crate::telemetry::SESSION_ID_FIELD;
    use opentelemetry::InstrumentationScope;
    use opentelemetry::KeyValue;
    use opentelemetry::trace::{
        SpanContext, SpanId, SpanKind, Status, TraceFlags, TraceId, TraceState,
    };
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};
    use std::time::UNIX_EPOCH;
    use tokio::sync::RwLock as AsyncRwLock;

    use super::super::snowflake_exporter::ExporterSession;

    fn exporter_session(server_url: &str) -> Arc<ExporterSession> {
        use crate::config::rest_parameters::test_fixtures::test_client_info;
        let tokens = SessionTokens {
            session_token: SensitiveString::from("test_token"),
            master_token: SensitiveString::from("master_token"),
            session_id: 1,
            session_expires_at: None,
            master_expires_at: None,
            master_validity: None,
        };
        Arc::new(ExporterSession {
            client: reqwest::Client::new(),
            query_parameters: QueryParameters {
                server_url: server_url.to_string(),
                client_info: test_client_info(),
                log_max_query_length: 80,
                log_query_text: false,
                log_query_parameters: false,
            },
            session_token: Arc::new(AsyncRwLock::new(Some(tokens))),
        })
    }

    fn registry_with(session_id: i64) -> SessionRegistry {
        let mut map = HashMap::new();
        // A dead port: any send attempt fails fast (connection refused) and is
        // swallowed. These tests assert buffering/drain, never a successful POST.
        map.insert(session_id, exporter_session("http://127.0.0.1:1"));
        Arc::new(RwLock::new(map))
    }

    fn test_span(session_id: Option<i64>) -> SpanData {
        let mut attributes = vec![];
        if let Some(id) = session_id {
            attributes.push(KeyValue::new(SESSION_ID_FIELD, id.to_string()));
        }
        SpanData {
            span_context: SpanContext::new(
                TraceId::from_hex("0102030405060708090a0b0c0d0e0f10").unwrap(),
                SpanId::from_hex("0102030405060708").unwrap(),
                TraceFlags::SAMPLED,
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

    #[test]
    fn should_buffer_raw_log_for_registered_session() {
        let t = SessionTelemetry::new(registry_with(1));
        t.add_log(1, r#"{"type":"ct"}"#.to_string(), 1700000000000);
        assert_eq!(t.buffer_len(1), 1);
    }

    #[test]
    fn should_drop_raw_log_for_unregistered_session() {
        let t = SessionTelemetry::new(Arc::new(RwLock::new(HashMap::new())));
        t.add_log(1, r#"{"type":"ct"}"#.to_string(), 1700000000000);
        assert_eq!(t.buffer_len(1), 0);
    }

    #[test]
    fn should_buffer_span_with_session_id() {
        let t = SessionTelemetry::new(registry_with(1));
        t.add_span(test_span(Some(1)));
        assert_eq!(t.buffer_len(1), 1);
    }

    #[test]
    fn should_drop_span_without_session_id() {
        let t = SessionTelemetry::new(registry_with(1));
        t.add_span(test_span(None));
        assert_eq!(t.buffer_len(1), 0);
    }

    #[test]
    fn should_buffer_span_and_raw_log_in_one_session_buffer() {
        let t = SessionTelemetry::new(registry_with(1));
        t.add_span(test_span(Some(1)));
        t.add_log(1, r#"{"type":"ct"}"#.to_string(), 1700000000000);
        assert_eq!(t.buffer_len(1), 2, "span + raw-log share one buffer");
    }

    #[test]
    fn should_drain_buffer_at_flush_threshold() {
        let t = SessionTelemetry::new(registry_with(1));
        for _ in 0..FLUSH_THRESHOLD {
            t.add_log(1, r#"{"type":"ct"}"#.to_string(), 1700000000000);
        }
        // Reaching the threshold drains the buffer synchronously (the spawned
        // POST is a no-op here — no tokio runtime — but the drain is what we assert).
        assert_eq!(t.buffer_len(1), 0, "buffer must drain at threshold");
    }

    #[tokio::test]
    async fn should_swap_and_clear_on_flush_session() {
        let t = SessionTelemetry::new(registry_with(1));
        t.add_log(1, r#"{"type":"a"}"#.to_string(), 1700000000000);
        t.add_span(test_span(Some(1)));
        assert_eq!(t.buffer_len(1), 2);

        // Send fails against the dead port, but the batch is taken first.
        t.flush_session(1).await;
        assert_eq!(t.buffer_len(1), 0, "flush must swap-and-clear");
    }

    #[tokio::test]
    async fn should_be_noop_on_empty_flush_session() {
        let t = SessionTelemetry::new(registry_with(1));
        t.flush_session(1).await;
        assert_eq!(t.buffer_len(1), 0);
    }

    #[test]
    fn raw_log_entry_serializes_timestamp_as_string_and_preserves_nesting() {
        let entry = RawLogEntry {
            message_json: r#"{"type":"ct","value":42,"nested":{"k":true}}"#.to_string(),
            timestamp_ms: 1700000000123,
        };
        let logs = entry.into_log_entries();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0]["timestamp"], "1700000000123");
        let msg = &logs[0]["message"];
        assert_eq!(msg["type"], "ct");
        assert_eq!(msg["value"], 42); // JSON number, not "42"
        assert_eq!(msg["nested"]["k"], true);
    }

    #[test]
    fn raw_log_entry_nests_malformed_json_under_message() {
        let entry = RawLogEntry {
            message_json: "not valid json".to_string(),
            timestamp_ms: 1700000000000,
        };
        let logs = entry.into_log_entries();
        assert_eq!(logs.len(), 1);
        // Retained rather than dropped, but nested so `message` stays an object.
        assert!(logs[0]["message"].is_object());
        assert_eq!(logs[0]["message"]["message"], "not valid json");
        assert_eq!(logs[0]["timestamp"], "1700000000000");
    }

    /// Valid JSON of the wrong shape is the case that used to slip through:
    /// it parsed, so it was embedded as-is, and `message:data:<field>` then
    /// read NULL downstream instead of failing.
    #[test]
    fn raw_log_entry_nests_non_object_json_under_message() {
        for (input, expected) in [
            ("42", json!(42)),
            ("[1,2,3]", json!([1, 2, 3])),
            (r#""text""#, json!("text")),
            ("true", json!(true)),
            ("null", json!(null)),
        ] {
            let entry = RawLogEntry {
                message_json: input.to_string(),
                timestamp_ms: 1700000000123,
            };
            let logs = entry.into_log_entries();
            assert_eq!(logs.len(), 1);
            let msg = &logs[0]["message"];
            assert!(
                msg.is_object(),
                "message must be an object for input {input}"
            );
            assert_eq!(
                msg["message"], expected,
                "payload preserved for input {input}"
            );
            assert_eq!(logs[0]["timestamp"], "1700000000123");
        }
    }

    /// An object is passed through untouched — no extra nesting level, so the
    /// well-behaved caller's field paths are unchanged.
    #[test]
    fn raw_log_entry_does_not_nest_an_object_message() {
        let entry = RawLogEntry {
            message_json: r#"{"type":"ct","data":{"k":1}}"#.to_string(),
            timestamp_ms: 1700000000123,
        };
        let logs = entry.into_log_entries();
        let msg = &logs[0]["message"];
        assert_eq!(msg["type"], "ct");
        assert_eq!(msg["data"]["k"], 1);
        assert!(msg.get("message").is_none());
    }
}
