pub mod environment;
pub mod types;

// These modules are public for integration tests but are not part of the stable API.
#[doc(hidden)]
pub mod serialization;
#[doc(hidden)]
pub mod snowflake_exporter;

use environment::EnvironmentInfo;
use opentelemetry::InstrumentationScope;
use opentelemetry::KeyValue;
use opentelemetry::trace::{
    SpanContext, SpanId, SpanKind, Status, TraceFlags, TraceId, TraceState,
};
use opentelemetry_sdk::trace::SpanData;

/// Build a `session_init` OTel span carrying environment and session metadata.
///
/// The span is serialized by [`serialization::spans_to_snowflake_payload`] and
/// exported via [`snowflake_exporter::SnowflakeInBandExporter`] to the
/// `/telemetry/send` endpoint.
pub fn build_session_init_span(env: &EnvironmentInfo, session_id: i64) -> SpanData {
    let now = std::time::SystemTime::now();

    let mut attributes = vec![
        KeyValue::new("service.name", env.driver_name.clone()),
        KeyValue::new("service.version", env.driver_version.clone()),
        KeyValue::new("process.runtime.name", env.language_runtime.clone()),
        KeyValue::new("process.runtime.version", env.language_version.clone()),
        KeyValue::new("os.type", env.os_name.clone()),
        KeyValue::new("os.version", env.os_version.clone()),
        KeyValue::new("host.arch", env.os_architecture.clone()),
        KeyValue::new("snowflake.session.id", session_id),
    ];

    if let Some(ref compiler) = env.language_compiler {
        attributes.push(KeyValue::new("process.runtime.compiler", compiler.clone()));
    }

    SpanData {
        span_context: SpanContext::new(
            TraceId::INVALID,
            SpanId::INVALID,
            TraceFlags::default(),
            false,
            TraceState::default(),
        ),
        parent_span_id: SpanId::INVALID,
        span_kind: SpanKind::Internal,
        name: "session_init".into(),
        start_time: now,
        end_time: now,
        attributes,
        dropped_attributes_count: 0,
        events: Default::default(),
        links: Default::default(),
        status: Status::Ok,
        instrumentation_scope: InstrumentationScope::builder("snowflake.telemetry").build(),
    }
}
