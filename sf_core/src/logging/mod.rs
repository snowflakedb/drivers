use std::fmt::Debug;
use std::path::PathBuf;

pub use crate::logging::callback_layer::CLogCallback;
pub use crate::logging::callback_layer::CallbackLayer;
pub use crate::logging::error::LogError;
pub use crate::logging::log_manager::LogManager;
use tracing::Subscriber;
use tracing::level_filters::LevelFilter;
use tracing_core::Field;
use tracing_core::field::Visit;
use tracing_subscriber::Layer;

pub mod c_api;
pub(crate) mod callback_layer;
pub(crate) mod error;
pub mod log_manager;
pub(crate) mod opentelemetry;

/// Time-based log-file rotation strategy.
///
/// Wraps `tracing_appender::rolling::Rotation` so callers don't depend on
/// the appender crate directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogRotation {
    #[default]
    Never,
    Daily,
    Hourly,
    Minutely,
}

impl LogRotation {
    pub(crate) fn to_appender_rotation(self) -> tracing_appender::rolling::Rotation {
        match self {
            Self::Never => tracing_appender::rolling::Rotation::NEVER,
            Self::Daily => tracing_appender::rolling::Rotation::DAILY,
            Self::Hourly => tracing_appender::rolling::Rotation::HOURLY,
            Self::Minutely => tracing_appender::rolling::Rotation::MINUTELY,
        }
    }
}

/// Configuration for the logging subsystem.
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub enabled: bool,
    pub level: LevelFilter,
    pub log_path: Option<PathBuf>,
    pub log_file_name: Option<String>,
    /// Desired maximum size (in bytes) for a single log file.
    ///
    /// **Not yet enforced.** `tracing-appender` only supports time-based
    /// rotation, so size-based rotation is not available. When this field is
    /// `Some`, a warning is emitted at init time and the value is otherwise
    /// ignored. The field is retained for forward-compatibility with a future
    /// size-aware appender.
    pub max_file_size: Option<u64>,
    pub max_file_count: Option<u32>,
    pub rotation: LogRotation,
    pub open_telemetry: bool,
    pub stderr: bool,
    /// Process-wide default for `log_query_text`, applied as a fallback when
    /// neither a connection-string option nor a `connections.toml` /
    /// `config.toml` setting is provided. `None` means "unset; fall through to
    /// the registry default".
    pub log_query_text: Option<bool>,
    /// Process-wide default for `log_query_parameters`. See
    /// [`Self::log_query_text`] for precedence semantics.
    pub log_query_parameters: Option<bool>,
    /// When `true`, `OdbcError::message_text()` appends the full error trace
    /// to user-facing diagnostic messages. Default `true`.
    pub error_trace_enabled: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: LevelFilter::INFO,
            log_path: None,
            log_file_name: None,
            max_file_size: None,
            max_file_count: None,
            rotation: LogRotation::default(),
            open_telemetry: false,
            stderr: false,
            log_query_text: None,
            log_query_parameters: None,
            error_trace_enabled: true,
        }
    }
}

pub(crate) struct EmptyLayer;

impl<S: Subscriber> Layer<S> for EmptyLayer {}

/// Log a foreign/external error: emit the error type name at `error` level (safe for
/// WARN/ERROR logs) and the full `{:?}` detail at `debug` level.
///
/// # Usage
/// ```ignore
/// log_foreign_error!(e, "Failed to send result to channel");
/// log_foreign_error!(warn, e, "Failed to read response body");
/// ```
#[macro_export]
macro_rules! log_foreign_error {
    ($e:expr, $msg:literal) => {{
        tracing::error!(cause = ::std::any::type_name_of_val(&$e), $msg);
        tracing::debug!(concat!($msg, ": {:?}"), $e);
    }};
    (warn, $e:expr, $msg:literal) => {{
        tracing::warn!(cause = ::std::any::type_name_of_val(&$e), $msg);
        tracing::debug!(concat!($msg, ": {:?}"), $e);
    }};
}
pub use log_foreign_error;

/// Extract host and path from a URL string for safe logging (strips query strings and
/// fragments which can carry tokens and other sensitive identifiers).
///
/// Returns `"<unknown>"` if the URL cannot be parsed.
pub fn url_for_log(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .map(|u| {
            let host = u.host_str().unwrap_or("<unknown host>");
            let path = u.path();
            if path.is_empty() || path == "/" {
                host.to_string()
            } else {
                format!("{host}{path}")
            }
        })
        .unwrap_or_else(|| "<unknown>".into())
}

/// Tracing target carried by wrapper-originated events (the round-trip path).
/// Events on this target had their real source location and originating logger
/// name packed into event fields by the [`wrapper_event!`] macro; core-originated
/// events use the normal tracing metadata instead.
pub const WRAPPER_TARGET: &str = "sf_wrapper";

/// `tracing::event!` requires a compile-time level; dispatch at runtime via match.
#[macro_export]
macro_rules! wrapper_event {
    ($level:expr, $($fields:tt)*) => {
        match $level {
            0 => tracing::event!(target: $crate::logging::WRAPPER_TARGET, tracing::Level::ERROR, $($fields)*),
            1 => tracing::event!(target: $crate::logging::WRAPPER_TARGET, tracing::Level::WARN, $($fields)*),
            2 => tracing::event!(target: $crate::logging::WRAPPER_TARGET, tracing::Level::INFO, $($fields)*),
            _ => tracing::event!(target: $crate::logging::WRAPPER_TARGET, tracing::Level::DEBUG, $($fields)*),
        }
    };
}

