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

use sf_core::thrift_apis::DatabaseDriverV1;
use sf_core::thrift_apis::client::create_client;

#[test]
fn smoke_connection_set_tls_config() {
    let mut client = create_client::<DatabaseDriverV1>();
    let db = client.database_new().expect("database_new ok");
    client.database_init(db.clone()).expect("database_init ok");
    let conn = client.connection_new().expect("connection_new ok");

    // Option-based TLS/CRL configuration
    client
        .connection_set_option_string(
            conn.clone(),
            "verify_hostname".to_string(),
            "true".to_string(),
        )
        .expect("set verify_hostname");
    client
        .connection_set_option_string(
            conn.clone(),
            "verify_certificates".to_string(),
            "true".to_string(),
        )
        .expect("set verify_certificates");
    client
        .connection_set_option_string(conn.clone(), "crl_mode".to_string(), "ENABLED".to_string())
        .expect("set crl_mode");
    client
        .connection_set_option_int(conn.clone(), "crl_http_timeout".to_string(), 30)
        .expect("set crl_http_timeout");
    client
        .connection_set_option_int(conn.clone(), "crl_connection_timeout".to_string(), 10)
        .expect("set crl_connection_timeout");
}
