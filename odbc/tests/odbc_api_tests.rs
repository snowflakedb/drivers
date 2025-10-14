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
    protobuf_apis::database_driver_v1::DatabaseDriverClient,
    protobuf_gen::database_driver_v1::{
        ConnectionNewRequest, ConnectionSetOptionIntRequest, ConnectionSetOptionStringRequest,
        DatabaseInitRequest, DatabaseNewRequest,
    },
};

#[test]
fn smoke_connection_set_tls_config() {
    let db = DatabaseDriverClient::database_new(DatabaseNewRequest {}).expect("database_new ok");
    DatabaseDriverClient::database_init(DatabaseInitRequest {
        db_handle: db.db_handle,
    })
    .expect("database_init ok");
    let conn = DatabaseDriverClient::connection_new(ConnectionNewRequest {})
        .unwrap()
        .conn_handle
        .unwrap();

    // Option-based TLS/CRL configuration
    DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
        conn_handle: Some(conn),
        key: "verify_hostname".to_string(),
        value: "true".to_string(),
    })
    .expect("set verify_hostname");
    DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
        conn_handle: Some(conn),
        key: "verify_certificates".to_string(),
        value: "true".to_string(),
    })
    .expect("set verify_certificates");
    DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
        conn_handle: Some(conn),
        key: "crl_mode".to_string(),
        value: "ENABLED".to_string(),
    })
    .expect("set crl_mode");
    DatabaseDriverClient::connection_set_option_int(ConnectionSetOptionIntRequest {
        conn_handle: Some(conn),
        key: "crl_http_timeout".to_string(),
        value: 30,
    })
    .expect("set crl_http_timeout");
    DatabaseDriverClient::connection_set_option_int(ConnectionSetOptionIntRequest {
        conn_handle: Some(conn),
        key: "crl_connection_timeout".to_string(),
        value: 10,
    })
    .expect("set crl_connection_timeout");
}
