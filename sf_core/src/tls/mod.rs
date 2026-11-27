pub mod aws_http_client;
mod cipher_policy;
pub mod client;
pub(crate) mod client_slot;
pub(crate) mod client_spec;
pub mod config;
pub mod crl_verifier;
pub mod crypto_module;
pub mod error;
pub mod revocation;
#[cfg(test)]
pub mod test_helpers;
pub mod x509_utils;

pub(crate) use client::RootStoreSnapshot;
pub use client::{
    RootStoreProvider, create_tls_client_with_config, create_tls_client_with_proxy,
    create_tls_client_with_proxy_and_timeouts,
};
pub use config::{ProxyConfig, TlsConfig};
pub(crate) use crl_verifier::CrlServerCertVerifier;
pub use x509_utils::{crl_times, extract_skid, subject_der_hash, verify_crl_signature};

/// Guarantees a rustls default provider is in place, installing aws-lc-rs if
/// nothing has claimed the slot yet.
///
/// Every HTTPS client the driver builds carries an explicit linked-module
/// rustls config. Plain-HTTP clients and the standard-build
/// `verify_certificates=false` path still need this default installed to
/// prevent reqwest's no-provider panic; FIPS builds reject that opt-out.
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
/// The `fips` check below asks about the *linked module*, not whichever
/// provider won that race: it catches a build that claims `fips` without
/// having linked a FIPS-capable aws-lc. Whether the process-global slot is
/// also FIPS is a separate question, and one with somewhere to fail into, so
/// [`require_fips_provider`] covers it. Here a mismatch is only logged --
/// loudly -- because this runs beneath an FFI boundary where unwinding is
/// undefined behaviour.
pub(crate) fn ensure_crypto_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none()
        && let Err(existing) = (*crypto_module::CryptoModule::get().provider())
            .clone()
            .install_default()
    {
        tracing::debug!(
            existing_provider_is_fips = existing.fips(),
            "rustls crypto provider already installed; leaving it in place"
        );
    }

    #[cfg(feature = "fips")]
    if !tls_provider_is_fips() {
        tracing::error!(
            "driver was built with the `fips` feature but the linked module's \
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
/// no-connection status report in any wrapper, use [`fips_status`].
///
/// This reports the linked module rather than the process default: connection,
/// storage and HTTPS CRL clients use its provider for their handshakes, whereas
/// the global provider may belong to an embedding application. FIPS builds
/// reject disabled certificate verification; standard builds retain reqwest's
/// insecure global-provider path.
///
/// Necessary but not sufficient for a driver-wide compliance claim:
/// [`require_fips_provider`] also checks the global provider used by platform
/// probes in FIPS builds. Neither check attests the entire driver or deployment.
pub fn tls_provider_is_fips() -> bool {
    crypto_module::CryptoModule::get().provider_is_fips()
}

/// Whether this process is the FIPS driver.
///
/// True when this binary was built with `fips` and the linked TLS provider
/// reports FIPS mode. A standard build returns false.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FipsStatus {
    pub is_fips: bool,
}

/// Reports whether this process is the FIPS driver, without opening a
/// connection or making a network call.
pub fn fips_status() -> FipsStatus {
    FipsStatus {
        is_fips: cfg!(feature = "fips") && tls_provider_is_fips(),
    }
}

/// Fails closed in `fips` builds unless *both* the linked crypto module and
/// the process-global provider are in FIPS mode.
///
/// `ensure_crypto_provider` only logs a mismatch because it has no error
/// channel and can run beneath an FFI boundary where unwinding is undefined.
/// Connection/storage builders, platform-detection probes and WIF attestation
/// use this gate.
///
/// Compiles to `Ok(())` without the feature.
///
/// # Why the global provider is checked too
///
/// All HTTPS clients use preconfigured rustls configs with the linked module's
/// provider, so their handshakes do not depend on who installed the process
/// default, and the insecure reqwest path is compiled only in standard builds.
/// The global check keeps the existing fail-closed startup policy: a `fips`
/// process whose default provider is non-FIPS is misconfigured.
pub(crate) fn require_fips_provider() -> Result<(), error::TlsError> {
    #[cfg(feature = "fips")]
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

/// Proves the `fips` feature actually puts the linked crypto module into FIPS
/// mode, rather than merely pulling `aws-lc-fips-sys` into the link.
///
/// This is the check that distinguishes "we depend on a FIPS-capable crate"
/// from "we are running approved algorithms in an approved mode" -- the former
/// is a build-graph property, the latter is what an auditor asks about.
#[cfg(all(test, feature = "fips"))]
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

    /// `require_fips_provider` expects a FIPS-approved process default.
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

    /// The process-global builder must still use approved algorithms.
    /// Module-backed configs have provider-ownership tests in `tls::client`.
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
        use crate::protobuf::generated::database_driver_v1::DriverGetFipsStatusRequest;

        let status = database_driver_client()
            .driver_get_fips_status_blocking(DriverGetFipsStatusRequest {})
            .expect("no-connection status RPC must succeed");
        assert!(status.is_fips);
    }
}

#[cfg(test)]
mod status_tests {
    #[test]
    fn status_describes_linked_tls_provider_in_both_builds() {
        let status = super::fips_status();
        assert_eq!(status.is_fips, cfg!(feature = "fips"));
    }
}
