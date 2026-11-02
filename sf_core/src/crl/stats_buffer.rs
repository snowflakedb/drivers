use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::telemetry::session_telemetry::SessionTelemetry;
use crate::utils::sync::MutexRecoverExt;

/// Matches JDBC `PreSessionTelemetryClient.MAX_BUFFER_SIZE`.
const CAP: usize = 1000;

/// One `client_crl_stats` event. Absent fields are stages this attempt did not reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CrlStat {
    pub url: String,
    pub crl_bytes: Option<i64>,
    pub revoked_certificates: Option<i64>,
    pub download_time_ms: Option<i64>,
    pub parse_time_ms: Option<i64>,
    timestamp_ms: i64,
}

enum State {
    Buffering {
        pending: Vec<CrlStat>,
    },
    Attached {
        telemetry_id: u64,
        telemetry: SessionTelemetry,
    },
    Discarded,
}

/// One connection's `client_crl_stats` events. [`super::ConnectionCrl`] clones
/// share this buffer. A discarded buffer drops every later record.
#[derive(Clone)]
pub(crate) struct CrlStatsBuffer {
    state: Arc<Mutex<State>>,
}

impl std::fmt::Debug for CrlStatsBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &*self.state.lock_recover() {
            State::Buffering { pending } => {
                write!(f, "CrlStatsBuffer::Buffering({})", pending.len())
            }
            State::Attached { telemetry_id, .. } => {
                write!(f, "CrlStatsBuffer::Attached({telemetry_id})")
            }
            State::Discarded => f.write_str("CrlStatsBuffer::Discarded"),
        }
    }
}

impl CrlStatsBuffer {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State::Buffering {
                pending: Vec::new(),
            })),
        }
    }

    pub(crate) fn record(
        &self,
        url: &str,
        crl_bytes: Option<i64>,
        revoked_certificates: Option<i64>,
        download_time_ms: Option<i64>,
        parse_time_ms: Option<i64>,
    ) {
        let stat = CrlStat {
            url: telemetry_url(url),
            crl_bytes,
            revoked_certificates,
            download_time_ms,
            parse_time_ms,
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
        };
        let mut state = self.state.lock_recover();
        match &mut *state {
            State::Buffering { pending } => {
                if pending.len() < CAP {
                    pending.push(stat);
                }
            }
            State::Attached {
                telemetry_id,
                telemetry,
            } => {
                let (telemetry_id, telemetry) = (*telemetry_id, telemetry.clone());
                drop(state);
                push(&telemetry, telemetry_id, &stat);
            }
            State::Discarded => {}
        }
    }

    /// Attaches this connection's session and sends parked events there.
    /// A later call does not retarget. A discarded buffer sends nothing.
    pub(crate) fn flush_to(&self, telemetry: &SessionTelemetry, telemetry_id: u64) {
        let pending = {
            let mut state = self.state.lock_recover();
            let State::Buffering { pending } = &mut *state else {
                return;
            };
            let pending = std::mem::take(pending);
            *state = State::Attached {
                telemetry_id,
                telemetry: telemetry.clone(),
            };
            pending
        };
        for stat in pending {
            push(telemetry, telemetry_id, &stat);
        }
    }

    /// Drops parked events and ignores later records. Used when this
    /// connection's server left `CLIENT_TELEMETRY_ENABLED` off.
    pub(crate) fn discard(&self) {
        *self.state.lock_recover() = State::Discarded;
    }

    #[cfg(test)]
    pub(crate) fn pending(&self) -> Vec<CrlStat> {
        match &*self.state.lock_recover() {
            State::Buffering { pending } => pending.clone(),
            State::Attached { .. } | State::Discarded => Vec::new(),
        }
    }
}

fn telemetry_url(url: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(url) else {
        return crate::logging::sanitize_config_url_for_log(url).into_owned();
    };
    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);
    parsed.set_query(None);
    parsed.set_fragment(None);
    parsed.into()
}

fn push(telemetry: &SessionTelemetry, telemetry_id: u64, stat: &CrlStat) {
    let mut message = serde_json::json!({
        "type": "client_crl_stats",
        "crl_url": stat.url,
    });
    if let Some(crl_bytes) = stat.crl_bytes {
        message["crl_bytes"] = serde_json::json!(crl_bytes);
    }
    if let Some(download_time_ms) = stat.download_time_ms {
        message["crl_download_time_ms"] = serde_json::json!(download_time_ms);
    }
    if let Some(revoked_certificates) = stat.revoked_certificates {
        message["crl_revoked_certificates"] = serde_json::json!(revoked_certificates);
    }
    if let Some(parse_time_ms) = stat.parse_time_ms {
        message["crl_parse_time_ms"] = serde_json::json!(parse_time_ms);
    }
    telemetry.add_log(telemetry_id, message.to_string(), stat.timestamp_ms);
}

