use crate::crl::validator::RevocationOutcome;
use lru::LruCache;
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use x509_parser::prelude::FromDer;

/// Configuration for CRL cache behavior.
#[derive(Debug, Clone)]
pub struct CrlCacheConfig {
    /// Enable CRL cache logic. When disabled, operations return Unknown (fail-open).
    pub enabled: bool,
    /// Maximum number of outcome entries to keep in-memory.
    pub outcome_capacity: usize,
    /// Maximum number of raw CRL blobs to keep in-memory.
    pub raw_capacity: usize,
    /// Fallback TTL to apply to CRLs when no validity info is available.
    pub ttl_seconds: u64,
    /// Base backoff in milliseconds after a failed fetch.
    pub backoff_base_ms: u64,
    /// Maximum backoff cap in milliseconds.
    pub backoff_cap_ms: u64,
    /// Maximum jitter in milliseconds added to backoff.
    pub backoff_jitter_ms: u64,
}

impl Default for CrlCacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            outcome_capacity: 10_000,
            raw_capacity: 1_000,
            ttl_seconds: 60 * 10, // 10 minutes
            backoff_base_ms: 100,
            backoff_cap_ms: 5_000,
            backoff_jitter_ms: 100,
        }
    }
}

/// Abstraction for fetching CRLs from URLs. Synchronous for simplicity.
pub trait CrlFetcher: Send + Sync + 'static {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, String>;
}

struct CachedCrl {
    bytes: Vec<u8>,
    expires_at: Instant,
}

