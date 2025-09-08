// CRL validator that orchestrates cache-based revocation checks
use crate::crl::cache_simple::CrlCache;
use crate::crl::certificate_parser::is_short_lived_certificate;
use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::crl::error::CrlError;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum CertificateStatus {
    /// Certificate is not revoked
    Unrevoked,
    /// Certificate is revoked
    Revoked,
    /// Error occurred while checking revocation status
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChainValidationResult {
    /// All certificates in the chain are unrevoked
    ChainUnrevoked,
    /// At least one certificate in the chain is revoked
    ChainRevoked,
    /// Error occurred while validating the chain
    ChainError,
}

pub struct CrlValidator {
    config: CrlConfig,
    cache: Arc<CrlCache>,
}

impl std::fmt::Debug for CrlValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrlValidator")
            .field("config", &self.config)
            .field("cache", &"<SimpleCrlCache>")
            .finish()
    }
}

impl CrlValidator {
    pub fn new(config: CrlConfig) -> Result<Self, CrlError> {
        let cache = CrlCache::global(config.clone(), 100).clone();
        Ok(Self { config, cache })
    }
    // Static verification moved to tls::x509_utils::verify_crl_signature_best_effort

    /// Main entry point for certificate revocation validation
    pub async fn validate_certificate_chains(
        &self,
        cert_chains: &[Vec<Vec<u8>>], // DER-encoded certificates
    ) -> Result<(), CrlError> {
        match self.config.check_mode {
            CertRevocationCheckMode::Disabled => {
                tracing::debug!("CRL checking is disabled, skipping validation");
                Ok(())
            }
            CertRevocationCheckMode::Enabled => {
                tracing::debug!(
                    target: "sf_core::crl::validate",
                    "CRL validation enabled - performing certificate revocation checks"
                );
                self.perform_validation(cert_chains, true).await
            }
            CertRevocationCheckMode::Advisory => {
                tracing::debug!(
                    target: "sf_core::crl::validate",
                    "CRL advisory mode - performing certificate revocation checks (fail-open)"
                );
                self.perform_validation(cert_chains, false).await
            }
        }
    }

    /// Perform the actual validation logic
    async fn perform_validation(
        &self,
        cert_chains: &[Vec<Vec<u8>>],
        fail_on_error: bool,
    ) -> Result<(), CrlError> {
        if cert_chains.is_empty() {
            tracing::warn!("No certificate chains provided for validation");
            return Ok(());
        }

        tracing::debug!(
            target: "sf_core::crl::validate",
            "CRL validation starting for {} certificate chains",
            cert_chains.len()
        );

        let mut validation_results = Vec::new();

        // Validate each chain until we find a valid one
        for (chain_idx, cert_chain) in cert_chains.iter().enumerate() {
            tracing::debug!(target: "sf_core::crl::validate", "Validating certificate chain {}", chain_idx);

            let chain_result = self.validate_certificate_chain(cert_chain).await;
            validation_results.push(chain_result.clone());

            // If we found an unrevoked chain, we can stop here
            if chain_result == ChainValidationResult::ChainUnrevoked {
                tracing::debug!("Found certificate chain with all certificates unrevoked");
                return Ok(());
            }
        }

        // Analyze results according to the algorithm
        if validation_results.contains(&ChainValidationResult::ChainUnrevoked) {
            tracing::debug!("Found certificate chain with all certificates unrevoked");
            return Ok(());
        }

        if validation_results
            .iter()
            .all(|r| *r == ChainValidationResult::ChainRevoked)
        {
            tracing::error!("Every verified certificate chain contained revoked certificates");
            if fail_on_error {
                return Err(CrlError::InvalidCrlSignature {
                    location: snafu::Location::new(file!(), line!(), 0),
                });
            } else {
                tracing::warn!("Advisory mode: allowing connection despite revoked certificates");
                return Ok(());
            }
        }

        tracing::debug!(
            "Some certificate chains didn't pass or driver wasn't able to perform the checks"
        );

        if fail_on_error {
            tracing::error!("Enabled mode: failing connection due to validation issues");
            Err(CrlError::InvalidCrlSignature {
                location: snafu::Location::new(file!(), line!(), 0),
            })
        } else {
            tracing::info!("Advisory mode: allowing connection despite validation issues");
            Ok(())
        }
    }

