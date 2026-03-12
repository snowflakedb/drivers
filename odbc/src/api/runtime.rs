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

pub fn env_allocated() {
    ENV_COUNT.fetch_add(1, Ordering::AcqRel);
    GLOBALS.get_or_init(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
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
