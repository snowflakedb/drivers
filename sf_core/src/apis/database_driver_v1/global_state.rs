use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use super::connection::Connection;
#[cfg(feature = "protobuf")]
use super::connection::WrapperIdentity;
use super::database::Database;
use super::result_set::ResultSet;
use super::statement::Statement;
use super::stream_transfer::{DownloadStream, UploadStreamSession};
use crate::config::param_registry::Wrapper;
use crate::crl::worker::{CrlWorker, SharedCrlWorker};
use crate::fs_adapter::{FsAdapter, RealFs};
use crate::handle_manager::{Handle, HandleManager};
use crate::logging::LogManager;
use crate::rest::snowflake::prompt_lock::PromptLockMap;
use crate::telemetry::platform_detection::{DetectionConfig, detect_platforms};
use crate::telemetry::snowflake_exporter::SessionRegistry;
use crate::token_cache::{KeyringTokenCache, TokenCache, TokenCacheError};
use crate::xp_backend::{SnowflakeBackend, XpSlot};

/// Which shape the PUT/GET result set should take.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum PutGetResultsetFlavor {
    #[default]
    Python,
    Odbc,
    Jdbc,
    NodeJs,
}

/// Immutable behavioural presets declared by each wrapper (Python, ODBC, JDBC,
/// Node.js) at startup. These are **not** exposed to end users — they capture
/// compile-time / init-time differences between wrappers so that shared Rust
/// code can branch on them without hard-coding wrapper knowledge everywhere.
#[derive(Debug, Clone)]
pub struct WrapperPresets {
    /// Which wrapper is talking to core. Used during the transitional period
    /// where core still remaps wire aliases via
    /// `ParamRegistry::resolve_for`.
    pub configuration_flavor: Wrapper,
    pub put_get_resultset_flavor: PutGetResultsetFlavor,
    /// When true, PUT auto-detect mirrors legacy libsnowflakeclient
    /// behavior: (1) unsupported compression formats are silently
    /// treated as uncompressed instead of erroring, and (2) magic-byte
    /// detection consults a short-prefix table (2-byte gzip, 2-byte
    /// zlib mapped to `Deflate`, 4-byte snowflake brotli marker) ahead
    /// of the `infer` crate.
    pub legacy_odbc_compression_autodetect: bool,
    /// Default for PUT_FASTFAIL/GET_FASTFAIL when unset (mirrors old ODBC's
    /// connection-string attrs). `true` = fail-fast (abort on first error);
    /// `false` = collect-all (ODBC's default; failures become ERROR rows).
    pub put_get_fastfail_default: bool,
    /// Gzip level for PUT AUTO_COMPRESS when `put_compress_level` is unset or
    /// outside 0–9.
    pub put_compress_level_default: u32,
    /// When true, GET of a staged path that matches no object returns an empty
    /// result set (legacy snowflake-jdbc). When false, it errors with
    /// `RemoteFileNotFound` (Python, ODBC, and core).
    pub legacy_empty_get_on_missing: bool,
    /// When true, the client `enablePutGet` property and the server
    /// `JDBC_ENABLE_PUT_GET` session parameter can disable PUT/GET (rejected
    /// before dispatch with "File transfers have been disabled."). Both flags
    /// are JDBC-specific; legacy Python and other drivers honor neither, so this
    /// stays false for them and the disable gate never fires.
    pub honor_put_get_disable: bool,
    /// When `true`, receiving `queryContext: { entries: null }` in a response
    /// clears the client-side query context cache. When `false`, null entries
    /// are treated as absent (cache unchanged). JDBC and ODBC keeps `false` to match
    /// the original driver behavior.
    pub clear_query_context_on_null_entries: bool,
    /// When true, `ALTER SESSION SET` assignments are parsed out of the
    /// submitted SQL and written to the session-parameter cache without waiting
    /// for the server to echo them, which keeps Python's `_session_parameters`
    /// proxy in sync for parameters the response omits. When false, the cache
    /// only ever reflects parameters a response carries.
    pub optimistic_alter_session_param_cache: bool,
    /// Default when the `serialize_session_operations` connection parameter is unset.
    /// When true, one in-flight session operation per connection.
    pub serialize_session_operations: bool,
    /// When true, a login with pre-acquired session and master tokens proves the pair
    /// with a token-request RENEW, which costs a round-trip, rotates the pair, and
    /// reports the session id telemetry is keyed by. When false the pair is adopted as
    /// handed over: no per-operation telemetry, and a gone session surfaces on the
    /// first real request. Only Node.js sets this false, for `deserializeConnection`.
    pub validate_session_token: bool,
}

