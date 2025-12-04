use super::config::CrlConfig;
use crate::crl::cache::CrlCache;
use crate::crl::error::CrlError;
use std::sync::Arc;
use x509_cert::der::{Decode, Encode};

#[derive(Debug)]
pub struct CrlValidator {
    pub config: CrlConfig,
    cache: Arc<CrlCache>,
    root_store: Option<Arc<rustls::RootCertStore>>, // used to verify CRL signatures for top-most intermediates
}

impl CrlValidator {
    pub fn new(config: CrlConfig) -> Result<Self, CrlError> {
        let cache = CrlCache::global(config.clone()).clone();
        Ok(Self {
            config,
            cache,
            root_store: None,
        })
    }

    pub fn new_with_root_store(
        config: CrlConfig,
        root_store: Option<Arc<rustls::RootCertStore>>,
    ) -> Result<Self, CrlError> {
        let cache = CrlCache::global(config.clone()).clone();
        Ok(Self {
            config,
            cache,
            root_store,
        })
    }

    /// Returns true if chain is unrevoked and without errors; errors mark chain invalid
    pub(crate) async fn validate_certificate_chain(
        &self,
        chain: &[Vec<u8>],
    ) -> Result<bool, CrlError> {
        if chain.is_empty() {
            return Ok(true);
        }

        if chain.len() == 1 {
            return self.validate_single_certificate(&chain[0], &[], true).await;
        }

        for (idx, pair) in chain.windows(2).enumerate() {
            let [cert, parent] = pair else {
                continue;
            };
            let issuers = &chain[idx + 1..];
            if !self
                .validate_single_certificate(cert, issuers, idx == 0)
                .await?
            {
                return Ok(false);
            }
            // This shouldn't be necessary since the chains we pass in are anchored, but let's be safe.
            if self.is_anchor(parent) {
                return Ok(true);
            }
        }

        // Validate the top-most certificate (no further issuers)
        self.validate_single_certificate(chain.last().unwrap(), &[], false)
            .await
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
    ) -> Result<bool, CrlError> {
        if matches!(
            crate::crl::certificate_parser::is_short_lived_certificate(cert_der),
            Ok(true)
        ) {
            return Ok(true);
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
                self.root_store.as_deref(),
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
                            self.root_store.as_deref(),
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
                    return Err(CrlError::EndEntityRevoked {
                        location: snafu::Location::new(file!(), line!(), 0),
                    });
                }
                Ok(false)
            }
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
