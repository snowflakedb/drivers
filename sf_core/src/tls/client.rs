use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::crl::worker::SharedCrlWorker;
use crate::tls::CrlServerCertVerifier;
use crate::tls::config::{ProxyConfig, TlsConfig};
use crate::tls::error::{
    ClientBuildSnafu, PemParseSnafu, ProxyBuildSnafu, RedactedUrl, RootStoreAddSnafu, TlsError,
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
pub fn create_tls_client_with_config(
    tls_config: TlsConfig,
    crl_worker: SharedCrlWorker,
) -> Result<Client, TlsError> {
    create_tls_client_with_proxy(tls_config, None, crl_worker)
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
    crl_worker: SharedCrlWorker,
) -> Result<Client, TlsError> {
    build_tls_client_and_rustls_config(&tls_config, proxy, crl_worker, None).map(|(c, _)| c)
}

/// Build a reqwest [`Client`] and the [`rustls::ClientConfig`] it was derived from.
///
/// The returned [`Arc`] is the exact config the connection uses — hand it to
/// [`DiagnosticRunner`] so the diagnostic observes identical TLS behaviour without
/// re-deriving the config from [`TlsConfig`].
pub(crate) fn build_tls_client_and_rustls_config(
    tls_config: &TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_worker: SharedCrlWorker,
    connect_timeout: Option<Duration>,
) -> Result<(Client, Arc<rustls::ClientConfig>), TlsError> {
    if !tls_config.verify_certificates {
        tracing::warn!("Creating insecure TLS client - certificate verification disabled");
        let builder = apply_reqwest_tls_versions(
            configure_http_client(Client::builder(), proxy)?,
            tls_config,
        );
        let mut builder = builder
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true);
        if let Some(ct) = connect_timeout {
            builder = builder.connect_timeout(ct);
        }
        let client = builder.build().context(ClientBuildSnafu)?;
        return Ok((client, build_insecure_rustls_config()));
    }

    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let protocol_versions = tls_config.versions.enabled_rustls_versions();

    let custom_pem = if let Some(pem_path) = tls_config.custom_root_store_path.as_ref() {
        tracing::debug!(
            "Loading custom root certificate store from: {}",
            pem_path.display()
        );
        Some(std::fs::read(pem_path).context(PemParseSnafu)?)
    } else {
        None
    };

    match tls_config.crl_config.check_mode {
        CertRevocationCheckMode::Disabled => {
            let mut builder = apply_reqwest_tls_versions(
                configure_http_client(Client::builder(), proxy)?,
                tls_config,
            );
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
            if let Some(ct) = connect_timeout {
                builder = builder.connect_timeout(ct);
            }
            let client = builder.build().context(ClientBuildSnafu)?;
            let rustls_cfg = Arc::new(build_plain_rustls_client_config(
                custom_pem.as_deref(),
                &protocol_versions,
            )?);
            Ok((client, rustls_cfg))
        }
        CertRevocationCheckMode::Enabled | CertRevocationCheckMode::Advisory => {
            tracing::debug!(
                "CRL validation enabled, creating client with full TLS handshake validation"
            );
            // Compute the enabled protocol-version window before moving
            // `crl_config` into the builder. reqwest's min/max_tls_version are
            // ignored on the preconfigured-rustls path, so the window must be
            // baked into the rustls ClientConfig instead.
            let custom_root_store = match &custom_pem {
                Some(pem) => Some(create_root_store_from_pem(pem)?),
                None => None,
            };
            let reqwest_rustls_cfg = build_crl_rustls_config(
                tls_config.crl_config.clone(),
                custom_root_store,
                tls_config.verify_hostname,
                &protocol_versions,
                crl_worker,
            )?;
            let mut client_builder = configure_http_client(Client::builder(), proxy)?
                .use_preconfigured_tls(reqwest_rustls_cfg);
            if let Some(ct) = connect_timeout {
                client_builder = client_builder.connect_timeout(ct);
            }
            let client = client_builder.build().context(ClientBuildSnafu)?;
            // For the diagnostic, use a plain config (same root store, no CRL
            // verifier) so inspect_tls can complete the TLS handshake and show
            // the cert chain even when CRL endpoints are unreachable or slow —
            // which is exactly when a user reaches for the diagnostic tool.
            let diag_rustls_cfg = Arc::new(build_plain_rustls_client_config(
                custom_pem.as_deref(),
                &protocol_versions,
            )?);
            Ok((client, diag_rustls_cfg))
        }
    }
}

