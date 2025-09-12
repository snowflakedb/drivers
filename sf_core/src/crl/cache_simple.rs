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
// FromDer not needed after centralizing helpers
use opentelemetry::metrics::{Counter, Histogram, Meter};
use opentelemetry::{KeyValue, global};

/// Represents a cached CRL with metadata
#[derive(Debug, Clone)]
pub struct CachedCrl {
    pub crl: Vec<u8>,
    pub download_time: DateTime<Utc>,
    pub url: String,
}

/// Simple (interface) to Cache for CRLs
#[derive(Debug)]
pub struct CrlCache {
    config: CrlConfig,
    memory_cache: Option<Arc<Mutex<LruCache<String, CachedCrl>>>>,
    url_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    backoff: Arc<Mutex<HashMap<String, (u32, std::time::Instant)>>>,
    // New: per-certificate outcome cache keyed by (issuer_key, serial)
    #[allow(clippy::type_complexity)]
    outcome_cache: Arc<Mutex<OutcomeLru>>,
}

#[derive(Debug, Clone)]
struct OutcomeCacheEntry {
    outcome: crate::tls::revocation::RevocationOutcome,
    valid_until: Option<chrono::DateTime<chrono::Utc>>,
}

type OutcomeKey = (Vec<u8>, Vec<u8>);
type OutcomeLru = LruCache<OutcomeKey, OutcomeCacheEntry>;

#[derive(Debug, Clone)]
struct CrlMetrics {
    outcome_hit_total: Counter<u64>,
    outcome_stale_total: Counter<u64>,
    outcome_store_total: Counter<u64>,
    get_total: Counter<u64>,
    get_disk_expired_total: Counter<u64>,
    fetch_total: Counter<u64>,
    fetch_error_total: Counter<u64>,
    revocation_check_ms: Histogram<u64>,
    get_ms: Histogram<u64>,
    fetch_ms: Histogram<u64>,
}

impl CrlMetrics {
    fn init(meter: &Meter) -> Self {
        Self {
            outcome_hit_total: meter
                .u64_counter("crl_outcome_cache_hit_total")
                .with_description("Outcome cache hits")
                .build(),
            outcome_stale_total: meter
                .u64_counter("crl_outcome_cache_stale_total")
                .with_description("Outcome cache stale entries")
                .build(),
            outcome_store_total: meter
                .u64_counter("crl_outcome_cache_store_total")
                .with_description("Outcome cache stores")
                .build(),
            get_total: meter
                .u64_counter("crl_get_total")
                .with_description("CRL get by source")
                .build(),
            get_disk_expired_total: meter
                .u64_counter("crl_get_disk_expired_total")
                .with_description("Disk-cached CRLs found expired")
                .build(),
            fetch_total: meter
                .u64_counter("crl_fetch_total")
                .with_description("CRL fetch attempts")
                .build(),
            fetch_error_total: meter
                .u64_counter("crl_fetch_error_total")
                .with_description("CRL fetch errors")
                .build(),
            revocation_check_ms: meter
                .u64_histogram("crl_revocation_check_ms")
                .with_description("Revocation check latency (ms)")
                .build(),
            get_ms: meter
                .u64_histogram("crl_get_ms")
                .with_description("CRL get latency (ms)")
                .build(),
            fetch_ms: meter
                .u64_histogram("crl_fetch_ms")
                .with_description("CRL fetch latency (ms)")
                .build(),
        }
    }
}

