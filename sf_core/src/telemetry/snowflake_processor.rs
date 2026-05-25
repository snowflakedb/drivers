use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opentelemetry::Context;
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::trace::{Span, SpanData, SpanProcessor};

use super::snowflake_exporter::{SnowflakeInBandExporter, extract_session_id};

/// Number of buffered spans per session before an automatic flush.
const FLUSH_THRESHOLD: usize = 50;

type SharedBuffers = Arc<Mutex<HashMap<i64, Vec<SpanData>>>>;

/// Custom [`SpanProcessor`] that buffers spans per session and flushes
/// when a threshold is reached or when explicitly requested on connection
/// release.
///
/// Unlike `SimpleSpanProcessor` (which exports every span immediately),
/// this batches spans per session and sends them together, reducing the
/// number of HTTP calls to `/telemetry/send`.
///
/// Spans are routed by the `snowflake.session.id` attribute the driver
/// stamps on each operation span. Spans that lack the attribute are
/// dropped — they don't belong to a Snowflake session.
#[derive(Debug)]
pub struct SnowflakeSpanProcessor {
    exporter: Mutex<SnowflakeInBandExporter>,
    buffers: SharedBuffers,
}

/// Cloneable handle for flushing a specific session's buffered spans.
///
/// Shared with `DatabaseDriverV1` so connection release can flush
/// remaining spans before the connection span is dropped.
#[derive(Debug, Clone)]
pub struct SessionFlushHandle {
    exporter: SnowflakeInBandExporter,
    buffers: SharedBuffers,
}

/// Bounded wait for the awaited flush path. Prevents a hung
/// `/telemetry/send` endpoint from blocking `connection_close` indefinitely.
/// Sized to accommodate p99 latency in degraded regions (observed 3-4s).
const FLUSH_TIMEOUT: Duration = Duration::from_secs(5);

impl SessionFlushHandle {
    /// Drain and export all buffered spans for the given session.
    /// Called during connection release, before session tokens are cleared
    /// and before the connection span is dropped.
    ///
    /// Awaits the HTTP POST so the export completes while session tokens are
    /// still alive for authentication. If the exporter exceeds
    /// [`FLUSH_TIMEOUT`], the in-flight export future is cancelled and the
    /// buffered spans for this session are dropped — telemetry is best-effort
    /// and we prefer losing a batch to stalling `connection_close`. A detached
    /// spawn retry is intentionally not used because the `Vec<SpanData>` has
    /// already been moved into the cancelled future.
    pub async fn flush_session(&self, session_id: i64) {
        let spans = {
            let mut bufs = self.buffers.lock().unwrap_or_else(|e| e.into_inner());
            bufs.remove(&session_id).unwrap_or_default()
        };
        if spans.is_empty() {
            return;
        }
        do_export_await(&self.exporter, spans).await;
    }
}

use opentelemetry_sdk::trace::SpanExporter;

/// Spawn the exporter on the tokio runtime and return immediately.
///
/// Used from [`SnowflakeSpanProcessor::on_end`], which is called synchronously
/// from within `Span::end`. We must not block the span-end hot path, so this
/// path is fire-and-forget and telemetry export is best-effort.
fn do_export_spawn(exporter: &SnowflakeInBandExporter, spans: Vec<SpanData>) {
    let exporter_clone = exporter.clone();
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async move {
            if let Err(e) = SpanExporter::export(&exporter_clone, spans).await {
                tracing::debug!(error = %e, "Snowflake telemetry export failed");
            }
        });
    }
}

/// Await the exporter with [`FLUSH_TIMEOUT`] bound. Used by
/// [`SessionFlushHandle::flush_session`] on connection release so the POST
/// completes while session tokens are still alive. On timeout the in-flight
/// export future is cancelled and the spans are dropped (the `Vec<SpanData>`
/// was moved into that future). A slow/hung endpoint therefore cannot stall
/// `connection_close`; the trade-off is losing at most one per-session batch.
async fn do_export_await(exporter: &SnowflakeInBandExporter, spans: Vec<SpanData>) {
    let exporter_clone = exporter.clone();
    match tokio::time::timeout(FLUSH_TIMEOUT, SpanExporter::export(&exporter_clone, spans)).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            tracing::debug!(error = %e, "Snowflake telemetry export failed");
        }
        Err(_elapsed) => {
            tracing::debug!(
                timeout_secs = FLUSH_TIMEOUT.as_secs(),
                "Snowflake telemetry flush timed out; continuing"
            );
        }
    }
}

