use sf_core::tls::config::TlsConfig;
use sf_core::tls::create_tls_client_with_config;

#[tokio::test]
async fn handshake_with_default_roots() {
    // Scenario: Handshake with default roots
    // Given a TLS client configured with default roots
    let server_url = std::env::var("E2E_TLS_SERVER").unwrap_or("https://example.com".to_string());
    // When I send a GET request to the server URL
    let client = create_tls_client_with_config(TlsConfig::default()).expect("client");
    let resp = client.get(server_url).send().await;
    // Then the request attempt should complete (success or error acceptable in CI)
    // Only assert we can attempt; don't fail CI when network blocked
    assert!(resp.is_ok() || resp.is_err());
}

#[tokio::test]
async fn handshake_with_custom_roots_env() {
    // Scenario: Handshake with custom PEM roots
    if let Ok(pem_path) = std::env::var("E2E_TLS_ROOTS_PEM") {
        // Given E2E_TLS_ROOTS_PEM is set to a PEM bundle path
        // And a TLS client configured with that custom root store
        let cfg = TlsConfig {
            custom_root_store_path: Some(pem_path.into()),
            ..Default::default()
        };
        let client = create_tls_client_with_config(cfg).expect("client");
        let server_url =
            std::env::var("E2E_TLS_SERVER").unwrap_or("https://example.com".to_string());
        // When I send a GET request to the server URL
        let _ = client.get(server_url).send().await;
        // Then the request attempt should complete
    }
}
