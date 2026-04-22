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

/// Force-flush all pending telemetry spans so they are exported before
/// a session is deregistered. No-op if the provider has not been initialized.
///
/// Uses `block_in_place` when on a multi-threaded Tokio runtime to avoid
/// stalling the executor. Bounded to 2 seconds so release can't hang.
pub fn flush_telemetry() {
    let Some(provider) = TELEMETRY_PROVIDER.get() else {
        return;
    };

    let do_flush = || {
        let _ = provider.force_flush();
    };

    if let Ok(handle) = tokio::runtime::Handle::try_current()
        && handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread
    {
        tokio::task::block_in_place(do_flush);
        return;
    }

    do_flush();
}

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
        // Build the provider once and keep it alive for the process lifetime.
        // get_or_init ensures the tracer always references the long-lived
        // instance, even if init_logging is called more than once.
        let provider = TELEMETRY_PROVIDER.get_or_init(|| {
            let sessions = crate::telemetry::snowflake_exporter::global_session_registry();
            let exporter =
                crate::telemetry::snowflake_exporter::SnowflakeInBandExporter::new(sessions);
            opentelemetry_sdk::trace::SdkTracerProvider::builder()
                .with_simple_exporter(exporter)
                .build()
        });
        let tracer = provider.tracer("snowflake.telemetry");
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
