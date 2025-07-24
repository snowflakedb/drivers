extern crate sf_core;

mod test_utils;

use arrow::array::{Array, StringArray};
use sf_core::api_client::new_database_driver_v1_client;
use sf_core::api_server::database_driver_v1::DatabaseDriverV1;
use sf_core::thrift_gen::database_driver_v1::DatabaseDriverSyncHandler;
use sf_core::thrift_gen::database_driver_v1::InfoCode;
use std::io::Write;
use tempfile::NamedTempFile;
use test_utils::{ArrowResultHelper, SnowflakeTestClient, get_parameters, setup_logging};

// Database operation tests
#[test]
fn test_database_new_and_release() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_release(db).unwrap();
}

#[test]
fn test_database_set_option_string() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client
        .database_set_option_string(
            db.clone(),
            "test_option".to_string(),
            "test_value".to_string(),
        )
        .unwrap();
    client.database_release(db).unwrap();
}

#[test]
fn test_database_set_option_bytes() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    let test_bytes = vec![1, 2, 3, 4, 5];
    client
        .database_set_option_bytes(db.clone(), "test_option".to_string(), test_bytes)
        .unwrap();
    client.database_release(db).unwrap();
}

#[test]
fn test_database_set_option_int() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client
        .database_set_option_int(db.clone(), "test_option".to_string(), 42)
        .unwrap();
    client.database_release(db).unwrap();
}

#[test]
fn test_database_set_option_double() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client
        .database_set_option_double(
            db.clone(),
            "test_option".to_string(),
            std::f64::consts::PI.into(),
        )
        .unwrap();
    client.database_release(db).unwrap();
}

#[test]
fn test_database_init() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();
    client.database_release(db).unwrap();
}

#[test]
fn test_database_lifecycle() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    // Create database
    let db = client.database_new().unwrap();

    // Set various options
    client
        .database_set_option_string(db.clone(), "driver".to_string(), "test_driver".to_string())
        .unwrap();
    client
        .database_set_option_int(db.clone(), "timeout".to_string(), 30)
        .unwrap();

    // Initialize database
    client.database_init(db.clone()).unwrap();

    // Release database
    client.database_release(db).unwrap();
}

// Connection operation tests
#[test]
fn test_connection_new_and_release() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let conn = client.connection_new().unwrap();

    client.connection_release(conn).unwrap();
}

#[test]
fn test_connection_set_option_string() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let conn = client.connection_new().unwrap();
    client
        .connection_set_option_string(
            conn.clone(),
            "username".to_string(),
            "test_user".to_string(),
        )
        .unwrap();
    client.connection_release(conn).unwrap();
}

#[test]
fn test_connection_set_option_bytes() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let conn = client.connection_new().unwrap();
    let test_bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
    client
        .connection_set_option_bytes(conn.clone(), "cert".to_string(), test_bytes)
        .unwrap();
    client.connection_release(conn).unwrap();
}

#[test]
fn test_connection_set_option_int() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let conn = client.connection_new().unwrap();
    client
        .connection_set_option_int(conn.clone(), "port".to_string(), 5432)
        .unwrap();
    client.connection_release(conn).unwrap();
}

#[test]
fn test_connection_set_option_double() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let conn = client.connection_new().unwrap();
    client
        .connection_set_option_double(conn.clone(), "timeout_seconds".to_string(), 30.5.into())
        .unwrap();
    client.connection_release(conn).unwrap();
}

