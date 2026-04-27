use std::path::PathBuf;

pub use crate::logging::callback_layer::CLogCallback;
pub use crate::logging::callback_layer::CallbackLayer;
pub use crate::logging::error::LogError;
use crate::logging::opentelemetry::init_tracer;
use crate::telemetry::snowflake_exporter::SessionRegistry;
use ::opentelemetry::trace::TracerProvider;
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

/// Initialize logging without a telemetry session registry.
///
/// The Snowflake in-band telemetry layer is not installed. Callers that
/// need telemetry should use [`init_logging`] instead.
pub fn init(config: LoggingConfig) -> Result<(), LogError> {
    init_logging_inner::<EmptyLayer>(config, None, None).map(|_provider| ())
}

/// Initialize logging and return the Snowflake telemetry provider.
///
/// The caller is responsible for keeping the returned `SdkTracerProvider`
/// alive for the process lifetime (typically by storing it in the
/// `DatabaseDriverV1` via `DriverProviders`). Dropping the provider will
/// shut down the exporter.
pub fn init_logging<L>(
    config: LoggingConfig,
    extra_layer: Option<L>,
    telemetry_sessions: SessionRegistry,
) -> Result<opentelemetry_sdk::trace::SdkTracerProvider, LogError>
where
    L: Layer<Registry> + Send + Sync,
{
    init_logging_inner(config, extra_layer, Some(telemetry_sessions))
        .map(|provider| provider.expect("provider is always Some when sessions are provided"))
}

fn init_logging_inner<L>(
    config: LoggingConfig,
    extra_layer: Option<L>,
    telemetry_sessions: Option<SessionRegistry>,
) -> Result<Option<opentelemetry_sdk::trace::SdkTracerProvider>, LogError>
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

    let (snowflake_layer, provider) = if let Some(sessions) = telemetry_sessions {
        let exporter = crate::telemetry::snowflake_exporter::SnowflakeInBandExporter::new(sessions);
        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_simple_exporter(exporter)
            .build();
        let tracer = provider.tracer("snowflake.telemetry");
        // Only process "connection" spans (which carry snowflake.session.id)
        // and events within them, so the extra provider does minimal work for
        // non-telemetry code paths.
        let layer =
            OpenTelemetryLayer::new(tracer).with_filter(tracing_subscriber::filter::filter_fn(
                |metadata| metadata.name() == "connection" || metadata.is_event(),
            ));
        (Some(layer), Some(provider))
    } else {
        (None, None)
    };
    let subscriber = subscriber.with(snowflake_layer);

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
    Ok(provider)
}
