extern crate lazy_static;
extern crate sf_core;
extern crate tracing;
extern crate tracing_subscriber;

use arrow::array::{Array, Int8Array};
use arrow::ffi_stream::ArrowArrayStreamReader;
use arrow::ffi_stream::FFI_ArrowArrayStream;
use sf_core::api_client::new_database_driver_v1_client;
use sf_core::api_server::database_driver_v1::DatabaseDriverV1;
use sf_core::thrift_gen::database_driver_v1::DatabaseDriverSyncHandler;
use sf_core::thrift_gen::database_driver_v1::InfoCode;
use tracing::Level;
use tracing_subscriber::EnvFilter;

// Use serde to parse parameters.json
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct ParametersFile {
    testconnection: Parameters,
}

#[derive(Deserialize, Serialize)]
struct Parameters {
    #[serde(rename = "SNOWFLAKE_TEST_ACCOUNT")]
    account_name: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_USER")]
    user: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_PASSWORD")]
    password: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_DATABASE")]
    database: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_SCHEMA")]
    schema: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_WAREHOUSE")]
    warehouse: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_HOST")]
    host: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_ROLE")]
    role: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_SERVER_URL")]
    server_url: Option<String>,
}

use lazy_static::lazy_static;
use std::fs;

lazy_static! {
    static ref PARAMETERS: Parameters = {
        let parameter_path = std::env::var("PARAMETER_PATH").unwrap();
        println!("Parameter path: {parameter_path}");
        let parameters = fs::read_to_string(parameter_path).unwrap();
        let parameters: ParametersFile = serde_json::from_str(&parameters).unwrap();
        println!(
            "Parameters: {:?}",
            serde_json::to_string_pretty(&parameters).unwrap()
        );
        parameters.testconnection
    };
}

fn setup_logging() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::DEBUG.into())
        .from_env()
        .unwrap();
    let _ = tracing_subscriber::fmt::fmt()
        .with_env_filter(env_filter)
        .try_init();
}

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
    let account_name = PARAMETERS.account_name.clone().unwrap();
    let user = PARAMETERS.user.clone().unwrap();
    let password = PARAMETERS.password.clone().unwrap();

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

    if let Some(server_url) = PARAMETERS.server_url.clone() {
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
    setup_logging();
    let mut driver = new_database_driver_v1_client();
    let db_handle = driver.database_new().unwrap();
    driver.database_init(db_handle.clone()).unwrap();

    let conn_handle = driver.connection_new().unwrap();
    driver
        .connection_set_option_string(
            conn_handle.clone(),
            "account".to_string(),
            PARAMETERS.account_name.clone().unwrap(),
        )
        .unwrap();
    driver
        .connection_set_option_string(
            conn_handle.clone(),
            "user".to_string(),
            PARAMETERS.user.clone().unwrap(),
        )
        .unwrap();
    driver
        .connection_set_option_string(
            conn_handle.clone(),
            "password".to_string(),
            PARAMETERS.password.clone().unwrap(),
        )
        .unwrap();
    // driver.connection_set_option_string(conn_handle.clone(), "server_url".to_string(), PARAMETERS.server_url.clone().unwrap()).unwrap();
    driver
        .connection_init(conn_handle.clone(), db_handle.clone())
        .unwrap();
    let stmt_handle = driver.statement_new(conn_handle.clone()).unwrap();
    driver
        .statement_set_sql_query(stmt_handle.clone(), "SELECT 1".to_string())
        .unwrap();
    let result = driver.statement_execute_query(stmt_handle.clone()).unwrap();
    println!("result: {result:?}");
    let stream_ptr: *mut FFI_ArrowArrayStream = result.stream.into();
    println!("stream_ptr: {stream_ptr:?}");
    let stream: FFI_ArrowArrayStream = unsafe { FFI_ArrowArrayStream::from_raw(stream_ptr) };
    let mut reader = ArrowArrayStreamReader::try_new(stream).unwrap();
    let record_batch = reader.next().unwrap().unwrap();
    let array_ref = record_batch.column(0);
    assert!(array_ref.data_type() == &arrow::datatypes::DataType::Int8);
    assert!(
        array_ref
            .as_any()
            .downcast_ref::<Int8Array>()
            .unwrap()
            .value(0)
            == 1
    );
    driver.statement_release(stmt_handle).unwrap();
    driver.connection_release(conn_handle).unwrap();
}
