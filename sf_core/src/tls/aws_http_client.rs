//! TLS-configured AWS HTTP client.
//!
//! `aws-smithy-http-client`'s TLS API (`TlsContext` / `Provider`) configures
//! only the crypto provider and trust store — it always builds its
//! `rustls::ClientConfig` with `with_safe_default_protocol_versions()` and
//! exposes no min/max protocol-version knob, no CRL hook, and no custom root
//! store. To honour the connection's full [`TlsConfig`] on the AWS SDK paths
//! (S3 PUT/GET and the credential chain), we hand-build a `hyper-util` client
//! over a `hyper-rustls` HTTPS connector and adapt it to smithy's
//! [`HttpClient`] trait.

use aws_smithy_runtime_api::client::http::{
    HttpClient, HttpConnector, HttpConnectorFuture, HttpConnectorSettings, SharedHttpConnector,
};
use aws_smithy_runtime_api::client::orchestrator::{HttpRequest, HttpResponse};
use aws_smithy_runtime_api::client::result::ConnectorError;
use aws_smithy_runtime_api::client::runtime_components::RuntimeComponents;
use aws_smithy_types::body::SdkBody;
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::client::legacy::connect::HttpConnector as HyperHttpConnector;
use hyper_util::rt::TokioExecutor;

use std::sync::Arc;

use crate::crl::config::CertRevocationCheckMode;
use crate::crl::worker::SharedCrlWorker;
use crate::tls::CrlServerCertVerifier;
use crate::tls::client::create_root_store_from_pem;
use crate::tls::config::TlsConfig;

type PinnedHyperClient = HyperClient<HttpsConnector<HyperHttpConnector>, SdkBody>;

/// Adapts a `hyper-util` client to smithy's [`HttpConnector`]. The smithy
/// `Request`/`Response` types convert to/from `http` 1.x directly, so the
/// adapter is a thin request → hyper → response hop with error mapping.
#[derive(Clone, Debug)]
struct TlsConfiguredConnector {
    client: PinnedHyperClient,
}

impl HttpConnector for TlsConfiguredConnector {
    fn call(&self, request: HttpRequest) -> HttpConnectorFuture {
        let client = self.client.clone();
        HttpConnectorFuture::new(async move {
            let request = request
                .try_into_http1x()
                .map_err(|err| ConnectorError::user(err.into()))?;
            let response = client
                .request(request)
                .await
                .map_err(|err| ConnectorError::io(err.into()))?;
            HttpResponse::try_from(response.map(SdkBody::from_body_1_x))
                .map_err(|err| ConnectorError::other(err.into(), None))
        })
    }
}

#[derive(Clone, Debug)]
struct TlsConfiguredHttpClient {
    connector: SharedHttpConnector,
}

impl HttpClient for TlsConfiguredHttpClient {
    fn http_connector(
        &self,
        _settings: &HttpConnectorSettings,
        _components: &RuntimeComponents,
    ) -> SharedHttpConnector {
        self.connector.clone()
    }
}

/// Returns an AWS [`HttpClient`] that applies the connection's full
/// [`TlsConfig`] (version window, CRL, custom root store). Always injects a
/// custom hyper/rustls client so every S3 PUT/GET connection honours the
/// driver's TLS policy uniformly.
pub(crate) fn tls_configured_aws_http_client(
    tls_config: &TlsConfig,
    crl_worker: SharedCrlWorker,
) -> impl HttpClient + 'static {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let client_config = build_aws_rustls_config(tls_config, crl_worker);

    let mut http = HyperHttpConnector::new();
    http.enforce_http(false);
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(client_config)
        .https_or_http()
        .enable_http1()
        .enable_http2()
        .wrap_connector(http);

    let client: PinnedHyperClient = HyperClient::builder(TokioExecutor::new()).build(https);

    TlsConfiguredHttpClient {
        connector: SharedHttpConnector::new(TlsConfiguredConnector { client }),
    }
}

