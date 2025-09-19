use super::config::{CertRevocationCheckMode, CrlConfig};

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
        match self.config.check_mode {
            CertRevocationCheckMode::Disabled => Err(CrlError::Disabled),
            _ => Ok(RevocationOutcome::Unknown),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_returns_error() {
        let v = CrlValidator::new(CrlConfig {
            check_mode: CertRevocationCheckMode::Disabled,
            ..Default::default()
        });
        let res = v.check_certificate_revocation(&[], None);
        assert!(matches!(res, Err(CrlError::Disabled)));
    }

    #[test]
    fn enabled_returns_unknown() {
        let v = CrlValidator::new(CrlConfig {
            check_mode: CertRevocationCheckMode::Enabled,
            ..Default::default()
        });
        let res = v.check_certificate_revocation(&[], None).unwrap();
        assert_eq!(res, RevocationOutcome::Unknown);
    }
}
