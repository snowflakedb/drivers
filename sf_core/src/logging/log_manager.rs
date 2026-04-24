use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, Registry};

use super::error::{InitSnafu, LogError};
use super::{EmptyLayer, LogConfig};

type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

/// Namespace for logging initialisation.
///
/// Call exactly one of [`LogManager::init`], [`LogManager::with_app_sink`],
/// [`LogManager::for_odbc`], or [`LogManager::for_toml`] once per process.
/// If initialisation fails the global tracing subscriber stays on its
/// default no-op state.
pub struct LogManager {
    _private: (),
}

impl LogManager {
    /// Initialise logging with the given config and no application sink.
    pub fn init(config: LogConfig) {
        Self::do_init(config, None::<EmptyLayer>);
    }

    /// Initialise logging with an application-provided sink (e.g.
    /// `CallbackLayer`, `SFLoggerLayer`) that receives events in parallel
    /// with the core file layer.
    pub fn with_app_sink<L>(config: LogConfig, app_sink: L)
    where
        L: Layer<Registry> + Send + Sync + 'static,
    {
        Self::do_init(config, Some(app_sink));
    }

    /// Factory: find and parse `sf.odbc.ini`, falling back to defaults.
    pub fn for_odbc() {
        let config = match super::ini_config::find_odbc_ini() {
            Some(path) => super::ini_config::parse_ini_file(&path).unwrap_or_else(|e| {
                eprintln!(
                    "Failed to parse sf.odbc.ini at {}: {e:?}, using defaults",
                    path.display()
                );
                LogConfig::default()
            }),
            None => LogConfig::default(),
        };
        Self::init(config);
    }

    /// Factory: load `[log]` section from `config.toml`, falling back to
    /// defaults.
    pub fn for_toml() {
        let config = match crate::config::config_manager::load_config_section("log") {
            Ok(Some(section)) => super::ini_config::load_from_toml_section(&section),
            _ => LogConfig::default(),
        };
        Self::init(config);
    }

    fn do_init<L>(config: LogConfig, app_sink: Option<L>)
    where
        L: Layer<Registry> + Send + Sync + 'static,
    {
        if let Err(e) = Self::try_init(config, app_sink) {
            eprintln!("Failed to initialize logging: {e:?}");
        }
    }

    fn try_init<L>(config: LogConfig, app_sink: Option<L>) -> Result<(), LogError>
    where
        L: Layer<Registry> + Send + Sync + 'static,
    {
        let mut layers: Vec<BoxedLayer> = Vec::new();

        layers.push(Self::build_core_layer(&config)?);

        if let Some(sink) = app_sink {
            layers.push(sink.boxed());
        }

        if config.opentelemetry {
            layers.push(OpenTelemetryLayer::new(super::opentelemetry::init_tracer()?).boxed());
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

        Ok(())
    }

    fn build_core_layer(config: &LogConfig) -> Result<BoxedLayer, LogError> {
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
