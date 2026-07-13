use crate::apis::database_driver_v1::WrapperPresets;
use crate::logging;
use crate::logging::callback_layer::WRAPPER_TARGET;
use crate::logging::{LogManager, LoggingConfig};
use crate::telemetry::snowflake_exporter::SessionRegistry;
use std::ffi::{CStr, c_char};
use std::sync::OnceLock;

static LOG_DISPATCH: OnceLock<tracing::dispatcher::Dispatch> = OnceLock::new();

fn cstr_or_empty<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }
    // SAFETY: caller must provide a valid NUL-terminated C string when non-null.
    unsafe { CStr::from_ptr(ptr).to_str().unwrap_or("") }
}

/// `tracing::event!` requires a compile-time level; dispatch at runtime via match.
macro_rules! wrapper_event {
    ($level:expr, $($fields:tt)*) => {
        match $level {
            0 => tracing::event!(target: WRAPPER_TARGET, tracing::Level::ERROR, $($fields)*),
            1 => tracing::event!(target: WRAPPER_TARGET, tracing::Level::WARN, $($fields)*),
            2 => tracing::event!(target: WRAPPER_TARGET, tracing::Level::INFO, $($fields)*),
            _ => tracing::event!(target: WRAPPER_TARGET, tracing::Level::DEBUG, $($fields)*),
        }
    };
}

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
            let _ = LOG_DISPATCH.set(lm.dispatch().clone());
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

/// Emit a wrapper-originated log event through the tracing pipeline.
///
/// Uses the same level encoding as the inbound [`logging::CLogCallback`]:
/// 0=ERROR, 1=WARN, 2=INFO, 3 or higher=DEBUG.
///
/// Returns `0` on success, `1` when the pipeline is uninitialised, and `2` if the body panics.
///
/// # Safety
/// All string pointers must be valid NUL-terminated UTF-8 for the duration of
/// this call. Null pointers are treated as empty strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_log_event(
    level: u32,
    message: *const c_char,
    file: *const c_char,
    line: u32,
    function: *const c_char,
    logger_name: *const c_char,
) -> u32 {
    // Prevent unwinding across the FFI boundary; any panic becomes status 2.
    std::panic::catch_unwind(|| {
        let Some(dispatch) = LOG_DISPATCH.get() else {
            return 1;
        };
        let _guard = tracing::dispatcher::set_default(dispatch);

        let message = cstr_or_empty(message);
        let file = cstr_or_empty(file);
        let function = cstr_or_empty(function);
        let logger_name = cstr_or_empty(logger_name);

        wrapper_event!(
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
