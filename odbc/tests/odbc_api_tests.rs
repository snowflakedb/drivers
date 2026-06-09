// extern crate odbc;
// extern crate odbc_sys;
// use odbc::api::*;
// use odbc_sys as sql;

// #[test]
// fn test_alloc_and_free_env_handle() {
//     let mut env_handle: sql::Handle = std::ptr::null_mut();

//     let ret = unsafe {
//         SQLAllocHandle(
//             sql::HandleType::Env,
//             std::ptr::null_mut(),
//             &mut env_handle as *mut sql::Handle,
//         )
//     };

//     assert_eq!(ret, sql::SqlReturn::SUCCESS);
//     assert!(!env_handle.is_null());

//     let ret = unsafe { SQLFreeHandle(sql::HandleType::Env, env_handle) };
//     assert_eq!(ret, sql::SqlReturn::SUCCESS);
// }

// #[test]
// #[ignore]
// fn test_connect_and_disconnect() {
//     let mut env_handle: sql::Handle = std::ptr::null_mut();
//     let ret = unsafe {
//         SQLAllocHandle(
//             sql::HandleType::Env,
//             std::ptr::null_mut(),
//             &mut env_handle as *mut sql::Handle,
//         )
//     };
//     assert_eq!(ret, sql::SqlReturn::SUCCESS);

//     let mut conn_handle: sql::Handle = std::ptr::null_mut();
//     let ret = unsafe {
//         SQLAllocHandle(
//             sql::HandleType::Dbc,
//             env_handle,
//             &mut conn_handle as *mut sql::Handle,
//         )
//     };
//     assert_eq!(ret, sql::SqlReturn::SUCCESS);

//     let server_name = "server_name";
//     let ret = unsafe {
//         SQLConnect(
//             conn_handle,
//             server_name.as_ptr(),
//             server_name.len() as sql::SmallInt,
//             std::ptr::null(),
//             0,
//             std::ptr::null(),
//             0,
//         )
//     };
//     assert_eq!(ret, sql::SqlReturn::SUCCESS);

//     let ret = unsafe { SQLDisconnect(conn_handle) };
//     assert_eq!(ret, sql::SqlReturn::SUCCESS);

//     let ret = unsafe { SQLFreeHandle(sql::HandleType::Dbc, conn_handle) };
//     assert_eq!(ret, sql::SqlReturn::SUCCESS);

//     let ret = unsafe { SQLFreeHandle(sql::HandleType::Env, env_handle) };
//     assert_eq!(ret, sql::SqlReturn::SUCCESS);
// }

use sf_core::{
    protobuf::apis::database_driver_v1::{DatabaseDriverClientBlockingExt, database_driver_client},
    protobuf::config_setting_ext::config_option,
    protobuf::generated::database_driver_v1::{
        ConnectionNewRequest, ConnectionSetOptionsRequest, DatabaseInitRequest, DatabaseNewRequest,
    },
};

#[test]
fn smoke_connection_set_tls_config() {
    let client = database_driver_client();
    let db = client
        .database_new_blocking(DatabaseNewRequest {})
        .expect("database_new ok");
    client
        .database_init_blocking(DatabaseInitRequest {
            db_handle: db.db_handle,
        })
        .expect("database_init ok");
    let conn = client
        .connection_new_blocking(ConnectionNewRequest {})
        .unwrap()
        .conn_handle
        .unwrap();

    client
        .connection_set_options_blocking(ConnectionSetOptionsRequest {
            conn_handle: Some(conn),
            options: vec![
                config_option("verify_hostname", "true"),
                config_option("verify_certificates", "true"),
                config_option("crl_mode", "ENABLED"),
                config_option("crl_http_timeout", 30_i64),
                config_option("crl_connection_timeout", 10_i64),
            ]
            .into_iter()
            .collect(),
        })
        .expect("set options");
}

