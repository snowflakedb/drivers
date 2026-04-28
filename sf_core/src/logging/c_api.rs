use crate::logging;
use crate::logging::{LogManager, LoggingConfig};
use crate::telemetry::snowflake_exporter::SessionRegistry;

/// Initialise logging with a C callback as the application sink.
///
/// Wrapper calls this at import time, before any API call.
/// Returns 0 on success, 1 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn sf_core_init_logger(callback: logging::CLogCallback) -> u32 {
    let layer = logging::CallbackLayer::new(callback);
    let sessions = SessionRegistry::default();

    match LogManager::with_app_sink(LoggingConfig::default(), layer, sessions) {
        Ok(lm) => {
            #[cfg(feature = "protobuf")]
            crate::protobuf::c_api::set_log_manager(lm);
            #[cfg(not(feature = "protobuf"))]
            drop(lm);
            0
        }
        Err(e) => {
            eprintln!("Failed to initialize logging: {e:?}");
            1
        }
    }
}
