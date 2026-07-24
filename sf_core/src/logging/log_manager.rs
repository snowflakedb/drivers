use std::collections::HashMap;
use std::sync::Arc;

use opentelemetry::trace::TracerProvider;
use tracing::Level;
use tracing::level_filters::LevelFilter;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, Registry};

use crate::env_vars;
use crate::fs_adapter::{FsAdapter, RealFs};
use crate::telemetry::os_details::detect_os_details;
use crate::telemetry::snowflake_exporter::SessionRegistry;
use crate::telemetry::snowflake_processor::SessionFlushHandle;

use super::error::{InitSnafu, LogError};
use super::{EmptyLayer, LoggingConfig};

type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

/// Wraps the composed tracing subscriber and owns the `SdkTracerProvider`,
/// tying the OTEL exporter lifetime to the subscriber. Lives inside a
/// [`tracing::dispatcher::Dispatch`] (which `Arc`-wraps it); the provider
/// stays alive as long as any `Dispatch` clone exists.
struct DriverSubscriber {
    #[allow(dead_code)]
    telemetry_provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
    inner: Box<dyn tracing::Subscriber + Send + Sync>,
}

impl tracing::Subscriber for DriverSubscriber {
    fn register_callsite(
        &self,
        metadata: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        self.inner.register_callsite(metadata)
    }

    fn max_level_hint(&self) -> Option<tracing::level_filters::LevelFilter> {
        self.inner.max_level_hint()
    }

    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        self.inner.enabled(metadata)
    }

    fn new_span(&self, attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        self.inner.new_span(attrs)
    }

    fn record(&self, span: &tracing::span::Id, values: &tracing::span::Record<'_>) {
        self.inner.record(span, values);
    }

    fn record_follows_from(&self, span: &tracing::span::Id, follows: &tracing::span::Id) {
        self.inner.record_follows_from(span, follows);
    }

    fn event_enabled(&self, event: &tracing::Event<'_>) -> bool {
        self.inner.event_enabled(event)
    }

    fn event(&self, event: &tracing::Event<'_>) {
        self.inner.event(event);
    }

    fn enter(&self, span: &tracing::span::Id) {
        self.inner.enter(span);
    }

    fn exit(&self, span: &tracing::span::Id) {
        self.inner.exit(span);
    }

    fn clone_span(&self, id: &tracing::span::Id) -> tracing::span::Id {
        self.inner.clone_span(id)
    }

    fn try_close(&self, id: tracing::span::Id) -> bool {
        self.inner.try_close(id)
    }

    fn current_span(&self) -> tracing_core::span::Current {
        self.inner.current_span()
    }

    fn on_register_dispatch(&self, subscriber: &tracing::dispatcher::Dispatch) {
        self.inner.on_register_dispatch(subscriber);
    }

    unsafe fn downcast_raw(&self, id: std::any::TypeId) -> Option<*const ()> {
        if id == std::any::TypeId::of::<Self>() {
            Some(self as *const Self as *const ())
        } else {
            // SAFETY: delegating to inner subscriber which upholds the same
            // contract — returned pointer is valid for the type identified by `id`.
            unsafe { self.inner.downcast_raw(id) }
        }
    }
}

/// Telemetry and logging state created during logging initialisation.
///
/// Call exactly one of [`LogManager::init`], [`LogManager::with_app_sink`],
/// [`LogManager::for_odbc`], or [`LogManager::for_toml`] per driver
/// lifecycle. The returned instance holds a [`tracing::dispatcher::Dispatch`]
/// that consumers install per-thread via
/// [`tracing::dispatcher::set_default`], the `SessionRegistry`, and
/// lazily-computed OS details. Inject it into [`DatabaseDriverV1`] via
/// [`DriverProviders`].
pub struct LogManager {
    dispatch: tracing::dispatcher::Dispatch,
    telemetry_sessions: SessionRegistry,
    session_flusher: Option<SessionFlushHandle>,
    os_details: once_cell::sync::OnceCell<Option<HashMap<String, String>>>,
    fs: Arc<dyn FsAdapter>,
    /// Process-wide default for `log_query_text` parsed from `sf.odbc.ini` /
    /// `[log]` TOML. `None` means "unset; let the registry default win".
    log_query_text: Option<bool>,
    /// Process-wide default for `log_query_parameters`. See
    /// [`Self::log_query_text`].
    log_query_parameters: Option<bool>,
    error_trace_enabled: bool,
    /// Whether troubleshooting mode is active. Resolved once at init from
    /// `SNOWFLAKE_TROUBLESHOOTING_ENABLED` env var; immutable thereafter.
    troubleshooting: bool,
    /// Resolved once at construction so all consumers (file layer, diagnostic
    /// runner) agree on the directory even if the env var changes mid-process.
    troubleshooting_log_dir: std::path::PathBuf,
}

