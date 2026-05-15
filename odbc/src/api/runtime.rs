use std::ops::Deref;
use std::sync::Arc;
use std::sync::RwLock;

use sf_core::protobuf::apis::database_driver_v1::{
    DatabaseDriverClient, DriverProviders, WrapperPresets, database_driver_client_with,
};
use snafu::{Location, ResultExt, Snafu};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;

use crate::api::handle_registry::HandleManager;
use crate::api::telemetry::{
    TelemetryEvent, debug_log_telemetry_dropped_queue_full, drain_telemetry,
};

/// Capacity of the in-process telemetry channel. At ~32 B per event, the
/// queue tops out at ~256 KiB. Sized to absorb the largest realistic
/// burst we expect before the drainer catches up; once full,
/// further events are dropped (see [`OdbcGlobals::record_telemetry`]) and
/// a [`tracing::debug`] line is emitted (target `odbc::telemetry`).
const TELEMETRY_QUEUE_CAPACITY: usize = 8 * 1024;

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
/// The `client` is held behind an `Arc` so the telemetry drainer task
/// can take a cheap clone without forcing the generated
/// `DatabaseDriverClient` itself to be `Clone`.
///
/// # Two runtimes + an mpsc channel
///
/// We hold **two** tokio runtimes plus a process-wide telemetry channel:
///
/// - `runtime` — multi-threaded (`num_cpus` workers). Drives every
///   foreground SQL operation via [`block_on`](Self::block_on).
/// - `telemetry_runtime` — single-worker, dedicated to running the
///   long-lived telemetry drainer task.
/// - `telemetry_tx` — sender end of a bounded
///   [`mpsc::channel`](tokio::sync::mpsc::channel) of capacity
///   [`TELEMETRY_QUEUE_CAPACITY`]. The matching receiver lives inside
///   the drainer task and never escapes it.
///
/// On every `SQL*` entry point the wrapper calls
/// [`record_telemetry`](Self::record_telemetry), which performs a
/// non-blocking [`try_send`](tokio::sync::mpsc::Sender::try_send) on
/// `telemetry_tx`. The drainer then dequeues events and issues
/// `telemetry_send_*` RPCs in receive order on the dedicated runtime.
/// This collapses the per-call `tokio::spawn` cost (~1.2 µs) into a
/// single channel push (~100 ns) and reduces the per-iteration task
/// count from 977 to 1, which keeps the SQL hot path effectively free
/// of telemetry-induced scheduler pressure.
///
/// The runtime split also guarantees that the drainer task cannot
/// occupy a SQL worker while waiting on sf_core's `Mutex<Connection>`,
/// which the SQL fetch path also uses.
///
/// Telemetry does not expose a generic `spawn(future)` helper: the
/// `SQL*` hot path enqueues fixed-size [`crate::api::telemetry::TelemetryEvent`]
/// values (see [`Self::record_telemetry`]) instead of boxing per-call futures on
/// the runtime.
pub struct OdbcGlobals {
    runtime: tokio::runtime::Runtime,
    /// Telemetry executor — on [`Drop`](Drop) we call
    /// [`Runtime::shutdown_background`](tokio::runtime::Runtime::shutdown_background)
    /// so teardown never blocks on an in-flight `telemetry_send_*` awaiting I/O /
    /// `Mutex<Connection>` (otherwise [`Runtime`](tokio::runtime::Runtime) destruction waits for spawned tasks).
    telemetry_runtime: Option<tokio::runtime::Runtime>,
    client: Arc<DatabaseDriverClient>,
    telemetry_tx: mpsc::Sender<TelemetryEvent>,
    pub env_registry: HandleManager<crate::api::Env>,
    pub dbc_registry: HandleManager<crate::api::Dbc>,
    pub stmt_registry: HandleManager<crate::api::Statement>,
}

impl Drop for OdbcGlobals {
    fn drop(&mut self) {
        // Fire-and-forget extends to unload: `Runtime::drop` waits for all
        // spawned work; our drainer can be stuck in `telemetry_send_*`.
        if let Some(rt) = self.telemetry_runtime.take() {
            rt.shutdown_background();
        }
    }
}

impl OdbcGlobals {
    pub fn block_on<T>(&self, f: impl AsyncFnOnce(&DatabaseDriverClient) -> T) -> T {
        self.runtime.block_on(f(&self.client))
    }

    /// Push a telemetry event to the dedicated drainer task.
    ///
    /// Non-blocking and lossy: if the channel is full (drainer fell
    /// behind under sustained load) the event is dropped to preserve the
    /// fire-and-forget contract, and [`tracing::debug`] metadata is recorded
    /// (target `odbc::telemetry`; see `debug_log_telemetry_dropped_queue_full`). Holds
    /// only the brief channel-internal critical section, so it is safe to
    /// call while holding the [`global()`] read guard.
    pub fn record_telemetry(&self, event: TelemetryEvent) {
        match self.telemetry_tx.try_send(event) {
            Ok(()) => {}
            Err(TrySendError::Full(ev)) => {
                debug_log_telemetry_dropped_queue_full(&ev, TELEMETRY_QUEUE_CAPACITY);
            }
            Err(TrySendError::Closed(_ev)) => {
                tracing::debug!(
                    target: "odbc::telemetry",
                    telemetry_event = "channel_closed",
                    "in-band telemetry dropped: channel closed (driver shutting down)"
                );
            }
        }
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
        // Single-worker runtime dedicated to the telemetry drainer task
        // (see the `OdbcGlobals` doc-comment for why the split).
        let telemetry_runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("odbc-telemetry")
            .enable_all()
            .build()
            .context(RuntimeCreationSnafu)?;
        let client = Arc::new(database_driver_client_with(providers));
        let (telemetry_tx, telemetry_rx) = mpsc::channel(TELEMETRY_QUEUE_CAPACITY);
        // Long-lived drainer: returns when `telemetry_tx` is dropped at
        // env teardown (the only `Sender` lives in `OdbcGlobals`).
        let drain_client = Arc::clone(&client);
        telemetry_runtime.spawn(drain_telemetry(telemetry_rx, drain_client));
        guard.globals = Some(OdbcGlobals {
            runtime,
            telemetry_runtime: Some(telemetry_runtime),
            client,
            telemetry_tx,
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
        let globals = guard.globals.take();
        drop(guard);
        drop(globals);
    }
    Ok(())
}
