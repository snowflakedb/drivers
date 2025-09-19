use crate::crl::config::CrlConfig;
use crate::crl::error::CrlError;
use chrono::{DateTime, Utc};
use lru::LruCache;
use once_cell::sync::OnceCell;
use opentelemetry::metrics::{Counter, Histogram, Meter};
use opentelemetry::{KeyValue, global};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct CachedCrl {
    pub crl: Vec<u8>,
    pub download_time: DateTime<Utc>,
    pub url: String,
}

#[derive(Debug)]
pub struct CrlCache {
    config: CrlConfig,
    memory_cache: Option<Arc<Mutex<LruCache<String, CachedCrl>>>>,
    url_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    backoff: Arc<Mutex<HashMap<String, (u32, std::time::Instant)>>>,
}

// Outcome memoization removed for now; can be reintroduced if needed

#[derive(Debug, Clone)]
struct CrlMetrics {
    get_total: Counter<u64>,
    get_ms: Histogram<u64>,
    fetch_total: Counter<u64>,
    fetch_error_total: Counter<u64>,
    fetch_ms: Histogram<u64>,
}

impl CrlMetrics {
    fn init(meter: &Meter) -> Self {
        Self {
            get_total: meter.u64_counter("crl_get_total").build(),
            get_ms: meter.u64_histogram("crl_get_ms").build(),
            fetch_total: meter.u64_counter("crl_fetch_total").build(),
            fetch_error_total: meter.u64_counter("crl_fetch_error_total").build(),
            fetch_ms: meter.u64_histogram("crl_fetch_ms").build(),
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
        Ok(Self {
            config,
            memory_cache,
            url_locks: Arc::new(Mutex::new(HashMap::new())),
            backoff: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn global(config: CrlConfig, memory_capacity: usize) -> &'static Arc<CrlCache> {
        static INSTANCE: OnceCell<Arc<CrlCache>> = OnceCell::new();
        INSTANCE.get_or_init(|| {
            Arc::new(CrlCache::new(config, memory_capacity).expect("init CrlCache"))
        })
    }

    pub fn url_digest(url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let digest = hasher.finalize();
        hex::encode(digest)
    }

    pub fn get_cached(&self, url: &str) -> Result<Option<CachedCrl>, CrlError> {
        if let Some(memory) = &self.memory_cache
            && let Ok(mut cache) = memory.lock()
        {
            return Ok(cache.get(url).cloned());
        }
        Ok(None)
    }

    pub fn put(&self, cached_crl: CachedCrl) -> Result<(), CrlError> {
        if let Some(memory) = &self.memory_cache
            && let Ok(mut cache) = memory.lock()
        {
            cache.put(cached_crl.url.clone(), cached_crl);
        }
        Ok(())
    }

    pub async fn get(&self, url: &str) -> Result<Vec<u8>, CrlError> {
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

        // Fetch and optionally persist
        drop(guard);
        let fetched = self.fetch(url).await?;
        if self.config.enable_disk_caching
            && let Some(dir) = self.config.get_cache_dir()
        {
            let _ = std::fs::create_dir_all(&dir);
            let file_name = Self::url_digest(url);
            let path = dir.join(file_name);
            let _ = std::fs::write(&path, &fetched);
        }
        let _ = self.put(CachedCrl {
            crl: fetched.clone(),
            download_time: Utc::now(),
            url: url.to_string(),
        });
        let ms = start.elapsed().as_millis() as u64;
        metrics()
            .get_ms
            .record(ms, &[KeyValue::new("source", "network")]);
        metrics()
            .get_total
            .add(1, &[KeyValue::new("source", "network")]);
        Ok(fetched)
    }

    fn get_url_lock(&self, url: &str) -> Arc<Mutex<()>> {
        let mut locks = self.url_locks.lock().unwrap();
        locks
            .entry(url.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    async fn fetch(&self, url: &str) -> Result<Vec<u8>, CrlError> {
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
            metrics().fetch_error_total.add(1, &[]);
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
        metrics().fetch_ms.record(ms, &[]);
        metrics().fetch_total.add(1, &[]);
        Ok(bytes.to_vec())
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

// Tests removed in this branch; CRL behavior covered by integration tests.