impl LogManager {
    /// Returns the session registry shared with the Snowflake telemetry exporter.
    pub fn telemetry_sessions(&self) -> &SessionRegistry {
        &self.telemetry_sessions
    }

    /// Flush buffered telemetry spans for a specific session.
    /// Called during connection release before the connection span is dropped.
    /// Awaits the export so it completes while session tokens are still alive
    /// (see [`crate::telemetry::snowflake_processor::SessionFlushHandle::flush_session`]).
    pub async fn flush_session(&self, session_id: i64) {
        if let Some(ref flusher) = self.session_flusher {
            flusher.flush_session(session_id).await;
        }
    }

    /// Lazily detects and caches OS details (e.g. `/etc/os-release` on Linux).
    pub fn os_details(&self) -> &Option<HashMap<String, String>> {
        self.os_details
            .get_or_init(|| detect_os_details(self.fs.as_ref()))
    }

    /// Process-wide default for `log_query_text`, parsed from `sf.odbc.ini`
    /// (or the `[log]` TOML section). Acts as a fallback when no
    /// per-connection setting is supplied; explicit DSN / connection-string
    /// values still win.
    pub fn log_query_text(&self) -> Option<bool> {
        self.log_query_text
    }

    /// Process-wide default for `log_query_parameters`. See
    /// [`Self::log_query_text`] for precedence semantics.
    pub fn log_query_parameters(&self) -> Option<bool> {
        self.log_query_parameters
    }

    /// Whether user-facing error messages should include the full error trace,
    /// as parsed from the INI/TOML logging config. Consumer crates read this
    /// during init to seed their own rendering-policy state.
    pub fn error_trace_enabled(&self) -> bool {
        self.error_trace_enabled
    }

    /// Whether troubleshooting mode is active (resolved at init from
    /// `SNOWFLAKE_TROUBLESHOOTING_ENABLED` env var).
    pub fn is_troubleshooting(&self) -> bool {
        self.troubleshooting
    }

    /// Returns the resolved troubleshooting log path when troubleshooting is
    /// active, `None` otherwise.  Used as a fallback by `DiagnosticRunner`
    /// so the SnowCD report lands in the same directory as troubleshooting logs.
    pub fn troubleshooting_path(&self) -> Option<std::path::PathBuf> {
        if self.troubleshooting {
            Some(self.troubleshooting_log_dir.clone())
        } else {
            None
        }
    }

    /// The [`tracing::dispatcher::Dispatch`] wrapping the configured
    /// subscriber. Consumers install this per-thread via
    /// [`tracing::dispatcher::set_default`] or propagate it to spawned
    /// futures with `.with_current_subscriber()`.
    pub fn dispatch(&self) -> &tracing::dispatcher::Dispatch {
        &self.dispatch
    }

    /// Create a `LogManager` with a no-op tracing subscriber.
    ///
    /// The returned instance still provides a `SessionRegistry` and
    /// lazily-detected OS details via the given `fs`. The dispatch wraps
    /// a no-op subscriber so installing it is harmless. Use this when
    /// the real subscriber is managed externally (e.g. the host
    /// application or test harness already configures tracing).
    pub fn with_none_subscriber(fs: Arc<dyn FsAdapter>) -> Self {
        let noop = tracing::dispatcher::Dispatch::none();
        Self {
            dispatch: noop,
            telemetry_sessions: SessionRegistry::default(),
            session_flusher: None,
            os_details: once_cell::sync::OnceCell::new(),
            fs,
            log_query_text: None,
            log_query_parameters: None,
            error_trace_enabled: LoggingConfig::default().error_trace_enabled,
            troubleshooting: false,
            troubleshooting_log_dir: Self::resolve_troubleshooting_log_path(),
        }
    }

