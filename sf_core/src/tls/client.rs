use crate::crl::ConnectionCrl;
use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::tls::CrlServerCertVerifier;
use crate::tls::config::{ProxyConfig, TlsConfig};
use crate::tls::error::{
    ClientBuildSnafu, NativeRootLoadSnafu, PemParseSnafu, ProxyBuildSnafu, RedactedUrl,
    RootStoreAddSnafu, RustlsConfigSnafu, TlsError, VerifierBuildSnafu,
};
use once_cell::sync::OnceCell;
use reqwest::{Client, ClientBuilder, NoProxy, Proxy};
use rustls::ClientConfig;
use snafu::ResultExt;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct RootStoreProvider(Arc<RootStoreInner>);

#[derive(Debug)]
struct RootStoreInner {
    roots: OnceCell<PlatformRoots>,
    load_native: fn() -> rustls_native_certs::CertificateResult,
}

#[derive(Debug)]
struct PlatformRoots {
    platform_error: Option<Arc<rustls_native_certs::Error>>,
    native_certs: Vec<rustls::pki_types::CertificateDer<'static>>,
    native: Arc<rustls::RootCertStore>,
    with_bundled: OnceCell<Arc<rustls::RootCertStore>>,
}

enum RootCertificates {
    Default,
    Custom(Vec<u8>),
    Extra(Vec<u8>),
}

impl RootStoreProvider {
    pub fn new() -> Self {
        Self::with_native_loader(rustls_native_certs::load_native_certs)
    }

    fn with_native_loader(load_native: fn() -> rustls_native_certs::CertificateResult) -> Self {
        Self(Arc::new(RootStoreInner {
            roots: OnceCell::new(),
            load_native,
        }))
    }

    #[cfg(test)]
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn snapshot(&self) -> RootStoreSnapshot<'_> {
        RootStoreSnapshot {
            provider: self,
            platform: OnceCell::new(),
        }
    }

    pub(crate) async fn prepare_native_if_needed(
        &self,
        tls_config: &TlsConfig,
        crl_cache_exists: bool,
    ) -> Result<RootStoreSnapshot<'_>, TlsError> {
        let snapshot = self.snapshot();
        if !needs_native_roots(tls_config, crl_cache_exists) || self.0.roots.get().is_some() {
            return Ok(snapshot);
        }
        let provider = self.clone();
        let loaded = tokio::task::spawn_blocking(move || provider.platform().map(|_| ()))
            .await
            .boxed()
            .context(NativeRootLoadSnafu)?;
        if let Err(err) = loaded {
            let _ = snapshot.platform.set(Err(Arc::new(err)));
        }
        Ok(snapshot)
    }

    fn platform(&self) -> Result<&PlatformRoots, rustls_native_certs::Error> {
        self.0
            .roots
            .get_or_try_init(|| load_platform_roots(self.0.load_native))
    }
}

/// One client build's view of a [`RootStoreProvider`]. A failed OS load is
/// remembered only for the lifetime of this view, so every store a single
/// build asks for sees the same outcome while the next build retries.
pub(crate) struct RootStoreSnapshot<'a> {
    provider: &'a RootStoreProvider,
    platform: OnceCell<Result<&'a PlatformRoots, Arc<rustls_native_certs::Error>>>,
}

impl<'a> RootStoreSnapshot<'a> {
    fn platform(&self) -> Result<&'a PlatformRoots, Arc<rustls_native_certs::Error>> {
        let provider = self.provider;
        self.platform
            .get_or_init(|| provider.platform().map_err(Arc::new))
            .clone()
    }

    pub(crate) fn native_store_lenient(&self) -> Arc<rustls::RootCertStore> {
        match self.platform() {
            Ok(platform) => Arc::clone(&platform.native),
            Err(_) => Arc::new(native_store_from_certs(&[])),
        }
    }

    pub(crate) fn native_store_strict(
        &self,
    ) -> Result<Arc<rustls::RootCertStore>, Arc<rustls_native_certs::Error>> {
        let platform = self.platform()?;
        match &platform.platform_error {
            None => Ok(Arc::clone(&platform.native)),
            Some(err) => Err(Arc::clone(err)),
        }
    }

    pub(crate) fn with_bundled(&self) -> Result<Arc<rustls::RootCertStore>, TlsError> {
        match self.platform() {
            Ok(platform) => {
                let store = platform
                    .with_bundled
                    .get_or_try_init(|| build_bundled_store(&platform.native_certs))?;
                Ok(Arc::clone(store))
            }
            Err(_) => build_bundled_store(&[]),
        }
    }

    fn with_bundled_and_pem(
        &self,
        extra_pem: Option<&[u8]>,
    ) -> Result<Arc<rustls::RootCertStore>, TlsError> {
        extend_with_optional_pem(self.with_bundled()?, extra_pem)
    }

    fn with_native_and_pem(
        &self,
        extra_pem: &[u8],
    ) -> Result<Arc<rustls::RootCertStore>, TlsError> {
        extend_with_optional_pem(self.native_store_lenient(), Some(extra_pem))
    }

    fn plain_store(
        &self,
        root_certificates: &RootCertificates,
    ) -> Result<Arc<rustls::RootCertStore>, TlsError> {
        match root_certificates {
            RootCertificates::Custom(pem) => create_root_store_from_pem(pem).map(Arc::new),
            RootCertificates::Extra(pem) => self.with_bundled_and_pem(Some(pem)),
            RootCertificates::Default => self.with_bundled_and_pem(None),
        }
    }

    fn crl_store(
        &self,
        root_certificates: &RootCertificates,
    ) -> Result<Arc<rustls::RootCertStore>, TlsError> {
        match root_certificates {
            RootCertificates::Default => self
                .native_store_strict()
                .boxed()
                .context(VerifierBuildSnafu),
            RootCertificates::Custom(pem) => create_root_store_from_pem(pem).map(Arc::new),
            RootCertificates::Extra(pem) => self.with_native_and_pem(pem),
        }
    }
}

impl Default for RootStoreProvider {
    fn default() -> Self {
        Self::new()
    }
}

fn needs_native_roots(tls_config: &TlsConfig, crl_cache_exists: bool) -> bool {
    let crl_download_needs_native_roots = !crl_cache_exists
        && matches!(
            tls_config.crl_config.check_mode,
            CertRevocationCheckMode::Enabled | CertRevocationCheckMode::Advisory
        );
    tls_config.verify_certificates
        && (tls_config.custom_root_store_path.is_none() || crl_download_needs_native_roots)
}

