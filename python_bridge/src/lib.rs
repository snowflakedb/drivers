#[cfg(feature = "native-arrow")]
mod arrow;

use std::sync::OnceLock;

use futures::FutureExt;
use proto_utils::{CancellableTransport, ProtoError, Transport};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes};
use pyo3_async_runtimes::tokio::future_into_py;
use sf_core::apis::database_driver_v1::{DriverProviders, WrapperPresets};
use sf_core::logging::{CallbackLayer, LogManager, LoggingConfig, NormalizedEvent};
use sf_core::perf_timing;
use sf_core::protobuf::apis::RustTransport;
use sf_core::telemetry::snowflake_exporter::SessionRegistry;
use tracing::instrument::WithSubscriber;

#[cfg(feature = "native-arrow")]
use crate::arrow::ArrowStreamIterator;

static BRIDGE: OnceLock<Bridge> = OnceLock::new();

struct Bridge {
    runtime: tokio::runtime::Runtime,
    transport: RustTransport,
    dispatch: tracing::dispatcher::Dispatch,
}

impl Bridge {
    fn new(logger_callback: Py<PyAny>) -> Self {
        let layer = CallbackLayer::new(move |event| python_log_callback(&logger_callback, event));
        let sessions = SessionRegistry::default();
        let lm = LogManager::with_app_sink(LoggingConfig::default(), layer, sessions)
            .expect("Failed to initialize logging");
        let dispatch = lm.dispatch().clone();
        let providers = DriverProviders {
            log_manager: Some(lm),
            wrapper_presets: WrapperPresets::python(),
            ..Default::default()
        };
        Self {
            runtime: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime"),
            transport: RustTransport::new_with(providers),
            dispatch,
        }
    }
}

/// Forwards a core tracing event to the Python logger callable from [`init`].
///
/// Level encoding: 0=ERROR, 1=WARN, 2=INFO, 3 or higher=DEBUG.
/// `logger_name` is empty for core-originated events (Python uses
/// `snowflake.connector._core`); set for wrapper round-trip events.
///
/// Uses `eprintln` on failure — must not use tracing here (would recurse into this sink).
fn python_log_callback(logger_callback: &Py<PyAny>, event: NormalizedEvent) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Python::attach(|py| {
            if let Err(e) = logger_callback.call(
                py,
                (
                    event.level,
                    event.message,
                    event.file,
                    event.line,
                    event.function,
                    event.logger_name,
                ),
                None,
            ) {
                eprintln!("python log callback failed: {e}");
            }
        });
    }))
    .inspect_err(|_| eprintln!("python log callback panicked"));
}

/// Initialize the core state: logging, tokio runtime, and transport.
///
/// Called by the Python connector at import time, before any API call.
/// Must be called before any other function.
///
/// Returns `(status, troubleshooting_enabled)` where:
/// - `status`: `0` = success, non-zero = failure
/// - `troubleshooting_enabled`: whether troubleshooting mode is active at init time
///
/// If already initialised, returns success with the current troubleshooting flag
/// without creating another [`Bridge`].
#[pyfunction]
fn init(_py: Python, logger_callback: Py<PyAny>) -> (u32, bool) {
    // Prevent unwinding across the FFI boundary; any panic becomes status 1.
    // Detail stays on stderr via expect messages; Python raises from the status.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let bridge = BRIDGE.get_or_init(|| Bridge::new(logger_callback));
        // Register Bridge's runtime with pyo3-async-runtimes, otherwise it lazily builds a pool
        let _ = pyo3_async_runtimes::tokio::init_with_runtime(&bridge.runtime);
        (0, bridge.transport.is_troubleshooting())
    }))
    .unwrap_or_else(|_| {
        eprintln!("Failed to initialize core");
        (1, false)
    })
}

