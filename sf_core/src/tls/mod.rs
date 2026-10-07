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

/// Guarantees a rustls default provider is in place, installing aws-lc-rs if
/// nothing has claimed the slot yet.
///
/// The verified clients built through `tls::client` carry an explicit
/// linked-module rustls config, but raw auxiliary reqwest clients and the
/// `verify_certificates=false` path still use the process default. Installing
/// it prevents the no-provider panic. Only construction paths that separately
/// call [`require_fips_provider`] can fail closed on a non-FIPS global.
///
/// `install_default` is already one-shot inside rustls: a later call returns
/// `Err` with the provider that won. This function has no crate-local `Once`,
/// so it still installs after the Windows ODBC driver manager unloads and
/// reloads the DLL — rustls's slot is empty again then, and a `Once` in this
/// crate would not run.
///
/// Installation is best-effort by design: `install_default` returns `Err` when
/// an embedding application has already installed its own provider, and
/// stomping on that would be worse than honouring it.
///
/// The `fips-tls` check below asks about the *linked module*, not whichever
/// provider won that race: it catches a build that claims `fips-tls` without
/// having linked a FIPS-capable aws-lc. Whether the process-global slot is
/// also FIPS is a separate question, and one with somewhere to fail into, so
/// [`require_fips_provider`] covers it. Here a mismatch is only logged --
/// loudly -- because this runs beneath an FFI boundary where unwinding is
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
    if !tls_provider_is_fips() {
        tracing::error!(
            "driver was built with the `fips-tls` feature but the linked module's \
             TLS provider is not FIPS-approved; TLS is NOT FIPS compliant"
        );
    }
}

/// Whether the linked module's TLS crypto provider is FIPS-approved.
///
/// Named for the TLS provider because that is its scope. It forwards rustls's
/// `CryptoProvider::fips()`, which is a conjunction over *that provider's own*
/// cipher suites, key-exchange groups, signature algorithms, RNG, and key
/// provider. The driver also does crypto outside TLS -- JWT signing, private
/// key loading, client-side file encryption -- and this answers for none of
/// it.
///
/// The module-mode question is answered by `aws_lc_rs::try_fips_mode()`,
/// not this provider check. Neither establishes validated-module or whole-driver
/// compliance.
///
/// This function remains available in both builds for Rust consumers. For a
/// no-connection status report in any wrapper, use [`tls_status`].
///
/// This reports the linked module rather than the process default: the
/// verified connection/storage clients use its provider for their handshakes,
/// whereas the global provider may belong to an embedding application.
///
/// Necessary but not sufficient for a driver-wide compliance claim: verified
/// connection/storage clients now use this provider explicitly, while auxiliary
/// raw clients and the intentionally unchanged insecure path still resolve the
/// process-global provider. [`require_fips_provider`] checks both providers
/// in FIPS builds; it does not attest the entire driver or deployment.
pub fn tls_provider_is_fips() -> bool {
    crypto_module::CryptoModule::get().provider_is_fips()
}

/// The scope of the no-connection TLS status API. A provider verdict and
/// a Cargo feature are distinct facts; neither certifies the complete driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlsStatus {
    pub tls_provider_is_fips: bool,
    pub fips_tls_build_enabled: bool,
}

/// Reports the linked rustls TLS provider's FIPS verdict and this build's
/// `fips-tls` feature, without initializing a connection or making network calls.
///
/// This does not attest to an installed process-global provider, other driver
/// cryptography, a validated module version, or the complete artifact.
pub fn tls_status() -> TlsStatus {
    TlsStatus {
        tls_provider_is_fips: tls_provider_is_fips(),
        fips_tls_build_enabled: cfg!(feature = "fips-tls"),
    }
}

