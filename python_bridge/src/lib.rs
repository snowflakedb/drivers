use std::sync::OnceLock;

use futures::FutureExt;
use proto_utils::{ProtoError, Transport};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes};
use pyo3_async_runtimes::tokio::future_into_py;
use sf_core::apis::database_driver_v1::{DriverProviders, WrapperPresets};
use sf_core::logging::{CallbackLayer, LogManager, LoggingConfig};
use sf_core::perf_timing;
use sf_core::protobuf::apis::RustTransport;
use sf_core::telemetry::snowflake_exporter::SessionRegistry;

static BRIDGE: OnceLock<Bridge> = OnceLock::new();
static PY_LOG_CALLBACK: OnceLock<Py<PyAny>> = OnceLock::new();

struct Bridge {
    runtime: tokio::runtime::Runtime,
    transport: RustTransport,
    dispatch: tracing::dispatcher::Dispatch,
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
        }
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
        // Register Bridge's runtime with pyo3-async-runtimes, otherwise it lazily builds a pool
        let _ = pyo3_async_runtimes::tokio::init_with_runtime(&bridge.runtime);
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

    // Tokio worker threads do not inherit the caller's tracing dispatch.
    // Without this, async RPC tracing events skip file/OTLP/telemetry/CallbackLayer.
    let dispatch = bridge.dispatch.clone();
    let join_handle = bridge.runtime.spawn(async move {
        let _logging_guard = tracing::dispatcher::set_default(&dispatch);
        std::panic::AssertUnwindSafe(
            bridge
                .transport
                .handle_message_cancellable(&api, &method, request, handle),
        )
        .catch_unwind()
        .await
        .unwrap_or_else(|_| Err(ProtoError::Transport("sf_core panic in async task".into())))
    });

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

#[pymodule]
fn sf_core_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init, m)?)?;
    m.add_function(wrap_pyfunction!(log_event, m)?)?;
    m.add_function(wrap_pyfunction!(call_proto, m)?)?;
    m.add_function(wrap_pyfunction!(call_proto_async, m)?)?;
    m.add_function(wrap_pyfunction!(perf_enabled, m)?)?;
    m.add_function(wrap_pyfunction!(get_perf_data, m)?)?;
    m.add_function(wrap_pyfunction!(reset_perf_metrics, m)?)?;
    Ok(())
}
