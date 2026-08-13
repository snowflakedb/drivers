use crate::logging::url_for_log;
use snafu::{Location, Snafu};
use std::fmt;

/// A proxy URL with any embedded credentials stripped via [`url_for_log`].
///
/// The only public constructor is [`RedactedUrl::new`], which performs the
/// redaction — so, unlike a plain `String` field, the *type* guarantees a
/// credentialed proxy URL can never reach [`TlsError::ProxyBuild`]'s
/// `Debug`/`ErrorTrace` output: there is no way to populate this field with
/// an unredacted string without calling `url_for_log` first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedUrl(String);

impl RedactedUrl {
    /// Redacts `url`'s credentials (if any) before storing it.
    pub fn new(url: &str) -> Self {
        Self(url_for_log(url))
    }
}

impl fmt::Display for RedactedUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(visibility(pub))]
pub enum TlsError {
    #[snafu(display("Failed to build HTTP client"))]
    ClientBuild {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to initialize CRL validator"))]
    CrlInit {
        source: crate::crl::error::CrlError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to install crypto provider"))]
    CryptoProvider {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to build WebPki verifier"))]
    VerifierBuild {
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to parse PEM root certificates"))]
    PemParse {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to add certificate to root store"))]
    RootStoreAdd {
        source: rustls::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to build proxy from {redacted_url}"))]
    ProxyBuild {
        redacted_url: RedactedUrl,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
}
