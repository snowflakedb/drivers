use super::config::CrlConfig;
use crate::crl::cache::CrlCache;
use crate::crl::certificate_parser::is_short_lived_certificate;
use crate::crl::error::CrlError;
use std::sync::Arc;

#[derive(Debug)]
pub struct CrlValidator {
    pub config: CrlConfig,
    cache: Arc<CrlCache>,
}

impl CrlValidator {
    pub fn new(config: CrlConfig) -> Result<Self, CrlError> {
        let cache = CrlCache::global(config.clone()).clone();
        Ok(Self { config, cache })
    }

    /// Validate provided certificate chains. Returns Ok(()) if at least one chain is unrevoked.
    pub async fn validate_certificate_chains(
        &self,
        cert_chains: &[Vec<Vec<u8>>],
    ) -> Result<(), CrlError> {
        if cert_chains.is_empty() {
            return Ok(());
        }

        // Iterate chains; pass if any chain validates without revocations
        for chain in cert_chains {
            if self.validate_certificate_chain(chain).await? {
                return Ok(());
            }
        }

        // No fully valid chain found
        Err(CrlError::AllChainsRevoked {
            location: snafu::Location::new(file!(), line!(), 0),
        })
    }

    /// Returns true if chain is unrevoked and without errors; errors mark chain invalid
    async fn validate_certificate_chain(&self, chain: &[Vec<u8>]) -> Result<bool, CrlError> {
        if chain.is_empty() {
            return Ok(true);
        }
        // Check all but root
        let mut had_error = false;
        for (idx, cert_der) in chain.iter().enumerate() {
            if idx == chain.len() - 1 {
                break;
            }

            // Skip short-lived
            if matches!(is_short_lived_certificate(cert_der), Ok(true)) {
                continue;
            }

            let issuer_der = chain.get(idx + 1).map(|v| v.as_slice());

            // attempt once, and if NotDetermined due to expired CRL, refetch and retry
            let outcome_once = self.cache.check_revocation(cert_der, issuer_der).await;
            let outcome = match outcome_once {
                Ok(o) => Ok(o),
                Err(_) => {
                    // Force a refetch by removing any memory entry and calling fetch path
                    // Simplest approach: call get(url) again through check_revocation, which will
                    // build fresh CRL if expired (expires_at is checked in memory path). Just retry once.
                    self.cache.check_revocation(cert_der, issuer_der).await
                }
            };

            match outcome {
                Ok(outcome) => {
                    use crate::tls::revocation::RevocationOutcome;
                    match outcome {
                        RevocationOutcome::Revoked { .. } => {
                            // Chain is definitively revoked
                            return Ok(false);
                        }
                        RevocationOutcome::NotDetermined => {
                            if self.config.allow_certificates_without_crl_url {
                                tracing::warn!(
                                    target: "sf_core::crl",
                                    "Certificate missing CRL distribution points; allowing due to config"
                                );
                            } else {
                                had_error = true;
                            }
                        }
                        RevocationOutcome::NotRevoked => {}
                    }
                }
                Err(_) => {
                    had_error = true;
                }
            }
        }
        Ok(!had_error)
    }
}