fn load_platform_roots(
    load_native: fn() -> rustls_native_certs::CertificateResult,
) -> Result<PlatformRoots, rustls_native_certs::Error> {
    let mut result = load_native();
    if result.certs.is_empty()
        && let Some(err) = result.errors.pop()
    {
        return Err(err);
    }
    let platform_error = result.errors.pop().map(Arc::new);
    let native = Arc::new(native_store_from_certs(&result.certs));
    Ok(PlatformRoots {
        platform_error,
        native_certs: result.certs,
        native,
        with_bundled: OnceCell::new(),
    })
}

fn native_store_from_certs(
    certs: &[rustls::pki_types::CertificateDer<'static>],
) -> rustls::RootCertStore {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.add_parsable_certificates(certs.iter().cloned());
    root_store
}

fn build_bundled_store(
    native_certs: &[rustls::pki_types::CertificateDer<'static>],
) -> Result<Arc<rustls::RootCertStore>, TlsError> {
    let mut store = rustls::RootCertStore::empty();
    store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    add_native_roots_like_reqwest(&mut store, native_certs)?;
    Ok(Arc::new(store))
}

fn extend_with_optional_pem(
    base: Arc<rustls::RootCertStore>,
    extra_pem: Option<&[u8]>,
) -> Result<Arc<rustls::RootCertStore>, TlsError> {
    let Some(pem) = extra_pem else {
        return Ok(base);
    };
    let extra = create_root_store_from_pem(pem)?;
    let mut store = base.as_ref().clone();
    store.extend(extra.roots);
    Ok(Arc::new(store))
}

/// Create a reqwest Client with TLS configuration
///
/// This is the main entry point for creating HTTP clients in the application.
/// Handles all TLS configuration including CRL validation, custom root stores, etc.
pub fn create_tls_client_with_config(
    tls_config: TlsConfig,
    crl_manager: ConnectionCrl,
    roots: RootStoreProvider,
) -> Result<Client, TlsError> {
    create_tls_client_with_proxy(tls_config, None, crl_manager, roots)
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
    crl_manager: ConnectionCrl,
    roots: RootStoreProvider,
) -> Result<Client, TlsError> {
    build_tls_client_and_rustls_config_sync(
        &tls_config,
        proxy,
        crl_manager,
        &roots.snapshot(),
        None,
        false,
    )
    .map(|(c, _)| c)
}

pub(crate) async fn create_tls_client_with_proxy_prepared(
    tls_config: TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_manager: ConnectionCrl,
    roots: RootStoreProvider,
) -> Result<Client, TlsError> {
    let snapshot = roots
        .prepare_native_if_needed(&tls_config, crl_manager.has_cache())
        .await?;
    build_tls_client_and_rustls_config_sync(&tls_config, proxy, crl_manager, &snapshot, None, false)
        .map(|(c, _)| c)
}

pub(crate) async fn create_tls_client_with_config_prepared(
    tls_config: TlsConfig,
    crl_manager: ConnectionCrl,
    roots: RootStoreProvider,
) -> Result<Client, TlsError> {
    create_tls_client_with_proxy_prepared(tls_config, None, crl_manager, roots).await
}

/// Build a reqwest [`Client`] and, only when `need_diag_config` is set, its
/// diagnostic [`rustls::ClientConfig`].
///
/// On the verified CRL-disabled path it clones the traffic config (no second
/// trust-store load) but does not advertise h2: the diagnostic sends HTTP/1.0.
/// On the CRL path it deliberately omits the revocation verifier so it can
/// still show the chain when a CRL endpoint is unavailable.
pub(crate) async fn build_tls_client_and_rustls_config(
    tls_config: &TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_manager: ConnectionCrl,
    roots: RootStoreProvider,
    connect_timeout: Option<Duration>,
    need_diag_config: bool,
) -> Result<(Client, Option<Arc<rustls::ClientConfig>>), TlsError> {
    let snapshot = roots
        .prepare_native_if_needed(tls_config, crl_manager.has_cache())
        .await?;
    build_tls_client_and_rustls_config_sync(
        tls_config,
        proxy,
        crl_manager,
        &snapshot,
        connect_timeout,
        need_diag_config,
    )
}

fn build_tls_client_and_rustls_config_sync(
    tls_config: &TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_manager: ConnectionCrl,
    roots: &RootStoreSnapshot<'_>,
    connect_timeout: Option<Duration>,
    need_diag_config: bool,
) -> Result<(Client, Option<Arc<rustls::ClientConfig>>), TlsError> {
    #[cfg(feature = "fips")]
    if !tls_config.verify_certificates {
        return crate::tls::error::InsecureTlsRejectedSnafu.fail();
    }

    // The global install covers any reqwest client that still has no
    // preconfigured config. Verified and insecure clients both carry an
    // explicit config, so SNOWFLAKE_TLS_CIPHERS applies to either.
    super::ensure_crypto_provider();
    super::require_fips_provider()?;

    #[cfg(not(feature = "fips"))]
    if !tls_config.verify_certificates {
        tracing::warn!("Creating insecure TLS client - certificate verification disabled");
        let rustls_cfg = build_insecure_traffic_rustls_config(
            &tls_config.versions.enabled_rustls_versions(),
            ClientAlpn::Default,
        )?;
        let mut builder =
            configure_http_client(Client::builder(), proxy)?.use_preconfigured_tls(rustls_cfg);
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
            if matches!(root_certificates, RootCertificates::Default) {
                tracing::debug!("CRL disabled, using native and bundled roots");
            }
            let reqwest_rustls_cfg = build_verified_rustls_config(
                roots.plain_store(&root_certificates)?,
                &protocol_versions,
                tls_config.verify_hostname,
                ClientAlpn::Default,
            )?;
            let diag_rustls_cfg = need_diag_config.then(|| {
                let mut diagnostic = reqwest_rustls_cfg.clone();
                // inspect_tls writes HTTP/1.0; it must never negotiate h2.
                diagnostic.alpn_protocols.clear();
                Arc::new(diagnostic)
            });
            let mut builder = configure_http_client(Client::builder(), proxy)?
                .use_preconfigured_tls(reqwest_rustls_cfg);
            if let Some(ct) = connect_timeout {
                builder = builder.connect_timeout(ct);
            }
            let client = builder.build().context(ClientBuildSnafu)?;
            Ok((client, diag_rustls_cfg))
        }
        CertRevocationCheckMode::Enabled | CertRevocationCheckMode::Advisory => {
            tracing::debug!(
                "CRL validation enabled, creating client with full TLS handshake validation"
            );
            // Compute the enabled protocol-version window before moving
            // `crl_config` into the builder. reqwest's min/max_tls_version are
            // ignored on the preconfigured-rustls path, so the window must be
            // baked into the rustls ClientConfig instead.
            let root_store = roots.crl_store(&root_certificates)?;
            let reqwest_rustls_cfg = build_crl_rustls_config(
                tls_config.crl_config.clone(),
                root_store,
                tls_config.verify_hostname,
                &protocol_versions,
                ClientAlpn::Default,
                crl_manager,
                roots,
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
                    build_plain_rustls_client_config(roots, &root_certificates, &protocol_versions)
                        .map(Arc::new)
                })
                .transpose()?;
            Ok((client, diag_rustls_cfg))
        }
    }
}

