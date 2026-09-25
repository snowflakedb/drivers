use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::crl::worker::SharedCrlWorker;
use crate::tls::CrlServerCertVerifier;
use crate::tls::config::{ProxyConfig, TlsConfig};
use crate::tls::error::{
    ClientBuildSnafu, PemParseSnafu, ProxyBuildSnafu, RedactedUrl, RootStoreAddSnafu,
    RustlsConfigSnafu, TlsError, VerifierBuildSnafu,
};
use reqwest::{Client, ClientBuilder, NoProxy, Proxy};
use rustls::ClientConfig;
use snafu::ResultExt;
use std::sync::Arc;
use std::time::Duration;

enum RootCertificates {
    Default,
    Custom(Vec<u8>),
    Extra(Vec<u8>),
}

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
    build_tls_client_and_rustls_config(&tls_config, proxy, crl_worker, None, false).map(|(c, _)| c)
}

/// Build a reqwest [`Client`] and, only when `need_diag_config` is set, the
/// [`rustls::ClientConfig`] it was derived from.
///
/// The returned [`Arc`] is the exact config the connection uses — hand it to
/// [`DiagnosticRunner`] so the diagnostic observes identical TLS behaviour without
/// re-deriving the config from [`TlsConfig`]. It is `None` when `need_diag_config`
/// is `false`, so callers with no diagnostic to run avoid a second trust-store load.
pub(crate) fn build_tls_client_and_rustls_config(
    tls_config: &TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_worker: SharedCrlWorker,
    connect_timeout: Option<Duration>,
    need_diag_config: bool,
) -> Result<(Client, Option<Arc<rustls::ClientConfig>>), TlsError> {
    // Must precede every `Client::build()` below, including the insecure
    // early-return: reqwest resolves its crypto backend at build time, and
    // with the `-no-provider` feature selection it has no fallback to resolve
    // to, so a client built before the provider is installed panics with
    // "No provider set".
    super::ensure_crypto_provider();
    // Fail closed rather than serve traffic on a non-approved module: in
    // `fips-tls` builds this refuses to build a client unless both the linked
    // module and the process-global provider are FIPS. The global matters
    // because only the CRL branch below hands rustls our config -- the
    // insecure and CRL-disabled branches let reqwest resolve the global for
    // the handshake. Compiles away without the feature.
    super::require_fips_provider()?;

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
        return Ok((
            client,
            need_diag_config
                .then(build_insecure_rustls_config)
                .transpose()?,
        ));
    }

    let protocol_versions = tls_config.versions.enabled_rustls_versions();

    let root_certificates = load_root_certificates(tls_config)?;

    match tls_config.crl_config.check_mode {
        CertRevocationCheckMode::Disabled => {
            let mut builder = apply_reqwest_tls_versions(
                configure_http_client(Client::builder(), proxy)?,
                tls_config,
            );
            if matches!(root_certificates, RootCertificates::Default) {
                tracing::debug!("CRL disabled, using default system roots");
            }
            builder = apply_reqwest_root_certificates(builder, &root_certificates)?;
            if !tls_config.verify_hostname {
                tracing::warn!("Hostname verification disabled");
                builder = builder.danger_accept_invalid_hostnames(true);
            }
            if let Some(ct) = connect_timeout {
                builder = builder.connect_timeout(ct);
            }
            let client = builder.build().context(ClientBuildSnafu)?;
            let rustls_cfg = need_diag_config
                .then(|| {
                    build_plain_rustls_client_config(&root_certificates, &protocol_versions)
                        .map(Arc::new)
                })
                .transpose()?;
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
            let root_store_override = root_store_for_crl(&root_certificates)?;
            let reqwest_rustls_cfg = build_crl_rustls_config(
                tls_config.crl_config.clone(),
                root_store_override,
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
            let diag_rustls_cfg = need_diag_config
                .then(|| {
                    build_plain_rustls_client_config(&root_certificates, &protocol_versions)
                        .map(Arc::new)
                })
                .transpose()?;
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
    build_tls_client_and_rustls_config(&tls_config, proxy, crl_worker, connect_timeout, false)
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
    // Same ordering constraint as `build_tls_client_and_rustls_config`: the
    // returned builder is `.build()`-ed by the caller, so the provider has to
    // be in place before this function hands the builder back.
    super::ensure_crypto_provider();
    super::require_fips_provider()?;
    let builder = apply_proxy_to_builder(builder, proxy)?;
    if !tls_config.verify_certificates {
        tracing::warn!("Creating insecure TLS client - certificate verification disabled");
        return Ok(apply_reqwest_tls_versions(builder, tls_config)
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true));
    }

    let root_certificates = load_root_certificates(tls_config)?;

    match tls_config.crl_config.check_mode {
        CertRevocationCheckMode::Disabled => {
            let mut b = apply_reqwest_tls_versions(builder, tls_config);
            b = apply_reqwest_root_certificates(b, &root_certificates)?;
            if !tls_config.verify_hostname {
                b = b.danger_accept_invalid_hostnames(true);
            }
            Ok(b)
        }
        CertRevocationCheckMode::Enabled | CertRevocationCheckMode::Advisory => {
            tracing::debug!("CRL validation enabled, configuring storage TLS client");
            let protocol_versions = tls_config.versions.enabled_rustls_versions();
            let root_store_override = root_store_for_crl(&root_certificates)?;
            let rustls_cfg = build_crl_rustls_config(
                tls_config.crl_config.clone(),
                root_store_override,
                tls_config.verify_hostname,
                &protocol_versions,
                crl_worker,
            )?;
            Ok(builder.use_preconfigured_tls(rustls_cfg))
        }
    }
}

/// [`configure_tls_builder`] plus `.no_gzip()`, for the Azure and GCS
/// transfers that move opaque, possibly CSE-encrypted bytes whose
/// downstream SHA-256 digest / Content-Length / ranged-download checks
/// assume wire bytes == body bytes. Without it, a response carrying
/// `Content-Encoding: gzip` (e.g. from `gsutil cp -Z`, BigQuery exports, or
/// other external loaders) is transparently gunzipped by reqwest, silently
/// substituting the decoded body for the actual on-cloud bytes. Mirrors
/// JDBC's `HttpUtil.disableContentCompression()`
/// (`SnowflakeGCSClient.java:237,:432` via `HttpUtil.java:420`) and the
/// intent of Python's `remove_content_encoding` urllib3 hook
/// (`storage_client.py:54-59`).
///
/// The GS/REST client still wants gzip, so this can't be folded into
/// `configure_tls_builder` itself. S3 does not come through here: it reaches
/// the AWS SDK through [`AwsSdkReqwestClient`](crate::tls::aws_http_client::AwsSdkReqwestClient),
/// which calls `configure_tls_builder` directly and then applies `.no_gzip()`
/// alongside the two SDK-only adjustments (`.redirect(Policy::none())`,
/// `.http1_only()`) that cannot be set on an already-built client.
pub(crate) fn configure_storage_client_builder(
    builder: ClientBuilder,
    tls_config: &TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_worker: SharedCrlWorker,
) -> Result<ClientBuilder, TlsError> {
    configure_tls_builder(builder, tls_config, proxy, crl_worker).map(ClientBuilder::no_gzip)
}

/// Build a rustls `ClientConfig` with CRL validation. Shared by
/// [`configure_tls_builder`] (storage clients) and
/// [`build_tls_client_and_rustls_config`] (connection-level client).
fn build_crl_rustls_config(
    crl_config: CrlConfig,
    root_store_override: Option<rustls::RootCertStore>,
    verify_hostname: bool,
    protocol_versions: &[&'static rustls::SupportedProtocolVersion],
    crl_worker: SharedCrlWorker,
) -> Result<rustls::ClientConfig, TlsError> {
    if !verify_hostname {
        tracing::warn!("Hostname verification disabled (CRL path)");
    }
    let crl_verifier = CrlServerCertVerifier::new_with_root_store(
        crl_config,
        root_store_override,
        verify_hostname,
        crl_worker,
    )
    .context(VerifierBuildSnafu)?;

    // Explicit provider rather than `ClientConfig::builder()`: the latter reads
    // the process-global default, so the module verifying this connection's
    // chain would be whichever one won a startup race. Under `fips-tls` that
    // race is the compliance claim.
    let provider = crate::tls::crypto_module::CryptoModule::get().provider();
    let version_builder = if protocol_versions.is_empty() {
        tracing::debug!("empty TLS protocol-version window; falling back to rustls defaults");
        ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .context(RustlsConfigSnafu)?
    } else {
        ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(protocol_versions)
            .context(RustlsConfigSnafu)?
    };
    Ok(version_builder
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(crl_verifier))
        .with_no_client_auth())
}

/// Build a plain rustls [`ClientConfig`] (no CRL verifier) from the configured roots.
fn build_plain_rustls_client_config(
    root_certificates: &RootCertificates,
    protocol_versions: &[&'static rustls::SupportedProtocolVersion],
) -> Result<rustls::ClientConfig, TlsError> {
    let root_store = match root_certificates {
        RootCertificates::Default => create_native_root_store(),
        RootCertificates::Custom(pem) => create_root_store_from_pem(pem)?,
        RootCertificates::Extra(pem) => create_extended_root_store(pem)?,
    };
    let provider = crate::tls::crypto_module::CryptoModule::get().provider();
    let builder = if protocol_versions.is_empty() {
        rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .context(RustlsConfigSnafu)?
    } else {
        rustls::ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(protocol_versions)
            .context(RustlsConfigSnafu)?
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
pub(crate) fn build_insecure_rustls_config() -> Result<Arc<rustls::ClientConfig>, TlsError> {
    super::ensure_crypto_provider();
    let provider = crate::tls::crypto_module::CryptoModule::get().provider();
    Ok(Arc::new(
        rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .context(RustlsConfigSnafu)?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifyCertVerifier::new()))
            .with_no_client_auth(),
    ))
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
            supported_algs: crate::tls::crypto_module::CryptoModule::get()
                .signature_verification_algorithms(),
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
    ensure_pem_not_empty(&certs)?;
    for cert in certs {
        root_store.add(cert).context(RootStoreAddSnafu)?;
    }
    Ok(root_store)
}

fn load_root_certificates(tls_config: &TlsConfig) -> Result<RootCertificates, TlsError> {
    if let Some(pem_path) = tls_config.custom_root_store_path.as_ref() {
        if let Some(extra_path) = tls_config.extra_root_store_path.as_ref() {
            tracing::warn!(
                custom_root_store_path = %pem_path.display(),
                extra_root_store_path = %extra_path.display(),
                "extra_root_store_path is ignored because custom_root_store_path is set"
            );
        }
        tracing::debug!(
            path = %pem_path.display(),
            "loading custom root certificate store"
        );
        return Ok(RootCertificates::Custom(
            std::fs::read(pem_path).context(PemParseSnafu)?,
        ));
    }
    if let Some(pem_path) = tls_config.extra_root_store_path.as_ref() {
        tracing::debug!(
            path = %pem_path.display(),
            "loading extra root certificates"
        );
        return Ok(RootCertificates::Extra(
            std::fs::read(pem_path).context(PemParseSnafu)?,
        ));
    }
    Ok(RootCertificates::Default)
}

fn apply_reqwest_root_certificates(
    builder: ClientBuilder,
    root_certificates: &RootCertificates,
) -> Result<ClientBuilder, TlsError> {
    match root_certificates {
        RootCertificates::Default => Ok(builder),
        RootCertificates::Custom(pem) => {
            add_reqwest_pem_roots(builder.tls_built_in_root_certs(false), pem)
        }
        RootCertificates::Extra(pem) => add_reqwest_pem_roots(builder, pem),
    }
}

fn add_reqwest_pem_roots(
    mut builder: ClientBuilder,
    pem: &[u8],
) -> Result<ClientBuilder, TlsError> {
    let certs = reqwest::Certificate::from_pem_bundle(pem).context(ClientBuildSnafu)?;
    ensure_pem_not_empty(&certs)?;
    for cert in certs {
        builder = builder.add_root_certificate(cert);
    }
    Ok(builder)
}

fn root_store_for_crl(
    root_certificates: &RootCertificates,
) -> Result<Option<rustls::RootCertStore>, TlsError> {
    match root_certificates {
        RootCertificates::Default => Ok(None),
        RootCertificates::Custom(pem) => create_root_store_from_pem(pem).map(Some),
        RootCertificates::Extra(pem) => create_extended_root_store(pem).map(Some),
    }
}

fn create_extended_root_store(pem_data: &[u8]) -> Result<rustls::RootCertStore, TlsError> {
    let extra = create_root_store_from_pem(pem_data)?;
    let mut root_store = create_native_root_store();
    root_store.extend(extra.roots);
    Ok(root_store)
}

fn create_native_root_store() -> rustls::RootCertStore {
    let mut native = rustls_native_certs::load_native_certs();
    native.errors.clear();
    let mut root_store = rustls::RootCertStore::empty();
    root_store.add_parsable_certificates(native.certs);
    root_store
}

fn ensure_pem_not_empty<T>(certs: &[T]) -> Result<(), TlsError> {
    if certs.is_empty() {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "no certs in PEM",
        ))
        .context(PemParseSnafu)
    } else {
        Ok(())
    }
}