/// Like [`create_tls_client_with_proxy`], but with an optional TCP connect
/// timeout: when `connect_timeout` is `Some`, it is applied to the underlying
/// HTTP client; when `None`, the system default is used.
pub fn create_tls_client_with_proxy_and_timeouts(
    tls_config: TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_worker: SharedCrlWorker,
    connect_timeout: Option<Duration>,
) -> Result<Client, TlsError> {
    build_tls_client_and_rustls_config(&tls_config, proxy, crl_worker, connect_timeout)
        .map(|(c, _)| c)
}

pub(crate) fn apply_reqwest_tls_versions(
    builder: ClientBuilder,
    tls_config: &TlsConfig,
) -> ClientBuilder {
    apply_reqwest_tls_versions_window(builder, tls_config.versions)
}

pub(crate) fn apply_reqwest_tls_versions_window(
    builder: ClientBuilder,
    versions: crate::tls::config::TlsVersions,
) -> ClientBuilder {
    builder
        .min_tls_version(versions.min.to_reqwest())
        .max_tls_version(versions.max.to_reqwest())
}

/// Apply `tls_config` and an optional explicit `proxy` to a reqwest
/// `ClientBuilder`, returning the configured builder without calling
/// `.build()`. Call sites can chain additional options (e.g. `.no_gzip()`,
/// `.timeout()`) before the final `.build()`.
///
/// `proxy` is threaded through [`apply_proxy_to_builder`] — the same helper the
/// GS/REST client uses — so the storage clients honour `proxy_host`/
/// `proxy_port`/`no_proxy`/`use_proxy_env` identically. Passing `None` leaves
/// reqwest's env-var proxy auto-detection in effect (the historical
/// storage-client behaviour).
///
/// For the default `TlsConfig` and `proxy = None` this is a no-op: the original
/// builder is returned unchanged.
pub(crate) fn configure_tls_builder(
    builder: ClientBuilder,
    tls_config: &TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_worker: SharedCrlWorker,
) -> Result<ClientBuilder, TlsError> {
    let builder = apply_proxy_to_builder(builder, proxy)?;
    if !tls_config.verify_certificates {
        tracing::warn!("Creating insecure TLS client - certificate verification disabled");
        return Ok(apply_reqwest_tls_versions(builder, tls_config)
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true));
    }

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

    match tls_config.crl_config.check_mode {
        CertRevocationCheckMode::Disabled => {
            let mut b = apply_reqwest_tls_versions(builder, tls_config);
            if let Some(ref pem) = custom_pem {
                let certs = reqwest::Certificate::from_pem_bundle(pem).context(ClientBuildSnafu)?;
                b = b.tls_built_in_root_certs(false);
                for cert in certs {
                    b = b.add_root_certificate(cert);
                }
            }
            if !tls_config.verify_hostname {
                b = b.danger_accept_invalid_hostnames(true);
            }
            Ok(b)
        }
        CertRevocationCheckMode::Enabled | CertRevocationCheckMode::Advisory => {
            tracing::debug!("CRL validation enabled, configuring storage TLS client");
            let protocol_versions = tls_config.versions.enabled_rustls_versions();
            let custom_root_store = match custom_pem {
                Some(pem) => Some(create_root_store_from_pem(&pem)?),
                None => None,
            };
            let rustls_cfg = build_crl_rustls_config(
                tls_config.crl_config.clone(),
                custom_root_store,
                tls_config.verify_hostname,
                &protocol_versions,
                crl_worker,
            )?;
            Ok(builder.use_preconfigured_tls(rustls_cfg))
        }
    }
}