/// Like [`create_tls_client_with_proxy`], but with an optional connect timeout
/// covering name lookup, TCP, and TLS. `None` leaves reqwest's connect timer
/// unset.
pub fn create_tls_client_with_proxy_and_timeouts(
    tls_config: TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_manager: ConnectionCrl,
    roots: RootStoreProvider,
    connect_timeout: Option<Duration>,
) -> Result<Client, TlsError> {
    build_tls_client_and_rustls_config_sync(
        &tls_config,
        proxy,
        crl_manager,
        &roots.snapshot(),
        connect_timeout,
        false,
    )
    .map(|(c, _)| c)
}

/// Configure the AWS SDK transport and keep its HTTP/1.1-only ALPN preference
/// in step with the HTTP version reqwest actually sends.
pub(crate) fn configure_http1_tls_builder(
    builder: ClientBuilder,
    tls_config: &TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_manager: ConnectionCrl,
    roots: &RootStoreSnapshot<'_>,
) -> Result<ClientBuilder, TlsError> {
    configure_tls_builder(
        builder.http1_only(),
        tls_config,
        proxy,
        crl_manager,
        roots,
        ClientAlpn::Http1Only,
    )
}

#[derive(Clone, Copy)]
enum ClientAlpn {
    Default,
    Http1Only,
}

impl ClientAlpn {
    fn protocols(self) -> Vec<Vec<u8>> {
        match self {
            Self::Default => vec![b"h2".to_vec(), b"http/1.1".to_vec()],
            Self::Http1Only => vec![b"http/1.1".to_vec()],
        }
    }
}

/// Shared reqwest TLS policy for storage clients. Explicit provider, roots,
/// versions, and ALPN must be set before `.use_preconfigured_tls`: reqwest
/// ignores TLS options subsequently set on its builder. FIPS builds reject
/// insecure configs. Standard builds accept any certificate and handshake
/// signature, and apply `SNOWFLAKE_TLS_CIPHERS` on that config.
fn configure_tls_builder(
    builder: ClientBuilder,
    tls_config: &TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_manager: ConnectionCrl,
    roots: &RootStoreSnapshot<'_>,
    alpn: ClientAlpn,
) -> Result<ClientBuilder, TlsError> {
    #[cfg(feature = "fips")]
    if !tls_config.verify_certificates {
        return crate::tls::error::InsecureTlsRejectedSnafu.fail();
    }

    super::ensure_crypto_provider();
    super::require_fips_provider()?;
    let builder = apply_proxy_to_builder(builder, proxy)?;
    #[cfg(not(feature = "fips"))]
    if !tls_config.verify_certificates {
        tracing::warn!("Creating insecure TLS client - certificate verification disabled");
        let rustls_cfg = build_insecure_traffic_rustls_config(
            &tls_config.versions.enabled_rustls_versions(),
            alpn,
        )?;
        return Ok(builder.use_preconfigured_tls(rustls_cfg));
    }

    let root_certificates = load_root_certificates(tls_config)?;

    match tls_config.crl_config.check_mode {
        CertRevocationCheckMode::Disabled => {
            let rustls_cfg = build_verified_rustls_config(
                roots.plain_store(&root_certificates)?,
                &tls_config.versions.enabled_rustls_versions(),
                tls_config.verify_hostname,
                alpn,
            )?;
            Ok(builder.use_preconfigured_tls(rustls_cfg))
        }
        CertRevocationCheckMode::Enabled | CertRevocationCheckMode::Advisory => {
            tracing::debug!("CRL validation enabled, configuring storage TLS client");
            let protocol_versions = tls_config.versions.enabled_rustls_versions();
            let root_store = roots.crl_store(&root_certificates)?;
            let rustls_cfg = build_crl_rustls_config(
                tls_config.crl_config.clone(),
                root_store,
                tls_config.verify_hostname,
                &protocol_versions,
                alpn,
                crl_manager,
                roots,
            )?;
            Ok(builder.use_preconfigured_tls(rustls_cfg))
        }
    }
}

/// Configure the Azure/GCS HTTP/1.1 storage transport plus `.no_gzip()`.
/// Their encrypted or opaque bytes must not be transparently gunzipped:
/// downstream SHA-256 digests, Content-Length, and ranged-download checks
/// compare the wire bytes, not a decoded body. This mirrors
/// JDBC's `HttpUtil.disableContentCompression()`
/// (`SnowflakeGCSClient.java:237,:432` via `HttpUtil.java:420`) and the
/// intent of Python's `remove_content_encoding` urllib3 hook
/// (`storage_client.py:54-59`).
///
/// The GS/REST client still wants gzip, so this can't be folded into
/// `configure_tls_builder` itself. Azure and GCS call `.http1_only()` after
/// this helper; the preconfigured TLS config must advertise only HTTP/1.1
/// before reqwest builds that client. S3 uses [`configure_http1_tls_builder`]
/// for the same ALPN requirement.
pub(crate) fn configure_storage_client_builder(
    builder: ClientBuilder,
    tls_config: &TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_manager: ConnectionCrl,
    roots: RootStoreProvider,
) -> Result<ClientBuilder, TlsError> {
    configure_storage_from_snapshot(builder, tls_config, proxy, crl_manager, &roots.snapshot())
}

pub(crate) async fn configure_storage_client_builder_prepared(
    builder: ClientBuilder,
    tls_config: &TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_manager: ConnectionCrl,
    roots: RootStoreProvider,
) -> Result<ClientBuilder, TlsError> {
    let snapshot = roots
        .prepare_native_if_needed(tls_config, crl_manager.has_cache())
        .await?;
    configure_storage_from_snapshot(builder, tls_config, proxy, crl_manager, &snapshot)
}

fn configure_storage_from_snapshot(
    builder: ClientBuilder,
    tls_config: &TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_manager: ConnectionCrl,
    roots: &RootStoreSnapshot<'_>,
) -> Result<ClientBuilder, TlsError> {
    configure_tls_builder(
        builder,
        tls_config,
        proxy,
        crl_manager,
        roots,
        ClientAlpn::Http1Only,
    )
    .map(ClientBuilder::no_gzip)
}