#[test]
#[ignore]
fn test_connection_init() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_connection_get_info() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let info_codes = vec![InfoCode::DRIVER_NAME, InfoCode::DRIVER_VERSION];
    let _info_result = client
        .connection_get_info(conn.clone(), info_codes)
        .unwrap();

    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_connection_get_objects() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let _objects = client
        .connection_get_objects(
            conn.clone(),
            1, // depth
            "catalog".to_string(),
            "schema".to_string(),
            "table".to_string(),
            vec!["TABLE".to_string()],
            "column".to_string(),
        )
        .unwrap();

    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_connection_get_table_schema() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let _schema = client
        .connection_get_table_schema(
            conn.clone(),
            "catalog".to_string(),
            "schema".to_string(),
            "table".to_string(),
        )
        .unwrap();

    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_connection_get_table_types() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let _table_types = client.connection_get_table_types(conn.clone()).unwrap();

    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_connection_commit() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    client.connection_commit(conn.clone()).unwrap();

    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_connection_rollback() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    client.connection_rollback(conn.clone()).unwrap();

    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_connection_lifecycle() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    // Setup database
    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    // Create connection
    let conn = client.connection_new().unwrap();

    // Set connection options
    client
        .connection_set_option_string(conn.clone(), "host".to_string(), "localhost".to_string())
        .unwrap();
    client
        .connection_set_option_int(conn.clone(), "port".to_string(), 5432)
        .unwrap();
    client
        .connection_set_option_string(
            conn.clone(),
            "username".to_string(),
            "test_user".to_string(),
        )
        .unwrap();

    // Initialize connection
    client.connection_init(conn.clone(), db.clone()).unwrap();

    // Get driver info
    let info_codes = vec![InfoCode::DRIVER_NAME, InfoCode::DRIVER_VERSION];
    let _info = client
        .connection_get_info(conn.clone(), info_codes)
        .unwrap();

    // Get table types
    let _table_types = client.connection_get_table_types(conn.clone()).unwrap();

    // Release connection
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

