use super::config::CrlConfig;
use crate::crl::cache::CrlCache;
use crate::crl::stats_buffer::CrlStatsBuffer;
use std::sync::Arc;
use x509_cert::der::{Decode, Encode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevokedCert {
    EndEntity,
    Intermediate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainValidationOutcome {
    NotRevoked,
    Revoked(RevokedCert),
    Indeterminate,
}

#[derive(Debug)]
pub struct CrlValidator {
    pub config: CrlConfig,
    cache: Arc<CrlCache>,
    root_store: Option<Arc<rustls::RootCertStore>>, // used to verify CRL signatures for top-most intermediates
    crl_stats: Option<CrlStatsBuffer>,
}

impl CrlValidator {
    pub fn new(config: CrlConfig, cache: Arc<CrlCache>) -> Self {
        Self {
            config,
            cache,
            root_store: None,
            crl_stats: None,
        }
    }

    pub fn new_with_root_store(
        config: CrlConfig,
        root_store: Option<Arc<rustls::RootCertStore>>,
        cache: Arc<CrlCache>,
    ) -> Self {
        Self {
            config,
            cache,
            root_store,
            crl_stats: None,
        }
    }

    pub(crate) fn with_crl_stats(mut self, crl_stats: CrlStatsBuffer) -> Self {
        self.crl_stats = Some(crl_stats);
        self
    }

    pub(crate) async fn validate_certificate_chain(
        &self,
        chain: &[Vec<u8>],
    ) -> ChainValidationOutcome {
        if chain.is_empty() {
            return ChainValidationOutcome::NotRevoked;
        }

        if chain.len() == 1 {
            return self.validate_single_certificate(&chain[0], &[], true).await;
        }

        let mut chain_outcome = ChainValidationOutcome::NotRevoked;
        for (idx, pair) in chain.windows(2).enumerate() {
            let [cert, parent] = pair else {
                continue;
            };
            let issuers = &chain[idx + 1..];
            match self
                .validate_single_certificate(cert, issuers, idx == 0)
                .await
            {
                ChainValidationOutcome::NotRevoked => {}
                outcome @ ChainValidationOutcome::Revoked(_) => return outcome,
                ChainValidationOutcome::Indeterminate => {
                    chain_outcome = ChainValidationOutcome::Indeterminate;
                }
            }
            // This shouldn't be necessary since the chains we pass in are anchored, but let's be safe.
            if self.is_anchor(parent) {
                return chain_outcome;
            }
        }

        // Validate the top-most certificate (no further issuers).
        let Some(top_cert) = chain.last() else {
            // Unreachable: empty and single-element chains are handled above, so
            // the chain has >= 2 elements here. If that invariant is ever broken,
            // fail closed (treat the chain as invalid) rather than silently
            // reporting the chain as validated.
            tracing::error!(
                target: "sf_core::crl",
                "validate_certificate_chain reached the tail with an empty chain (unexpected); failing closed"
            );
            return ChainValidationOutcome::Indeterminate;
        };
        match self.validate_single_certificate(top_cert, &[], false).await {
            ChainValidationOutcome::NotRevoked => chain_outcome,
            outcome => outcome,
        }
    }

    fn is_anchor(&self, cert_der: &[u8]) -> bool {
        let store = match self.root_store.as_deref() {
            Some(s) => s,
            None => return false,
        };
        if let Ok(cert) = x509_cert::Certificate::from_der(cert_der)
            && let Ok(subject_der) = cert.tbs_certificate.subject.to_der()
        {
            return store
                .roots
                .iter()
                .any(|a| a.subject.as_ref() == subject_der.as_slice());
        }
        false
    }

    async fn validate_single_certificate(
        &self,
        cert_der: &[u8],
        issuers: &[Vec<u8>],
        is_end_entity: bool,
    ) -> ChainValidationOutcome {
        if matches!(
            crate::crl::certificate_parser::is_short_lived_certificate(cert_der),
            Ok(true)
        ) {
            return ChainValidationOutcome::NotRevoked;
        }

        // For non-top certs, the next certificate(s) act as issuer candidates.
        // For the top cert (no issuers), rely on the configured root store for CRL signature verification.
        let (mut issuer_der, issuer_candidates): (Option<&[u8]>, Vec<&[u8]>) = if issuers.is_empty()
        {
            (None, Vec::new())
        } else {
            (
                issuers.first().map(|v| v.as_slice()),
                issuers.iter().map(|v| v.as_slice()).collect(),
            )
        };

        if issuer_der.is_none()
            && issuer_candidates.is_empty()
            && crate::crl::certificate_parser::is_ca_certificate(cert_der).unwrap_or(false)
        {
            issuer_der = Some(cert_der);
        }

        let outcome = match self
            .cache
            .check_revocation(
                cert_der,
                issuer_der,
                if issuer_candidates.is_empty() {
                    None
                } else {
                    Some(&issuer_candidates)
                },
                self.root_store.clone(),
                self.crl_stats.as_ref(),
            )
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
                        .check_revocation(
                            cert_der,
                            issuer_der,
                            Some(&issuer_candidates),
                            self.root_store.clone(),
                            self.crl_stats.as_ref(),
                        )
                        .await
                } else {
                    Err(e)
                }
            }
        };

        match outcome {
            Ok(crate::tls::revocation::RevocationOutcome::Revoked { .. }) => {
                if is_end_entity {
                    ChainValidationOutcome::Revoked(RevokedCert::EndEntity)
                } else {
                    ChainValidationOutcome::Revoked(RevokedCert::Intermediate)
                }
            }
            Ok(crate::tls::revocation::RevocationOutcome::NotDetermined) => {
                if self.config.allow_certificates_without_crl_url {
                    tracing::warn!(
                        target: "sf_core::crl",
                        "Certificate missing CRL distribution points; allowing due to config"
                    );
                    ChainValidationOutcome::NotRevoked
                } else {
                    ChainValidationOutcome::Indeterminate
                }
            }
            Ok(crate::tls::revocation::RevocationOutcome::NotRevoked) => {
                ChainValidationOutcome::NotRevoked
            }
            Err(e) => {
                tracing::warn!(target: "sf_core::crl", error = %e, "CRL check failed for one certificate in the chain");
                ChainValidationOutcome::Indeterminate
            }
        }
    }
}

#[cfg(test)]
impl CrlValidator {
    pub(crate) async fn fetch_crl_with_cache(
        &self,
        url: &str,
    ) -> Result<Vec<u8>, crate::crl::error::CrlError> {
        self.cache.get(url).await
    }
}
