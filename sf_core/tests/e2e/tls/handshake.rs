use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair, KeyUsagePurpose,
};
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use sf_core::crl::config::CertRevocationCheckMode;
use sf_core::tls::config::TlsConfig;
use sf_core::tls::create_tls_client_with_config;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Spawns a TLS-terminating proxy backed by a wiremock `MockServer` (200 for all
/// requests). Returns the proxy's listen address and a `NamedTempFile` containing
/// the root certificate PEM used to sign the "localhost" certificate.
///
/// The returned `NamedTempFile` must be kept alive for the PEM file on disk to
/// remain valid.
async fn spawn_tls_proxy() -> (SocketAddr, tempfile::NamedTempFile) {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let ca_key = KeyPair::generate().expect("CA key generation");
    let mut ca_params = CertificateParams::new(vec![]).expect("CA parameters");
    let mut ca_name = DistinguishedName::new();
    ca_name.push(DnType::CommonName, "Test Root CA");
    ca_params.distinguished_name = ca_name;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let ca_cert = ca_params.self_signed(&ca_key).expect("CA certificate");
    let cert_pem = ca_cert.pem();

    let intermediate_key = KeyPair::generate().expect("intermediate key generation");
    let mut intermediate_params = CertificateParams::new(vec![]).expect("intermediate parameters");
    let mut intermediate_name = DistinguishedName::new();
    intermediate_name.push(DnType::CommonName, "Test Intermediate CA");
    intermediate_params.distinguished_name = intermediate_name;
    intermediate_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    intermediate_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let intermediate_cert = intermediate_params
        .signed_by(&intermediate_key, &ca_cert, &ca_key)
        .expect("intermediate certificate");

    let server_key = KeyPair::generate().expect("server key generation");
    let server_params =
        CertificateParams::new(vec!["localhost".to_string()]).expect("server parameters");
    let server_cert = server_params
        .signed_by(&server_key, &intermediate_cert, &intermediate_key)
        .expect("server certificate");
    let cert_der = CertificateDer::from(server_cert);
    let intermediate_der = CertificateDer::from(intermediate_cert);
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(server_key.serialize_der()));

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
        .with_single_cert(vec![cert_der, intermediate_der], key_der)
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
    let server_url =
        std::env::var("E2E_TLS_SERVER").unwrap_or("https://www.snowflake.com".to_string());

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
            std::env::var("E2E_TLS_SERVER").unwrap_or("https://www.snowflake.com".to_string());

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
async fn should_replace_default_roots_with_custom_root_store() {
    let (_, pem_file) = spawn_tls_proxy().await;
    let server_url =
        std::env::var("E2E_TLS_SERVER").unwrap_or("https://www.snowflake.com".to_string());
    let default_client =
        create_tls_client_with_config(TlsConfig::default(), sf_core::crl::CrlWorker::shared_lazy())
            .expect("default client");
    let default_response = default_client.get(&server_url).send().await;
    assert!(
        default_response.is_ok(),
        "default roots should trust the public server: {default_response:?}"
    );

    // When a request uses a custom root store containing only a private CA
    let cfg = TlsConfig {
        custom_root_store_path: Some(pem_file.path().to_path_buf()),
        ..Default::default()
    };
    let client =
        create_tls_client_with_config(cfg, sf_core::crl::CrlWorker::shared_lazy()).expect("client");

    let response = client.get(server_url).send().await;

    // Then the public server is no longer trusted
    assert!(
        response.is_err(),
        "custom root store should replace default roots"
    );
}

#[tokio::test]
async fn should_trust_extra_root_store_when_crl_disabled() {
    // Given a private TLS server signed by a CA that is not in the system store
    let (proxy_addr, pem_file) = spawn_tls_proxy().await;
    let cfg = TlsConfig {
        extra_root_store_path: Some(pem_file.path().to_path_buf()),
        ..Default::default()
    };
    let client =
        create_tls_client_with_config(cfg, sf_core::crl::CrlWorker::shared_lazy()).expect("client");

    // When a request is sent to a server signed by the extra root
    let response = client
        .get(format!("https://localhost:{}", proxy_addr.port()))
        .send()
        .await;

    // Then the extra root is trusted
    assert!(
        response.is_ok(),
        "extra root certificate should be trusted: {response:?}"
    );
}

#[tokio::test]
async fn should_keep_default_roots_when_extra_root_store_is_configured() {
    // Given extra roots that do not include the public server's CA
    let (_, pem_file) = spawn_tls_proxy().await;
    let cfg = TlsConfig {
        extra_root_store_path: Some(pem_file.path().to_path_buf()),
        ..Default::default()
    };
    let client =
        create_tls_client_with_config(cfg, sf_core::crl::CrlWorker::shared_lazy()).expect("client");
    let server_url =
        std::env::var("E2E_TLS_SERVER").unwrap_or("https://www.snowflake.com".to_string());

    // When a request is sent to a server signed by a default root
    let response = client.get(server_url).send().await;

    // Then the default root remains trusted
    assert!(
        response.is_ok(),
        "default roots should remain trusted: {response:?}"
    );
}

#[tokio::test]
async fn should_trust_extra_root_store_when_crl_enabled() {
    // Given a private TLS server signed by a CA that is not in the system store
    let (proxy_addr, pem_file) = spawn_tls_proxy().await;
    let crl_config = sf_core::crl::config::CrlConfig {
        check_mode: CertRevocationCheckMode::Enabled,
        allow_certificates_without_crl_url: true,
        ..Default::default()
    };
    let cfg = TlsConfig {
        crl_config,
        extra_root_store_path: Some(pem_file.path().to_path_buf()),
        ..Default::default()
    };
    let client =
        create_tls_client_with_config(cfg, sf_core::crl::CrlWorker::shared_lazy()).expect("client");

    // When a CRL-enabled request is sent to a server signed by the extra root
    let response = client
        .get(format!("https://localhost:{}", proxy_addr.port()))
        .send()
        .await;

    // Then the extra root is trusted
    assert!(
        response.is_ok(),
        "extra root certificate should be trusted with CRL enabled: {response:?}"
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