fn metrics() -> &'static CrlMetrics {
    static METRICS: OnceCell<CrlMetrics> = OnceCell::new();
    METRICS.get_or_init(|| {
        let meter = global::meter("sf_core.crl");
        CrlMetrics::init(&meter)
    })
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

        let outcome_capacity = config.outcome_cache_capacity;
        Ok(Self {
            config,
            memory_cache,
            url_locks: Arc::new(Mutex::new(HashMap::new())),
            backoff: Arc::new(Mutex::new(HashMap::new())),
            outcome_cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(outcome_capacity).unwrap_or(NonZeroUsize::new(10_000).unwrap()),
            ))),
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
        let span = tracing::span!(tracing::Level::DEBUG, "crl_check_revocation");
        let _enter = span.enter();
        let start = std::time::Instant::now();
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

        // Outcome cache lookup when issuer known
        if let Some(issuer) = issuer_der
            && let Some((issuer_key, serial_key)) = self.make_outcome_key(issuer, &serial)
            && let Ok(mut oc) = self.outcome_cache.lock()
            && let Some(entry) = oc.get(&(issuer_key.clone(), serial_key.clone()))
        {
            if entry.valid_until.is_none_or(|dt| chrono::Utc::now() <= dt) {
                tracing::debug!(
                    target:"sf_core::crl::cache",
                    "Outcome cache HIT (valid) for issuer_key_len={} serial_len={}",
                    issuer_key.len(),
                    serial_key.len()
                );
                metrics().outcome_hit_total.add(1, &[]);
                tracing::trace!("revocation_check_ms={}", start.elapsed().as_millis() as u64);
                return Ok(entry.outcome.clone());
            }
            tracing::debug!(
                target:"sf_core::crl::cache",
                "Outcome cache STALE for issuer_key_len={} serial_len={}",
                issuer_key.len(),
                serial_key.len()
            );
            metrics().outcome_stale_total.add(1, &[]);
        }

        let mut any_checked = false;
        let mut ttl: Option<chrono::DateTime<chrono::Utc>> = None;
        for url in crl_urls.iter() {
            let bytes = self.get(url).await.map_err(|e| RevocationError::Crl {
                source: e,
                location: snafu::Location::new(file!(), line!(), 0),
            })?;

            // Verify CRL signature best-effort with issuer when provided
            if let Err(e) =
                crate::tls::x509_utils::verify_crl_signature_best_effort(&bytes, issuer_der)
            {
                tracing::warn!(target:"sf_core::crl", "CRL signature verification failed for {}: {}", url, e);
                continue;
            }

            // Compute TTL from nextUpdate if present (via x509_utils)
            if let Ok((_, Some(next_dt))) = crate::tls::x509_utils::crl_times(bytes.as_slice()) {
                ttl = match ttl {
                    Some(cur) => Some(std::cmp::min(cur, next_dt)),
                    None => Some(next_dt),
                };
            }

            any_checked = true;
            let is_revoked = crate::crl::certificate_parser::check_certificate_in_crl(
                &serial, &bytes,
            )
            .map_err(|e| RevocationError::Crl {
                source: e,
                location: snafu::Location::new(file!(), line!(), 0),
            })?;
            if is_revoked {
                let outcome = RevocationOutcome::Revoked {
                    reason: None,
                    revocation_time: None,
                };
                self.maybe_store_outcome(issuer_der, &serial, outcome.clone(), ttl);
                let elapsed = start.elapsed();
                tracing::trace!("revocation_check_ms={}", elapsed.as_millis() as u64);
                metrics().revocation_check_ms.record(
                    elapsed.as_millis() as u64,
                    &[KeyValue::new("outcome", "revoked")],
                );
                return Ok(outcome);
            }
        }
        let outcome = if any_checked {
            RevocationOutcome::NotRevoked
        } else {
            RevocationOutcome::NotDetermined
        };
        self.maybe_store_outcome(issuer_der, &serial, outcome.clone(), ttl);
        let elapsed = start.elapsed();
        tracing::trace!("revocation_check_ms={}", elapsed.as_millis() as u64);
        let label = match outcome {
            RevocationOutcome::NotRevoked => "not_revoked",
            RevocationOutcome::NotDetermined => "not_determined",
            RevocationOutcome::Revoked { .. } => "revoked",
        };
        metrics().revocation_check_ms.record(
            elapsed.as_millis() as u64,
            &[KeyValue::new("outcome", label)],
        );
        Ok(outcome)
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

    /// Public: get CRL bytes for URL. Fetch and put if missing.
    pub async fn get(&self, url: &str) -> Result<Vec<u8>, CrlError> {
        let span = tracing::span!(tracing::Level::DEBUG, "crl_get", url = url);
        let _enter = span.enter();
        let start = std::time::Instant::now();
        if let Some(mem) = self.get_cached(url)? {
            let ms = start.elapsed().as_millis() as u64;
            metrics()
                .get_ms
                .record(ms, &[KeyValue::new("source", "memory")]);
            metrics()
                .get_total
                .add(1, &[KeyValue::new("source", "memory")]);
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
                        metrics().get_disk_expired_total.add(1, &[]);
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
                let ms = start.elapsed().as_millis() as u64;
                tracing::trace!("crl_get_ms={}", ms);
                metrics()
                    .get_ms
                    .record(ms, &[KeyValue::new("source", "disk")]);
                metrics()
                    .get_total
                    .add(1, &[KeyValue::new("source", "disk")]);
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
            tracing::trace!(
                target: "sf_core::crl::cache",
                "Persisted CRL to disk cache: {} ({} bytes)",
                path.display(),
                fetched.len()
            );
        }
        self.remember(url, &fetched);
        let ms = start.elapsed().as_millis() as u64;
        tracing::trace!("crl_get_ms={}", ms);
        metrics()
            .get_ms
            .record(ms, &[KeyValue::new("source", "network")]);
        metrics()
            .get_total
            .add(1, &[KeyValue::new("source", "network")]);
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
        let span = tracing::span!(tracing::Level::DEBUG, "crl_fetch", url = url);
        let _enter = span.enter();
        let start = std::time::Instant::now();
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
            metrics().fetch_error_total.add(
                1,
                &[
                    KeyValue::new("kind", "http_status"),
                    KeyValue::new("code", resp.status().as_u16().to_string()),
                ],
            );
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
        let ms = start.elapsed().as_millis() as u64;
        tracing::trace!("crl_fetch_ms={}", ms);
        metrics().fetch_ms.record(ms, &[]);
        metrics().fetch_total.add(1, &[]);
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
                    tracing::trace!(
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

    fn make_outcome_key(&self, issuer_der: &[u8], serial: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
        // Prefer SKID; fallback to subject DER hash
        let issuer_key = if let Some(s) = crate::tls::x509_utils::extract_skid(issuer_der) {
            s
        } else {
            crate::tls::x509_utils::subject_der_hash(issuer_der).unwrap_or_else(|| {
                let mut hasher = Sha256::new();
                hasher.update(issuer_der);
                hasher.finalize().to_vec()
            })
        };
        let serial_key = Self::normalize_serial(serial);
        Some((issuer_key, serial_key))
    }

    fn normalize_serial(serial: &[u8]) -> Vec<u8> {
        let mut i = 0usize;
        while i < serial.len() && serial[i] == 0 {
            i += 1;
        }
        if i >= serial.len() {
            vec![0]
        } else {
            serial[i..].to_vec()
        }
    }

    fn maybe_store_outcome(
        &self,
        issuer_der: Option<&[u8]>,
        serial: &[u8],
        outcome: crate::tls::revocation::RevocationOutcome,
        valid_until: Option<chrono::DateTime<chrono::Utc>>,
    ) {
        if let Some(issuer) = issuer_der
            && let Some((issuer_key, serial_key)) = self.make_outcome_key(issuer, serial)
            && let Ok(mut oc) = self.outcome_cache.lock()
        {
            tracing::debug!(
                target:"sf_core::crl::cache",
                "Outcome cache STORE issuer_key_len={} serial_len={} ttl={:?}",
                issuer_key.len(),
                serial_key.len(),
                valid_until
            );
            metrics().outcome_store_total.add(1, &[]);
            oc.put(
                (issuer_key, serial_key),
                OutcomeCacheEntry {
                    outcome,
                    valid_until,
                },
            );
        }
    }
}

#[cfg(test)]
impl CrlCache {
    fn test_outcome_cache_put_raw(
        &self,
        issuer_key: Vec<u8>,
        serial_key: Vec<u8>,
        outcome: crate::tls::revocation::RevocationOutcome,
        valid_until: Option<chrono::DateTime<chrono::Utc>>,
    ) {
        if let Ok(mut oc) = self.outcome_cache.lock() {
            oc.put(
                (issuer_key, serial_key),
                OutcomeCacheEntry {
                    outcome,
                    valid_until,
                },
            );
        }
    }

    fn test_outcome_cache_get_validated(
        &self,
        issuer_key: Vec<u8>,
        serial_key: Vec<u8>,
    ) -> Option<crate::tls::revocation::RevocationOutcome> {
        if let Ok(mut oc) = self.outcome_cache.lock()
            && let Some(entry) = oc.get(&(issuer_key, serial_key))
        {
            if entry.valid_until.is_none_or(|dt| chrono::Utc::now() <= dt) {
                return Some(entry.outcome.clone());
            }
            return None;
        }
        None
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

    #[test]
    fn test_outcome_cache_hit_and_stale() {
        let cache = CrlCache::new(CrlConfig::default(), 8).unwrap();

        let issuer_key = vec![1, 2, 3, 4];
        let serial_key = vec![0, 0, 0x01, 0x02];

        // Store a valid outcome
        let future = chrono::Utc::now() + chrono::Duration::seconds(60);
        cache.test_outcome_cache_put_raw(
            issuer_key.clone(),
            serial_key.clone(),
            crate::tls::revocation::RevocationOutcome::NotRevoked,
            Some(future),
        );

        // Should be a hit and return the stored outcome
        let hit = cache.test_outcome_cache_get_validated(issuer_key.clone(), serial_key.clone());
        assert!(matches!(
            hit,
            Some(crate::tls::revocation::RevocationOutcome::NotRevoked)
        ));

        // Overwrite with a stale entry
        let past = chrono::Utc::now() - chrono::Duration::seconds(60);
        cache.test_outcome_cache_put_raw(
            issuer_key.clone(),
            serial_key.clone(),
            crate::tls::revocation::RevocationOutcome::Revoked {
                reason: None,
                revocation_time: None,
            },
            Some(past),
        );

        // Should be treated as stale and not returned
        let miss = cache.test_outcome_cache_get_validated(issuer_key, serial_key);
        assert!(miss.is_none());
    }
}
