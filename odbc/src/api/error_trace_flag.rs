use std::sync::atomic::{AtomicBool, Ordering};

/// User-facing error-rendering policy toggle, seeded at env allocation time
/// from `LogManager::error_trace_enabled()`. Defaults to `true` so errors
/// produced before env init (lock poisoning, invalid handles) still print a
/// full trace. `Relaxed` is sufficient: the flag is a standalone boolean that is
/// initialized as a part of env allocation during sqlallochandle init.
/// Nothing happens before that concludes.
static ERROR_TRACE_ENABLED: AtomicBool = AtomicBool::new(true);

/// Whether `OdbcError::message_text()` should append the full error trace.
pub(crate) fn error_trace_enabled() -> bool {
    ERROR_TRACE_ENABLED.load(Ordering::Relaxed)
}

/// Records the configured error-trace flag. Called once from
/// `api::runtime::env_allocated` during global init.
pub(crate) fn set_error_trace_enabled(value: bool) {
    ERROR_TRACE_ENABLED.store(value, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::OdbcError;
    use std::sync::Mutex;

    /// Serializes any test that reads or writes the process-wide
    /// `ERROR_TRACE_ENABLED`, so parallel test execution doesn't let two
    /// tests interleave their get/set pairs.
    static FLAG_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn sample_error() -> OdbcError {
        OdbcError::EnvironmentHasConnections {
            location: snafu::Location::new("test", 0, 0),
        }
    }

    #[test]
    fn message_text_includes_trace_when_enabled() {
        let _guard = FLAG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let previous = error_trace_enabled();
        set_error_trace_enabled(true);

        let rendered = sample_error().message_text();
        assert!(
            rendered.contains("error trace:"),
            "flag=true should include `error trace:` header, got: {rendered:?}",
        );
        assert!(
            rendered.contains("environment has connections"),
            "flag=true should also contain base message, got: {rendered:?}",
        );

        set_error_trace_enabled(previous);
    }

    #[test]
    fn message_text_omits_trace_when_disabled() {
        let _guard = FLAG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let previous = error_trace_enabled();
        set_error_trace_enabled(false);

        let rendered = sample_error().message_text();
        assert!(
            !rendered.contains("error trace:"),
            "flag=false must omit `error trace:` header, got: {rendered:?}",
        );
        assert!(
            rendered.contains("environment has connections"),
            "flag=false should still contain base message, got: {rendered:?}",
        );

        set_error_trace_enabled(previous);
    }
}
