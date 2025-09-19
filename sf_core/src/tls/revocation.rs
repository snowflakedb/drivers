#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevocationOutcome {
    NotRevoked,
    Revoked {
        reason: Option<String>,
        revocation_time: Option<String>,
    },
    NotDetermined,
}

#[derive(thiserror::Error, Debug)]
pub enum RevocationError {
    #[error("CRL error: {0}")]
    Crl(String),
}
