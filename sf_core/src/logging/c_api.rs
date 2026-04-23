use std::sync::OnceLock;

use crate::logging;
use crate::telemetry::TelemetryInit;
use crate::telemetry::snowflake_exporter::SessionRegistry;

/// Telemetry state created during logging initialization.
/// Read by `protobuf::c_api::CApiState` to pass context to `DatabaseDriverV1`.
pub(crate) static TELEMETRY_INIT: OnceLock<TelemetryInit> = OnceLock::new();

#[unsafe(no_mangle)]
pub extern "C" fn sf_core_init_logger(callback: logging::CLogCallback) -> u32 {
    let config = logging::LoggingConfig::new(None, false, false);
    let layer = logging::CallbackLayer::new(callback);
    let sessions = SessionRegistry::default();
    match logging::init_logging(config, Some(layer), sessions.clone()) {
        Ok(provider) => {
            TELEMETRY_INIT
                .set(TelemetryInit { provider, sessions })
                .ok();
            0
        }
        Err(e) => {
            eprintln!("Failed to initialize logging: {e:?}");
            1
        }
    }
}
