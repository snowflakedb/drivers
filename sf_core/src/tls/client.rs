use crate::tls::config::TlsConfig;
use reqwest::Client;

#[derive(thiserror::Error, Debug)]
pub enum TlsError {
    #[error("Failed to build HTTP client: {0}")]
    ClientBuild(reqwest::Error),
    #[error("Failed to read PEM file: {0}")]
    PemRead(std::io::Error),
    #[error("Invalid PEM: {0}")]
    PemParse(std::io::Error),
    #[error("Failed to add cert to root store: {0}")]
    RootStoreAdd(rustls::Error),
}

pub fn create_tls_client_with_config(cfg: TlsConfig) -> Result<Client, TlsError> {
    if !cfg.verify_certificates {
        return Client::builder()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(!cfg.verify_hostname)
            .build()
            .map_err(TlsError::ClientBuild);
    }

    // Install aws-lc-rs provider (idempotent)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let mut root_store = rustls::RootCertStore::empty();
    if let Some(pem_path) = cfg.custom_root_store_path.as_ref() {
        let pem_data = std::fs::read(pem_path).map_err(TlsError::PemRead)?;
        let mut cursor = std::io::Cursor::new(pem_data);
        for cert in rustls_pemfile::certs(&mut cursor) {
            let cert = cert.map_err(TlsError::PemParse)?;
            root_store.add(cert).map_err(TlsError::RootStoreAdd)?;
        }
    } else {
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    }

    let tls = rustls::ClientConfig::builder()
        .with_root_certificates(std::sync::Arc::new(root_store))
        .with_no_client_auth();

    Client::builder()
        .use_preconfigured_tls(tls)
        .build()
        .map_err(TlsError::ClientBuild)
}
