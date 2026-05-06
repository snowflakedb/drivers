use std::ffi::CStr;

use crate::apis::database_driver_v1::WrapperPresets;
use crate::logging;
use crate::logging::{LogManager, LoggingConfig};
use crate::telemetry::snowflake_exporter::SessionRegistry;

/// Initialise the core state: logging, tokio runtime, and transport.
///
/// Wrapper calls this at import time, before any API call.
/// `wrapper_name` is a null-terminated C string identifying the calling
/// wrapper (e.g. `"python"`, `"odbc"`). It selects behavioural presets
/// baked into the driver for that wrapper.
///
/// Returns 0 on success, 1 on failure.
///
/// # Safety
/// `wrapper_name` must be either null or point to a valid null-terminated
/// C string that remains alive for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_init(
    callback: logging::CLogCallback,
    wrapper_name: *const std::ffi::c_char,
) -> u32 {
    let name = if wrapper_name.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(wrapper_name) }
            .to_str()
            .unwrap_or("")
    };
    let wrapper_presets = WrapperPresets::for_wrapper(name);

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
