//! Logging utilities for the Snowflake universal driver core.
//!
//! # URL sanitization
//!
//! Two public functions sanitize URLs before they reach log sinks. They have
//! intentionally different contracts because they serve different log contexts:
//!
//! | | [`url_for_log`] | [`sanitize_config_url_for_log`] |
//! |---|---|---|
//! | Used for | HTTP/REST call logs | Connection-parameter dumps |
//! | Input | Scheme-bearing parseable URL | Any string; may lack scheme |
//! | Keeps | host + path only | scheme + user + host + port + path |
//! | Userinfo | Dropped entirely | Username kept, password masked |
//! | Query/fragment | Dropped | Dropped |
//! | `scheme://` with a non-numeric `:` and no `@` | `<unknown>` | `****` |
//! | `scheme://` single-label name, numeric port, no `@` | host | `****` |
//! | Other non-URL-shaped input | `<unknown>` | unchanged; query and fragment stripped |
//!
//! Both share the private [`strip_query_and_fragment`] helper, which must run
//! before any `@` search so that an `@` inside a query string is never mistaken
//! for a userinfo delimiter.

use std::fmt::Debug;
use std::path::PathBuf;

pub use crate::logging::callback_layer::CLogCallback;
pub use crate::logging::callback_layer::CallbackLayer;
pub use crate::logging::error::LogError;
pub use crate::logging::log_manager::{LogManager, LogReloadHandle, ReloadOutcome};
use tracing::Subscriber;
use tracing::level_filters::LevelFilter;
use tracing_core::Field;
use tracing_core::field::Visit;
use tracing_subscriber::Layer;

pub mod c_api;
pub(crate) mod callback_layer;
pub(crate) mod error;
pub mod log_manager;
pub(crate) mod opentelemetry;

/// Time-based log-file rotation strategy.
///
/// Wraps `tracing_appender::rolling::Rotation` so callers don't depend on
/// the appender crate directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogRotation {
    #[default]
    Never,
    Daily,
    Hourly,
    Minutely,
}

impl LogRotation {
    pub(crate) fn to_appender_rotation(self) -> tracing_appender::rolling::Rotation {
        match self {
            Self::Never => tracing_appender::rolling::Rotation::NEVER,
            Self::Daily => tracing_appender::rolling::Rotation::DAILY,
            Self::Hourly => tracing_appender::rolling::Rotation::HOURLY,
            Self::Minutely => tracing_appender::rolling::Rotation::MINUTELY,
        }
    }
}

/// Configuration for the logging subsystem.
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub enabled: bool,
    pub level: LevelFilter,
    pub log_path: Option<PathBuf>,
    pub log_file_name: Option<String>,
    /// Desired maximum size (in bytes) for a single log file.
    ///
    /// **Not yet enforced.** `tracing-appender` only supports time-based
    /// rotation, so size-based rotation is not available. When this field is
    /// `Some`, a warning is emitted at init time and the value is otherwise
    /// ignored. The field is retained for forward-compatibility with a future
    /// size-aware appender.
    pub max_file_size: Option<u64>,
    pub max_file_count: Option<u32>,
    pub rotation: LogRotation,
    pub open_telemetry: bool,
    pub stderr: bool,
    /// Process-wide default for `log_query_text`, applied as a fallback when
    /// neither a connection-string option nor a `connections.toml` /
    /// `config.toml` setting is provided. `None` means "unset; fall through to
    /// the registry default".
    pub log_query_text: Option<bool>,
    /// Process-wide default for `log_query_parameters`. See
    /// [`Self::log_query_text`] for precedence semantics.
    pub log_query_parameters: Option<bool>,
    /// When `true`, `OdbcError::message_text()` appends the full error trace
    /// to user-facing diagnostic messages. Default `true`.
    pub error_trace_enabled: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: LevelFilter::INFO,
            log_path: None,
            log_file_name: None,
            max_file_size: None,
            max_file_count: None,
            rotation: LogRotation::default(),
            open_telemetry: false,
            stderr: false,
            log_query_text: None,
            log_query_parameters: None,
            error_trace_enabled: true,
        }
    }
}

