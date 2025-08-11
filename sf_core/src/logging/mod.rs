use std::path::PathBuf;

pub use crate::logging::callback_layer::CLogCallback;
pub use crate::logging::callback_layer::CallbackLayer;
pub use crate::logging::error::LogError;
use crate::logging::opentelemetry::init_tracer;
use tracing::level_filters::LevelFilter;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt;

mod callback_layer;
mod error;
mod opentelemetry;

pub struct LoggingConfig {
    pub log_file: Option<PathBuf>,
    pub stderr: bool,
    pub opentelemetry: bool,
}

impl LoggingConfig {
    pub fn new(log_file: Option<PathBuf>, stderr: bool, opentelemetry: bool) -> Self {
        Self {
            log_file,
            stderr,
            opentelemetry,
        }
    }
}

struct EmptyLayer;

impl Layer<Registry> for EmptyLayer {}

pub fn init(config: LoggingConfig) -> Result<(), LogError> {
    eprintln!("Initializing logging");
    init_logging::<EmptyLayer>(config, None)
}

pub fn init_logging<L>(config: LoggingConfig, extra_layer: Option<L>) -> Result<(), LogError>
where
    L: Layer<Registry> + Send + Sync,
{
    let subscriber = Registry::default();
    let subscriber = subscriber.with(extra_layer);

    let file_layer = if let Some(log_file) = config.log_file {
        let log_file =
            std::fs::File::create(log_file).map_err(|e| LogError::InitError(e.to_string()))?;
        Some(tracing_subscriber::fmt::layer().with_writer(log_file))
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
                .with_filter(LevelFilter::DEBUG),
        )
    } else {
        None
    };
    let subscriber = subscriber.with(stderr_layer);

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| LogError::InitError(e.to_string()))?;
    Ok(())
}
