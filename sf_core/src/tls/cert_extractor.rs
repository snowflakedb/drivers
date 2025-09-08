// Certificate chain extraction from TLS connections
use crate::crl::error::CrlError;
use crate::crl::validator::CrlValidator;
use std::sync::Arc;

/// Extract certificate chains from TLS connection and validate with CRL
pub struct TlsCertificateExtractor {
    crl_validator: Arc<CrlValidator>,
}

impl TlsCertificateExtractor {
    pub fn new(crl_validator: Arc<CrlValidator>) -> Self {
        Self { crl_validator }
    }

    /// Validate certificate chains extracted from TLS connection
    /// This would be called after a TLS connection is established
    pub async fn validate_connection_certificates(
        &self,
        peer_certificates: &[Vec<u8>], // DER-encoded certificates from TLS peer
    ) -> Result<(), CrlError> {
        if peer_certificates.is_empty() {
            tracing::warn!("No peer certificates provided for validation");
            return Ok(());
        }

        tracing::info!(
            "Validating {} peer certificates from TLS connection",
            peer_certificates.len()
        );

        // Convert to the format expected by CRL validator
        let cert_chains = vec![peer_certificates.to_vec()];

        // Perform CRL validation
        self.crl_validator
            .validate_certificate_chains(&cert_chains)
            .await
    }

    /// Extract certificate information for debugging
    pub fn extract_certificate_info(&self, cert_der: &[u8]) -> Result<CertificateInfo, CrlError> {
        use x509_parser::prelude::FromDer;

        let (_, cert) =
            x509_parser::certificate::X509Certificate::from_der(cert_der).map_err(|e| {
                CrlError::CrlParsing {
                    source: e.into(),
                    location: snafu::Location::new(file!(), line!(), 0),
                }
            })?;

        let serial_number = cert.serial.to_bytes_be();
        let subject = format!("{:?}", cert.subject);
        let issuer = format!("{:?}", cert.issuer);
        let not_before = cert.validity.not_before.to_string();
        let not_after = cert.validity.not_after.to_string();

        Ok(CertificateInfo {
            serial_number: hex::encode(&serial_number),
            subject,
            issuer,
            not_before,
            not_after,
        })
    }
}

/// Certificate information extracted from X.509 certificate
#[derive(Debug, Clone)]
pub struct CertificateInfo {
    pub serial_number: String,
    pub subject: String,
    pub issuer: String,
    pub not_before: String,
    pub not_after: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crl::config::{CertRevocationCheckMode, CrlConfig};

    #[tokio::test]
    async fn test_certificate_extractor_creation() {
        let config = CrlConfig {
            check_mode: CertRevocationCheckMode::Advisory,
            ..Default::default()
        };

        let validator = CrlValidator::new(config).unwrap();
        let extractor = TlsCertificateExtractor::new(Arc::new(validator));

        // Test with empty certificate list
        let result = extractor.validate_connection_certificates(&[]).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_certificate_info_extraction_invalid() {
        let config = CrlConfig::default();
        let validator = CrlValidator::new(config).unwrap();
        let extractor = TlsCertificateExtractor::new(Arc::new(validator));

        let invalid_cert = vec![0x30, 0x82, 0x01, 0x00];
        let result = extractor.extract_certificate_info(&invalid_cert);
        assert!(result.is_err());
    }
}