fn default_expires(ttl_seconds: u64) -> Instant {
    Instant::now() + Duration::from_secs(ttl_seconds)
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// In-memory CRL cache with raw CRL and outcome memoization.
pub struct CrlCache<F: CrlFetcher> {
    config: CrlCacheConfig,
    raw_crls: Arc<Mutex<LruCache<String, CachedCrl>>>,
    outcomes: Arc<Mutex<LruCache<(u64, String), RevocationOutcome>>>, // (serial_hash, url)
    fetcher: F,
    url_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    backoff: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
}

impl<F: CrlFetcher> CrlCache<F> {
    pub fn new(config: CrlCacheConfig, fetcher: F) -> Self {
        let raw_cap = NonZeroUsize::new(config.raw_capacity.max(1)).unwrap();
        let out_cap = NonZeroUsize::new(config.outcome_capacity.max(1)).unwrap();
        Self {
            config,
            raw_crls: Arc::new(Mutex::new(LruCache::new(raw_cap))),
            outcomes: Arc::new(Mutex::new(LruCache::new(out_cap))),
            fetcher,
            url_locks: Arc::new(Mutex::new(HashMap::new())),
            backoff: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Test-oriented API: check revocation outcome given certificate serial and CRL URLs.
    /// Returns Unknown on errors (fail-open).
    pub fn check_revocation_with_urls(
        &self,
        cert_serial: &[u8],
        crl_urls: &[String],
    ) -> RevocationOutcome {
        if !self.config.enabled {
            return RevocationOutcome::Unknown;
        }
        let serial_hash = hash_bytes(cert_serial);
        for url in crl_urls {
            if let Ok(mut outcomes) = self.outcomes.lock()
                && let Some(outcome) = outcomes.get(&(serial_hash, url.clone())).copied()
            {
                return outcome;
            }

            let crl_bytes = match self.get_or_fetch_crl(url) {
                Some(bytes) => bytes,
                None => continue,
            };

            // Parse CRL and attempt to find the serial number. On parse error, fail-open.
            let outcome =
                match x509_parser::prelude::CertificateRevocationList::from_der(&crl_bytes) {
                    Ok((_, crl)) => {
                        let mut revoked = false;
                        for rc in &crl.tbs_cert_list.revoked_certificates {
                            if rc.user_certificate.to_bytes_be() == cert_serial {
                                revoked = true;
                                break;
                            }
                        }
                        if revoked {
                            RevocationOutcome::Revoked
                        } else {
                            RevocationOutcome::NotRevoked
                        }
                    }
                    Err(_) => RevocationOutcome::Unknown,
                };

            // Memoize outcome for this serial/url pair.
            if let Ok(mut outcomes) = self.outcomes.lock() {
                outcomes.put((serial_hash, url.clone()), outcome);
            }

            if matches!(outcome, RevocationOutcome::Revoked) {
                return outcome;
            }
        }
        RevocationOutcome::Unknown
    }

    fn get_or_fetch_crl(&self, url: &str) -> Option<Vec<u8>> {
        let now = Instant::now();
        if let Ok(mut cache) = self.raw_crls.lock()
            && let Some(entry) = cache.get(url)
            && entry.expires_at > now
        {
            return Some(entry.bytes.clone());
        }

        // Acquire single-flight lock for this URL
        let lock = {
            let mut locks = self.url_locks.lock().unwrap();
            locks
                .entry(url.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _guard = lock.lock().unwrap();

        // Re-check cache after obtaining the lock
        if let Ok(mut cache) = self.raw_crls.lock()
            && let Some(entry) = cache.get(url)
            && entry.expires_at > Instant::now()
        {
            return Some(entry.bytes.clone());
        }

        // Apply backoff if prior failures exist
        if let Some(delay) = self.compute_backoff_delay(url) {
            std::thread::sleep(delay);
        }

        match self.fetcher.fetch(url) {
            Ok(bytes) => {
                // Clear backoff on success
                if let Ok(mut b) = self.backoff.lock() {
                    b.remove(url);
                }
                let expires_at = default_expires(self.config.ttl_seconds);
                if let Ok(mut cache) = self.raw_crls.lock() {
                    cache.put(
                        url.to_string(),
                        CachedCrl {
                            bytes: bytes.clone(),
                            expires_at,
                        },
                    );
                }
                Some(bytes)
            }
            Err(_) => {
                self.record_backoff_failure(url);
                None
            }
        }
    }

    fn compute_backoff_delay(&self, url: &str) -> Option<Duration> {
        let (failures, last) = {
            let guard = self.backoff.lock().unwrap();
            guard.get(url).cloned()?
        };
        if failures == 0 {
            return None;
        }
        let exp = failures.min(6);
        let factor = 1u64 << exp;
        let mut delay_ms = self.config.backoff_base_ms.saturating_mul(factor);
        if delay_ms > self.config.backoff_cap_ms {
            delay_ms = self.config.backoff_cap_ms;
        }
        let jitter = if self.config.backoff_jitter_ms > 0 {
            (rand::random::<u32>() as u64) % self.config.backoff_jitter_ms
        } else {
            0
        };
        let total = Duration::from_millis(delay_ms + jitter);
        let elapsed = last.elapsed();
        if elapsed >= total {
            None
        } else {
            Some(total - elapsed)
        }
    }

    fn record_backoff_failure(&self, url: &str) {
        let mut guard = self.backoff.lock().unwrap();
        let entry = guard.entry(url.to_string()).or_insert((0, Instant::now()));
        entry.0 = entry.0.saturating_add(1);
        entry.1 = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct CountingFetcher {
        count: Arc<Mutex<usize>>,
        payload: Vec<u8>,
    }

    impl CountingFetcher {
        fn new(payload: Vec<u8>) -> (Self, Arc<Mutex<usize>>) {
            let counter = Arc::new(Mutex::new(0usize));
            (
                Self {
                    count: counter.clone(),
                    payload,
                },
                counter,
            )
        }
    }

    impl CrlFetcher for CountingFetcher {
        fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
            let mut g = self.count.lock().unwrap();
            *g += 1;
            Ok(self.payload.clone())
        }
    }

    #[test]
    fn caches_raw_crl_fetches() {
        // Invalid CRL bytes to exercise fail-open path; still cached.
        let (fetcher, counter) = CountingFetcher::new(vec![0x00, 0x01, 0x02]);
        let cache = CrlCache::new(
            CrlCacheConfig {
                enabled: true,
                outcome_capacity: 16,
                raw_capacity: 4,
                ttl_seconds: 60,
                backoff_base_ms: 50,
                backoff_cap_ms: 200,
                backoff_jitter_ms: 0,
            },
            fetcher,
        );

        let serial = [0x12, 0x34, 0x56];
        let urls = vec!["https://example.com/crl1".to_string()];

        let _ = cache.check_revocation_with_urls(&serial, &urls);
        let _ = cache.check_revocation_with_urls(&serial, &urls);

        let calls = *counter.lock().unwrap();
        assert_eq!(calls, 1, "second call should hit raw cache, not refetch");
    }

    #[test]
    fn outcome_memoization_short_circuits_fetch() {
        let (fetcher, counter) = CountingFetcher::new(vec![0x30, 0x00]); // invalid but deterministic
        let cache = CrlCache::new(
            CrlCacheConfig {
                enabled: true,
                outcome_capacity: 16,
                raw_capacity: 4,
                ttl_seconds: 600,
                backoff_base_ms: 50,
                backoff_cap_ms: 200,
                backoff_jitter_ms: 0,
            },
            fetcher,
        );

        let url = "https://example.com/crl".to_string();
        let serial = [0xAA, 0xBB];

        // First call fetches and caches outcome (Unknown)
        let _ = cache.check_revocation_with_urls(&serial, std::slice::from_ref(&url));
        let calls_after_first = *counter.lock().unwrap();
        assert_eq!(calls_after_first, 1);

        // Clear raw cache to ensure only outcome memoization can prevent a fetch
        if let Ok(mut rc) = cache.raw_crls.lock() {
            rc.clear();
        }

        // Second call with same serial/url should NOT fetch due to outcome memoization
        let _ = cache.check_revocation_with_urls(&serial, std::slice::from_ref(&url));
        let calls_after_second = *counter.lock().unwrap();
        assert_eq!(
            calls_after_second, 1,
            "outcome cache should short-circuit fetch"
        );
    }

    #[test]
    fn raw_lru_eviction_triggers_refetch() {
        let (fetcher, counter) = CountingFetcher::new(vec![0x30, 0x00]);
        // raw_capacity = 1 to force eviction
        let cache = CrlCache::new(
            CrlCacheConfig {
                enabled: true,
                outcome_capacity: 16,
                raw_capacity: 1,
                ttl_seconds: 600,
                backoff_base_ms: 50,
                backoff_cap_ms: 200,
                backoff_jitter_ms: 0,
            },
            fetcher,
        );

        let url1 = "https://example.com/crl1".to_string();
        let url2 = "https://example.com/crl2".to_string();

        // Use different serials to avoid outcome short-circuiting
        let s1 = [0x01];
        let s2 = [0x02];
        let s3 = [0x03];

        // Fetch url1
        let _ = cache.check_revocation_with_urls(&s1, std::slice::from_ref(&url1));
        // Fetch url2 causing url1 to be evicted (raw cap = 1)
        let _ = cache.check_revocation_with_urls(&s2, std::slice::from_ref(&url2));
        // Now access url1 again with a new serial so outcome doesn't hit; should refetch
        let _ = cache.check_revocation_with_urls(&s3, std::slice::from_ref(&url1));

        let calls = *counter.lock().unwrap();
        assert_eq!(calls, 3, "eviction should force a refetch of url1");
    }

    #[test]
    fn ttl_expiry_refetches() {
        let (fetcher, counter) = CountingFetcher::new(vec![0x30, 0x00]);
        let cache = CrlCache::new(
            CrlCacheConfig {
                enabled: true,
                outcome_capacity: 16,
                raw_capacity: 4,
                ttl_seconds: 600,
                backoff_base_ms: 50,
                backoff_cap_ms: 200,
                backoff_jitter_ms: 0,
            },
            fetcher,
        );

        let url = "https://example.com/crl".to_string();
        let s1 = [0x10];
        let s2 = [0x11];

        // First fetch
        let _ = cache.check_revocation_with_urls(&s1, std::slice::from_ref(&url));

        // Force expiry by mutating the cached entry
        if let Ok(mut raw) = cache.raw_crls.lock()
            && let Some(entry) = raw.get_mut(&url)
        {
            entry.expires_at = Instant::now() - Duration::from_secs(1);
        }

        // Second call with new serial should refetch due to expiry
        let _ = cache.check_revocation_with_urls(&s2, std::slice::from_ref(&url));

        let calls = *counter.lock().unwrap();
        assert_eq!(calls, 2, "expired entry should be refetched");
    }

    struct FlakyFetcher {
        count: Arc<Mutex<usize>>,
        success_payload: Vec<u8>,
    }

    impl FlakyFetcher {
        fn new(success_payload: Vec<u8>) -> (Self, Arc<Mutex<usize>>) {
            let counter = Arc::new(Mutex::new(0usize));
            (
                Self {
                    count: counter.clone(),
                    success_payload,
                },
                counter,
            )
        }
    }

    impl CrlFetcher for FlakyFetcher {
        fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
            let mut g = self.count.lock().unwrap();
            *g += 1;
            if *g == 1 {
                Err("network error".to_string())
            } else {
                Ok(self.success_payload.clone())
            }
        }
    }

    #[test]
    fn backoff_applies_after_failure() {
        let (fetcher, _counter) = FlakyFetcher::new(vec![0x30, 0x00]);
        let cache = CrlCache::new(
            CrlCacheConfig {
                enabled: true,
                outcome_capacity: 16,
                raw_capacity: 4,
                ttl_seconds: 600,
                backoff_base_ms: 50,
                backoff_cap_ms: 50,
                backoff_jitter_ms: 0,
            },
            fetcher,
        );

        let url = "https://example.com/crl".to_string();
        let serial = [0xDE, 0xAD];

        // First call fails fast (no backoff applied before first failure recorded)
        let start1 = Instant::now();
        let _ = cache.check_revocation_with_urls(&serial, std::slice::from_ref(&url));
        let elapsed1 = start1.elapsed();
        assert!(elapsed1 < Duration::from_millis(40));

        // Second call should sleep ~50ms due to backoff
        let start2 = Instant::now();
        let _ = cache.check_revocation_with_urls(&serial, std::slice::from_ref(&url));
        let elapsed2 = start2.elapsed();
        assert!(
            elapsed2 >= Duration::from_millis(45),
            "expected backoff delay, got {:?}",
            elapsed2
        );
    }
}