fn handshake_provider(
    protocol_versions: &[&'static rustls::SupportedProtocolVersion],
) -> Result<Arc<rustls::crypto::CryptoProvider>, TlsError> {
    super::cipher_policy::provider_for_versions(protocol_versions)
}

/// Build a rustls `ClientConfig` with CRL validation. Shared by
/// [`configure_tls_builder`] (storage clients) and
/// [`build_tls_client_and_rustls_config`] (connection-level client).
fn build_crl_rustls_config(
    crl_config: CrlConfig,
    root_store: Arc<rustls::RootCertStore>,
    verify_hostname: bool,
    protocol_versions: &[&'static rustls::SupportedProtocolVersion],
    alpn: ClientAlpn,
    crl_manager: ConnectionCrl,
    roots: &RootStoreSnapshot<'_>,
) -> Result<rustls::ClientConfig, TlsError> {
    if !verify_hostname {
        tracing::warn!("Hostname verification disabled (CRL path)");
    }
    // Reject an empty version window instead of silently restoring rustls's
    // defaults, which could re-enable a protocol the caller excluded.
    let provider = handshake_provider(protocol_versions)?;
    let version_builder = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(protocol_versions)
        .context(RustlsConfigSnafu)?;

    let crl_verifier = CrlServerCertVerifier::new_with_root_store(
        crl_config,
        root_store,
        verify_hostname,
        crl_manager,
        roots,
    )
    .context(VerifierBuildSnafu)?;

    // Explicit provider rather than `ClientConfig::builder()`: the latter reads
    // the process-global default, so the module verifying this connection's
    // chain would be whichever one won a startup race. Under `fips` that
    // race is the compliance claim.
    let mut config = version_builder
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(crl_verifier))
        .with_no_client_auth();
    config.alpn_protocols = alpn.protocols();
    Ok(config)
}

/// Diagnostic-only rustls [`ClientConfig`] without CRL verification. Unlike
/// the verified reqwest client, CRL verification currently uses native roots;
/// keep its diagnostic on the same roots instead of the native ∪ webpki union.
fn build_plain_rustls_client_config(
    roots: &RootStoreSnapshot<'_>,
    root_certificates: &RootCertificates,
    protocol_versions: &[&'static rustls::SupportedProtocolVersion],
) -> Result<rustls::ClientConfig, TlsError> {
    let root_store = match root_certificates {
        RootCertificates::Default => roots.native_store_lenient(),
        RootCertificates::Custom(pem) => Arc::new(create_root_store_from_pem(pem)?),
        RootCertificates::Extra(pem) => roots.with_native_and_pem(pem)?,
    };
    let versions = if protocol_versions.is_empty() {
        rustls::DEFAULT_VERSIONS
    } else {
        protocol_versions
    };
    let provider = handshake_provider(versions)?;
    let builder = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(versions)
        .context(RustlsConfigSnafu)?;
    Ok(builder
        .with_root_certificates(root_store)
        .with_no_client_auth())
}

/// Module-backed config with reqwest's default trust (native ∪ webpki roots)
/// and no CRL checking, for auxiliary HTTPS clients that have no connection
/// `TlsConfig`: platform detection probes and WIF attestation.
pub(crate) async fn default_verified_client_builder(
    roots: &RootStoreProvider,
) -> Result<ClientBuilder, TlsError> {
    let snapshot = roots
        .prepare_native_if_needed(&TlsConfig::default(), false)
        .await?;
    default_verified_client_builder_from_snapshot(&snapshot)
}

pub(crate) fn default_verified_client_builder_from_snapshot(
    roots: &RootStoreSnapshot<'_>,
) -> Result<ClientBuilder, TlsError> {
    Ok(apply_http_pool_settings(Client::builder())
        .use_preconfigured_tls(build_default_verified_rustls_config(roots)?))
}

fn build_default_verified_rustls_config(
    roots: &RootStoreSnapshot<'_>,
) -> Result<rustls::ClientConfig, TlsError> {
    build_verified_rustls_config(
        roots.with_bundled()?,
        rustls::DEFAULT_VERSIONS,
        true,
        ClientAlpn::Default,
    )
}

/// Reproduce reqwest's verified rustls path with an explicit linked-module
/// provider. The root store supplied here already reflects custom/extra/default
/// precedence; reqwest ignores builder TLS settings after use_preconfigured_tls.
fn build_verified_rustls_config(
    root_store: Arc<rustls::RootCertStore>,
    protocol_versions: &[&'static rustls::SupportedProtocolVersion],
    verify_hostname: bool,
    alpn: ClientAlpn,
) -> Result<rustls::ClientConfig, TlsError> {
    let provider = handshake_provider(protocol_versions)?;
    let supported_algs = provider.signature_verification_algorithms;
    // An empty version window can be constructed through TlsConfig's public
    // fields. Reqwest rejects it; silently using safe defaults would downgrade
    // a caller asking for TLS 1.3 to TLS 1.2.
    let builder = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(protocol_versions)
        .context(RustlsConfigSnafu)?;
    let mut config = if verify_hostname {
        builder
            .with_root_certificates(root_store)
            .with_no_client_auth()
    } else {
        tracing::warn!("Hostname verification disabled");
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(IgnoreHostnameCertVerifier {
                roots: root_store,
                supported_algs,
            }))
            .with_no_client_auth()
    };
    config.alpn_protocols = alpn.protocols();
    Ok(config)
}

#[derive(Debug)]
struct IgnoreHostnameCertVerifier {
    roots: Arc<rustls::RootCertStore>,
    supported_algs: rustls::crypto::WebPkiSupportedAlgorithms,
}

impl rustls::client::danger::ServerCertVerifier for IgnoreHostnameCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::pki_types::CertificateDer<'_>,
        intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        let cert = rustls::server::ParsedCertificate::try_from(end_entity)?;
        rustls::client::verify_server_cert_signed_by_trust_anchor(
            &cert,
            &self.roots,
            intermediates,
            now,
            self.supported_algs.all,
        )?;
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

#[cfg(not(feature = "fips"))]
fn build_insecure_traffic_rustls_config(
    protocol_versions: &[&'static rustls::SupportedProtocolVersion],
    alpn: ClientAlpn,
) -> Result<rustls::ClientConfig, TlsError> {
    let provider = handshake_provider(protocol_versions)?;
    let mut config = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(protocol_versions)
        .context(RustlsConfigSnafu)?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptAnyCert))
        .with_no_client_auth();
    config.alpn_protocols = alpn.protocols();
    Ok(config)
}

#[cfg(not(feature = "fips"))]
/// Accepts any certificate and any handshake signature.
///
/// This is the verifier on the traffic client when `verify_certificates` is
/// false. [`NoVerifyCertVerifier`] still checks handshake signatures and stays
/// on the diagnostic config.
#[derive(Debug)]
struct AcceptAnyCert;

#[cfg(not(feature = "fips"))]
impl rustls::client::danger::ServerCertVerifier for AcceptAnyCert {
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
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        // reqwest's NoVerifier advertises this set today. A narrower list
        // would reject a signature the insecure client currently accepts.
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA1,
            rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}

#[cfg(not(feature = "fips"))]
/// rustls [`ClientConfig`] that skips all certificate verification.
///
/// Returned by [`build_tls_client_and_rustls_config`] for the `verify_certificates=false`
/// path so the diagnostic's TLS probe behaves consistently with the reqwest client: neither
/// verifies the server certificate.  Using a cert-verified config here would cause
/// false-negative TLS failures in environments with custom or self-signed CAs, which is
/// exactly the case where users reach for `verify_certificates=false`.
pub(crate) fn build_insecure_rustls_config() -> Result<Arc<rustls::ClientConfig>, TlsError> {
    super::ensure_crypto_provider();
    let provider = handshake_provider(rustls::DEFAULT_VERSIONS)?;
    Ok(Arc::new(
        rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .context(RustlsConfigSnafu)?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifyCertVerifier::new()))
            .with_no_client_auth(),
    ))
}

