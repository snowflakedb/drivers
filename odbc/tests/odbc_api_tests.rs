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
