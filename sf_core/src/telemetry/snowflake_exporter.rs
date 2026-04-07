use std::sync::Arc;
use std::time::Duration;

use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::metrics::Temporality;
use opentelemetry_sdk::metrics::data::ResourceMetrics;
use opentelemetry_sdk::metrics::exporter::PushMetricExporter;
use opentelemetry_sdk::trace::{SpanData, SpanExporter};
use tokio::sync::RwLock as AsyncRwLock;
use url::Url;

use crate::config::rest_parameters::ClientInfo;
use crate::rest::snowflake::SessionTokens;
use crate::rest::snowflake::telemetry as rest;

use super::serialization;

/// Shared session context that the exporter uses to POST telemetry.
pub struct ExporterSession {
    pub client: reqwest::Client,
    pub server_url: Url,
    pub client_info: ClientInfo,
    pub session_token: Arc<AsyncRwLock<Option<SessionTokens>>>,
}

impl std::fmt::Debug for ExporterSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExporterSession")
            .field("server_url", &self.server_url)
            .finish_non_exhaustive()
    }
}

/// Custom OTel exporter that sends telemetry to Snowflake's in-band `/telemetry/send` endpoint.
///
/// All errors are treated as non-fatal — telemetry must never break the user's workflow.
#[derive(Debug, Clone)]
pub struct SnowflakeInBandExporter {
    session: Arc<ExporterSession>,
}

impl SnowflakeInBandExporter {
    pub fn new(session: Arc<ExporterSession>) -> Self {
        Self { session }
    }
}

/// Clone the session token under a short-lived read guard, then send
/// telemetry without holding the lock across the network call.
async fn send_with_token(session: &ExporterSession, payload: &serde_json::Value) -> OTelSdkResult {
    let token = {
        let guard = session.session_token.read().await;
        match guard.as_ref() {
            Some(tokens) => tokens.session_token.clone(),
            None => {
                tracing::debug!("No active session token, dropping telemetry");
                return Ok(());
            }
        }
    };

    if let Err(e) = rest::send_telemetry(
        &session.client,
        &session.server_url,
        &session.client_info,
        token.reveal(),
        payload,
    )
    .await
    {
        tracing::warn!("Failed to export telemetry: {e}");
    }

    // Best-effort: always return Ok
    Ok(())
}

impl SpanExporter for SnowflakeInBandExporter {
    fn export(
        &self,
        batch: Vec<SpanData>,
    ) -> impl std::future::Future<Output = OTelSdkResult> + Send {
        let session = self.session.clone();
        async move {
            if batch.is_empty() {
                return Ok(());
            }

            let payload = serialization::spans_to_snowflake_payload(&batch);
            send_with_token(&session, &payload).await
        }
    }

    fn shutdown_with_timeout(&mut self, _timeout: Duration) -> OTelSdkResult {
        Ok(())
    }
}

impl PushMetricExporter for SnowflakeInBandExporter {
    fn export(
        &self,
        metrics: &ResourceMetrics,
    ) -> impl std::future::Future<Output = OTelSdkResult> + Send {
        let session = self.session.clone();
        // Check if there are any metrics worth serializing before doing work.
        let has_metrics = metrics
            .scope_metrics()
            .any(|sm| sm.metrics().next().is_some());
        async move {
            if !has_metrics {
                return Ok(());
            }

            let payload = serialization::metrics_to_snowflake_payload(metrics);
            if payload["logs"].as_array().is_some_and(|a| a.is_empty()) {
                return Ok(());
            }

            send_with_token(&session, &payload).await
        }
    }

    fn force_flush(&self) -> OTelSdkResult {
        Ok(())
    }

    fn shutdown_with_timeout(&self, _timeout: Duration) -> OTelSdkResult {
        Ok(())
    }

