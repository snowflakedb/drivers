use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use futures::FutureExt;
use proto_utils::{ProtoError, Transport};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes};
use sf_core::apis::database_driver_v1::{DriverProviders, WrapperPresets};
use sf_core::logging::{CallbackLayer, LogManager, LoggingConfig};
use sf_core::perf_timing;
use sf_core::protobuf::apis::RustTransport;
use sf_core::telemetry::snowflake_exporter::SessionRegistry;
use sf_core::utils::sync::MutexRecoverExt;
use tokio_util::sync::CancellationToken;

static BRIDGE: OnceLock<Bridge> = OnceLock::new();
static PY_LOG_CALLBACK: OnceLock<Py<PyAny>> = OnceLock::new();

struct Bridge {
    runtime: tokio::runtime::Runtime,
    transport: RustTransport,
    dispatch: tracing::dispatcher::Dispatch,

    /// monotonic source for async handles returned by [`call_proto_async`].
    next_async_handle: AtomicU64,
    /// In-flight async calls keyed by handle for cooperative cancellation.
    handle_registry: Mutex<HashMap<u64, CancellationToken>>,
}

impl Bridge {
    fn new() -> Self {
        let layer = CallbackLayer::new(python_log_callback);
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
            next_async_handle: AtomicU64::new(1),
            handle_registry: Mutex::new(HashMap::new()),
        }
    }

    fn registry(&self) -> std::sync::MutexGuard<'_, HashMap<u64, CancellationToken>> {
        self.handle_registry.lock_recover()
    }
}

/// C callback registered with [`CallbackLayer`] that forwards core tracing events
/// to the Python logger callable stored by [`init`].
///
/// Level encoding matches [`sf_core::logging::CLogCallback`]:
/// 0=ERROR, 1=WARN, 2=INFO, 3 or higher=DEBUG.
///
/// `logger_name` is empty for core-originated events (e.g. Python uses
/// `snowflake.connector._core`). Wrapper round-trip events set it so the
/// wrapper can dispatch to that module logger.
///
/// # Safety
/// All C string pointers must be valid for the duration of this call.
unsafe extern "C" fn python_log_callback(
    level: u32,
    message: *const std::ffi::c_char,
    filename: *const std::ffi::c_char,
    line: u32,
    function: *const std::ffi::c_char,
    logger_name: *const std::ffi::c_char,
) -> u32 {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(callback) = PY_LOG_CALLBACK.get() else {
            return 1;
        };

        let msg = unsafe { std::ffi::CStr::from_ptr(message) }
            .to_string_lossy()
            .into_owned();
        let file = unsafe { std::ffi::CStr::from_ptr(filename) }
            .to_string_lossy()
            .into_owned();
        let func = unsafe { std::ffi::CStr::from_ptr(function) }
            .to_string_lossy()
            .into_owned();
        let name = unsafe { std::ffi::CStr::from_ptr(logger_name) }
            .to_string_lossy()
            .into_owned();

        Python::attach(|py| {
            if let Err(e) = callback.call(py, (level, msg, file, line, func, name), None) {
                eprintln!("python log callback failed: {e}");
            }
        });
        0
    }))
    .unwrap_or_else(|_| {
        eprintln!("python log callback panicked");
        1
    })
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
    let _ = PY_LOG_CALLBACK.set(logger_callback);
    // Prevent unwinding across the FFI boundary; any panic becomes status 1.
    // Detail stays on stderr via expect messages; Python raises from the status.
    std::panic::catch_unwind(|| {
        let bridge = BRIDGE.get_or_init(Bridge::new);
        (0, bridge.transport.is_troubleshooting())
    })
    .unwrap_or_else(|_| {
        eprintln!("Failed to initialize core");
        (1, false)
    })
}

/// Emit a wrapper-originated log event through the tracing pipeline.
///
/// Uses the same level encoding as the inbound [`sf_core::logging::CLogCallback`]:
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
        let _guard = tracing::dispatcher::set_default(&bridge.dispatch);
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
            let _guard = tracing::dispatcher::set_default(&bridge.dispatch);
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

