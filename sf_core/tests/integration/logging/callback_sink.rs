use std::ffi::{CStr, c_char};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use sf_core::logging::{CLogCallback, CallbackLayer, LogManager, LoggingConfig};
use sf_core::telemetry::snowflake_exporter::SessionRegistry;

struct CapturedEvent {
    level: u32,
    message: String,
}

static CAPTURED: Mutex<Vec<CapturedEvent>> = Mutex::new(Vec::new());

unsafe extern "C" fn test_callback(
    level: u32,
    message: *const c_char,
    _filename: *const c_char,
    _line: u32,
    _function: *const c_char,
) -> u32 {
    let msg = unsafe { CStr::from_ptr(message) }
        .to_string_lossy()
        .into_owned();
    CAPTURED.lock().unwrap().push(CapturedEvent {
        level,
        message: msg,
    });
    0
}

/// Verify that events emitted through tracing are delivered to a C callback
/// registered via `CallbackLayer`.
#[test]
fn callback_layer_delivers_events() {
    let config = LoggingConfig::default();
    let cb: CLogCallback = test_callback;
    LogManager::with_app_sink(config, CallbackLayer::new(cb), SessionRegistry::default()).unwrap();

    tracing::info!("callback_info_msg");
    tracing::warn!("callback_warn_msg");

    thread::sleep(Duration::from_millis(200));

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
