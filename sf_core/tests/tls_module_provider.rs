//! Isolated integration binary: no shared test ctor installs the global provider.
//! The process default deliberately cannot speak TLS 1.3, so a successful TLS
//! 1.3 connection proves reqwest received the linked module's config instead.

use std::io::Write;
use std::sync::Arc;

use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair, KeyUsagePurpose};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use sf_core::tls::config::{ProxyConfig, TlsConfig, TlsVersion, TlsVersions};
use sf_core::tls::create_tls_client_with_proxy;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

#[tokio::test]
async fn default_verified_client_uses_linked_module_not_process_default() {
    let mut restricted = rustls::crypto::aws_lc_rs::default_provider();
    restricted
        .cipher_suites
        .retain(|suite| suite.version() == &rustls::version::TLS12);
    assert!(
        !restricted.cipher_suites.is_empty(),
        "TLS 1.2 control provider must have suites"
    );
    restricted
        .install_default()
        .expect("this isolated test must claim the global provider before the driver");

    let ca_key = KeyPair::generate().expect("generate CA key");
    let mut ca_params = CertificateParams::new(vec![]).expect("create CA parameters");
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
    let ca = ca_params.self_signed(&ca_key).expect("self-sign CA");
    let key = KeyPair::generate().expect("generate server key");
    let server = CertificateParams::new(vec!["localhost".to_string()])
        .expect("create server parameters")
        .signed_by(&key, &ca, &ca_key)
        .expect("sign server certificate");
    let mut pem = tempfile::NamedTempFile::new().expect("create CA PEM");
    pem.write_all(ca.pem().as_bytes()).expect("write CA PEM");
    pem.flush().expect("flush CA PEM");

    // The server uses its own provider. Using ServerConfig::builder() would
    // inherit the restricted global and invalidate the positive control.
    let server_config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .expect("TLS 1.3 server config")
    .with_no_client_auth()
    .with_single_cert(
        vec![CertificateDer::from(server)],
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der())),
    )
    .expect("set server identity");
    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind server");
    let port = listener.local_addr().expect("server address").port();
    let server_task = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.expect("accept client");
        let mut tls = acceptor.accept(tcp).await.expect("TLS 1.3 handshake");
        let mut request = [0u8; 2048];
        let size = tls.read(&mut request).await.expect("receive HTTP request");
        assert!(
            request[..size].starts_with(b"GET / HTTP/1.1"),
            "TLS 1.3 server must receive the client's HTTP/1.1 GET"
        );
        tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
            .await
            .expect("respond over TLS");
    });

    let config = TlsConfig {
        custom_root_store_path: Some(pem.path().to_path_buf()),
        versions: TlsVersions {
            min: TlsVersion::Tls13,
            max: TlsVersion::Tls13,
        },
        ..Default::default()
    };
    let client = create_tls_client_with_proxy(
        config,
        Some(&ProxyConfig::default()),
        sf_core::crl::CrlManager::new(),
    )
    .expect("the module must build a TLS 1.3 client despite the TLS 1.2 global provider");
    let response = client
        .get(format!("https://localhost:{port}/"))
        .send()
        .await
        .expect("linked module must negotiate TLS 1.3 and trust the custom CA");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(response.text().await.expect("read HTTP response"), "ok");
    server_task.await.expect("server task");
}
