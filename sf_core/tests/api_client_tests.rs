pub mod common;

use common::arrow_deserialize::ArrowDeserialize;
use common::arrow_result_helper::ArrowResultHelper;
use common::test_utils::*;
use flate2::{
    Compression,
    write::{DeflateEncoder, GzEncoder},
};
use sf_core::api_client::new_database_driver_v1_client;
use sf_core::thrift_gen::database_driver_v1::InfoCode;
use std::fs;
use std::io::Write;

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

#[test]
fn test_put_select() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_SELECT";
    let filename = "test_put_select.csv";

    // Create test file with CSV data
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = create_test_file(temp_dir.path(), filename, "1,2,3\n");

    // Setup stage and upload file
    client.create_temporary_stage(stage_name);

    let put_sql = format!(
        "PUT 'file://{}' @{stage_name}",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    client.execute_query(&put_sql);

    // Query the uploaded file data
    let select_sql = format!("select $1, $2, $3 from @{stage_name}");
    let result = client.execute_query(&select_sql);

    // Verify the data matches what we uploaded
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper.assert_equals_single_row(vec!["1".to_string(), "2".to_string(), "3".to_string()]);
}

#[test]
fn test_put_ls() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_LS";
    let filename = "test_put_ls.csv";

    // Setup test environment
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = create_test_file(temp_dir.path(), filename, "1,2,3\n");

    // Set up stage and upload file
    client.create_temporary_stage(stage_name);

    let put_sql = format!(
        "PUT 'file://{}' @{stage_name}",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    client.execute_query(&put_sql);

    // Verify file was uploaded with LS command
    let expected_filename = format!("{}/test_put_ls.csv.gz", stage_name.to_lowercase()); // File is compressed by default
    let ls_result = client.execute_query(&format!("LS @{stage_name}"));
    let result_vector = ArrowResultHelper::from_result(ls_result)
        .transform_into_array::<String>()
        .unwrap();
    assert_eq!(
        result_vector[0][0], expected_filename,
        "File should be listed in stage"
    );
}

#[test]
fn test_get() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_GET";
    let filename = "test_get.csv";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = create_test_file(temp_dir.path(), filename, "1,2,3\n");

    // Setup stage and upload file
    client.create_temporary_stage(stage_name);

    let put_sql = format!(
        "PUT 'file://{}' @{stage_name}",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    client.execute_query(&put_sql);

    // Create directory for download
    let download_dir = temp_dir.path().join("download");
    fs::create_dir_all(&download_dir).unwrap();

    // Download file using GET
    let get_sql = format!(
        "GET @{stage_name}/{filename} file://{}/",
        download_dir.to_str().unwrap().replace("\\", "/")
    );
    client.execute_query(&get_sql);

    // Verify the downloaded file exists and content matches
    let expected_file_path = download_dir.join("test_get.csv.gz");
    assert!(
        expected_file_path.exists(),
        "Downloaded gzipped file should exist at {expected_file_path:?}",
    );

    // Decompress and verify content
    let decompressed_content =
        decompress_gzipped_file(&expected_file_path).expect("Failed to decompress downloaded file");
    let original_content = fs::read_to_string(&test_file_path).unwrap();
    assert_eq!(
        decompressed_content, original_content,
        "Downloaded and decompressed content should match original"
    );
}

#[test]
fn test_put_get_with_auto_compress_false() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_GET_COMPRESS_FALSE";
    let filename = "test_put_get_compress_false.csv";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = create_test_file(temp_dir.path(), filename, "1,2,3\n");

    // Setup stage and upload file
    client.create_temporary_stage(stage_name);

    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} AUTO_COMPRESS=FALSE",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    client.execute_query(&put_sql);

    // Create directory for download
    let download_dir = temp_dir.path().join("download");
    fs::create_dir_all(&download_dir).unwrap();

    // Download file using GET
    let get_sql = format!(
        "GET @{stage_name}/{filename} file://{}/",
        download_dir.to_str().unwrap().replace("\\", "/")
    );
    client.execute_query(&get_sql);

    // Verify the downloaded file exists and content matches
    let expected_file_path = download_dir.join("test_put_get_compress_false.csv");
    assert!(
        expected_file_path.exists(),
        "Downloaded file should exist at {expected_file_path:?}",
    );
    let not_expected_file_path = download_dir.join("test_put_get_compress_false.csv.gz");
    assert!(
        !not_expected_file_path.exists(),
        "Compressed file should not exist at {not_expected_file_path:?}",
    );

    // Decompress and verify content
    let downloaded_content = fs::read_to_string(&expected_file_path).unwrap();
    let original_content = fs::read_to_string(&test_file_path).unwrap();
    assert_eq!(
        downloaded_content, original_content,
        "Downloaded content should match original"
    );
}