/// Emit a wrapper-originated log event through the tracing pipeline.
///
/// Uses the same level encoding as the inbound wrapper log callback:
/// 0=ERROR, 1=WARN, 2=INFO, 3 or higher=DEBUG.
///
/// Returns `0` on success, `1` when the pipeline is uninitialised, and `2` if the body panics.
#[pyfunction]
fn log_event(
    level: u32,
    message: &str,
    file: &str,
    line: u32,
    function: &str,
    logger_name: &str,
) -> u32 {
    // Prevent unwinding across the FFI boundary; any panic becomes status 2.
    std::panic::catch_unwind(|| {
        let Some(bridge) = BRIDGE.get() else {
            return 1;
        };
        let _logging_guard = tracing::dispatcher::set_default(&bridge.dispatch);
        sf_core::wrapper_event!(
            level,
            message = message,
            file = file,
            function = function,
            line = line,
            logger_name = logger_name,
        );
        0
    })
    .unwrap_or(2)
}

/// Synchronous proto API call. Releases the GIL and blocks until complete.
///
/// Returns `(status_code, response_bytes)` where status is:
/// - `0` — success
/// - `1` — application error (response holds the error payload)
/// - `2` — transport error (including panics / missing init caught at this boundary)
#[pyfunction]
fn call_proto<'py>(
    py: Python<'py>,
    api: &str,
    method: &str,
    request: &[u8],
) -> (u32, Bound<'py, PyBytes>) {
    let api = api.to_owned();
    let method = method.to_owned();
    let request = request.to_vec();
    let (code, bytes) = py.detach(|| {
        std::panic::catch_unwind(|| {
            let Some(bridge) = BRIDGE.get() else {
                return (2, b"init was not called".to_vec());
            };
            let _logging_guard = tracing::dispatcher::set_default(&bridge.dispatch);
            match bridge
                .runtime
                .block_on(bridge.transport.handle_message(&api, &method, request))
            {
                Ok(bytes) => (0, bytes),
                Err(ProtoError::Application(e)) => (1, e),
                Err(ProtoError::Transport(e)) => (2, e.into_bytes()),
            }
        })
        .unwrap_or_else(|_| (2, b"sf_core panic in call_proto".to_vec()))
    });
    (code, PyBytes::new(py, &bytes))
}

/// Async proto API call. Returns a Python awaitable → `(status_code, response)`.
///
/// Same contract as [`call_proto`]: always returns `(u32, bytes)`, never raises
/// for protocol-level failures.
///
/// Python cancel drops the awaitable which fires the CancellationToken.
#[pyfunction]
fn call_proto_async<'py>(
    py: Python<'py>,
    api: &str,
    method: &str,
    request: &[u8],
) -> PyResult<Bound<'py, PyAny>> {
    // No bridge → return a ready future with status 2, same as the sync path
    let Some(bridge) = BRIDGE.get() else {
        return future_into_py(py, async { Ok((2, b"init was not called".to_vec())) });
    };

    let api = api.to_owned();
    let method = method.to_owned();
    let request = request.to_vec();

    let (handle, cancel_token) = bridge.transport.register();

    let dispatch = bridge.dispatch.clone();
    let join_handle = bridge.runtime.spawn(
        async move {
            std::panic::AssertUnwindSafe(
                bridge
                    .transport
                    .handle_message_cancellable(&api, &method, request, handle),
            )
            .catch_unwind()
            .await
            .unwrap_or_else(|_| Err(ProtoError::Transport("sf_core panic in async task".into())))
        }
        .with_subscriber(dispatch),
    );

    // Waiter: what Python actually awaits; dropping it fires the cancel token
    future_into_py(py, async move {
        let _cancel_guard = cancel_token.drop_guard();
        let result = join_handle
            .await
            .unwrap_or_else(|e| Err(ProtoError::Transport(format!("task join error: {e}"))));
        Ok(match result {
            Ok(bytes) => (0, bytes),
            Err(ProtoError::Application(e)) => (1, e),
            Err(ProtoError::Transport(e)) => (2, e.into_bytes()),
        })
    })
}

