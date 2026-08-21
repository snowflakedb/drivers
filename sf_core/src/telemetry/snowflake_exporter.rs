use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use opentelemetry::KeyValue;
use opentelemetry_sdk::error::OTelSdkResult;
use tokio::sync::RwLock as AsyncRwLock;

use crate::config::rest_parameters::QueryParameters;
use crate::log_foreign_error;
use crate::rest::snowflake::SessionTokens;
use crate::rest::snowflake::telemetry as rest;

use super::SESSION_ID_FIELD;

/// Shared registry mapping session IDs to their exporter sessions.
/// Connections register on init, deregister on release.
pub type SessionRegistry = Arc<RwLock<HashMap<i64, Arc<ExporterSession>>>>;

/// Shared session context used to POST telemetry to Snowflake's in-band
/// `/telemetry/send` endpoint. Both telemetry producers (span lane, raw-log
/// lane) look this up by session id via [`SessionRegistry`] and egress through
/// [`send_with_token`], so they share one wire path and one auth path.
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

/// Clone the session token under a short-lived read guard, then send
/// telemetry without holding the lock across the network call. Shared by both
/// telemetry lanes so both egress through one path. All errors are non-fatal —
/// telemetry must never break the user's workflow.
pub(crate) async fn send_with_token(
    session: &ExporterSession,
    payload: &serde_json::Value,
) -> OTelSdkResult {
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
        log_foreign_error!(warn, e, "Failed to export telemetry");
    }

    // Best-effort: always return Ok
    Ok(())
}

/// Extract the `snowflake.session.id` attribute value from a span's attributes.
/// Handles both I64 (from tracing i64 fields) and String representations.
pub(crate) fn extract_session_id(attrs: &[KeyValue]) -> Option<i64> {
    use opentelemetry::Value;
    attrs.iter().find_map(|kv| {
        if kv.key.as_str() == SESSION_ID_FIELD {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_session_id_handles_i64_value() {
        let attrs = vec![KeyValue::new(SESSION_ID_FIELD, 42i64)];
        assert_eq!(extract_session_id(&attrs), Some(42));
    }

    #[test]
    fn extract_session_id_handles_string_value() {
        let attrs = vec![KeyValue::new(SESSION_ID_FIELD, "99")];
        assert_eq!(extract_session_id(&attrs), Some(99));
    }

    #[test]
    fn extract_session_id_returns_none_when_missing() {
        let attrs = vec![KeyValue::new("other.attr", "value")];
        assert_eq!(extract_session_id(&attrs), None);
    }
}
