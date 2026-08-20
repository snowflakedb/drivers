//! Node.js bindings for `sf_core`.

// `Connection::execute` awaits `statement_execute_query`, whose future nests the
// whole query chain (statement setup → HTTP → result set) and alone adds ~130 to
// rustc's layout query depth — already within a frame or two of the default 128
// limit before anything in this crate is counted, so adding any item elsewhere
// tips it over. Raising the ceiling is compile-time only and costs nothing at
// runtime; the alternative is `Box::pin`ning that operation inside `sf_core`,
// which belongs with the work that restructures it (SNOW-3675196) rather than
// here.
//
// NOTE: dev-profile incremental builds cache these layout queries, so a
// regression here can compile fine locally after an earlier successful build.
// Reproduce with `CARGO_INCREMENTAL=0` against a cleaned crate.
#![recursion_limit = "256"]

mod connection;
mod error;
mod session_params;
mod sql_value;
mod statement;

pub use connection::Connection;
pub use statement::{Column, Statement};

use sf_core::apis::database_driver_v1::DatabaseDriverV1;
use std::sync::LazyLock;

pub(crate) static DRIVER: LazyLock<DatabaseDriverV1> = LazyLock::new(|| {
    // TODO:
    // Implement proper bidirectional logger with configurable level,  as is done by other driver wrappers.
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::DEBUG)
        .try_init();
    DatabaseDriverV1::new()
});
