use crate::crl::cache::{BackgroundRefresher, CrlCache};
use crate::crl::config::CrlConfig;
use crate::crl::error::CrlError;
use crate::crl::worker::{CrlWorker, SharedCrlWorker};
use crate::utils::sync::MutexRecoverExt;
use once_cell::sync::OnceCell;
use std::fmt;
use std::sync::{Arc, Mutex};

/// Owns the CRL worker, the cache, and the `crl-refresh` thread for one driver.
///
/// The last drop cancels and joins the refresher. Clones share that ownership.
/// A [`CrlCache`] clone does not keep the refresher alive.
#[derive(Clone)]
pub struct CrlManager(Arc<ManagerInner>);

struct ManagerInner {
    worker: SharedCrlWorker,
    cache: OnceCell<Arc<CrlCache>>,
    refresher: Mutex<Option<BackgroundRefresher>>,
}

impl CrlManager {
    pub fn new() -> Self {
        Self(Arc::new(ManagerInner {
            worker: CrlWorker::new_lazy(),
            cache: OnceCell::new(),
            refresher: Mutex::new(None),
        }))
    }

    pub fn worker(&self) -> SharedCrlWorker {
        Arc::clone(&self.0.worker)
    }

    /// Returns the cache for this manager. The first `config` wins. A later
    /// call returns that cache and starts the refresher when it is not running.
    pub fn cache(&self, config: CrlConfig) -> Result<Arc<CrlCache>, CrlError> {
        let cache = self
            .0
            .cache
            .get_or_try_init(|| CrlCache::open_cache(config))?;
        let mut slot = self.0.refresher.lock_recover();
        if slot.is_none() {
            *slot = CrlCache::spawn_background_refresher(Arc::clone(cache));
        }
        Ok(Arc::clone(cache))
    }
}

impl Default for CrlManager {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for CrlManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CrlManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_only() -> CrlConfig {
        CrlConfig {
            enable_memory_caching: true,
            enable_disk_caching: false,
            ..CrlConfig::default()
        }
    }

    #[test]
    fn repeated_worker_calls_share_one_handle() {
        let manager = CrlManager::new();
        assert!(Arc::ptr_eq(&manager.worker(), &manager.worker()));
    }

    #[test]
    fn first_config_wins_and_starts_one_refresher() {
        let manager = CrlManager::new();
        let first = manager.cache(memory_only()).expect("first cache");
        first.wait_for_scheduler(true);
        let second = manager
            .cache(CrlConfig {
                enable_memory_caching: false,
                enable_disk_caching: false,
                ..CrlConfig::default()
            })
            .expect("second cache");
        assert!(Arc::ptr_eq(&first, &second));
        assert!(first.scheduler_is_published());
    }

    #[test]
    fn disabled_caching_does_not_publish_a_scheduler() {
        let manager = CrlManager::new();
        let cache = manager
            .cache(CrlConfig {
                enable_memory_caching: false,
                enable_disk_caching: false,
                ..CrlConfig::default()
            })
            .expect("cache");
        assert!(!cache.scheduler_is_published());
    }

    #[test]
    fn last_manager_drop_joins_refresher_while_cache_remains() {
        let manager = CrlManager::new();
        let cache = manager.cache(memory_only()).expect("cache");
        cache.wait_for_scheduler(true);
        let clone = manager.clone();
        drop(manager);
        assert!(cache.scheduler_is_published());
        drop(clone);
        cache.wait_for_scheduler(false);
    }

    #[test]
    fn a_new_manager_starts_its_own_refresher() {
        let first = CrlManager::new();
        let first_cache = first.cache(memory_only()).expect("first cache");
        first_cache.wait_for_scheduler(true);
        drop(first);
        first_cache.wait_for_scheduler(false);

        let second = CrlManager::new();
        let second_cache = second.cache(memory_only()).expect("second cache");
        second_cache.wait_for_scheduler(true);
        assert!(!Arc::ptr_eq(&first_cache, &second_cache));
        drop(second);
        second_cache.wait_for_scheduler(false);
    }
}
