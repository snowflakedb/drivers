pub mod environment;
pub mod types;

// These modules are public for integration tests but are not part of the stable API.
#[doc(hidden)]
pub mod serialization;
#[doc(hidden)]
pub mod snowflake_exporter;

pub mod os_details;
pub mod platform_detection;

use crate::apis::database_driver_v1::DriverProviders;

/// Telemetry state created during logging initialization.
///
/// Bridges (C API, JDBC, ODBC) should store this between `init_logging` and
/// `DatabaseDriverV1` creation, then pass it to [`DriverProviders`] via
/// [`into_providers`](TelemetryInit::into_providers).
pub struct TelemetryInit {
    pub provider: opentelemetry_sdk::trace::SdkTracerProvider,
    pub sessions: snowflake_exporter::SessionRegistry,
}

impl TelemetryInit {
    /// Convert into `DriverProviders` fields for `DatabaseDriverV1`.
    pub fn into_providers(self) -> DriverProviders {
        DriverProviders {
            telemetry_sessions: Some(self.sessions),
            telemetry_provider: Some(self.provider),
            ..Default::default()
        }
    }

    /// Create `DriverProviders` by cloning (provider uses `Arc` internally).
    pub fn to_providers(&self) -> DriverProviders {
        DriverProviders {
            telemetry_sessions: Some(self.sessions.clone()),
            telemetry_provider: Some(self.provider.clone()),
            ..Default::default()
        }
    }
}

use opentelemetry::trace::TraceContextExt;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Record a session_init event on the **current** tracing span.
///
/// The caller must have entered the connection span before calling this.
/// Uses the OTel API directly because `tracing::event!` does not support
/// dotted field names (e.g. `service.name`).
pub fn record_session_init(env: &environment::EnvironmentInfo) {
    let span = tracing::Span::current();
    let otel_ctx = span.context();
    let otel_span = otel_ctx.span();
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
    otel_span.add_event("session_init", attrs);
}

/// Record an api_call event on the **current** tracing span.
pub fn record_api_call(api_method: &str) {
    let span = tracing::Span::current();
    let otel_ctx = span.context();
    let otel_span = otel_ctx.span();
    otel_span.add_event(
        "api_call",
        vec![opentelemetry::KeyValue::new(
            "api_method",
            api_method.to_string(),
        )],
    );
}

/// Record an exception event on the **current** tracing span.
pub fn record_exception(exception_type: &str, error_source: &str) {
    let span = tracing::Span::current();
    let otel_ctx = span.context();
    let otel_span = otel_ctx.span();
    otel_span.add_event(
        "exception",
        vec![
            opentelemetry::KeyValue::new("exception.type", exception_type.to_string()),
            opentelemetry::KeyValue::new("exception.source", error_source.to_string()),
        ],
    );
}
