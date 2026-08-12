use std::time::Duration;

use opentelemetry::Context;
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::trace::{Span, SpanData, SpanProcessor};

use super::session_telemetry::SessionTelemetry;

/// Custom [`SpanProcessor`] that feeds driver-instrumented spans into the shared
/// per-session telemetry buffer ([`SessionTelemetry`]). It is a thin adapter:
/// the OTel SDK hands it every ended span via [`Self::on_end`]; it drops
/// unsampled spans and forwards the rest to [`SessionTelemetry::add_span`], which
/// routes by the `snowflake.session.id` attribute and buffers alongside the
/// raw-log lane. No serialization or I/O happens here — the span-end hot path
/// only enqueues.
#[derive(Debug)]
pub struct SnowflakeSpanProcessor {
    telemetry: SessionTelemetry,
}

impl SnowflakeSpanProcessor {
    /// Wrap the shared [`SessionTelemetry`] as an OTel span processor. The same
    /// `SessionTelemetry` is held by `LogManager` for the raw-log lane and for
    /// the awaited connection-release flush, so both producers share one buffer.
    pub fn new(telemetry: SessionTelemetry) -> Self {
        Self { telemetry }
    }
}

impl SpanProcessor for SnowflakeSpanProcessor {
    fn on_start(&self, _span: &mut Span, _cx: &Context) {
        // Nothing to do on span start.
    }

    fn on_end(&self, span: SpanData) {
        if !span.span_context.is_sampled() {
            return;
        }
        self.telemetry.add_span(span);
    }

    /// Best-effort spawn flush on tracer-provider shutdown.
    ///
    /// The `SpanProcessor` trait requires a sync `force_flush`, so we cannot
    /// await here. The authoritative flush that guarantees the POST completes
    /// while session tokens are still alive is
    /// [`SessionTelemetry::flush_session`], called from
    /// `flush_connection_telemetry` on connection release.
    fn force_flush(&self) -> OTelSdkResult {
        self.telemetry.force_flush_all();
        Ok(())
    }

    fn shutdown_with_timeout(&self, _timeout: Duration) -> OTelSdkResult {
        self.force_flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::SESSION_ID_FIELD;
    use crate::telemetry::snowflake_exporter::SessionRegistry;
    use opentelemetry::InstrumentationScope;
    use opentelemetry::KeyValue;
    use opentelemetry::trace::{
        SpanContext, SpanId, SpanKind, Status, TraceFlags, TraceId, TraceState,
    };
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};
    use std::time::UNIX_EPOCH;

    fn empty_telemetry() -> SessionTelemetry {
        // add_span buffers regardless of registry membership (it drops at send),
        // so an empty registry is enough to assert routing/sampling here.
        let registry: SessionRegistry = Arc::new(RwLock::new(HashMap::new()));
        SessionTelemetry::new(registry)
    }

    fn test_span(session_id: Option<i64>, sampled: bool) -> SpanData {
        let mut attributes = vec![];
        if let Some(id) = session_id {
            attributes.push(KeyValue::new(SESSION_ID_FIELD, id.to_string()));
        }
        let flags = if sampled {
            TraceFlags::SAMPLED
        } else {
            TraceFlags::default()
        };
        SpanData {
            span_context: SpanContext::new(
                TraceId::from_hex("0102030405060708090a0b0c0d0e0f10").unwrap(),
                SpanId::from_hex("0102030405060708").unwrap(),
                flags,
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
    fn unsampled_span_is_ignored() {
        let telemetry = empty_telemetry();
        let processor = SnowflakeSpanProcessor::new(telemetry.clone());
        processor.on_end(test_span(Some(1), false));
        assert_eq!(telemetry.buffer_len(1), 0, "unsampled span must not buffer");
    }

    #[test]
    fn sampled_span_with_session_id_is_buffered() {
        let telemetry = empty_telemetry();
        let processor = SnowflakeSpanProcessor::new(telemetry.clone());
        processor.on_end(test_span(Some(1), true));
        assert_eq!(telemetry.buffer_len(1), 1);
    }

    #[test]
    fn sampled_span_without_session_id_is_dropped() {
        let telemetry = empty_telemetry();
        let processor = SnowflakeSpanProcessor::new(telemetry.clone());
        processor.on_end(test_span(None, true));
        assert_eq!(telemetry.buffer_len(1), 0);
    }

    #[test]
    fn force_flush_drains_all_sessions() {
        let telemetry = empty_telemetry();
        let processor = SnowflakeSpanProcessor::new(telemetry.clone());
        for _ in 0..3 {
            processor.on_end(test_span(Some(1), true));
            processor.on_end(test_span(Some(2), true));
        }
        assert_eq!(telemetry.buffer_len(1), 3);

        let result = processor.force_flush();
        assert!(result.is_ok());
        assert_eq!(
            telemetry.buffer_len(1),
            0,
            "force_flush drains every session"
        );
        assert_eq!(telemetry.buffer_len(2), 0);
    }
}