    /// Override the cached `log_query_text` / `log_query_parameters` defaults
    /// without re-installing a tracing subscriber. Test-only ergonomics: lets
    /// integration tests build a `LogManager` via [`Self::with_none_subscriber`]
    /// and still simulate values parsed from `sf.odbc.ini`.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn with_query_log_defaults(
        mut self,
        log_query_text: Option<bool>,
        log_query_parameters: Option<bool>,
    ) -> Self {
        self.log_query_text = log_query_text;
        self.log_query_parameters = log_query_parameters;
        self
    }

    /// Initialise logging with the given config, creating a fresh
    /// `SessionRegistry` so the Snowflake telemetry layer is always
    /// installed.
    pub fn init(config: LoggingConfig) -> Result<Self, LogError> {
        let sessions = SessionRegistry::default();
        let log_query_text = config.log_query_text;
        let log_query_parameters = config.log_query_parameters;
        let error_trace_enabled = config.error_trace_enabled;
        let troubleshooting_log_dir = Self::resolve_troubleshooting_log_path();
        let troubleshooting = Self::resolve_troubleshooting();
        let (dispatch, flusher) = Self::try_init(
            config,
            None::<EmptyLayer>,
            Some(sessions.clone()),
            troubleshooting,
            &troubleshooting_log_dir,
        )?;
        let lm = Self {
            dispatch,
            telemetry_sessions: sessions,
            session_flusher: flusher,
            os_details: once_cell::sync::OnceCell::new(),
            fs: Arc::new(RealFs),
            log_query_text,
            log_query_parameters,
            error_trace_enabled,
            troubleshooting,
            troubleshooting_log_dir,
        };
        Ok(lm)
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
        let log_query_text = config.log_query_text;
        let log_query_parameters = config.log_query_parameters;
        let error_trace_enabled = config.error_trace_enabled;
        let troubleshooting_log_dir = Self::resolve_troubleshooting_log_path();
        let troubleshooting = Self::resolve_troubleshooting();
        let (dispatch, flusher) = Self::try_init(
            config,
            Some(app_sink),
            Some(registry.clone()),
            troubleshooting,
            &troubleshooting_log_dir,
        )?;
        let lm = Self {
            dispatch,
            telemetry_sessions: registry,
            session_flusher: flusher,
            os_details: once_cell::sync::OnceCell::new(),
            fs: Arc::new(RealFs),
            log_query_text,
            log_query_parameters,
            error_trace_enabled,
            troubleshooting,
            troubleshooting_log_dir,
        };
        Ok(lm)
    }

    /// Factory: derive a [`LoggingConfig`] from the process-wide INI
    /// snapshot loaded via [`crate::config::load_ini_files`] and initialise
    /// logging with it.
    ///
    /// Recoverable failures degrade to [`LoggingConfig::default`] with a
    /// diagnostic on stderr: a missing snapshot (caller forgot to invoke
    /// `load_ini_files`, or that call returned an error and left the global
    /// uninitialised), a parse failure inside `logging_config_from_ini`, or
    /// a recognised key carrying an invalid value all surface as a
    /// `Failed to derive logging config from sf.odbc.ini: ...` line so the
    /// silent fallback is at least visible.
    pub fn for_odbc() -> Option<Self> {
        let config = match crate::config::get_ini_config() {
            Some(ini) => crate::config::logging_config_from_ini(ini).unwrap_or_else(|e| {
                eprintln!(
                    "Failed to derive logging config from sf.odbc.ini: {e:?}; using defaults"
                );
                LoggingConfig::default()
            }),
            None => {
                // The ODBC wrapper is expected to call `load_ini_files` before
                // `for_odbc`. Reaching this arm means either the wrapper
                // skipped that step or `load_ini_files` itself failed; the
                // wrapper already emits its own diagnostic in the latter case,
                // so a second line here is redundant but harmless.
                eprintln!(
                    "No sf.odbc.ini snapshot available (sf_core::config::load_ini_files \
                     was not called or did not complete successfully); using default \
                     logging configuration"
                );
                LoggingConfig::default()
            }
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
        let config = match crate::config::config_manager::load_config_section(
            "log",
            crate::config::toml_loader::FilePermissionCheck::Enabled,
        ) {
            Ok(Some(section)) => crate::config::logging_config_from_toml_section(&section),
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

    fn resolve_troubleshooting() -> bool {
        std::env::var(env_vars::SNOWFLAKE_TROUBLESHOOTING_ENABLED)
            .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
            .unwrap_or(false)
    }

    fn resolve_troubleshooting_log_path() -> std::path::PathBuf {
        std::env::var(env_vars::SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| ".".into()))
    }

    fn try_init<L>(
        config: LoggingConfig,
        app_sink: Option<L>,
        registry: Option<SessionRegistry>,
        troubleshooting: bool,
        troubleshooting_log_dir: &std::path::Path,
    ) -> Result<(tracing::dispatcher::Dispatch, Option<SessionFlushHandle>), LogError>
    where
        L: Layer<Registry> + Send + Sync + 'static,
    {
        let mut layers: Vec<BoxedLayer> = Vec::new();

        layers.push(Self::build_core_layer(&config)?);

        if troubleshooting {
            layers.push(Self::build_troubleshooting_layer(troubleshooting_log_dir)?);
        }

        if let Some(sink) = app_sink {
            layers.push(sink.boxed());
        }

        if config.open_telemetry {
            layers.push(OpenTelemetryLayer::new(super::opentelemetry::init_tracer()?).boxed());
        }

        let (snowflake_layer, provider, flush_handle) = if let Some(sessions) = registry {
            let exporter =
                crate::telemetry::snowflake_exporter::SnowflakeInBandExporter::new(sessions);
            let (processor, flush_handle) =
                crate::telemetry::snowflake_processor::SnowflakeSpanProcessor::new(exporter);
            let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
                .with_span_processor(processor)
                .build();
            let tracer = provider.tracer("snowflake.telemetry");
            // Restrict the OpenTelemetryLayer to spans emitted by sf_core itself.
            // Without this filter every span from tokio, hyper, tower, tonic, …
            // would flow through `SnowflakeSpanProcessor::on_end`, take the
            // per-session buffers mutex, scan attributes for `snowflake.session.id`
            // (always absent), and be dropped — pure overhead on the hot path.
            let layer = OpenTelemetryLayer::new(tracer).with_filter(
                tracing_subscriber::filter::Targets::new().with_target("sf_core", Level::TRACE),
            );
            (Some(layer), Some(provider), Some(flush_handle))
        } else {
            (None, None, None)
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

        let driver_subscriber = DriverSubscriber {
            telemetry_provider: provider,
            inner: Box::new(subscriber),
        };

        Ok((
            tracing::dispatcher::Dispatch::new(driver_subscriber),
            flush_handle,
        ))
    }

    fn build_troubleshooting_layer(log_path: &std::path::Path) -> Result<BoxedLayer, LogError> {
        if let Err(e) = std::fs::create_dir_all(log_path) {
            eprintln!(
                "SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH: failed to create log directory {}: {e}",
                log_path.display()
            );
            return Ok(EmptyLayer.boxed());
        }
        // Rotation::NEVER → single file, no date/sequence suffix appended.
        let appender = tracing_appender::rolling::RollingFileAppender::builder()
            .rotation(tracing_appender::rolling::Rotation::NEVER)
            .filename_prefix("sf_driver_troubleshooting.log")
            .build(log_path)
            .map_err(|e| {
                InitSnafu {
                    message: format!("Failed to create troubleshooting log appender: {e}"),
                }
                .build()
            })?;

        let layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(appender);

        Ok(layer.boxed())
    }

    fn build_core_layer(config: &LoggingConfig) -> Result<BoxedLayer, LogError> {
        if !config.enabled {
            return Ok(EmptyLayer.boxed());
        }

        if let Some(ref log_path) = config.log_path {
            let file_name = config.log_file_name.as_deref().unwrap_or("sf_driver.log");

            // When max_file_count is set but rotation is Never, default to
            // Daily so that file pruning actually takes effect (rotation is a
            // prerequisite for max_log_files in tracing-appender).
            let effective_rotation = if config.max_file_count.is_some()
                && config.rotation == super::LogRotation::Never
            {
                super::LogRotation::Daily
            } else {
                config.rotation
            };

            let mut builder = tracing_appender::rolling::RollingFileAppender::builder()
                .rotation(effective_rotation.to_appender_rotation())
                .filename_prefix(file_name);

            if let Some(count) = config.max_file_count {
                builder = builder.max_log_files(count as usize);
            }

            let appender = builder.build(log_path).map_err(|e| {
                InitSnafu {
                    message: format!("Failed to create log appender: {e}"),
                }
                .build()
            })?;

            if config.max_file_size.is_some() {
                eprintln!(
                    "WARNING: max_file_size is configured but size-based log rotation is not \
                     yet supported; the setting will be ignored"
                );
            }

            let fmt_layer = tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(appender);

            Ok(fmt_layer.with_filter(config.level).boxed())
        } else {
            Ok(EmptyLayer.boxed())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_core_layer_disabled_returns_empty() {
        let config = LoggingConfig {
            enabled: false,
            ..LoggingConfig::default()
        };
        assert!(LogManager::build_core_layer(&config).is_ok());
    }

    #[test]
    fn build_core_layer_no_path_returns_empty() {
        let config = LoggingConfig {
            enabled: true,
            log_path: None,
            ..LoggingConfig::default()
        };
        assert!(LogManager::build_core_layer(&config).is_ok());
    }

    #[test]
    fn build_core_layer_with_path_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let config = LoggingConfig {
            enabled: true,
            log_path: Some(dir.path().to_path_buf()),
            ..LoggingConfig::default()
        };
        assert!(LogManager::build_core_layer(&config).is_ok());
    }

    #[test]
    fn build_core_layer_with_custom_file_name() {
        let dir = tempfile::tempdir().unwrap();
        let config = LoggingConfig {
            enabled: true,
            log_path: Some(dir.path().to_path_buf()),
            log_file_name: Some("custom.log".to_string()),
            ..LoggingConfig::default()
        };
        assert!(
            LogManager::build_core_layer(&config).is_ok(),
            "build_core_layer should succeed with a custom file name"
        );
    }

    #[test]
    fn build_core_layer_respects_level_filter() {
        use tracing::level_filters::LevelFilter;

        let dir = tempfile::tempdir().unwrap();
        for level in [
            LevelFilter::OFF,
            LevelFilter::ERROR,
            LevelFilter::WARN,
            LevelFilter::INFO,
            LevelFilter::DEBUG,
            LevelFilter::TRACE,
        ] {
            let config = LoggingConfig {
                enabled: true,
                log_path: Some(dir.path().to_path_buf()),
                level,
                ..LoggingConfig::default()
            };
            assert!(
                LogManager::build_core_layer(&config).is_ok(),
                "build_core_layer should succeed for level {level:?}"
            );
        }
    }

    #[test]
    fn with_none_subscriber_log_query_defaults_are_none() {
        let lm = LogManager::with_none_subscriber(Arc::new(RealFs));
        assert_eq!(lm.log_query_text(), None);
        assert_eq!(lm.log_query_parameters(), None);
    }

    #[test]
    fn with_query_log_defaults_overrides_values() {
        let lm = LogManager::with_none_subscriber(Arc::new(RealFs))
            .with_query_log_defaults(Some(true), Some(false));
        assert_eq!(lm.log_query_text(), Some(true));
        assert_eq!(lm.log_query_parameters(), Some(false));
    }

    #[test]
    fn with_query_log_defaults_supports_partial_set() {
        let lm = LogManager::with_none_subscriber(Arc::new(RealFs))
            .with_query_log_defaults(Some(true), None);
        assert_eq!(lm.log_query_text(), Some(true));
        assert_eq!(lm.log_query_parameters(), None);
    }

    #[test]
    fn default_config_is_enabled_info_no_path() {
        use tracing::level_filters::LevelFilter;

        let config = LoggingConfig::default();
        assert!(config.enabled, "default config should be enabled");
        assert_eq!(
            config.level,
            LevelFilter::INFO,
            "default level should be INFO"
        );
        assert!(config.log_path.is_none(), "default log_path should be None");
        assert!(
            config.log_file_name.is_none(),
            "default log_file_name should be None"
        );
        assert!(
            !config.open_telemetry,
            "default opentelemetry should be false"
        );
    }

    #[test]
    fn with_none_subscriber_exposes_session_registry() {
        let lm = LogManager::with_none_subscriber(Arc::new(crate::fs_adapter::RealFs));
        let guard = lm.telemetry_sessions().read().unwrap();
        assert!(guard.is_empty(), "session registry should start empty");
    }

    #[tokio::test]
    async fn flush_session_is_noop_without_flusher() {
        let lm = LogManager::with_none_subscriber(Arc::new(crate::fs_adapter::RealFs));
        // session_flusher is None for with_none_subscriber — should not panic
        lm.flush_session(42).await;
    }
}
