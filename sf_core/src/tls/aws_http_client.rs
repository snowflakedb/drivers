//! AWS SDK HTTP client backed by the shared reqwest transport.
//!
//! `aws-smithy-http-client`'s TLS API configures only the crypto provider and
//! trust store — no min/max protocol-version knob, no CRL hook, no custom root
//! store — so it cannot honour the connection's full [`TlsConfig`]. This adapter
//! instead hands the AWS SDK an [`HttpClient`] over the same `reqwest::Client`
//! Azure and GCS transfers build via [`configure_tls_builder`], so S3 inherits
//! one implementation of the connection's TLS policy (version window, CRL, custom
//! root store) and proxy handling (`proxy_host`/`proxy_port`/`no_proxy`/
//! `use_proxy_env`, HTTPS CONNECT-tunnelling, `HTTP_PROXY`/`HTTPS_PROXY` fallback).
//!
//! Three `reqwest` defaults are adjusted for the SDK:
//! - redirect following — the SDK owns signing and retries, and a SigV4-signed
//!   request cannot be redirected without re-signing;
//! - gzip decompression — the SDK does its own content-encoding handling and
//!   expects response bodies exactly as they arrived on the wire;
//! - HTTP version — pinned to HTTP/1.1 to match the AWS SDK's default connector
//!   rather than negotiating HTTP/2 via the enabled `http2` feature.

use aws_smithy_runtime_api::client::http::{
    HttpClient, HttpConnector, HttpConnectorFuture, HttpConnectorSettings, SharedHttpConnector,
};
use aws_smithy_runtime_api::client::orchestrator::{HttpRequest, HttpResponse};
use aws_smithy_runtime_api::client::result::ConnectorError;
use aws_smithy_runtime_api::client::runtime_components::RuntimeComponents;
use aws_smithy_types::body::SdkBody;
use snafu::ResultExt;

use crate::crl::worker::SharedCrlWorker;
use crate::tls::client::configure_tls_builder;
use crate::tls::config::{ProxyConfig, TlsConfig};
use crate::tls::error::{ClientBuildSnafu, TlsError};

/// Builds the `reqwest::Client` that backs the S3 [`HttpClient`] adapter.
///
/// Delegates to [`configure_tls_builder`] — the exact TLS + proxy path Azure and
/// GCS transfers use — so S3 gets identical `TlsConfig`/`ProxyConfig` handling,
/// then adjusts three `reqwest` defaults the AWS SDK must own itself: redirect
/// following (SigV4 re-signing), gzip auto-decompression, and the HTTP protocol
/// version. The version is pinned to HTTP/1.1 (`http1_only`) so this client
/// matches the AWS SDK's own default connector rather than negotiating HTTP/2
/// via the enabled `http2` feature — pinning here, at the S3-specific call site,
/// leaves the shared Azure/GCS path untouched. No request-level `.timeout()` is
/// set: the SDK's `TimeoutConfig` (`operation_attempt_timeout`/`operation_timeout`)
/// governs S3 request timing.
///
/// Connection-pool tuning is deliberately left at `reqwest`'s defaults (no
/// `pool_idle_timeout`/`pool_max_idle_per_host`/`tcp_keepalive`), matching
/// Azure/GCS on this shared entry point; only the GS/REST client tunes the pool
/// (via `configure_http_client`).
pub(crate) fn build_s3_reqwest_client(
    tls_config: &TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_worker: SharedCrlWorker,
) -> Result<reqwest::Client, TlsError> {
    configure_tls_builder(reqwest::Client::builder(), tls_config, proxy, crl_worker)?
        .redirect(reqwest::redirect::Policy::none())
        .no_gzip()
        .http1_only()
        .build()
        .context(ClientBuildSnafu)
}

/// Wraps a `reqwest::Client` in an [`HttpClient`] the AWS SDK can consume via
/// `aws_config::defaults(...).http_client(...)`.
pub(crate) fn reqwest_aws_http_client(client: reqwest::Client) -> impl HttpClient + 'static {
    ReqwestHttpClient {
        connector: SharedHttpConnector::new(ReqwestConnector { client }),
    }
}

/// Adapts a `reqwest::Client` to smithy's [`HttpConnector`]: convert the smithy
/// request into a `reqwest::Request`, execute it, and convert the response back.
/// Request and response bodies are *wrapped*, never buffered, so large multipart
/// PUT uploads and ranged GET downloads stream — and http-body trailer frames
/// (e.g. the `aws-chunked` checksum trailer on PUT) survive the round-trip.
#[derive(Clone, Debug)]
struct ReqwestConnector {
    client: reqwest::Client,
}

