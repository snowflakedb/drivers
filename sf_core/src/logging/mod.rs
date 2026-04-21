use std::path::PathBuf;
use std::sync::OnceLock;

pub use crate::logging::callback_layer::CLogCallback;
pub use crate::logging::callback_layer::CallbackLayer;
pub use crate::logging::error::LogError;
use crate::logging::opentelemetry::init_tracer;
use ::opentelemetry::trace::TracerProvider;
use tracing::Subscriber;
use tracing::level_filters::LevelFilter;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt;

/// Keeps the `SdkTracerProvider` alive for the process lifetime so the batch
/// exporter worker is not shut down when the local variable goes out of scope.
static TELEMETRY_PROVIDER: OnceLock<opentelemetry_sdk::trace::SdkTracerProvider> = OnceLock::new();

pub mod c_api;
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

impl<S: Subscriber> Layer<S> for EmptyLayer {}

pub fn init(config: LoggingConfig) -> Result<(), LogError> {
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
        Some(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(log_file)
                .with_filter(LevelFilter::INFO),
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

    let snowflake_layer = {
        let sessions = crate::telemetry::snowflake_exporter::global_session_registry();
        let exporter = crate::telemetry::snowflake_exporter::SnowflakeInBandExporter::new(sessions);
        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .build();
        let tracer = provider.tracer("snowflake.telemetry");
        // Keep the provider alive for the process lifetime so the batch
        // exporter worker thread is not shut down.
        TELEMETRY_PROVIDER.get_or_init(|| provider);
        OpenTelemetryLayer::new(tracer)
    };
    let subscriber = subscriber.with(Some(snowflake_layer));

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