#[test]
fn test_put_get_with_auto_compress_true() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_GET_COMPRESS_TRUE";
    let filename = "test_put_get_compress_true.csv";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = create_test_file(temp_dir.path(), filename, "1,2,3\n");

    // Setup stage and upload file
    client.create_temporary_stage(stage_name);

    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} AUTO_COMPRESS=TRUE",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    client.execute_query(&put_sql);

    // Create directory for download
    let download_dir = temp_dir.path().join("download");
    fs::create_dir_all(&download_dir).unwrap();

    let get_sql = format!(
        "GET @{stage_name}/{filename} file://{}/",
        download_dir.to_str().unwrap().replace("\\", "/")
    );
    client.execute_query(&get_sql);

    // Verify the downloaded file exists and content matches
    let expected_file_path = download_dir.join("test_put_get_compress_true.csv.gz");
    assert!(
        expected_file_path.exists(),
        "Downloaded gzipped file should exist at {expected_file_path:?}",
    );
    let not_expected_file_path = download_dir.join("test_put_get_compress_true.csv");
    assert!(
        !not_expected_file_path.exists(),
        "Uncompressed file should not exist at {not_expected_file_path:?}",
    );

    // Decompress and verify content
    let decompressed_content =
        decompress_gzipped_file(&expected_file_path).expect("Failed to decompress downloaded file");
    let original_content = fs::read_to_string(&test_file_path).unwrap();
    assert_eq!(
        decompressed_content, original_content,
        "Downloaded and decompressed content should match original"
    );
}

#[test]
fn test_put_ls_wildcard_question_mark() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_WILDCARD_QUESTION_MARK";
    let base_name = "test_put_wildcard_question_mark";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();

    for i in 1..=5 {
        let filename = format!("{base_name}_{i}.csv");
        create_test_file(temp_dir.path(), &filename, "1,2,3\n");
    }

    // Create files that should NOT match the '?' wildcard pattern
    let non_matching_file1 = format!("{base_name}_10.csv"); // Two digits instead of one
    let non_matching_file2 = format!("{base_name}_abc.csv"); // Multiple characters
    create_test_file(temp_dir.path(), &non_matching_file1, "1,2,3\n");
    create_test_file(temp_dir.path(), &non_matching_file2, "1,2,3\n");

    let files_wildcard = format!(
        "{}/{base_name}_?.csv",
        temp_dir.path().to_str().unwrap().replace("\\", "/"),
    );

    // Setup stage and upload files
    client.create_temporary_stage(stage_name);

    let put_sql = format!("PUT 'file://{files_wildcard}' @{stage_name}");
    client.execute_query(&put_sql);

    let ls_result = client.execute_query(&format!("LS @{stage_name}"));
    let result_vector = ArrowResultHelper::from_result(ls_result)
        .transform_into_array::<String>()
        .unwrap();

    for i in 1..=5 {
        let expected_filename = format!("{}/{}_{i}.csv.gz", stage_name.to_lowercase(), base_name);
        assert!(
            result_vector
                .iter()
                .any(|row| row.contains(&expected_filename)),
            "File {expected_filename} should be listed in stage"
        );
    }

    // Assert that non-matching files are NOT present
    let non_matching_file1_gz = format!("{}/{}_10.csv.gz", stage_name.to_lowercase(), base_name);
    let non_matching_file2_gz = format!("{}/{}_abc.csv.gz", stage_name.to_lowercase(), base_name);

    assert!(
        !result_vector
            .iter()
            .any(|row| row.contains(&non_matching_file1_gz)),
        "File {non_matching_file1_gz} should NOT be listed in stage (doesn't match '?' pattern)"
    );
    assert!(
        !result_vector
            .iter()
            .any(|row| row.contains(&non_matching_file2_gz)),
        "File {non_matching_file2_gz} should NOT be listed in stage (doesn't match '?' pattern)"
    );
}

