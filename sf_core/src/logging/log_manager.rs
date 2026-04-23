use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use once_cell::sync::OnceCell;
use tracing::level_filters::LevelFilter;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, Registry};

use crate::logging::LoggingConfig;
use crate::logging::callback_layer::{CLogCallback, CallbackLayer, CallbackState};
use crate::logging::error::LogError;
use crate::logging::event_sanitizer::EventSanitizerLayer;
use crate::logging::opentelemetry::init_tracer;
use crate::logging::rolling_writer::RollingFileWriter;

static INSTANCE: OnceCell<LogManager> = OnceCell::new();

/// Process-global logging manager that owns dynamic handles for runtime
/// reconfiguration of log levels and callback subscriptions.
///
/// Only the first [`init`](LogManager::init) or
/// [`init_with_layer`](LogManager::init_with_layer) call takes effect;
/// subsequent calls return the existing singleton. Use [`set_level`],
/// [`subscribe_wrapper`], or [`disable`] for runtime changes.
///
/// [`set_level`]: LogManager::set_level
/// [`subscribe_wrapper`]: LogManager::subscribe_wrapper
/// [`disable`]: LogManager::disable
pub struct LogManager {
    file_level: SharedLevelFilter,
    callback_state: Arc<CallbackState>,
    callback_level: SharedLevelFilter,
}

impl LogManager {
    /// Initialise the global `LogManager` with the given configuration.
    pub fn init(config: LoggingConfig) -> Result<&'static Self, LogError> {
        Self::init_with_layer::<super::EmptyLayer>(config, None)
    }

    /// Initialise with an additional wrapper-specific layer (e.g. JDBC SLF4J
    /// bridge). The extra layer is composed into the subscriber below the
    /// callback and file layers.
    pub fn init_with_layer<L>(
        config: LoggingConfig,
        extra: Option<L>,
    ) -> Result<&'static Self, LogError>
    where
        L: Layer<Registry> + Send + Sync + 'static,
    {
        INSTANCE.get_or_try_init(|| Self::build(config, extra))
    }

    /// Returns the singleton if it has been initialised.
    pub fn get() -> Option<&'static Self> {
        INSTANCE.get()
    }

    /// Change the file logger's level filter at runtime.
    pub fn set_level(&self, level: LevelFilter) {
        self.file_level.set(level);
    }

    /// Register a C-ABI log callback with its own independent level filter.
    pub fn subscribe_wrapper(&self, cb: CLogCallback, level: LevelFilter) {
        self.callback_state.set_legacy(cb);
        self.callback_level.set(level);
    }

    /// Disable all logging output (file and callback).
    pub fn disable(&self) {
        self.file_level.set(LevelFilter::OFF);
        self.callback_level.set(LevelFilter::OFF);
    }

    fn build<L>(config: LoggingConfig, extra_layer: Option<L>) -> Result<Self, LogError>
    where
        L: Layer<Registry> + Send + Sync + 'static,
    {
        let file_level = SharedLevelFilter::new(if config.enabled {
            config.log_level
        } else {
            LevelFilter::OFF
        });
        let callback_state = Arc::new(CallbackState::new());
        let callback_level = SharedLevelFilter::new(LevelFilter::OFF);

        let subscriber = Registry::default();

        let subscriber = subscriber.with(extra_layer);

        let callback_layer =
            CallbackLayer::from_shared(callback_state.clone()).with_filter(callback_level.clone());
        let subscriber = subscriber.with(callback_layer);

        let file_layer = if config.enabled {
            if let Some(ref log_path) = config.log_path {
                let writer =
                    RollingFileWriter::new(log_path, config.log_file_size, config.log_file_count)
                        .map_err(|e| LogError::InitError(e.to_string()))?;
                Some(
                    tracing_subscriber::fmt::layer()
                        .with_ansi(false)
                        .with_writer(writer)
                        .with_filter(file_level.clone()),
                )
            } else {
                None
            }
        } else {
            None
        };
        let subscriber = subscriber.with(file_layer);

        let otel_layer = if config.enabled && config.opentelemetry {
            Some(OpenTelemetryLayer::new(init_tracer()?))
        } else {
            None
        };
        let subscriber = subscriber.with(otel_layer);

        let stderr_layer = if config.enabled && config.stderr {
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
        let subscriber = subscriber.with(None::<super::EmptyLayer>);

        let subscriber = subscriber.with(EventSanitizerLayer::new());

        tracing::subscriber::set_global_default(subscriber)
            .map_err(|e| LogError::InitError(e.to_string()))?;

        Ok(Self {
            file_level,
            callback_state,
            callback_level,
        })
    }
}

