#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevocationOutcome {
    NotRevoked,
    Revoked {
        reason: Option<String>,
        revocation_time: Option<String>,
    },
    NotDetermined,
}

use snafu::{Location, Snafu};

#[derive(Snafu, Debug)]
#[snafu(visibility(pub))]
pub enum RevocationError {
    #[snafu(display("CRL error: {message}"))]
    Crl {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
}