    fn temporality(&self) -> Temporality {
        Temporality::Delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::InstrumentationScope;
    use opentelemetry::trace::{
        SpanContext, SpanId, SpanKind, Status, TraceFlags, TraceId, TraceState,
    };
    use serde_json::json;
    use std::time::UNIX_EPOCH;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_exporter_session(server_url: Url) -> Arc<ExporterSession> {
        use crate::crl::config::CrlConfig;
        use crate::tls::config::TlsConfig;

        use crate::sensitive::SensitiveString;

        let tokens = SessionTokens {
            session_token: SensitiveString::from("test_token"),
            master_token: SensitiveString::from("master_token"),
            session_id: 1,
            session_expires_at: None,
            master_expires_at: None,
        };

        Arc::new(ExporterSession {
            client: reqwest::Client::new(),
            server_url,
            client_info: ClientInfo {
                application: "TestApp".to_string(),
                version: "1.0.0".to_string(),
                os: "Linux".to_string(),
                os_version: "5.15".to_string(),
                ocsp_mode: None,
                crl_config: CrlConfig::default(),
                tls_config: TlsConfig::default(),
            },
            session_token: Arc::new(AsyncRwLock::new(Some(tokens))),
        })
    }

    fn make_test_span() -> SpanData {
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
            name: "test_span".into(),
            start_time: UNIX_EPOCH + std::time::Duration::from_millis(1700000000000),
            end_time: UNIX_EPOCH + std::time::Duration::from_millis(1700000001000),
            attributes: vec![],
            dropped_attributes_count: 0,
            events: Default::default(),
            links: Default::default(),
            status: Status::Ok,
            instrumentation_scope: InstrumentationScope::builder("test").build(),
        }
    }

