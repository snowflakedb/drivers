use snafu::{Location, Snafu};

#[derive(Snafu, Debug)]
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
}
