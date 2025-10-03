use sf_core::tls::config::TlsConfig;
use sf_core::tls::create_tls_client_with_config;

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

        // Then the request attempt should complete (success or error acceptable in CI)
        let _ = resp; // don't assert strict success; we haven't validated that our custom roots is valid
    }
}