pub(crate) struct EmptyLayer;

impl<S: Subscriber> Layer<S> for EmptyLayer {}

/// Log a foreign/external error: emit the error type name at `error` level (safe for
/// WARN/ERROR logs) and the full `{:?}` detail at `debug` level.
///
/// # Usage
/// ```ignore
/// log_foreign_error!(e, "Failed to send result to channel");
/// log_foreign_error!(warn, e, "Failed to read response body");
/// ```
#[macro_export]
macro_rules! log_foreign_error {
    ($e:expr, $msg:literal) => {{
        tracing::error!(cause = ::std::any::type_name_of_val(&$e), $msg);
        tracing::debug!(concat!($msg, ": {:?}"), $e);
    }};
    (warn, $e:expr, $msg:literal) => {{
        tracing::warn!(cause = ::std::any::type_name_of_val(&$e), $msg);
        tracing::debug!(concat!($msg, ": {:?}"), $e);
    }};
}
pub use log_foreign_error;

/// Extract host and path from a URL string for safe logging (strips query strings and
/// fragments which can carry tokens and other sensitive identifiers).
///
/// Returns `"<unknown>"` if the URL cannot be parsed.
pub fn url_for_log(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .map(|u| {
            let host = u.host_str().unwrap_or("<unknown host>");
            let path = u.path();
            if path.is_empty() || path == "/" {
                host.to_string()
            } else {
                format!("{host}{path}")
            }
        })
        .unwrap_or_else(|| "<unknown>".into())
}

fn authority_embeds_unseparated_password(authority: &str) -> bool {
    if let Some(rest) = authority.strip_prefix('[') {
        if !authority.contains(':') {
            return false;
        }
        let Some((ipv6, after)) = rest.split_once(']') else {
            return true;
        };
        let ipv6_ok = !ipv6.is_empty()
            && ipv6.contains(':')
            && ipv6
                .bytes()
                .all(|b| b.is_ascii_hexdigit() || b == b':' || b == b'.');
        let port_ok = after.is_empty() || is_port_suffix(after);
        return !(ipv6_ok && port_ok);
    }
    let Some((host, port)) = authority.rsplit_once(':') else {
        return false;
    };
    let host_ok = !host.is_empty()
        && !host.contains(':')
        && host.contains('.')
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'_');
    !(host_ok && is_port(port))
}

fn is_port_suffix(after_bracket: &str) -> bool {
    after_bracket.strip_prefix(':').is_some_and(is_port)
}

fn is_port(port: &str) -> bool {
    !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) && port.parse::<u16>().is_ok()
}

/// Strips everything from the first `?` or `#`, returning the prefix.
fn strip_query_and_fragment(url: &str) -> &str {
    let q = url.find('?').unwrap_or(url.len());
    let f = url.find('#').unwrap_or(url.len());
    &url[..q.min(f)]
}

