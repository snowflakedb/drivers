use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::tls::CrlServerCertVerifier;
use crate::tls::config::{ProxyConfig, TlsConfig};
use crate::tls::error::{
    ClientBuildSnafu, PemParseSnafu, ProxyBuildSnafu, RootStoreAddSnafu, TlsError,
    VerifierBuildSnafu,
};
use reqwest::{Client, ClientBuilder, NoProxy, Proxy};
use rustls::ClientConfig;
use snafu::ResultExt;
use std::sync::Arc;
use std::time::Duration;

/// Create a reqwest Client with TLS configuration
///
/// This is the main entry point for creating HTTP clients in the application.
/// Handles all TLS configuration including CRL validation, custom root stores, etc.
pub fn create_tls_client_with_config(tls_config: TlsConfig) -> Result<Client, TlsError> {
    create_tls_client_with_proxy(tls_config, None)
}

/// Create a reqwest Client with TLS configuration and an optional explicit proxy.
///
/// When `proxy` is `Some` and has a `host` set, an explicit proxy is applied to
/// every HTTP/HTTPS request and reqwest's default env-var detection is suppressed
/// (matches JDBC/Go/Node/ODBC precedence: connection params > env vars). When
/// `proxy` is `None` or has no host, env vars (`HTTP_PROXY`/`HTTPS_PROXY`/
/// `NO_PROXY`, plus lowercase variants) continue to work via reqwest defaults.
pub fn create_tls_client_with_proxy(
    tls_config: TlsConfig,
    proxy: Option<&ProxyConfig>,
) -> Result<Client, TlsError> {
    // Note: `proxy` is always forwarded to `configure_http_client` even when
    // `proxy.host` is `None`.  That path needs the full `ProxyConfig` to
    // honor `use_proxy_env=false` (default-deny env detection).

    // Handle insecure configurations
    if !tls_config.verify_certificates {
        tracing::warn!("Creating insecure TLS client - certificate verification disabled");
        return configure_http_client(Client::builder(), proxy)?
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true)
            .build()
            .context(ClientBuildSnafu);
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
            let mut builder = configure_http_client(Client::builder(), proxy)?;
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
                proxy,
            )
        }
    }
}

