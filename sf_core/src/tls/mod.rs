pub mod cert_extractor;
pub mod client;
pub mod config;
pub mod crl_verifier;
pub mod error;
pub mod x509_utils;

// OCSP scaffold removed per design decision

pub mod revocation {
    use chrono::{DateTime, Utc};
    use snafu::{Location, Snafu};

    #[derive(Debug, Clone, PartialEq)]
    pub enum RevocationOutcome {
        NotRevoked,
        Revoked {
            reason: Option<String>,
            revocation_time: Option<DateTime<Utc>>,
        },
        NotDetermined,
    }

    #[derive(Debug, Snafu)]
    pub enum RevocationError {
        #[snafu(display("CRL error"))]
        Crl {
            source: crate::crl::error::CrlError,
            #[snafu(implicit)]
            location: Location,
        },
        #[snafu(display("OCSP error"))]
        Ocsp {
            source: Box<dyn std::error::Error + Send + Sync>,
            #[snafu(implicit)]
            location: Location,
        },
        #[snafu(display("Unsupported algorithm: {alg}"))]
        UnsupportedAlgorithm {
            alg: String,
            #[snafu(implicit)]
            location: Location,
        },
        #[snafu(display("Policy violation: {message}"))]
        Policy {
            message: String,
            #[snafu(implicit)]
            location: Location,
        },
        #[snafu(display("Internal error: {message}"))]
        Internal {
            message: String,
            #[snafu(implicit)]
            location: Location,
        },
    }
}

pub use cert_extractor::{CertificateInfo, TlsCertificateExtractor};
pub use client::{create_root_store_from_pem, create_tls_client_with_config};
pub use config::TlsConfig;
pub use crl_verifier::CrlServerCertVerifier;
pub use error::TlsError;
