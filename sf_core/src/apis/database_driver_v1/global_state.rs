use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use super::connection::Connection;
use super::database::Database;
use super::statement::Statement;
use crate::fs_adapter::{FsAdapter, RealFs};
use crate::handle_manager::HandleManager;
use crate::telemetry::os_details::detect_os_details;
use crate::telemetry::platform_detection::{DetectionConfig, detect_platforms};
use crate::telemetry::snowflake_exporter::SessionRegistry;
use crate::token_cache::{KeyringTokenCache, TokenCacheError};

/// Injection points for `DatabaseDriverV1`.
///
/// Each field is optional; `None` means "use the production default".
/// Add a new field (plus a default in `DatabaseDriverV1::new`) whenever
/// a new provider becomes injectable — call sites that use
/// `..Default::default()` won't need to change.
#[derive(Default)]
pub struct DriverProviders {
    pub fs: Option<Arc<dyn FsAdapter>>,
    /// Session registry shared with the Snowflake telemetry exporter layer.
    /// Created by the initialization code that calls `init_logging` and passed
    /// here so `DatabaseDriverV1` can register/deregister sessions.
    pub telemetry_sessions: Option<SessionRegistry>,
    /// The `SdkTracerProvider` returned by `init_logging`. Must be kept alive
    /// for the process lifetime to prevent the exporter from shutting down.
    pub telemetry_provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
}

pub struct DatabaseDriverV1 {
    pub(super) databases: HandleManager<Mutex<Database>>,
    pub(super) connections: HandleManager<Mutex<Connection>>,
    pub(super) statements: HandleManager<Mutex<Statement>>,
    token_cache: once_cell::sync::OnceCell<KeyringTokenCache>,
    fs: Arc<dyn FsAdapter>,
    platforms: tokio::sync::OnceCell<Vec<String>>,
    telemetry_sessions: Option<SessionRegistry>,
    /// Kept alive so the Snowflake exporter is not shut down.
    #[allow(dead_code)]
    telemetry_provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
    os_details: once_cell::sync::OnceCell<Option<HashMap<String, String>>>,
}

impl Default for DatabaseDriverV1 {
    fn default() -> Self {
        Self::new()
    }
}

impl DatabaseDriverV1 {
    pub fn new() -> Self {
        Self::with_providers(DriverProviders::default())
    }

    pub fn with_providers(providers: DriverProviders) -> Self {
        Self {
            databases: HandleManager::new(),
            connections: HandleManager::new(),
            statements: HandleManager::new(),
            token_cache: once_cell::sync::OnceCell::new(),
            fs: providers.fs.unwrap_or_else(|| Arc::new(RealFs)),
            platforms: tokio::sync::OnceCell::const_new(),
            telemetry_sessions: providers.telemetry_sessions,
            telemetry_provider: providers.telemetry_provider,
            os_details: once_cell::sync::OnceCell::new(),
        }
    }

    /// Returns the session registry, checking DriverProviders first then
    /// falling back to the C API telemetry init state (which may be populated
    /// after CApiState initialization).
    pub(super) fn telemetry_sessions(&self) -> Option<&SessionRegistry> {
        self.telemetry_sessions
            .as_ref()
            .or_else(|| {
                crate::logging::c_api::TELEMETRY_INIT
                    .get()
                    .map(|init| &init.sessions)
            })
    }

    pub fn token_cache(&self) -> Result<&KeyringTokenCache, TokenCacheError> {
        self.token_cache.get_or_try_init(KeyringTokenCache::new)
    }

    pub fn fs_adapter(&self) -> Arc<dyn FsAdapter> {
        self.fs.clone()
    }

    pub async fn platforms(&self) -> &Vec<String> {
        self.platforms
            .get_or_init(|| async { detect_platforms(&DetectionConfig::default()).await })
            .await
    }

    pub fn os_details(&self) -> &Option<HashMap<String, String>> {
        self.os_details
            .get_or_init(|| detect_os_details(self.fs.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_cache_lazy_init_succeeds() {
        let driver = DatabaseDriverV1::new();
        let result = driver.token_cache();
        assert!(
            result.is_ok(),
            "token_cache() should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn token_cache_returns_same_instance() {
        let driver = DatabaseDriverV1::new();
        let first = driver.token_cache().expect("first call failed");
        let second = driver.token_cache().expect("second call failed");
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
}
