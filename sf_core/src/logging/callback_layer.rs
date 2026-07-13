//! Tracing layer that forwards every event to a single C callback (the Python/JDBC log sink).
//! Formats level, message, source location, and optional `logger_name` for the wrapper round-trip path.

use std::ffi::{CString, c_char};
use std::fmt::Debug;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

pub(crate) const WRAPPER_TARGET: &str = "sf_wrapper";

pub type CLogCallback = unsafe extern "C" fn(
    level: u32,
    message: *const c_char,
    filename: *const c_char,
    line: u32,
    function: *const c_char,
    // Empty for core-originated events (e.g. Python uses `snowflake.connector._core`).
    // Set for wrapper round-trip events (wrapper dispatches to that module logger).
    logger_name: *const c_char,
) -> u32;

pub struct CallbackLayer {
    callback: CLogCallback,
}

impl CallbackLayer {
    pub fn new(callback: CLogCallback) -> Self {
        Self { callback }
    }
}

#[derive(Default)]
struct WrapperEventFields {
    message: String,
    logger_name: String,
    file: Option<String>,
    line: Option<u32>,
    function: Option<String>,
    extra: String,
}

impl WrapperEventFields {
    fn record_extra(&mut self, name: &str, value: impl std::fmt::Display) {
        use std::fmt::Write as _;
        let _ = write!(self.extra, " {name}={value}");
    }
}

impl Visit for WrapperEventFields {
    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == "line" {
            self.line = u32::try_from(value).ok();
        } else {
            self.record_extra(field.name(), value);
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "message" => self.message = value.to_owned(),
            "logger_name" => self.logger_name = value.to_owned(),
            "file" => self.file = Some(value.to_owned()),
            "function" => self.function = Some(value.to_owned()),
            name => self.record_extra(name, value),
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        match field.name() {
            "message" if self.message.is_empty() => self.message = format!("{value:?}"),
            name => self.record_extra(name, format_args!("{value:?}")),
        }
    }
}

impl<S> Layer<S> for CallbackLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let level = match *meta.level() {
            Level::ERROR => 0,
            Level::WARN => 1,
            Level::INFO => 2,
            Level::DEBUG => 3,
            Level::TRACE => 4,
        };

        // for events coming from wrapper we need to fetch metadata from event fields
        // usual metadata points to sf_core_log_event function for those
        let mut fields = WrapperEventFields::default();
        event.record(&mut fields);

        let is_wrapper = meta.target() == WRAPPER_TARGET;
        let message = if is_wrapper || fields.extra.is_empty() {
            fields.message
        } else {
            format!("{}{}", fields.message, fields.extra)
        };
        let (filename, line, function, logger_name) = if is_wrapper {
            (
                fields.file.as_deref().unwrap_or("unknown"),
                fields.line.unwrap_or(0),
                fields.function.as_deref().unwrap_or("unknown"),
                fields.logger_name.as_str(),
            )
        } else {
            (
                meta.file().unwrap_or("unknown"),
                meta.line().unwrap_or(0),
                meta.name(),
                "",
            )
        };

        unsafe {
            (self.callback)(
                level,
                CString::new(message).unwrap_or_default().as_ptr(),
                CString::new(filename).unwrap_or_default().as_ptr(),
                line,
                CString::new(function).unwrap_or_default().as_ptr(),
                CString::new(logger_name).unwrap_or_default().as_ptr(),
            )
        };
    }
}
