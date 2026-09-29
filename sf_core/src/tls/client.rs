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

/// Build a reqwest [`Client`] and, only when `need_diag_config` is set, its
/// diagnostic [`rustls::ClientConfig`].
///
/// On the verified CRL-disabled path it clones the traffic config (no second
/// trust-store load) but does not advertise h2: the diagnostic sends HTTP/1.0.
/// On the CRL path it deliberately omits the revocation verifier so it can
/// still show the chain when a CRL endpoint is unavailable.
pub(crate) fn build_tls_client_and_rustls_config(
    tls_config: &TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_worker: SharedCrlWorker,
    connect_timeout: Option<Duration>,
    need_diag_config: bool,
) -> Result<(Client, Option<Arc<rustls::ClientConfig>>), TlsError> {
    // The insecure branch still lets reqwest resolve the global provider at
    // build time. With the `-no-provider` features an empty slot would panic
    // ("No provider set"). Verified traffic below uses an explicit config,
    // but both paths pass through the process-wide FIPS gate.
    super::ensure_crypto_provider();
    // In `fips-tls` builds this construction path fails closed if the global
    // provider is non-FIPS. The insecure branch below still uses that provider,
    // verified traffic uses the linked module explicitly. Auxiliary raw
    // clients require their own gate and are not covered by this call.
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
            if matches!(root_certificates, RootCertificates::Default) {
                tracing::debug!("CRL disabled, using native and bundled roots");
            }
            let reqwest_rustls_cfg = build_verified_rustls_config(
                build_plain_root_store(&root_certificates)?,
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
            let root_store_override = root_store_for_crl(&root_certificates)?;
            let reqwest_rustls_cfg = build_crl_rustls_config(
                tls_config.crl_config.clone(),
                root_store_override,
                tls_config.verify_hostname,
                &protocol_versions,
                ClientAlpn::Default,
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

/// Configure the AWS SDK transport and keep its HTTP/1.1-only ALPN preference
/// in step with the HTTP version reqwest actually sends.
pub(crate) fn configure_http1_tls_builder(
    builder: ClientBuilder,
    tls_config: &TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_worker: SharedCrlWorker,
) -> Result<ClientBuilder, TlsError> {
    configure_tls_builder(
        builder.http1_only(),
        tls_config,
        proxy,
        crl_worker,
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

/// Shared reqwest TLS policy for HTTP/1.1 storage clients. Explicit provider,
/// roots, versions, and ALPN must be set before `.use_preconfigured_tls`:
/// reqwest ignores TLS options subsequently set on its builder.
/// The insecure path retains reqwest's verifier under the FIPS global gate.
fn configure_tls_builder(
    builder: ClientBuilder,
    tls_config: &TlsConfig,
    proxy: Option<&ProxyConfig>,
    crl_worker: SharedCrlWorker,
    alpn: ClientAlpn,
) -> Result<ClientBuilder, TlsError> {
    // The returned builder is built by the caller. Its insecure path still
    // resolves the global provider, so install it before handing the builder
    // back; verified clients use their explicit linked-module config.
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
            let rustls_cfg = build_verified_rustls_config(
                build_plain_root_store(&root_certificates)?,
                &tls_config.versions.enabled_rustls_versions(),
                tls_config.verify_hostname,
                alpn,
            )?;
            Ok(builder.use_preconfigured_tls(rustls_cfg))
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
                alpn,
                crl_worker,
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
    crl_worker: SharedCrlWorker,
) -> Result<ClientBuilder, TlsError> {
    configure_tls_builder(
        builder,
        tls_config,
        proxy,
        crl_worker,
        ClientAlpn::Http1Only,
    )
    .map(ClientBuilder::no_gzip)
}

/// Build a rustls `ClientConfig` with CRL validation. Shared by
/// [`configure_tls_builder`] (storage clients) and
/// [`build_tls_client_and_rustls_config`] (connection-level client).
fn build_crl_rustls_config(
    crl_config: CrlConfig,
    root_store_override: Option<rustls::RootCertStore>,
    verify_hostname: bool,
    protocol_versions: &[&'static rustls::SupportedProtocolVersion],
    alpn: ClientAlpn,
    crl_worker: SharedCrlWorker,
) -> Result<rustls::ClientConfig, TlsError> {
    if !verify_hostname {
        tracing::warn!("Hostname verification disabled (CRL path)");
    }
    // Reject an empty version window instead of silently restoring rustls's
    // defaults, which could re-enable a protocol the caller excluded.
    let provider = crate::tls::crypto_module::CryptoModule::get().provider();
    let version_builder = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(protocol_versions)
        .context(RustlsConfigSnafu)?;

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

/// Reproduce reqwest's verified rustls path with an explicit linked-module
/// provider. The root store supplied here already reflects custom/extra/default
/// precedence; reqwest ignores builder TLS settings after use_preconfigured_tls.
fn build_verified_rustls_config(
    root_store: rustls::RootCertStore,
    protocol_versions: &[&'static rustls::SupportedProtocolVersion],
    verify_hostname: bool,
    alpn: ClientAlpn,
) -> Result<rustls::ClientConfig, TlsError> {
    let provider = crate::tls::crypto_module::CryptoModule::get().provider();
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
    roots: rustls::RootCertStore,
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

/// Reqwest's built-in verified rustls client trusts custom roots instead of
/// bundled roots, extra roots alongside them, or the native ∪ webpki default.
/// A preconfigured config must make that choice itself: reqwest ignores its
/// `.add_root_certificate` and `.tls_built_in_root_certs` settings in that mode.
fn build_plain_root_store(
    root_certificates: &RootCertificates,
) -> Result<rustls::RootCertStore, TlsError> {
    let mut store = match root_certificates {
        RootCertificates::Custom(pem) => return create_root_store_from_pem(pem),
        RootCertificates::Extra(pem) => create_root_store_from_pem(pem)?,
        RootCertificates::Default => rustls::RootCertStore::empty(),
    };
    store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    add_native_roots_like_reqwest(&mut store)?;
    Ok(store)
}

fn add_native_roots_like_reqwest(store: &mut rustls::RootCertStore) -> Result<(), TlsError> {
    let mut valid_count = 0;
    let mut last_invalid = None;
    for cert in rustls_native_certs::load_native_certs().certs {
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

        let result = configure_storage_client_builder(
            Client::builder(),
            &config,
            None,
            crate::crl::CrlWorker::shared_lazy(),
        );

        assert!(matches!(result, Err(TlsError::PemParse { .. })));
    }

    #[test]
    fn default_roots_include_bundled_webpki_and_native_anchors() {
        let roots =
            build_plain_root_store(&RootCertificates::Default).expect("build default trust store");
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

        let custom = build_plain_root_store(&RootCertificates::Custom(pem.clone()))
            .expect("build custom trust store");
        assert_eq!(
            custom.roots, private.roots,
            "custom CA must replace defaults"
        );

        let extra =
            build_plain_root_store(&RootCertificates::Extra(pem)).expect("build extra trust store");
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
                crate::crl::CrlWorker::shared_lazy(),
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