#[test]
fn test_put_ls_wildcard_star() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_WILDCARD_STAR";
    let base_name = "test_put_wildcard_star";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();

    for i in 1..=5 {
        let filename = format!("{base_name}_{i}{i}{i}.csv");
        create_test_file(temp_dir.path(), &filename, "1,2,3\n");
    }

    // Create files that should NOT match the '*' wildcard pattern
    let non_matching_file1 = format!("{base_name}.csv"); // No underscore and suffix
    let non_matching_file2 = format!("{base_name}_test.txt"); // Different extension
    create_test_file(temp_dir.path(), &non_matching_file1, "1,2,3\n");
    create_test_file(temp_dir.path(), &non_matching_file2, "1,2,3\n");

    let files_wildcard = format!(
        "{}/{base_name}_*.csv",
        temp_dir.path().to_str().unwrap().replace("\\", "/"),
    );

    // Setup stage and upload files
    client.create_temporary_stage(stage_name);

    let put_sql = format!("PUT 'file://{files_wildcard}' @{stage_name}");
    client.execute_query(&put_sql);

    let ls_result = client.execute_query(&format!("LS @{stage_name}"));
    let result_vector = ArrowResultHelper::from_result(ls_result)
        .transform_into_array::<String>()
        .unwrap();

    for i in 1..=5 {
        let expected_filename =
            format!("{}/{base_name}_{i}{i}{i}.csv.gz", stage_name.to_lowercase(),);
        assert!(
            result_vector
                .iter()
                .any(|row| row.contains(&expected_filename)),
            "File {expected_filename} should be listed in stage"
        );
    }

    // Assert that non-matching files are NOT present
    let non_matching_file1_gz = format!("{}/{}.csv.gz", stage_name.to_lowercase(), base_name);
    let non_matching_file2_gz = format!("{}/{}_test.txt.gz", stage_name.to_lowercase(), base_name);

    assert!(
        !result_vector
            .iter()
            .any(|row| row.contains(&non_matching_file1_gz)),
        "File {non_matching_file1_gz} should NOT be listed in stage (doesn't match '*' pattern)"
    );
    assert!(
        !result_vector
            .iter()
            .any(|row| row.contains(&non_matching_file2_gz)),
        "File {non_matching_file2_gz} should NOT be listed in stage (doesn't match '*' pattern)"
    );
}

// This test's purpose is to check if download of multiple files is working correctly.
// Regular expression handling is the job of Snowflake's backend.
// Escaping in the regexp does not seem to work correctly, it should be taken care of in the future.

#[test]
fn test_put_get_regexp() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_GET_REGEXP";
    let base_name = "data";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Setup stage
    client.create_temporary_stage(stage_name);

    // Create and upload test files that match the regexp pattern
    for i in 1..=5 {
        let filename = format!("{base_name}_{i}.csv");
        let file_path = create_test_file(temp_dir.path(), &filename, "1,2,3\n");
        let put_sql = format!(
            "PUT 'file://{}' @{stage_name}",
            file_path.to_str().unwrap().replace("\\", "/"),
        );
        client.execute_query(&put_sql);
    }

    // Create and upload files that should NOT match the regexp pattern
    let non_matching_file1 = format!("{base_name}_10.csv"); // Two digits instead of one
    let non_matching_file2 = format!("{base_name}_abc.csv"); // Multiple characters
    create_test_file(temp_dir.path(), &non_matching_file1, "1,2,3\n");
    create_test_file(temp_dir.path(), &non_matching_file2, "1,2,3\n");
    client.execute_query(&format!(
        "PUT 'file://{}/{}' @{stage_name}",
        temp_dir.path().to_str().unwrap().replace("\\", "/"),
        non_matching_file1
    ));
    client.execute_query(&format!(
        "PUT 'file://{}/{}' @{stage_name}",
        temp_dir.path().to_str().unwrap().replace("\\", "/"),
        non_matching_file2
    ));

    // Create directory for download
    let download_dir = temp_dir.path().join("download");
    fs::create_dir_all(&download_dir).unwrap();

    // The last two dots are escaped to match literal ".csv.gz"
    let get_pattern = format!(r".*/{base_name}_.\.csv\.gz");

    let get_sql = format!(
        "GET @{stage_name} file://{}/ PATTERN='{}'",
        download_dir.to_str().unwrap().replace("\\", "/"),
        get_pattern
    );
    client.execute_query(&get_sql);

    // Verify the downloaded files exist
    for i in 1..=5 {
        let expected_file_path = download_dir.join(format!("{base_name}_{i}.csv.gz"));
        assert!(
            expected_file_path.exists(),
            "Downloaded file should exist at {expected_file_path:?}",
        );
    }

    // Assert that non-matching files are NOT present
    let non_matching_file1_gz = download_dir.join(format!("{base_name}_10.csv.gz"));
    let non_matching_file2_gz = download_dir.join(format!("{base_name}_abc.csv.gz"));
    assert!(
        !non_matching_file1_gz.exists(),
        "Non-matching file should NOT exist at {non_matching_file1_gz:?}"
    );
    assert!(
        !non_matching_file2_gz.exists(),
        "Non-matching file should NOT exist at {non_matching_file2_gz:?}"
    );
}