/// Sanitize a connection-parameter value that is expected to be a URL for safe logging.
///
/// Keeps scheme, username, host, port, and path. Masks any password in userinfo
/// with `****`. Drops query strings and fragments. Accepts schemeless values
/// such as `user:pass@host:8080`.
///
/// Returns `"****"` when a `/` appears before the `@`, or when the value has
/// `scheme://`, no `@`, and a colon in the authority that is not a numeric port
/// on a dotted host or a bracketed IPv6 literal. `https://user:SECRET`,
/// `https://user:SECRET?token=x`, and `https://user:8080` therefore become
/// `"****"`. A dotted host or bracketed IPv6 with a numeric port stays visible.
///
/// Values with no `://` and no `@` keep their text, with any query or fragment
/// removed. A username with no password is kept the same way.
pub fn sanitize_config_url_for_log(url: &str) -> std::borrow::Cow<'_, str> {
    use std::borrow::Cow;

    let stripped = strip_query_and_fragment(url);

    let (prefix, after_scheme) = match stripped.find("://") {
        Some(pos) => (&stripped[..pos + 3], &stripped[pos + 3..]),
        None => ("", stripped),
    };

    let at_pos = match after_scheme.rfind('@') {
        Some(pos) => pos,
        None => {
            if !prefix.is_empty() {
                let authority_end = after_scheme.find('/').unwrap_or(after_scheme.len());
                let authority = &after_scheme[..authority_end];
                if authority_embeds_unseparated_password(authority) {
                    return Cow::Borrowed("****");
                }
            }
            return if stripped.len() == url.len() {
                Cow::Borrowed(url)
            } else {
                Cow::Owned(stripped.to_owned())
            };
        }
    };

    let before_at = &after_scheme[..at_pos];

    if before_at.contains('/') {
        return Cow::Borrowed("****");
    }

    let userinfo = before_at;
    let after_at = &after_scheme[at_pos + 1..];
    let host_end = after_at.find('/').unwrap_or(after_at.len());
    let host_and_port = &after_at[..host_end];
    let path = &after_at[host_end..];

    match userinfo.find(':') {
        Some(colon_pos) => {
            let user = &userinfo[..colon_pos];
            Cow::Owned(format!("{prefix}{user}:****@{host_and_port}{path}"))
        }
        None => {
            if stripped.len() == url.len() {
                Cow::Borrowed(url)
            } else {
                Cow::Owned(stripped.to_owned())
            }
        }
    }
}

/// Tracing target carried by wrapper-originated events (the round-trip path).
/// Events on this target had their real source location and originating logger
/// name packed into event fields by the [`wrapper_event!`] macro; core-originated
/// events use the normal tracing metadata instead.
pub const WRAPPER_TARGET: &str = "sf_wrapper";

/// `tracing::event!` requires a compile-time level; dispatch at runtime via match.
#[macro_export]
macro_rules! wrapper_event {
    ($level:expr, $($fields:tt)*) => {
        match $level {
            0 => tracing::event!(target: $crate::logging::WRAPPER_TARGET, tracing::Level::ERROR, $($fields)*),
            1 => tracing::event!(target: $crate::logging::WRAPPER_TARGET, tracing::Level::WARN, $($fields)*),
            2 => tracing::event!(target: $crate::logging::WRAPPER_TARGET, tracing::Level::INFO, $($fields)*),
            _ => tracing::event!(target: $crate::logging::WRAPPER_TARGET, tracing::Level::DEBUG, $($fields)*),
        }
    };
}

/// Flattened tracing event fields for wrapper log sinks (FFI/JNI callback layers).
///
/// `logger_name` is empty for core-originated events; set for wrapper round-trip events.
/// `level` uses the shared FFI encoding: 0=ERROR, 1=WARN, 2=INFO, 3=DEBUG.
/// TRACE is 3 under [`TraceEncoding::FoldIntoDebug`] and 4 under [`TraceEncoding::Distinct`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedEvent {
    pub level: u32,
    pub message: String,
    pub file: String,
    pub line: u32,
    pub function: String,
    pub logger_name: String,
}

#[derive(Default)]
struct EventVisitor {
    message: String,
    file: Option<String>,
    line: Option<u32>,
    function: Option<String>,
    logger_name: String,
    extra: String,
}

impl EventVisitor {
    fn record_extra(&mut self, name: &str, value: impl std::fmt::Display) {
        use std::fmt::Write as _;
        let _ = write!(self.extra, " {name}={value}");
    }
}