/// Async proto API call. Returns immediately and invokes
/// `callback(status, response_bytes)` from a tokio worker thread when complete.
///
/// Returns a non-zero **async handle** for cancellation via [`cancel`], or `0`
/// if [`init`] has not been called (no task is spawned).
///
/// Unlike the sync variant, this does **not** block the caller, so multiple
/// requests run concurrently on the shared tokio runtime.
///
/// The callback fires exactly once (unless cancelled). It is called from a
/// tokio worker thread — the Python side must use `loop.call_soon_threadsafe`
/// to resolve a Future from within the callback.
///
/// Python equivalent of `sf_core_api_call_proto_async` from the C API.
#[pyfunction]
fn call_proto_async(
    _py: Python,
    api: &str,
    method: &str,
    request: &[u8],
    callback: Py<PyAny>,
) -> u64 {
    let Some(bridge) = BRIDGE.get() else {
        return 0;
    };

    let api = api.to_owned();
    let method = method.to_owned();
    let request = request.to_vec();

    let async_handle = bridge.next_async_handle.fetch_add(1, Ordering::Relaxed);

    let cancel_token = CancellationToken::new();
    let cancel_for_task = cancel_token.clone();
    bridge.registry().insert(async_handle, cancel_token);

    // Tokio worker threads do not inherit the caller's tracing dispatch.
    // Without this, async RPC tracing events skip file/OTLP/telemetry/CallbackLayer.
    let dispatch = bridge.dispatch.clone();
    bridge.runtime.spawn(async move {
        let _guard = tracing::dispatcher::set_default(&dispatch);
        let result: Option<(u32, Vec<u8>)> = tokio::select! {
            biased;
            _ = cancel_for_task.cancelled() => None,
            // TODO(SNOW-3675196): handle_message should accept CancellationToken and pass it down the stack,
            //       then every operation can race against this token and cancel at a proper time
            result = std::panic::AssertUnwindSafe(
                bridge.transport.handle_message(&api, &method, request)
            )
            .catch_unwind() => Some(match result {
                Ok(Ok(r)) => (0, r),
                Ok(Err(ProtoError::Application(e))) => (1, e),
                Ok(Err(ProtoError::Transport(e))) => (2, e.into_bytes()),
                Err(_) => (2, b"sf_core panic in async task".to_vec()),
            }),
        };

        bridge.registry().remove(&async_handle);

        // Cancellation is always initiated by the caller, not the Tokio runtime.
        // So, caller already raised CancelledError, and we can skip the callback.
        // Underlying work may still run until SNOW-3675196 lands.
        if let Some((status, response_bytes)) = result {
            Python::attach(|py| {
                let py_bytes = PyBytes::new(py, &response_bytes);
                if let Err(e) = callback.call(py, (status, py_bytes), None) {
                    tracing::error!("python async response callback failed: {e}");
                }
            });
        }
    });

    async_handle
}

/// Cancel the wait for an in-flight async call started by [`call_proto_async`].
///
/// Signals the call's [`CancellationToken`] so the waiter skips the Python
/// callback. Until SNOW-3675196, in-flight `handle_message` work is not aborted.
///
/// Unknown async handles (and calls before [`init`]) are silently ignored.
#[pyfunction]
fn cancel(async_handle: u64) {
    let Some(bridge) = BRIDGE.get() else {
        return;
    };
    if let Some(token) = bridge.registry().remove(&async_handle) {
        token.cancel();
    }
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

#[pymodule]
fn sf_core_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init, m)?)?;
    m.add_function(wrap_pyfunction!(log_event, m)?)?;
    m.add_function(wrap_pyfunction!(call_proto, m)?)?;
    m.add_function(wrap_pyfunction!(call_proto_async, m)?)?;
    m.add_function(wrap_pyfunction!(cancel, m)?)?;
    m.add_function(wrap_pyfunction!(perf_enabled, m)?)?;
    m.add_function(wrap_pyfunction!(get_perf_data, m)?)?;
    m.add_function(wrap_pyfunction!(reset_perf_metrics, m)?)?;
    Ok(())
}
