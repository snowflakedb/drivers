use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::tls::CrlServerCertVerifier;
use crate::tls::config::{ProxyConfig, TlsConfig};
use crate::tls::error::{
    ClientBuildSnafu, PemParseSnafu, RootStoreAddSnafu, TlsError, VerifierBuildSnafu,
};
use reqwest::{Client, ClientBuilder, Proxy};
use rustls::ClientConfig;
use snafu::ResultExt;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

/// Create a reqwest Client with TLS configuration
///
/// This is the main entry point for creating HTTP clients in the application.
/// Handles all TLS configuration including CRL validation, custom root stores, etc.
pub fn create_tls_client_with_config(
    tls_config: TlsConfig,
    proxy_config: &ProxyConfig,
) -> Result<Client, TlsError> {
    // Handle insecure configurations
    if !tls_config.verify_certificates {
        tracing::warn!("Creating insecure TLS client - certificate verification disabled");
        let builder = configure_http_client(Client::builder())
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true);
        let builder = apply_proxy(builder, proxy_config)?;
        return builder.build().context(ClientBuildSnafu);
    }

    // Install aws-lc-rs provider (idempotent)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let custom_pem = if let Some(pem_path) = tls_config.custom_root_store_path.as_ref() {
        tracing::debug!(
            "Loading custom root certificate store from: {}",
            pem_path.display()
        );
        Some(std::fs::read(pem_path).context(PemParseSnafu)?)
    } else {
        None
    };

    // Create client based on CRL configuration
    match tls_config.crl_config.check_mode {
        CertRevocationCheckMode::Disabled => {
            let mut builder = configure_http_client(Client::builder());
            if let Some(ref pem) = custom_pem {
                tracing::debug!("CRL disabled, applying custom root store");
                let certs = reqwest::Certificate::from_pem_bundle(pem).context(ClientBuildSnafu)?;
                builder = builder.tls_built_in_root_certs(false);
                for cert in certs {
                    builder = builder.add_root_certificate(cert);
                }
            } else {
                tracing::debug!("CRL disabled, using default system roots");
            }
            if !tls_config.verify_hostname {
                tracing::warn!("Hostname verification disabled");
                builder = builder.danger_accept_invalid_hostnames(true);
            }
            let builder = apply_proxy(builder, proxy_config)?;
            builder.build().context(ClientBuildSnafu)
        }
        CertRevocationCheckMode::Enabled | CertRevocationCheckMode::Advisory => {
            tracing::debug!(
                "CRL validation enabled, creating client with full TLS handshake validation"
            );
            let custom_root_store = match custom_pem {
                Some(pem) => Some(create_root_store_from_pem(&pem)?),
                None => None,
            };
            create_crl_tls_client_with_root_store(
                tls_config.crl_config,
                custom_root_store,
                tls_config.verify_hostname,
                proxy_config,
            )
        }
    }
}

/// Create a reqwest client with custom rustls configuration and optional custom root store
pub(crate) fn create_crl_tls_client_with_root_store(
    crl_config: CrlConfig,
    custom_root_store: Option<rustls::RootCertStore>,
    verify_hostname: bool,
    proxy_config: &ProxyConfig,
) -> Result<Client, TlsError> {
    tracing::debug!("Creating custom TLS client with CRL handshake validation");
    if !verify_hostname {
        tracing::warn!("Hostname verification disabled (CRL path)");
    }

    // Install default crypto provider for rustls (aws-lc-rs)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // Create custom certificate verifier with CRL validation
    let crl_verifier = CrlServerCertVerifier::new_with_root_store(
        crl_config.clone(),
        custom_root_store,
        verify_hostname,
    )
    .context(VerifierBuildSnafu)?;

    // Create rustls client configuration with our custom verifier
    let tls_config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(crl_verifier))
        .with_no_client_auth();

    // Create reqwest client with custom TLS configuration
    let builder = configure_http_client(Client::builder())
        .use_preconfigured_tls(tls_config)
        .timeout(Duration::from_secs(
            crl_config.http_timeout.num_seconds() as u64
        ))
        .connect_timeout(Duration::from_secs(
            crl_config.connection_timeout.num_seconds() as u64,
        ));
    let builder = apply_proxy(builder, proxy_config)?;
    let client = builder.build().context(ClientBuildSnafu)?;

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