pub(crate) fn apply_http_pool_settings(builder: ClientBuilder) -> ClientBuilder {
    builder
        .pool_idle_timeout(Some(Duration::from_secs(30)))
        .pool_max_idle_per_host(32)
        .tcp_keepalive(Some(Duration::from_secs(60)))
}

pub(crate) fn configure_http_client(
    builder: ClientBuilder,
    proxy: Option<&ProxyConfig>,
) -> Result<ClientBuilder, TlsError> {
    apply_proxy_to_builder(apply_http_pool_settings(builder), proxy)
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
        scheme = proxy.scheme.as_str(),
        use_proxy_env = proxy.use_proxy_env,
        explicitly_disabled = proxy.explicitly_disabled,
        "proxy config"
    );

    if let Some(host) = proxy.host.as_deref().filter(|s| !s.is_empty()) {
        // Explicit proxy → applied for all schemes; reqwest's `.proxy()` call
        // disables auto env detection (matches JDBC/Go/Node precedence).
        let url = build_proxy_url(host, proxy);
        // `url` is the fully credentialed `{scheme}://user:pass@host` form;
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

/// Build a `{scheme}://[user:pass@]host[:port]` URL from a `ProxyConfig`.
/// Credentials are percent-encoded so values containing `:`, `@`, or `/`
/// don't break URL parsing (a known footgun in the legacy Python connector).
fn build_proxy_url(host: &str, proxy: &ProxyConfig) -> String {
    let scheme = proxy.scheme.as_str();
    let mut url = format!("{scheme}://");
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
    use std::io::Write;

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

    /// Every rustls config the driver builds must carry the linked crypto module's
    /// provider *object*, not merely an equivalent one.
    ///
    /// This is the Phase 3 property that the previous `ClientConfig::builder()`
    /// call sites could not offer: that entry point reads the process-global
    /// default, so the module verifying a connection was decided by whichever
    /// provider won a startup race. `Arc::ptr_eq` is deliberate -- comparing
    /// contents would pass even if each config had built its own provider,
    /// which is exactly the ambiguity being removed.
    #[test]
    fn plain_config_uses_the_module_provider() {
        let config = build_plain_rustls_client_config(&RootCertificates::Default, &[])
            .expect("plain rustls config");
        assert!(
            Arc::ptr_eq(
                config.crypto_provider(),
                &crate::tls::crypto_module::CryptoModule::get().provider()
            ),
            "plain config should use the linked crypto module's provider"
        );
    }

    /// The `verify_certificates=false` path is the one most likely to be built
    /// first in a process, so it is the one most exposed to the global-provider
    /// race that this phase removes.
    #[test]
    fn insecure_config_uses_the_module_provider() {
        let config = build_insecure_rustls_config().expect("insecure rustls config");
        assert!(
            Arc::ptr_eq(
                config.crypto_provider(),
                &crate::tls::crypto_module::CryptoModule::get().provider()
            ),
            "insecure config should use the linked crypto module's provider"
        );
    }

    /// A narrowed protocol-version window must not change which module is in
    /// play -- only which versions it offers.
    #[test]
    fn version_window_does_not_change_the_provider() {
        let config = build_plain_rustls_client_config(
            &RootCertificates::Default,
            &[&rustls::version::TLS13],
        )
        .expect("versioned rustls config");
        assert!(
            Arc::ptr_eq(
                config.crypto_provider(),
                &crate::tls::crypto_module::CryptoModule::get().provider()
            ),
            "version-restricted config should still use the crypto module's provider"
        );
    }

    #[test]
    fn should_prefer_custom_root_store_over_extra_root_store() {
        let mut custom = tempfile::NamedTempFile::new().expect("custom PEM file");
        custom.write_all(b"custom").expect("write custom PEM");
        let mut extra = tempfile::NamedTempFile::new().expect("extra PEM file");
        extra.write_all(b"extra").expect("write extra PEM");
        let config = TlsConfig {
            custom_root_store_path: Some(custom.path().to_path_buf()),
            extra_root_store_path: Some(extra.path().to_path_buf()),
            ..Default::default()
        };

        let roots = load_root_certificates(&config).expect("load roots");

        assert!(matches!(roots, RootCertificates::Custom(pem) if pem == b"custom"));
    }

    #[test]
    fn should_reject_empty_extra_root_store() {
        let extra = tempfile::NamedTempFile::new().expect("extra PEM file");
        let config = TlsConfig {
            extra_root_store_path: Some(extra.path().to_path_buf()),
            ..Default::default()
        };

        let result = configure_tls_builder(
            Client::builder(),
            &config,
            None,
            crate::crl::CrlWorker::shared_lazy(),
        );

        assert!(matches!(result, Err(TlsError::PemParse { .. })));
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
    fn build_proxy_url_preserves_https_scheme() {
        let mut p = proxy(Some("p.example.com"), Some(8443), None, None);
        p.scheme = crate::tls::config::ProxyScheme::Https;
        let url = build_proxy_url("p.example.com", &p);
        assert_eq!(url, "https://p.example.com:8443");
        Proxy::all(&url).expect("reqwest must accept an https proxy URL");
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
        let _ = configure_http_client(Client::builder(), Some(&p)).unwrap();
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

    #[test]
    fn build_tls_client_returns_diag_rustls_config_only_when_requested() {
        let config = TlsConfig::default();

        let (_, without_diag) = build_tls_client_and_rustls_config(
            &config,
            None,
            crate::crl::CrlWorker::shared_lazy(),
            None,
            false,
        )
        .expect("client must build without a diagnostic config");
        assert!(without_diag.is_none());

        let (_, with_diag) = build_tls_client_and_rustls_config(
            &config,
            None,
            crate::crl::CrlWorker::shared_lazy(),
            None,
            true,
        )
        .expect("client must build with a diagnostic config");
        assert!(with_diag.is_some());
    }
}