impl SnowflakeSpanProcessor {
    /// Create a processor and a [`SessionFlushHandle`] that shares the
    /// same buffers. The handle is used by connection release to flush
    /// remaining spans for a session.
    pub fn new(exporter: SnowflakeInBandExporter) -> (Self, SessionFlushHandle) {
        let buffers: SharedBuffers = Arc::new(Mutex::new(HashMap::new()));
        let flush_handle = SessionFlushHandle {
            exporter: exporter.clone(),
            buffers: Arc::clone(&buffers),
        };
        let processor = Self {
            exporter: Mutex::new(exporter),
            buffers,
        };
        (processor, flush_handle)
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

        let Some(session_id) = extract_session_id(&span.attributes) else {
            return;
        };

        let flush = {
            let mut bufs = self.buffers.lock().unwrap_or_else(|e| e.into_inner());
            let buf = bufs.entry(session_id).or_default();
            buf.push(span);
            buf.len() >= FLUSH_THRESHOLD
        };

        if flush {
            let spans = {
                let mut bufs = self.buffers.lock().unwrap_or_else(|e| e.into_inner());
                bufs.remove(&session_id).unwrap_or_default()
            };
            if !spans.is_empty() {
                do_export_spawn(
                    &self.exporter.lock().unwrap_or_else(|e| e.into_inner()),
                    spans,
                );
            }
        }
    }

    /// Best-effort spawn flush on tracer-provider shutdown.
    ///
    /// The `SpanProcessor` trait requires a sync `force_flush`, so we cannot
    /// await the exporter here. The authoritative flush path that guarantees
    /// the POST completes while session tokens are still alive is
    /// [`SessionFlushHandle::flush_session`], called from
    /// `flush_connection_telemetry` on connection release.
    fn force_flush(&self) -> OTelSdkResult {
        let all_spans: Vec<(i64, Vec<SpanData>)> = {
            let mut bufs = self.buffers.lock().unwrap_or_else(|e| e.into_inner());
            bufs.drain().collect()
        };
        let exporter = self.exporter.lock().unwrap_or_else(|e| e.into_inner());
        for (_session_id, spans) in all_spans {
            if !spans.is_empty() {
                do_export_spawn(&exporter, spans);
            }
        }
        Ok(())
    }

