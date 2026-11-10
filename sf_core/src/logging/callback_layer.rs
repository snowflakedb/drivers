//! Tracing layer that forwards every event to a wrapper log sink callback.
//! Formats level, message, source location, and optional `logger_name` for the
//! wrapper round-trip path.

use crate::logging::{NormalizedEvent, TraceEncoding, normalize_event_with_trace_encoding};
use std::ffi::{CString, c_char};
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

/// C ABI log callback used by the legacy FFI (`sf_core_init_python`) and its tests.
///
/// `level` is the outbound wire encoding: 0=ERROR, 1=WARN, 2=INFO, 3=DEBUG.
/// TRACE is 3 or 4 depending on the layer's [`TraceEncoding`].
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
    trace_encoding: TraceEncoding,
}

impl CallbackLayer {
    /// Build a layer from a Rust callback (e.g. a PyO3 closure).
    ///
    /// Uses [`TraceEncoding::FoldIntoDebug`] unless [`Self::with_trace_encoding`] overrides it.
    pub fn new(callback: impl Fn(NormalizedEvent) + Send + Sync + 'static) -> Self {
        Self {
            callback: Box::new(callback),
            trace_encoding: TraceEncoding::default(),
        }
    }

    pub(crate) fn with_trace_encoding(mut self, trace_encoding: TraceEncoding) -> Self {
        self.trace_encoding = trace_encoding;
        self
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
        (self.callback)(normalize_event_with_trace_encoding(
            event,
            self.trace_encoding,
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tracing_subscriber::layer::SubscriberExt;

    use super::CallbackLayer;
    use crate::logging::{DEBUG_WIRE_LEVEL, DISTINCT_TRACE_WIRE_LEVEL, TraceEncoding};

    fn delivered_levels(trace_encoding: Option<TraceEncoding>) -> Vec<(u32, String)> {
        let events = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&events);
        let layer = CallbackLayer::new(move |event| {
            seen.lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .push((event.level, event.message));
        });
        let layer = match trace_encoding {
            Some(encoding) => layer.with_trace_encoding(encoding),
            None => layer,
        };
        let subscriber = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(subscriber, || {
            tracing::trace!("trace detail");
            tracing::debug!("debug detail");
        });
        events
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }

    fn level_of(events: &[(u32, String)], message: &str) -> Option<u32> {
        events
            .iter()
            .find(|(_, text)| text == message)
            .map(|(level, _)| *level)
    }

    #[test]
    fn default_layer_folds_trace_into_debug() {
        let events = delivered_levels(None);
        assert_eq!(level_of(&events, "trace detail"), Some(DEBUG_WIRE_LEVEL));
        assert_eq!(level_of(&events, "debug detail"), Some(DEBUG_WIRE_LEVEL));
    }

    #[test]
    fn distinct_trace_encoding_leaves_debug_at_debug() {
        let events = delivered_levels(Some(TraceEncoding::Distinct));
        assert_eq!(
            level_of(&events, "trace detail"),
            Some(DISTINCT_TRACE_WIRE_LEVEL)
        );
        assert_eq!(level_of(&events, "debug detail"), Some(DEBUG_WIRE_LEVEL));
    }
}