// Structured types for Snowflake command results using our arrow_deserialize macro
#[derive(ArrowDeserialize, Debug, PartialEq)]
struct PutResult {
    source: String,
    target: String,
    source_size: i64,
    target_size: i64,
    source_compression: String,
    target_compression: String,
    status: String,
    message: String,
}

#[derive(ArrowDeserialize, Debug, PartialEq)]
struct GetResult {
    file: String,
    size: i64,
    status: String,
    message: String,
}

#[test]
fn test_put_get_rowset() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_ROWSET";
    let filename = "test_put_get_rowset.csv";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = create_test_file(temp_dir.path(), filename, "1,2,3\n");

    // Setup stage and upload file
    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name}",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    let put_data = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);

    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    assert_eq!(put_result.source, "test_put_get_rowset.csv");
    assert_eq!(put_result.target, "test_put_get_rowset.csv.gz");
    assert_eq!(put_result.source_size, 6);
    assert_eq!(put_result.target_size, 64);
    assert_eq!(put_result.source_compression, "NONE");
    assert_eq!(put_result.target_compression, "GZIP");
    assert_eq!(put_result.status, "UPLOADED");
    assert_eq!(put_result.message, "");

    let get_sql = format!(
        "GET @{stage_name}/{filename} file://{}/",
        temp_dir.path().to_str().unwrap().replace("\\", "/")
    );
    let get_data = client.execute_query(&get_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(get_data);

    let get_result: GetResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch GET result");

    assert_eq!(get_result.file, "test_put_get_rowset.csv.gz");
    assert_eq!(get_result.size, 52);
    assert_eq!(get_result.status, "DOWNLOADED");
    assert_eq!(get_result.message, "");
}

// Tests for SOURCE_COMPRESSION parameter

#[test]
fn test_put_source_compression_auto_detect_standard_types() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_STANDARD";
    let temp_dir = tempfile::TempDir::new().unwrap();
    let content = "1,2,3\n";

    // Setup stage once for all tests
    client.create_temporary_stage(stage_name);

    // Test cases for standard compression types that follow the same pattern
    let test_cases = [
        ("test_gzip.csv.gz", "GZIP"),
        ("test_bzip2.csv.bz2", "BZ2"),
        ("test_brotli.csv.br", "BROTLI"),
        ("test_zstd.csv.zst", "ZSTD"),
        ("test_deflate.csv.deflate", "DEFLATE"),
    ];

    for (filename, expected_compression) in test_cases {
        // Create compressed test file
        let test_file_path = temp_dir.path().join(filename);
        let file = fs::File::create(&test_file_path).unwrap();

        // Create appropriate encoder based on the compression type
        match expected_compression {
            "GZIP" => {
                let mut encoder = GzEncoder::new(file, Compression::default());
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            "BZ2" => {
                let mut encoder = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            "BROTLI" => {
                let mut encoder = brotli::CompressorWriter::new(file, 4096, 6, 22);
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.flush().unwrap();
            }
            "ZSTD" => {
                let mut encoder = zstd::stream::write::Encoder::new(file, 3).unwrap();
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            "DEFLATE" => {
                let mut encoder = DeflateEncoder::new(file, Compression::default());
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            _ => panic!("Unsupported compression type in test: {expected_compression}"),
        }

        // Upload file with AUTO_DETECT
        let put_sql = format!(
            "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT",
            test_file_path.to_str().unwrap().replace("\\", "/")
        );
        let put_data = client.execute_query(&put_sql);
        let mut arrow_helper = ArrowResultHelper::from_result(put_data);

        let put_result: PutResult = arrow_helper
            .fetch_one()
            .unwrap_or_else(|_| panic!("Failed to fetch PUT result for {filename}"));

        // Verify results
        assert_eq!(
            put_result.source, filename,
            "Source filename mismatch for {filename}"
        );
        assert_eq!(
            put_result.target, filename,
            "Target should equal source when file is already compressed for {filename}"
        );
        assert_eq!(
            put_result.source_compression, expected_compression,
            "Source compression type mismatch for {filename}"
        );
        assert_eq!(
            put_result.target_compression, expected_compression,
            "Target compression should match source for {filename}"
        );
        assert_eq!(
            put_result.status, "UPLOADED",
            "Upload status mismatch for {filename}"
        );
    }
}

#[test]
fn test_put_source_compression_auto_detect_raw_deflate() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_RAW_DEFLATE";
    let filename = "test_raw_deflate.csv.raw_deflate";

    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    let compressed_data = {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all("1,2,3\n".as_bytes()).unwrap();
        encoder.finish().unwrap()
    };
    fs::write(&test_file_path, compressed_data).unwrap();

    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    let put_data = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);

    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    assert_eq!(put_result.source, filename);
    assert_eq!(put_result.target, filename);
    assert_eq!(put_result.source_compression, "RAW_DEFLATE");
    assert_eq!(put_result.target_compression, "RAW_DEFLATE");
    assert_eq!(put_result.status, "UPLOADED");
}

#[test]
fn test_put_source_compression_auto_detect_none_no_auto_compress() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_NONE_NO_AUTO_COMPRESS";
    let filename = "test_none.csv";

    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    fs::write(&test_file_path, "1,2,3\n").unwrap();

    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=FALSE",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    let put_data = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);

    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    assert_eq!(put_result.source, filename);
    assert_eq!(put_result.target, filename);
    assert_eq!(put_result.source_compression, "NONE");
    assert_eq!(put_result.target_compression, "NONE");
    assert_eq!(put_result.status, "UPLOADED");
}

