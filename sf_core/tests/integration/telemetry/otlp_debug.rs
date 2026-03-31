//! Smoke test: creates real OTel spans and metrics via a TracerProvider /
//! MeterProvider wired to the OTLP debug exporter, then flushes them.
//!
//! Run with Jaeger to see the events:
//!
//! ```sh
//! ./scripts/run_jaeger.sh                             # terminal 1
//! cargo test -p sf_core --test integration_tests      \
//!   --features otlp_debug                             \
//!   telemetry::otlp_debug                             \
//!   -- --ignored                                      # terminal 2
//! ```
//!
//! Then open http://localhost:16686 and look for service "sf_core_test".

use opentelemetry::KeyValue;
use opentelemetry::metrics::MeterProvider;
use opentelemetry::trace::{Tracer, TracerProvider};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::SdkTracerProvider;
use sf_core::telemetry::otlp_debug;
use std::time::Duration;

fn test_resource() -> Resource {
    Resource::builder()
        .with_service_name("sf_core_test")
        .build()
}

#[tokio::test]
#[ignore] // requires a running Jaeger instance
async fn spans_appear_in_jaeger() {
    let span_exporter =
        otlp_debug::otlp_span_exporter().expect("failed to build OTLP span exporter");

    let tracer_provider = SdkTracerProvider::builder()
        .with_resource(test_resource())
        .with_batch_exporter(span_exporter)
        .build();

    let tracer = tracer_provider.tracer("sf_core_test");

    // Simulate a session_init span
    {
        use opentelemetry::trace::SpanKind;
        let mut span = tracer
            .span_builder("session_init")
            .with_kind(SpanKind::Internal)
            .with_attributes(vec![
                KeyValue::new("snowflake.driver.name", "universal-driver"),
                KeyValue::new("snowflake.driver.version", env!("CARGO_PKG_VERSION")),
                KeyValue::new("os.type", std::env::consts::OS),
                KeyValue::new("os.arch", std::env::consts::ARCH),
            ])
            .start(&tracer);

        // Add an exception event
        use opentelemetry::trace::Span;
        span.add_event(
            "exception",
            vec![
                KeyValue::new("exception.type", "RuntimeError"),
                KeyValue::new("exception.message", "test error for Jaeger"),
            ],
        );
    } // span ends here

    // Simulate a driver_exception span
    {
        let _span = tracer
            .span_builder("driver_exception")
            .with_attributes(vec![
                KeyValue::new("exception.type", "NetworkError"),
                KeyValue::new("error.source", "connection timeout"),
            ])
            .start(&tracer);
    }

    // Flush is best-effort — collector may not be running in all environments.
    let _ = tracer_provider.force_flush();
    let _ = tracer_provider.shutdown();
}

#[tokio::test]
#[ignore] // requires a running Jaeger instance
async fn metrics_appear_in_jaeger() {
    let metric_exporter =
        otlp_debug::otlp_metric_exporter().expect("failed to build OTLP metric exporter");

    let reader = PeriodicReader::builder(metric_exporter)
        .with_interval(Duration::from_secs(1))
        .build();

    let meter_provider = SdkMeterProvider::builder()
        .with_resource(test_resource())
        .with_reader(reader)
        .build();

    let meter = meter_provider.meter("sf_core_test");

    // Simulate API call counters
    let counter = meter.u64_counter("snowflake.driver.api.call").build();
    counter.add(3, &[KeyValue::new("api_method", "execute")]);
    counter.add(1, &[KeyValue::new("api_method", "fetch")]);
    counter.add(2, &[KeyValue::new("api_method", "close_session")]);

    // Flush is best-effort — collector may not be running in all environments.
    let _ = meter_provider.force_flush();
    let _ = meter_provider.shutdown();
}
