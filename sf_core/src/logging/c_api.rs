use crate::logging;
use crate::logging::{LogManager, LoggingConfig};
use crate::telemetry::snowflake_exporter::SessionRegistry;

/// Initialise the core state: logging, tokio runtime, and transport.
///
/// Wrapper calls this at import time, before any API call.
/// Returns 0 on success, 1 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn sf_core_init(callback: logging::CLogCallback) -> u32 {
    let layer = logging::CallbackLayer::new(callback);
    let sessions = SessionRegistry::default();

    match LogManager::with_app_sink(LoggingConfig::default(), layer, sessions) {
        Ok(lm) => {
            #[cfg(feature = "protobuf")]
            crate::protobuf::c_api::init_core_state(lm);
            #[cfg(not(feature = "protobuf"))]
            drop(lm);
            0
        }
        Err(e) => {
            eprintln!("Failed to initialize core: {e:?}");
            1
        }
    }
}

/// Disable the Python-bound log callback before interpreter shutdown.
///
/// Python wrapper registers this as an `atexit` handler so it runs before
/// `Py_Finalize` tears down the interpreter. Without this, the Rust tracing
/// subscriber fires events into a dead Python callback, causing SIGABRT.
#[unsafe(no_mangle)]
pub extern "C" fn sf_core_shutdown() {
    logging::disable_callback();
}