impl HttpConnector for ReqwestConnector {
    fn call(&self, request: HttpRequest) -> HttpConnectorFuture {
        let client = self.client.clone();
        HttpConnectorFuture::new(async move {
            // smithy request → `http::Request<SdkBody>` → `reqwest::Request`,
            // wrapping (not draining) the body so streaming and trailers survive.
            let (parts, body) = request
                .try_into_http1x()
                .map_err(|err| ConnectorError::user(err.into()))?
                .into_parts();
            let request = http::Request::from_parts(parts, reqwest::Body::wrap(body));
            let request = reqwest::Request::try_from(request)
                .map_err(|err| ConnectorError::user(err.into()))?;

            // Every S3 HTTP call the SDK makes funnels through here, so log it at
            // INFO (host + path only, no query string — it can carry presigned
            // auth) per ud-log-every-http-call-at-info, matching Azure/GCS.
            tracing::info!(
                method = %request.method(),
                host = request.url().host_str().unwrap_or("<none>"),
                path = %request.url().path(),
                "outbound HTTP call"
            );

            let response = client
                .execute(request)
                .await
                .map_err(|err| ConnectorError::io(err.into()))?;
            tracing::info!(status = response.status().as_u16(), "HTTP response");

            // `reqwest::Response` → `http::Response<SdkBody>` → smithy response.
            let response = http::Response::from(response).map(SdkBody::from_body_1_x);
            HttpResponse::try_from(response).map_err(|err| ConnectorError::other(err.into(), None))
        })
    }
}

#[derive(Clone, Debug)]
struct ReqwestHttpClient {
    connector: SharedHttpConnector,
}