/// Fails closed in `fips-tls` builds unless *both* the linked crypto module and
/// the process-global provider are in FIPS mode.
///
/// `ensure_crypto_provider` only logs a mismatch because it has no error
/// channel and can run beneath an FFI boundary where unwinding is undefined.
/// The connection/storage builders (and callers explicitly using this gate)
/// can return a `TlsError` instead of silently constructing a client against
/// an unapproved global provider. Raw auxiliary clients without this check
/// are not covered by that guarantee.
///
/// Compiles to `Ok(())` without the feature.
///
/// # Why the global provider is checked too
///
/// Verified CRL-disabled clients now use a preconfigured rustls config with
/// the linked module's provider and reqwest's native ∪ bundled webpki trust
/// roots. Custom roots still replace that union; extra roots extend it. Their
/// crypto provider no longer depends on who installed the process default.
///
/// The `verify_certificates=false` path is deliberately different: reqwest's
/// built-in `NoVerifier` accepts handshake signatures without verification,
/// whereas the diagnostic's `NoVerifyCertVerifier` still checks signatures.
/// Swapping the traffic client to the diagnostic config would change behavior.
/// That path keeps reqwest's verifier and process-global provider.
///
/// Raw auxiliary clients (telemetry, CRL fetch, IMDS) also resolve the global
/// provider. An embedding application may have installed a different provider
/// first. Requiring *both* providers to be FIPS fails closed for builders that
/// call this function, including module-backed and insecure connection/storage
/// clients; it does not make their providers identical. Raw auxiliary
/// constructors that call only `ensure_crypto_provider` retain a potential
/// non-FIPS global-provider path until separately gated or module-backed.
pub(crate) fn require_fips_provider() -> Result<(), error::TlsError> {
    #[cfg(feature = "fips-tls")]
    {
        if !tls_provider_is_fips() {
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
    // Update together with the exact aws-lc-fips-sys pin in sf_core/Cargo.toml.
    const EXPECTED_FIPS_MODULE_VERSION: &str = "AWS-LC FIPS 3.6.0";

    #[test]
    fn linked_module_version_matches_pin() {
        let version = super::crypto_module::CryptoModule::get().version();
        assert_eq!(version.to_str(), Ok(EXPECTED_FIPS_MODULE_VERSION));
    }

    /// The aws-lc module reports FIPS mode at runtime. Fails if the build
    /// silently linked non-FIPS aws-lc-sys instead of aws-lc-fips-sys.
    #[test]
    fn aws_lc_reports_fips_mode() {
        assert!(
            aws_lc_rs::try_fips_mode().is_ok(),
            "aws-lc is linked but not in FIPS mode"
        );
    }

    /// The process default installed by `ensure_crypto_provider` must remain
    /// FIPS-approved for auxiliary raw reqwest clients and the insecure path,
    /// even though verified connection/storage clients now use the module.
    #[test]
    fn installed_rustls_provider_is_fips() {
        super::ensure_crypto_provider();
        let provider = rustls::crypto::CryptoProvider::get_default()
            .expect("ensure_crypto_provider must leave a process-default provider installed");
        assert!(
            provider.fips(),
            "installed process-default rustls provider is not FIPS-approved"
        );
        assert!(
            provider.cipher_suites.iter().all(|cs| cs.fips()),
            "installed provider offers a non-FIPS cipher suite"
        );
    }

    /// The process-global builder must still use approved algorithms where
    /// raw or insecure clients rely on it. Module-backed client configs are
    /// checked separately by the provider-ownership tests in `tls::client`.
    #[test]
    fn client_config_is_fips() {
        super::ensure_crypto_provider();
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(rustls::RootCertStore::empty())
            .with_no_client_auth();
        assert!(config.fips(), "ClientConfig is not FIPS-approved");
    }

    #[cfg(feature = "protobuf")]
    #[test]
    fn public_rpc_reports_fips_provider_without_connection() {
        use crate::protobuf::apis::database_driver_v1::{
            DatabaseDriverClientBlockingExt, database_driver_client,
        };
        use crate::protobuf::generated::database_driver_v1::DriverGetTlsStatusRequest;

        let status = database_driver_client()
            .driver_get_tls_status_blocking(DriverGetTlsStatusRequest {})
            .expect("no-connection status RPC must succeed");
        assert!(status.tls_provider_is_fips);
        assert!(status.fips_tls_build_enabled);
    }
}

#[cfg(test)]
mod status_tests {
    #[test]
    fn status_describes_linked_tls_provider_in_both_builds() {
        let status = super::tls_status();
        assert_eq!(status.fips_tls_build_enabled, cfg!(feature = "fips-tls"));
        assert_eq!(status.tls_provider_is_fips, cfg!(feature = "fips-tls"));
    }
}