// Statement operation tests
#[test]
#[ignore]
fn test_statement_new_and_release() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let stmt = client.statement_new(conn.clone()).unwrap();

    client.statement_release(stmt).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_statement_set_sql_query() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let stmt = client.statement_new(conn.clone()).unwrap();
    client
        .statement_set_sql_query(stmt.clone(), "SELECT 1".to_string())
        .unwrap();

    client.statement_release(stmt).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_statement_set_substrait_plan() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let stmt = client.statement_new(conn.clone()).unwrap();
    let plan_bytes = vec![0x00, 0x01, 0x02, 0x03]; // Mock substrait plan
    client
        .statement_set_substrait_plan(stmt.clone(), plan_bytes)
        .unwrap();

    client.statement_release(stmt).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_statement_prepare() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let stmt = client.statement_new(conn.clone()).unwrap();
    client
        .statement_set_sql_query(stmt.clone(), "SELECT ? as value".to_string())
        .unwrap();
    client.statement_prepare(stmt.clone()).unwrap();

    client.statement_release(stmt).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_statement_set_option_string() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let stmt = client.statement_new(conn.clone()).unwrap();
    client
        .statement_set_option_string(stmt.clone(), "query_timeout".to_string(), "30".to_string())
        .unwrap();

    client.statement_release(stmt).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_statement_set_option_bytes() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let stmt = client.statement_new(conn.clone()).unwrap();
    let option_bytes = vec![0xFF, 0xFE, 0xFD];
    client
        .statement_set_option_bytes(stmt.clone(), "binary_option".to_string(), option_bytes)
        .unwrap();

    client.statement_release(stmt).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_statement_set_option_int() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let stmt = client.statement_new(conn.clone()).unwrap();
    client
        .statement_set_option_int(stmt.clone(), "max_rows".to_string(), 1000)
        .unwrap();

    client.statement_release(stmt).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_statement_set_option_double() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let stmt = client.statement_new(conn.clone()).unwrap();
    client
        .statement_set_option_double(stmt.clone(), "timeout_seconds".to_string(), 30.5.into())
        .unwrap();

    client.statement_release(stmt).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_statement_get_parameter_schema() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let stmt = client.statement_new(conn.clone()).unwrap();
    client
        .statement_set_sql_query(stmt.clone(), "SELECT ? as value".to_string())
        .unwrap();
    client.statement_prepare(stmt.clone()).unwrap();

    let _schema = client.statement_get_parameter_schema(stmt.clone()).unwrap();

    client.statement_release(stmt).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_statement_bind() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let stmt = client.statement_new(conn.clone()).unwrap();
    client
        .statement_set_sql_query(stmt.clone(), "SELECT ? as value".to_string())
        .unwrap();
    client.statement_prepare(stmt.clone()).unwrap();

    // Mock Arrow RecordBatch in IPC format
    let record_batch_bytes = vec![0x41, 0x52, 0x52, 0x4F, 0x57]; // "ARROW" magic bytes
    client
        .statement_bind(stmt.clone(), record_batch_bytes)
        .unwrap();

    client.statement_release(stmt).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_statement_bind_stream() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let stmt = client.statement_new(conn.clone()).unwrap();
    client
        .statement_set_sql_query(stmt.clone(), "INSERT INTO table VALUES (?)".to_string())
        .unwrap();

    // Mock Arrow stream in IPC format
    let stream_bytes = vec![0x41, 0x52, 0x52, 0x4F, 0x57, 0x31]; // Mock stream
    client
        .statement_bind_stream(stmt.clone(), stream_bytes)
        .unwrap();

    client.statement_release(stmt).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_statement_execute_query() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let stmt = client.statement_new(conn.clone()).unwrap();
    client
        .statement_set_sql_query(stmt.clone(), "SELECT 1 as value".to_string())
        .unwrap();

    client.statement_execute_query(stmt.clone()).unwrap();

    client.statement_release(stmt).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_statement_execute_partitions() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let stmt = client.statement_new(conn.clone()).unwrap();
    client
        .statement_set_sql_query(stmt.clone(), "SELECT * FROM large_table".to_string())
        .unwrap();

    let result = client.statement_execute_partitions(stmt.clone()).unwrap();
    assert!(result.schema > 0); // Should have a valid schema pointer
    assert!(!result.partitions.is_empty()); // Should have partition descriptors

    client.statement_release(stmt).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_statement_read_partition() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    let conn = client.connection_new().unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    let stmt = client.statement_new(conn.clone()).unwrap();
    client
        .statement_set_sql_query(stmt.clone(), "SELECT * FROM large_table".to_string())
        .unwrap();

    let partitions = client.statement_execute_partitions(stmt.clone()).unwrap();
    if !partitions.partitions.is_empty() {
        let partition_descriptor = partitions.partitions[0].clone();
        let _stream_ptr = client
            .statement_read_partition(stmt.clone(), partition_descriptor)
            .unwrap();
    }

    client.statement_release(stmt).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_statement_lifecycle() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    // Setup database
    let db = client.database_new().unwrap();
    client.database_init(db.clone()).unwrap();

    // Create connection
    let conn = client.connection_new().unwrap();

    // Set connection options
    client
        .connection_set_option_string(conn.clone(), "host".to_string(), "localhost".to_string())
        .unwrap();
    client
        .connection_set_option_int(conn.clone(), "port".to_string(), 5432)
        .unwrap();
    client
        .connection_set_option_string(
            conn.clone(),
            "username".to_string(),
            "test_user".to_string(),
        )
        .unwrap();

    // Initialize connection
    client.connection_init(conn.clone(), db.clone()).unwrap();

    // Create statement
    let stmt = client.statement_new(conn.clone()).unwrap();

    // Set statement options
    client
        .statement_set_option_int(stmt.clone(), "max_rows".to_string(), 100)
        .unwrap();
    client
        .statement_set_option_string(stmt.clone(), "query_timeout".to_string(), "30".to_string())
        .unwrap();

    // Set and prepare query
    client
        .statement_set_sql_query(stmt.clone(), "SELECT ? as value, ? as name".to_string())
        .unwrap();
    client.statement_prepare(stmt.clone()).unwrap();

    // Get parameter schema
    let _param_schema = client.statement_get_parameter_schema(stmt.clone()).unwrap();

    // Bind parameters
    let record_batch_bytes = vec![0x41, 0x52, 0x52, 0x4F, 0x57]; // Mock data
    client
        .statement_bind(stmt.clone(), record_batch_bytes)
        .unwrap();

    // Execute query
    client.statement_execute_query(stmt.clone()).unwrap();

    // Clean up
    client.statement_release(stmt).unwrap();
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
#[ignore]
fn test_full_adbc_workflow() {
    setup_logging();
    let mut client = new_database_driver_v1_client();

    // Database lifecycle
    let db = client.database_new().unwrap();
    client
        .database_set_option_string(db.clone(), "driver".to_string(), "test_driver".to_string())
        .unwrap();
    client.database_init(db.clone()).unwrap();

    // Connection lifecycle
    let conn = client.connection_new().unwrap();
    client
        .connection_set_option_string(conn.clone(), "host".to_string(), "localhost".to_string())
        .unwrap();
    client.connection_init(conn.clone(), db.clone()).unwrap();

    // Get driver info
    let info_codes = vec![
        InfoCode::DRIVER_NAME,
        InfoCode::DRIVER_VERSION,
        InfoCode::VENDOR_NAME,
    ];
    let _info = client
        .connection_get_info(conn.clone(), info_codes)
        .unwrap();

    // Statement lifecycle for DDL
    let ddl_stmt = client.statement_new(conn.clone()).unwrap();
    client
        .statement_set_sql_query(
            ddl_stmt.clone(),
            "CREATE TABLE test (id INT, name TEXT)".to_string(),
        )
        .unwrap();
    let _ddl_result = client.statement_execute_query(ddl_stmt.clone()).unwrap();
    client.statement_release(ddl_stmt).unwrap();

    // Statement lifecycle for INSERT
    let insert_stmt = client.statement_new(conn.clone()).unwrap();
    client
        .statement_set_sql_query(
            insert_stmt.clone(),
            "INSERT INTO test VALUES (?, ?)".to_string(),
        )
        .unwrap();
    client.statement_prepare(insert_stmt.clone()).unwrap();

    let record_batch = vec![0x41, 0x52, 0x52, 0x4F, 0x57]; // Mock Arrow data
    client
        .statement_bind(insert_stmt.clone(), record_batch)
        .unwrap();
    let _insert_result = client.statement_execute_query(insert_stmt.clone()).unwrap();
    client.statement_release(insert_stmt).unwrap();

    // Statement lifecycle for SELECT
    let select_stmt = client.statement_new(conn.clone()).unwrap();
    client
        .statement_set_sql_query(select_stmt.clone(), "SELECT * FROM test".to_string())
        .unwrap();
    client.statement_execute_query(select_stmt.clone()).unwrap();
    client.statement_release(select_stmt).unwrap();

    // Transaction operations
    client.connection_commit(conn.clone()).unwrap();

    // Cleanup
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}