#[test]
fn test_put_source_compression_auto_detect_none_with_auto_compress() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_NONE_WITH_AUTO_COMPRESS";
    let filename = "test_none.csv";

    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    fs::write(&test_file_path, "1,2,3\n").unwrap();

    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=TRUE",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    let put_data = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);

    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    assert_eq!(put_result.source, filename);
    assert_eq!(put_result.target, format!("{filename}.gz"));
    assert_eq!(put_result.source_compression, "NONE");
    assert_eq!(put_result.target_compression, "GZIP");
    assert_eq!(put_result.status, "UPLOADED");
}

#[test]
fn test_put_source_compression_auto_detect_unsupported() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_UNSUPPORTED";
    let filename = "test_auto_detect.csv.lz";

    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    fs::write(&test_file_path, "1,2,3\n").unwrap();

    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );

    // This should fail because .lz extension indicates unsupported LZIP compression
    let result = client.execute_query_no_unwrap(&put_sql);

    assert!(
        matches!(
            &result,
            Err(e) if format!("{e:?}").contains("Unsupported compression type")
        ),
        "Expected unsupported compression error, got: {result:?}"
    );
}

#[test]
fn test_put_source_compression_auto_detect_content_based() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_CONTENT";
    let filename = "test_auto_detect_no_extension";

    // Set up test environment - create a gzipped file without extension
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    let file = fs::File::create(&test_file_path).unwrap();
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder.write_all("1,2,3\n".as_bytes()).unwrap();
    encoder.finish().unwrap();

    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    let put_data = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);

    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    assert_eq!(put_result.source, filename);
    assert_eq!(put_result.target, filename);
    // Should detect GZIP based on file content since there's no extension
    assert_eq!(put_result.source_compression, "GZIP");
    assert_eq!(put_result.target_compression, "GZIP");
    assert_eq!(put_result.status, "UPLOADED");
}

// Tests for explicit SOURCE_COMPRESSION specification

