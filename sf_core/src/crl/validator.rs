use super::config::CrlConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevocationOutcome {
    NotRevoked,
    Revoked,
    Unknown,
}

#[derive(thiserror::Error, Debug)]
pub enum CrlError {
    #[error("CRL validation disabled")]
    Disabled,
}

pub struct CrlValidator {
    pub config: CrlConfig,
}

impl CrlValidator {
    pub fn new(config: CrlConfig) -> Self {
        Self { config }
    }

    pub fn check_certificate_revocation(
        &self,
        _cert_der: &[u8],
        _issuer_der: Option<&[u8]>,
    ) -> Result<RevocationOutcome, CrlError> {
        if !self.config.enabled {
            return Err(CrlError::Disabled);
        }
        Ok(RevocationOutcome::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_returns_error() {
        let v = CrlValidator::new(CrlConfig { enabled: false });
        let res = v.check_certificate_revocation(&[], None);
        assert!(matches!(res, Err(CrlError::Disabled)));
    }

    #[test]
    fn enabled_returns_unknown() {
        let v = CrlValidator::new(CrlConfig { enabled: true });
        let res = v.check_certificate_revocation(&[], None).unwrap();
        assert_eq!(res, RevocationOutcome::Unknown);
    }
}
