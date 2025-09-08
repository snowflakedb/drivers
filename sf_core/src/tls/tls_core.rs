// Public TLS shim expected by CI
// Re-export the consolidated TLS client API and config
pub use crate::tls::client::create_tls_client_with_config;
pub use crate::tls::config::TlsConfig;
