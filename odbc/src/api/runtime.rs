use std::ops::Deref;
use std::sync::RwLock;

use sf_core::protobuf::apis::database_driver_v1::{DatabaseDriverClient, database_driver_client};
use snafu::{Location, ResultExt, Snafu};

pub struct OdbcGlobals {
    runtime: tokio::runtime::Runtime,
    client: DatabaseDriverClient,
}

impl OdbcGlobals {
    pub fn block_on<T>(&self, f: impl AsyncFnOnce(&DatabaseDriverClient) -> T) -> T {
        self.runtime.block_on(f(&self.client))
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
    guard.env_count += 1;
    if guard.globals.is_none() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .context(RuntimeCreationSnafu)?;
        let client = database_driver_client();
        guard.globals = Some(OdbcGlobals { runtime, client });
    }
    Ok(())
}

pub fn env_freed() -> Result<(), OdbcRuntimeError> {
    let mut guard = STATE.write().map_err(|_| LockPoisonedSnafu.build())?;
    guard.env_count = guard.env_count.saturating_sub(1);
    if guard.env_count == 0 {
        guard.globals = None;
    }
    Ok(())
}
