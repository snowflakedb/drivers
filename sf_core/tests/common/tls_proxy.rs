//! TLS-terminating reverse proxy for testing HTTPS endpoints.
//!
//! Generates a self-signed certificate at startup using `rcgen` and forwards
//! decrypted traffic to a plain-HTTP backend (e.g. `wiremock::MockServer`).
//! This replaces the Java WireMock standalone JAR that was previously needed
//! solely for HTTPS support.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use rcgen::generate_simple_self_signed;
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;

/// A running TLS proxy that terminates TLS and forwards to a backend HTTP port.
pub struct TlsProxy {
    addr: SocketAddr,
    /// PEM of the self-signed cert this proxy presents, so callers can trust it
    /// via a `TlsConfig` custom root store (used by the proxy-transfer tests).
    cert_pem: String,
    accept_handle: tokio::task::JoinHandle<()>,
    child_handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl TlsProxy {
    /// Start a TLS reverse proxy on a random port with the default
    /// `localhost`/`127.0.0.1` SANs.
    ///
    /// Must be called within a tokio runtime context.  All accepted TLS
    /// connections are transparently forwarded (at the TCP byte level) to
    /// `backend_addr`.  The proxy runs as a background tokio task and lives
    /// for the duration of the runtime.
    pub async fn start(backend_addr: SocketAddr) -> Self {
        Self::start_with_sans(
            backend_addr,
            vec!["localhost".to_string(), "127.0.0.1".to_string()],
        )
        .await
    }

    /// Like [`Self::start`] but with caller-supplied subject-alternative-names,
    /// so a test can make the presented cert valid for a specific (possibly
    /// unresolvable) hostname it will dial through a CONNECT tunnel — letting
    /// TLS hostname verification genuinely pass instead of being disabled.
    pub async fn start_with_sans(backend_addr: SocketAddr, sans: Vec<String>) -> Self {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        let cert =
            generate_simple_self_signed(sans).expect("Failed to generate self-signed certificate");

        let cert_pem = cert.cert.pem();
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

        let child_handles = Arc::new(Mutex::new(Vec::new()));
        let children = Arc::clone(&child_handles);
        let accept_handle = tokio::spawn(async move {
            loop {
                let Ok((tcp_stream, _)) = listener.accept().await else {
                    continue;
                };
                let acceptor = acceptor.clone();
                let handle = tokio::spawn(async move {
                    let Ok(mut tls_stream) = acceptor.accept(tcp_stream).await else {
                        return;
                    };
                    let Ok(mut backend) = TcpStream::connect(backend_addr).await else {
                        return;
                    };
                    let _ = tokio::io::copy_bidirectional(&mut tls_stream, &mut backend).await;
                });
                children.lock().unwrap().push(handle);
            }
        });

        Self {
            addr,
            cert_pem,
            accept_handle,
            child_handles,
        }
    }

    pub fn url(&self) -> String {
        format!("https://localhost:{}", self.addr.port())
    }

    /// The loopback address this proxy listens on (for a CONNECT proxy to
    /// bridge tunneled bytes into).
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// PEM of the self-signed cert this proxy presents.
    pub fn cert_pem(&self) -> &str {
        &self.cert_pem
    }
}

impl Drop for TlsProxy {
    /// Aborts the accept-loop task and every per-connection task it spawned,
    /// so none of them keep running on the shared tokio runtime after this
    /// proxy goes out of scope at the end of a test (even one still mid-copy).
    fn drop(&mut self) {
        self.accept_handle.abort();
        for handle in self.child_handles.lock().unwrap().drain(..) {
            handle.abort();
        }
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

    /// Returns all requests the mock server recorded, in order.
    pub fn received_requests(&self) -> Vec<wiremock::Request> {
        self.runtime
            .block_on(self.server.received_requests())
            .unwrap_or_default()
    }
}
