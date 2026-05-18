use std::ops::Deref;
use std::sync::{Arc, RwLock};

use parking_lot::Mutex;
use sf_core::protobuf::apis::database_driver_v1::{
    DatabaseDriverClient, DriverProviders, WrapperPresets, database_driver_client_with,
};
use snafu::{Location, ResultExt, Snafu};

use crate::api::handle_registry::HandleManager;

/// Serializes "last environment freed → `OdbcGlobals` destroyed" vs "first environment of the
/// next epoch allocates a new `OdbcGlobals`".
///
/// Without this, `env_freed` can release `STATE`'s write lock and then spend a long time in Tokio
/// teardown while `globals` is already `None`, letting another thread run `env_allocated` and
/// create a second runtime + client while the first is still shutting down.
///
/// **Lock order:** [`env_allocated`] must take this mutex **before** any `STATE` write lock.
/// [`env_freed`] takes `STATE` first, then this mutex only on the last-env path—so the slow path
/// of `env_allocated` must never do `STATE` → release → `GLOBALS_TEARDOWN_LOCK`, which would
/// invert the order relative to `env_freed` and can deadlock (seen as ODBC CI failures under
/// parallel load, e.g. macOS matrix).
static GLOBALS_TEARDOWN_LOCK: Mutex<()> = Mutex::new(());

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
/// The `client` is held behind an `Arc` so fire-and-forget telemetry futures
/// can take a cheap clone without forcing the generated `DatabaseDriverClient`
/// itself to be `Clone`.
///
/// # Single runtime + per-call spawn for telemetry
///
/// A single multi-threaded tokio runtime drives every foreground SQL
/// operation via [`block_on`](Self::block_on) and also hosts fire-and-forget
/// telemetry tasks via [`spawn_telemetry`](Self::spawn_telemetry). Each
/// `SQL*` entry point pays one [`tokio::runtime::Runtime::spawn`] per
/// telemetry event (~1 µs); the spawned future calls
/// `client.telemetry_send_*` directly, which only records an in-memory
/// OTel event under the per-connection span (no network I/O) and so
/// returns promptly.
///
/// The [`Drop`] impl calls
/// [`Runtime::shutdown_background`](tokio::runtime::Runtime::shutdown_background)
/// so process exit never blocks on a stray in-flight telemetry task. At
/// `env_freed` time all user-facing SQL handles have already been freed,
/// so abandoning any remaining spawned futures is safe.
pub struct OdbcGlobals {
    /// Wrapped in `Option` so [`Drop`] can `.take()` the runtime out of
    /// `&mut self` and call
    /// [`Runtime::shutdown_background`](tokio::runtime::Runtime::shutdown_background)
    /// instead of joining all spawned tasks on the current thread.
    runtime: Option<tokio::runtime::Runtime>,
    client: Arc<DatabaseDriverClient>,
    pub env_registry: HandleManager<crate::api::Env>,
    pub dbc_registry: HandleManager<crate::api::Dbc>,
    pub stmt_registry: HandleManager<crate::api::Statement>,
}

impl Drop for OdbcGlobals {
    fn drop(&mut self) {
        if let Some(rt) = self.runtime.take() {
            rt.shutdown_background();
        }
    }
}

impl OdbcGlobals {
    fn runtime(&self) -> &tokio::runtime::Runtime {
        self.runtime
            .as_ref()
            .expect("OdbcGlobals runtime accessed after Drop (bug)")
    }

    pub fn block_on<T>(&self, f: impl AsyncFnOnce(&DatabaseDriverClient) -> T) -> T {
        self.runtime().block_on(f(&self.client))
    }

    /// Spawn a fire-and-forget telemetry future on the main runtime.
    ///
    /// The closure receives an `Arc` clone of the shared
    /// [`DatabaseDriverClient`] and returns the future to spawn. Returns
    /// immediately to the SQL hot path; the future itself runs on the
    /// runtime and is abandoned at process exit (see
    /// [`Drop`](Self#impl-Drop-for-OdbcGlobals)).
    pub fn spawn_telemetry<F, Fut>(&self, f: F)
    where
        F: FnOnce(Arc<DatabaseDriverClient>) -> Fut,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let client = Arc::clone(&self.client);
        self.runtime().spawn(f(client));
    }
}

struct GlobalState {
    env_count: usize,
    globals: Option<Arc<OdbcGlobals>>,
}

static STATE: RwLock<GlobalState> = RwLock::new(GlobalState {
    env_count: 0,
    globals: None,
});

/// [`Arc::clone`] of the process-wide ODBC globals; [`global()`] does not keep `STATE`'s read
/// lock for the whole duration of [`OdbcGlobals::block_on`].
pub struct GlobalsGuard(Arc<OdbcGlobals>);

impl Deref for GlobalsGuard {
    type Target = OdbcGlobals;
    fn deref(&self) -> &OdbcGlobals {
        self.0.as_ref()
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
    let Some(arc) = guard.globals.as_ref() else {
        return NotInitializedSnafu.fail();
    };
    let arc = Arc::clone(arc);
    drop(guard);
    Ok(GlobalsGuard(arc))
}

pub fn env_allocated() -> Result<(), OdbcRuntimeError> {
    // Take the teardown mutex before `STATE`'s write lock so we never invert lock order relative
    // to `env_freed` (which does `STATE` then teardown on the last-env path).
    let _teardown = GLOBALS_TEARDOWN_LOCK.lock();
    let mut guard = STATE.write().map_err(|_| LockPoisonedSnafu.build())?;
    if guard.globals.is_some() {
        guard.env_count += 1;
        return Ok(());
    }
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
    guard.globals = Some(Arc::new(OdbcGlobals {
        runtime: Some(runtime),
        client,
        env_registry: HandleManager::new(),
        dbc_registry: HandleManager::new(),
        stmt_registry: HandleManager::new(),
    }));
    tracing::info!("ODBC driver starting v{}", env!("CARGO_PKG_VERSION"));
    guard.env_count += 1;
    Ok(())
}

pub fn env_freed() -> Result<(), OdbcRuntimeError> {
    let mut guard = STATE.write().map_err(|_| LockPoisonedSnafu.build())?;
    guard.env_count = guard.env_count.saturating_sub(1);
    if guard.env_count == 0 {
        tracing::info!("Last ODBC environment freed, tearing down global state");
        let globals = guard.globals.take();
        drop(guard);
        if let Some(arc) = globals {
            let _teardown = GLOBALS_TEARDOWN_LOCK.lock();
            while Arc::strong_count(&arc) > 1 {
                std::thread::yield_now();
            }
            drop(arc);
        }
    }
    Ok(())
}
