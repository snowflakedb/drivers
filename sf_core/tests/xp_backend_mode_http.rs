//! Outside XP the seam is inert even when a host has registered. Own binary
//! because C registration attaches to a process-wide slot. XP hosts exist only
//! on Linux.

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
use sf_core::apis::database_driver_v1::DatabaseDriverV1;
#[cfg(target_os = "linux")]
use sf_core::rest::snowflake::{QueryInput, QueryOptions, RestError, snowflake_query_with_client};
#[cfg(target_os = "linux")]
use sf_core::xp_backend::ffi::sf_core_register_xp_backend;

#[cfg(target_os = "linux")]
use fake_host::{HostState, callbacks_for};
#[cfg(target_os = "linux")]
use fixtures::{clear_running_inside_xp, query_parameters};

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "current_thread")]
async fn without_the_env_var_a_registered_backend_is_never_routed_to() {
    clear_running_inside_xp();
    assert!(std::env::var_os(sf_core::env_vars::SNOWFLAKE_RUNNING_INSIDE_XP).is_none());

    let driver = DatabaseDriverV1::new();

    let host = HostState::leaked();
    let callbacks = callbacks_for(host);

    assert_eq!(unsafe { sf_core_register_xp_backend(&callbacks) }, 0);
    let xp_backend = driver
        .xp_backend()
        .expect("HTTP mode should not require a backend");
    assert!(xp_backend.is_none());

    let err = snowflake_query_with_client(
        &reqwest::Client::new(),
        query_parameters(),
        "unused-session-token",
        QueryInput::new("select 1"),
        QueryOptions::default(),
        xp_backend.map(|backend| backend.as_ref()),
    )
    .await
    .map(|_| ())
    .expect_err("HTTP mode must not succeed against an unreachable server");
    assert!(
        !matches!(err, RestError::Backend { .. }),
        "a registered host must not serve queries when the env var is unset: {err:?}"
    );
    assert_eq!(host.call_count(), 0, "no callback may run outside XP");
}
