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

mod class_constructors;
mod connection;
mod date_format;
mod error;
mod logging;
mod query;
mod query_status;
mod rows;
mod session;
mod session_params;
mod session_state;
mod statement;
mod telemetry;
mod token_cache;
mod validation_utils;

pub use connection::Connection;
pub use date_format::format_snowflake_date;
pub use session_params::KnownSessionParameters;
pub use session_state::SessionState;
pub use statement::{Column, Statement};
pub use token_cache::set_custom_credential_manager;

use logging::JsLogging;
use napi::bindgen_prelude::{Function, Object};
use napi::{Env, Result};
use napi_derive::napi;
use sf_core::apis::database_driver_v1::{DatabaseDriverV1, DriverProviders, WrapperPresets};
use sf_core::logging::LogManager;
use sf_core::token_cache::TokenCache;
use std::sync::{Arc, LazyLock};

static BRIDGE: LazyLock<Bridge> = LazyLock::new(Bridge::new);

struct Bridge {
    driver: DatabaseDriverV1,
    token_cache: Arc<token_cache::NodeTokenCache>,
    troubleshooting: bool,
    logging: JsLogging,
}

impl Bridge {
    fn new() -> Self {
        let token_cache = Arc::new(token_cache::NodeTokenCache::new());
        let logging = JsLogging::default();
        let log_manager = logging.log_manager();
        let troubleshooting = log_manager
            .as_ref()
            .is_some_and(LogManager::is_troubleshooting);
        Self {
            driver: DatabaseDriverV1::with_providers(DriverProviders {
                log_manager,
                wrapper_presets: WrapperPresets::nodejs(),
                token_cache: Some(Arc::clone(&token_cache) as Arc<dyn TokenCache>),
                ..Default::default()
            }),
            token_cache,
            troubleshooting,
            logging,
        }
    }
}

#[napi(object)]
pub struct FipsStatus {
    pub is_fips: bool,
}

#[napi]
pub fn get_fips_status() -> FipsStatus {
    let status = sf_core::tls::fips_status();
    FipsStatus {
        is_fips: status.is_fips,
    }
}

#[napi(object)]
pub struct InitBridgeResult {
    pub troubleshooting: bool,
}

/// Connects the bridge to the JS side. The package entry point calls it once at load.
///
/// `classConstructors` holds the JS classes the bridge creates but napi cannot define:
/// `SnowflakeDate` extends `Date`, and napi classes can't extend built-in JS classes, so the
/// class lives in TypeScript and the bridge calls its constructor. The constructors are
/// stored per JS env, because napi references don't work across envs.
///
/// `logCallback` is how log records reach the JS logger. Every record goes through sf_core
/// first, including records logged in JS, which JS sends in with `logEvent`. sf_core writes
/// them to its own outputs, such as the troubleshooting file, then calls `logCallback`
/// with each record allowed by the level set with `setLogLevel`.
///
/// The result tells JS whether sf_core runs in troubleshooting mode. In that mode JS also
/// sends records below the configured level, so they reach the troubleshooting file.
#[napi(
    ts_args_type = "classConstructors: { SnowflakeDate: new (epochMillis: number, nanos: number, scale: number, timezone: string | number, format: string) => unknown }, logCallback: (arg: LogEvent) => void"
)]
pub fn init_bridge(
    env: Env,
    class_constructors: Object,
    log_callback: Function<'static, logging::LogEvent, ()>,
) -> Result<InitBridgeResult> {
    class_constructors::register_class_constructors(env, class_constructors)?;
    logging::init(log_callback)?;
    Ok(InitBridgeResult {
        troubleshooting: BRIDGE.troubleshooting,
    })
}
