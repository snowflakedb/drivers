//! Tracing layer that forwards every event to a single C callback (the Python/JDBC log sink).
//! Formats level, message, source location, and optional `logger_name` for the wrapper round-trip path.

use crate::logging::normalize_event;
use std::ffi::{CString, c_char};
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

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

impl<S> Layer<S> for CallbackLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let fields = normalize_event(event);
        unsafe {
            (self.callback)(
                fields.level,
                CString::new(fields.message).unwrap_or_default().as_ptr(),
                CString::new(fields.file).unwrap_or_default().as_ptr(),
                fields.line,
                CString::new(fields.function).unwrap_or_default().as_ptr(),
                CString::new(fields.logger_name)
                    .unwrap_or_default()
                    .as_ptr(),
            )
        };
    }
}
