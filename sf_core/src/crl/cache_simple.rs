// Simplified CRL cache implementation that compiles
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
pub struct SimpleCrlCache {
    memory_cache: Option<Arc<Mutex<LruCache<String, CachedCrl>>>>,
    url_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl SimpleCrlCache {
    pub fn new(enable_memory: bool, memory_capacity: usize) -> Result<Self, CrlError> {
        let memory_cache = if enable_memory {
            Some(Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(memory_capacity).unwrap_or(NonZeroUsize::new(100).unwrap()),
            ))))
        } else {
            None
        };

        Ok(Self {
            memory_cache,
            url_locks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Global singleton cache accessor
    pub fn global(enable_memory: bool, memory_capacity: usize) -> &'static Arc<SimpleCrlCache> {
        static INSTANCE: OnceCell<Arc<SimpleCrlCache>> = OnceCell::new();
        INSTANCE.get_or_init(|| {
            Arc::new(
                SimpleCrlCache::new(enable_memory, memory_capacity).expect("init SimpleCrlCache"),
            )
        })
    }

    /// Get a lock for the given URL to prevent concurrent downloads
    pub fn get_url_lock(&self, url: &str) -> Arc<Mutex<()>> {
        let mut locks = self.url_locks.lock().unwrap();
        locks
            .entry(url.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Get CRL from memory cache
    pub fn get(&self, url: &str) -> Result<Option<CachedCrl>, CrlError> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_digest_stable() {
        let a = SimpleCrlCache::url_digest("http://example.com/a");
        let b = SimpleCrlCache::url_digest("http://example.com/a");
        let c = SimpleCrlCache::url_digest("http://example.com/b");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64); // sha256 hex
    }

    #[test]
    fn test_memory_cache_put_get() {
        let cache = SimpleCrlCache::new(true, 8).unwrap();
        let url = "http://example.com/test.crl".to_string();
        let entry = CachedCrl {
            crl: vec![1, 2, 3],
            download_time: Utc::now(),
            url: url.clone(),
        };
        cache.put(entry.clone()).unwrap();
        let got = cache.get(&url).unwrap();
        assert!(got.is_some());
        let got = got.unwrap();
        assert_eq!(got.url, url);
        assert_eq!(got.crl, vec![1, 2, 3]);
    }
}
