use std::collections::HashMap;
use std::sync::Arc;

use opentelemetry::trace::TracerProvider;
use tracing::level_filters::LevelFilter;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, Registry};

use crate::fs_adapter::{FsAdapter, RealFs};
use crate::telemetry::os_details::detect_os_details;
use crate::telemetry::snowflake_exporter::SessionRegistry;

use super::error::{InitSnafu, LogError};
use super::{EmptyLayer, LoggingConfig};

type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

/// Telemetry and logging state created during logging initialisation.
///
/// Call exactly one of [`LogManager::init`], [`LogManager::with_app_sink`],
/// [`LogManager::for_odbc`], or [`LogManager::for_toml`] once per process.
/// The returned instance owns the `SdkTracerProvider`, `SessionRegistry`,
/// and lazily-computed OS details. Inject it into [`DatabaseDriverV1`] via
/// [`DriverProviders`].
pub struct LogManager {
    /// Kept alive so the Snowflake exporter is not shut down.
    #[allow(dead_code)]
    telemetry_provider: opentelemetry_sdk::trace::SdkTracerProvider,
    telemetry_sessions: SessionRegistry,
    os_details: once_cell::sync::OnceCell<Option<HashMap<String, String>>>,
    fs: Arc<dyn FsAdapter>,
}

impl LogManager {
    /// Returns the session registry shared with the Snowflake telemetry exporter.
    pub fn telemetry_sessions(&self) -> &SessionRegistry {
        &self.telemetry_sessions
    }

    /// Lazily detects and caches OS details (e.g. `/etc/os-release` on Linux).
    pub fn os_details(&self) -> &Option<HashMap<String, String>> {
        self.os_details
            .get_or_init(|| detect_os_details(self.fs.as_ref()))
    }

    /// Initialise logging with the given config, creating a fresh
    /// `SessionRegistry` so the Snowflake telemetry layer is always
    /// installed.
    pub fn init(config: LoggingConfig) -> Result<Self, LogError> {
        let sessions = SessionRegistry::default();
        let provider = Self::try_init(config, None::<EmptyLayer>, Some(sessions.clone()))?
            .expect("provider is always Some when sessions are provided");
        Ok(Self {
            telemetry_provider: provider,
            telemetry_sessions: sessions,
            os_details: once_cell::sync::OnceCell::new(),
            fs: Arc::new(RealFs),
        })
    }

    /// Initialise logging with an application-provided sink (e.g.
    /// `CallbackLayer`, `SFLoggerLayer`) that receives events in parallel
    /// with the core file layer.
    pub fn with_app_sink<L>(
        config: LoggingConfig,
        app_sink: L,
        registry: SessionRegistry,
    ) -> Result<Self, LogError>
    where
        L: Layer<Registry> + Send + Sync + 'static,
    {
        let provider = Self::try_init(config, Some(app_sink), Some(registry.clone()))?
            .expect("provider is always Some when registry is Some");
        Ok(Self {
            telemetry_provider: provider,
            telemetry_sessions: registry,
            os_details: once_cell::sync::OnceCell::new(),
            fs: Arc::new(RealFs),
        })
    }

    /// Factory: find and parse `sf.odbc.ini`, falling back to defaults.
    pub fn for_odbc() -> Option<Self> {
        let config = match super::ini_config::find_odbc_ini() {
            Some(path) => super::ini_config::parse_ini_file(&path).unwrap_or_else(|e| {
                eprintln!(
                    "Failed to parse sf.odbc.ini at {}: {e:?}, using defaults",
                    path.display()
                );
                LoggingConfig::default()
            }),
            None => LoggingConfig::default(),
        };
        match Self::init(config) {
            Ok(lm) => Some(lm),
            Err(e) => {
                eprintln!("Failed to initialize logging: {e:?}");
                None
            }
        }
    }

    /// Factory: load `[log]` section from `config.toml`, falling back to
    /// defaults.
    pub fn for_toml() -> Option<Self> {
        let config = match crate::config::config_manager::load_config_section("log") {
            Ok(Some(section)) => super::ini_config::load_from_toml_section(&section),
            _ => LoggingConfig::default(),
        };
        match Self::init(config) {
            Ok(lm) => Some(lm),
            Err(e) => {
                eprintln!("Failed to initialize logging: {e:?}");
                None
            }
        }
    }

    fn try_init<L>(
        config: LoggingConfig,
        app_sink: Option<L>,
        registry: Option<SessionRegistry>,
    ) -> Result<Option<opentelemetry_sdk::trace::SdkTracerProvider>, LogError>
    where
        L: Layer<Registry> + Send + Sync + 'static,
    {
        let mut layers: Vec<BoxedLayer> = Vec::new();

        layers.push(Self::build_core_layer(&config)?);

        if let Some(sink) = app_sink {
            layers.push(sink.boxed());
        }

        if config.open_telemetry {
            layers.push(OpenTelemetryLayer::new(super::opentelemetry::init_tracer()?).boxed());
        }

        let (snowflake_layer, provider) = if let Some(sessions) = registry {
            let exporter =
                crate::telemetry::snowflake_exporter::SnowflakeInBandExporter::new(sessions);
            let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
                .with_simple_exporter(exporter)
                .build();
            let tracer = provider.tracer("snowflake.telemetry");
            let layer =
                OpenTelemetryLayer::new(tracer).with_filter(tracing_subscriber::filter::filter_fn(
                    |metadata| metadata.name() == "connection" || metadata.is_event(),
                ));
            (Some(layer), Some(provider))
        } else {
            (None, None)
        };

        if let Some(layer) = snowflake_layer {
            layers.push(layer.boxed());
        }

        if config.stderr {
            layers.push(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stderr)
                    .with_filter(LevelFilter::ERROR)
                    .boxed(),
            );
        }

        #[cfg(feature = "perf_timing")]
        layers.push(crate::perf_timing::create_perf_layer().boxed());

        let subscriber = Registry::default().with(layers);

        tracing::subscriber::set_global_default(subscriber).map_err(|e| {
            InitSnafu {
                message: e.to_string(),
            }
            .build()
        })?;

        Ok(provider)
    }

    fn build_core_layer(config: &LoggingConfig) -> Result<BoxedLayer, LogError> {
        if !config.enabled {
            return Ok(EmptyLayer.boxed());
        }

        if let Some(ref log_path) = config.log_path {
            let file_name = config.log_file_name.as_deref().unwrap_or("sf_driver.log");

            let appender = tracing_appender::rolling::RollingFileAppender::builder()
                .rotation(tracing_appender::rolling::Rotation::NEVER)
                .filename_prefix(file_name)
                .build(log_path)
                .map_err(|e| {
                    InitSnafu {
                        message: format!("Failed to create log appender: {e}"),
                    }
                    .build()
                })?;

            Ok(tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(appender)
                .with_filter(config.level)
                .boxed())
        } else {
            Ok(EmptyLayer.boxed())
        }
    }
}
