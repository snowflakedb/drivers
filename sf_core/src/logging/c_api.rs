use crate::logging;

#[cfg(not(feature = "protobuf"))]
use crate::telemetry::snowflake_exporter::SessionRegistry;

#[unsafe(no_mangle)]
pub extern "C" fn sf_core_init_logger(callback: logging::CLogCallback) -> u32 {
    let config = logging::LoggingConfig::new(None, false, false);
    let layer = logging::CallbackLayer::new(callback);

    // When the protobuf feature is enabled the SessionRegistry is owned by
    // CApiState (protobuf::c_api) — no process-global OnceLock needed.
    // Without protobuf we create a local registry that lives only in the
    // tracing subscriber (telemetry will be a no-op without a driver).
    #[cfg(feature = "protobuf")]
    let sessions = crate::protobuf::c_api::telemetry_sessions();
    #[cfg(not(feature = "protobuf"))]
    let sessions = SessionRegistry::default();

    match logging::init_logging(config, Some(layer), sessions) {
        Ok(provider) => {
            #[cfg(feature = "protobuf")]
            crate::protobuf::c_api::set_telemetry_provider(provider);
            // Without protobuf the provider is dropped here, which is fine
            // because there is no exporter to keep alive.
            #[cfg(not(feature = "protobuf"))]
            drop(provider);
            0
        }
        Err(e) => {
            eprintln!("Failed to initialize logging: {e:?}");
            1
        }
    }
}
