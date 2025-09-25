use crate::crl::config::CrlConfig;
use crate::crl::error::{CrlDownloadSnafu, CrlError, MutexPoisonedSnafu};
use chrono::{DateTime, Utc};
use once_cell::sync::OnceCell;
use opentelemetry::metrics::{Counter, Histogram, Meter};
use opentelemetry::{KeyValue, global};
use sha2::{Digest, Sha256};
use snafu::ResultExt;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct CachedCrl {
    pub crl: Vec<u8>,
    pub download_time: DateTime<Utc>,
    pub url: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct CrlCache {
    config: CrlConfig,
    memory_cache: Option<Arc<Mutex<HashMap<String, CachedCrl>>>>,
    url_locks: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
    backoff: Arc<Mutex<HashMap<String, (u32, std::time::Instant)>>>,
    http_client: reqwest::Client,
}

#[derive(Debug, Clone)]
struct CrlMetrics {
    get_total: Counter<u64>,
    get_ms: Histogram<u64>,
    fetch_total: Counter<u64>,
    fetch_ms: Histogram<u64>,
}

impl CrlMetrics {
    fn init(meter: &Meter) -> Self {
        Self {
            get_total: meter.u64_counter("crl_get_total").build(),
            get_ms: meter.u64_histogram("crl_get_ms").build(),
            fetch_total: meter.u64_counter("crl_fetch_total").build(),
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
    pub async fn check_revocation(
        &self,
        cert_der: &[u8],
        issuer_der: Option<&[u8]>,
    ) -> Result<crate::tls::revocation::RevocationOutcome, crate::tls::revocation::RevocationError>
    {
        use crate::tls::revocation::RevocationOutcome;
        // Extract CRL URLs
        let crl_urls = crate::crl::certificate_parser::extract_crl_distribution_points(cert_der)
            .context(crate::tls::revocation::DistributionPointsSnafu)?;
        if crl_urls.is_empty() {
            return Ok(RevocationOutcome::NotDetermined);
        }
        // Get certificate serial
        let serial = crate::crl::certificate_parser::get_certificate_serial_number(cert_der)
            .context(crate::tls::revocation::CrlOperationSnafu)?;
        // Try URLs
        for url in crl_urls.iter() {
            let bytes = self
                .get(url)
                .await
                .context(crate::tls::revocation::CrlOperationSnafu)?;
            if let Err(_e) =
                crate::tls::x509_utils::verify_crl_signature_best_effort(&bytes, issuer_der)
            {
                continue;
            }
            let is_revoked =
                crate::crl::certificate_parser::check_certificate_in_crl(&serial, &bytes)
                    .context(crate::tls::revocation::CrlOperationSnafu)?;
            if is_revoked {
                return Ok(RevocationOutcome::Revoked {
                    reason: None,
                    revocation_time: None,
                });
            }
        }
        Ok(RevocationOutcome::NotRevoked)
    }
    pub fn new(config: CrlConfig) -> Result<Self, CrlError> {
        let memory_cache = if config.enable_memory_caching {
            Some(Arc::new(Mutex::new(HashMap::new())))
        } else {
            None
        };
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(
                config.http_timeout.num_seconds() as u64,
            ))
            .connect_timeout(std::time::Duration::from_secs(
                config.connection_timeout.num_seconds() as u64,
            ))
            .build()
            .context(crate::crl::error::HttpClientBuildSnafu)?;

        Ok(Self {
            config,
            memory_cache,
            url_locks: Arc::new(Mutex::new(HashMap::new())),
            backoff: Arc::new(Mutex::new(HashMap::new())),
            http_client,
        })
    }

    pub fn global(config: CrlConfig) -> &'static Arc<CrlCache> {
        static INSTANCE: OnceCell<Arc<CrlCache>> = OnceCell::new();
        INSTANCE.get_or_init(|| {
            let cache = match CrlCache::new(config) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!(target: "sf_core::crl", "Failed to initialize CRL cache: {e}. Falling back to default config");
                    match CrlCache::new(CrlConfig::default()) {
                        Ok(c2) => c2,
                        Err(e2) => {
                            tracing::error!(target: "sf_core::crl", "Failed to initialize fallback CRL cache: {e2}. Using minimal no-op cache.");
                            CrlCache {
                                config: CrlConfig::default(),
                                memory_cache: None,
                                url_locks: Arc::new(Mutex::new(HashMap::new())),
                                backoff: Arc::new(Mutex::new(HashMap::new())),
                                http_client: reqwest::Client::new(),
                            }
                        }
                    }
                }
            };
            Arc::new(cache)
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
            if let Some(entry) = cache.get(url)
                && Utc::now() <= entry.expires_at
            {
                return Ok(Some(entry.clone()));
            }
            cache.remove(url);
        }
        Ok(None)
    }

    pub fn put(&self, cached_crl: CachedCrl) -> Result<(), CrlError> {
        if let Some(memory) = &self.memory_cache
            && let Ok(mut cache) = memory.lock()
        {
            cache.insert(cached_crl.url.clone(), cached_crl);
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
        let lock = self.get_url_lock(url)?;
        let _guard = lock.lock().await;
        if let Some(mem) = self.get_cached(url)? {
            return Ok(mem.crl);
        }

        // Fetch and optionally persist while holding the per-URL lock to avoid duplicate downloads
        let fetched = self.fetch(url).await?;
        if self.config.enable_disk_caching
            && let Some(dir) = self.config.get_cache_dir()
        {
            if let Err(e) = std::fs::create_dir_all(&dir) {
                tracing::warn!(
                    target: "sf_core::crl",
                    "Failed to create CRL cache directory {}: {}",
                    dir.display(), e
                );
            }
            let file_name = Self::url_digest(url);
            let path = dir.join(file_name);
            if let Err(e) = std::fs::write(&path, &fetched) {
                tracing::warn!(
                    target: "sf_core::crl",
                    "Failed to write CRL cache to disk at {}: {}",
                    path.display(), e
                );
            }
        }
        let expires_at = match crate::tls::x509_utils::extract_crl_next_update(&fetched) {
            Ok(Some(dt)) => dt,
            _ => Utc::now() + self.config.validity_time,
        };
        if let Err(e) = self.put(CachedCrl {
            crl: fetched.clone(),
            download_time: Utc::now(),
            url: url.to_string(),
            expires_at,
        }) {
            tracing::warn!(
                target: "sf_core::crl",
                "Failed to put CRL into memory cache for url {}: {}",
                url, e
            );
        }
        let ms = start.elapsed().as_millis() as u64;
        metrics()
            .get_ms
            .record(ms, &[KeyValue::new("source", "network")]);
        metrics()
            .get_total
            .add(1, &[KeyValue::new("source", "network")]);
        Ok(fetched)
    }

    fn get_url_lock(&self, url: &str) -> Result<Arc<tokio::sync::Mutex<()>>, CrlError> {
        let mut locks = self.url_locks.lock().map_err(|e| {
            MutexPoisonedSnafu {
                message: format!("url_locks map poisoned: {e}"),
            }
            .build()
        })?;
        Ok(locks
            .entry(url.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone())
    }

    async fn fetch(&self, url: &str) -> Result<Vec<u8>, CrlError> {
        let start = std::time::Instant::now();
        self.maybe_sleep_backoff(url).await?;
        let resp = self
            .http_client
            .get(url)
            .send()
            .await
            .context(CrlDownloadSnafu {
                url: url.to_string(),
            })?;
        let resp = resp.error_for_status().context(CrlDownloadSnafu {
            url: url.to_string(),
        })?;
        let bytes = resp.bytes().await.context(CrlDownloadSnafu {
            url: url.to_string(),
        })?;
        self.record_backoff_success(url)?;
        let ms = start.elapsed().as_millis() as u64;
        metrics().fetch_ms.record(ms, &[]);
        metrics().fetch_total.add(1, &[]);
        Ok(bytes.to_vec())
    }

    async fn maybe_sleep_backoff(&self, url: &str) -> Result<(), CrlError> {
        let (failures, last) = {
            let guard = self.backoff.lock().map_err(|e| {
                MutexPoisonedSnafu {
                    message: format!("backoff map poisoned: {e}"),
                }
                .build()
            })?;
            guard
                .get(url)
                .cloned()
                .unwrap_or((0, std::time::Instant::now()))
        };
        if failures == 0 {
            return Ok(());
        }
        let base_ms = 100u64;
        let cap_ms = 5_000u64;
        let exp: u32 = failures.min(5u32);
        let factor = 1u64.checked_shl(exp).unwrap_or(u64::MAX);
        let delay_ms = base_ms.saturating_mul(factor).min(cap_ms);
        let jitter = (rand::random::<u32>() % 100) as u64;
        let total_ms = delay_ms + jitter;
        let elapsed = last.elapsed();
        let needed = std::time::Duration::from_millis(total_ms);
        if elapsed < needed {
            tokio::time::sleep(needed - elapsed).await;
        }
        Ok(())
    }

    fn record_backoff_success(&self, url: &str) -> Result<(), CrlError> {
        let mut guard = self.backoff.lock().map_err(|e| {
            MutexPoisonedSnafu {
                message: format!("backoff map poisoned: {e}"),
            }
            .build()
        })?;
        guard.remove(url);
        Ok(())
    }
}
