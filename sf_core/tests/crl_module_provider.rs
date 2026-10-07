//! Default trust overrides and the process-global provider stay in a child
//! process so this regression cannot affect other TLS tests.

use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair, KeyUsagePurpose};
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use sf_core::crl::{CrlCache, config::CrlConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::time::timeout;
use tokio_rustls::TlsAcceptor;

#[tokio::test]
async fn https_crl_download_uses_module_not_process_default() {
    const CHILD_URL: &str = "SF_CORE_CRL_PROVIDER_TEST_URL";
    const BODY: &[u8] = b"CRL download transport fixture";

    if let Ok(url) = std::env::var(CHILD_URL) {
        let mut restricted = rustls::crypto::aws_lc_rs::default_provider();
        restricted
            .cipher_suites
            .retain(|suite| suite.version() == &rustls::version::TLS12);
        assert!(!restricted.cipher_suites.is_empty());
        restricted
            .install_default()
            .expect("isolated child must claim the global provider before the CRL client");

        let cache = CrlCache::new(CrlConfig {
            enable_memory_caching: false,
            enable_disk_caching: false,
            ..Default::default()
        })
        .expect("build CRL client");
        let body = timeout(Duration::from_secs(10), cache.get(&url))
            .await
            .expect("CRL download must finish")
            .expect("CRL download must negotiate TLS 1.3 despite the TLS 1.2 global provider");
        assert_eq!(body, BODY);
        return;
    }

    let ca_key = KeyPair::generate().expect("generate CA key");
    let mut ca_params = CertificateParams::new(vec![]).expect("CA parameters");
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
    let ca = ca_params.self_signed(&ca_key).expect("sign CA");
    let server_key = KeyPair::generate().expect("generate server key");
    let server_cert = CertificateParams::new(vec!["localhost".to_string()])
        .expect("server parameters")
        .signed_by(&server_key, &ca, &ca_key)
        .expect("sign server certificate");
    let mut pem = tempfile::NamedTempFile::new().expect("CA PEM file");
    pem.write_all(ca.pem().as_bytes()).expect("write CA PEM");
    pem.flush().expect("flush CA PEM");

    let server_config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .expect("TLS 1.3 server config")
    .with_no_client_auth()
    .with_single_cert(
        vec![server_cert.der().clone()],
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(server_key.serialize_der())),
    )
    .expect("server identity");
    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind server");
    let port = listener.local_addr().expect("server address").port();
    let server_task = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.expect("accept CRL client");
        let mut tls = acceptor.accept(tcp).await.expect("TLS 1.3 handshake");
        let mut request = [0u8; 2048];
        let size = tls.read(&mut request).await.expect("read CRL request");
        assert!(request[..size].starts_with(b"GET /issuer.crl HTTP/1.1"));
        let header = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", BODY.len());
        tls.write_all(header.as_bytes())
            .await
            .expect("write headers");
        tls.write_all(BODY).await.expect("write CRL body");
    });

    // rustls-native-certs honors SSL_CERT_FILE on every platform. Set it only
    // in the child: no unsafe mutation of the test runner's environment.
    let output = timeout(
        Duration::from_secs(20),
        Command::new(std::env::current_exe().expect("test executable"))
            .args([
                "--exact",
                "https_crl_download_uses_module_not_process_default",
            ])
            .env(CHILD_URL, format!("https://localhost:{port}/issuer.crl"))
            .env("SSL_CERT_FILE", pem.path())
            .env_remove("SSL_CERT_DIR")
            .env("NO_PROXY", "*")
            .env("no_proxy", "*")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("isolated CRL test must finish")
    .expect("run isolated CRL test");
    assert!(
        output.status.success(),
        "CRL provider regression failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    server_task.await.expect("CRL server task");
}
