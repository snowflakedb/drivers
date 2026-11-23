//! The driver's cryptographic module: aws-lc, in FIPS mode or not.
//!
//! There is one crypto backend, always aws-lc, and one feature flag deciding
//! which build of it gets linked -- `fips` selects aws-lc-fips-sys, its
//! absence selects aws-lc-sys. Nothing here is swappable at runtime and there
//! is no second backend to choose between, so this type is a single accessor
//! rather than an abstraction over alternatives.
//!
//! Today it owns the TLS provider and the signature algorithms used for
//! certificate-chain and CRL verification. As the remaining OpenSSL call sites
//! (JWT signing, private-key parsing, stage file encryption, DPoP) move onto
//! aws-lc, they come through here too, so that one accessor answers "which
//! module performed this operation" for every operation.
//!
//! # Why this owns a provider instead of reading the installed one
//!
//! `ensure_crypto_provider` installs a process-global default and deliberately
//! yields to an embedding application that got there first. That is the right
//! behaviour for a *global* slot, but it means the module carrying our traffic
//! is decided by a startup race we do not control -- and under `fips` that
//! race is the entire compliance claim.
//!
//! So this holds its own [`CryptoProvider`] and hands it explicitly to
//! `ClientConfig::builder_with_provider` and
//! `WebPkiServerVerifier::builder_with_provider`. Connection, storage and HTTPS
//! CRL clients use this explicit provider, and FIPS builds reject disabled
//! certificate verification. The global install remains for standard-build
//! insecure clients, plain-HTTP clients and fail-closed platform-detection
//! probes; it no longer selects the CRL downloader's TLS provider.
//!
//! # Why one process-wide instance
//!
//! `aws_lc_rs::default_provider()` allocates a fresh `CryptoProvider` on every
//! call, and the pre-existing call sites did exactly that -- three separate
//! providers could back one connection's handshake, chain verification, and
//! CRL check. They were identical in practice, so nothing misbehaved, but
//! "identical in practice" is not the property an auditor is asking about.
//! A single [`LazyLock`] instance makes them the same object by construction.

use std::sync::{Arc, LazyLock};

use rustls::NamedGroup;
use rustls::crypto::{CryptoProvider, WebPkiSupportedAlgorithms};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyShareOrder {
    ClassicalFirst,
    PostQuantumFirst,
}

impl KeyShareOrder {
    pub(crate) fn from_prefer_post_quantum(prefer_post_quantum: bool) -> Self {
        if prefer_post_quantum {
            Self::PostQuantumFirst
        } else {
            Self::ClassicalFirst
        }
    }
}

pub(crate) fn is_post_quantum(group: NamedGroup) -> bool {
    post_quantum_classification(group) == Some(true)
}

/// `NamedGroup` is non-exhaustive, so groups added by a rustls upgrade land in
/// `None`; the provider test rejects any offered group that is unclassified.
fn post_quantum_classification(group: NamedGroup) -> Option<bool> {
    match group {
        NamedGroup::X25519MLKEM768
        | NamedGroup::secp256r1MLKEM768
        | NamedGroup::MLKEM512
        | NamedGroup::MLKEM768
        | NamedGroup::MLKEM1024 => Some(true),
        NamedGroup::secp256r1
        | NamedGroup::secp384r1
        | NamedGroup::secp521r1
        | NamedGroup::X25519
        | NamedGroup::X448
        | NamedGroup::FFDHE2048
        | NamedGroup::FFDHE3072
        | NamedGroup::FFDHE4096
        | NamedGroup::FFDHE6144
        | NamedGroup::FFDHE8192 => Some(false),
        _ => None,
    }
}

/// rustls sends the key share of the first group in the ClientHello, and
/// servers that honor the client's preference pick the first group they share.
fn ordered(mut provider: CryptoProvider, order: KeyShareOrder) -> CryptoProvider {
    let post_quantum_first = order == KeyShareOrder::PostQuantumFirst;
    provider
        .kx_groups
        .sort_by_key(|group| is_post_quantum(group.name()) != post_quantum_first);
    provider
}

/// The aws-lc module this build linked.
///
/// Obtain the process-wide instance with [`CryptoModule::get`]; it is
/// constructed once and shared. There is deliberately no constructor taking an
/// arbitrary provider: which module is in use is a compile-time fact, and a
/// runtime setter would reintroduce the ambiguity this type exists to remove.
#[derive(Debug)]
pub(crate) struct CryptoModule {
    provider: Arc<CryptoProvider>,
    post_quantum_first: Arc<CryptoProvider>,
}

static MODULE: LazyLock<CryptoModule> = LazyLock::new(|| {
    let base = rustls::crypto::aws_lc_rs::default_provider();
    let module = CryptoModule {
        post_quantum_first: Arc::new(ordered(base.clone(), KeyShareOrder::PostQuantumFirst)),
        provider: Arc::new(ordered(base, KeyShareOrder::ClassicalFirst)),
    };
    #[cfg(feature = "fips")]
    let version = tracing::field::debug(module.version());
    #[cfg(not(feature = "fips"))]
    let version = tracing::field::Empty;
    // Module identity, recorded once. A `fips` build whose module did not
    // enter FIPS mode is the case worth seeing in a log: the two fields
    // disagree, and the build is not making the claim its name implies.
    tracing::debug!(
        linked = module.name(),
        version,
        provider_is_fips = module.provider_is_fips(),
        "crypto module initialised"
    );
    module
});

