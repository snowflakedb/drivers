pub use crate::logging::callback_layer::CLogCallback;
use crate::logging::callback_layer::CallbackLayer;
use crate::logging::error::LogError;
use crate::logging::opentelemetry::init_meter_provider;
use crate::logging::opentelemetry::init_tracer;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt;

mod callback_layer;
mod error;
mod opentelemetry;

pub struct LoggingConfig {
    c_callback: CLogCallback,
    log_file: String,
}

impl LoggingConfig {
    pub fn new(c_callback: CLogCallback, log_file: String) -> Self {
        Self {
            c_callback,
            log_file,
        }
    }
}

/*
    TODO:
    - Configure logging dynamically
    - Disable opentelemetry if not needed
*/

pub fn init_logging(config: LoggingConfig) -> Result<(), LogError> {
    let subscriber = Registry::default();

    let log_file =
        std::fs::File::create(config.log_file).map_err(|e| LogError::InitError(e.to_string()))?;
    let subscriber = subscriber.with(tracing_subscriber::fmt::layer().with_writer(log_file));

    let subscriber = subscriber.with(CallbackLayer::new(config.c_callback));

    let meter_provider = init_meter_provider()?;
    let subscriber = subscriber.with(MetricsLayer::new(meter_provider));

    let tracer_layer = init_tracer()?;
    let subscriber = subscriber.with(OpenTelemetryLayer::new(tracer_layer));

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| LogError::InitError(e.to_string()))
}
