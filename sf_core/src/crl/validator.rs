use super::config::CrlConfig;
use crate::crl::cache::CrlCache;
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

        for (idx, cert_der) in chain.iter().enumerate().take(chain.len() - 1) {
            let issuers = &chain[idx + 1..];
            if !self.validate_single_certificate(cert_der, issuers).await? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn validate_single_certificate(
        &self,
        cert_der: &[u8],
        issuers: &[Vec<u8>],
    ) -> Result<bool, CrlError> {
        if matches!(
            crate::crl::certificate_parser::is_short_lived_certificate(cert_der),
            Ok(true)
        ) {
            return Ok(true);
        }

        let issuer_der = issuers.first().map(|v| v.as_slice());
        let issuer_candidates: Vec<&[u8]> = issuers.iter().map(|v| v.as_slice()).collect();

        let outcome = match self
            .cache
            .check_revocation(cert_der, issuer_der, Some(&issuer_candidates))
            .await
        {
            Ok(o) => Ok(o),
            Err(e) => {
                let should_retry = matches!(
                    e,
                    crate::tls::revocation::RevocationError::CrlOperation {
                        source: crate::crl::error::CrlError::CrlExpired { .. },
                        ..
                    }
                );
                if should_retry {
                    tracing::debug!(target: "sf_core::crl", "CRL expired, attempting refetch");
                    self.cache
                        .check_revocation(cert_der, issuer_der, Some(&issuer_candidates))
                        .await
                } else {
                    Err(e)
                }
            }
        };

        match outcome {
            Ok(crate::tls::revocation::RevocationOutcome::Revoked { .. }) => Ok(false),
            Ok(crate::tls::revocation::RevocationOutcome::NotDetermined) => {
                if self.config.allow_certificates_without_crl_url {
                    tracing::warn!(
                        target: "sf_core::crl",
                        "Certificate missing CRL distribution points; allowing due to config"
                    );
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Ok(crate::tls::revocation::RevocationOutcome::NotRevoked) => Ok(true),
            Err(e) => {
                tracing::warn!(target: "sf_core::crl", error = %e, "CRL check failed for one certificate in the chain");
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
impl CrlValidator {
    pub(crate) async fn fetch_crl_with_cache(&self, url: &str) -> Result<Vec<u8>, CrlError> {
        self.cache.get(url).await
    }

    pub(crate) fn write_crl_atomic(&self, path: &std::path::Path, data: &[u8]) {
        use std::io::Write;
        // Best-effort atomic write: write to a temp file in the same directory, then rename
        if let Some(dir) = path.parent() {
            let tmp_name = match path.file_name().and_then(|s| s.to_str()) {
                Some(name) => format!(".{}.tmp", name),
                None => ".tmp_crl.tmp".to_string(),
            };
            let tmp_path = dir.join(tmp_name);
            if let Ok(mut file) = std::fs::File::create(&tmp_path) {
                let _ = file.write_all(data);
                let _ = file.sync_all();
                let _ = std::fs::rename(&tmp_path, path);
                return;
            }
        }
        // Fallback: direct write
        let _ = std::fs::write(path, data);
    }
}
