//! Per-session batching for caller-produced in-band telemetry.
//!
//! This is the raw-log lane beside the OTel span lane: wrappers forward
//! arbitrary telemetry entries (e.g. Snowpark's) that core does not own the
//! shape of, so they cannot flow through the OTel attribute model. Entries are
//! buffered per Snowflake session and flushed to `/telemetry/send` — reusing the
//! span lane's [`SessionRegistry`], [`ExporterSession`], and egress path
//! ([`send_with_token`]) so both lanes share one wire contract and one auth path.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;

use super::serialization::{log_entry, logs_payload};
use super::snowflake_exporter::{SessionRegistry, send_with_token};

/// Buffered entries per session before an automatic flush. Matches the legacy
/// drivers' `DEFAULT_FORCE_FLUSH_SIZE`.
const LOG_FLUSH_THRESHOLD: usize = 100;

/// Bounds the awaited flush path (connection release) so a slow `/telemetry/send`
/// cannot stall `connection_close`. Mirrors the span lane's `FLUSH_TIMEOUT`.
const FLUSH_TIMEOUT: Duration = Duration::from_secs(5);

/// One caller-produced telemetry entry. `message_json` is the caller's `message`
/// already JSON-encoded by the wrapper; it is not inspected until serialization.
struct RawLogEntry {
    message_json: String,
    timestamp_ms: i64,
}

/// Buffers raw log-telemetry entries per session and flushes them to
/// `/telemetry/send`. Cloneable: every clone shares the same buffers and
/// session registry (both `Arc`).
#[derive(Clone)]
pub struct LogBatcher {
    buffers: Arc<Mutex<HashMap<i64, Vec<RawLogEntry>>>>,
    sessions: SessionRegistry,
}

impl LogBatcher {
    /// Create a batcher sharing `sessions` with the span exporter. Membership in
    /// that registry is the `CLIENT_TELEMETRY_ENABLED` gate: sessions are only
    /// registered when telemetry is enabled, so gating adds on membership needs
    /// no second flag.
    pub fn new(sessions: SessionRegistry) -> Self {
        Self {
            buffers: Arc::new(Mutex::new(HashMap::new())),
            sessions,
        }
    }

    /// Buffer one entry for `session_id`. No network I/O. Drops silently when the
    /// session is not registered (telemetry disabled / login incomplete). When the
    /// buffer reaches [`LOG_FLUSH_THRESHOLD`] the batch is drained synchronously and
    /// its POST is spawned best-effort.
    pub fn add_log(&self, session_id: i64, message_json: String, timestamp_ms: i64) {
        {
            let sessions = self.sessions.read().unwrap_or_else(|e| e.into_inner());
            if !sessions.contains_key(&session_id) {
                return;
            }
        }
        let overflow = {
            let mut bufs = self.buffers.lock().unwrap_or_else(|e| e.into_inner());
            let buf = bufs.entry(session_id).or_default();
            buf.push(RawLogEntry {
                message_json,
                timestamp_ms,
            });
            if buf.len() >= LOG_FLUSH_THRESHOLD {
                std::mem::take(buf)
            } else {
                Vec::new()
            }
        };
        if !overflow.is_empty() {
            self.spawn_send(session_id, overflow);
        }
    }

    /// Drain and POST the buffered batch for `session_id`. Empty batch → no HTTP.
    /// The batch is taken before the network call, so a failed send drops it (no
    /// retry) — telemetry is best-effort and must never surface to the caller.
    pub async fn send_log_batch(&self, session_id: i64) {
        let entries = self.take(session_id);
        self.send_entries(session_id, entries).await;
    }

    /// Connection-release hook: flush this session's batch, bounded by
    /// [`FLUSH_TIMEOUT`] so a hung endpoint cannot stall `connection_close`. On
    /// timeout the in-flight batch is dropped (already taken from the buffer).
    pub async fn flush_session(&self, session_id: i64) {
        if tokio::time::timeout(FLUSH_TIMEOUT, self.send_log_batch(session_id))
            .await
            .is_err()
        {
            tracing::debug!(session_id, "log telemetry flush timed out; continuing");
        }
    }

    async fn send_entries(&self, session_id: i64, entries: Vec<RawLogEntry>) {
        if entries.is_empty() {
            return;
        }
        let session = {
            let sessions = self.sessions.read().unwrap_or_else(|e| e.into_inner());
            sessions.get(&session_id).cloned()
        };
        let Some(session) = session else {
            tracing::debug!(
                session_id,
                "No registered session for log telemetry; dropping batch"
            );
            return;
        };
        let payload = raw_entries_to_payload(&entries);
        let _ = send_with_token(session.as_ref(), &payload).await;
    }

    fn take(&self, session_id: i64) -> Vec<RawLogEntry> {
        let mut bufs = self.buffers.lock().unwrap_or_else(|e| e.into_inner());
        bufs.remove(&session_id).unwrap_or_default()
    }

    /// Fire-and-forget the POST of an already-drained batch. No-op outside a tokio
    /// runtime (the batch is dropped) — matches the span lane's `do_export_spawn`.
    fn spawn_send(&self, session_id: i64, entries: Vec<RawLogEntry>) {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let this = self.clone();
            handle.spawn(async move {
                this.send_entries(session_id, entries).await;
            });
        }
    }
}