#[cfg(test)]
pub(crate) fn disconnected_session_telemetry(telemetry_id: u64) -> SessionTelemetry {
    use crate::config::rest_parameters::QueryParameters;
    use crate::config::rest_parameters::test_fixtures::test_client_info;
    use crate::rest::snowflake::SessionTokens;
    use crate::sensitive::SensitiveString;
    use crate::telemetry::snowflake_exporter::ExporterSession;
    use std::collections::HashMap;
    use std::sync::RwLock;
    use tokio::sync::RwLock as AsyncRwLock;

    let tokens = SessionTokens {
        session_token: SensitiveString::from("test_token"),
        master_token: SensitiveString::from("master_token"),
        session_id: Some(1),
        session_expires_at: None,
        master_expires_at: None,
        master_validity: None,
    };
    let session = Arc::new(ExporterSession {
        client: reqwest::Client::new(),
        query_parameters: QueryParameters {
            server_url: "http://127.0.0.1:1".to_string(),
            client_info: test_client_info(),
            log_max_query_length: 80,
            log_query_text: false,
            log_query_parameters: false,
            include_retry_reason: false,
            external_session_id: None,
            service_name: None,
        },
        session_token: Arc::new(AsyncRwLock::new(Some(tokens))),
    });
    let mut map = HashMap::new();
    map.insert(telemetry_id, session);
    SessionTelemetry::new(Arc::new(RwLock::new(map)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record_sample(buffer: &CrlStatsBuffer, url: &str) {
        buffer.record(url, Some(123), Some(1), Some(100), Some(25));
    }

    #[test]
    fn should_buffer_crl_stats_until_flush() {
        let buffer = CrlStatsBuffer::new();
        let url = "https://snowflake.com/ca.crl";
        record_sample(&buffer, url);
        assert_eq!(buffer.pending().len(), 1);

        let telemetry = disconnected_session_telemetry(1);
        buffer.flush_to(&telemetry, 1);
        assert!(buffer.pending().is_empty());
        let message = flushed_message(&telemetry, 1);
        assert_eq!(message["type"], "client_crl_stats");
        assert_eq!(message["crl_url"], url);
        assert_eq!(message["crl_bytes"], 123);
        assert_eq!(message["crl_revoked_certificates"], 1);
        assert_eq!(message["crl_download_time_ms"], 100);
        assert_eq!(message["crl_parse_time_ms"], 25);
        buffer.flush_to(&telemetry, 1);
        assert_eq!(telemetry.buffer_len(1), 1);
    }

    #[test]
    fn should_drop_crl_stats_past_the_cap() {
        let buffer = CrlStatsBuffer::new();
        for i in 0..(CAP + 1) {
            record_sample(&buffer, &format!("https://snowflake.com/{i}.crl"));
        }
        assert_eq!(buffer.pending().len(), CAP);
    }

    #[test]
    fn should_send_later_crl_stats_to_the_attached_session() {
        let buffer = CrlStatsBuffer::new();
        let telemetry = disconnected_session_telemetry(7);
        buffer.flush_to(&telemetry, 7);
        let url = "https://snowflake.com/ca.crl";
        record_sample(&buffer, url);
        assert!(buffer.pending().is_empty());
        assert_eq!(flushed_message(&telemetry, 7)["crl_url"], url);
    }

    #[test]
    fn should_discard_crl_stats_instead_of_sending() {
        let buffer = CrlStatsBuffer::new();
        record_sample(&buffer, "https://snowflake.com/ca.crl");
        buffer.discard();
        record_sample(&buffer, "https://snowflake.com/later.crl");
        let telemetry = disconnected_session_telemetry(1);
        buffer.flush_to(&telemetry, 1);
        assert!(buffer.pending().is_empty());
        assert_eq!(telemetry.buffer_len(1), 0);
    }

    #[test]
    fn should_omit_crl_stat_fields_the_attempt_did_not_reach() {
        let buffer = CrlStatsBuffer::new();
        buffer.record(
            "https://snowflake.com/bad.crl",
            Some(40),
            None,
            Some(12),
            None,
        );
        let telemetry = disconnected_session_telemetry(1);
        buffer.flush_to(&telemetry, 1);
        let message = flushed_message(&telemetry, 1);
        assert_eq!(message["crl_url"], "https://snowflake.com/bad.crl");
        assert_eq!(message["crl_bytes"], 40);
        assert_eq!(message["crl_download_time_ms"], 12);
        assert!(message.get("crl_parse_time_ms").is_none());
        assert!(message.get("crl_revoked_certificates").is_none());
    }

    #[test]
    fn should_send_only_scheme_host_and_path_of_the_crl_url() {
        let buffer = CrlStatsBuffer::new();
        record_sample(
            &buffer,
            "https://user:secret@crl.example.com:8443/ca.crl?token=abc#frag",
        );
        let telemetry = disconnected_session_telemetry(1);
        buffer.flush_to(&telemetry, 1);
        assert_eq!(
            flushed_message(&telemetry, 1)["crl_url"],
            "https://crl.example.com:8443/ca.crl"
        );
    }

    fn flushed_message(telemetry: &SessionTelemetry, telemetry_id: u64) -> serde_json::Value {
        let messages = telemetry.raw_log_messages(telemetry_id);
        let [message] = messages.as_slice() else {
            panic!("expected one flushed client_crl_stats log, got {messages:?}");
        };
        serde_json::from_str(message).unwrap_or_else(|err| {
            panic!("flushed client_crl_stats log is not json: {err}; body={message}")
        })
    }
}
