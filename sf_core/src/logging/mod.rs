use std::path::PathBuf;

pub use crate::logging::callback_layer::CLogCallback;
pub use crate::logging::callback_layer::CallbackLayer;
pub use crate::logging::callback_layer::StructuredLogCallback;
pub use crate::logging::error::LogError;
pub use crate::logging::log_manager::LogManager;
use tracing::Subscriber;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;

pub mod c_api;
mod callback_layer;
mod error;
pub mod event_sanitizer;
pub mod log_manager;
mod opentelemetry;
pub mod rolling_writer;

const DEFAULT_LOG_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB
const DEFAULT_LOG_FILE_COUNT: usize = 5;

pub struct LoggingConfig {
    /// Directory or file path for log output. `None` disables file logging.
    pub log_path: Option<PathBuf>,
    /// Maximum size in bytes per log file before rotation.
    pub log_file_size: u64,
    /// Maximum number of rotated log files to retain.
    pub log_file_count: usize,
    /// Minimum severity level for log output.
    pub log_level: LevelFilter,
    /// Master switch for the core logging subsystem.
    pub enabled: bool,
    pub stderr: bool,
    pub opentelemetry: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_path: None,
            log_file_size: DEFAULT_LOG_FILE_SIZE,
            log_file_count: DEFAULT_LOG_FILE_COUNT,
            log_level: LevelFilter::INFO,
            enabled: true,
            stderr: false,
            opentelemetry: false,
        }
    }
}

impl LoggingConfig {
    pub fn new(log_path: Option<PathBuf>, stderr: bool, opentelemetry: bool) -> Self {
        Self {
            log_path,
            stderr,
            opentelemetry,
            ..Default::default()
        }
    }
}

struct EmptyLayer;

impl<S: Subscriber> Layer<S> for EmptyLayer {}

/// Convenience wrapper that initialises the global [`LogManager`] without an
/// extra wrapper layer.
pub fn init(config: LoggingConfig) -> Result<(), LogError> {
    LogManager::init(config)?;
    Ok(())
}

/// Initialise logging with an additional wrapper-specific layer (e.g. JDBC
/// SLF4J bridge, C-API callback layer).
///
/// Delegates to [`LogManager::init_with_layer`].
pub fn init_logging<L>(config: LoggingConfig, extra_layer: Option<L>) -> Result<(), LogError>
where
    L: Layer<Registry> + Send + Sync + 'static,
{
    LogManager::init_with_layer(config, extra_layer)?;
    Ok(())
}
