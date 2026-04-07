//! Optional OTLP exporters for local debugging with Jaeger or any OTLP-compatible collector.
//!
//! Gated behind the `otlp_debug` feature flag. When enabled, the telemetry pipeline
//! can add these exporters alongside the Snowflake in-band exporter so that spans
//! and metrics are also sent to a local collector for real-time inspection.
//!
//! # Usage
//!
//! 1. Start Jaeger locally:
//!    ```sh
//!    ./scripts/run_jaeger.sh
//!    ```
//!
//! 2. Build with the feature enabled:
//!    ```sh
//!    cargo build -p sf_core --features otlp_debug
//!    ```
//!
//! 3. Open Jaeger UI at `http://localhost:16686`

use opentelemetry_otlp::WithExportConfig;

const DEFAULT_OTLP_HTTP_ENDPOINT: &str = "http://localhost:8318";

/// Create an OTLP span exporter targeting a local collector.
///
/// Uses `SF_OTLP_ENDPOINT` env var if set, otherwise defaults to
/// `http://localhost:8318` (Jaeger's OTLP HTTP port as mapped in `scripts/run_jaeger.sh`).
pub fn otlp_span_exporter()
-> Result<opentelemetry_otlp::SpanExporter, opentelemetry_otlp::ExporterBuildError> {
    let endpoint = endpoint();
    tracing::info!("OTLP debug span exporter → {endpoint}/v1/traces");

    opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(format!("{endpoint}/v1/traces"))
        .with_protocol(opentelemetry_otlp::Protocol::HttpJson)
        .build()
}

/// Create an OTLP metric exporter targeting a local collector.
///
/// Uses `SF_OTLP_ENDPOINT` env var if set, otherwise defaults to
/// `http://localhost:8318`. Metrics are exported to `{endpoint}/v1/metrics`.
pub fn otlp_metric_exporter()
-> Result<opentelemetry_otlp::MetricExporter, opentelemetry_otlp::ExporterBuildError> {
    let endpoint = endpoint();
    tracing::info!("OTLP debug metric exporter → {endpoint}/v1/metrics");

    opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_temporality(opentelemetry_sdk::metrics::Temporality::Delta)
        .with_endpoint(format!("{endpoint}/v1/metrics"))
        .build()
}

fn endpoint() -> String {
    std::env::var("SF_OTLP_ENDPOINT").unwrap_or_else(|_| DEFAULT_OTLP_HTTP_ENDPOINT.to_string())
}
