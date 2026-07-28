use std::collections::HashMap;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;

use rcgen::generate_simple_self_signed;
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use sf_core::config::settings::Setting;
use sf_core::tls::config::TlsConfig;
use sf_core::tls::create_tls_client_with_config;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Spawns a TLS-terminating proxy (backed by a wiremock returning 200) whose
/// `rustls` server config is restricted to exactly `versions`. Returns the
/// listen address and a `NamedTempFile` holding the self-signed root PEM for
/// "localhost"; keep the file alive for the PEM path to remain valid.
async fn spawn_tls_server_with_versions(
    versions: &[&'static rustls::SupportedProtocolVersion],
) -> (SocketAddr, tempfile::NamedTempFile) {
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

    // Restrict the server to the requested protocol version(s) so the client's
    // [min, max] window is what decides whether a common version exists.
    let server_config = ServerConfig::builder_with_protocol_versions(versions)
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)
        .expect("server config");
    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().unwrap();
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

    (addr, pem_file)
}

#[tokio::test]
async fn should_negotiate_tls_when_the_server_offers_a_version_inside_the_window() {
    // Given a TLS server that offers only TLS 1.3
    let (addr, pem_file) = spawn_tls_server_with_versions(&[&rustls::version::TLS13]).await;
    let port = addr.port();

    // And a client configured with min_tls_version tls12 and max_tls_version tls13
    let cfg = TlsConfig {
        custom_root_store_path: Some(pem_file.path().to_path_buf()),
        ..Default::default()
    };
    let client =
        create_tls_client_with_config(cfg, sf_core::crl::CrlWorker::shared_lazy()).expect("client");

    // When a request is sent to the server
    let resp = client.get(format!("https://localhost:{port}")).send().await;

    // Then the handshake succeeds
    assert!(
        resp.is_ok(),
        "TLS 1.3 is inside the default tls12..=tls13 window, handshake should succeed: {resp:?}"
    );
}

#[tokio::test]
async fn should_fail_the_handshake_when_the_server_only_offers_a_version_below_the_minimum() {
    // Given a TLS server that offers only TLS 1.2
    let (addr, pem_file) = spawn_tls_server_with_versions(&[&rustls::version::TLS12]).await;
    let port = addr.port();

    // And a client configured with min_tls_version tls13
    let settings: HashMap<String, Setting> = [
        (
            "custom_root_store_path".to_string(),
            Setting::String(pem_file.path().to_string_lossy().into_owned()),
        ),
        (
            "min_tls_version".to_string(),
            Setting::String("tls13".to_string()),
        ),
    ]
    .into_iter()
    .collect();
    let cfg = TlsConfig::from_settings(&settings).expect("tls13-only window is valid");
    let client =
        create_tls_client_with_config(cfg, sf_core::crl::CrlWorker::shared_lazy()).expect("client");

    // When a request is sent to the server
    let resp = client.get(format!("https://localhost:{port}")).send().await;

    // Then the handshake fails
    assert!(
        resp.is_err(),
        "server offers only TLS 1.2 but client floor is TLS 1.3; handshake must fail"
    );

    // Positive control: the *same* server and root store, but with the default
    // window (which includes TLS 1.2), must succeed — proving the failure above
    // is specifically the version floor, not a cert/connectivity problem.
    let permissive = TlsConfig {
        custom_root_store_path: Some(pem_file.path().to_path_buf()),
        ..Default::default()
    };
    let permissive_client =
        create_tls_client_with_config(permissive, sf_core::crl::CrlWorker::shared_lazy())
            .expect("client");
    let ok = permissive_client
        .get(format!("https://localhost:{port}"))
        .send()
        .await;
    assert!(
        ok.is_ok(),
        "same TLS 1.2 server must succeed once TLS 1.2 is inside the window: {ok:?}"
    );
}

#[test]
fn should_reject_the_configuration_when_the_minimum_exceeds_the_maximum() {
    // Given settings with min_tls_version tls13 and max_tls_version tls12
    let settings: HashMap<String, Setting> = [
        (
            "min_tls_version".to_string(),
            Setting::String("tls13".to_string()),
        ),
        (
            "max_tls_version".to_string(),
            Setting::String("tls12".to_string()),
        ),
    ]
    .into_iter()
    .collect();

    // When the TLS configuration is built from settings
    let result = TlsConfig::from_settings(&settings);

    // Then a configuration error is returned
    let err = result.expect_err("min > max must be rejected");
    assert!(
        matches!(
            &err,
            sf_core::config::ConfigError::InvalidParameterValue { parameter, .. }
                if parameter == "max_tls_version"
        ),
        "expected InvalidParameterValue for max_tls_version, got: {err:?}"
    );
}
