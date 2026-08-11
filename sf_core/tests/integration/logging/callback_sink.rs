use std::ffi::{CStr, c_char};
use std::sync::Mutex;

use sf_core::logging::{CLogCallback, CallbackLayer, LogManager, LoggingConfig};
use sf_core::telemetry::snowflake_exporter::SessionRegistry;

struct CapturedEvent {
    level: u32,
    message: String,
    logger_name: String,
}

static CAPTURED: Mutex<Vec<CapturedEvent>> = Mutex::new(Vec::new());

/// Serializes tests in this module: they share the `CAPTURED` static, so
/// clearing/reading it must not interleave with another test's writes. Held
/// for the whole test body; the callback locks `CAPTURED` (a different mutex),
/// so there is no deadlock.
static TEST_GUARD: Mutex<()> = Mutex::new(());

unsafe extern "C" fn test_callback(
    level: u32,
    message: *const c_char,
    _filename: *const c_char,
    _line: u32,
    _function: *const c_char,
    logger_name: *const c_char,
) -> u32 {
    let msg = unsafe { CStr::from_ptr(message) }
        .to_string_lossy()
        .into_owned();
    let name = if logger_name.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(logger_name) }
            .to_string_lossy()
            .into_owned()
    };
    CAPTURED.lock().unwrap().push(CapturedEvent {
        level,
        message: msg,
        logger_name: name,
    });
    0
}

/// Verify that events emitted through tracing are delivered to a C callback
/// registered via `CallbackLayer`.
#[test]
fn callback_layer_delivers_events() {
    let _serial = TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
    CAPTURED.lock().unwrap().clear();

    let config = LoggingConfig::default();
    let cb: CLogCallback = test_callback;
    let lm = LogManager::with_app_sink(
        config,
        CallbackLayer::from_c(cb),
        SessionRegistry::default(),
    )
    .unwrap();
    let _guard = tracing::dispatcher::set_default(lm.dispatch());

    tracing::info!("callback_info_msg");
    tracing::warn!("callback_warn_msg");

    let events = CAPTURED.lock().unwrap();
    let info_event = events
        .iter()
        .find(|e| e.message.contains("callback_info_msg"));
    assert!(
        info_event.is_some(),
        "callback should have received INFO event"
    );
    assert_eq!(info_event.unwrap().level, 2, "INFO level should map to 2");

    let warn_event = events
        .iter()
        .find(|e| e.message.contains("callback_warn_msg"));
    assert!(
        warn_event.is_some(),
        "callback should have received WARN event"
    );
    assert_eq!(warn_event.unwrap().level, 1, "WARN level should map to 1");
}

/// Verify that wrapper logs submitted via `sf_core_log_event` round-trip through
/// the tracing pipeline and arrive at the C callback with `logger_name` set.
#[test]
fn sf_core_log_event_delivers_wrapper_events() {
    let _serial = TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
    CAPTURED.lock().unwrap().clear();

    let cb: CLogCallback = test_callback;
    let init = sf_core::logging::c_api::sf_core_init(cb);
    assert_eq!(init.status, 0);

    let message = c"wrapper round trip";
    let file = c"cursor.py";
    let function = c"execute";
    let logger_name = c"snowflake.connector.cursor._base";

    unsafe {
        assert_eq!(
            sf_core::logging::c_api::sf_core_log_event(
                2,
                message.as_ptr(),
                file.as_ptr(),
                10,
                function.as_ptr(),
                logger_name.as_ptr(),
            ),
            0,
            "sf_core_log_event should return 0 on success"
        );
    }

    let events = CAPTURED.lock().unwrap();
    let event = events
        .iter()
        .find(|e| e.message.contains("wrapper round trip"));
    assert!(
        event.is_some(),
        "callback should have received wrapper round-trip event"
    );
    let event = event.unwrap();
    assert_eq!(event.level, 2, "INFO level should map to 2");
    assert_eq!(event.logger_name, "snowflake.connector.cursor._base");
}