/// Create a reqwest client with custom rustls configuration and optional custom root store
pub(crate) fn create_crl_tls_client_with_root_store(
    crl_config: CrlConfig,
    custom_root_store: Option<rustls::RootCertStore>,
    verify_hostname: bool,
    proxy: Option<&ProxyConfig>,
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
    let client = configure_http_client(Client::builder(), proxy)?
        .use_preconfigured_tls(tls_config)
        .timeout(Duration::from_secs(
            crl_config.http_timeout.num_seconds() as u64
        ))
        .connect_timeout(Duration::from_secs(
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

fn configure_http_client(
    builder: ClientBuilder,
    proxy: Option<&ProxyConfig>,
) -> Result<ClientBuilder, TlsError> {
    let builder = builder
        .pool_idle_timeout(Some(Duration::from_secs(30)))
        .pool_max_idle_per_host(32)
        .tcp_keepalive(Some(Duration::from_secs(60)));

    // No ProxyConfig → preserve historical behaviour (reqwest auto-detects
    // HTTP_PROXY etc.). All connection paths now construct one explicitly,
    // but tests/bins may still pass `None`.
    let Some(proxy) = proxy else {
        return Ok(builder);
    };
    eprintln!(
        "PROXY_DEBUG: host={:?} use_proxy_env={} explicitly_disabled={}",
        proxy.host, proxy.use_proxy_env, proxy.explicitly_disabled,
    );

    if let Some(host) = proxy.host.as_deref().filter(|s| !s.is_empty()) {
        // Explicit proxy → applied for all schemes; reqwest's `.proxy()` call
        // disables auto env detection (matches JDBC/Go/Node precedence).
        let url = build_proxy_url(host, proxy);
        let reqwest_proxy = Proxy::all(&url)
            .context(ProxyBuildSnafu { url: url.clone() })?
            .no_proxy(proxy.no_proxy.as_deref().and_then(NoProxy::from_string));
        return Ok(builder.proxy(reqwest_proxy));
    }

    // No explicit proxy.  Either the customer asked us to honour env vars
    // (legacy POSIX behaviour) or they explicitly disabled the proxy via the
    // legacy ODBC `PROXY=""` + `AllowEmptyProxy=true` form.
    if proxy.use_proxy_env && !proxy.explicitly_disabled {
        Ok(builder)
    } else {
        Ok(builder.no_proxy())
    }
}

/// Build an `http://[user:pass@]host[:port]` URL from a `ProxyConfig`.
/// Credentials are percent-encoded so values containing `:`, `@`, or `/`
/// don't break URL parsing (a known footgun in the legacy Python connector).
fn build_proxy_url(host: &str, proxy: &ProxyConfig) -> String {
    let mut url = String::from("http://");
    if let Some(user) = proxy.user.as_deref().filter(|s| !s.is_empty()) {
        url.push_str(&urlencoding::encode(user));
        if let Some(pw) = proxy
            .password
            .as_ref()
            .map(|p| p.reveal())
            .filter(|s| !s.is_empty())
        {
            url.push(':');
            url.push_str(&urlencoding::encode(pw));
        }
        url.push('@');
    }
    url.push_str(host);
    if let Some(port) = proxy.port.filter(|p| *p > 0) {
        url.push(':');
        url.push_str(&port.to_string());
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensitive::SensitiveString;

    fn proxy(
        host: Option<&str>,
        port: Option<i64>,
        user: Option<&str>,
        password: Option<&str>,
    ) -> ProxyConfig {
        ProxyConfig {
            host: host.map(String::from),
            port,
            user: user.map(String::from),
            password: password.map(|s| SensitiveString::from(s.to_string())),
            ..Default::default()
        }
    }

    #[test]
    fn build_proxy_url_host_only() {
        let p = proxy(Some("p.example.com"), None, None, None);
        assert_eq!(build_proxy_url("p.example.com", &p), "http://p.example.com");
    }

    #[test]
    fn build_proxy_url_host_port() {
        let p = proxy(Some("p.example.com"), Some(8080), None, None);
        assert_eq!(
            build_proxy_url("p.example.com", &p),
            "http://p.example.com:8080"
        );
    }

    #[test]
    fn build_proxy_url_with_creds() {
        let p = proxy(
            Some("p.example.com"),
            Some(8080),
            Some("alice"),
            Some("s3cret"),
        );
        assert_eq!(
            build_proxy_url("p.example.com", &p),
            "http://alice:s3cret@p.example.com:8080"
        );
    }

    #[test]
    fn build_proxy_url_percent_encodes_special_chars_in_creds() {
        // Legacy Python footgun: raw `:` / `@` / `/` in user or password
        // breaks URL parsing. Verify we percent-encode.
        let p = proxy(
            Some("p.example.com"),
            Some(8080),
            Some("user@corp"),
            Some("p:a/ss@1"),
        );
        let url = build_proxy_url("p.example.com", &p);
        assert_eq!(url, "http://user%40corp:p%3Aa%2Fss%401@p.example.com:8080");
        // Sanity-check: reqwest parses the resulting URL successfully.
        Proxy::all(&url).expect("reqwest must accept percent-encoded creds");
    }

    #[test]
    fn build_proxy_url_omits_port_when_zero_or_negative() {
        let p = proxy(Some("p.example.com"), Some(0), None, None);
        assert_eq!(build_proxy_url("p.example.com", &p), "http://p.example.com");
        let p = proxy(Some("p.example.com"), Some(-1), None, None);
        assert_eq!(build_proxy_url("p.example.com", &p), "http://p.example.com");
    }

    #[test]
    fn build_proxy_url_omits_creds_when_user_empty() {
        let p = proxy(Some("p.example.com"), None, Some(""), Some("ignored"));
        assert_eq!(build_proxy_url("p.example.com", &p), "http://p.example.com");
    }

    #[test]
    fn configure_http_client_no_proxy_returns_builder_unchanged() {
        // When proxy is None, no error and reqwest's env-var detection
        // remains in effect (we don't assert env behavior here, only that
        // the call succeeds and returns a usable builder).
        let builder = configure_http_client(Client::builder(), None).unwrap();
        builder.build().expect("client must build");
    }

    #[test]
    fn configure_http_client_empty_host_treated_as_none() {
        let p = proxy(Some(""), Some(8080), None, None);
        let builder = configure_http_client(Client::builder(), Some(&p)).unwrap();
        builder
            .build()
            .expect("empty-host proxy must be ignored, not fail");
    }

    #[test]
    fn configure_http_client_with_explicit_proxy() {
        let p = proxy(Some("p.example.com"), Some(8080), Some("u"), Some("p"));
        let builder = configure_http_client(Client::builder(), Some(&p)).unwrap();
        builder
            .build()
            .expect("client with explicit proxy must build");
    }

    #[test]
    fn configure_http_client_disables_env_when_use_proxy_env_false() {
        // Default ProxyConfig has use_proxy_env=false: the builder should
        // invoke `.no_proxy()` so reqwest does not auto-detect HTTP_PROXY.
        let p = ProxyConfig::default();
        let builder = configure_http_client(Client::builder(), Some(&p)).unwrap();
        builder
            .build()
            .expect("default-deny env path must build cleanly");
    }

    #[test]
    fn configure_http_client_allows_env_when_use_proxy_env_true() {
        let p = ProxyConfig {
            use_proxy_env: true,
            ..Default::default()
        };
        let builder = configure_http_client(Client::builder(), Some(&p)).unwrap();
        builder.build().expect("env-fallback path must build");
    }

    #[test]
    fn configure_http_client_explicitly_disabled_overrides_env() {
        // Empty PROXY with allow_empty_proxy=true: customer says "no proxy"
        // even though use_proxy_env is true.
        let p = ProxyConfig {
            use_proxy_env: true,
            explicitly_disabled: true,
            ..Default::default()
        };
        let builder = configure_http_client(Client::builder(), Some(&p)).unwrap();
        builder
            .build()
            .expect("explicit disable must build cleanly");
    }
}
