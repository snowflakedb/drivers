use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use super::connection::{Connection, WrapperIdentity};
use super::database::Database;
use super::result_set::ResultSet;
use super::statement::Statement;
use crate::crl::worker::{CrlWorker, SharedCrlWorker};
use crate::fs_adapter::{FsAdapter, RealFs};
use crate::handle_manager::{Handle, HandleManager};
use crate::logging::LogManager;
use crate::rest::snowflake::prompt_lock::PromptLockMap;
use crate::telemetry::platform_detection::{DetectionConfig, detect_platforms};
use crate::telemetry::snowflake_exporter::SessionRegistry;
use crate::token_cache::{KeyringTokenCache, TokenCache, TokenCacheError};

/// Which shape the PUT/GET result set should take.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum PutGetResultsetFlavor {
    #[default]
    Python,
    Odbc,
    Jdbc,
}

/// Immutable behavioural presets declared by each wrapper (Python, ODBC, JDBC)
/// at startup. These are **not** exposed to end users — they capture
/// compile-time / init-time differences between wrappers so that shared Rust
/// code can branch on them without hard-coding wrapper knowledge everywhere.
#[derive(Debug, Clone, Default)]
pub struct WrapperPresets {
    pub put_get_resultset_flavor: PutGetResultsetFlavor,
    /// When true, PUT auto-detect mirrors legacy libsnowflakeclient
    /// behavior: (1) unsupported compression formats are silently
    /// treated as uncompressed instead of erroring, and (2) magic-byte
    /// detection consults a short-prefix table (2-byte gzip, 2-byte
    /// zlib mapped to `Deflate`, 4-byte snowflake brotli marker) ahead
    /// of the `infer` crate.
    pub legacy_odbc_compression_autodetect: bool,
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
            legacy_odbc_compression_autodetect: true,
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
    /// Inject a shared prompt-lock map so that multiple `DatabaseDriverV1`
    /// instances (each created by a separate `SnowflakeTestClient`) serialize
    /// interactive-auth prompts against the same lock entries.  Production code
    /// always uses `..Default::default()` and gets a fresh map.
    pub prompt_locks: Option<Arc<PromptLockMap>>,
    /// Inject a shared lazy CRL worker so that multiple `DatabaseDriverV1`
    /// instances reuse the same background thread. Production code always uses
    /// `..Default::default()` and gets a fresh lazy handle.
    pub crl_worker: Option<SharedCrlWorker>,
}

pub struct DatabaseDriverV1 {
    pub(super) databases: HandleManager<Mutex<Database>>,
    pub(super) connections: HandleManager<Mutex<Connection>>,
    pub(super) statements: HandleManager<Mutex<Statement>>,
    pub(super) results: HandleManager<Mutex<ResultSet>>,
    token_cache: once_cell::sync::OnceCell<Arc<dyn TokenCache>>,
    fs: Arc<dyn FsAdapter>,
    platforms: tokio::sync::OnceCell<Vec<String>>,
    log_manager: Option<LogManager>,
    pub(super) wrapper_presets: WrapperPresets,
    /// Process-global per-[`crate::token_cache::CacheKey`] prompt locks
    /// (scoped by idp, snowflake, username, role, and token_type).
    /// Shared across all connections on this driver instance.
    pub(crate) prompt_locks: Arc<PromptLockMap>,
    /// Lazy CRL worker shared across all connections on this driver instance.
    pub(crate) crl_worker: SharedCrlWorker,
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
            token_cache: once_cell::sync::OnceCell::new(),
            fs: providers.fs.unwrap_or_else(|| Arc::new(RealFs)),
            platforms: tokio::sync::OnceCell::const_new(),
            log_manager: providers.log_manager,
            wrapper_presets: providers.wrapper_presets,
            prompt_locks: providers
                .prompt_locks
                .unwrap_or_else(|| Arc::new(std::sync::Mutex::new(HashMap::new()))),
            crl_worker: providers.crl_worker.unwrap_or_else(CrlWorker::new_lazy),
        }
    }

    /// Returns the session registry if telemetry was configured via `DriverProviders`.
    pub(super) fn telemetry_sessions(&self) -> Option<&SessionRegistry> {
        self.log_manager.as_ref().map(|lm| lm.telemetry_sessions())
    }

    /// Resolve the Snowflake session id for a connection handle by reading
    /// `Connection::session_id` under the connection mutex. Returns `None`
    /// when the handle is unknown, login has not completed, or the connection
    /// has been released.
    pub(crate) async fn session_id_for_conn(&self, conn_handle: Handle) -> Option<i64> {
        let conn_ptr = self.connections.get_obj(conn_handle)?;
        let conn = conn_ptr.lock().await;
        conn.session_id
    }

    /// Read both `session_id` and `wrapper_identity` under a single lock guard,
    /// eliminating the TOCTOU window that exists when the two fields are fetched
    /// with separate awaits.
    pub(crate) async fn session_id_and_identity_for_conn(
        &self,
        conn_handle: Handle,
    ) -> (Option<i64>, Option<WrapperIdentity>) {
        let Some(conn_ptr) = self.connections.get_obj(conn_handle) else {
            return (None, None);
        };
        let conn = conn_ptr.lock().await;
        (conn.session_id, conn.wrapper_identity.clone())
    }

    /// Resolve the Snowflake session id for a statement handle by traversing
    /// to its owning connection. Acquires the statement mutex to read the
    /// `Arc<Mutex<Connection>>`, then the connection mutex to read its
    /// cached `session_id`.
    pub(crate) async fn session_id_for_stmt(&self, stmt_handle: Handle) -> Option<i64> {
        let stmt_ptr = self.statements.get_obj(stmt_handle)?;
        let conn_arc = {
            let stmt = stmt_ptr.lock().await;
            stmt.conn.clone()
        };
        let conn = conn_arc.lock().await;
        conn.session_id
    }

    /// Flush buffered telemetry spans for a specific session.
    pub(super) async fn flush_telemetry_session(&self, session_id: i64) {
        if let Some(ref lm) = self.log_manager {
            lm.flush_session(session_id).await;
        }
    }

    pub fn token_cache(&self) -> Result<Arc<dyn TokenCache>, TokenCacheError> {
        self.token_cache
            .get_or_try_init(|| {
                KeyringTokenCache::new().map(|c| Arc::new(c) as Arc<dyn TokenCache>)
            })
            .map(Arc::clone)
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
            Arc::ptr_eq(&first, &second),
            "token_cache() should return the same instance on repeated calls"
        );
    }

    #[test]
    fn crl_worker_lazy_init_succeeds() {
        let driver = DatabaseDriverV1::new();
        let worker = driver.crl_worker.clone();
        assert!(Arc::strong_count(&worker) >= 1);
    }

    #[test]
    fn crl_worker_returns_same_instance() {
        let driver = DatabaseDriverV1::new();
        let first = driver.crl_worker.clone();
        let second = driver.crl_worker.clone();
        assert!(
            Arc::ptr_eq(&first, &second),
            "crl_worker field should return the same Arc on repeated clones"
        );
    }

    #[test]
    fn driver_state_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DatabaseDriverV1>();
    }
}
