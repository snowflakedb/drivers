pub mod environment;
pub mod types;

// These modules are public for integration tests but are not part of the stable API.
#[doc(hidden)]
pub mod log_batch;
#[doc(hidden)]
pub mod serialization;
#[doc(hidden)]
pub mod snowflake_exporter;
#[doc(hidden)]
pub mod snowflake_processor;

pub mod os_details;
pub mod platform_detection;

use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Span attribute key for the Snowflake session id. Read by the exporter to
/// route spans to the right session. The [`snowflake_op_span!`] macro stamps
/// the same string as a literal field name (it has to — `tracing::info_span!`
/// requires field names at macro-expansion time, not runtime values), so any
/// rename here must be mirrored in the macro.
pub const SESSION_ID_FIELD: &str = "snowflake.session.id";

/// Build a bounded span for a single FFI / driver operation tagged with the
/// owning session id. Each entry-point method opens one of these and lets it
/// end with the operation — there is no long-lived parent span.
///
/// `$session_id` must be `Option<i64>`. When `None` (handle unknown, login
/// not yet completed, mid-teardown) the macro returns `Span::none()` so the
/// span is silently disabled rather than stamped with a sentinel that could
/// collide with a real session id and route telemetry to the wrong tenant.
///
/// `event_kind = "span"` is stamped so downstream consumers can distinguish
/// per-operation span records from event records (`session_init`, `api_call`,
/// `exception`) that share the same `/telemetry/send` payload shape.
///
/// The `"snowflake.session.id"` literal must match [`SESSION_ID_FIELD`].
#[macro_export]
macro_rules! snowflake_op_span {
    ($name:expr, $session_id:expr) => {
        match $session_id {
            ::std::option::Option::Some(id) => ::tracing::info_span!(
                $name,
                "db.system" = "snowflake",
                "snowflake.session.id" = id,
                "event_kind" = "span",
            ),
            ::std::option::Option::None => ::tracing::Span::none(),
        }
    };
}

/// Record a session_init event on the **current** tracing span.
///
/// The caller must have entered a span tagged with `snowflake.session.id`
/// (e.g. one built by [`snowflake_op_span!`]) before calling this. Uses
/// `OpenTelemetrySpanExt::add_event` (rather than `tracing::event!`) because
/// events with dotted field names (e.g. `service.name`) are not supported by
/// the `event!` macro, and because
/// `Span::current().context().span().add_event(...)` operates on a detached
/// `SpanRef` and does not mutate the underlying tracing span.
pub fn record_session_init(env: &environment::EnvironmentInfo) {
    let span = tracing::Span::current();
    let mut attrs = vec![
        opentelemetry::KeyValue::new("service.name", env.driver_name.clone()),
        opentelemetry::KeyValue::new("service.version", env.driver_version.clone()),
        opentelemetry::KeyValue::new("process.runtime.name", env.language_runtime.clone()),
        opentelemetry::KeyValue::new("process.runtime.version", env.language_version.clone()),
        opentelemetry::KeyValue::new("os.type", env.os_name.clone()),
        opentelemetry::KeyValue::new("os.version", env.os_version.clone()),
        opentelemetry::KeyValue::new("host.arch", env.os_architecture.clone()),
    ];
    if let Some(ref compiler) = env.language_compiler {
        attrs.push(opentelemetry::KeyValue::new(
            "process.runtime.compiler",
            compiler.clone(),
        ));
    }
    span.add_event("session_init", attrs);
}

/// Stamp wrapper identity fields as span attributes on the **current** span.
///
/// Called before [`record_api_call`] / [`record_exception`] so that the span
/// attributes (and therefore all events on that span) carry the identity.
/// No-ops on a disabled (noop) span, so it is safe to call unconditionally.
pub fn record_wrapper_identity_on_span(
    identity: &crate::apis::database_driver_v1::connection::WrapperIdentity,
) {
    let span = tracing::Span::current();
    span.set_attribute("service.name", identity.driver_name.clone());
    span.set_attribute("service.version", identity.driver_version.clone());
    span.set_attribute("process.runtime.name", identity.language_runtime.clone());
    span.set_attribute("process.runtime.version", identity.language_version.clone());
    if let Some(ref compiler) = identity.language_compiler {
        span.set_attribute("process.runtime.compiler", compiler.clone());
    }
}

/// Record an api_call event on the **current** tracing span.
///
/// `passed_arguments` are the names of the arguments the caller explicitly
/// passed to the API method (names only — never values, defaults omitted).
/// When non-empty they are attached as a comma-joined `api_arguments`
/// attribute so the serialized payload stays a flat string.
pub fn record_api_call(api_method: &str, passed_arguments: &[String]) {
    let mut attrs = vec![opentelemetry::KeyValue::new(
        "api_method",
        api_method.to_string(),
    )];
    if !passed_arguments.is_empty() {
        attrs.push(opentelemetry::KeyValue::new(
            "api_arguments",
            passed_arguments.join(","),
        ));
    }
    tracing::Span::current().add_event("api_call", attrs);
}

/// Record an exception event on the **current** tracing span.
pub fn record_exception(exception_type: &str, error_source: &str) {
    tracing::Span::current().add_event(
        "exception",
        vec![
            opentelemetry::KeyValue::new("exception.type", exception_type.to_string()),
            opentelemetry::KeyValue::new("exception.source", error_source.to_string()),
        ],
    );
}
