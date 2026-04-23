use std::ffi::{CString, c_char};
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tracing::field::Field;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

use crate::logging::event_sanitizer::with_sanitized_event;

/// C-ABI log callback for legacy wrapper integrations (ODBC, C API).
pub type CLogCallback = unsafe extern "C" fn(
    level: u32,
    message: *const c_char,
    filename: *const c_char,
    line: u32,
    function: *const c_char,
) -> u32;

/// C-ABI callback that receives structured log data as a JSON string.
///
/// The JSON payload contains `timestamp`, `level`, `module`, `message`, and
/// a `fields` object with all remaining event key-value pairs (e.g.
/// `session_id`, `query_id`, `error_code`).
pub type StructuredLogCallback = unsafe extern "C" fn(json: *const c_char, json_len: usize) -> u32;

// ---------------------------------------------------------------------------
// CallbackState — shared mutable slot for runtime callback registration
// ---------------------------------------------------------------------------

/// Shared mutable state backing a [`CallbackLayer`], allowing callbacks to be
/// registered or replaced at runtime via
/// [`LogManager`](crate::logging::log_manager::LogManager).
pub(crate) struct CallbackState {
    inner: Mutex<CallbackSlots>,
    has_callback: AtomicBool,
}

struct CallbackSlots {
    legacy: Option<CLogCallback>,
    structured: Option<StructuredLogCallback>,
}

impl CallbackState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(CallbackSlots {
                legacy: None,
                structured: None,
            }),
            has_callback: AtomicBool::new(false),
        }
    }

    pub fn set_legacy(&self, cb: CLogCallback) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.legacy = Some(cb);
        self.has_callback.store(true, Ordering::Release);
    }

    #[allow(dead_code)]
    pub fn set_structured(&self, cb: StructuredLogCallback) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.structured = Some(cb);
        self.has_callback.store(true, Ordering::Release);
    }

    #[allow(dead_code)]
    pub fn clear(&self) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.legacy = None;
        guard.structured = None;
        self.has_callback.store(false, Ordering::Release);
    }

    fn snapshot(&self) -> (Option<CLogCallback>, Option<StructuredLogCallback>) {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        (guard.legacy, guard.structured)
    }
}

// ---------------------------------------------------------------------------
// CallbackLayer
// ---------------------------------------------------------------------------

/// A [`tracing_subscriber::Layer`] that forwards log events to one or both
/// C-ABI callbacks (legacy and/or structured JSON).
///
/// When an [`EventSanitizerLayer`](crate::logging::event_sanitizer::EventSanitizerLayer)
/// is present in the subscriber stack (and runs first), this layer
/// automatically uses the sanitised field values. Otherwise it falls back to
/// the raw event fields.
///
/// Level filtering is handled externally via a per-layer
/// [`SharedLevelFilter`](crate::logging::log_manager::SharedLevelFilter)
/// applied with [`Layer::with_filter`] by the
/// [`LogManager`](crate::logging::log_manager::LogManager).
pub struct CallbackLayer {
    state: Arc<CallbackState>,
}

impl CallbackLayer {
    /// Creates a `CallbackLayer` with a pre-set legacy callback.
    ///
    /// Use this for standalone initialisation without [`LogManager`].
    pub fn new(callback: CLogCallback) -> Self {
        let state = Arc::new(CallbackState::new());
        state.set_legacy(callback);
        Self { state }
    }

    /// Creates a `CallbackLayer` backed by externally-owned shared state,
    /// for use with [`LogManager`](crate::logging::log_manager::LogManager).
    pub(crate) fn from_shared(state: Arc<CallbackState>) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for CallbackLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if !self.state.has_callback.load(Ordering::Acquire) {
            return;
        }

        let (legacy_cb, structured_cb) = self.state.snapshot();
        if legacy_cb.is_none() && structured_cb.is_none() {
            return;
        }

        let meta = event.metadata();

        // Prefer sanitised fields set by EventSanitizerLayer; fall back to raw.
        let (message, fields) = if let Some(result) =
            with_sanitized_event(|s| (s.format_message(), s.fields().to_vec()))
        {
            result
        } else {
            collect_raw_fields(event)
        };

        if let Some(cb) = legacy_cb {
            let level = level_to_u32(meta.level());
            let line = meta.line().unwrap_or(0);
            let c_message = CString::new(message.as_str()).unwrap_or_default();
            let c_filename = CString::new(meta.file().unwrap_or("unknown")).unwrap_or_default();
            let c_function = CString::new(meta.name()).unwrap_or_default();
            unsafe {
                cb(
                    level,
                    c_message.as_ptr(),
                    c_filename.as_ptr(),
                    line,
                    c_function.as_ptr(),
                );
            }
        }