    /// Validate a single certificate chain
    async fn validate_certificate_chain(
        &self,
        cert_chain: &[Vec<u8>], // DER-encoded certificates
    ) -> ChainValidationResult {
        if cert_chain.is_empty() {
            tracing::warn!("Empty certificate chain");
            return ChainValidationResult::ChainError;
        }

        tracing::debug!(
            "REAL validation of chain with {} certificates",
            cert_chain.len()
        );

        let mut chain_result = ChainValidationResult::ChainUnrevoked;

        // Check all certificates in the chain except the root (last certificate)
        for (cert_idx, cert_der) in cert_chain.iter().enumerate() {
            // Skip the root certificate (last in chain)
            if cert_idx == cert_chain.len() - 1 {
                tracing::debug!("Skipping root certificate (index {})", cert_idx);
                continue;
            }

            tracing::debug!(target: "sf_core::crl::validate", "Validating certificate {} in chain", cert_idx);

            // Check if certificate is short-lived
            match is_short_lived_certificate(cert_der) {
                Ok(true) => {
                    tracing::debug!(
                        target: "sf_core::crl::validate",
                        "Certificate {} is short-lived, skipping CRL check",
                        cert_idx
                    );
                    continue;
                }
                Ok(false) => {
                    tracing::debug!(
                        target: "sf_core::crl::validate",
                        "Certificate {} is not short-lived, proceeding with CRL check",
                        cert_idx
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to check if certificate {} is short-lived: {}",
                        cert_idx,
                        e
                    );
                    chain_result = ChainValidationResult::ChainError;
                    continue;
                }
            }

            // Ask cache to check revocation holistically; it will extract URLs and check CRLs
            let issuer_der = cert_chain.get(cert_idx + 1).map(|v| v.as_slice());
            match self.cache.check_revocation(cert_der, issuer_der).await {
                Ok(outcome) => {
                    use crate::tls::revocation::RevocationOutcome;
                    match outcome {
                        RevocationOutcome::Revoked { .. } => {
                            tracing::error!(target: "sf_core::crl::validate", "Certificate {} is REVOKED", cert_idx);
                            chain_result = ChainValidationResult::ChainRevoked;
                            break;
                        }
                        RevocationOutcome::NotRevoked => {
                            tracing::info!(
                                target: "sf_core::crl::validate",
                                "Certificate {} is NOT revoked according to CRL(s)",
                                cert_idx
                            );
                        }
                        RevocationOutcome::NotDetermined => {
                            tracing::warn!(
                                target: "sf_core::crl::validate",
                                "Revocation status not determined for certificate {}",
                                cert_idx
                            );
                            chain_result = ChainValidationResult::ChainError;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        target: "sf_core::crl::validate",
                        "Error during revocation check for certificate {}: {}",
                        cert_idx,
                        e
                    );
                    chain_result = ChainValidationResult::ChainError;
                }
            }
        }

        match chain_result {
            ChainValidationResult::ChainUnrevoked => {
                tracing::info!(target: "sf_core::crl::validate", "Certificate chain validation PASSED - all certificates unrevoked");
            }
            ChainValidationResult::ChainRevoked => {
                tracing::error!(
                    target: "sf_core::crl::validate",
                    "Certificate chain validation FAILED - contains revoked certificates"
                );
            }
            ChainValidationResult::ChainError => {
                tracing::warn!(target: "sf_core::crl::validate", "Certificate chain validation had ERRORS");
            }
        }

        chain_result
    }

    #[cfg(test)]
    pub(crate) async fn fetch_crl_with_cache(&self, url: &str) -> Result<Vec<u8>, CrlError> {
        self.cache.get(url).await
    }

    #[cfg(test)]
    pub(crate) fn write_crl_atomic(&self, path: &Path, data: &[u8]) {
        Self::write_crl_atomic_internal(path, data)
    }

    #[allow(dead_code)]
    fn write_crl_atomic_internal(path: &Path, data: &[u8]) {
        let tmp = path.with_extension("tmp");
        if let Ok(mut f) = fs::File::create(&tmp) {
            let _ = f.write_all(data);
            let _ = f.sync_all();
            let _ = fs::rename(&tmp, path);
        } else {
            tracing::warn!(
                "Failed to create temp file for CRL cache: {}",
                tmp.display()
            );
        }
    }
}
