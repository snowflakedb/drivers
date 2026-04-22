use opentelemetry::trace::{
    SpanContext, SpanId, SpanKind, Status, TraceFlags, TraceId, TraceState,
};
use opentelemetry::{InstrumentationScope, KeyValue};
use opentelemetry_sdk::trace::SpanData;
use sf_core::telemetry::serialization::spans_to_snowflake_payload;
use std::time::{Duration, UNIX_EPOCH};

fn make_span(name: &'static str, attributes: Vec<KeyValue>) -> SpanData {
    SpanData {
        span_context: SpanContext::new(
            TraceId::from_hex("0102030405060708090a0b0c0d0e0f10").unwrap(),
            SpanId::from_hex("0102030405060708").unwrap(),
            TraceFlags::default(),
            false,
            TraceState::default(),
        ),
        parent_span_id: SpanId::INVALID,
        span_kind: SpanKind::Internal,
        name: name.into(),
        start_time: UNIX_EPOCH + Duration::from_millis(1700000000000),
        end_time: UNIX_EPOCH + Duration::from_millis(1700000001000),
        attributes,
        dropped_attributes_count: 0,
        events: Default::default(),
        links: Default::default(),
        status: Status::Ok,
        instrumentation_scope: InstrumentationScope::builder("test").build(),
    }
}

#[test]
fn session_init_span_produces_correct_snowflake_json() {
    let span = make_span(
        "session_init",
        vec![
            KeyValue::new("service.name", "snowflake-python"),
            KeyValue::new("service.version", "3.5.0"),
            KeyValue::new("os.type", "linux"),
            KeyValue::new("os.version", "6.1"),
            KeyValue::new("host.arch", "aarch64"),
            KeyValue::new("process.runtime.name", "CPython"),
            KeyValue::new("process.runtime.version", "3.11.6"),
            KeyValue::new("snowflake.driver.core_version", "0.1.0"),
            KeyValue::new("snowflake.driver.is_ci_cd", false),
            KeyValue::new("snowflake.driver.is_interactive", true),
        ],
    );

    let payload = spans_to_snowflake_payload(&[span]);

    let logs = payload["logs"].as_array().unwrap();
    assert_eq!(logs.len(), 1);

    let msg = &logs[0]["message"];
    assert_eq!(msg["type"], "session_init");
    assert_eq!(msg["service.name"], "snowflake-python");
    assert_eq!(msg["service.version"], "3.5.0");
    assert_eq!(msg["os.type"], "linux");
    assert_eq!(msg["host.arch"], "aarch64");
    assert_eq!(msg["process.runtime.name"], "CPython");
    assert_eq!(msg["snowflake.driver.core_version"], "0.1.0");
    assert_eq!(msg["snowflake.driver.is_ci_cd"], false);
    assert_eq!(msg["snowflake.driver.is_interactive"], true);

    assert_eq!(logs[0]["timestamp"], "1700000000000");
}

#[test]
fn multiple_spans_produce_multiple_log_entries() {
    let spans = vec![
        make_span(
            "session_init",
            vec![KeyValue::new("service.name", "python")],
        ),
        make_span(
            "driver_exception",
            vec![KeyValue::new("exception.type", "RuntimeError")],
        ),
    ];

    let payload = spans_to_snowflake_payload(&spans);
    let logs = payload["logs"].as_array().unwrap();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0]["message"]["type"], "session_init");
    assert_eq!(logs[1]["message"]["type"], "driver_exception");
}

#[test]
fn exception_span_event_attributes_are_flattened_into_message() {
    use opentelemetry::trace::Event;

    let mut span = make_span("driver_exception", vec![]);
    let event = Event::new(
        "exception",
        UNIX_EPOCH + Duration::from_millis(1700000000500),
        vec![
            KeyValue::new("exception.type", "ConnectionError"),
            KeyValue::new("exception.message", "timeout after 30s"),
            KeyValue::new("exception.stacktrace", "at line 42"),
        ],
        0,
    );
    let mut events = opentelemetry_sdk::trace::SpanEvents::default();
    events.events.push(event);
    span.events = events;

    let payload = spans_to_snowflake_payload(&[span]);
    let msg = &payload["logs"][0]["message"];

    assert_eq!(msg["type"], "driver_exception");
    assert_eq!(msg["exception.type"], "ConnectionError");
    assert_eq!(msg["exception.message"], "timeout after 30s");
    assert_eq!(msg["exception.stacktrace"], "at line 42");
}

#[test]
fn non_exception_span_events_are_not_flattened() {
    use opentelemetry::trace::Event;

    let mut span = make_span("some_span", vec![]);
    let event = Event::new(
        "custom_event",
        UNIX_EPOCH + Duration::from_millis(1700000000500),
        vec![KeyValue::new("custom.key", "custom_value")],
        0,
    );
    let mut events = opentelemetry_sdk::trace::SpanEvents::default();
    events.events.push(event);
    span.events = events;

    let payload = spans_to_snowflake_payload(&[span]);
    let msg = &payload["logs"][0]["message"];

    // Only "type" should be present — custom event attrs should NOT be flattened
    assert_eq!(msg["type"], "some_span");
    assert!(msg.get("custom.key").is_none());
}
