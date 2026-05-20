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
/// The `client` is held behind an `Arc` so callers that want to invoke
/// protobuf RPCs (foreground SQL via [`block_on`](Self::block_on),
/// in-band telemetry via the same helper) do not force the generated
/// `DatabaseDriverClient` itself to be `Clone`.
///
/// # Single runtime
///
/// A single multi-threaded tokio runtime (`num_cpus` workers) drives
/// every interaction with sf_core through the protobuf client -
/// foreground SQL, connection lifecycle, **and** in-band telemetry
/// (see [`crate::api::telemetry`]). The wrapper holds **no** typed
/// sf_core handles: the only sf_core symbols reachable from here are
/// the prost-generated `DatabaseDriverClient` + request/response
/// messages, exactly mirroring the surface area that
/// `snowflake.connector` (Python) consumes over its C ABI.
///
/// At `env_freed` time all user-facing SQL handles have already been
/// freed, so the runtime is quiescent and `Runtime::drop` does not block
/// process exit.
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
    load_ini_config();
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
        runtime,
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

/// Build the ordered candidate path list and seed `sf_core`'s process-wide
/// INI snapshot before logging initialisation. A subsequent environment
/// allocation in the same process re-enters this function; the underlying
/// `OnceLock` accepts only the first successful load, so the
/// `IniAlreadyLoaded` arm is benign and intentionally silenced.
fn load_ini_config() {
    let paths = crate::api::ini_paths::default_paths();
    match sf_core::config::load_ini_files(&paths) {
        Ok(()) | Err(sf_core::config::ConfigError::IniAlreadyLoaded { .. }) => {}
        Err(e) => {
            eprintln!("Failed to load sf.odbc.ini: {e:?}; using defaults");
        }
    }
}