/// Build the `/telemetry/send` body from raw entries. Each `message_json` is
/// parsed back to a JSON value so it embeds as a nested object; entries whose
/// `message_json` is not valid JSON are dropped (a malformed entry must never
/// sink the whole batch). Uses the shared [`log_entry`]/[`logs_payload`] helpers
/// so the wire shape matches the span lane byte-for-byte.
fn raw_entries_to_payload(entries: &[RawLogEntry]) -> Value {
    let logs: Vec<Value> = entries
        .iter()
        .filter_map(|e| match serde_json::from_str::<Value>(&e.message_json) {
            Ok(message) => Some(log_entry(message, e.timestamp_ms)),
            Err(err) => {
                tracing::debug!(error = %err, "Dropping log telemetry entry with malformed message JSON");
                None
            }
        })
        .collect();
    logs_payload(logs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rest_parameters::QueryParameters;
    use crate::rest::snowflake::SessionTokens;
    use crate::sensitive::SensitiveString;
    use std::sync::RwLock;
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

    fn buffer_len(batcher: &LogBatcher, session_id: i64) -> usize {
        batcher
            .buffers
            .lock()
            .unwrap()
            .get(&session_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    #[test]
    fn should_buffer_entry_for_registered_session() {
        let batcher = LogBatcher::new(registry_with(1));
        batcher.add_log(1, r#"{"type":"ct"}"#.to_string(), 1700000000000);
        assert_eq!(buffer_len(&batcher, 1), 1);
    }

    #[test]
    fn should_drop_entry_for_unregistered_session() {
        // Empty registry == telemetry disabled for every session.
        let batcher = LogBatcher::new(Arc::new(RwLock::new(HashMap::new())));
        batcher.add_log(1, r#"{"type":"ct"}"#.to_string(), 1700000000000);
        assert_eq!(buffer_len(&batcher, 1), 0);
    }

    #[test]
    fn should_drain_buffer_at_flush_threshold() {
        let batcher = LogBatcher::new(registry_with(1));
        for _ in 0..LOG_FLUSH_THRESHOLD {
            batcher.add_log(1, r#"{"type":"ct"}"#.to_string(), 1700000000000);
        }
        // Reaching the threshold drains the buffer synchronously (the spawned POST
        // is a no-op here — no tokio runtime — but the drain is what we assert).
        assert_eq!(buffer_len(&batcher, 1), 0, "buffer must drain at threshold");
    }

    #[tokio::test]
    async fn should_swap_and_clear_on_send_log_batch() {
        let batcher = LogBatcher::new(registry_with(1));
        batcher.add_log(1, r#"{"type":"a"}"#.to_string(), 1700000000000);
        batcher.add_log(1, r#"{"type":"b"}"#.to_string(), 1700000000001);
        assert_eq!(buffer_len(&batcher, 1), 2);

        // Send fails against the dead port, but the batch is taken first.
        batcher.send_log_batch(1).await;
        assert_eq!(buffer_len(&batcher, 1), 0, "send must swap-and-clear");
    }

    #[tokio::test]
    async fn should_be_noop_on_empty_send_log_batch() {
        let batcher = LogBatcher::new(registry_with(1));
        batcher.send_log_batch(1).await;
        assert_eq!(buffer_len(&batcher, 1), 0);
    }

    #[tokio::test]
    async fn flush_session_drains_buffer() {
        let batcher = LogBatcher::new(registry_with(7));
        batcher.add_log(7, r#"{"type":"a"}"#.to_string(), 1700000000000);
        assert_eq!(buffer_len(&batcher, 7), 1);
        batcher.flush_session(7).await;
        assert_eq!(buffer_len(&batcher, 7), 0);
    }

    #[test]
    fn payload_serializes_timestamp_as_string() {
        let entries = [RawLogEntry {
            message_json: r#"{"type":"ct"}"#.to_string(),
            timestamp_ms: 1700000000123,
        }];
        let payload = raw_entries_to_payload(&entries);
        assert_eq!(payload["logs"][0]["timestamp"], "1700000000123");
    }

    #[test]
    fn payload_preserves_nested_message() {
        let entries = [RawLogEntry {
            message_json: r#"{"type":"ct","value":42,"nested":{"k":true}}"#.to_string(),
            timestamp_ms: 1700000000000,
        }];
        let payload = raw_entries_to_payload(&entries);
        let msg = &payload["logs"][0]["message"];
        assert_eq!(msg["type"], "ct");
        assert_eq!(msg["value"], 42); // JSON number, not "42"
        assert_eq!(msg["nested"]["k"], true);
    }

    #[test]
    fn payload_drops_malformed_message_json() {
        let entries = [
            RawLogEntry {
                message_json: "not valid json".to_string(),
                timestamp_ms: 1700000000000,
            },
            RawLogEntry {
                message_json: r#"{"type":"ok"}"#.to_string(),
                timestamp_ms: 1700000000001,
            },
        ];
        let payload = raw_entries_to_payload(&entries);
        let logs = payload["logs"].as_array().unwrap();
        assert_eq!(logs.len(), 1, "malformed entry dropped, valid entry kept");
        assert_eq!(logs[0]["message"]["type"], "ok");
    }
}