impl Default for WrapperPresets {
    /// Hand-written rather than derived: `#[derive(Default)]` would give
    /// `put_get_fastfail_default` `bool::default() == false`, flipping every
    /// wrapper but ODBC to collect-all by accident, and
    /// `put_compress_level_default` `u32::default() == 0` (store-only gzip).
    ///
    /// `configuration_flavor` is `Wrapper::Python`, so a wrapper resolving aliases
    /// under the Python flavor leaves its own scoped aliases in `sf_params_spec`
    /// inert. That covers .NET (no constructor yet) and, on purpose,
    /// [`Self::nodejs`], which has a constructor but keeps `Wrapper::Python`
    /// because only its PUT/GET result-set flavor differs -- beside
    /// [`Self::python`], [`Self::odbc`] and [`Self::jdbc`].
    fn default() -> Self {
        Self {
            configuration_flavor: Wrapper::Python,
            put_get_resultset_flavor: PutGetResultsetFlavor::default(),
            legacy_odbc_compression_autodetect: false,
            put_get_fastfail_default: true,
            put_compress_level_default: 9,
            legacy_empty_get_on_missing: false,
            honor_put_get_disable: false,
            clear_query_context_on_null_entries: true,
            optimistic_alter_session_param_cache: false,
            serialize_session_operations: false,
            validate_session_token: true,
        }
    }
}

impl WrapperPresets {
    /// Presets for the Python connector.
    pub fn python() -> Self {
        Self {
            optimistic_alter_session_param_cache: true,
            ..Self::default()
        }
    }

    /// Presets for the ODBC driver.
    pub fn odbc() -> Self {
        Self {
            configuration_flavor: Wrapper::Odbc,
            put_get_resultset_flavor: PutGetResultsetFlavor::Odbc,
            legacy_odbc_compression_autodetect: true,
            put_get_fastfail_default: false,
            put_compress_level_default: 6,
            legacy_empty_get_on_missing: false,
            honor_put_get_disable: false,
            clear_query_context_on_null_entries: false,
            optimistic_alter_session_param_cache: false,
            serialize_session_operations: true,
            validate_session_token: true,
        }
    }

    /// Presets for the JDBC bridge.
    pub fn jdbc() -> Self {
        Self {
            configuration_flavor: Wrapper::Jdbc,
            put_get_resultset_flavor: PutGetResultsetFlavor::Jdbc,
            legacy_empty_get_on_missing: true,
            honor_put_get_disable: true,
            clear_query_context_on_null_entries: false,
            put_compress_level_default: 6,
            ..Self::default()
        }
    }

    /// Presets for the Node.js bridge.
    pub fn nodejs() -> Self {
        Self {
            put_get_resultset_flavor: PutGetResultsetFlavor::NodeJs,
            put_compress_level_default: 6,
            validate_session_token: false,
            ..Self::default()
        }
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
    /// Host-owned query/login transport. Production XP hosts register after
    /// construction via [`DatabaseDriverV1::register_xp_backend`].
    pub xp_backend: Option<Arc<dyn SnowflakeBackend>>,
    /// When `None`, [`DatabaseDriverV1::with_providers`] reads
    /// [`crate::env_vars::SNOWFLAKE_RUNNING_INSIDE_XP`].
    pub running_inside_xp: Option<bool>,
}

pub struct DatabaseDriverV1 {
    pub(super) databases: HandleManager<Mutex<Database>>,
    pub(super) connections: HandleManager<Mutex<Connection>>,
    pub(super) statements: HandleManager<Mutex<Statement>>,
    pub(super) results: HandleManager<Mutex<ResultSet>>,
    /// Pending chunked uploads; see `stream_transfer::UploadStreamSession` for
    /// the registration/mutation/consumption lifecycle. `UploadStreamSession`
    /// carries its own interior `Mutex` around the growing buffer (its
    /// `conn_handle`/`sql` fields are set once and never mutated), so it is
    /// stored directly rather than double-wrapped.
    pub(super) upload_streams: HandleManager<UploadStreamSession>,
    /// Pending chunked downloads; see `stream_transfer::DownloadStream` for
    /// the registration/mutation/teardown lifecycle (`download_stream_begin`,
    /// `download_stream_chunk`, `download_stream_close`).
    pub(super) download_streams: HandleManager<DownloadStream>,
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
    pub(crate) xp_slot: Arc<XpSlot>,
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
        // Pin the rustls crypto backend before anything can build an HTTP
        // client, so every connection this driver makes -- including telemetry
        // and CRL fetches that run ahead of the first query -- shares one
        // provider. See `tls::ensure_crypto_provider`.
        crate::tls::ensure_crypto_provider();