/// Build a rustls `ClientConfig` with CRL validation. Shared by
/// [`configure_tls_builder`] (storage clients) and
/// [`build_tls_client_and_rustls_config`] (connection-level client).
fn build_crl_rustls_config(
    crl_config: CrlConfig,
    custom_root_store: Option<rustls::RootCertStore>,
    verify_hostname: bool,
    protocol_versions: &[&'static rustls::SupportedProtocolVersion],
    crl_worker: SharedCrlWorker,
) -> Result<rustls::ClientConfig, TlsError> {
    if !verify_hostname {
        tracing::warn!("Hostname verification disabled (CRL path)");
    }
    let crl_verifier = CrlServerCertVerifier::new_with_root_store(
        crl_config,
        custom_root_store,
        verify_hostname,
        crl_worker,
    )
    .context(VerifierBuildSnafu)?;

    let version_builder = if protocol_versions.is_empty() {
        tracing::debug!("empty TLS protocol-version window; falling back to rustls defaults");
        ClientConfig::builder()
    } else {
        ClientConfig::builder_with_protocol_versions(protocol_versions)
    };
    Ok(version_builder
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(crl_verifier))
        .with_no_client_auth())
}

/// Build a plain rustls [`ClientConfig`] (no CRL verifier) from system roots or
/// a custom PEM bundle.  Used by [`build_tls_client_and_rustls_config`] (CRL-disabled
/// branch).
fn build_plain_rustls_client_config(
    custom_pem: Option<&[u8]>,
    protocol_versions: &[&'static rustls::SupportedProtocolVersion],
) -> Result<rustls::ClientConfig, TlsError> {
    let root_store = match custom_pem {
        Some(pem) => create_root_store_from_pem(pem)?,
        None => {
            let mut native = rustls_native_certs::load_native_certs();
            native.errors.clear();
            let mut store = rustls::RootCertStore::empty();
            store.add_parsable_certificates(native.certs);
            store
        }
    };
    let builder = if protocol_versions.is_empty() {
        rustls::ClientConfig::builder()
    } else {
        rustls::ClientConfig::builder_with_protocol_versions(protocol_versions)
    };
    Ok(builder
        .with_root_certificates(root_store)
        .with_no_client_auth())
}

/// rustls [`ClientConfig`] that skips all certificate verification.
///
/// Returned by [`build_tls_client_and_rustls_config`] for the `verify_certificates=false`
/// path so the diagnostic's TLS probe behaves consistently with the reqwest client: neither
/// verifies the server certificate.  Using a cert-verified config here would cause
/// false-negative TLS failures in environments with custom or self-signed CAs, which is
/// exactly the case where users reach for `verify_certificates=false`.
pub(crate) fn build_insecure_rustls_config() -> Arc<rustls::ClientConfig> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    Arc::new(
        rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifyCertVerifier::new()))
            .with_no_client_auth(),
    )
}

/// A [`ServerCertVerifier`] that accepts any certificate chain without validation.
///
/// Mirrors `reqwest`'s `danger_accept_invalid_certs(true)` at the rustls layer: the
/// certificate chain is not checked against any trust anchor, but the TLS handshake
/// signatures are still verified against the crypto provider's algorithms so the peer
/// genuinely holds the private key for the leaf it presented.
///
/// Signature verification goes through the provider's [`WebPkiSupportedAlgorithms`]
/// directly rather than a [`WebPkiServerVerifier`]: the latter's builder returns
/// `NoRootAnchors` when given an empty root store, which would make a no-verify config
/// impossible to construct.
#[derive(Debug)]
struct NoVerifyCertVerifier {
    supported_algs: rustls::crypto::WebPkiSupportedAlgorithms,
}

impl NoVerifyCertVerifier {
    fn new() -> Self {
        Self {
            supported_algs: rustls::crypto::aws_lc_rs::default_provider()
                .signature_verification_algorithms,
        }
    }
}

impl rustls::client::danger::ServerCertVerifier for NoVerifyCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.supported_algs)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.supported_algs)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.supported_algs.supported_schemes()
    }
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

    apply_proxy_to_builder(builder, proxy)
}

