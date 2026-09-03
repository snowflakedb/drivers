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
use crate::config::ParamStore;
use crate::config::param_registry::{Wrapper, param_names};
use crate::config::settings::Setting;
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
    /// When true, `OAUTH_AUTHORIZATION_CODE` connections cache the access /
    /// refresh token unless `client_store_temporary_credential` is set
    /// explicitly. When false, caching stays off until the caller opts in.
    pub oauth_authorization_code_cache_default: bool,
}

impl Default for WrapperPresets {
    /// Hand-written rather than derived: `#[derive(Default)]` would give
    /// `put_get_fastfail_default` `bool::default() == false`, flipping every
    /// wrapper but ODBC to collect-all by accident.
    ///
    /// `configuration_flavor` is `Wrapper::Python`, so any wrapper without its
    /// own constructor here — today the Node.js bridge and .NET — resolves
    /// aliases under the Python flavor, and a `NodeJs`- or `DotNet`-scoped alias
    /// in `sf_params_spec` is inert until that wrapper gets a
    /// `WrapperPresets::…()` beside [`Self::python`], [`Self::odbc`] and
    /// [`Self::jdbc`].
    fn default() -> Self {
        Self {
            configuration_flavor: Wrapper::Python,
            put_get_resultset_flavor: PutGetResultsetFlavor::default(),
            legacy_odbc_compression_autodetect: false,
            put_get_fastfail_default: true,
            legacy_empty_get_on_missing: false,
            honor_put_get_disable: false,
            clear_query_context_on_null_entries: true,
            oauth_authorization_code_cache_default: false,
        }
    }
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
    pub fn odbc() -> Self {
        Self {
            configuration_flavor: Wrapper::Odbc,
            put_get_resultset_flavor: PutGetResultsetFlavor::Odbc,
            legacy_odbc_compression_autodetect: true,
            put_get_fastfail_default: false,
            legacy_empty_get_on_missing: false,
            honor_put_get_disable: false,
            clear_query_context_on_null_entries: false,
            oauth_authorization_code_cache_default: true,
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
            ..Self::default()
        }
    }

    /// Default `client_store_temporary_credential` on for the OAuth
    /// authorization-code flow, for wrappers whose legacy driver cached those
    /// tokens by default.
    ///
    /// `user_seed` must carry every layer the user can set the flag through —
    /// the database handle as well as the connection — because this only fills
    /// in a default and must never overwrite a value the caller chose. Passing a
    /// narrower seed silently re-enables caching for anyone who disabled it on
    /// the layer that was left out.
    pub fn apply_oauth_authorization_code_cache_default(
        &self,
        resolved: &mut ParamStore,
        user_seed: &ParamStore,
    ) {
        if !self.oauth_authorization_code_cache_default {
            return;
        }
        let authenticator = resolved
            .get_string(param_names::AUTHENTICATOR)
            .or_else(|| user_seed.get_string(param_names::AUTHENTICATOR))
            .unwrap_or_default();
        if !authenticator.eq_ignore_ascii_case("OAUTH_AUTHORIZATION_CODE") {
            return;
        }
        if user_seed
            .get_bool(param_names::CLIENT_STORE_TEMPORARY_CREDENTIAL)
            .is_some()
        {
            return;
        }
        resolved.insert(
            param_names::CLIENT_STORE_TEMPORARY_CREDENTIAL.into(),
            Setting::Bool(true),
        );
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
    fn only_odbc_defaults_oauth_ac_token_cache_on() {
        assert!(WrapperPresets::odbc().oauth_authorization_code_cache_default);
        assert!(!WrapperPresets::python().oauth_authorization_code_cache_default);
        assert!(!WrapperPresets::jdbc().oauth_authorization_code_cache_default);
        assert!(!WrapperPresets::default().oauth_authorization_code_cache_default);
    }

    fn oauth_ac_stores(cache_in_seed: Option<bool>) -> (ParamStore, ParamStore) {
        let mut resolved = ParamStore::new();
        resolved.insert(
            param_names::AUTHENTICATOR.into(),
            Setting::String("OAUTH_AUTHORIZATION_CODE".into()),
        );
        resolved.insert(
            param_names::CLIENT_STORE_TEMPORARY_CREDENTIAL.into(),
            Setting::Bool(false),
        );
        let mut seed = ParamStore::new();
        seed.insert(
            param_names::AUTHENTICATOR.into(),
            Setting::String("OAUTH_AUTHORIZATION_CODE".into()),
        );
        if let Some(explicit) = cache_in_seed {
            seed.insert(
                param_names::CLIENT_STORE_TEMPORARY_CREDENTIAL.into(),
                Setting::Bool(explicit),
            );
        }
        (resolved, seed)
    }

    #[test]
    fn odbc_oauth_ac_cache_default_fills_true_when_unset() {
        let (mut resolved, seed) = oauth_ac_stores(None);
        WrapperPresets::odbc().apply_oauth_authorization_code_cache_default(&mut resolved, &seed);
        assert_eq!(
            resolved.get_bool(param_names::CLIENT_STORE_TEMPORARY_CREDENTIAL),
            Some(true)
        );
    }

    #[test]
    fn odbc_oauth_ac_cache_default_honors_explicit_false() {
        let (mut resolved, seed) = oauth_ac_stores(Some(false));
        WrapperPresets::odbc().apply_oauth_authorization_code_cache_default(&mut resolved, &seed);
        assert_eq!(
            resolved.get_bool(param_names::CLIENT_STORE_TEMPORARY_CREDENTIAL),
            Some(false)
        );
    }

    #[test]
    fn python_oauth_ac_cache_default_leaves_registry_false() {
        let (mut resolved, seed) = oauth_ac_stores(None);
        WrapperPresets::python().apply_oauth_authorization_code_cache_default(&mut resolved, &seed);
        assert_eq!(
            resolved.get_bool(param_names::CLIENT_STORE_TEMPORARY_CREDENTIAL),
            Some(false)
        );
    }

    #[test]
    fn odbc_oauth_ac_cache_default_ignores_other_authenticators() {
        let mut resolved = ParamStore::new();
        resolved.insert(
            param_names::AUTHENTICATOR.into(),
            Setting::String("USERNAME_PASSWORD_MFA".into()),
        );
        resolved.insert(
            param_names::CLIENT_STORE_TEMPORARY_CREDENTIAL.into(),
            Setting::Bool(false),
        );
        let seed = ParamStore::new();
        WrapperPresets::odbc().apply_oauth_authorization_code_cache_default(&mut resolved, &seed);
        assert_eq!(
            resolved.get_bool(param_names::CLIENT_STORE_TEMPORARY_CREDENTIAL),
            Some(false)
        );
    }
}