        let xp_slot = Arc::new(match providers.running_inside_xp {
            Some(inside) => XpSlot::new(inside, providers.xp_backend),
            None => XpSlot::from_env(providers.xp_backend),
        });
        crate::xp_backend::registry::install_c_registration_target(Arc::clone(&xp_slot));
        Self {
            databases: HandleManager::new(),
            connections: HandleManager::new(),
            statements: HandleManager::new(),
            results: HandleManager::new(),
            upload_streams: HandleManager::new(),
            download_streams: HandleManager::new(),
            token_cache: once_cell::sync::OnceCell::new(),
            fs: providers.fs.unwrap_or_else(|| Arc::new(RealFs)),
            platforms: tokio::sync::OnceCell::const_new(),
            log_manager: providers.log_manager,
            wrapper_presets: providers.wrapper_presets,
            prompt_locks: providers
                .prompt_locks
                .unwrap_or_else(|| Arc::new(std::sync::Mutex::new(HashMap::new()))),
            crl_worker: providers.crl_worker.unwrap_or_else(CrlWorker::new_lazy),
            xp_slot,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn register_xp_backend(
        &self,
        backend: Arc<dyn SnowflakeBackend>,
    ) -> Result<(), crate::xp_backend::BackendError> {
        self.xp_slot.register(backend)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn xp_backend(
        &self,
    ) -> Result<Option<&Arc<dyn SnowflakeBackend>>, crate::xp_backend::BackendError> {
        self.xp_slot.active()
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
    #[cfg(feature = "protobuf")]
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

    /// Flush buffered telemetry spans for a specific session.
    pub(super) async fn flush_telemetry_session(&self, session_id: i64) {
        if let Some(ref lm) = self.log_manager {
            lm.flush_session(session_id).await;
        }
    }

    /// Forward one caller-produced telemetry entry to the core's in-band batch.
    /// No-ops when telemetry is unconfigured or the session id is not yet known
    /// (login incomplete / handle released). Core owns batching, flush threshold,
    /// and egress.
    pub(crate) async fn telemetry_send_log(
        &self,
        conn_handle: Handle,
        message_json: String,
        timestamp_ms: i64,
    ) {
        tracing::debug!("telemetry_send_log: entry");
        let Some(lm) = self.log_manager.as_ref() else {
            tracing::debug!("telemetry_send_log: exit");
            return;
        };
        let Some(session_id) = self.session_id_for_conn(conn_handle).await else {
            tracing::debug!("telemetry_send_log: exit");
            return;
        };
        lm.telemetry()
            .add_log(session_id, message_json, timestamp_ms);
        tracing::debug!("telemetry_send_log: exit");
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

    /// Whether troubleshooting mode is currently active. Delegates to the
    /// `LogManager` if one was injected; returns `false` otherwise.
    pub fn is_troubleshooting(&self) -> bool {
        self.log_manager
            .as_ref()
            .is_some_and(|lm| lm.is_troubleshooting())
    }

    /// Resolved troubleshooting log directory when troubleshooting is active.
    /// Used as a fallback for `DiagnosticConfig::log_path`.
    pub(crate) fn troubleshooting_path(&self) -> Option<std::path::PathBuf> {
        self.log_manager
            .as_ref()
            .and_then(|lm| lm.troubleshooting_path())
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
    use crate::xp_backend::TestBackend;

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

    #[test]
    fn only_jdbc_honors_put_get_disable() {
        // The `enablePutGet` client property and `JDBC_ENABLE_PUT_GET` server
        // param are JDBC-specific; only the JDBC preset opts the shared gate in.
        assert!(WrapperPresets::jdbc().honor_put_get_disable);
        assert!(!WrapperPresets::python().honor_put_get_disable);
        assert!(!WrapperPresets::odbc().honor_put_get_disable);
        assert!(!WrapperPresets::default().honor_put_get_disable);
    }

    #[test]
    fn session_rpc_serialization_follows_wrapper() {
        assert!(!WrapperPresets::default().serialize_session_operations);
        assert!(!WrapperPresets::python().serialize_session_operations);
        assert!(!WrapperPresets::jdbc().serialize_session_operations);
        assert!(WrapperPresets::odbc().serialize_session_operations);
    }

    #[test]
    fn put_compress_level_default_follows_wrapper() {
        assert_eq!(WrapperPresets::odbc().put_compress_level_default, 6);
        assert_eq!(WrapperPresets::python().put_compress_level_default, 9);
        assert_eq!(WrapperPresets::jdbc().put_compress_level_default, 6);
        assert_eq!(WrapperPresets::nodejs().put_compress_level_default, 6);
        assert_eq!(WrapperPresets::default().put_compress_level_default, 9);
    }

    #[test]
    fn only_nodejs_adopts_session_tokens_unvalidated() {
        assert!(!WrapperPresets::nodejs().validate_session_token);
        assert!(WrapperPresets::python().validate_session_token);
        assert!(WrapperPresets::odbc().validate_session_token);
        assert!(WrapperPresets::jdbc().validate_session_token);
        assert!(WrapperPresets::default().validate_session_token);
    }

    #[test]
    fn xp_mode_without_backend_fails_closed() {
        let driver = DatabaseDriverV1::with_providers(DriverProviders {
            running_inside_xp: Some(true),
            ..Default::default()
        });
        let err = match driver.xp_backend() {
            Err(err) => err,
            Ok(_) => panic!("XP mode should require a backend"),
        };
        assert_eq!(err.code, crate::xp_backend::error_codes::NOT_REGISTERED);
    }

    #[test]
    fn http_mode_has_no_backend() {
        let driver = DatabaseDriverV1::with_providers(DriverProviders {
            running_inside_xp: Some(false),
            ..Default::default()
        });
        assert!(driver.xp_backend().unwrap().is_none());
    }

    #[test]
    fn injected_backend_is_active_in_xp() {
        let backend: Arc<dyn SnowflakeBackend> = Arc::new(TestBackend);
        let driver = DatabaseDriverV1::with_providers(DriverProviders {
            running_inside_xp: Some(true),
            xp_backend: Some(Arc::clone(&backend)),
            ..Default::default()
        });
        let active = match driver.xp_backend() {
            Ok(Some(active)) => active,
            Ok(None) => panic!("XP mode should use the backend"),
            Err(err) => panic!("registered backend should be active: {err}"),
        };
        assert!(Arc::ptr_eq(active, &backend));
    }

    #[test]
    fn register_xp_backend_attaches_after_construction() {
        let driver = DatabaseDriverV1::with_providers(DriverProviders {
            running_inside_xp: Some(true),
            ..Default::default()
        });
        let backend: Arc<dyn SnowflakeBackend> = Arc::new(TestBackend);
        driver
            .register_xp_backend(Arc::clone(&backend))
            .expect("registration should succeed");
        let active = match driver.xp_backend() {
            Ok(Some(active)) => active,
            Ok(None) => panic!("XP mode should use the backend"),
            Err(err) => panic!("registered backend should be active: {err}"),
        };
        assert!(Arc::ptr_eq(active, &backend));
    }

    #[tokio::test]
    async fn connection_new_shares_the_driver_slot() {
        let backend: Arc<dyn SnowflakeBackend> = Arc::new(TestBackend);
        let driver = DatabaseDriverV1::with_providers(DriverProviders {
            running_inside_xp: Some(true),
            xp_backend: Some(Arc::clone(&backend)),
            ..Default::default()
        });
        let handle = driver.connection_new();
        let conn = driver.connections.get_obj(handle).unwrap();
        let conn = conn.lock().await;
        let active = match conn.xp_slot.active() {
            Ok(Some(active)) => active,
            Ok(None) => panic!("XP mode should use the backend"),
            Err(err) => panic!("connection should see the driver backend: {err}"),
        };
        assert!(Arc::ptr_eq(active, &backend));
    }
}
