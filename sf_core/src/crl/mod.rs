//! Certificate Revocation List (CRL) validation module.
//!
//! This module provides CRL validation functionality for TLS connections.
//! It is only available for native builds (requires x509-parser, rustls, tokio).

#[cfg(feature = "native")]
pub mod cache;
#[cfg(feature = "native")]
pub mod certificate_parser;
pub mod config;
#[cfg(feature = "native")]
pub mod error;
#[cfg(feature = "native")]
pub mod validator;
#[cfg(feature = "native")]
pub mod worker;

#[cfg(feature = "native")]
mod disk_tests;
#[cfg(feature = "native")]
mod integration_test;

#[cfg(feature = "native")]
pub use cache::{CachedCrl, CrlCache};
#[cfg(feature = "native")]
pub use certificate_parser::{
    check_certificate_in_crl, extract_crl_distribution_points, get_certificate_serial_number,
    is_short_lived_certificate,
};
pub use config::{CertRevocationCheckMode, CrlConfig};
#[cfg(feature = "native")]
pub use error::CrlError;
#[cfg(feature = "native")]
pub use validator::CrlValidator;
#[cfg(feature = "native")]
pub use worker::CrlWorker;
