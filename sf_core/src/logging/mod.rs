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
pub mod ini_config;
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
#[derive(Debug)]
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
        }
    }
}

pub(crate) struct EmptyLayer;

impl<S: Subscriber> Layer<S> for EmptyLayer {}