#[test]
fn test_snowflake_connection_settings() {
    setup_logging();
    let driver = DatabaseDriverV1::new();

    let db_handle = driver.handle_database_new().unwrap();
    driver.handle_database_init(db_handle.clone()).unwrap();

    // Get credentials from parameters.json
    let parameters = get_parameters();
    let account_name = parameters.account_name.clone().unwrap();
    let user = parameters.user.clone().unwrap();
    let password = parameters.password.clone().unwrap();

    // Create a new connection
    let conn_handle = driver.handle_connection_new().unwrap();

    // Set required connection settings
    driver
        .handle_connection_set_option_string(
            conn_handle.clone(),
            "account".to_string(),
            account_name,
        )
        .unwrap();

    driver
        .handle_connection_set_option_string(conn_handle.clone(), "user".to_string(), user)
        .unwrap();

    driver
        .handle_connection_set_option_string(conn_handle.clone(), "password".to_string(), password)
        .unwrap();

    if let Some(server_url) = parameters.server_url.clone() {
        driver
            .handle_connection_set_option_string(
                conn_handle.clone(),
                "server_url".to_string(),
                server_url,
            )
            .unwrap();
    }

    // Attempt to initialize the connection with real credentials
    let result = driver.handle_connection_init(conn_handle.clone(), db_handle.clone());
    println!("result: {result:?}");
    assert!(result.is_ok());
    driver.handle_connection_release(conn_handle).unwrap();
}