    #[tokio::test]
    async fn span_exporter_posts_to_telemetry_endpoint() {
        let server = MockServer::start().await;
        let session = make_exporter_session(Url::parse(&server.uri()).unwrap());

        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
            .expect(1)
            .mount(&server)
            .await;

        let exporter = SnowflakeInBandExporter::new(session);
        let result = SpanExporter::export(&exporter, vec![make_test_span()]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn span_exporter_empty_batch_skips_post() {
        let server = MockServer::start().await;
        let session = make_exporter_session(Url::parse(&server.uri()).unwrap());

        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
            .expect(0)
            .mount(&server)
            .await;

        let exporter = SnowflakeInBandExporter::new(session);
        let result = SpanExporter::export(&exporter, vec![]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn span_exporter_no_session_token_drops_silently() {
        let server = MockServer::start().await;
        let session = Arc::new(ExporterSession {
            client: reqwest::Client::new(),
            server_url: Url::parse(&server.uri()).unwrap(),
            client_info: ClientInfo {
                application: "TestApp".to_string(),
                version: "1.0.0".to_string(),
                os: "Linux".to_string(),
                os_version: "5.15".to_string(),
                ocsp_mode: None,
                crl_config: crate::crl::config::CrlConfig::default(),
                tls_config: crate::tls::config::TlsConfig::default(),
            },
            session_token: Arc::new(AsyncRwLock::new(None)),
        });

        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;

        let exporter = SnowflakeInBandExporter::new(session);
        let result = SpanExporter::export(&exporter, vec![make_test_span()]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn span_exporter_server_error_returns_ok() {
        let server = MockServer::start().await;
        let session = make_exporter_session(Url::parse(&server.uri()).unwrap());

        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(500).set_body_string("error"))
            .mount(&server)
            .await;

        let exporter = SnowflakeInBandExporter::new(session);
        let result = SpanExporter::export(&exporter, vec![make_test_span()]).await;
        assert!(result.is_ok());
    }

    // -- PushMetricExporter unit tests --

    use opentelemetry::KeyValue;
    use opentelemetry::metrics::MeterProvider;
    use opentelemetry_sdk::metrics::ManualReader;
    use opentelemetry_sdk::metrics::Pipeline;
    use opentelemetry_sdk::metrics::reader::MetricReader;
    use std::sync::Weak;

    /// Wrapper around `Arc<ManualReader>` that implements `MetricReader`,
    /// allowing the reader to be shared between the `SdkMeterProvider` and
    /// test code that calls `collect()`.
    #[derive(Debug, Clone)]
    struct SharedManualReader(Arc<ManualReader>);

    impl MetricReader for SharedManualReader {
        fn register_pipeline(&self, pipeline: Weak<Pipeline>) {
            self.0.register_pipeline(pipeline);
        }
        fn collect(
            &self,
            rm: &mut opentelemetry_sdk::metrics::data::ResourceMetrics,
        ) -> opentelemetry_sdk::error::OTelSdkResult {
            self.0.collect(rm)
        }
        fn force_flush(&self) -> opentelemetry_sdk::error::OTelSdkResult {
            self.0.force_flush()
        }
        fn shutdown_with_timeout(
            &self,
            timeout: Duration,
        ) -> opentelemetry_sdk::error::OTelSdkResult {
            self.0.shutdown_with_timeout(timeout)
        }
        fn temporality(&self, kind: opentelemetry_sdk::metrics::InstrumentKind) -> Temporality {
            self.0.temporality(kind)
        }
    }

    fn make_exporter_session_no_token(server_url: Url) -> Arc<ExporterSession> {
        use crate::crl::config::CrlConfig;
        use crate::tls::config::TlsConfig;
        Arc::new(ExporterSession {
            client: reqwest::Client::new(),
            server_url,
            client_info: ClientInfo {
                application: "TestApp".to_string(),
                version: "1.0.0".to_string(),
                os: "Linux".to_string(),
                os_version: "5.15".to_string(),
                ocsp_mode: None,
                crl_config: CrlConfig::default(),
                tls_config: TlsConfig::default(),
            },
            session_token: Arc::new(AsyncRwLock::new(None)),
        })
    }

    fn collect_test_metrics(
        counter_name: &'static str,
        attrs: &[KeyValue],
    ) -> opentelemetry_sdk::metrics::data::ResourceMetrics {
        let reader = SharedManualReader(Arc::new(
            ManualReader::builder()
                .with_temporality(Temporality::Delta)
                .build(),
        ));
        let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
            .with_reader(reader.clone())
            .build();
        let meter = provider.meter("test");
        let counter = meter.u64_counter(counter_name).build();
        counter.add(1, attrs);

        let mut rm = opentelemetry_sdk::metrics::data::ResourceMetrics::default();
        reader.collect(&mut rm).unwrap();
        rm
    }

    #[tokio::test]
    async fn metric_exporter_posts_to_telemetry_endpoint() {
        let server = MockServer::start().await;
        let session = make_exporter_session(Url::parse(&server.uri()).unwrap());

        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
            .expect(1)
            .mount(&server)
            .await;

        let exporter = SnowflakeInBandExporter::new(session);
        let rm = collect_test_metrics(
            "snowflake.driver.api.call",
            &[KeyValue::new("api_method", "execute")],
        );
        let result = PushMetricExporter::export(&exporter, &rm).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn metric_exporter_empty_metrics_skips_post() {
        let server = MockServer::start().await;
        let session = make_exporter_session(Url::parse(&server.uri()).unwrap());

        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
            .expect(0)
            .mount(&server)
            .await;

        let exporter = SnowflakeInBandExporter::new(session);
        let empty = opentelemetry_sdk::metrics::data::ResourceMetrics::default();
        let result = PushMetricExporter::export(&exporter, &empty).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn metric_exporter_no_token_drops_silently() {
        let server = MockServer::start().await;
        let session = make_exporter_session_no_token(Url::parse(&server.uri()).unwrap());

        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;

        let exporter = SnowflakeInBandExporter::new(session);
        let rm = collect_test_metrics("test.counter", &[]);
        let result = PushMetricExporter::export(&exporter, &rm).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn metric_exporter_server_error_returns_ok() {
        let server = MockServer::start().await;
        let session = make_exporter_session(Url::parse(&server.uri()).unwrap());

        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(500).set_body_string("error"))
            .mount(&server)
            .await;

        let exporter = SnowflakeInBandExporter::new(session);
        let rm = collect_test_metrics("test.counter", &[]);
        let result = PushMetricExporter::export(&exporter, &rm).await;
        assert!(result.is_ok());
    }

    #[test]
    fn metric_exporter_temporality_is_delta() {
        let session = make_exporter_session(Url::parse("http://localhost:1").unwrap());
        let exporter = SnowflakeInBandExporter::new(session);
        assert_eq!(exporter.temporality(), Temporality::Delta);
    }
}
