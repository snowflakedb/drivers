use std::path::PathBuf;

pub use crate::logging::callback_layer::CLogCallback;
pub use crate::logging::callback_layer::CallbackLayer;
pub use crate::logging::error::LogError;
pub use crate::logging::log_manager::LogManager;
use tracing::Subscriber;
use tracing::level_filters::LevelFilter;
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