impl Visit for EventVisitor {
    fn record_i64(&mut self, field: &Field, value: i64) {
        if field.name() == "line" {
            self.line = u32::try_from(value).ok();
        } else {
            self.record_extra(field.name(), value);
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == "line" {
            self.line = u32::try_from(value).ok();
        } else {
            self.record_extra(field.name(), value);
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "message" => self.message = value.to_owned(),
            "logger_name" => self.logger_name = value.to_owned(),
            "file" => self.file = Some(value.to_owned()),
            "function" => self.function = Some(value.to_owned()),
            name => self.record_extra(name, value),
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        match field.name() {
            "message" if self.message.is_empty() => self.message = format!("{value:?}"),
            name => self.record_extra(name, format_args!("{value:?}")),
        }
    }
}

const DEBUG_WIRE_LEVEL: u32 = 3;
const DISTINCT_TRACE_WIRE_LEVEL: u32 = 4;

/// How `tracing::Level::TRACE` is encoded in [`NormalizedEvent::level`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraceEncoding {
    /// TRACE shares wire level 3 with DEBUG.
    #[default]
    FoldIntoDebug,
    /// TRACE has its own wire level 4.
    Distinct,
}

impl TraceEncoding {
    fn wire_level(self) -> u32 {
        match self {
            Self::FoldIntoDebug => DEBUG_WIRE_LEVEL,
            Self::Distinct => DISTINCT_TRACE_WIRE_LEVEL,
        }
    }
}

/// Flatten a tracing [`Event`] into [`NormalizedEvent`] for a wrapper log sink.
///
/// ERROR, WARN, INFO and DEBUG are wire levels 0 to 3. TRACE follows
/// `trace_encoding`. For wrapper round-trip events ([`WRAPPER_TARGET`]) the real
/// source location and originating logger name live in event fields; for
/// core-originated events they come from the event metadata and `logger_name`
/// is left empty.
pub fn normalize_event_with_trace_encoding(
    event: &tracing::Event<'_>,
    trace_encoding: TraceEncoding,
) -> NormalizedEvent {
    let meta = event.metadata();
    let level = match *meta.level() {
        tracing::Level::ERROR => 0,
        tracing::Level::WARN => 1,
        tracing::Level::INFO => 2,
        tracing::Level::DEBUG => DEBUG_WIRE_LEVEL,
        tracing::Level::TRACE => trace_encoding.wire_level(),
    };

    let mut visitor = EventVisitor::default();
    event.record(&mut visitor);

    if meta.target() == WRAPPER_TARGET {
        NormalizedEvent {
            level,
            message: visitor.message,
            file: visitor.file.unwrap_or_else(|| "unknown".to_owned()),
            line: visitor.line.unwrap_or(0),
            function: visitor.function.unwrap_or_else(|| "unknown".to_owned()),
            logger_name: visitor.logger_name,
        }
    } else {
        let message = if visitor.extra.is_empty() {
            visitor.message
        } else {
            format!("{}{}", visitor.message, visitor.extra)
        };
        NormalizedEvent {
            level,
            message,
            file: meta.file().unwrap_or("unknown").to_owned(),
            line: meta.line().unwrap_or(0),
            function: meta.name().to_owned(),
            logger_name: String::new(),
        }
    }
}

/// [`normalize_event_with_trace_encoding`] with [`TraceEncoding::FoldIntoDebug`].
pub fn normalize_event(event: &tracing::Event<'_>) -> NormalizedEvent {
    normalize_event_with_trace_encoding(event, TraceEncoding::FoldIntoDebug)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing::Subscriber;
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::{Context, SubscriberExt};

    struct CapturingLayer {
        events: Arc<Mutex<Vec<NormalizedEvent>>>,
    }

    impl CapturingLayer {
        fn new(events: Arc<Mutex<Vec<NormalizedEvent>>>) -> Self {
            Self { events }
        }
    }

    impl<S: Subscriber> Layer<S> for CapturingLayer {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            self.events
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(normalize_event(event));
        }
    }