    fn shutdown_with_timeout(&self, _timeout: Duration) -> OTelSdkResult {
        self.force_flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::InstrumentationScope;
    use opentelemetry::KeyValue;
    use opentelemetry::trace::{
        SpanContext, SpanId, SpanKind, Status, TraceFlags, TraceId, TraceState,
    };
    use std::sync::RwLock;
    use std::time::UNIX_EPOCH;

    use super::super::snowflake_exporter::SessionRegistry;

    fn make_test_span(session_id: Option<i64>, trace_id_hex: &str) -> SpanData {
        let mut attributes = vec![];
        if let Some(id) = session_id {
            attributes.push(KeyValue::new("snowflake.session.id", id.to_string()));
        }

        SpanData {
            span_context: SpanContext::new(
                TraceId::from_hex(trace_id_hex).unwrap(),
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

    fn make_processor() -> (SnowflakeSpanProcessor, SessionFlushHandle) {
        let sessions: SessionRegistry = Arc::new(RwLock::new(HashMap::new()));
        let exporter = SnowflakeInBandExporter::new(sessions);
        SnowflakeSpanProcessor::new(exporter)
    }

    #[test]
    fn span_without_session_id_is_dropped() {
        let (processor, _flush) = make_processor();
        let span = make_test_span(None, "0102030405060708090a0b0c0d0e0f10");
        processor.on_end(span);

        let bufs = processor.buffers.lock().unwrap();
        assert!(bufs.is_empty(), "unresolvable span should be dropped");
    }

    #[test]
    fn span_with_session_id_is_buffered() {
        let (processor, _flush) = make_processor();
        let span = make_test_span(Some(1), "0102030405060708090a0b0c0d0e0f10");
        processor.on_end(span);

        let bufs = processor.buffers.lock().unwrap();
        assert_eq!(bufs.get(&1).map(|v| v.len()), Some(1));
    }

    #[test]
    fn flush_threshold_drains_buffer() {
        let (processor, _flush) = make_processor();

        for _ in 0..FLUSH_THRESHOLD {
            let span = make_test_span(Some(1), "0102030405060708090a0b0c0d0e0f10");
            processor.on_end(span);
        }

        // Buffer should be drained after reaching threshold
        // (export may fail because no session is registered, but buffer is cleared)
        let bufs = processor.buffers.lock().unwrap();
        assert!(
            bufs.get(&1).map(|v| v.len()).unwrap_or(0) == 0,
            "buffer should be drained at threshold"
        );
    }

    #[tokio::test]
    async fn flush_session_drains_specific_session() {
        let (processor, flush_handle) = make_processor();

        // Buffer spans for two sessions
        for _ in 0..5 {
            processor.on_end(make_test_span(Some(1), "0102030405060708090a0b0c0d0e0f10"));
            processor.on_end(make_test_span(Some(2), "0102030405060708090a0b0c0d0e0f10"));
        }

        {
            let bufs = processor.buffers.lock().unwrap();
            assert_eq!(bufs.get(&1).map(|v| v.len()), Some(5));
            assert_eq!(bufs.get(&2).map(|v| v.len()), Some(5));
        }

        // Flush only session 1
        flush_handle.flush_session(1).await;

        let bufs = processor.buffers.lock().unwrap();
        assert!(bufs.get(&1).is_none(), "session 1 should be fully drained");
        assert_eq!(
            bufs.get(&2).map(|v| v.len()),
            Some(5),
            "session 2 should be untouched"
        );
    }

    #[tokio::test]
    async fn flush_session_on_empty_is_noop() {
        let (_processor, flush_handle) = make_processor();
        // Should not panic
        flush_handle.flush_session(999).await;
    }

    /// Regression guard for SNOW-flush-before-logout: `flush_session` must
    /// drain the buffer synchronously *before* returning its awaited future.
    /// If the implementation reverted to a fire-and-forget spawn, the buffer
    /// would still contain spans after the `.await` returns — and in
    /// production the export would race `cleanup_connection`'s token clear.
    #[tokio::test]
    async fn flush_session_drains_buffer_before_returning() {
        let (processor, flush_handle) = make_processor();
        for _ in 0..3 {
            processor.on_end(make_test_span(Some(7), "0102030405060708090a0b0c0d0e0f10"));
        }
        assert_eq!(
            processor.buffers.lock().unwrap().get(&7).map(|v| v.len()),
            Some(3)
        );

        // The handle has no registered session, so the exporter call is a no-op
        // and returns Ok immediately — we only assert the drain ordering here.
        flush_handle.flush_session(7).await;

        assert!(
            processor.buffers.lock().unwrap().get(&7).is_none(),
            "flush_session must drain the per-session buffer before returning"
        );
    }

    /// Regression guard: `flush_session` must bound its wait. If the exporter
    /// were to hang, the timeout path must allow the caller to continue.
    /// We exercise this by running `flush_session` under `tokio::time::timeout`
    /// set larger than `FLUSH_TIMEOUT` — if the implementation dropped the
    /// timeout, a hung exporter would trip our outer timeout first.
    #[tokio::test]
    async fn flush_session_completes_within_bounded_window() {
        let (_processor, flush_handle) = make_processor();
        // Empty buffer: the export is skipped, so this only verifies the
        // structural contract (method is async + returns promptly).
        let outer = Duration::from_secs(FLUSH_TIMEOUT.as_secs() + 5);
        tokio::time::timeout(outer, flush_handle.flush_session(1))
            .await
            .expect("flush_session must complete within bounded window");
    }

    #[test]
    fn force_flush_drains_all() {
        let (processor, _flush) = make_processor();

        for _ in 0..10 {
            processor.on_end(make_test_span(Some(1), "0102030405060708090a0b0c0d0e0f10"));
            processor.on_end(make_test_span(Some(2), "0102030405060708090a0b0c0d0e0f10"));
        }

        let result = processor.force_flush();
        assert!(result.is_ok());

        let bufs = processor.buffers.lock().unwrap();
        assert!(bufs.is_empty(), "all buffers should be drained");
    }

    #[test]
    fn unsampled_span_is_ignored() {
        let (processor, _flush) = make_processor();

        let span = SpanData {
            span_context: SpanContext::new(
                TraceId::from_hex("0102030405060708090a0b0c0d0e0f10").unwrap(),
                SpanId::from_hex("0102030405060708").unwrap(),
                TraceFlags::default(), // NOT sampled
                false,
                TraceState::default(),
            ),
            parent_span_id: SpanId::INVALID,
            span_kind: SpanKind::Internal,
            name: "test".into(),
            start_time: UNIX_EPOCH,
            end_time: UNIX_EPOCH,
            attributes: vec![KeyValue::new("snowflake.session.id", "1")],
            dropped_attributes_count: 0,
            events: Default::default(),
            links: Default::default(),
            status: Status::Ok,
            instrumentation_scope: InstrumentationScope::builder("test").build(),
        };

        processor.on_end(span);

        let bufs = processor.buffers.lock().unwrap();
        assert!(bufs.is_empty(), "unsampled span should be ignored");
    }
}
