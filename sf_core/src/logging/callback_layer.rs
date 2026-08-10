//! Tracing layer that forwards every event to a wrapper log sink callback.
//! Formats level, message, source location, and optional `logger_name` for the
//! wrapper round-trip path.

use crate::logging::{NormalizedEvent, normalize_event};
use std::ffi::{CString, c_char};
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

/// C ABI log callback used by the legacy FFI (`sf_core_init`) and its tests.
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

/// Sink that receives flattened log events from the tracing pipeline.
type LogCallback = Box<dyn Fn(NormalizedEvent) + Send + Sync>;

pub struct CallbackLayer {
    callback: LogCallback,
}

impl CallbackLayer {
    /// Build a layer from a Rust callback (e.g. a PyO3 closure).
    pub fn new(callback: impl Fn(NormalizedEvent) + Send + Sync + 'static) -> Self {
        Self {
            callback: Box::new(callback),
        }
    }

    /// Adapt a [`CLogCallback`] for use as the app sink (C FFI / tests).
    pub fn from_c(callback: CLogCallback) -> Self {
        Self::new(move |fields| {
            // Bind CStrings so pointers remain valid for the duration of the call.
            let message = CString::new(fields.message).unwrap_or_default();
            let filename = CString::new(fields.file).unwrap_or_default();
            let function = CString::new(fields.function).unwrap_or_default();
            let logger_name = CString::new(fields.logger_name).unwrap_or_default();
            unsafe {
                callback(
                    fields.level,
                    message.as_ptr(),
                    filename.as_ptr(),
                    fields.line,
                    function.as_ptr(),
                    logger_name.as_ptr(),
                );
            }
        })
    }
}

impl<S> Layer<S> for CallbackLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        (self.callback)(normalize_event(event));
    }
}
