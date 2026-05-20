use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tokio::sync::Mutex;

use super::connection::Connection;
use super::database::Database;
use super::result_set::ResultSet;
use super::statement::Statement;
use crate::fs_adapter::{FsAdapter, RealFs};
use crate::handle_manager::{Handle, HandleManager};
use crate::logging::LogManager;
use crate::telemetry::platform_detection::{DetectionConfig, detect_platforms};
use crate::telemetry::snowflake_exporter::SessionRegistry;
use crate::token_cache::{KeyringTokenCache, TokenCacheError};

/// Which shape the PUT/GET result set should take.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum PutGetResultsetFlavor {
    #[default]
    Python,
    Odbc,
}

/// Immutable behavioural presets declared by each wrapper (Python, ODBC, JDBC)
/// at startup. These are **not** exposed to end users — they capture
/// compile-time / init-time differences between wrappers so that shared Rust
/// code can branch on them without hard-coding wrapper knowledge everywhere.
#[derive(Debug, Clone, Default)]
pub struct WrapperPresets {
    pub put_get_resultset_flavor: PutGetResultsetFlavor,
}

impl WrapperPresets {
    /// Presets for the Python connector.
    ///
    /// Currently identical to `Default` — listed explicitly so that
    /// future Python-specific overrides have a clear home.
    pub fn python() -> Self {
        Self::default()
    }

    /// Presets for the ODBC driver.
    #[allow(clippy::needless_update)]
    pub fn odbc() -> Self {
        Self {
            put_get_resultset_flavor: PutGetResultsetFlavor::Odbc,
            ..Self::default()
        }
    }

    /// Presets for the JDBC bridge.
    pub fn jdbc() -> Self {
        Self::default()
    }
}

/// Injection points for `DatabaseDriverV1`.
///
/// Each field is optional; `None` means "use the production default".
/// Add a new field (plus a default in `DatabaseDriverV1::new`) whenever
/// a new provider becomes injectable — call sites that use
/// `..Default::default()` won't need to change.
#[derive(Default)]
pub struct DriverProviders {
    pub fs: Option<Arc<dyn FsAdapter>>,
    /// `LogManager` instance created during logging initialization.
    /// Owns the `SdkTracerProvider`, `SessionRegistry`, and OS details.
    pub log_manager: Option<LogManager>,
    pub wrapper_presets: WrapperPresets,
}

