//! Inside XP with nothing registered: fail fast. Own binary because the
//! process-wide C registration slot must stay empty until this assertion runs.
//! XP hosts exist only on Linux.

#[path = "common/crypto_provider.rs"]
mod crypto_provider;

#[cfg(not(target_os = "linux"))]
#[path = "xp_backend/link_sf_core.rs"]
mod link_sf_core;

#[cfg(target_os = "linux")]
#[path = "xp_backend/fake_host.rs"]
mod fake_host;
#[cfg(target_os = "linux")]
#[path = "xp_backend/fixtures.rs"]
mod fixtures;

#[cfg(target_os = "linux")]
use std::collections::HashMap;

#[cfg(target_os = "linux")]
use sf_core::apis::database_driver_v1::{ApiError, DatabaseDriverV1};
#[cfg(target_os = "linux")]
use sf_core::rest::snowflake::RestError;
#[cfg(target_os = "linux")]
use sf_core::xp_backend::error_codes;
#[cfg(target_os = "linux")]
use sf_core::xp_backend::ffi::sf_core_register_xp_backend;

#[cfg(target_os = "linux")]
use fake_host::{HostState, callbacks_for};
#[cfg(target_os = "linux")]
use fixtures::set_running_inside_xp;

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "current_thread")]
async fn xp_mode_without_a_registration_fails_fast() {
    set_running_inside_xp();
    let driver = DatabaseDriverV1::new();

    let err = match driver.xp_backend() {
        Err(err) => err,
        Ok(_) => panic!("xp mode with no backend must not fall back to HTTP"),
    };
    assert_eq!(err.code, error_codes::NOT_REGISTERED);
    assert!(
        err.message
            .contains(sf_core::env_vars::SNOWFLAKE_RUNNING_INSIDE_XP)
            && err.message.contains("register the XP backend"),
        "the message must name both the variable and the missing registration: {}",
        err.message
    );

    let db = driver.database_new();
    driver.database_init(db).expect("database handle");
    let conn = driver.connection_new();
    driver
        .connection_set_options(conn, HashMap::new(), false, None)
        .await
        .expect("host-backend placeholders should satisfy option validation");
    let err = driver
        .connection_init(None, conn, db)
        .await
        .expect_err("connection_init must fail fast rather than fall back to HTTP");
    match err {
        ApiError::Login { source, .. } => match source.as_ref() {
            RestError::Backend { source, .. } => {
                assert_eq!(source.code, error_codes::NOT_REGISTERED);
            }
            other => panic!("expected RestError::Backend, got: {other:?}"),
        },
        other => panic!("expected ApiError::Login, got: {other:?}"),
    }
    driver.connection_release(conn).expect("release connection");
    driver.database_release(db).expect("release database");

    let host = HostState::leaked();
    let callbacks = callbacks_for(host);
    assert_eq!(
        unsafe { sf_core_register_xp_backend(&callbacks) },
        0,
        "a failed lookup does not lock the slot; registration can still attach"
    );
    driver
        .xp_backend()
        .expect("late registration should attach")
        .expect("XP mode should use the C adapter");
}