impl HttpClient for ReqwestHttpClient {
    fn http_connector(
        &self,
        _settings: &HttpConnectorSettings,
        _components: &RuntimeComponents,
    ) -> SharedHttpConnector {
        self.connector.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crl::worker::CrlWorker;
    use crate::tls::config::{TlsVersion, TlsVersions};

    use std::io::Write as _;
    use std::net::SocketAddr;

    use http_body_util::BodyExt as _;
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::TcpListener;

    fn default_client() -> reqwest::Client {
        build_s3_reqwest_client(&TlsConfig::default(), None, CrlWorker::new_lazy())
            .expect("default S3 reqwest client must build")
    }

    /// Serves exactly one HTTP/1.1 request on a fresh loopback port, replying
    /// with `response` verbatim. Returns the address to target. Reads the whole
    /// request head first so `reqwest` sees a well-formed exchange.
    async fn serve_once(response: Vec<u8>) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback listener");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut head = Vec::with_capacity(256);
            let mut byte = [0u8; 1];
            loop {
                match socket.read(&mut byte).await {
                    Ok(0) | Err(_) => return,
                    Ok(_) => {}
                }
                head.push(byte[0]);
                if head.ends_with(b"\r\n\r\n") || head.len() > 8192 {
                    break;
                }
            }
            let _ = socket.write_all(&response).await;
            let _ = socket.flush().await;
            let _ = socket.shutdown().await;
        });
        addr
    }

    /// Drives one GET through the adapter against `addr` and returns the smithy
    /// response.
    async fn get_through_adapter(addr: SocketAddr) -> HttpResponse {
        let connector = ReqwestConnector {
            client: default_client(),
        };
        let request = HttpRequest::get(format!("http://{addr}/")).expect("valid request uri");
        connector
            .call(request)
            .await
            .expect("adapter call succeeds")
    }

    #[test]
    fn builds_client_for_default_config() {
        // Default config must build through the shared reqwest path.
        let _ = default_client();
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
        build_s3_reqwest_client(&cfg, None, CrlWorker::new_lazy())
            .expect("TLS 1.3-only window must build");
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
        build_s3_reqwest_client(&cfg, None, CrlWorker::new_lazy())
            .expect("TLS 1.2-only window must build");
    }

    #[test]
    fn builds_client_with_explicit_proxy() {
        let proxy = ProxyConfig {
            host: Some("proxy.internal".to_string()),
            port: Some(3128),
            ..Default::default()
        };
        build_s3_reqwest_client(&TlsConfig::default(), Some(&proxy), CrlWorker::new_lazy())
            .expect("explicit-proxy client must build");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn adapter_does_not_follow_redirects() {
        // A 3xx must surface to the SDK unfollowed: SigV4-signed requests cannot
        // be transparently redirected without re-signing.
        let addr = serve_once(
            b"HTTP/1.1 302 Found\r\n\
              Location: http://redirected.invalid/\r\n\
              Content-Length: 0\r\n\r\n"
                .to_vec(),
        )
        .await;

        let response = get_through_adapter(addr).await;

        assert_eq!(
            response.status().as_u16(),
            302,
            "adapter must return the 3xx verbatim, not follow it"
        );
        assert_eq!(
            response.headers().get("location"),
            Some("http://redirected.invalid/"),
            "the Location header proves the redirect was surfaced, not consumed"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn adapter_does_not_decompress_gzip_bodies() {
        // A `Content-Encoding: gzip` body must reach the SDK exactly as received:
        // the SDK does its own content-encoding handling.
        let plaintext = b"the SDK must receive these bytes still gzip-compressed";
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(plaintext).expect("gzip write");
        let gzipped = encoder.finish().expect("gzip finish");
        assert_ne!(gzipped, plaintext, "sanity: gzip output differs from input");

        let mut response = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Encoding: gzip\r\n\
             Content-Length: {}\r\n\r\n",
            gzipped.len()
        )
        .into_bytes();
        response.extend_from_slice(&gzipped);
        let addr = serve_once(response).await;

        let response = get_through_adapter(addr).await;

        assert_eq!(
            response.headers().get("content-encoding"),
            Some("gzip"),
            "content-encoding must be preserved, not stripped by auto-decode"
        );
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        assert_eq!(
            body.as_ref(),
            gzipped.as_slice(),
            "body must arrive still gzip-compressed, not auto-decompressed"
        );
    }

    /// Starts a TLS server restricted to exactly `versions`, TCP-relaying each
    /// completed handshake to a `wiremock` backend that answers `200 OK` (the
    /// same TLS-relay-to-wiremock shape `tests/e2e/tls/tls_version.rs` uses), so
    /// the client's `[min, max]` window alone decides whether a common TLS
    /// version exists. Returns the listen address and the self-signed cert PEM
    /// for the client's trust store.
    async fn spawn_tls_server_offering_only(
        versions: &[&'static rustls::SupportedProtocolVersion],
    ) -> (SocketAddr, String) {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let cert = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()])
            .expect("cert generation");
        let cert_pem = cert.cert.pem();

        let cert_der = rustls::pki_types::CertificateDer::from(cert.cert);
        let key_der = rustls::pki_types::PrivateKeyDer::Pkcs8(
            rustls::pki_types::PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der()),
        );
        let server_config = rustls::ServerConfig::builder_with_protocol_versions(versions)
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)
            .expect("server config");
        let acceptor = tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(server_config));

        let backend = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(wiremock::ResponseTemplate::new(200))
            .mount(&backend)
            .await;
        let backend_addr = *backend.address();

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback listener");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            let _backend = backend;
            loop {
                let Ok((tcp, _)) = listener.accept().await else {
                    continue;
                };
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    let Ok(mut tls) = acceptor.accept(tcp).await else {
                        return;
                    };
                    let Ok(mut upstream) = tokio::net::TcpStream::connect(backend_addr).await
                    else {
                        return;
                    };
                    let _ = tokio::io::copy_bidirectional(&mut tls, &mut upstream).await;
                });
            }
        });
        (addr, cert_pem)
    }

    /// The adapter must enforce the connection's TLS protocol-version window at
    /// the wire level: `configure_tls_builder` derives the rustls `ClientConfig`
    /// from `TlsConfig::versions.enabled_rustls_versions()`, so a TLS 1.3 floor
    /// rejects a TLS-1.2-only server.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn adapter_tls_version_window_rejects_handshake_below_floor() {
        let (addr, cert_pem) = spawn_tls_server_offering_only(&[&rustls::version::TLS12]).await;
        let mut cert_file = tempfile::NamedTempFile::new().expect("temp file");
        cert_file.write_all(cert_pem.as_bytes()).expect("write pem");
        cert_file.flush().expect("flush");
        let url = format!("https://127.0.0.1:{}/", addr.port());

        // A TLS 1.3-only floor against a server that only offers TLS 1.2 must
        // fail the handshake.
        let narrow_client = build_s3_reqwest_client(
            &TlsConfig {
                custom_root_store_path: Some(cert_file.path().to_path_buf()),
                versions: TlsVersions {
                    min: TlsVersion::Tls13,
                    max: TlsVersion::Tls13,
                },
                ..TlsConfig::default()
            },
            None,
            CrlWorker::new_lazy(),
        )
        .expect("client must build");
        let resp = narrow_client.get(&url).send().await;
        assert!(
            resp.is_err(),
            "server offers only TLS 1.2; adapter's TLS 1.3 floor must reject the handshake, got: {resp:?}"
        );

        // Positive control: the identical server and root store, but with the
        // default window (which includes TLS 1.2), must succeed — proving the
        // rejection above is specifically the version floor, not a cert or
        // connectivity problem.
        let permissive_client = build_s3_reqwest_client(
            &TlsConfig {
                custom_root_store_path: Some(cert_file.path().to_path_buf()),
                ..TlsConfig::default()
            },
            None,
            CrlWorker::new_lazy(),
        )
        .expect("client must build");
        let resp = permissive_client.get(&url).send().await;
        assert!(
            resp.is_ok(),
            "same TLS 1.2 server must succeed once TLS 1.2 is inside the window: {resp:?}"
        );
    }

    /// Starts a TLS server that advertises ALPN `["h2", "http/1.1"]` with `h2`
    /// preferred, and serves one HTTP/1.1 `200 OK`. Returns the listen address,
    /// the self-signed cert PEM for the client trust store, and a receiver that
    /// yields the ALPN protocol the server negotiated. A client offering `h2`
    /// would make the server pick `h2`; a client that offers only `http/1.1`
    /// leaves `http/1.1` as the sole common protocol.
    async fn spawn_tls_server_alpn_h2_first() -> (
        SocketAddr,
        String,
        tokio::sync::oneshot::Receiver<Option<Vec<u8>>>,
    ) {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let cert = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()])
            .expect("cert generation");
        let cert_pem = cert.cert.pem();
        let cert_der = rustls::pki_types::CertificateDer::from(cert.cert);
        let key_der = rustls::pki_types::PrivateKeyDer::Pkcs8(
            rustls::pki_types::PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der()),
        );
        let mut server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)
            .expect("server config");
        server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        let acceptor = tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(server_config));

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback listener");
        let addr = listener.local_addr().expect("local addr");
        let (alpn_tx, alpn_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let Ok((tcp, _)) = listener.accept().await else {
                return;
            };
            let Ok(mut tls) = acceptor.accept(tcp).await else {
                let _ = alpn_tx.send(None);
                return;
            };
            let _ = alpn_tx.send(tls.get_ref().1.alpn_protocol().map(<[u8]>::to_vec));
            // Drain the request head, then reply with a fixed HTTP/1.1 response.
            let mut head = Vec::with_capacity(256);
            let mut byte = [0u8; 1];
            loop {
                match tls.read(&mut byte).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                head.push(byte[0]);
                if head.ends_with(b"\r\n\r\n") || head.len() > 8192 {
                    break;
                }
            }
            let _ = tls
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .await;
            let _ = tls.flush().await;
            let _ = tls.shutdown().await;
        });
        (addr, cert_pem, alpn_rx)
    }

    /// `build_s3_reqwest_client` pins the client to HTTP/1.1 (`.http1_only()`) so
    /// it never negotiates HTTP/2 with S3, even though the `http2` feature is
    /// compiled in. Against a TLS server offering both `h2` (preferred) and
    /// `http/1.1`, a client that still advertised `h2` would negotiate it — so
    /// the exchange coming back over HTTP/1.1, and the server seeing `http/1.1`
    /// as the negotiated ALPN, together prove the pin holds. Dropping
    /// `.http1_only()` flips ALPN to `h2` and fails both assertions.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn adapter_pins_http1_even_when_server_offers_h2() {
        let (addr, cert_pem, alpn_rx) = spawn_tls_server_alpn_h2_first().await;
        let mut cert_file = tempfile::NamedTempFile::new().expect("temp file");
        cert_file.write_all(cert_pem.as_bytes()).expect("write pem");
        cert_file.flush().expect("flush");

        let client = build_s3_reqwest_client(
            &TlsConfig {
                custom_root_store_path: Some(cert_file.path().to_path_buf()),
                ..TlsConfig::default()
            },
            None,
            CrlWorker::new_lazy(),
        )
        .expect("client must build");

        let url = format!("https://127.0.0.1:{}/", addr.port());
        let response =
            tokio::time::timeout(std::time::Duration::from_secs(10), client.get(&url).send())
                .await
                .expect("request must not hang")
                .expect("HTTP/1.1 request must succeed");

        assert_eq!(
            response.version(),
            http::Version::HTTP_11,
            "S3 client must speak HTTP/1.1, got {:?}",
            response.version(),
        );

        let negotiated = alpn_rx.await.expect("server reports negotiated ALPN");
        assert_eq!(
            negotiated.as_deref(),
            Some(b"http/1.1".as_ref()),
            "client pinned to HTTP/1.1 must offer only http/1.1 in ALPN; \
             server offering h2+http/1.1 negotiated {negotiated:?}",
        );
    }
}
