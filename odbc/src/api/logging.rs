use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use snafu::{Location, OptionExt, ResultExt, Snafu};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::Layer as SubscriberLayer;
use tracing_subscriber::{Registry, reload};

type BoxedLayer = Box<dyn SubscriberLayer<Registry> + Send + Sync>;
type InnerLayer = Option<BoxedLayer>;
type ReloadHandle = reload::Handle<InnerLayer, Registry>;

static LOG_HANDLE: OnceLock<ReloadHandle> = OnceLock::new();

#[allow(dead_code)]
pub(crate) struct OdbcLogConfig {
    pub log_path: Option<PathBuf>,
    pub log_level: Option<LevelFilter>,
    pub log_file_size_mb: Option<u64>,
    pub log_file_count: Option<u32>,
}

#[derive(Snafu, Debug)]
pub(crate) enum LoggingError {
    #[snafu(display("Failed to create log file"))]
    FileCreation {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to reload logging layer"))]
    Reload {
        source: reload::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Logging reload handle not initialized"))]
    HandleNotInitialized {
        #[snafu(implicit)]
        location: Location,
    },
}

/// Creates a reloadable logging layer that starts with no file output.
///
/// The returned layer should be passed to `sf_core::logging::init_logging` as
/// the `extra_layer`. The reload handle is stored in a process-wide static for
/// later use by [`reconfigure_logging`].
pub(crate) fn create_reload_layer() -> reload::Layer<InnerLayer, Registry> {
    let (layer, handle) = reload::Layer::new(None);
    if LOG_HANDLE.set(handle).is_err() {
        panic!("ODBC reload handle already initialized");
    }
    layer
}

/// Reconfigures the ODBC file logging layer based on DSN parameters.
///
/// When `log_path` is `Some`, creates an `fmt` layer writing to
/// `<log_path>/odbc.log` filtered at the requested level (defaulting to INFO).
/// When `log_path` is `None`, disables file logging.
pub(crate) fn reconfigure_logging(config: &OdbcLogConfig) -> Result<(), LoggingError> {
    let handle = LOG_HANDLE.get().context(HandleNotInitializedSnafu)?;

    let new_layer: InnerLayer = match &config.log_path {
        Some(path) => {
            let log_file_path = path.join("odbc.log");
            let file = std::fs::File::create(&log_file_path).context(FileCreationSnafu)?;
            let level = config.log_level.unwrap_or(LevelFilter::INFO);
            Some(Box::new(
                tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_writer(Mutex::new(file))
                    .with_filter(level),
            ))
        }
        None => None,
    };

    handle.reload(new_layer).context(ReloadSnafu)?;
    Ok(())
}