/// Flattened tracing event fields for wrapper log sinks (FFI/JNI callback layers).
///
/// `logger_name` is empty for core-originated events; set for wrapper round-trip events.
/// `level` uses the shared FFI/JNI encoding: 0=ERROR, 1=WARN, 2=INFO, 3 or higher=DEBUG.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedEvent {
    pub level: u32,
    pub message: String,
    pub file: String,
    pub line: u32,
    pub function: String,
    pub logger_name: String,
}

#[derive(Default)]
struct EventVisitor {
    message: String,
    file: Option<String>,
    line: Option<u32>,
    function: Option<String>,
    logger_name: String,
    extra: String,
}

impl EventVisitor {
    fn record_extra(&mut self, name: &str, value: impl std::fmt::Display) {
        use std::fmt::Write as _;
        let _ = write!(self.extra, " {name}={value}");
    }
}

impl Visit for EventVisitor {
    fn record_i64(&mut self, field: &Field, value: i64) {
        if field.name() == "line" {
            self.line = u32::try_from(value).ok();
        } else {
            self.record_extra(field.name(), value);
        }
    }

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

/// Flatten a tracing [`Event`] into [`NormalizedEvent`] for a wrapper log sink.
///
/// For wrapper round-trip events ([`WRAPPER_TARGET`]) the real source location
/// and originating logger name live in event fields; for core-originated events
/// they come from the event metadata and `logger_name` is left empty.
pub fn normalize_event(event: &tracing::Event<'_>) -> NormalizedEvent {
    let meta = event.metadata();
    let level = match *meta.level() {
        tracing::Level::ERROR => 0,
        tracing::Level::WARN => 1,
        tracing::Level::INFO => 2,
        _ => 3,
    };

    let mut visitor = EventVisitor::default();
    event.record(&mut visitor);

    if meta.target() == WRAPPER_TARGET {
        NormalizedEvent {
            level,
            message: visitor.message,
            file: visitor.file.unwrap_or_else(|| "unknown".to_owned()),
            line: visitor.line.unwrap_or(0),
            function: visitor.function.unwrap_or_else(|| "unknown".to_owned()),
            logger_name: visitor.logger_name,
        }
    } else {
        let message = if visitor.extra.is_empty() {
            visitor.message
        } else {
            format!("{}{}", visitor.message, visitor.extra)
        };
        NormalizedEvent {
            level,
            message,
            file: meta.file().unwrap_or("unknown").to_owned(),
            line: meta.line().unwrap_or(0),
            function: meta.name().to_owned(),
            logger_name: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing::Subscriber;
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::{Context, SubscriberExt};

    struct CapturingLayer {
        events: Arc<Mutex<Vec<NormalizedEvent>>>,
    }

    impl CapturingLayer {
        fn new(events: Arc<Mutex<Vec<NormalizedEvent>>>) -> Self {
            Self { events }
        }
    }

    impl<S: Subscriber> Layer<S> for CapturingLayer {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            self.events
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(normalize_event(event));
        }
    }

    fn capture_events<F: FnOnce()>(emit: F) -> Vec<NormalizedEvent> {
        let events = Arc::new(Mutex::new(Vec::new()));
        let subscriber =
            tracing_subscriber::registry().with(CapturingLayer::new(Arc::clone(&events)));
        let _guard = tracing::subscriber::set_default(subscriber);
        emit();
        events.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    #[test]
    fn should_map_core_info_event_from_metadata() {
        let events = capture_events(|| tracing::info!(target: "sf_core", "core info message"));
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.level, 2);
        assert_eq!(event.message, "core info message");
        assert!(event.logger_name.is_empty());
        assert!(!event.file.is_empty());
        assert_ne!(event.function, "unknown");
    }

    #[test]
    fn should_map_core_trace_event_to_debug_wire_level() {
        let events = capture_events(|| tracing::trace!(target: "sf_core", "trace detail"));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, 3);
        assert_eq!(events[0].message, "trace detail");
        assert!(events[0].logger_name.is_empty());
    }

    #[test]
    fn should_map_wrapper_round_trip_fields_from_event_payload() {
        let events = capture_events(|| {
            crate::wrapper_event!(
                2,
                message = "wrapper payload",
                file = "Driver.java",
                function = "connect",
                line = 99,
                logger_name = "net.snowflake.client.Driver",
            );
        });
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.level, 2);
        assert_eq!(event.message, "wrapper payload");
        assert_eq!(event.file, "Driver.java");
        assert_eq!(event.line, 99);
        assert_eq!(event.function, "connect");
        assert_eq!(event.logger_name, "net.snowflake.client.Driver");
    }

    #[test]
    fn should_append_extra_fields_on_core_originated_events() {
        let events = capture_events(|| tracing::info!(target: "sf_core", user_id = 42, "hello"));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].message, "hello user_id=42");
        assert!(events[0].logger_name.is_empty());
    }

    #[test]
    fn should_not_append_extra_fields_on_wrapper_round_trip_events() {
        let events = capture_events(|| {
            crate::wrapper_event!(
                3,
                message = "already formatted",
                file = "",
                function = "",
                line = 0,
                logger_name = "com.example.Logger",
                spare = "ignored",
            );
        });
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].message, "already formatted");
        assert_eq!(events[0].logger_name, "com.example.Logger");
    }
}
