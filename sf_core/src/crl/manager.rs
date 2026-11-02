use crate::crl::cache::{BackgroundRefresher, CrlCache};
use crate::crl::config::CrlConfig;
use crate::crl::error::CrlError;
use crate::crl::stats_buffer::CrlStatsBuffer;
use crate::crl::worker::{CrlWorker, SharedCrlWorker};
use crate::telemetry::session_telemetry::SessionTelemetry;
use crate::tls::RootStoreSnapshot;
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

    pub(crate) fn has_cache(&self) -> bool {
        self.0.cache.get().is_some()
    }

    /// Returns the cache for this manager. The first `config` and `roots` win.
    /// A later call returns that cache and starts the refresher when it is not running.
    pub(crate) fn cache(
        &self,
        config: CrlConfig,
        roots: &RootStoreSnapshot<'_>,
    ) -> Result<Arc<CrlCache>, CrlError> {
        let cache = self
            .0
            .cache
            .get_or_try_init(|| CrlCache::open_cache(config, roots))?;
        let mut slot = self.0.refresher.lock_recover();
        if slot.is_none() {
            *slot = CrlCache::spawn_background_refresher(Arc::clone(cache));
        }
        Ok(Arc::clone(cache))
    }

    /// A copy that shares this manager's worker and caches and carries a new
    /// `client_crl_stats` buffer. Clones of the returned handle share that buffer.
    pub fn connection_crl(&self) -> ConnectionCrl {
        ConnectionCrl {
            manager: self.clone(),
            stats: CrlStatsBuffer::new(),
        }
    }
}

/// The driver's [`CrlManager`] plus the `client_crl_stats` buffer for one connection.
///
/// [`Deref`] forwards cache and worker calls to the shared manager. Cloning keeps
/// both the manager and this connection's buffer.
#[derive(Clone, Debug)]
pub struct ConnectionCrl {
    manager: CrlManager,
    stats: CrlStatsBuffer,
}

impl ConnectionCrl {
    pub(crate) fn stats(&self) -> CrlStatsBuffer {
        self.stats.clone()
    }

    pub(crate) fn flush_crl_stats(&self, telemetry: &SessionTelemetry, telemetry_id: u64) {
        self.stats.flush_to(telemetry, telemetry_id);
    }

    pub(crate) fn discard_crl_stats(&self) {
        self.stats.discard();
    }

    #[cfg(test)]
    pub(crate) fn pending_crl_stats(&self) -> Vec<crate::crl::stats_buffer::CrlStat> {
        self.stats.pending()
    }
}

impl std::ops::Deref for ConnectionCrl {
    type Target = CrlManager;

    fn deref(&self) -> &Self::Target {
        &self.manager
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
    use crate::tls::RootStoreProvider;

    fn memory_only() -> CrlConfig {
        CrlConfig {
            enable_memory_caching: true,
            enable_disk_caching: false,
            ..CrlConfig::default()
        }
    }

    #[test]
    fn connection_crl_shares_the_worker_and_keeps_its_own_stats() {
        let driver = CrlManager::new();
        let account_a = driver.connection_crl();
        let account_b = driver.connection_crl();
        assert!(Arc::ptr_eq(&account_a.worker(), &account_b.worker()));

        let telemetry_a = crate::crl::stats_buffer::disconnected_session_telemetry(1);
        account_a.flush_crl_stats(&telemetry_a, 1);
        account_b
            .stats()
            .record("http://crl.snowflake.com/b.crl", None, None, None, None);

        assert!(telemetry_a.raw_log_messages(1).is_empty());
        let pending_b = account_b.pending_crl_stats();
        let [stat] = pending_b.as_slice() else {
            panic!("expected one parked event for account B, got {pending_b:?}");
        };
        assert_eq!(stat.url, "http://crl.snowflake.com/b.crl");

        let cloned = account_b.clone();
        cloned
            .stats()
            .record("http://crl.snowflake.com/b2.crl", None, None, None, None);
        assert_eq!(account_b.pending_crl_stats().len(), 2);
    }

    #[test]
    fn repeated_worker_calls_share_one_handle() {
        let manager = CrlManager::new();
        assert!(Arc::ptr_eq(&manager.worker(), &manager.worker()));
    }

    #[test]
    fn first_config_wins_and_starts_one_refresher() {
        let manager = CrlManager::new();
        let first = manager
            .cache(memory_only(), &RootStoreProvider::new().snapshot())
            .expect("first cache");
        first.wait_for_scheduler(true);
        let second = manager
            .cache(
                CrlConfig {
                    enable_memory_caching: false,
                    enable_disk_caching: false,
                    ..CrlConfig::default()
                },
                &RootStoreProvider::new().snapshot(),
            )
            .expect("second cache");
        assert!(Arc::ptr_eq(&first, &second));
        assert!(first.scheduler_is_published());
    }

    #[test]
    fn disabled_caching_does_not_publish_a_scheduler() {
        let manager = CrlManager::new();
        let cache = manager
            .cache(
                CrlConfig {
                    enable_memory_caching: false,
                    enable_disk_caching: false,
                    ..CrlConfig::default()
                },
                &RootStoreProvider::new().snapshot(),
            )
            .expect("cache");
        assert!(!cache.scheduler_is_published());
    }

    #[test]
    fn last_manager_drop_joins_refresher_while_cache_remains() {
        let manager = CrlManager::new();
        let cache = manager
            .cache(memory_only(), &RootStoreProvider::new().snapshot())
            .expect("cache");
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
        let first_cache = first
            .cache(memory_only(), &RootStoreProvider::new().snapshot())
            .expect("first cache");
        first_cache.wait_for_scheduler(true);
        drop(first);
        first_cache.wait_for_scheduler(false);

        let second = CrlManager::new();
        let second_cache = second
            .cache(memory_only(), &RootStoreProvider::new().snapshot())
            .expect("second cache");
        second_cache.wait_for_scheduler(true);
        assert!(!Arc::ptr_eq(&first_cache, &second_cache));
        drop(second);
        second_cache.wait_for_scheduler(false);
    }
}