impl CryptoModule {
    /// The process-wide module for this build.
    pub(crate) fn get() -> &'static Self {
        &MODULE
    }

    /// The provider to hand to `builder_with_provider` entry points. It offers
    /// a classical key share first.
    ///
    /// Cloning is an `Arc` bump; rustls takes the provider by value.
    pub(crate) fn provider(&self) -> Arc<CryptoProvider> {
        Arc::clone(&self.provider)
    }

    pub(crate) fn provider_for(&self, order: KeyShareOrder) -> Arc<CryptoProvider> {
        match order {
            KeyShareOrder::ClassicalFirst => self.provider(),
            KeyShareOrder::PostQuantumFirst => Arc::clone(&self.post_quantum_first),
        }
    }

    /// Signature algorithms for certificate-chain and CRL signature
    /// verification.
    ///
    /// Sourced from this module's provider rather than a fresh
    /// `default_provider()`, so a verifier cannot end up accepting an
    /// algorithm set the negotiating provider would not offer.
    pub(crate) fn signature_verification_algorithms(&self) -> WebPkiSupportedAlgorithms {
        self.provider.signature_verification_algorithms
    }

    /// Whether this module's rustls provider is FIPS-approved.
    ///
    /// Scoped to the TLS provider, not to the artifact. rustls computes it as
    /// a conjunction over *this provider's own* components -- every offered
    /// cipher suite and key-exchange group, the signature-verification
    /// algorithms, the RNG, and the key provider. For the aws-lc-rs backend
    /// each of those bottoms out in `aws_lc_rs::try_fips_mode()`, so a
    /// standard build (non-FIPS aws-lc-sys) answers `false`.
    ///
    /// That per-component conjunction is the reason this is not a synonym for
    /// "the module is in approved mode": a provider carrying one non-approved
    /// suite answers `false` with the C module still in FIPS mode. The
    /// artifact-level question is `try_fips_mode()` itself, asserted by the
    /// `aws_lc_reports_fips_mode` lib test.
    pub(crate) fn provider_is_fips(&self) -> bool {
        self.provider.fips()
    }

    #[cfg(feature = "fips")]
    pub(crate) fn version(&self) -> &'static std::ffi::CStr {
        // SAFETY: AWS-LC returns a static, non-null, NUL-terminated version string.
        unsafe { std::ffi::CStr::from_ptr(aws_lc_fips_sys::awslc_version_string()) }
    }

    /// Which aws-lc build was linked, for diagnostics and wrapper-facing
    /// status.
    ///
    /// Tracks the Cargo feature rather than the runtime state: it answers
    /// "which module was linked", while
    /// [`provider_is_fips`](Self::provider_is_fips) answers "is that module's
    /// provider FIPS-approved". Both are needed -- a `fips` build whose
    /// module failed to enter FIPS mode must not look like a non-FIPS build.
    pub(crate) fn name(&self) -> &'static str {
        if cfg!(feature = "fips") {
            "aws-lc-fips"
        } else {
            "aws-lc"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_offered_group_is_classified() {
        let unclassified: Vec<_> = CryptoModule::get()
            .provider()
            .kx_groups
            .iter()
            .map(|group| group.name())
            .filter(|name| post_quantum_classification(*name).is_none())
            .collect();
        assert!(
            unclassified.is_empty(),
            "unclassified key exchange groups: {unclassified:?}"
        );
    }

    /// The instance is shared, not rebuilt per call. This is the property that
    /// makes "the handshake and the CRL check used the same module" true by
    /// construction rather than by coincidence.
    #[test]
    fn module_is_a_single_shared_instance() {
        let first = CryptoModule::get();
        let second = CryptoModule::get();
        assert!(
            std::ptr::eq(first, second),
            "crypto module should be a single process-wide instance"
        );
        assert!(
            Arc::ptr_eq(&first.provider(), &second.provider()),
            "repeated provider() calls should hand out the same provider"
        );
    }

    /// The algorithms a verifier will use come from the same provider that
    /// negotiates the handshake.
    #[test]
    fn signature_algorithms_come_from_the_module_provider() {
        let module = CryptoModule::get();
        let from_module = module.signature_verification_algorithms();
        let from_provider = module.provider().signature_verification_algorithms;
        assert_eq!(
            format!("{from_module:?}"),
            format!("{from_provider:?}"),
            "verifier algorithms should be the negotiating provider's"
        );
    }

    /// `name()` reports which aws-lc build was linked, from the compiled
    /// feature rather than runtime state.
    #[test]
    fn name_tracks_the_linked_module() {
        let expected = if cfg!(feature = "fips") {
            "aws-lc-fips"
        } else {
            "aws-lc"
        };
        assert_eq!(CryptoModule::get().name(), expected);
    }

    /// A non-FIPS build must not claim FIPS. Gated to non-FIPS builds only:
    /// under `fips` the corresponding assertion lives in
    /// `tls::fips_tests`, which asserts the positive case end to end.
    #[cfg(not(feature = "fips"))]
    #[test]
    fn non_fips_build_does_not_report_fips() {
        assert!(
            !CryptoModule::get().provider_is_fips(),
            "a build without `fips` links aws-lc-sys and must report false"
        );
    }
}