// ---------------------------------------------------------------------------
// SharedLevelFilter — atomic level gate usable as a per-layer Filter
// ---------------------------------------------------------------------------

/// An atomically-updatable level filter that can be shared between a
/// [`LogManager`] handle and one or more tracing layers.
///
/// Implements [`tracing_subscriber::layer::Filter`] so it can be used with
/// [`Layer::with_filter`] for per-layer filtering.
#[derive(Clone)]
pub(crate) struct SharedLevelFilter {
    value: Arc<AtomicU8>,
}

impl SharedLevelFilter {
    pub fn new(level: LevelFilter) -> Self {
        Self {
            value: Arc::new(AtomicU8::new(encode_level_filter(level))),
        }
    }

    pub fn set(&self, level: LevelFilter) {
        self.value
            .store(encode_level_filter(level), Ordering::Release);
        tracing::callsite::rebuild_interest_cache();
    }

    pub fn get(&self) -> LevelFilter {
        decode_level_filter(self.value.load(Ordering::Acquire))
    }
}

impl<S> tracing_subscriber::layer::Filter<S> for SharedLevelFilter {
    fn enabled(
        &self,
        meta: &tracing::Metadata<'_>,
        _cx: &tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        encode_level(meta.level()) <= self.value.load(Ordering::Relaxed)
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(self.get())
    }
}

// Encoding follows the Simba DSI / ODBC numeric scale (higher = more verbose):
//
// | Value | Simba DSI    | tracing        |
// |-------|--------------|----------------|
// |   0   | OFF          | `LevelFilter::OFF`   |
// |   1   | FATAL        | — (unused)     |
// |   2   | ERROR        | `ERROR`        |
// |   3   | WARNING      | `WARN`         |
// |   4   | INFO         | `INFO`         |
// |   5   | DEBUG        | `DEBUG`        |
// |   6   | TRACE        | `TRACE`        |
//
// The filter check is: event_encoded <= filter_encoded
// (an event passes when its verbosity doesn't exceed the threshold).

fn encode_level_filter(lf: LevelFilter) -> u8 {
    match lf.into_level() {
        Some(level) => encode_level(&level),
        None => 0, // OFF
    }
}

fn decode_level_filter(v: u8) -> LevelFilter {
    match v {
        0 => LevelFilter::OFF,
        1 | 2 => LevelFilter::ERROR, // FATAL (1) collapses into ERROR
        3 => LevelFilter::WARN,
        4 => LevelFilter::INFO,
        5 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    }
}