    fn capture_events<F: FnOnce()>(emit: F) -> Vec<NormalizedEvent> {
        let events = Arc::new(Mutex::new(Vec::new()));
        let subscriber =
            tracing_subscriber::registry().with(CapturingLayer::new(Arc::clone(&events)));
        let _guard = tracing::subscriber::set_default(subscriber);
        emit();
        events.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    #[test]
    fn url_for_log_strips_query_string_and_fragment() {
        assert_eq!(
            url_for_log("https://example.com/oauth/token?token=super-secret&user=jdoe#frag"),
            "example.com/oauth/token"
        );
    }

    #[test]
    fn url_for_log_returns_host_only_for_root_path() {
        assert_eq!(
            url_for_log("https://example.com/?token=super-secret"),
            "example.com"
        );
        assert_eq!(url_for_log("https://example.com"), "example.com");
    }

    #[test]
    fn url_for_log_returns_unknown_for_unparseable_url() {
        assert_eq!(url_for_log("not a url"), "<unknown>");
    }

    #[test]
    fn sanitize_config_url_masks_password_and_drops_query() {
        assert_eq!(
            sanitize_config_url_for_log("https://alice:secret@host:8080/path?q=1#frag"),
            "https://alice:****@host:8080/path"
        );
    }

    #[test]
    fn sanitize_config_url_no_password_with_query_strips_query() {
        assert_eq!(
            sanitize_config_url_for_log("https://host/path?q=1"),
            "https://host/path"
        );
    }

    #[test]
    fn sanitize_config_url_no_userinfo_returns_borrowed() {
        let url = "https://example.com/path";
        assert!(matches!(
            sanitize_config_url_for_log(url),
            std::borrow::Cow::Borrowed(_)
        ));
        assert_eq!(sanitize_config_url_for_log(url), url);
    }

    #[test]
    fn sanitize_config_url_schemeless_with_credentials() {
        assert_eq!(
            sanitize_config_url_for_log("alice:pass@host:1234"),
            "alice:****@host:1234"
        );
    }

    #[test]
    fn sanitize_config_url_slash_before_at_fails_closed() {
        assert_eq!(
            sanitize_config_url_for_log("https://user/path@host"),
            "****"
        );
    }

    #[test]
    fn sanitize_config_url_at_in_query_not_mistaken_for_userinfo() {
        assert_eq!(
            sanitize_config_url_for_log("https://host/path?ref=user@example.com"),
            "https://host/path"
        );
    }

    #[test]
    fn sanitize_config_url_schemeless_no_userinfo_plain_value_returns_borrowed() {
        let val = "snowflake";
        assert!(matches!(
            sanitize_config_url_for_log(val),
            std::borrow::Cow::Borrowed(_)
        ));
        assert_eq!(sanitize_config_url_for_log(val), val);
    }

    #[test]
    fn sanitize_config_url_username_only_no_password_and_no_query_is_unchanged() {
        let url = "https://alice@host/path";
        assert_eq!(sanitize_config_url_for_log(url), url);
    }

    #[test]
    fn sanitize_config_url_scheme_with_password_colon_and_no_at_fails_closed() {
        let canary = "DO_NOT_LEAK_SECRET_42";
        assert_eq!(
            sanitize_config_url_for_log(&format!("https://user:{canary}?token=x")),
            "****"
        );
        assert_eq!(
            sanitize_config_url_for_log(&format!("https://user:{canary}")),
            "****"
        );
        assert_eq!(
            sanitize_config_url_for_log(&format!("https://user:{canary}/path")),
            "****"
        );
        assert_eq!(sanitize_config_url_for_log("https://user:8080"), "****");
        assert_eq!(
            sanitize_config_url_for_log("https://user:8080/path"),
            "****"
        );
        assert_eq!(
            sanitize_config_url_for_log("https://user:8080?token=x"),
            "****"
        );
        let rendered = format!(
            "{} {} {}",
            sanitize_config_url_for_log(&format!("https://user:{canary}?token=x")),
            sanitize_config_url_for_log(&format!("https://user:{canary}")),
            sanitize_config_url_for_log(&format!("https://user:{canary}/path")),
        );
        assert!(
            !rendered.contains(canary),
            "password leaked from scheme URL with no @: {rendered}"
        );
    }

    #[test]
    fn sanitize_config_url_host_port_without_userinfo_stays_visible() {
        assert_eq!(
            sanitize_config_url_for_log("http://proxy.example.com:8080/path?q=1"),
            "http://proxy.example.com:8080/path"
        );
        assert_eq!(
            sanitize_config_url_for_log("http://[::1]:8080/"),
            "http://[::1]:8080/"
        );
        assert_eq!(
            sanitize_config_url_for_log("http://127.0.0.1:8080/"),
            "http://127.0.0.1:8080/"
        );
    }

    #[test]
    fn sanitize_config_url_ipv6_masks_password() {
        assert_eq!(
            sanitize_config_url_for_log("http://user:pass@[::1]:8080"),
            "http://user:****@[::1]:8080"
        );
        assert_eq!(
            sanitize_config_url_for_log("http://user:pass@[::1]/"),
            "http://user:****@[::1]/"
        );
    }

    #[test]
    fn sanitize_config_url_socks5_masks_password() {
        assert_eq!(
            sanitize_config_url_for_log("socks5://user:pass@host:1080"),
            "socks5://user:****@host:1080"
        );
    }

    #[test]
    fn sanitize_config_url_query_at_does_not_become_host() {
        let url = "http://user:pass@host:8080?ref=x@secret.example";
        assert_eq!(
            sanitize_config_url_for_log(url),
            "http://user:****@host:8080"
        );
        assert!(
            !sanitize_config_url_for_log(url).contains("secret.example"),
            "query @ must not replace the host"
        );
    }

    #[test]
    fn sanitize_config_url_fragment_at_does_not_become_host() {
        let url = "http://user:pass@host:8080#x@secret.example";
        assert_eq!(
            sanitize_config_url_for_log(url),
            "http://user:****@host:8080"
        );
        assert!(
            !sanitize_config_url_for_log(url).contains("secret.example"),
            "fragment @ must not replace the host"
        );
    }

    #[test]
    fn should_map_core_info_event_from_metadata() {
        let events = capture_events(|| tracing::info!(target: "sf_core", "core info message"));
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.level, 2);
        assert_eq!(event.message, "core info message");
        assert!(event.logger_name.is_empty());
        assert!(!event.file.is_empty());
        assert_ne!(event.function, "unknown");
    }

