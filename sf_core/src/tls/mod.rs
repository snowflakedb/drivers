pub mod aws_http_client;
pub mod client;
pub mod config;
pub mod crl_verifier;
pub mod crypto_module;
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

/// Guarantees a process-wide rustls default provider is in place, installing
/// aws-lc-rs if nothing has claimed the slot yet. Runs its work exactly once.
///
/// Every `reqwest::Client` the driver builds resolves its crypto backend
/// through `CryptoProvider::get_default()`. Installing from one place, rather
/// than ad hoc next to each client, is what stops a client built early in the
/// process (telemetry, CRL prefetch, cloud transfers) from silently resolving
/// to a different backend than the one carrying the session's own traffic --
/// under `--features fips-tls` that difference is the whole compliance claim.
///
/// Installation is best-effort by design: `install_default` returns `Err` when
/// an embedding application has already installed its own provider, and
/// stomping on that would be worse than honouring it. In `fips-tls` builds the
/// resulting provider is checked and a mismatch is logged loudly rather than
/// panicking, because this runs beneath an FFI boundary where unwinding is
/// undefined behaviour.
pub(crate) fn ensure_crypto_provider() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        // `install_default` has exactly one failure mode -- a provider is
        // already installed -- and the error payload is that provider, so log
        // what actually won rather than just that we lost the race.
        if let Err(existing) = rustls::crypto::aws_lc_rs::default_provider().install_default() {
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
    });
}

/// Whether the crypto provider actually in force is operating in FIPS mode.
///
/// Reports on the provider that carries the driver's traffic -- the compiled
/// module's own -- rather than on build flags. A build without `fips-tls`
/// links non-FIPS aws-lc-sys and so reports `false`.
///
/// Deliberately not gated on the feature. The wrappers will surface this as a
/// customer-facing accessor (plan Phase 4), and a function that is *absent*
/// from standard builds would make "you installed the wrong artifact" look
/// identical to "you are running a driver too old to have the accessor at
/// all". Always present, answering `false`, keeps those two distinguishable.
///
/// `pub` rather than `pub(crate)` for the same reason: its in-crate callers
/// sit under `#[cfg(feature = "fips-tls")]`, so a crate-private version is
/// dead code in every standard build. The only ways to keep it crate-private
/// are an `#[allow(dead_code)]` or the feature gate this doc block just
/// explained we do not want -- both of which hide the accessor Phase 4 is
/// going to export anyway.
///
/// Phase 3 note: this used to read `CryptoProvider::get_default()`, so that a
/// standard build into which an embedding application had installed a FIPS
/// provider reported `true`. That was the honest answer while the driver
/// *used* whatever won the global slot. The configs this crate builds itself
/// now come from the linked crypto module (`tls::crypto_module`), so reading
/// the global would report on a module that verifies none of the chains we
/// validate. The intent is unchanged: answer for whatever is actually doing
/// the work.
///
/// That makes this a *necessary but not sufficient* FIPS answer, which is why
/// it is not the whole gate. Only the CRL-enabled paths hand their config to
/// reqwest; elsewhere reqwest still resolves the global provider for the
/// handshake itself, so `require_fips_provider` checks that too. See its docs
/// for why the two can diverge and why closing the gap properly is a separate
/// change.
pub fn fips_mode_active() -> bool {
    crypto_module::CryptoModule::get().fips()
}

/// Fails closed in `fips-tls` builds unless *both* the linked crypto module and
/// the process-global provider are in FIPS mode.
///
/// `ensure_crypto_provider` only logs a mismatch, because it cannot fail: it
/// runs from constructors and from paths with no error channel, and it sits
/// beneath an FFI boundary where unwinding is undefined behaviour. Logging
/// alone would mean a `fips-tls` build silently serving traffic on a
/// non-approved module, so every TLS client construction routes through here
/// instead, where there *is* an error channel and the failure propagates as a
/// `TlsError` out through the normal FFI error path.
///
/// Compiles to `Ok(())` without the feature.
///
/// # Why the global provider is checked too
///
/// Checking the linked module alone was not enough, and the gap was the
/// default configuration rather than an edge case. Only the CRL-enabled paths
/// hand reqwest a `ClientConfig` built from the module
/// (`use_preconfigured_tls`); on every other path -- including the
/// CRL-disabled one, which is the default (`CertRevocationCheckMode::Disabled`)
/// -- reqwest builds its own config and resolves crypto through
/// `CryptoProvider::get_default()`. `ensure_crypto_provider` installs the
/// module when the slot is free but deliberately honours a provider an
/// embedding application installed first, so those two can differ. A
/// module-only check would then pass while reporting on a module carrying none
/// of that client's traffic -- exactly the "gate says FIPS, handshake runs on
/// ring" case this must refuse.
///
/// The refusal is process-wide rather than per-path on purpose. The auxiliary
/// raw clients (telemetry, CRL fetch, IMDS) have no error channel to fail into
/// and always resolve the global provider, so a non-FIPS global in a
/// `fips-tls` build is a misconfiguration that no individual client can be
/// exempted from -- including the module-backed CRL path, which would
/// otherwise look safe in isolation.
///
/// The stronger fix is to give every traffic-carrying client a module-backed
/// config and stop consulting the global slot at all. That is deliberately not
/// done here: reqwest is configured with *both* `rustls-tls-native-roots-no-provider`
/// and `rustls-tls-webpki-roots-no-provider`, so its default trust set is
/// native roots union webpki roots, while the rustls configs this module builds
/// use native roots only. Switching the default path to
/// `use_preconfigured_tls` would therefore silently narrow the trust anchors of
/// every connection, which is a behavioural change that needs its own change
/// and its own testing rather than riding along with a crypto port.
pub(crate) fn require_fips_provider() -> Result<(), error::TlsError> {
    #[cfg(feature = "fips-tls")]
    {
        if !fips_mode_active() {
            return Err(error::FipsModeUnavailableSnafu.build());
        }
        // `None` fails closed as well: `ensure_crypto_provider` runs before
        // every call site, so an empty slot means installation itself failed,
        // and reqwest would go on to panic with "No provider set" anyway.
        if !rustls::crypto::CryptoProvider::get_default().is_some_and(|provider| provider.fips()) {
            return Err(error::FipsGlobalProviderUnavailableSnafu.build());
        }
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