        if let Some(cb) = structured_cb {
            let json = format_structured_json(meta, &message, &fields);
            if let Ok(c_json) = CString::new(json.as_str()) {
                unsafe {
                    cb(c_json.as_ptr(), json.len());
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Collects event fields by recording through the `Debug` visitor.
/// Used as a fallback when no `EventSanitizerLayer` is installed.
fn collect_raw_fields(event: &Event<'_>) -> (String, Vec<(String, String)>) {
    let mut fields: Vec<(String, String)> = Vec::new();
    event.record(&mut |field: &Field, value: &dyn Debug| {
        fields.push((field.name().to_string(), format!("{value:?}")));
    });
    let message = format_message_from_fields(&fields);
    (message, fields)
}

/// Formats collected fields in the standard tracing style:
/// `message field1=value1 field2=value2`.
fn format_message_from_fields(fields: &[(String, String)]) -> String {
    let mut out = String::new();
    for (name, value) in fields {
        if name == "message" {
            out.push_str(value);
        } else {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(name);
            out.push('=');
            out.push_str(value);
        }
    }
    out
}

/// Builds a JSON string with structured log metadata and event fields.
fn format_structured_json(
    meta: &tracing::Metadata<'_>,
    message: &str,
    fields: &[(String, String)],
) -> String {
    let mut field_map = serde_json::Map::new();
    for (name, value) in fields {
        if name != "message" {
            field_map.insert(name.clone(), serde_json::Value::String(value.clone()));
        }
    }

    let json = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
        "level": meta.level().as_str(),
        "module": meta.module_path().unwrap_or(""),
        "message": message,
        "fields": field_map,
    });

    json.to_string()
}

/// Maps a tracing [`Level`] to the Simba DSI / ODBC numeric scale.
///
/// | Value | Simba DSI    | tracing      |
/// |-------|--------------|--------------|
/// |   0   | OFF          | —            |
/// |   1   | FATAL        | — (unused)   |
/// |   2   | ERROR        | `ERROR`      |
/// |   3   | WARNING      | `WARN`       |
/// |   4   | INFO         | `INFO`       |
/// |   5   | DEBUG        | `DEBUG`      |
/// |   6   | TRACE        | `TRACE`      |
fn level_to_u32(level: &Level) -> u32 {
    match *level {
        Level::ERROR => 2,
        Level::WARN => 3,
        Level::INFO => 4,
        Level::DEBUG => 5,
        Level::TRACE => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_state_lifecycle() {
        let state = CallbackState::new();
        assert!(!state.has_callback.load(Ordering::Acquire));

        let (legacy, structured) = state.snapshot();
        assert!(legacy.is_none());
        assert!(structured.is_none());

        unsafe extern "C" fn dummy_legacy(
            _level: u32,
            _msg: *const c_char,
            _file: *const c_char,
            _line: u32,
            _func: *const c_char,
        ) -> u32 {
            0
        }

        state.set_legacy(dummy_legacy);
        assert!(state.has_callback.load(Ordering::Acquire));
        let (legacy, _) = state.snapshot();
        assert!(legacy.is_some());

        state.clear();
        assert!(!state.has_callback.load(Ordering::Acquire));
        let (legacy, structured) = state.snapshot();
        assert!(legacy.is_none());
        assert!(structured.is_none());
    }

    #[test]
    fn format_message_from_fields_basic() {
        let fields = vec![
            ("message".to_string(), "hello".to_string()),
            ("key".to_string(), "value".to_string()),
        ];
        assert_eq!(format_message_from_fields(&fields), "hello key=value");
    }

    #[test]
    fn format_message_from_fields_no_message() {
        let fields = vec![
            ("host".to_string(), "example.com".to_string()),
            ("port".to_string(), "443".to_string()),
        ];
        assert_eq!(
            format_message_from_fields(&fields),
            "host=example.com port=443"
        );
    }

    #[test]
    fn format_structured_json_includes_all_fields() {
        let meta = tracing::Metadata::new(
            "test_event",
            "test_module",
            tracing::Level::INFO,
            Some("test.rs"),
            Some(42),
            Some("test_module"),
            tracing::field::FieldSet::new(&[], tracing::callsite::Identifier(&CALLSITE)),
            tracing::metadata::Kind::EVENT,
        );

        let fields = vec![
            ("message".to_string(), "hello world".to_string()),
            ("session_id".to_string(), "sess-123".to_string()),
            ("query_id".to_string(), "qid-456".to_string()),
        ];

        let json_str = format_structured_json(&meta, "hello world", &fields);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["level"], "INFO");
        assert_eq!(parsed["module"], "test_module");
        assert_eq!(parsed["message"], "hello world");
        assert_eq!(parsed["fields"]["session_id"], "sess-123");
        assert_eq!(parsed["fields"]["query_id"], "qid-456");
        assert!(parsed["timestamp"].is_string());
        assert!(
            parsed["fields"].get("message").is_none(),
            "message should not appear in fields"
        );
    }

    #[test]
    fn level_to_u32_matches_odbc_convention() {
        assert_eq!(level_to_u32(&Level::ERROR), 2);
        assert_eq!(level_to_u32(&Level::WARN), 3);
        assert_eq!(level_to_u32(&Level::INFO), 4);
        assert_eq!(level_to_u32(&Level::DEBUG), 5);
        assert_eq!(level_to_u32(&Level::TRACE), 6);
    }

    static CALLSITE: FakeCallsite = FakeCallsite;

    struct FakeCallsite;

    impl tracing::callsite::Callsite for FakeCallsite {
        fn set_interest(&self, _: tracing::subscriber::Interest) {}
        fn metadata(&self) -> &tracing::Metadata<'_> {
            unimplemented!()
        }
    }
}
