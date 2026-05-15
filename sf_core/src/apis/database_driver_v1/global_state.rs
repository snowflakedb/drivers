use std::collections::HashMap;
use std::sync::Arc;

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
    /// When true, PUT auto-detect mirrors legacy libsnowflakeclient
    /// behavior: (1) unsupported compression formats are silently
    /// treated as uncompressed instead of erroring, and (2) magic-byte
    /// detection consults a short-prefix table (2-byte gzip, 2-byte
    /// zlib mapped to `Deflate`, 4-byte snowflake brotli marker) ahead
    /// of the `infer` crate.
    pub legacy_compression_autodetect_libsnowflakeclient_behavior: bool,
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
            legacy_compression_autodetect_libsnowflakeclient_behavior: true,
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

    /// Resolve the Snowflake session id for a connection handle by reading
    /// `Connection::session_id` under the connection mutex. Returns `None`
    /// when the handle is unknown, login has not completed, or the connection
    /// has been released.
    pub(crate) async fn session_id_for_conn(&self, conn_handle: Handle) -> Option<i64> {
        let conn_ptr = self.connections.get_obj(conn_handle)?;
        let conn = conn_ptr.lock().await;
        conn.session_id
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

    /// Resolve the Snowflake session_id for a connection handle.
    pub(crate) async fn session_id(&self, handle: Handle) -> Option<i64> {
        let conn_ptr = self.connections.get_obj(handle)?;
        let conn = conn_ptr.lock().await;
        let tokens = conn.tokens.read().await;
        tokens.as_ref().map(|t| t.session_id)
    }

    /// Flush buffered telemetry spans for a specific session.
    pub(crate) async fn flush_telemetry_session(&self, session_id: i64) {
        if let Some(ref lm) = self.log_manager {
            lm.flush_session(session_id).await;
        }
    }

    /// Add a pre-formatted user telemetry log entry to the session's buffer.
    pub(crate) fn add_user_telemetry_log(&self, session_id: i64, entry: serde_json::Value) {
        if let Some(ref lm) = self.log_manager {
            lm.add_user_log(session_id, entry);
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
