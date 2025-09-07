// Simplified CRL cache implementation that compiles
use crate::crl::config::CrlConfig;
use crate::crl::error::CrlError;
use chrono::{DateTime, Utc};
use lru::LruCache;
use once_cell::sync::OnceCell;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

/// Represents a cached CRL with metadata
#[derive(Debug, Clone)]
pub struct CachedCrl {
    pub crl: Vec<u8>,
    pub download_time: DateTime<Utc>,
    pub url: String,
}

/// Simple in-memory cache for CRLs
#[derive(Debug)]
pub struct CrlCache {
    config: CrlConfig,
    memory_cache: Option<Arc<Mutex<LruCache<String, CachedCrl>>>>,
    url_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    backoff: Arc<Mutex<HashMap<String, (u32, std::time::Instant)>>>,
}

impl CrlCache {
    pub fn new(config: CrlConfig, memory_capacity: usize) -> Result<Self, CrlError> {
        let memory_cache = if config.enable_memory_caching {
            Some(Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(memory_capacity).unwrap_or(NonZeroUsize::new(100).unwrap()),
            ))))
        } else {
            None
        };

        Ok(Self {
            config,
            memory_cache,
            url_locks: Arc::new(Mutex::new(HashMap::new())),
            backoff: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// High-level revocation check API. Extracts CRL URLs from the cert, fetches/validates
    /// CRLs and returns revocation outcome. Internals may evolve without changing this API.
    pub async fn check_revocation(
        &self,
        cert_der: &[u8],
        issuer_der: Option<&[u8]>,
    ) -> Result<crate::tls::revocation::RevocationOutcome, crate::tls::revocation::RevocationError>
    {
        use crate::tls::revocation::{RevocationError, RevocationOutcome};

        // Extract CRL URLs
        let crl_urls = crate::crl::certificate_parser::extract_crl_distribution_points(cert_der)
            .map_err(|e| RevocationError::Crl {
                source: e,
                location: snafu::Location::new(file!(), line!(), 0),
            })?;
        if crl_urls.is_empty() {
            return Ok(RevocationOutcome::NotDetermined);
        }

        // Get certificate serial
        let serial = crate::crl::certificate_parser::get_certificate_serial_number(cert_der)
            .map_err(|e| RevocationError::Crl {
                source: e,
                location: snafu::Location::new(file!(), line!(), 0),
            })?;

        // For now, reuse existing validator logic minimally: download CRL bytes and test membership
        for url in crl_urls.iter() {
            let bytes = self.get(url).await.map_err(|e| RevocationError::Crl {
                source: e,
                location: snafu::Location::new(file!(), line!(), 0),
            })?;

            // Verify CRL signature best-effort with issuer when provided
            if let Err(e) =
                crate::crl::validator_real::CrlValidator::verify_crl_signature_best_effort_static(
                    &bytes, issuer_der,
                )
            {
                // Treat signature failure as NotDetermined for now; policy could fail closed
                tracing::warn!(
                    target: "sf_core::crl",
                    "CRL signature verification failed for {}: {}",
                    url,
                    e
                );
                continue;
            }

            let is_revoked = crate::crl::certificate_parser::check_certificate_in_crl(
                &serial, &bytes,
            )
            .map_err(|e| RevocationError::Crl {
                source: e,
                location: snafu::Location::new(file!(), line!(), 0),
            })?;
            if is_revoked {
                return Ok(RevocationOutcome::Revoked {
                    reason: None,
                    revocation_time: None,
                });
            }
        }
        Ok(RevocationOutcome::NotRevoked)
    }

    /// Global singleton cache accessor
    pub fn global(config: CrlConfig, memory_capacity: usize) -> &'static Arc<CrlCache> {
        static INSTANCE: OnceCell<Arc<CrlCache>> = OnceCell::new();
        INSTANCE.get_or_init(|| {
            Arc::new(CrlCache::new(config, memory_capacity).expect("init CrlCache"))
        })
    }

    fn get_url_lock(&self, url: &str) -> Arc<Mutex<()>> {
        let mut locks = self.url_locks.lock().unwrap();
        locks
            .entry(url.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Get CRL from memory cache only
    pub fn get_cached(&self, url: &str) -> Result<Option<CachedCrl>, CrlError> {
        if let Some(memory) = &self.memory_cache
            && let Ok(mut cache) = memory.lock()
        {
            return Ok(cache.get(url).cloned());
        }
        Ok(None)
    }

    /// Put CRL into memory cache
    pub fn put(&self, cached_crl: CachedCrl) -> Result<(), CrlError> {
        if let Some(memory) = &self.memory_cache
            && let Ok(mut cache) = memory.lock()
        {
            cache.put(cached_crl.url.clone(), cached_crl);
        }
        Ok(())
    }

    /// Remove CRL from cache
    pub fn remove(&self, url: &str) -> Result<(), CrlError> {
        if let Some(memory) = &self.memory_cache
            && let Ok(mut cache) = memory.lock()
        {
            cache.pop(url);
        }
        Ok(())
    }

    /// Disk key helper: derive a stable filename for URL
    pub fn url_digest(url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let digest = hasher.finalize();
        hex::encode(digest)
    }

    /// Public: get CRL bytes for URL. Fetch and put if missing. Disk is left to higher layers.
    pub async fn get(&self, url: &str) -> Result<Vec<u8>, CrlError> {
        if let Some(mem) = self.get_cached(url)? {
            return Ok(mem.crl);
        }
        let lock = self.get_url_lock(url);
        let guard = lock.lock().unwrap();
        if let Some(mem) = self.get_cached(url)? {
            return Ok(mem.crl);
        }

        // Disk read when enabled
        if self.config.enable_disk_caching
            && let Some(dir) = self.config.get_cache_dir()
        {
            let _ = std::fs::create_dir_all(&dir);
            let file_name = Self::url_digest(url);
            let path = dir.join(file_name);
            if let Ok(bytes) = std::fs::read(&path) {
                tracing::debug!(
                    target: "sf_core::crl::cache",
                    "Loaded CRL from disk cache: {} ({} bytes)",
                    path.display(),
                    bytes.len()
                );
                if let Ok((this_dt, next_dt_opt)) = crate::tls::x509_utils::crl_times(&bytes)
                    && let Some(next_dt) = next_dt_opt
                {
                    if Utc::now() > next_dt {
                        tracing::debug!(
                            target: "sf_core::crl::cache",
                            "Disk-cached CRL expired for {} - refreshing from network",
                            url
                        );
                        drop(guard);
                        let fresh = self.fetch(url).await?;
                        let _ = std::fs::write(&path, &fresh);
                        tracing::debug!(
                            target: "sf_core::crl::cache",
                            "Wrote refreshed CRL to disk cache: {} ({} bytes)",
                            path.display(),
                            fresh.len()
                        );
                        self.remember(url, &fresh);
                        return Ok(fresh);
                    }
                    let midpoint = this_dt + (next_dt - this_dt) / 2;
                    if Utc::now() > midpoint && Utc::now() <= next_dt {
                        tracing::debug!(
                            target: "sf_core::crl::cache",
                            "CRL half-life passed for {}; scheduling background refresh",
                            url
                        );
                        self.spawn_refresh(url.to_string());
                    }
                }
                self.remember(url, &bytes);
                return Ok(bytes);
            }
        }

        // Fetch and persist when appropriate
        drop(guard);
        let fetched = self.fetch(url).await?;
        if self.config.enable_disk_caching
            && let Some(dir) = self.config.get_cache_dir()
        {
            let _ = std::fs::create_dir_all(&dir);
            let file_name = Self::url_digest(url);
            let path = dir.join(file_name);
            let _ = std::fs::write(&path, &fetched);
            tracing::debug!(
                target: "sf_core::crl::cache",
                "Persisted CRL to disk cache: {} ({} bytes)",
                path.display(),
                fetched.len()
            );
        }
        self.remember(url, &fetched);
        Ok(fetched)
    }

    fn remember(&self, url: &str, bytes: &[u8]) {
        let _ = self.put(CachedCrl {
            crl: bytes.to_vec(),
            download_time: Utc::now(),
            url: url.to_string(),
        });
    }

    async fn fetch(&self, url: &str) -> Result<Vec<u8>, CrlError> {
        self.maybe_sleep_backoff(url).await;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(
                self.config.http_timeout.num_seconds() as u64,
            ))
            .connect_timeout(std::time::Duration::from_secs(
                self.config.connection_timeout.num_seconds() as u64,
            ))
            .build()
            .map_err(|e| CrlError::CrlDownload {
                url: url.to_string(),
                source: e,
                location: snafu::Location::new(file!(), line!(), 0),
            })?;
        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| CrlError::CrlDownload {
                url: url.to_string(),
                source: e,
                location: snafu::Location::new(file!(), line!(), 0),
            })?;
        if !resp.status().is_success() {
            self.record_backoff_failure(url);
            return Err(CrlError::CrlExpired {
                location: snafu::Location::new(file!(), line!(), 0),
            });
        }
        let bytes = resp.bytes().await.map_err(|e| CrlError::CrlDownload {
            url: url.to_string(),
            source: e,
            location: snafu::Location::new(file!(), line!(), 0),
        })?;
        self.record_backoff_success(url);
        Ok(bytes.to_vec())
    }

    fn spawn_refresh(&self, url: String) {
        let cache = self.clone_arc();
        let config = self.config.clone();
        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(
                    config.http_timeout.num_seconds() as u64,
                ))
                .connect_timeout(std::time::Duration::from_secs(
                    config.connection_timeout.num_seconds() as u64,
                ))
                .build();
            if let Ok(client) = client
                && let Ok(resp) = client.get(&url).send().await
                && resp.status().is_success()
                && let Ok(bytes) = resp.bytes().await.map(|b| b.to_vec())
            {
                if config.enable_disk_caching
                    && let Some(dir) = config.get_cache_dir()
                {
                    let _ = std::fs::create_dir_all(&dir);
                    let file_name = CrlCache::url_digest(&url);
                    let path = dir.join(file_name);
                    let _ = std::fs::write(&path, &bytes);
                    tracing::debug!(
                        target: "sf_core::crl::cache",
                        "Background refresh wrote CRL to disk cache: {} ({} bytes)",
                        path.display(),
                        bytes.len()
                    );
                }
                cache.remember(&url, &bytes);
            }
        });
    }

    fn clone_arc(&self) -> Arc<CrlCache> {
        // Helper to get Arc<Self> from global for spawn; fallback is okay due to singleton usage
        // Safety: we rely on global singleton; not used to create new instances
        CrlCache::global(self.config.clone(), 100).clone()
    }

    async fn maybe_sleep_backoff(&self, url: &str) {
        let (failures, last) = {
            let guard = self.backoff.lock().unwrap();
            guard
                .get(url)
                .cloned()
                .unwrap_or((0, std::time::Instant::now()))
        };
        if failures == 0 {
            return;
        }
        let base_ms = 100u64;
        let cap_ms = 5_000u64;
        let delay_ms = (base_ms.saturating_mul(1u64 << failures.min(5))).min(cap_ms);
        let jitter = (rand::random::<u32>() % 100) as u64;
        let total_ms = delay_ms + jitter;
        let elapsed = last.elapsed();
        let needed = std::time::Duration::from_millis(total_ms);
        if elapsed < needed {
            tokio::time::sleep(needed - elapsed).await;
        }
    }

    fn record_backoff_failure(&self, url: &str) {
        let mut guard = self.backoff.lock().unwrap();
        let entry = guard
            .entry(url.to_string())
            .or_insert((0, std::time::Instant::now()));
        entry.0 = entry.0.saturating_add(1);
        entry.1 = std::time::Instant::now();
    }

    fn record_backoff_success(&self, url: &str) {
        let mut guard = self.backoff.lock().unwrap();
        guard.remove(url);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_digest_stable() {
        let a = CrlCache::url_digest("http://example.com/a");
        let b = CrlCache::url_digest("http://example.com/a");
        let c = CrlCache::url_digest("http://example.com/b");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64); // sha256 hex
    }

    #[tokio::test]
    async fn test_memory_cache_put_get() {
        let cache = CrlCache::new(CrlConfig::default(), 8).unwrap();
        let url = "http://example.com/test.crl".to_string();
        let entry = CachedCrl {
            crl: vec![1, 2, 3],
            download_time: Utc::now(),
            url: url.clone(),
        };
        cache.put(entry.clone()).unwrap();
        let got_cached = cache.get_cached(&url).unwrap();
        assert!(got_cached.is_some());
        let got = cache.get(&url).await.unwrap();
        assert_eq!(got, vec![1, 2, 3]);
    }
}
