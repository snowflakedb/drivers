use std::future::Future;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use sf_core::protobuf::apis::database_driver_v1::{DatabaseDriverClient, database_driver_client};

pub struct OdbcGlobals {
    pub runtime: tokio::runtime::Runtime,
    pub client: DatabaseDriverClient,
}

static ENV_COUNT: AtomicUsize = AtomicUsize::new(0);
static GLOBALS: OnceLock<OdbcGlobals> = OnceLock::new();

pub fn global() -> &'static OdbcGlobals {
    GLOBALS
        .get()
        .expect("ODBC globals not initialized; allocate an environment handle first")
}

/// Spawn `future` on a tokio worker thread (8 MB stack) and block the calling
/// thread until it completes.
///
/// This solves two problems:
/// 1. Avoids polling large async state machines on the caller's stack, which
///    overflows on Windows (1 MB default thread stack).
/// 2. Uses a channel instead of `Runtime::block_on` so the calling thread never
///    enters the tokio runtime context.  This prevents `EnterGuard` conflicts
///    when downstream code (e.g. `ChunkReader`) calls `Handle::block_on`.
pub fn run<F, T>(future: F) -> T
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let g = global();
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    g.runtime.spawn(async move {
        let result = future.await;
        let _ = tx.send(result);
    });
    rx.recv()
        .expect("ODBC async task failed — possible panic in spawned task")
}

pub fn env_allocated() {
    ENV_COUNT.fetch_add(1, Ordering::AcqRel);
    GLOBALS.get_or_init(|| {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_stack_size(8 * 1024 * 1024)
            .enable_all()
            .build()
            .expect("Failed to create ODBC tokio runtime");
        let client = database_driver_client();
        OdbcGlobals { runtime, client }
    });
}

pub fn env_freed() {
    ENV_COUNT.fetch_sub(1, Ordering::AcqRel);
}
