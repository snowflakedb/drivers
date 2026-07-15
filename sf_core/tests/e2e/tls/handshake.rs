use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use rcgen::generate_simple_self_signed;
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use sf_core::tls::config::TlsConfig;
use sf_core::tls::create_tls_client_with_config;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Spawns a TLS-terminating proxy backed by a wiremock `MockServer` (200 for all
/// requests). Returns the proxy's listen address and a `NamedTempFile` containing
/// the self-signed root certificate PEM for "localhost".
///
/// The returned `NamedTempFile` must be kept alive for the PEM file on disk to
/// remain valid.
async fn spawn_tls_proxy() -> (SocketAddr, tempfile::NamedTempFile) {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let cert = generate_simple_self_signed(vec!["localhost".to_string()]).expect("cert generation");
    let cert_pem = cert.cert.pem();

    let cert_der = CertificateDer::from(cert.cert);
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der()));

    let mut pem_file = tempfile::NamedTempFile::new().expect("temp file");
    pem_file.write_all(cert_pem.as_bytes()).expect("write PEM");
    pem_file.flush().expect("flush");

    let backend = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .mount(&backend)
        .await;

    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)
        .expect("server config");
    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let proxy_addr = listener.local_addr().unwrap();
    let backend_addr = *backend.address();

    tokio::spawn(async move {
        let _backend = backend;
        loop {
            let Ok((tcp, _)) = listener.accept().await else {
                continue;
            };
            let acc = acceptor.clone();
            tokio::spawn(async move {
                let Ok(mut tls) = acc.accept(tcp).await else {
                    return;
                };
                let Ok(mut upstream) = tokio::net::TcpStream::connect(backend_addr).await else {
                    return;
                };
                let _ = tokio::io::copy_bidirectional(&mut tls, &mut upstream).await;
            });
        }
    });

    (proxy_addr, pem_file)
}

#[tokio::test]
async fn should_complete_handshake_with_default_roots() {
    // Given a TLS client configured with default roots
    let server_url = std::env::var("E2E_TLS_SERVER").unwrap_or("https://example.com".to_string());

    // When GET request is sent to the server URL
    let client =
        create_tls_client_with_config(TlsConfig::default(), sf_core::crl::CrlWorker::shared_lazy())
            .expect("client");
    let resp = client.get(server_url).send().await;

    // Then the request attempt should be successful
    assert!(resp.is_ok());
}

#[tokio::test]
async fn should_complete_handshake_with_custom_pem_roots() {
    // Given E2E_TLS_ROOTS_PEM is set to a PEM bundle path
    if let Ok(pem_path) = std::env::var("E2E_TLS_ROOTS_PEM") {
        // And a TLS client configured with that custom root store
        let cfg = TlsConfig {
            custom_root_store_path: Some(pem_path.into()),
            ..Default::default()
        };
        let client = create_tls_client_with_config(cfg, sf_core::crl::CrlWorker::shared_lazy())
            .expect("client");
        let server_url =
            std::env::var("E2E_TLS_SERVER").unwrap_or("https://example.com".to_string());

        // When GET request is sent to the server URL
        let resp = client.get(server_url).send().await;

        // Then the request attempt should be successful
        assert!(resp.is_ok(), "Custom PEM roots should enable TLS handshake");
    }
}

#[tokio::test]
async fn should_trust_custom_root_store_when_crl_disabled() {
    let (proxy_addr, pem_file) = spawn_tls_proxy().await;
    let port = proxy_addr.port();

    // When a TLS client is created with custom_root_store_path and CRL disabled (default)
    let cfg = TlsConfig {
        custom_root_store_path: Some(pem_file.path().to_path_buf()),
        ..Default::default()
    };
    let client =
        create_tls_client_with_config(cfg, sf_core::crl::CrlWorker::shared_lazy()).expect("client");
    let resp = client.get(format!("https://localhost:{port}")).send().await;

    // Then the handshake succeeds because the custom root store is applied
    assert!(
        resp.is_ok(),
        "Custom root store must be applied when CRL is disabled"
    );
}

#[tokio::test]
async fn should_skip_hostname_verification_when_disabled() {
    let (proxy_addr, pem_file) = spawn_tls_proxy().await;
    let port = proxy_addr.port();
    let pem_path: PathBuf = pem_file.path().to_path_buf();

    // When connecting as 127.0.0.1 (hostname mismatch: cert says "localhost")
    // with verify_hostname=false and the custom root PEM
    let cfg = TlsConfig {
        custom_root_store_path: Some(pem_path.clone()),
        verify_hostname: false,
        ..Default::default()
    };
    let client =
        create_tls_client_with_config(cfg, sf_core::crl::CrlWorker::shared_lazy()).expect("client");
    let resp = client.get(format!("https://127.0.0.1:{port}")).send().await;

    // Then the handshake succeeds despite hostname mismatch
    assert!(
        resp.is_ok(),
        "verify_hostname=false should allow hostname mismatch"
    );

    // And with verify_hostname=true the same connection should fail
    let cfg_strict = TlsConfig {
        custom_root_store_path: Some(pem_path),
        verify_hostname: true,
        ..Default::default()
    };
    let client_strict =
        create_tls_client_with_config(cfg_strict, sf_core::crl::CrlWorker::shared_lazy())
            .expect("client");
    let resp_strict = client_strict
        .get(format!("https://127.0.0.1:{port}"))
        .send()
        .await;

    assert!(
        resp_strict.is_err(),
        "verify_hostname=true should reject hostname mismatch"
    );
}