#[cfg(not(feature = "fips"))]
/// A diagnostic [`ServerCertVerifier`] that accepts any certificate chain.
///
/// Unlike reqwest's built-in `NoVerifier`, this diagnostic still verifies TLS
/// handshake signatures with the linked provider. It must not be handed to the
/// insecure traffic client as a silent replacement for reqwest's verifier.
///
/// Signature verification goes through the provider's [`WebPkiSupportedAlgorithms`]
/// directly rather than a [`WebPkiServerVerifier`]: the latter's builder returns
/// `NoRootAnchors` when given an empty root store, which would make a no-verify config
/// impossible to construct.
#[derive(Debug)]
struct NoVerifyCertVerifier {
    supported_algs: rustls::crypto::WebPkiSupportedAlgorithms,
}

#[cfg(not(feature = "fips"))]
impl NoVerifyCertVerifier {
    fn new() -> Self {
        Self {
            supported_algs: crate::tls::crypto_module::CryptoModule::get()
                .signature_verification_algorithms(),
        }
    }
}

#[cfg(not(feature = "fips"))]
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

fn add_native_roots_like_reqwest(
    store: &mut rustls::RootCertStore,
    native_certs: &[rustls::pki_types::CertificateDer<'static>],
) -> Result<(), TlsError> {
    let mut valid_count = 0;
    let mut last_invalid = None;
    for cert in native_certs.iter().cloned() {
        match store.add(cert) {
            Ok(()) => valid_count += 1,
            Err(err) => {
                tracing::debug!(?err, "rustls failed to parse native root certificate");
                last_invalid = Some(err);
            }
        }
    }
    // Reqwest also refuses a native store containing only invalid certificates
    // even if its bundled webpki roots are present.
    if valid_count == 0
        && let Some(err) = last_invalid
    {
        return Err(err).context(RootStoreAddSnafu);
    }
    Ok(())
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
    use crate::crl::CrlManager;
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
        let config = build_plain_rustls_client_config(
            &RootStoreProvider::new().snapshot(),
            &RootCertificates::Default,
            &[],
        )
        .expect("plain rustls config");
        assert!(
            Arc::ptr_eq(
                config.crypto_provider(),
                &crate::tls::crypto_module::CryptoModule::get().provider()
            ),
            "plain config should use the linked crypto module's provider"
        );
    }

    #[test]
    fn default_verified_config_uses_the_module_provider() {
        let config = build_default_verified_rustls_config(&RootStoreProvider::new().snapshot())
            .expect("default verified rustls config");
        assert!(Arc::ptr_eq(
            config.crypto_provider(),
            &crate::tls::crypto_module::CryptoModule::get().provider()
        ));
    }

    #[cfg(not(feature = "fips"))]
    /// The standard-build diagnostic must not inherit a host-installed provider.
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
            &RootStoreProvider::new().snapshot(),
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
    fn verified_client_offers_only_the_named_tls13_suites() {
        let config = crate::tls::cipher_policy::with_tls_ciphers_env(
            Some("TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256"),
            || {
                build_verified_rustls_config(
                    RootStoreProvider::new()
                        .snapshot()
                        .with_bundled()
                        .expect("roots"),
                    &[&rustls::version::TLS13],
                    true,
                    ClientAlpn::Default,
                )
                .expect("verified config")
            },
        );
        let suites: Vec<_> = config
            .crypto_provider()
            .cipher_suites
            .iter()
            .map(|suite| suite.suite())
            .collect();
        assert_eq!(
            suites,
            vec![
                rustls::CipherSuite::TLS13_AES_256_GCM_SHA384,
                rustls::CipherSuite::TLS13_AES_128_GCM_SHA256,
            ]
        );
    }

    #[cfg(not(feature = "fips"))]
    #[test]
    fn insecure_traffic_config_applies_the_cipher_allow_list() {
        let (config, http1) = crate::tls::cipher_policy::with_tls_ciphers_env(
            Some("TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256"),
            || {
                let config = build_insecure_traffic_rustls_config(
                    &[&rustls::version::TLS13],
                    ClientAlpn::Default,
                )
                .expect("insecure traffic config");
                let http1 = build_insecure_traffic_rustls_config(
                    rustls::DEFAULT_VERSIONS,
                    ClientAlpn::Http1Only,
                )
                .expect("http1 insecure config");
                (config, http1)
            },
        );
        let suites: Vec<_> = config
            .crypto_provider()
            .cipher_suites
            .iter()
            .map(|suite| suite.suite())
            .collect();
        assert_eq!(
            suites,
            vec![
                rustls::CipherSuite::TLS13_AES_256_GCM_SHA384,
                rustls::CipherSuite::TLS13_AES_128_GCM_SHA256,
            ]
        );
        assert_eq!(
            config.alpn_protocols,
            vec![b"h2".to_vec(), b"http/1.1".to_vec()]
        );
        assert_eq!(http1.alpn_protocols, vec![b"http/1.1".to_vec()]);
        let http1_suites: Vec<_> = http1
            .crypto_provider()
            .cipher_suites
            .iter()
            .map(|suite| suite.suite())
            .collect();
        assert!(http1_suites.contains(&rustls::CipherSuite::TLS13_AES_256_GCM_SHA384));
        assert!(http1_suites.contains(&rustls::CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256));
        assert!(!http1_suites.contains(&rustls::CipherSuite::TLS13_CHACHA20_POLY1305_SHA256));
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

        let result = configure_storage_client_builder(
            Client::builder(),
            &config,
            None,
            crate::crl::CrlManager::new().connection_crl(),
            RootStoreProvider::new(),
        );

        assert!(matches!(result, Err(TlsError::PemParse { .. })));
    }

    #[test]
    fn one_provider_reuses_the_loaded_root_stores() {
        let provider = RootStoreProvider::new();
        let first = provider.snapshot().with_bundled().expect("bundled");
        let second = provider.snapshot().with_bundled().expect("bundled");
        assert!(Arc::ptr_eq(&first, &second));
        assert!(Arc::ptr_eq(
            &provider.snapshot().native_store_lenient(),
            &provider.snapshot().native_store_lenient(),
        ));
    }

    static FAILED_NATIVE_LOADS: std::sync::atomic::AtomicUsize =
        std::sync::atomic::AtomicUsize::new(0);

    fn count_failed_native_load() -> rustls_native_certs::CertificateResult {
        FAILED_NATIVE_LOADS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        empty_native_error()
    }

    #[tokio::test]
    async fn prepared_client_build_reuses_a_failed_native_load() {
        let provider = RootStoreProvider::with_native_loader(count_failed_native_load);
        build_tls_client_and_rustls_config(
            &TlsConfig::default(),
            Some(&ProxyConfig::default()),
            CrlManager::new().connection_crl(),
            provider.clone(),
            None,
            true,
        )
        .await
        .expect("bundled fallback client");
        assert_eq!(
            FAILED_NATIVE_LOADS.load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        assert!(provider.0.roots.get().is_none());
    }

    static STORAGE_LOAD_THREADS: std::sync::Mutex<Vec<std::thread::ThreadId>> =
        std::sync::Mutex::new(Vec::new());

    fn record_failed_native_load_thread() -> rustls_native_certs::CertificateResult {
        STORAGE_LOAD_THREADS
            .lock()
            .expect("load-thread list")
            .push(std::thread::current().id());
        empty_native_error()
    }

    #[tokio::test]
    async fn failed_native_load_at_login_retries_storage_off_the_worker() {
        STORAGE_LOAD_THREADS
            .lock()
            .expect("load-thread list")
            .clear();
        let worker = std::thread::current().id();
        let provider = RootStoreProvider::with_native_loader(record_failed_native_load_thread);
        build_tls_client_and_rustls_config(
            &TlsConfig::default(),
            Some(&ProxyConfig::default()),
            CrlManager::new().connection_crl(),
            provider.clone(),
            None,
            false,
        )
        .await
        .expect("login client");
        configure_storage_client_builder_prepared(
            Client::builder(),
            &TlsConfig::default(),
            Some(&ProxyConfig::default()),
            CrlManager::new().connection_crl(),
            provider.clone(),
        )
        .await
        .expect("storage client")
        .build()
        .expect("build storage client");
        let threads = STORAGE_LOAD_THREADS.lock().expect("load-thread list");
        assert_eq!(threads.len(), 2);
        assert!(threads.iter().all(|id| *id != worker));
        assert!(provider.0.roots.get().is_none());
    }

    fn empty_native_error() -> rustls_native_certs::CertificateResult {
        let mut result = rustls_native_certs::CertificateResult::default();
        result.errors.push(rustls_native_certs::Error {
            context: "opening root store",
            kind: rustls_native_certs::ErrorKind::Os(Box::new(std::io::Error::other(
                "native store unavailable",
            ))),
        });
        result
    }

    fn native_load_must_not_run() -> rustls_native_certs::CertificateResult {
        panic!("native load must not run when custom_root_store_path replaces the OS store");
    }

    fn fail_once_then_empty_ok() -> rustls_native_certs::CertificateResult {
        thread_local! {
            static CALLS: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
        }
        CALLS.with(|calls| {
            let n = calls.get();
            calls.set(n.saturating_add(1));
            if n == 0 {
                empty_native_error()
            } else {
                rustls_native_certs::CertificateResult::default()
            }
        })
    }

    fn test_ca_pem() -> Vec<u8> {
        use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair};

        let key = KeyPair::generate().expect("generate private CA key");
        let mut params = CertificateParams::new(vec![]).expect("CA parameters");
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params
            .self_signed(&key)
            .expect("sign CA")
            .pem()
            .into_bytes()
    }

    #[test]
    fn failed_native_load_does_not_block_custom_pem() {
        let pem = test_ca_pem();
        let mut pem_file = tempfile::NamedTempFile::new().expect("custom PEM");
        pem_file.write_all(&pem).expect("write custom PEM");
        let config = TlsConfig {
            custom_root_store_path: Some(pem_file.path().to_path_buf()),
            ..Default::default()
        };
        let provider = RootStoreProvider::with_native_loader(native_load_must_not_run);
        create_tls_client_with_proxy(
            config,
            Some(&ProxyConfig::default()),
            crate::crl::CrlManager::new().connection_crl(),
            provider.clone(),
        )
        .expect("custom PEM client");
        assert!(provider.0.roots.get().is_none());
    }

    #[test]
    fn needs_native_roots_includes_crl_download_until_the_cache_exists() {
        let custom = TlsConfig {
            custom_root_store_path: Some(std::path::PathBuf::from("custom.pem")),
            ..Default::default()
        };
        assert!(!needs_native_roots(&custom, false));

        let mut crl_on = custom.clone();
        crl_on.crl_config.check_mode = CertRevocationCheckMode::Enabled;
        assert!(needs_native_roots(&crl_on, false));
        assert!(!needs_native_roots(&crl_on, true));

        crl_on.crl_config.check_mode = CertRevocationCheckMode::Advisory;
        assert!(needs_native_roots(&crl_on, false));
        assert!(!needs_native_roots(&crl_on, true));

        crl_on.verify_certificates = false;
        assert!(!needs_native_roots(&crl_on, false));

        assert!(needs_native_roots(&TlsConfig::default(), true));
    }

    static CUSTOM_CRL_NATIVE_LOADS: std::sync::atomic::AtomicUsize =
        std::sync::atomic::AtomicUsize::new(0);
    static CUSTOM_CRL_LOAD_THREAD: std::sync::Mutex<Option<std::thread::ThreadId>> =
        std::sync::Mutex::new(None);

    fn count_custom_crl_native_load() -> rustls_native_certs::CertificateResult {
        CUSTOM_CRL_NATIVE_LOADS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *CUSTOM_CRL_LOAD_THREAD
            .lock()
            .expect("custom CRL load thread") = Some(std::thread::current().id());
        empty_native_error()
    }

    #[tokio::test]
    async fn custom_crl_prepares_a_failed_native_load_once_per_cache() {
        CUSTOM_CRL_NATIVE_LOADS.store(0, std::sync::atomic::Ordering::Relaxed);
        let worker = std::thread::current().id();
        let pem = test_ca_pem();
        let mut pem_file = tempfile::NamedTempFile::new().expect("custom PEM");
        pem_file.write_all(&pem).expect("write custom PEM");
        let config = TlsConfig {
            custom_root_store_path: Some(pem_file.path().to_path_buf()),
            crl_config: CrlConfig {
                check_mode: CertRevocationCheckMode::Enabled,
                enable_memory_caching: false,
                enable_disk_caching: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let provider = RootStoreProvider::with_native_loader(count_custom_crl_native_load);
        let manager = CrlManager::new().connection_crl();
        build_tls_client_and_rustls_config(
            &config,
            Some(&ProxyConfig::default()),
            manager.clone(),
            provider.clone(),
            None,
            false,
        )
        .await
        .expect("custom PEM client with CRL download");
        assert_eq!(
            CUSTOM_CRL_NATIVE_LOADS.load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        let load_thread = CUSTOM_CRL_LOAD_THREAD
            .lock()
            .expect("custom CRL load thread")
            .expect("load thread was recorded");
        assert_ne!(load_thread, worker);
        assert!(provider.0.roots.get().is_none());

        build_tls_client_and_rustls_config(
            &config,
            Some(&ProxyConfig::default()),
            manager,
            provider.clone(),
            None,
            false,
        )
        .await
        .expect("second client reuses the CRL cache");
        assert_eq!(
            CUSTOM_CRL_NATIVE_LOADS.load(std::sync::atomic::Ordering::Relaxed),
            1
        );

        build_tls_client_and_rustls_config(
            &config,
            Some(&ProxyConfig::default()),
            CrlManager::new().connection_crl(),
            provider,
            None,
            false,
        )
        .await
        .expect("a new CRL cache prepares again");
        assert_eq!(
            CUSTOM_CRL_NATIVE_LOADS.load(std::sync::atomic::Ordering::Relaxed),
            2
        );
    }

    static EXTRA_CRL_NATIVE_LOADS: std::sync::atomic::AtomicUsize =
        std::sync::atomic::AtomicUsize::new(0);

    fn count_extra_crl_native_load() -> rustls_native_certs::CertificateResult {
        EXTRA_CRL_NATIVE_LOADS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        empty_native_error()
    }

    #[tokio::test]
    async fn extra_crl_reuses_a_failed_native_load() {
        EXTRA_CRL_NATIVE_LOADS.store(0, std::sync::atomic::Ordering::Relaxed);
        let pem = test_ca_pem();
        let mut pem_file = tempfile::NamedTempFile::new().expect("extra PEM");
        pem_file.write_all(&pem).expect("write extra PEM");
        let config = TlsConfig {
            extra_root_store_path: Some(pem_file.path().to_path_buf()),
            crl_config: CrlConfig {
                check_mode: CertRevocationCheckMode::Enabled,
                enable_memory_caching: false,
                enable_disk_caching: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let provider = RootStoreProvider::with_native_loader(count_extra_crl_native_load);
        build_tls_client_and_rustls_config(
            &config,
            Some(&ProxyConfig::default()),
            CrlManager::new().connection_crl(),
            provider.clone(),
            None,
            false,
        )
        .await
        .expect("extra PEM client with CRL");
        assert_eq!(
            EXTRA_CRL_NATIVE_LOADS.load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        assert!(provider.0.roots.get().is_none());
    }

    #[test]
    fn failed_native_load_falls_back_to_bundled_roots() {
        let provider = RootStoreProvider::with_native_loader(empty_native_error);
        let default = provider
            .snapshot()
            .plain_store(&RootCertificates::Default)
            .expect("bundled fallback");
        assert!(default.roots.contains(&webpki_roots::TLS_SERVER_ROOTS[0]));

        let pem = test_ca_pem();
        let extra = provider
            .snapshot()
            .plain_store(&RootCertificates::Extra(pem.clone()))
            .expect("extra PEM plus bundled fallback");
        let extra_anchor = &create_root_store_from_pem(&pem)
            .expect("parse extra CA")
            .roots[0];
        assert!(extra.roots.contains(extra_anchor));
        assert!(extra.roots.contains(&webpki_roots::TLS_SERVER_ROOTS[0]));
        assert!(
            provider.0.roots.get().is_none(),
            "a failed native load must not be cached"
        );
    }

    #[test]
    fn failed_native_load_retries_on_the_next_plain_client() {
        let provider = RootStoreProvider::with_native_loader(fail_once_then_empty_ok);
        let _first = provider
            .snapshot()
            .plain_store(&RootCertificates::Default)
            .expect("bundled fallback");
        assert!(provider.0.roots.get().is_none());

        let second = provider
            .snapshot()
            .plain_store(&RootCertificates::Default)
            .expect("retry native load");
        assert!(provider.0.roots.get().is_some());
        assert!(second.roots.contains(&webpki_roots::TLS_SERVER_ROOTS[0]));
        assert!(Arc::ptr_eq(
            &second,
            &provider
                .snapshot()
                .with_bundled()
                .expect("cached bundled store")
        ));
    }

    #[test]
    fn failed_native_load_still_fails_the_crl_default_path() {
        let provider = RootStoreProvider::with_native_loader(empty_native_error);
        assert!(provider.snapshot().native_store_strict().is_err());
        let config = TlsConfig {
            crl_config: CrlConfig {
                check_mode: CertRevocationCheckMode::Enabled,
                ..Default::default()
            },
            ..Default::default()
        };
        let result = create_tls_client_with_proxy(
            config,
            Some(&ProxyConfig::default()),
            crate::crl::CrlManager::new().connection_crl(),
            provider,
        );
        assert!(
            matches!(result, Err(TlsError::VerifierBuild { .. })),
            "CRL default path must surface the native load error: {result:?}"
        );
    }

    #[test]
    fn default_roots_include_bundled_webpki_and_native_anchors() {
        let provider = RootStoreProvider::new();
        let roots = provider
            .snapshot()
            .plain_store(&RootCertificates::Default)
            .expect("build default trust store");
        assert!(
            roots.roots.contains(&webpki_roots::TLS_SERVER_ROOTS[0]),
            "default roots must include the bundled webpki trust anchors"
        );

        let mut native = rustls::RootCertStore::empty();
        native.add_parsable_certificates(rustls_native_certs::load_native_certs().certs);
        for anchor in &native.roots {
            assert!(
                roots.roots.contains(anchor),
                "default roots must also include every usable native trust anchor"
            );
        }
    }

    #[test]
    fn custom_roots_replace_defaults_and_extra_roots_extend_them() {
        use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair};

        let key = KeyPair::generate().expect("generate private CA key");
        let mut params = CertificateParams::new(vec![]).expect("CA parameters");
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let pem = params
            .self_signed(&key)
            .expect("sign CA")
            .pem()
            .into_bytes();
        let private = create_root_store_from_pem(&pem).expect("parse private CA");
        let anchor = &private.roots[0];

        let provider = RootStoreProvider::new();
        let custom = provider
            .snapshot()
            .plain_store(&RootCertificates::Custom(pem.clone()))
            .expect("build custom trust store");
        assert_eq!(
            custom.roots, private.roots,
            "custom CA must replace defaults"
        );

        let extra = provider
            .snapshot()
            .plain_store(&RootCertificates::Extra(pem))
            .expect("build extra trust store");
        assert!(extra.roots.contains(anchor), "extra CA must be trusted");
        assert!(
            extra.roots.contains(&webpki_roots::TLS_SERVER_ROOTS[0]),
            "extra CA must not replace bundled defaults"
        );
    }

    #[test]
    fn inverted_public_version_window_does_not_silently_enable_tls12() {
        use crate::tls::config::{TlsVersion, TlsVersions};

        for check_mode in [
            CertRevocationCheckMode::Disabled,
            CertRevocationCheckMode::Enabled,
            CertRevocationCheckMode::Advisory,
        ] {
            let config = TlsConfig {
                versions: TlsVersions {
                    min: TlsVersion::Tls13,
                    max: TlsVersion::Tls12,
                },
                crl_config: CrlConfig {
                    check_mode,
                    ..Default::default()
                },
                ..Default::default()
            };
            let result = create_tls_client_with_proxy(
                config,
                Some(&ProxyConfig::default()),
                crate::crl::CrlManager::new().connection_crl(),
                RootStoreProvider::new(),
            );
            assert!(
                matches!(&result, Err(TlsError::RustlsConfig { .. })),
                "a reversed TLS version window must be rejected before connecting: {result:?}"
            );
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

    #[cfg(feature = "fips")]
    #[test]
    fn fips_rejects_insecure_connection_clients() {
        for config in [
            TlsConfig {
                verify_certificates: false,
                ..Default::default()
            },
            TlsConfig::insecure(),
        ] {
            for need_diag_config in [false, true] {
                let result = build_tls_client_and_rustls_config_sync(
                    &config,
                    None,
                    CrlManager::new().connection_crl(),
                    &RootStoreProvider::new().snapshot(),
                    None,
                    need_diag_config,
                );

                assert!(matches!(result, Err(TlsError::InsecureTlsRejected { .. })));
            }
        }
    }

    #[cfg(feature = "fips")]
    #[test]
    fn fips_rejects_insecure_storage_clients() {
        let config = TlsConfig {
            verify_certificates: false,
            ..Default::default()
        };

        let s3 = configure_http1_tls_builder(
            Client::builder(),
            &config,
            None,
            CrlManager::new().connection_crl(),
            &RootStoreProvider::new().snapshot(),
        );
        assert!(matches!(s3, Err(TlsError::InsecureTlsRejected { .. })));
        let azure_gcs = configure_storage_client_builder(
            Client::builder(),
            &config,
            None,
            CrlManager::new().connection_crl(),
            RootStoreProvider::new(),
        );
        assert!(matches!(
            azure_gcs,
            Err(TlsError::InsecureTlsRejected { .. })
        ));
    }

    #[cfg(not(feature = "fips"))]
    #[test]
    fn non_fips_builders_accept_insecure_clients() {
        let config = TlsConfig::insecure();
        let roots = RootStoreProvider::new();

        let (_, diagnostic) = build_tls_client_and_rustls_config_sync(
            &config,
            None,
            CrlManager::new().connection_crl(),
            &roots.snapshot(),
            None,
            true,
        )
        .expect("non-FIPS insecure connection client");
        assert!(diagnostic.is_some());
        configure_http1_tls_builder(
            Client::builder(),
            &config,
            None,
            CrlManager::new().connection_crl(),
            &roots.snapshot(),
        )
        .expect("non-FIPS insecure S3 client")
        .build()
        .expect("build S3 client");
        configure_storage_client_builder(
            Client::builder(),
            &config,
            None,
            CrlManager::new().connection_crl(),
            roots,
        )
        .expect("non-FIPS insecure Azure/GCS client")
        .build()
        .expect("build Azure/GCS client");
    }

    #[cfg(not(feature = "fips"))]
    #[test]
    fn insecure_builders_reject_an_unknown_cipher() {
        crate::tls::cipher_policy::with_tls_ciphers_env(Some("DHE-RSA-AES256-GCM-SHA384"), || {
            let connection = build_tls_client_and_rustls_config_sync(
                &TlsConfig::insecure(),
                None,
                CrlManager::new().connection_crl(),
                &RootStoreProvider::new().snapshot(),
                None,
                false,
            )
            .expect_err("unknown cipher on the connection client");
            let text = connection.to_string();
            assert!(text.contains("SNOWFLAKE_TLS_CIPHERS"), "{text}");
            assert!(text.contains("DHE-RSA-AES256-GCM-SHA384"), "{text}");

            let storage = configure_http1_tls_builder(
                Client::builder(),
                &TlsConfig::insecure(),
                None,
                CrlManager::new().connection_crl(),
                &RootStoreProvider::new().snapshot(),
            )
            .expect_err("unknown cipher on the storage client");
            assert!(
                storage.to_string().contains("SNOWFLAKE_TLS_CIPHERS"),
                "{storage}"
            );
        });
    }

    #[tokio::test]
    async fn build_tls_client_returns_diag_rustls_config_only_when_requested() {
        let config = TlsConfig::default();

        let roots = RootStoreProvider::new();
        let (_, without_diag) = build_tls_client_and_rustls_config(
            &config,
            None,
            crate::crl::CrlManager::new().connection_crl(),
            roots.clone(),
            None,
            false,
        )
        .await
        .expect("client must build without a diagnostic config");
        assert!(without_diag.is_none());

        let (_, with_diag) = build_tls_client_and_rustls_config(
            &config,
            None,
            crate::crl::CrlManager::new().connection_crl(),
            roots.clone(),
            None,
            true,
        )
        .await
        .expect("client must build with a diagnostic config");
        assert!(with_diag.is_some());
    }
}
