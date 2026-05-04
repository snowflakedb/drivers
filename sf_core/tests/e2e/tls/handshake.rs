use std::io::Write;
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

#[tokio::test]
async fn should_complete_handshake_with_default_roots() {
    // Given a TLS client configured with default roots
    let server_url = std::env::var("E2E_TLS_SERVER").unwrap_or("https://example.com".to_string());

    // When GET request is sent to the server URL
    let client = create_tls_client_with_config(TlsConfig::default()).expect("client");
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
        let client = create_tls_client_with_config(cfg).expect("client");
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
    // Given a self-signed certificate not in any system trust store
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let cert = generate_simple_self_signed(vec!["localhost".to_string()]).expect("cert generation");
    let cert_pem = cert.cert.pem();

    let cert_der = CertificateDer::from(cert.cert);
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der()));

    // And the certificate PEM is written to a temp file
    let mut pem_file = tempfile::NamedTempFile::new().expect("temp file");
    pem_file.write_all(cert_pem.as_bytes()).expect("write PEM");
    pem_file.flush().expect("flush");

    // And a mock HTTP backend returns 200
    let backend = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .mount(&backend)
        .await;

    // And a TLS proxy terminates TLS with the self-signed cert
    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)
        .expect("server config");
    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let proxy_addr = listener.local_addr().unwrap();
    let backend_addr = *backend.address();

    tokio::spawn(async move {
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

    // When a TLS client is created with custom_root_store_path and CRL disabled (default)
    let cfg = TlsConfig {
        custom_root_store_path: Some(pem_file.path().to_path_buf()),
        ..Default::default()
    };
    let client = create_tls_client_with_config(cfg).expect("client");
    let port = proxy_addr.port();
    let resp = client.get(format!("https://localhost:{port}")).send().await;

    // Then the handshake succeeds because the custom root store is applied
    assert!(
        resp.is_ok(),
        "Custom root store must be applied when CRL is disabled"
    );
}