/// Returns True if perf_timing feature is compiled in.
#[pyfunction]
fn perf_enabled() -> bool {
    perf_timing::perf_enabled()
}

/// Read-and-reset perf counters. Returns (batch_wait_ns, chunk_download_ns, arrow_decode_ns).
#[pyfunction]
fn get_perf_data() -> (u64, u64, u64) {
    let d = perf_timing::get_perf_data();
    (
        d.core_batch_wait_ns,
        d.core_chunk_download_ns,
        d.core_arrow_decode_ns,
    )
}

/// Reset perf counters without reading.
#[pyfunction]
fn reset_perf_metrics() {
    perf_timing::reset_perf_counters();
}

/// Returns True if the ``native-arrow`` Cargo feature is compiled in.
#[pyfunction]
fn native_arrow_enabled() -> bool {
    cfg!(feature = "native-arrow")
}

#[pymodule]
fn sf_core_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init, m)?)?;
    m.add_function(wrap_pyfunction!(log_event, m)?)?;
    m.add_function(wrap_pyfunction!(call_proto, m)?)?;
    m.add_function(wrap_pyfunction!(call_proto_async, m)?)?;
    m.add_function(wrap_pyfunction!(perf_enabled, m)?)?;
    m.add_function(wrap_pyfunction!(get_perf_data, m)?)?;
    m.add_function(wrap_pyfunction!(reset_perf_metrics, m)?)?;
    m.add_function(wrap_pyfunction!(native_arrow_enabled, m)?)?;
    #[cfg(feature = "native-arrow")]
    m.add_class::<ArrowStreamIterator>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use tracing::Subscriber;
    use tracing::instrument::WithSubscriber;
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::{Context, SubscriberExt};

    struct CaptureLayer {
        messages: Arc<Mutex<Vec<String>>>,
    }

    impl<S: Subscriber> Layer<S> for CaptureLayer {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            let normalized = sf_core::logging::normalize_event(event);
            self.messages
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(normalized.message);
        }
    }

    fn hop_and_capture(use_with_subscriber: bool) -> Option<Vec<String>> {
        let messages = Arc::new(Mutex::new(Vec::new()));
        let dispatch =
            tracing::dispatcher::Dispatch::new(tracing_subscriber::registry().with(CaptureLayer {
                messages: Arc::clone(&messages),
            }));
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("test runtime");
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = async move {
            let before = std::thread::current().id();
            let _ = rx.await;
            tracing::info!("after_await_on_possibly_other_worker");
            (before, std::thread::current().id())
        };
        let join = if use_with_subscriber {
            runtime.spawn(task.with_subscriber(dispatch))
        } else {
            runtime.spawn(async move {
                let _guard = tracing::dispatcher::set_default(&dispatch);
                task.await
            })
        };
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            let _ = tx.send(());
        });
        let (before, after) = runtime.block_on(join).expect("spawned task");
        if before == after {
            return None;
        }
        Some(messages.lock().unwrap_or_else(|e| e.into_inner()).clone())
    }

    fn capture_after_hop(use_with_subscriber: bool) -> Vec<String> {
        for _ in 0..40 {
            if let Some(captured) = hop_and_capture(use_with_subscriber) {
                return captured;
            }
        }
        panic!("could not force a tokio worker hop in 40 attempts");
    }

    #[test]
    fn set_default_across_await_drops_events_after_worker_hop() {
        let captured = capture_after_hop(false);
        assert!(
            !captured
                .iter()
                .any(|m| m.contains("after_await_on_possibly_other_worker")),
            "set_default is thread-local; a worker hop must drop the event; captured = {captured:?}"
        );
    }

    #[test]
    fn with_subscriber_keeps_events_after_worker_hop() {
        let captured = capture_after_hop(true);
        assert!(
            captured
                .iter()
                .any(|m| m.contains("after_await_on_possibly_other_worker")),
            "with_subscriber must keep the event after a worker hop; captured = {captured:?}"
        );
    }
}