#[test]
fn test_snowflake_select_1() {
    let mut client = SnowflakeTestClient::new();
    let result = client.execute_query("SELECT 1");

    let mut arrow_helper = ArrowResultHelper::from_result(result);
    let value = arrow_helper.first_int_value();
    assert_eq!(value, 1);
}

#[test]
fn test_create_temporary_stage() {
    let mut client = SnowflakeTestClient::new();
    let stage_name = "TEST_STAGE";
    let result = client.execute_query(&format!("create temporary stage {stage_name}"));

    let mut arrow_helper = ArrowResultHelper::from_result(result);
    let batch = arrow_helper.assert_single_row();
    let expected_message = format!("Stage area {stage_name} successfully created.");

    // Extract the string value from the batch
    let array_ref = batch.column(0);
    let string_array = array_ref
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("Expected string array");
    let message = string_array.value(0).to_string();

    assert_eq!(
        message, expected_message,
        "Expected stage creation success message"
    );
}

#[test]
fn test_put() {
    let mut client = SnowflakeTestClient::new();
    let stage_name = "TEST_STAGE_PUT";

    // Create temporary stage
    client.execute_query(&format!("create temporary stage {stage_name}"));

    // Create test file
    let mut test_file = NamedTempFile::new().unwrap();
    test_file.write_all("test\n".as_bytes()).unwrap();
    test_file.flush().unwrap();

    // Execute PUT command
    let put_sql = format!(
        "PUT 'file://{test_file}' @{stage_name}",
        test_file = test_file.path().to_str().unwrap().replace("\\", "/")
    );
    client.execute_query(&put_sql);

    // Verify file was uploaded with LS command
    let ls_result = client.execute_query(&format!("LS @{stage_name}"));

    // Parse Arrow result to verify file listing
    let mut arrow_helper = ArrowResultHelper::from_result(ls_result);
    let batch = arrow_helper.assert_single_row();

    // Verify LS result structure: [name, size, md5, last_modified]
    assert_eq!(batch.num_columns(), 4, "LS should return 4 columns");

    // Check file name (column 0)
    let name_array = batch.column(0);
    assert_eq!(
        name_array.data_type(),
        &arrow::datatypes::DataType::Utf8,
        "File name should be string"
    );
    let name_str = name_array
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap()
        .value(0);

    let temp_filename = test_file.path().file_name().unwrap().to_str().unwrap();
    let expected_file_name = format!("{temp_filename}.gz");
    let expected_full_path = format!("{}/{expected_file_name}", stage_name.to_lowercase());
    assert_eq!(
        name_str, expected_full_path,
        "File name should match uploaded file"
    );

    assert!(
        name_str.ends_with(".gz"),
        "File should be compressed with .gz"
    );
}

#[test]
fn test_get() {
    let mut client = SnowflakeTestClient::new();
    let stage_name = "TEST_STAGE_GET";

    // Create test file and temporary stage
    let mut test_file = NamedTempFile::new().unwrap();
    test_file.write_all("a,b,c\n1,2,3\n".as_bytes()).unwrap();
    test_file.flush().unwrap();
    client.execute_query(&format!("create temporary stage {stage_name}"));

    // Upload file using PUT (which now works)
    let put_sql = format!(
        "PUT 'file://{test_file}' @{stage_name}",
        test_file = test_file.path().to_str().unwrap().replace("\\", "/")
    );
    client.execute_query(&put_sql);

    // Try to download the file using GET (should fail)
    let temp_filename = test_file.path().file_name().unwrap().to_str().unwrap();
    let get_sql = format!("GET @{stage_name}/{temp_filename}.gz file://./downloaded/");
    client.execute_query_expect_error(&get_sql, "Handling GET queries is not yet implemented");
    println!("GET correctly failed with expected error: not yet implemented");
}