/// Apply the driver's [`ProxyConfig`] to a reqwest [`ClientBuilder`].
///
/// Single source of truth for this translation — shared by the GS/REST
/// client ([`configure_http_client`]) and the S3/GCS/Azure storage clients
/// ([`configure_tls_builder`]).
///
/// Semantics:
/// - `None` → the builder is returned unchanged, preserving reqwest's default
///   `HTTP_PROXY`/`HTTPS_PROXY`/`NO_PROXY` env-var auto-detection. Tests/bins
///   may still pass `None`.
/// - explicit `host` → an all-schemes proxy is applied; reqwest's `.proxy()`
///   call disables env auto-detection (matches JDBC/Go/Node precedence:
///   connection params > env vars).
/// - no `host`, `use_proxy_env = true`, not explicitly disabled → the builder
///   is returned unchanged so env vars keep working (legacy POSIX behaviour).
/// - otherwise → `.no_proxy()` suppresses reqwest's env auto-detection.
pub(crate) fn apply_proxy_to_builder(
    builder: ClientBuilder,
    proxy: Option<&ProxyConfig>,
) -> Result<ClientBuilder, TlsError> {
    let Some(proxy) = proxy else {
        return Ok(builder);
    };
    tracing::debug!(
        host = ?proxy.host,
        use_proxy_env = proxy.use_proxy_env,
        explicitly_disabled = proxy.explicitly_disabled,
        "proxy config"
    );

    if let Some(host) = proxy.host.as_deref().filter(|s| !s.is_empty()) {
        // Explicit proxy → applied for all schemes; reqwest's `.proxy()` call
        // disables auto env detection (matches JDBC/Go/Node precedence).
        let url = build_proxy_url(host, proxy);
        // `url` is the fully credentialed `http://user:pass@host` form;
        // `RedactedUrl::new` strips credentials before the value can reach
        // `ProxyBuild`'s `Debug`/`ErrorTrace` output.
        let reqwest_proxy = Proxy::all(&url)
            .context(ProxyBuildSnafu {
                redacted_url: RedactedUrl::new(&url),
            })?
            .no_proxy(proxy.no_proxy.as_deref().and_then(NoProxy::from_string));
        return Ok(builder.proxy(reqwest_proxy));
    }

    // No explicit proxy.  Either the customer asked us to honour env vars
    // (legacy POSIX behaviour) or they explicitly disabled the proxy via the
    // legacy ODBC `PROXY=""` + `AllowEmptyProxy=true` form.
    if proxy.use_proxy_env && !proxy.explicitly_disabled {
        tracing::warn!("Proxy configuration: using environment variable proxy settings");
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
    #[tracing_test::traced_test]
    fn should_log_info_when_use_proxy_env_is_enabled() {
        let p = ProxyConfig {
            use_proxy_env: true,
            ..Default::default()
        };
        configure_http_client(Client::builder(), Some(&p)).unwrap();
        assert!(logs_contain("environment variable proxy settings"));
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

    #[test]
    fn proxy_build_error_never_exposes_credentials_in_debug() {
        // A proxy host containing a URL-forbidden character makes reqwest's
        // `Proxy::all` reject the built `http://user:pass@host` URL, driving the
        // `ProxyBuild` error path with credentials present. The password must
        // not survive into `Debug`/`ErrorTrace`.
        const PASSWORD: &str = "sup3r-s3cret-pw";
        let p = proxy(Some("bad<host"), Some(8080), Some("alice"), Some(PASSWORD));

        let err = configure_http_client(Client::builder(), Some(&p))
            .expect_err("malformed proxy host must fail proxy construction");
        assert!(
            matches!(err, TlsError::ProxyBuild { .. }),
            "expected ProxyBuild, got: {err:?}"
        );

        let debug = format!("{err:?}");
        let display = err.to_string();
        let trace = error_trace::format_error_trace(&error_trace::ErrorTrace::error_trace(&err));

        assert!(
            !debug.contains(PASSWORD),
            "Debug output leaked the proxy password: {debug}"
        );
        assert!(
            !display.contains(PASSWORD),
            "Display output leaked the proxy password: {display}"
        );
        assert!(
            !trace.contains(PASSWORD),
            "ErrorTrace output leaked the proxy password: {trace}"
        );
    }
}
