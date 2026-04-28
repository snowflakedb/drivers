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

/// Configuration for the logging subsystem.
pub struct LoggingConfig {
    pub enabled: bool,
    pub level: LevelFilter,
    pub log_path: Option<PathBuf>,
    pub log_file_name: Option<String>,
    pub max_file_size: Option<u64>,
    pub max_file_count: Option<u32>,
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
            open_telemetry: false,
            stderr: false,
        }
    }
}

pub(crate) struct EmptyLayer;

impl<S: Subscriber> Layer<S> for EmptyLayer {}
