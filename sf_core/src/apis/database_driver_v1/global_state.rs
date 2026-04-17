use tokio::sync::Mutex;

use super::connection::Connection;
use super::database::Database;
use super::statement::Statement;
use crate::handle_manager::HandleManager;
use crate::telemetry::platform_detection::{DetectionConfig, detect_platforms};
use crate::token_cache::{KeyringTokenCache, TokenCacheError};

#[derive(Default)]
pub struct DatabaseDriverV1 {
    pub(super) databases: HandleManager<Mutex<Database>>,
    pub(super) connections: HandleManager<Mutex<Connection>>,
    pub(super) statements: HandleManager<Mutex<Statement>>,
    token_cache: once_cell::sync::OnceCell<KeyringTokenCache>,
    // Detected once per driver lifetime: first `connection_init` pays up to
    // `DETECTION_TIMEOUT` of extra latency, subsequent connections clone the
    // cached `Vec<String>` for free. Concurrent first-callers coalesce via
    // `tokio::sync::OnceCell::get_or_init`.
    detected_platforms: tokio::sync::OnceCell<Vec<String>>,
}

impl DatabaseDriverV1 {
    pub const fn new() -> Self {
        Self {
            databases: HandleManager::new(),
            connections: HandleManager::new(),
            statements: HandleManager::new(),
            token_cache: once_cell::sync::OnceCell::new(),
            detected_platforms: tokio::sync::OnceCell::const_new(),
        }
    }

    pub fn token_cache(&self) -> Result<&KeyringTokenCache, TokenCacheError> {
        self.token_cache.get_or_try_init(KeyringTokenCache::new)
    }

    /// Lazy-initialize and return the list of detected CLIENT_ENVIRONMENT
    /// platforms. Safe to call from multiple connections concurrently.
    pub async fn detected_platforms(&self) -> &Vec<String> {
        self.detected_platforms
            .get_or_init(|| async { detect_platforms(&DetectionConfig::default()).await })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static DRIVER_STATE: DatabaseDriverV1 = DatabaseDriverV1::new();

    #[test]
    fn token_cache_lazy_init_succeeds() {
        let result = DRIVER_STATE.token_cache();
        assert!(
            result.is_ok(),
            "token_cache() should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn token_cache_returns_same_instance() {
        let first = DRIVER_STATE.token_cache().expect("first call failed");
        let second = DRIVER_STATE.token_cache().expect("second call failed");
        assert!(
            std::ptr::eq(first, second),
            "token_cache() should return the same instance on repeated calls"
        );
    }

    #[test]
    fn driver_state_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DatabaseDriverV1>();
    }

    // Platform detection tests use a dedicated driver instance so the
    // `OnceCell` state doesn't bleed between assertions. The cache check
    // mutates env vars *after* the first call and verifies the second call
    // returns the same cached result rather than re-running detection.
    #[tokio::test(flavor = "multi_thread")]
    async fn detected_platforms_is_cached_across_calls() {
        let driver = DatabaseDriverV1::new();

        let first = temp_env::async_with_vars(
            [
                ("SNOWFLAKE_DISABLE_PLATFORM_DETECTION", Some("true")),
                ("LAMBDA_TASK_ROOT", None),
            ],
            async { driver.detected_platforms().await.clone() },
        )
        .await;

        assert_eq!(first, vec!["disabled".to_string()]);

        let second = temp_env::async_with_vars(
            [
                ("SNOWFLAKE_DISABLE_PLATFORM_DETECTION", None::<&str>),
                ("LAMBDA_TASK_ROOT", Some("/var/task")),
            ],
            async { driver.detected_platforms().await.clone() },
        )
        .await;

        assert_eq!(
            first, second,
            "second call must return the cached result, not re-run detection"
        );
    }
}
