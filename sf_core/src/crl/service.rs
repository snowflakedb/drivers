// CRL validation service for application-wide certificate validation
use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::crl::error::CrlError;
use crate::crl::validator_real::RealCrlValidator;
use crate::tls::cert_extractor::{CertificateInfo, TlsCertificateExtractor};
use std::sync::Arc;

/// Application-wide CRL validation service
pub struct CrlValidationService {
    validator: Arc<RealCrlValidator>,
    extractor: TlsCertificateExtractor,
    config: CrlConfig,
}

impl CrlValidationService {
    /// Create a new CRL validation service
    pub fn new(config: CrlConfig) -> Result<Self, CrlError> {
        let validator = Arc::new(RealCrlValidator::new(config.clone())?);
        let extractor = TlsCertificateExtractor::new(validator.clone());

        Ok(Self {
            validator,
            extractor,
            config,
        })
    }

    /// Validate certificate chains (main entry point)
    pub async fn validate_certificate_chains(
        &self,
        cert_chains: &[Vec<Vec<u8>>],
    ) -> Result<(), CrlError> {
        if self.config.check_mode == CertRevocationCheckMode::Disabled {
            tracing::debug!("CRL validation disabled");
            return Ok(());
        }

        tracing::info!(
            "Performing CRL validation for {} certificate chains",
            cert_chains.len()
        );
        self.validator
            .validate_certificate_chains(cert_chains)
            .await
    }

    /// Validate peer certificates from a TLS connection
    pub async fn validate_peer_certificates(
        &self,
        peer_certificates: &[Vec<u8>],
    ) -> Result<(), CrlError> {
        self.extractor
            .validate_connection_certificates(peer_certificates)
            .await
    }

    /// Extract certificate information for debugging/logging
    pub fn get_certificate_info(&self, cert_der: &[u8]) -> Result<CertificateInfo, CrlError> {
        self.extractor.extract_certificate_info(cert_der)
    }

    /// Get the current CRL configuration
    pub fn config(&self) -> &CrlConfig {
        &self.config
    }

    /// Check if CRL validation is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.check_mode != CertRevocationCheckMode::Disabled
    }

    /// Validate a single certificate chain with detailed logging
    pub async fn validate_with_details(
        &self,
        cert_chain: &[Vec<u8>],
        connection_info: &str,
    ) -> Result<(), CrlError> {
        if !self.is_enabled() {
            tracing::debug!(
                "CRL validation disabled for connection: {}",
                connection_info
            );
            return Ok(());
        }

        tracing::info!(
            "Starting CRL validation for connection: {}",
            connection_info
        );

        // Log certificate information
        for (i, cert_der) in cert_chain.iter().enumerate() {
            match self.get_certificate_info(cert_der) {
                Ok(info) => {
                    tracing::debug!(
                        "Certificate {}: Serial={}, Subject={}",
                        i,
                        info.serial_number,
                        info.subject
                    );
                }
                Err(e) => {
                    tracing::warn!("Failed to extract info for certificate {}: {}", i, e);
                }
            }
        }

        // Perform validation
        let cert_chains = vec![cert_chain.to_vec()];
        match self.validate_certificate_chains(&cert_chains).await {
            Ok(()) => {
                tracing::info!("CRL validation passed for connection: {}", connection_info);
                Ok(())
            }
            Err(e) => match self.config.check_mode {
                CertRevocationCheckMode::Enabled => {
                    tracing::error!(
                        "CRL validation failed for connection: {} - {}",
                        connection_info,
                        e
                    );
                    Err(e)
                }
                CertRevocationCheckMode::Advisory => {
                    tracing::warn!(
                        "CRL validation failed for connection: {} - {} (advisory mode, allowing)",
                        connection_info,
                        e
                    );
                    Ok(())
                }
                CertRevocationCheckMode::Disabled => unreachable!(),
            },
        }
    }
}

impl std::fmt::Debug for CrlValidationService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrlValidationService")
            .field("config", &self.config)
            .field("validator", &"<RealCrlValidator>")
            .field("extractor", &"<TlsCertificateExtractor>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_crl_service_creation() {
        let config = CrlConfig::default();
        let service = CrlValidationService::new(config);
        assert!(service.is_ok());

        let service = service.unwrap();
        assert!(!service.is_enabled()); // Default is disabled
    }

    #[tokio::test]
    async fn test_crl_service_disabled_validation() {
        let config = CrlConfig::default(); // Disabled by default
        let service = CrlValidationService::new(config).unwrap();

        let empty_chains: Vec<Vec<Vec<u8>>> = vec![];
        let result = service.validate_certificate_chains(&empty_chains).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_crl_service_enabled_validation() {
        let config = CrlConfig {
            check_mode: CertRevocationCheckMode::Advisory,
            ..Default::default()
        };
        let service = CrlValidationService::new(config).unwrap();
        assert!(service.is_enabled());

        // Test with mock certificate
        let mock_cert = vec![0x30, 0x82, 0x01, 0x00];
        let result = service
            .validate_with_details(&[mock_cert], "test_connection")
            .await;
        // Should pass in advisory mode even with invalid cert
        assert!(result.is_ok());
    }
}