fn encode_level(level: &tracing::Level) -> u8 {
    match *level {
        tracing::Level::ERROR => 2,
        tracing::Level::WARN => 3,
        tracing::Level::INFO => 4,
        tracing::Level::DEBUG => 5,
        tracing::Level::TRACE => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- SharedLevelFilter ---------------------------------------------------

    #[test]
    fn shared_level_filter_roundtrips() {
        for &lf in &[
            LevelFilter::TRACE,
            LevelFilter::DEBUG,
            LevelFilter::INFO,
            LevelFilter::WARN,
            LevelFilter::ERROR,
            LevelFilter::OFF,
        ] {
            let f = SharedLevelFilter::new(lf);
            assert_eq!(f.get(), lf, "roundtrip failed for {lf:?}");
        }
    }

    #[test]
    fn shared_level_filter_set_updates_value() {
        let f = SharedLevelFilter::new(LevelFilter::INFO);
        assert_eq!(f.get(), LevelFilter::INFO);
        f.set(LevelFilter::DEBUG);
        assert_eq!(f.get(), LevelFilter::DEBUG);
    }

    #[test]
    fn shared_level_filter_clones_share_state() {
        let f1 = SharedLevelFilter::new(LevelFilter::WARN);
        let f2 = f1.clone();
        f1.set(LevelFilter::TRACE);
        assert_eq!(f2.get(), LevelFilter::TRACE);
    }

    // -- encoding helpers ----------------------------------------------------

    #[test]
    fn encode_decode_level_filter_all_variants() {
        let variants = [
            LevelFilter::TRACE,
            LevelFilter::DEBUG,
            LevelFilter::INFO,
            LevelFilter::WARN,
            LevelFilter::ERROR,
            LevelFilter::OFF,
        ];
        for &lf in &variants {
            assert_eq!(decode_level_filter(encode_level_filter(lf)), lf);
        }
    }

    #[test]
    fn encode_level_matches_odbc_convention() {
        assert_eq!(encode_level(&tracing::Level::ERROR), 2);
        assert_eq!(encode_level(&tracing::Level::WARN), 3);
        assert_eq!(encode_level(&tracing::Level::INFO), 4);
        assert_eq!(encode_level(&tracing::Level::DEBUG), 5);
        assert_eq!(encode_level(&tracing::Level::TRACE), 6);
    }

    #[test]
    fn encode_level_filter_matches_odbc_convention() {
        assert_eq!(encode_level_filter(LevelFilter::OFF), 0);
        assert_eq!(encode_level_filter(LevelFilter::ERROR), 2);
        assert_eq!(encode_level_filter(LevelFilter::WARN), 3);
        assert_eq!(encode_level_filter(LevelFilter::INFO), 4);
        assert_eq!(encode_level_filter(LevelFilter::DEBUG), 5);
        assert_eq!(encode_level_filter(LevelFilter::TRACE), 6);
    }

    #[test]
    fn encode_level_ordering_higher_is_more_verbose() {
        assert!(encode_level(&tracing::Level::ERROR) < encode_level(&tracing::Level::WARN));
        assert!(encode_level(&tracing::Level::WARN) < encode_level(&tracing::Level::INFO));
        assert!(encode_level(&tracing::Level::INFO) < encode_level(&tracing::Level::DEBUG));
        assert!(encode_level(&tracing::Level::DEBUG) < encode_level(&tracing::Level::TRACE));
    }

    #[test]
    fn filter_enabled_allows_at_or_above_severity() {
        let f = SharedLevelFilter::new(LevelFilter::INFO);

        let pass_error = encode_level(&tracing::Level::ERROR) <= encode_level_filter(f.get());
        let pass_warn = encode_level(&tracing::Level::WARN) <= encode_level_filter(f.get());
        let pass_info = encode_level(&tracing::Level::INFO) <= encode_level_filter(f.get());
        let pass_debug = encode_level(&tracing::Level::DEBUG) <= encode_level_filter(f.get());
        let pass_trace = encode_level(&tracing::Level::TRACE) <= encode_level_filter(f.get());

        assert!(pass_error);
        assert!(pass_warn);
        assert!(pass_info);
        assert!(!pass_debug);
        assert!(!pass_trace);
    }

    #[test]
    fn filter_off_rejects_everything() {
        let f = SharedLevelFilter::new(LevelFilter::OFF);
        for &level in &[
            tracing::Level::ERROR,
            tracing::Level::WARN,
            tracing::Level::INFO,
            tracing::Level::DEBUG,
            tracing::Level::TRACE,
        ] {
            assert!(
                encode_level(&level) > encode_level_filter(f.get()),
                "OFF should reject {level:?}"
            );
        }
    }

    #[test]
    fn decode_level_filter_maps_fatal_to_error() {
        assert_eq!(decode_level_filter(1), LevelFilter::ERROR);
    }
}