pub struct DatabaseDriverV1 {
    pub(super) databases: HandleManager<Mutex<Database>>,
    pub(super) connections: HandleManager<Mutex<Connection>>,
    pub(super) statements: HandleManager<Mutex<Statement>>,
    pub(super) results: HandleManager<Mutex<ResultSet>>,
    /// Cached `connection_handle.id → Snowflake session_id` populated on login
    /// and removed on connection release. Keeping this off the `Connection`
    /// struct (which is behind a `tokio::Mutex`) lets entry-point methods
    /// resolve the session id without taking the connection mutex — the only
    /// writers are login / logout, so reads under the parallel `RwLock` are
    /// uncontended in normal operation.
    pub(super) session_ids: RwLock<HashMap<u64, i64>>,
    /// Cached `statement_handle.id → owning connection_handle.id` populated
    /// when a statement is created and removed on statement release. Lets
    /// statement-scoped entry points map to their connection's session id
    /// without locking the statement mutex.
    pub(super) stmt_to_conn: RwLock<HashMap<u64, u64>>,
    token_cache: once_cell::sync::OnceCell<KeyringTokenCache>,
    fs: Arc<dyn FsAdapter>,
    platforms: tokio::sync::OnceCell<Vec<String>>,
    log_manager: Option<LogManager>,
    pub(super) wrapper_presets: WrapperPresets,
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
            results: HandleManager::new(),
            session_ids: RwLock::new(HashMap::new()),
            stmt_to_conn: RwLock::new(HashMap::new()),
            token_cache: once_cell::sync::OnceCell::new(),
            fs: providers.fs.unwrap_or_else(|| Arc::new(RealFs)),
            platforms: tokio::sync::OnceCell::const_new(),
            log_manager: providers.log_manager,
            wrapper_presets: providers.wrapper_presets,
        }
    }

    /// Returns the session registry if telemetry was configured via `DriverProviders`.
    pub(super) fn telemetry_sessions(&self) -> Option<&SessionRegistry> {
        self.log_manager.as_ref().map(|lm| lm.telemetry_sessions())
    }

    /// Record a connection's Snowflake session id. Called once on successful
    /// login so subsequent entry-point methods can stamp `snowflake.session.id`
    /// on their telemetry spans without locking the connection mutex.
    pub(crate) fn register_session_id(&self, conn_handle: Handle, session_id: i64) {
        self.session_ids
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(conn_handle.id, session_id);
    }

    /// Forget a connection's session id. Called during connection release so a
    /// re-used handle id never resolves to a stale session.
    pub(crate) fn deregister_session_id(&self, conn_handle: Handle) {
        self.session_ids
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&conn_handle.id);
    }

    /// Resolve the Snowflake session id for a connection handle. Returns
    /// `None` when login has not populated the cache (or after release). Reads
    /// take only the parallel-map's read lock — does **not** touch the
    /// connection mutex, so it can be called while a query is executing on
    /// the same connection without contending.
    pub(crate) fn session_id_for_conn(&self, conn_handle: Handle) -> Option<i64> {
        self.session_ids
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&conn_handle.id)
            .copied()
    }

    /// Record a statement's owning connection handle id. Called when the
    /// statement is created so [`Self::session_id_for_stmt`] can resolve
    /// without locking the statement mutex.
    pub(crate) fn register_stmt_conn(&self, stmt_handle: Handle, conn_handle: Handle) {
        self.stmt_to_conn
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(stmt_handle.id, conn_handle.id);
    }

    /// Forget a statement's owning connection mapping. Called on statement
    /// release so a re-used handle id never resolves to a stale connection.
    pub(crate) fn deregister_stmt_conn(&self, stmt_handle: Handle) {
        self.stmt_to_conn
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&stmt_handle.id);
    }

    /// Resolve the Snowflake session id for a statement handle by reading the
    /// parallel `stmt → conn_handle` map then the parallel `conn_handle →
    /// session_id` map. Same lock-free properties as
    /// [`Self::session_id_for_conn`] — no statement / connection mutex taken.
    pub(crate) fn session_id_for_stmt(&self, stmt_handle: Handle) -> Option<i64> {
        let conn_handle_id = self
            .stmt_to_conn
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&stmt_handle.id)
            .copied()?;
        self.session_ids
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&conn_handle_id)
            .copied()
    }

    /// Flush buffered telemetry spans for a specific session.
    pub(super) async fn flush_telemetry_session(&self, session_id: i64) {
        if let Some(ref lm) = self.log_manager {
            lm.flush_session(session_id).await;
        }
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

    pub fn os_details(&self) -> Option<&HashMap<String, String>> {
        self.log_manager
            .as_ref()
            .and_then(|lm| lm.os_details().as_ref())
    }

    /// Process-wide default for `log_query_text`, sourced from the
    /// `LogManager` if one was injected (e.g. parsed from `sf.odbc.ini` or the
    /// `[log]` TOML section). `None` means "no global default; let the param
    /// registry default win".
    pub(crate) fn log_query_text(&self) -> Option<bool> {
        self.log_manager.as_ref().and_then(|lm| lm.log_query_text())
    }

    /// Process-wide default for `log_query_parameters`. See
    /// [`Self::log_query_text`] for precedence semantics.
    pub(crate) fn log_query_parameters(&self) -> Option<bool> {
        self.log_manager
            .as_ref()
            .and_then(|lm| lm.log_query_parameters())
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