    #[test]
    fn should_map_core_trace_event_to_debug_wire_level() {
        let events = capture_events(|| tracing::trace!(target: "sf_core", "trace detail"));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, DEBUG_WIRE_LEVEL);
        assert_eq!(events[0].message, "trace detail");
        assert!(events[0].logger_name.is_empty());
    }

    #[test]
    fn should_map_wrapper_round_trip_fields_from_event_payload() {
        let events = capture_events(|| {
            crate::wrapper_event!(
                2,
                message = "wrapper payload",
                file = "Driver.java",
                function = "connect",
                line = 99,
                logger_name = "net.snowflake.client.Driver",
            );
        });
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.level, 2);
        assert_eq!(event.message, "wrapper payload");
        assert_eq!(event.file, "Driver.java");
        assert_eq!(event.line, 99);
        assert_eq!(event.function, "connect");
        assert_eq!(event.logger_name, "net.snowflake.client.Driver");
    }

    #[test]
    fn should_append_extra_fields_on_core_originated_events() {
        let events = capture_events(|| tracing::info!(target: "sf_core", user_id = 42, "hello"));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].message, "hello user_id=42");
        assert!(events[0].logger_name.is_empty());
    }

    #[test]
    fn should_not_append_extra_fields_on_wrapper_round_trip_events() {
        let events = capture_events(|| {
            crate::wrapper_event!(
                3,
                message = "already formatted",
                file = "",
                function = "",
                line = 0,
                logger_name = "com.example.Logger",
                spare = "ignored",
            );
        });
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].message, "already formatted");
        assert_eq!(events[0].logger_name, "com.example.Logger");
    }
}
