pub mod aws_http_client;
pub mod client;
pub mod config;
pub mod crl_verifier;
pub mod error;
pub mod revocation;
#[cfg(test)]
pub mod test_helpers;
pub mod x509_utils;

pub use client::{
    create_tls_client_with_config, create_tls_client_with_proxy,
    create_tls_client_with_proxy_and_timeouts,
};
pub use config::{ProxyConfig, TlsConfig};
pub(crate) use crl_verifier::CrlServerCertVerifier;
pub use x509_utils::{crl_times, extract_skid, subject_der_hash, verify_crl_signature};

/// Guarantees a rustls default provider is in place, installing aws-lc-rs if
/// nothing has claimed the slot yet.
///
/// Every `reqwest::Client` the driver builds resolves its crypto backend
/// through `CryptoProvider::get_default()`. Installing from one place, rather
/// than ad hoc next to each client, is what stops a client built early in the
/// process (telemetry, CRL prefetch, cloud transfers) from silently resolving
/// to a different backend than the one carrying the session's own traffic --
/// under `--features fips-tls` that difference is the whole compliance claim.
///
/// `install_default` is already one-shot inside rustls: a later call returns
/// `Err` with the provider that won. This function has no crate-local `Once`,
/// so it still installs after the Windows ODBC driver manager unloads and
/// reloads the DLL — rustls's slot is empty again then, and a `Once` in this
/// crate would not run.
///
/// Installation is best-effort by design: `install_default` returns `Err` when
/// an embedding application has already installed its own provider, and
/// stomping on that would be worse than honouring it. In `fips-tls` builds the
/// resulting provider is checked and a mismatch is logged loudly rather than
/// panicking, because this runs beneath an FFI boundary where unwinding is
/// undefined behaviour.
pub(crate) fn ensure_crypto_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none()
        && let Err(existing) = rustls::crypto::aws_lc_rs::default_provider().install_default()
    {
        tracing::debug!(
            existing_provider_is_fips = existing.fips(),
            "rustls crypto provider already installed; leaving it in place"
        );
    }

    #[cfg(feature = "fips-tls")]
    if !fips_mode_active() {
        tracing::error!(
            "driver was built with the `fips-tls` feature but the active rustls crypto \
             provider is not in FIPS mode; TLS is NOT FIPS compliant"
        );
    }
}

/// Whether the rustls crypto provider actually in force is operating in FIPS
/// mode.
///
/// Answers a question about **TLS only**, and about the *installed provider*
/// rather than about this build. A build without `fips-tls` reports `true` if an
/// embedding application installed a FIPS provider before the driver
/// initialised — accurate for TLS, and still silent on JWT signing, DPoP, stage
/// file encryption and key parsing, which run on OpenSSL either way.
///
/// Deliberately not gated on the feature: always compiled, so the Phase 4
/// wrapper accessor can be built on it without a `#[cfg]` that would make
/// "you installed the wrong artifact" indistinguishable from "you are running a
/// driver too old to have the accessor". `#[allow(dead_code)]` covers the
/// standard build, where both callers (the mismatch log in
/// `ensure_crypto_provider`, the gate in `require_fips_provider`) are behind
/// `#[cfg(feature = "fips-tls")]`.
///
/// Crate-private until Phase 4 exports it deliberately. Publishing it now would
/// put a function named for FIPS mode on the public surface while it can only
/// speak for the TLS backend — the same overclaim the `fips-tls` feature name
/// exists to avoid. The eventual public accessor should be named after the TLS
/// provider, and its wording is compliance's to own.
#[allow(dead_code)]
pub(crate) fn fips_mode_active() -> bool {
    rustls::crypto::CryptoProvider::get_default().is_some_and(|p| p.fips())
}

/// Fails closed in `fips-tls` builds when the provider that actually won the
/// process-global slot is not in FIPS mode.
///
/// `ensure_crypto_provider` only logs the mismatch, because it cannot fail: it
/// runs from constructors and from paths with no error channel, and it sits
/// beneath an FFI boundary where unwinding is undefined behaviour. Logging
/// alone would mean a `fips-tls` build silently serving traffic on a non-approved
/// module, so every TLS client construction routes through here instead, where
/// there *is* an error channel and the failure propagates as a `TlsError` out
/// through the normal FFI error path.
///
/// Compiles to `Ok(())` without the feature.
///
/// Scope: this gates the clients that carry connection traffic (everything
/// built through `build_tls_client_and_rustls_config` / `configure_tls_builder`).
/// Auxiliary raw clients -- telemetry, CRL fetch, IMDS -- have no error channel
/// to fail into and still rely on the logged mismatch.
pub(crate) fn require_fips_provider() -> Result<(), error::TlsError> {
    #[cfg(feature = "fips-tls")]
    if !fips_mode_active() {
        return Err(error::FipsModeUnavailableSnafu.build());
    }
    Ok(())
}

/// Proves the `fips-tls` feature actually puts the linked crypto module into FIPS
/// mode, rather than merely pulling `aws-lc-fips-sys` into the link.
///
/// This is the check that distinguishes "we depend on a FIPS-capable crate"
/// from "we are running approved algorithms in an approved mode" -- the former
/// is a build-graph property, the latter is what an auditor asks about.
#[cfg(all(test, feature = "fips-tls"))]
mod fips_tests {
    /// The aws-lc module reports FIPS mode at runtime. Fails if the build
    /// silently linked non-FIPS aws-lc-sys instead of aws-lc-fips-sys.
    #[test]
    fn aws_lc_reports_fips_mode() {
        assert!(
            aws_lc_rs::try_fips_mode().is_ok(),
            "aws-lc is linked but not in FIPS mode"
        );
    }

    /// The provider `ensure_crypto_provider` actually installs is FIPS-approved
    /// end to end: its RNG, key provider, and every offered cipher suite.
    ///
    /// Asserts against the *process default* rather than a freshly constructed
    /// provider, because that is what `tls::client` resolves: it builds configs
    /// with `ClientConfig::builder()`, which reads the installed default. A
    /// fresh `aws_lc_rs::default_provider()` would only restate what
    /// `aws_lc_reports_fips_mode` already covers.
    #[test]
    fn installed_rustls_provider_is_fips() {
        super::ensure_crypto_provider();
        let provider = rustls::crypto::CryptoProvider::get_default()
            .expect("ensure_crypto_provider must leave a process-default provider installed");
        assert!(
            provider.fips(),
            "installed rustls provider that tls::client will resolve is not FIPS"
        );
        assert!(
            provider.cipher_suites.iter().all(|cs| cs.fips()),
            "installed provider offers a non-FIPS cipher suite"
        );
    }

    /// A `ClientConfig` built the way `tls::client` builds it stays FIPS after
    /// the builder chain -- `ClientConfig::fips()` is the assertion rustls
    /// documents for this, and it is what should eventually gate startup.
    ///
    /// Uses `ClientConfig::builder()`, the same entry point `tls::client` uses,
    /// so this exercises the installed process default instead of a provider
    /// handed in by the test.
    #[test]
    fn client_config_is_fips() {
        super::ensure_crypto_provider();
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(rustls::RootCertStore::empty())
            .with_no_client_auth();
        assert!(config.fips(), "ClientConfig is not in FIPS mode");
    }
}
