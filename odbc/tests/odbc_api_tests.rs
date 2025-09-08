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

use sf_core::api_client::new_database_driver_v1_client;
use sf_core::thrift_gen::database_driver_v1::{CertRevocationCheckMode, TlsConfig};

#[test]
fn smoke_connection_set_tls_config() {
    let mut client = new_database_driver_v1_client();
    let db = client.database_new().expect("database_new ok");
    client.database_init(db).expect("database_init ok");
    let conn = client.connection_new().expect("connection_new ok");

    let tls = TlsConfig {
        crl_mode: Some(CertRevocationCheckMode::ENABLED),
        crl_disk_caching: Some(true),
        crl_memory_caching: Some(true),
        crl_cache_dir: None,
        crl_validity_days: Some(10),
        allow_certs_without_crl_url: Some(true),
        crl_http_timeout_seconds: Some(30),
        crl_connection_timeout_seconds: Some(10),
        custom_root_store_path: None,
        verify_hostname: Some(true),
        verify_certificates: Some(true),
    };

    client
        .connection_set_tls_config(conn.clone(), tls)
        .expect("connection_set_tls_config ok");
}
