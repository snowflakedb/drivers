use std::path::PathBuf;

pub use crate::logging::callback_layer::CLogCallback;
pub use crate::logging::callback_layer::CallbackLayer;
pub use crate::logging::error::LogError;
use crate::logging::opentelemetry::init_tracer;
use tracing::Subscriber;
use tracing::level_filters::LevelFilter;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt;

pub mod c_api;
mod callback_layer;
mod error;
mod opentelemetry;

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

pub fn init(config: LoggingConfig) -> Result<(), LogError> {
    init_logging::<EmptyLayer>(config, None)
}

pub fn init_logging<L>(config: LoggingConfig, extra_layer: Option<L>) -> Result<(), LogError>
where
    L: Layer<Registry> + Send + Sync,
{
    if !config.enabled {
        return Ok(());
    }

    let subscriber = Registry::default();
    let subscriber = subscriber.with(extra_layer);

    let file_layer = if let Some(log_path) = config.log_path {
        let log_file =
            std::fs::File::create(log_path).map_err(|e| LogError::InitError(e.to_string()))?;
        Some(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(log_file)
                .with_filter(config.log_level),
        )
    } else {
        None
    };
    let subscriber = subscriber.with(file_layer);

    let opentelemetry_layer = if config.opentelemetry {
        let tracer_layer = init_tracer()?;
        Some(OpenTelemetryLayer::new(tracer_layer))
    } else {
        None
    };
    let subscriber = subscriber.with(opentelemetry_layer);

    let stderr_layer = if config.stderr {
        Some(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_filter(LevelFilter::ERROR),
        )
    } else {
        None
    };
    let subscriber = subscriber.with(stderr_layer);

    #[cfg(feature = "perf_timing")]
    let subscriber = subscriber.with(Some(crate::perf_timing::create_perf_layer()));
    #[cfg(not(feature = "perf_timing"))]
    let subscriber = subscriber.with(None::<EmptyLayer>);

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| LogError::InitError(e.to_string()))?;
    Ok(())
}