#[test]
fn test_put_source_compression_explicit_standard_types() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_EXPLICIT_COMPRESSION";
    let temp_dir = tempfile::TempDir::new().unwrap();
    let content = "1,2,3\n";

    // Setup stage once for all tests
    client.create_temporary_stage(stage_name);

    // Test cases for explicitly specified compression types
    let test_cases = [
        ("test_explicit_gzip.dat", "GZIP"),
        ("test_explicit_bzip2.dat", "BZ2"),
        ("test_explicit_brotli.dat", "BROTLI"),
        ("test_explicit_zstd.dat", "ZSTD"),
        ("test_explicit_deflate.dat", "DEFLATE"),
    ];

    for (filename, compression_type) in test_cases {
        // Create compressed test file (using .dat extension to avoid auto-detection)
        let test_file_path = temp_dir.path().join(filename);
        let file = fs::File::create(&test_file_path).unwrap();

        // Create appropriate encoder based on the compression type
        match compression_type {
            "GZIP" => {
                let mut encoder = GzEncoder::new(file, Compression::default());
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            "BZ2" => {
                let mut encoder = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            "BROTLI" => {
                let mut encoder = brotli::CompressorWriter::new(file, 4096, 6, 22);
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.flush().unwrap();
            }
            "ZSTD" => {
                let mut encoder = zstd::stream::write::Encoder::new(file, 3).unwrap();
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            "DEFLATE" => {
                let mut encoder = DeflateEncoder::new(file, Compression::default());
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            _ => panic!("Unsupported compression type in test: {compression_type}"),
        }

        // Upload file with explicit SOURCE_COMPRESSION
        let put_sql = format!(
            "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION={}",
            test_file_path.to_str().unwrap().replace("\\", "/"),
            compression_type
        );
        let put_data = client.execute_query(&put_sql);
        let mut arrow_helper = ArrowResultHelper::from_result(put_data);

        let put_result: PutResult = arrow_helper
            .fetch_one()
            .unwrap_or_else(|_| panic!("Failed to fetch PUT result for {filename}"));

        // Verify results - source and target should be the same (no double compression)
        assert_eq!(
            put_result.source, filename,
            "Source filename mismatch for {filename}"
        );
        assert_eq!(
            put_result.target, filename,
            "Target should equal source when file is already compressed for {filename}"
        );
        assert_eq!(
            put_result.source_compression, compression_type,
            "Source compression type mismatch for {filename}"
        );
        assert_eq!(
            put_result.target_compression, compression_type,
            "Target compression should match source for {filename}"
        );
        assert_eq!(
            put_result.status, "UPLOADED",
            "Upload status mismatch for {filename}"
        );
    }
}

#[test]
fn test_put_source_compression_explicit_raw_deflate() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_EXPLICIT_RAW_DEFLATE";
    let filename = "test_explicit_raw_deflate.dat";

    // Set up test environment - raw deflate needs special handling
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    let compressed_data = {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all("1,2,3\n".as_bytes()).unwrap();
        encoder.finish().unwrap()
    };
    fs::write(&test_file_path, compressed_data).unwrap();

    // Setup stage and upload file with explicit SOURCE_COMPRESSION
    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=RAW_DEFLATE",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    let put_data = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);

    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    // Verify no double compression occurred
    assert_eq!(put_result.source, filename);
    assert_eq!(put_result.target, filename);
    assert_eq!(put_result.source_compression, "RAW_DEFLATE");
    assert_eq!(put_result.target_compression, "RAW_DEFLATE");
    assert_eq!(put_result.status, "UPLOADED");
}

#[test]
fn test_put_source_compression_explicit_none_no_auto_compress() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_EXPLICIT_NONE_NO_AUTO_COMPRESS";
    let filename = "test_explicit_none.dat";

    // Set up test environment - uncompressed file
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    fs::write(&test_file_path, "1,2,3\n").unwrap();

    // Setup stage and upload file with explicit SOURCE_COMPRESSION=NONE
    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=NONE AUTO_COMPRESS=FALSE",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    let put_data = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);

    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    // Verify no compression occurred
    assert_eq!(put_result.source, filename);
    assert_eq!(put_result.target, filename);
    assert_eq!(put_result.source_compression, "NONE");
    assert_eq!(put_result.target_compression, "NONE");
    assert_eq!(put_result.status, "UPLOADED");
}

#[test]
fn test_put_source_compression_explicit_with_auto_compress() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_EXPLICIT_WITH_AUTO_COMPRESS";
    let filename = "test_explicit_with_auto.dat";

    // Set up test environment - uncompressed file
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    fs::write(&test_file_path, "1,2,3\n").unwrap();

    // Setup stage and upload file with SOURCE_COMPRESSION=NONE and AUTO_COMPRESS=TRUE
    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=NONE AUTO_COMPRESS=TRUE",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    let put_data = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);

    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    // Verify compression occurred due to AUTO_COMPRESS=TRUE
    assert_eq!(put_result.source, filename);
    assert_eq!(put_result.target, format!("{filename}.gz")); // Should be compressed
    assert_eq!(put_result.source_compression, "NONE");
    assert_eq!(put_result.target_compression, "GZIP"); // Should be compressed with GZIP
    assert_eq!(put_result.status, "UPLOADED");
}
