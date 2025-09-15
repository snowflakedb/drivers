use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::tls::CrlServerCertVerifier;
use crate::tls::config::TlsConfig;
use crate::tls::error::{
    ClientBuildSnafu, PemParseSnafu, RootStoreAddSnafu, TlsError, VerifierBuildSnafu,
};
use reqwest::Client;
use rustls::ClientConfig;
use snafu::ResultExt;
use std::sync::Arc;

/// Create a reqwest Client with TLS configuration
///
/// This is the main entry point for creating HTTP clients in the application.
/// Handles all TLS configuration including CRL validation, custom root stores, etc.
pub fn create_tls_client_with_config(tls_config: TlsConfig) -> Result<Client, TlsError> {
    // Handle insecure configurations
    if !tls_config.verify_certificates {
        tracing::warn!("Creating insecure TLS client - certificate verification disabled");
        return Client::builder()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true)
            .build()
            .context(ClientBuildSnafu);
    }

    // Install aws-lc-rs provider (idempotent)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let custom_root_store = if let Some(pem_path) = tls_config.custom_root_store_path.as_ref() {
        tracing::debug!(
            "Loading custom root certificate store from: {}",
            pem_path.display()
        );
        let pem_data = std::fs::read(pem_path).context(PemParseSnafu)?;
        Some(create_root_store_from_pem(&pem_data)?)
    } else {
        None
    };

    // Create client based on CRL configuration
    match tls_config.crl_config.check_mode {
        CertRevocationCheckMode::Disabled => {
            tracing::debug!("CRL validation disabled, creating standard client");
            if custom_root_store.is_some() {
                tracing::warn!(
                    "Custom root store specified but CRL validation disabled - custom roots will be ignored"
                );
            }
            Client::builder().build().context(ClientBuildSnafu)
        }
        CertRevocationCheckMode::Enabled | CertRevocationCheckMode::Advisory => {
            tracing::debug!(
                "CRL validation enabled, creating client with full TLS handshake validation"
            );
            create_crl_tls_client_with_root_store(tls_config.crl_config, custom_root_store)
        }
    }
}

/// Create a reqwest client with custom rustls configuration and optional custom root store
pub fn create_crl_tls_client_with_root_store(
    crl_config: CrlConfig,
    custom_root_store: Option<rustls::RootCertStore>,
) -> Result<Client, TlsError> {
    tracing::debug!("Creating custom TLS client with CRL handshake validation");

    // Install default crypto provider for rustls (aws-lc-rs)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // Create custom certificate verifier with CRL validation
    let crl_verifier =
        CrlServerCertVerifier::new_with_root_store(crl_config.clone(), custom_root_store)
            .context(VerifierBuildSnafu)?;

    // Create rustls client configuration with our custom verifier
    let tls_config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(crl_verifier))
        .with_no_client_auth();

    // Create reqwest client with custom TLS configuration
    let client = Client::builder()
        .use_preconfigured_tls(tls_config)
        .timeout(std::time::Duration::from_secs(
            crl_config.http_timeout.num_seconds() as u64,
        ))
        .connect_timeout(std::time::Duration::from_secs(
            crl_config.connection_timeout.num_seconds() as u64,
        ))
        .build()
        .context(ClientBuildSnafu)?;

    tracing::debug!("Created TLS client with full handshake CRL validation");
    Ok(client)
}

/// Convert PEM certificate data to rustls RootCertStore
pub fn create_root_store_from_pem(pem_data: &[u8]) -> Result<rustls::RootCertStore, TlsError> {
    use std::io::Cursor;
    let mut root_store = rustls::RootCertStore::empty();
    let mut cursor = Cursor::new(pem_data);
    let certs = rustls_pemfile::certs(&mut cursor)
        .collect::<Result<Vec<_>, _>>()
        .context(PemParseSnafu)?;
    if certs.is_empty() {
        return Err(TlsError::PemParse {
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, "no certs in PEM"),
            location: snafu::Location::new(file!(), line!(), 0),
        });
    }
    for cert in certs {
        root_store.add(cert).context(RootStoreAddSnafu)?;
    }
    Ok(root_store)
}
