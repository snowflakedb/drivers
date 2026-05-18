use std::sync::{Arc, Mutex};

use crate::telemetry::{record_api_call, record_exception};

#[derive(Clone)]
pub struct ConnectionTelemetry {
    cell: Arc<Mutex<Option<tracing::Span>>>,
}

impl ConnectionTelemetry {
    pub fn with_span(span: tracing::Span) -> Self {
        Self {
            cell: Arc::new(Mutex::new(Some(span))),
        }
    }

    /// Build a recorder whose `record_*` calls are silently dropped.
    ///
    /// Used as the default for not-yet-connected `Connection`s and as
    /// the fallback returned by
    /// [`DatabaseDriverV1::connection_telemetry`](crate::apis::database_driver_v1::DatabaseDriverV1::connection_telemetry)
    /// for unknown / disconnected handles.
    pub fn noop() -> Self {
        Self {
            cell: Arc::new(Mutex::new(None)),
        }
    }

    pub fn is_active(&self) -> bool {
        self.lock_span().is_some()
    }

    pub fn record_api_call(&self, api_method: &str) {
        if let Some(span) = self.lock_span() {
            let _g = span.enter();
            record_api_call(api_method);
        }
    }

    pub fn record_exception(&self, exception_type: &str, error_source: &str) {
        if let Some(span) = self.lock_span() {
            let _g = span.enter();
            record_exception(exception_type, error_source);
        }
    }

    pub fn close(&self) -> Option<tracing::Span> {
        self.cell.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    fn lock_span(&self) -> Option<tracing::Span> {
        self.cell.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl Default for ConnectionTelemetry {
    fn default() -> Self {
        Self::noop()
    }
}

impl std::fmt::Debug for ConnectionTelemetry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionTelemetry")
            .field("active", &self.is_active())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::info_span;

    #[test]
    fn noop_recorder_does_not_panic() {
        let rec = ConnectionTelemetry::noop();
        assert!(!rec.is_active());
        rec.record_api_call("SQLExecute");
        rec.record_exception("InvalidHandle", "client_handle");
        assert!(rec.close().is_none());
    }

    #[test]
    fn with_span_records_and_close_drops_span() {
        let rec = ConnectionTelemetry::with_span(info_span!("test"));
        assert!(rec.is_active());

        rec.record_api_call("SQLExecute");
        rec.record_exception("ConnectionError", "server");

        let span = rec.close();
        assert!(span.is_some(), "close() should return the live span");
        assert!(!rec.is_active(), "after close() the recorder is inert");
        rec.record_api_call("SQLExecute");
    }

    #[test]
    fn clone_shares_underlying_cell() {
        let a = ConnectionTelemetry::with_span(info_span!("shared"));
        let b = a.clone();
        assert!(a.is_active());
        assert!(b.is_active());

        let _ = a.close();
        assert!(!a.is_active());
        assert!(!b.is_active(), "clone observes close()");
    }
}
