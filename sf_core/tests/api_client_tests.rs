pub mod common;
use common::arrow_result_helper::ArrowResultHelper;
use common::test_utils::*;

use sf_core::api_client::new_database_driver_v1_client;
use sf_core::thrift_gen::database_driver_v1::InfoCode;

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
fn test_snowflake_select_1() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let result = client.execute_query("SELECT 1");

    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper.assert_equals_single_value(String::from("1"));
}

#[test]
fn test_create_temporary_stage() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE";
    let result = client.execute_query(&format!("create temporary stage {stage_name}"));

    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper
        .assert_equals_single_value(format!("Stage area {stage_name} successfully created."));
}
