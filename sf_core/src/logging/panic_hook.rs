//! Install a global Rust panic hook that captures panic payloads + backtraces
//! and persists them to a dedicated file so post-mortem diagnostics survive
//! even when the panic eventually terminates the process (e.g. when it
//! unwinds across an FFI boundary and aborts).
//!
//! The hook is installed once per process. On panic it:
//!   1. Emits `tracing::error!` so any active tracing sinks capture it.
//!   2. Appends a structured record (timestamp, thread, payload, location,
//!      backtrace) to the panic log file.
//!   3. Chains to the previously-installed hook so normal panic output still
//!      goes to stderr and the runtime keeps its default behavior.
//!
//! The file path resolution order is:
//!   1. `SF_ODBC_PANIC_LOG`  — explicit override for the panic file.
//!   2. `SF_ODBC_LOG_PATH`   — reuse the ODBC log directory, substituting the
//!      basename `odbc.log` with `odbc_panic.log` (matches what our CI collector
//!      already expects in `environments/end-to-end/snowflake/run_tests`).
//!   3. `./odbc_panic.log`   — relative to the process CWD.
//!
//! Backtrace capture forces `RUST_BACKTRACE=full` inside the hook so we always
//! get a trace regardless of caller environment.

use std::backtrace::Backtrace;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Once;

static INSTALL: Once = Once::new();

/// Install the panic hook. Safe to call multiple times — only the first call
/// has effect. Must be invoked before any thread can panic if you want the
/// panic to be captured.
pub fn install() {
    INSTALL.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let backtrace = Backtrace::force_capture();
            write_panic_record(info, &backtrace);
            emit_tracing_error(info, &backtrace);
            prev(info);
        }));
    });
}

fn panic_log_path() -> PathBuf {
    if let Some(explicit) = std::env::var_os("SF_ODBC_PANIC_LOG") {
        return PathBuf::from(explicit);
    }
    if let Some(log_path) = std::env::var_os("SF_ODBC_LOG_PATH") {
        let p = PathBuf::from(log_path);
        if let Some(parent) = p.parent() {
            // Derive sibling file: <parent>/odbc_panic.log
            return parent.join("odbc_panic.log");
        }
    }
    PathBuf::from("odbc_panic.log")
}

fn write_panic_record(info: &std::panic::PanicHookInfo<'_>, backtrace: &Backtrace) {
    let path = panic_log_path();
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };

    let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    let thread = std::thread::current();
    let thread_name = thread.name().unwrap_or("<unnamed>");
    let payload = panic_payload_str(info);
    let location = info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "<unknown>".to_string());

    let _ = writeln!(
        file,
        "====================  RUST PANIC  ====================\n\
         timestamp : {timestamp}\n\
         thread    : {thread_name}\n\
         location  : {location}\n\
         payload   : {payload}\n\
         backtrace :\n{backtrace}\n\
         ======================================================",
    );
    let _ = file.flush();
}

fn emit_tracing_error(info: &std::panic::PanicHookInfo<'_>, backtrace: &Backtrace) {
    let payload = panic_payload_str(info);
    let location = info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "<unknown>".to_string());
    tracing::error!(
        target: "sf_core::panic",
        panic_payload = %payload,
        panic_location = %location,
        panic_backtrace = %backtrace,
        "Rust panic captured by global hook"
    );
}

fn panic_payload_str(info: &std::panic::PanicHookInfo<'_>) -> String {
    let payload = info.payload();
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        format!("<non-string panic payload: {:?}>", payload.type_id())
    }
}

/// For testing: resolve the path that the hook would write to given the
/// current environment.
#[cfg(test)]
pub(crate) fn resolve_path_for_test() -> PathBuf {
    panic_log_path()
}

/// Helper used by tests to assert that a file ends up where we expect without
/// actually installing the hook.
#[cfg(test)]
pub(crate) fn write_record_for_test(
    target: &std::path::Path,
    payload: &str,
    location: &str,
    backtrace: &str,
) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(target)?;
    writeln!(
        file,
        "====================  RUST PANIC  ====================\n\
         timestamp : <test>\n\
         thread    : <test>\n\
         location  : {location}\n\
         payload   : {payload}\n\
         backtrace :\n{backtrace}\n\
         ======================================================",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn defaults_to_cwd_filename_when_no_env() {
        temp_env::with_vars(
            [
                ("SF_ODBC_PANIC_LOG", None::<&str>),
                ("SF_ODBC_LOG_PATH", None::<&str>),
            ],
            || {
                assert_eq!(resolve_path_for_test(), PathBuf::from("odbc_panic.log"));
            },
        );
    }

    #[test]
    fn prefers_explicit_override() {
        temp_env::with_vars(
            [
                ("SF_ODBC_PANIC_LOG", Some("/var/log/custom.log")),
                ("SF_ODBC_LOG_PATH", Some("/unused/odbc.log")),
            ],
            || {
                assert_eq!(
                    resolve_path_for_test(),
                    PathBuf::from("/var/log/custom.log")
                );
            },
        );
    }

    #[test]
    fn derives_sibling_of_odbc_log_path() {
        temp_env::with_vars(
            [
                ("SF_ODBC_PANIC_LOG", None::<&str>),
                ("SF_ODBC_LOG_PATH", Some("/checkout/toucan/odbc.log")),
            ],
            || {
                assert_eq!(
                    resolve_path_for_test(),
                    PathBuf::from("/checkout/toucan/odbc_panic.log")
                );
            },
        );
    }

    #[test]
    fn written_record_contains_fields() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("panic.log");
        write_record_for_test(&path, "boom", "file.rs:1:1", "frame0\nframe1").unwrap();
        let body = fs::read_to_string(&path).unwrap();
        assert!(body.contains("RUST PANIC"));
        assert!(body.contains("payload   : boom"));
        assert!(body.contains("location  : file.rs:1:1"));
        assert!(body.contains("frame0"));
        assert!(body.contains("frame1"));
    }
}