/// Build a `rustls::ClientConfig` for the AWS hyper client, honoring the full
/// [`TlsConfig`]. Falls back to the full TLS 1.2–1.3 window with native certs
/// if the window is somehow empty (a validated invariant that should not occur).
fn build_aws_rustls_config(
    tls_config: &TlsConfig,
    crl_worker: SharedCrlWorker,
) -> rustls::ClientConfig {
    let versions = tls_config.versions.enabled_rustls_versions();

    match tls_config.crl_config.check_mode {
        CertRevocationCheckMode::Disabled => {
            let mut roots = rustls::RootCertStore::empty();
            if let Some(pem_path) = tls_config.custom_root_store_path.as_ref() {
                match std::fs::read(pem_path).map(|pem| create_root_store_from_pem(&pem)) {
                    Ok(Ok(store)) => roots = store,
                    Ok(Err(e)) => tracing::error!("failed to load custom root store: {e}"),
                    Err(e) => tracing::error!("failed to read custom root store file: {e}"),
                }
            } else {
                let native = rustls_native_certs::load_native_certs();
                for cert in native.certs {
                    let _ = roots.add(cert);
                }
            }
            build_with_versions(&versions)
                .with_root_certificates(roots)
                .with_no_client_auth()
        }
        CertRevocationCheckMode::Enabled | CertRevocationCheckMode::Advisory => {
            tracing::debug!("CRL validation enabled for AWS HTTP client");
            let custom_root_store = tls_config.custom_root_store_path.as_ref().and_then(|p| {
                match std::fs::read(p).map(|pem| create_root_store_from_pem(&pem)) {
                    Ok(Ok(store)) => Some(store),
                    Ok(Err(e)) => {
                        tracing::error!("failed to load custom root store: {e}");
                        None
                    }
                    Err(e) => {
                        tracing::error!("failed to read custom root store file: {e}");
                        None
                    }
                }
            });
            match CrlServerCertVerifier::new_with_root_store(
                tls_config.crl_config.clone(),
                custom_root_store,
                tls_config.verify_hostname,
                crl_worker.clone(),
            ) {
                Ok(v) => build_with_versions(&versions)
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(v))
                    .with_no_client_auth(),
                Err(e) => {
                    tracing::error!("failed to build CRL verifier for AWS client: {e}");
                    build_aws_rustls_config(&TlsConfig::default(), crl_worker)
                }
            }
        }
    }
}

fn build_with_versions(
    versions: &[&'static rustls::SupportedProtocolVersion],
) -> rustls::ConfigBuilder<rustls::ClientConfig, rustls::WantsVerifier> {
    if versions.is_empty() {
        tracing::warn!("empty TLS version window; falling back to rustls defaults");
        rustls::ClientConfig::builder()
    } else {
        rustls::ClientConfig::builder_with_protocol_versions(versions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tls::config::{TlsVersion, TlsVersions};

    use crate::crl::worker::CrlWorker;

    #[test]
    fn builds_client_for_default_config() {
        // Default config must also produce a custom client now (always injected).
        let _ = tls_configured_aws_http_client(&TlsConfig::default(), CrlWorker::new_lazy());
    }

    #[test]
    fn builds_client_for_tls13_only_window() {
        let cfg = TlsConfig {
            versions: TlsVersions {
                min: TlsVersion::Tls13,
                max: TlsVersion::Tls13,
            },
            ..TlsConfig::default()
        };
        let _ = tls_configured_aws_http_client(&cfg, CrlWorker::new_lazy());
    }

    #[test]
    fn builds_client_for_tls12_only_window() {
        let cfg = TlsConfig {
            versions: TlsVersions {
                min: TlsVersion::Tls12,
                max: TlsVersion::Tls12,
            },
            ..TlsConfig::default()
        };
        let _ = tls_configured_aws_http_client(&cfg, CrlWorker::new_lazy());
    }
}
