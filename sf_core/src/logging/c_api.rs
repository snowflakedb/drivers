use crate::apis::database_driver_v1::WrapperPresets;
use crate::logging;
use crate::logging::{LogManager, LoggingConfig};
use crate::telemetry::snowflake_exporter::SessionRegistry;

/// Initialise the core state: logging, tokio runtime, and transport.
///
/// Called by the Python connector at import time, before any API call.
/// Returns 0 on success, 1 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn sf_core_init(callback: logging::CLogCallback) -> u32 {
    let wrapper_presets = WrapperPresets::python();

    let layer = logging::CallbackLayer::new(callback);
    let sessions = SessionRegistry::default();

    match LogManager::with_app_sink(LoggingConfig::default(), layer, sessions) {
        Ok(lm) => {
            #[cfg(feature = "protobuf")]
            crate::protobuf::c_api::init_core_state(lm, wrapper_presets);
            #[cfg(not(feature = "protobuf"))]
            drop((lm, wrapper_presets));
            0
        }
        Err(e) => {
            eprintln!("Failed to initialize core: {e:?}");
            1
        }
    }
}
