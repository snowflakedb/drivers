use crate::tls::CrlServerCertVerifier;
use crate::tls::config::TlsConfig;
use crate::tls::error::{
    ClientBuildSnafu, PemParseSnafu, RootStoreAddSnafu, TlsError, VerifierBuildSnafu,
};
use reqwest::Client;
use snafu::ResultExt;
use std::sync::Arc;

pub fn create_tls_client_with_config(cfg: TlsConfig) -> Result<Client, TlsError> {
    if !cfg.verify_certificates {
        return Client::builder()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(!cfg.verify_hostname)
            .build()
            .context(ClientBuildSnafu);
    }

    // Install aws-lc-rs provider (idempotent)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let custom_root_store = if let Some(pem_path) = cfg.custom_root_store_path.as_ref() {
        let pem_data = std::fs::read(pem_path).context(PemParseSnafu)?;
        Some(create_root_store_from_pem(&pem_data)?)
    } else {
        None
    };

    create_crl_tls_client_with_root_store(cfg, custom_root_store)
}

/// Create a reqwest client with custom rustls configuration and optional custom root store
pub fn create_crl_tls_client_with_root_store(
    cfg: TlsConfig,
    custom_root_store: Option<rustls::RootCertStore>,
) -> Result<Client, TlsError> {
    use rustls::ClientConfig;
    // Create custom certificate verifier with CRL validation
    let crl_verifier =
        CrlServerCertVerifier::new_with_root_store(cfg.crl_config.clone(), custom_root_store)
            .context(VerifierBuildSnafu)?;

    // Create rustls client configuration with our custom verifier
    let tls_config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(crl_verifier))
        .with_no_client_auth();

    // Create reqwest client with custom TLS configuration and CRL timeouts
    Client::builder()
        .use_preconfigured_tls(tls_config)
        .timeout(std::time::Duration::from_secs(
            cfg.crl_config.http_timeout.num_seconds() as u64,
        ))
        .connect_timeout(std::time::Duration::from_secs(
            cfg.crl_config.connection_timeout.num_seconds() as u64,
        ))
        .danger_accept_invalid_hostnames(!cfg.verify_hostname)
        .build()
        .context(ClientBuildSnafu)
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
