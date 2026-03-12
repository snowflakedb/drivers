//! TLS-terminating reverse proxy for testing HTTPS endpoints.
//!
//! Generates a self-signed certificate at startup using `rcgen` and forwards
//! decrypted traffic to a plain-HTTP backend (e.g. `wiremock::MockServer`).
//! This replaces the Java WireMock standalone JAR that was previously needed
//! solely for HTTPS support.

use std::net::SocketAddr;
use std::sync::Arc;

use rcgen::generate_simple_self_signed;
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;

/// A running TLS proxy that terminates TLS and forwards to a backend HTTP port.
pub struct TlsProxy {
    addr: SocketAddr,
}

impl TlsProxy {
    /// Start a TLS reverse proxy on a random port.
    ///
    /// Must be called within a tokio runtime context.  All accepted TLS
    /// connections are transparently forwarded (at the TCP byte level) to
    /// `backend_addr`.  The proxy runs as a background tokio task and lives
    /// for the duration of the runtime.
    pub async fn start(backend_addr: SocketAddr) -> Self {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
        let cert = generate_simple_self_signed(subject_alt_names)
            .expect("Failed to generate self-signed certificate");

        let cert_der = CertificateDer::from(cert.cert);
        let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der()));

        let tls_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)
            .expect("Failed to build TLS ServerConfig");

        let acceptor = TlsAcceptor::from(Arc::new(tls_config));
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind TLS proxy listener");
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                let Ok((tcp_stream, _)) = listener.accept().await else {
                    continue;
                };
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    let Ok(mut tls_stream) = acceptor.accept(tcp_stream).await else {
                        return;
                    };
                    let Ok(mut backend) = TcpStream::connect(backend_addr).await else {
                        return;
                    };
                    let _ = tokio::io::copy_bidirectional(&mut tls_stream, &mut backend).await;
                });
            }
        });

        Self { addr }
    }

    pub fn url(&self) -> String {
        format!("https://localhost:{}", self.addr.port())
    }
}

/// Wraps a `wiremock::MockServer` + `TlsProxy` with a **dedicated** tokio
/// runtime, so synchronous test functions can mount mocks without conflicting
/// with the runtime that `sf_core` itself creates.
pub struct MockServerWithTls {
    server: wiremock::MockServer,
    tls_proxy: TlsProxy,
    runtime: tokio::runtime::Runtime,
}

impl MockServerWithTls {
    /// Spin up a mock server + TLS proxy on a fresh tokio runtime.
    pub fn start() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime for mock server");

        let (server, tls_proxy) = runtime.block_on(async {
            let server = wiremock::MockServer::start().await;
            let tls_proxy = TlsProxy::start(*server.address()).await;
            (server, tls_proxy)
        });

        Self {
            server,
            tls_proxy,
            runtime,
        }
    }

    /// The plain-HTTP URL (for Snowflake API endpoints).
    pub fn http_url(&self) -> String {
        self.server.uri()
    }

    /// The HTTPS URL (for Okta endpoints through the TLS proxy).
    pub fn https_url(&self) -> String {
        self.tls_proxy.url()
    }

    /// Mount a mock on the server (blocking).
    pub fn mount(&self, mock: wiremock::Mock) {
        self.runtime.block_on(mock.mount(&self.server));
    }
}
