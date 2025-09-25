use snafu::{Location, Snafu};

#[derive(Snafu, Debug)]
#[snafu(visibility(pub))]
pub enum CrlError {
    #[snafu(display("Failed to download CRL from URL: {url}"))]
    CrlDownload {
        url: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse CRL data"))]
    CrlParsing {
        source: x509_parser::nom::Err<x509_parser::error::X509Error>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid CRL signature"))]
    InvalidCrlSignature {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("CRL issuer does not match certificate issuer"))]
    CrlIssuerMismatch {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read CRL from disk cache"))]
    DiskCacheRead {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to write CRL to disk cache"))]
    DiskCacheWrite {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to create cache directory"))]
    CacheDirectoryCreation {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("CRL has expired"))]
    CrlExpired {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("All certificate chains are revoked"))]
    AllChainsRevoked {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Certificate has no CRL distribution points"))]
    NoCrlDistributionPoints {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse URL: {url}"))]
    InvalidUrl {
        url: String,
        source: url::ParseError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("HTTP timeout while fetching CRL"))]
    HttpTimeout {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Mutex poisoned: {message}"))]
    MutexPoisoned {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to build HTTP client for CRL requests"))]
    HttpClientBuild {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse CRL using x509-cert"))]
    CrlListParse {
        source: x509_cert::der::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to encode CRL TBS to DER"))]
    CrlToDer {
        source: x509_cert::der::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse certificate using x509-cert"))]
    CertificateParse {
        source: x509_cert::der::Error,
        #[snafu(implicit)]
        location: Location,
    },
}