#[test]
fn c_api_lifecycle_with_telemetry_shim_is_non_fatal() {
    use odbc_sys as sql;
    use sfodbc::c_api::*;

    let mut env: sql::Handle = std::ptr::null_mut();
    let rc = unsafe {
        SQLAllocHandle(
            sql::HandleType::Env,
            std::ptr::null_mut(),
            &mut env as *mut sql::Handle,
        )
    };
    assert_eq!(rc, sql::SqlReturn::SUCCESS.0, "SQLAllocHandle(Env) failed");
    assert!(!env.is_null());

    let mut dbc: sql::Handle = std::ptr::null_mut();
    let rc = unsafe { SQLAllocHandle(sql::HandleType::Dbc, env, &mut dbc as *mut sql::Handle) };
    assert_eq!(rc, sql::SqlReturn::SUCCESS.0, "SQLAllocHandle(Dbc) failed");
    assert!(!dbc.is_null());

    // Statement allocation on a still-Disconnected Dbc exercises the
    // resolver against both the Dbc registry (state == Disconnected) and
    // the Stmt-via-Dbc lookup if allocation happens to succeed. Either way
    // the telemetry shim must not panic, regardless of the SQL return code.
    let mut stmt: sql::Handle = std::ptr::null_mut();
    let alloc_rc =
        unsafe { SQLAllocHandle(sql::HandleType::Stmt, dbc, &mut stmt as *mut sql::Handle) };
    if alloc_rc == sql::SqlReturn::SUCCESS.0 {
        assert!(!stmt.is_null(), "SUCCESS must populate output handle");
        let rc = unsafe { SQLFreeHandle(sql::HandleType::Stmt, stmt) };
        assert_eq!(rc, sql::SqlReturn::SUCCESS.0, "SQLFreeHandle(Stmt) failed");
    } else {
        assert_eq!(
            alloc_rc,
            sql::SqlReturn::ERROR.0,
            "Stmt-on-Disconnected-Dbc must either succeed or return ERROR (not panic / not INVALID_HANDLE)"
        );
    }

    // Tear down in reverse order. Each `SQLFreeHandle` is itself
    // instrumented — once a handle leaves its registry the resolver
    // no-ops on the next call.
    let rc = unsafe { SQLFreeHandle(sql::HandleType::Dbc, dbc) };
    assert_eq!(rc, sql::SqlReturn::SUCCESS.0, "SQLFreeHandle(Dbc) failed");
    let rc = unsafe { SQLFreeHandle(sql::HandleType::Env, env) };
    assert_eq!(rc, sql::SqlReturn::SUCCESS.0, "SQLFreeHandle(Env) failed");

    // After the last env is freed the runtime is torn down (`env_freed`
    // sets globals back to None). Subsequent SQL* calls must still be
    // non-fatal — the telemetry shim's `global()?` short-circuits cleanly.
    let rc = unsafe { SQLFreeStmt(std::ptr::null_mut(), 0) };
    assert_eq!(
        rc,
        sql::SqlReturn::INVALID_HANDLE.0,
        "SQLFreeStmt on null handle should return INVALID_HANDLE"
    );
}

#[test]
fn telemetry_rpcs_accept_unknown_handles_silently() {
    use sf_core::protobuf::generated::database_driver_v1::{
        ConnectionHandle, TelemetrySendApiUsageRequest, TelemetrySendWrapperErrorRequest,
    };
    let client = database_driver_client();
    let conn = ConnectionHandle {
        id: 0xDEAD,
        magic: 0,
    };

    // Each call is driven through its own `*_blocking` adapter — mirrors
    // how production ODBC telemetry code (`odbc/src/api/telemetry.rs`)
    // dispatches each RPC inside its own `block_on` scope, and avoids
    // composing two large futures into one async block (which overflows
    // the layout query depth for this generated client).
    client
        .telemetry_send_api_usage_blocking(TelemetrySendApiUsageRequest {
            conn_handle: Some(conn),
            api_method: "SQLExecDirect".to_string(),
        })
        .expect("telemetry_send_api_usage on unknown handle must not error");
    client
        .telemetry_send_wrapper_error_blocking(TelemetrySendWrapperErrorRequest {
            conn_handle: Some(conn),
            exception_type: "ConversionError".to_string(),
            // Wire format must match what `ErrorSource::DataConversion`
            // serialises to via `Display` (snake_case). Hardcoded here
            // because the `api` module isn't publicly re-exported;
            // the round-trip is enforced by the
            // `error_source_wire_format_round_trips` unit test.
            error_source: "data_conversion".to_string(),
        })
        .expect("telemetry_send_wrapper_error on unknown handle must not error");
}

// End-to-end proxy DSN coverage lives in
// `odbc_tests/tests/e2e/session/proxy.cpp`, which exercises the full SQL*
// connect path through a wiremock forward proxy (see Jakub's review on PR
// #1223). The Rust unit tests in `api/connection.rs::tests` cover DSN
// parsing/normalisation; sf_core's `ProxyConfig::from_settings` covers
// the URL-vs-fields merge.
