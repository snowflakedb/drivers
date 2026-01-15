//! TLS client and certificate validation module.
//!
//! This module provides TLS client creation and CRL validation functionality.
//! Most functionality is only available for native builds (requires rustls, x509-parser).

#[cfg(feature = "native")]
pub mod client;
pub mod config;
#[cfg(feature = "native")]
pub mod crl_verifier;
#[cfg(feature = "native")]
pub mod error;
#[cfg(feature = "native")]
pub mod revocation;
#[cfg(all(test, feature = "native"))]
pub mod test_helpers;
#[cfg(feature = "native")]
pub mod x509_utils;

#[cfg(feature = "native")]
pub use client::create_tls_client_with_config;
pub use config::TlsConfig;
#[cfg(feature = "native")]
pub use crl_verifier::CrlServerCertVerifier;
#[cfg(feature = "native")]
pub use x509_utils::{crl_times, extract_skid, subject_der_hash, verify_crl_signature};
