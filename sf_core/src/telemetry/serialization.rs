use std::time::{SystemTime, UNIX_EPOCH};

use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData, ResourceMetrics};
use opentelemetry_sdk::trace::SpanData;
use serde_json::{Value, json};

/// Convert a batch of OTel spans into Snowflake's `/telemetry/send` JSON payload.
///
/// Format: `{"logs": [{"message": {...}, "timestamp": "..."}]}`
pub fn spans_to_snowflake_payload(spans: &[SpanData]) -> Value {
    let logs: Vec<Value> = spans.iter().map(span_to_log_entry).collect();
    json!({ "logs": logs })
}

fn span_to_log_entry(span: &SpanData) -> Value {
    let mut message = serde_json::Map::new();

    message.insert("type".to_string(), Value::String(span.name.to_string()));

    for kv in &span.attributes {
        let key = kv.key.as_str();
        if key == "type" {
            tracing::warn!("Span attribute key 'type' conflicts with span name field, skipping");
            continue;
        }
        message.insert(key.to_string(), otel_value_to_json(&kv.value));
    }

    // Flatten span events (e.g., exception events) into the message.
    // Event attributes are prefixed with "exception." to avoid overwriting span attributes.
    for event in span.events.iter() {
        if event.name.as_ref() == "exception" {
            for kv in &event.attributes {
                let key = kv.key.as_str();
                let prefixed = if key.starts_with("exception.") {
                    key.to_string()
                } else {
                    format!("exception.{key}")
                };
                message.insert(prefixed, otel_value_to_json(&kv.value));
            }
        }
    }

    let timestamp = system_time_to_epoch_millis(span.start_time);

    json!({
        "message": message,
        "timestamp": timestamp.to_string()
    })
}

macro_rules! collect_sum_data_points {
    ($logs:expr, $metric_name:expr, $sum:expr, $ty:ty) => {{
        let timestamp = system_time_to_epoch_millis($sum.time());
        for dp in $sum.data_points() {
            let mut message = serde_json::Map::new();
            message.insert("type".to_string(), Value::String($metric_name.to_string()));
            message.insert("value".to_string(), json!(dp.value()));
            for kv in dp.attributes() {
                message.insert(kv.key.as_str().to_string(), otel_value_to_json(&kv.value));
            }
            $logs.push(json!({
                "message": message,
                "timestamp": timestamp.to_string()
            }));
        }
    }};
}

/// Convert aggregated metric data into Snowflake's `/telemetry/send` JSON payload.
pub fn metrics_to_snowflake_payload(metrics: &ResourceMetrics) -> Value {
    let mut logs = Vec::new();

    for scope_metrics in metrics.scope_metrics() {
        for metric in scope_metrics.metrics() {
            let metric_name = metric.name();
            match metric.data() {
                AggregatedMetrics::U64(MetricData::Sum(sum)) => {
                    collect_sum_data_points!(logs, metric_name, sum, u64);
                }
                AggregatedMetrics::I64(MetricData::Sum(sum)) => {
                    collect_sum_data_points!(logs, metric_name, sum, i64);
                }
                AggregatedMetrics::F64(MetricData::Sum(sum)) => {
                    collect_sum_data_points!(logs, metric_name, sum, f64);
                }
                _ => {
                    tracing::debug!("Skipping non-Sum metric type for telemetry: {metric_name}");
                }
            }
        }
    }

    json!({ "logs": logs })
}

fn otel_value_to_json(value: &opentelemetry::Value) -> Value {
    use opentelemetry::Value as OtelValue;
    match value {
        OtelValue::Bool(b) => Value::Bool(*b),
        OtelValue::I64(i) => json!(*i),
        OtelValue::F64(f) => json!(*f),
        OtelValue::String(s) => Value::String(s.to_string()),
        OtelValue::Array(_) => Value::String(format!("{value}")),
        _ => Value::String(format!("{value}")),
    }
}

fn system_time_to_epoch_millis(time: SystemTime) -> u128 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_millis(),
        Err(_) => {
            tracing::warn!("SystemTime before UNIX_EPOCH, using 0");
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::InstrumentationScope;
    use opentelemetry::KeyValue;
    use opentelemetry::trace::{
        SpanContext, SpanId, SpanKind, Status, TraceFlags, TraceId, TraceState,
    };

    fn make_test_span(name: &'static str, attributes: Vec<KeyValue>) -> SpanData {
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
            start_time: UNIX_EPOCH + std::time::Duration::from_millis(1700000000000),
            end_time: UNIX_EPOCH + std::time::Duration::from_millis(1700000001000),
            attributes,
            dropped_attributes_count: 0,
            events: Default::default(),
            links: Default::default(),
            status: Status::Ok,
            instrumentation_scope: InstrumentationScope::builder("test").build(),
        }
    }

    #[test]
    fn spans_to_payload_basic_structure() {
        let span = make_test_span(
            "session_init",
            vec![KeyValue::new("service.name", "snowflake-python")],
        );

        let payload = spans_to_snowflake_payload(&[span]);

        let logs = payload["logs"].as_array().unwrap();
        assert_eq!(logs.len(), 1);

        let entry = &logs[0];
        assert_eq!(entry["message"]["type"], "session_init");
        assert_eq!(entry["message"]["service.name"], "snowflake-python");
        assert_eq!(entry["timestamp"], "1700000000000");
    }

    #[test]
    fn spans_to_payload_empty_batch() {
        let payload = spans_to_snowflake_payload(&[]);
        let logs = payload["logs"].as_array().unwrap();
        assert!(logs.is_empty());
    }

    #[test]
    fn span_boolean_attribute() {
        let span = make_test_span(
            "session_init",
            vec![KeyValue::new("snowflake.driver.is_ci_cd", true)],
        );

        let payload = spans_to_snowflake_payload(&[span]);
        assert_eq!(
            payload["logs"][0]["message"]["snowflake.driver.is_ci_cd"],
            true
        );
    }

    #[test]
    fn span_numeric_attribute() {
        let span = make_test_span("session_init", vec![KeyValue::new("login_timeout", 30_i64)]);

        let payload = spans_to_snowflake_payload(&[span]);
        assert_eq!(payload["logs"][0]["message"]["login_timeout"], 30);
    }
}
