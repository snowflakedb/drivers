use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::RwLock;

use sf_core::protobuf::apis::database_driver_v1::{
    DatabaseDriverClient, DriverProviders, WrapperPresets, database_driver_client_with,
};
use snafu::{Location, ResultExt, Snafu};

use crate::api::handle_registry::HandleManager;

/// Holds the shared tokio runtime and driver client used by all ODBC
/// environments in this process.
///
/// ODBC requires an Environment handle before any Connection or Statement can
/// be created. An application may allocate multiple Environments (e.g. for
/// different global settings), but they all share the same underlying driver
/// state. We therefore keep a single `OdbcGlobals` instance behind a
/// reference-counted latch (`env_count`): the first `SQLAllocHandle(SQL_HANDLE_ENV)`
/// creates it, and the last `SQLFreeHandle(SQL_HANDLE_ENV)` tears it down.
/// On Windows the ODBC Driver Manager unloads the driver DLL after the last
/// environment is freed, so we must shut down before that happens.
///
/// The `client` is held behind an `Arc` so fire-and-forget tasks
/// (`spawn_telemetry`) can take a cheap clone without forcing the generated
/// `DatabaseDriverClient` itself to be `Clone`.
pub struct OdbcGlobals {
    runtime: tokio::runtime::Runtime,
    client: Arc<DatabaseDriverClient>,
    pub env_registry: HandleManager<crate::api::Env>,
    pub dbc_registry: HandleManager<crate::api::Dbc>,
    pub stmt_registry: HandleManager<crate::api::Statement>,
}

impl OdbcGlobals {
    pub fn block_on<T>(&self, f: impl AsyncFnOnce(&DatabaseDriverClient) -> T) -> T {
        self.runtime.block_on(f(&self.client))
    }

    /// Spawn a fire-and-forget telemetry task on the shared runtime.
    ///
    /// The closure receives a cheap `Arc` clone of the driver client so it
    /// can issue `telemetry_send_*` RPCs from inside the spawned future.
    /// Tasks are detached: failures are silently dropped, and the SQL hot
    /// path continues without waiting for the task to start running.
    ///
    /// Callers must not touch ODBC handle registries from inside the
    /// spawned future — registry guards are not `Send` across an await
    /// boundary.
    pub fn spawn_telemetry<F, Fut>(&self, f: F)
    where
        F: FnOnce(Arc<DatabaseDriverClient>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let client = Arc::clone(&self.client);
        self.runtime.spawn(f(client));
    }
}

struct GlobalState {
    env_count: usize,
    globals: Option<OdbcGlobals>,
}

static STATE: RwLock<GlobalState> = RwLock::new(GlobalState {
    env_count: 0,
    globals: None,
});

pub struct GlobalsGuard(std::sync::RwLockReadGuard<'static, GlobalState>);

impl Deref for GlobalsGuard {
    type Target = OdbcGlobals;
    fn deref(&self) -> &OdbcGlobals {
        self.0
            .globals
            .as_ref()
            .expect("GlobalsGuard created while globals are None (bug in global())")
    }
}

#[derive(Debug, Snafu)]
pub enum OdbcRuntimeError {
    #[snafu(display("ODBC globals not initialized; allocate an environment handle first"))]
    NotInitialized {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("ODBC globals RwLock poisoned"))]
    LockPoisoned {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to create ODBC tokio runtime"))]
    RuntimeCreation {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

pub fn global() -> Result<GlobalsGuard, OdbcRuntimeError> {
    let guard = STATE.read().map_err(|_| LockPoisonedSnafu.build())?;
    if guard.globals.is_none() {
        return NotInitializedSnafu.fail();
    }
    Ok(GlobalsGuard(guard))
}

pub fn env_allocated() -> Result<(), OdbcRuntimeError> {
    let mut guard = STATE.write().map_err(|_| LockPoisonedSnafu.build())?;
    if guard.globals.is_none() {
        let log_manager = sf_core::logging::LogManager::for_odbc();
        if let Some(lm) = &log_manager {
            crate::api::error_trace_flag::set_error_trace_enabled(lm.error_trace_enabled());
        }
        let providers = DriverProviders {
            log_manager,
            wrapper_presets: WrapperPresets::odbc(),
            ..Default::default()
        };

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context(RuntimeCreationSnafu)?;
        let client = Arc::new(database_driver_client_with(providers));
        guard.globals = Some(OdbcGlobals {
            runtime,
            client,
            env_registry: HandleManager::new(),
            dbc_registry: HandleManager::new(),
            stmt_registry: HandleManager::new(),
        });
        tracing::info!("ODBC driver starting v{}", env!("CARGO_PKG_VERSION"));
    }
    guard.env_count += 1;
    Ok(())
}

pub fn env_freed() -> Result<(), OdbcRuntimeError> {
    let mut guard = STATE.write().map_err(|_| LockPoisonedSnafu.build())?;
    guard.env_count = guard.env_count.saturating_sub(1);
    if guard.env_count == 0 {
        tracing::info!("Last ODBC environment freed, tearing down global state");
        guard.globals = None;
    }
    Ok(())
}
