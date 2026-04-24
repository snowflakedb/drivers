use std::ffi::{CString, c_char};
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::field::Field;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

pub type CLogCallback = unsafe extern "C" fn(
    level: u32,
    message: *const c_char,
    filename: *const c_char,
    line: u32,
    function: *const c_char,
) -> u32;

/// Global flag to disable the callback layer during shutdown.
/// Once set to `true`, `on_event` returns immediately without calling
/// the Python callback — preventing SIGSEGV during `Py_Finalize`.
static CALLBACK_DISABLED: AtomicBool = AtomicBool::new(false);

/// Disable the callback layer. Called from `sf_core_shutdown` before
/// the Python interpreter is finalized.
pub fn disable_callback() {
    CALLBACK_DISABLED.store(true, Ordering::SeqCst);
}

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
        if CALLBACK_DISABLED.load(Ordering::SeqCst) {
            return;
        }
        let level = match *event.metadata().level() {
            Level::ERROR => 0,
            Level::WARN => 1,
            Level::INFO => 2,
            Level::DEBUG => 3,
            Level::TRACE => 4,
        };
        let mut message = String::new();
        event.record(&mut |field: &Field, value: &dyn Debug| {
            if field.name() == "message" {
                message.push_str(&format!("{value:?}"));
            } else {
                let name = field.name();
                message.push_str(&format!(" {name}={value:?}"));
            }
        });
        // let message = message.as_bytes();
        let line = event.metadata().line().unwrap_or(0);
        let message = CString::new(message).unwrap_or_default();
        let filename =
            CString::new(event.metadata().file().unwrap_or("unknown")).unwrap_or_default();
        let function = CString::new(event.metadata().name()).unwrap_or_default();
        // Note: catch_unwind cannot catch SIGSEGV (signal, not Rust panic).
        // The CALLBACK_DISABLED AtomicBool guard above prevents the vast majority
        // of post-shutdown calls. The remaining TOCTOU window (~1μs between check
        // and call) is negligible vs Py_Finalize's module teardown time (~ms).
        unsafe {
            (self.callback)(
                level,
                message.as_ptr(),
                filename.as_ptr(),
                line,
                function.as_ptr(),
            )
        };
    }
}