fn configure_http_client(builder: ClientBuilder) -> ClientBuilder {
    builder
        .pool_idle_timeout(Some(Duration::from_secs(30)))
        .pool_max_idle_per_host(32)
        .tcp_keepalive(Some(Duration::from_secs(60)))
}

/// Normalize a proxy URL: ensure it has a scheme prefix (default `http://`).
fn normalize_proxy_url(raw: &str) -> Result<String, TlsError> {
    let with_scheme = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("http://{raw}")
    };
    // Validate the URL is parseable.
    Url::parse(&with_scheme).map_err(|e| TlsError::ProxyConfig {
        reason: format!("cannot parse proxy URL '{raw}': {e}"),
        location: snafu::Location::new(file!(), line!(), 0),
    })?;
    Ok(with_scheme)
}

/// Apply proxy configuration to a `ClientBuilder`.
fn apply_proxy(
    mut builder: ClientBuilder,
    config: &ProxyConfig,
) -> Result<ClientBuilder, TlsError> {
    match &config.proxy_url {
        Some(raw_url) if !raw_url.is_empty() => {
            let url = normalize_proxy_url(raw_url)?;
            tracing::debug!("Configuring explicit proxy: {}", url);
            let mut proxy = Proxy::all(&url).map_err(|e| TlsError::ProxyConfig {
                reason: format!("invalid proxy URL: {e}"),
                location: snafu::Location::new(file!(), line!(), 0),
            })?;
            if let Some(ref no_proxy) = config.no_proxy {
                tracing::debug!("Configuring no_proxy: {}", no_proxy);
                proxy = proxy.no_proxy(reqwest::NoProxy::from_string(no_proxy));
            }
            builder = builder.proxy(proxy);
        }
        Some(_) => {
            // Empty proxy string with allow_empty_proxy=true: explicitly disable proxy.
            tracing::debug!("Empty proxy value — disabling proxy (overrides env)");
            builder = builder.no_proxy();
        }
        None => {
            if config.use_proxy_env {
                tracing::debug!("Using proxy from environment variables");
            } else {
                builder = builder.no_proxy();
            }
        }
    }
    Ok(builder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_proxy_url_adds_scheme() {
        assert_eq!(
            normalize_proxy_url("myproxy:8080").unwrap(),
            "http://myproxy:8080"
        );
    }

    #[test]
    fn normalize_proxy_url_preserves_existing_scheme() {
        assert_eq!(
            normalize_proxy_url("https://proxy.corp:443").unwrap(),
            "https://proxy.corp:443"
        );
    }

    #[test]
    fn normalize_proxy_url_with_credentials() {
        let result = normalize_proxy_url("user:pass@proxy.corp:8080").unwrap();
        assert_eq!(result, "http://user:pass@proxy.corp:8080");
    }

    #[test]
    fn apply_proxy_no_proxy_disables_env() {
        let config = ProxyConfig::default();
        let builder = Client::builder();
        let builder = apply_proxy(builder, &config).unwrap();
        // Should succeed without error (no_proxy mode).
        let _ = builder;
    }

    #[test]
    fn apply_proxy_with_explicit_url() {
        let config = ProxyConfig {
            proxy_url: Some("http://proxy.test:3128".to_string()),
            no_proxy: Some("localhost,.internal".to_string()),
            ..Default::default()
        };
        let builder = Client::builder();
        let result = apply_proxy(builder, &config);
        assert!(result.is_ok());
    }
}
