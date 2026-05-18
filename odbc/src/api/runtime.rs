use std::ops::Deref;
use std::sync::{Arc, RwLock};

use parking_lot::Mutex;
use sf_core::apis::database_driver_v1::DatabaseDriverV1;
use sf_core::protobuf::apis::database_driver_v1::{
    DatabaseDriverClient, DriverProviders, WrapperPresets, database_driver_client_and_driver_with,
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
/// synchronous telemetry via the same helper) do not force the generated
/// `DatabaseDriverClient` itself to be `Clone`.
///
/// # Single runtime, direct-API telemetry
///
/// A single multi-threaded tokio runtime (`num_cpus` workers) drives
/// every foreground SQL operation through the protobuf client. The
/// in-band telemetry path, on the other hand, **never** touches the
/// runtime or the protobuf transport:
///
/// - At `SQLConnect` time the wrapper calls the **synchronous**
///   [`DatabaseDriverV1::connection_telemetry`] via [`driver()`](Self::driver),
///   which serves the recorder from sf_core's lock-light side-table.
///   No `block_on`.
/// - The returned [`ConnectionTelemetry`](sf_core::telemetry::ConnectionTelemetry)
///   is stashed on the `Dbc`'s `ConnectionState::Connected`. Each
///   subsequent `SQL*` entry point records its `api_call` /
///   `exception` event through that cached recorder — no `block_on`,
///   no protobuf serialisation, no async-mutex contention with the
///   SQL data path.
///
/// `driver` is the very same [`DatabaseDriverV1`] the `client`'s
/// transport routes to (shared via `Arc`), so protobuf and direct
/// callers always see the same handle registries.
///
/// At `env_freed` time all user-facing SQL handles have already been
/// freed, so the runtime is quiescent and `Runtime::drop` does not block
/// process exit.
pub struct OdbcGlobals {
    runtime: tokio::runtime::Runtime,
    client: Arc<DatabaseDriverClient>,
    driver: Arc<DatabaseDriverV1>,
    pub env_registry: HandleManager<crate::api::Env>,
    pub dbc_registry: HandleManager<crate::api::Dbc>,
    pub stmt_registry: HandleManager<crate::api::Statement>,
}

impl OdbcGlobals {
    pub fn block_on<T>(&self, f: impl AsyncFnOnce(&DatabaseDriverClient) -> T) -> T {
        self.runtime.block_on(f(&self.client))
    }

    /// Shared driver instance used by both the protobuf client above
    /// and by direct sync callers (notably the telemetry recorder
    /// fetch at `SQLConnect` time).
    pub fn driver(&self) -> &Arc<DatabaseDriverV1> {
        &self.driver
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
    let (client, driver) = database_driver_client_and_driver_with(providers);
    let client = Arc::new(client);
    guard.globals = Some(Arc::new(OdbcGlobals {
        runtime,
        client,
        driver,
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
