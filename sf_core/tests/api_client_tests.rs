pub mod common;

use common::test_utils::*;
use sf_core::api_client::new_database_driver_v1_client;
use sf_core::thrift_gen::database_driver_v1::InfoCode;
use std::fs;

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
    let file_name = "test_put_select.csv";

    // Create test file with CSV data
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = create_test_file(temp_dir.path(), file_name, "1,2,3\n");

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
    let file_name = "test_put_ls.csv";

    // Setup test environment
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = create_test_file(temp_dir.path(), file_name, "1,2,3\n");

    // Set up stage and upload file
    client.create_temporary_stage(stage_name);

    let put_sql = format!(
        "PUT 'file://{}' @{stage_name}",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    client.execute_query(&put_sql);

    // Verify file was uploaded with LS command
    let expected_file_name = format!("{}/test_put_ls.csv.gz", stage_name.to_lowercase()); // File is compressed by default
    let ls_result = client.execute_query(&format!("LS @{stage_name}"));
    let result_vector = ArrowResultHelper::from_result(ls_result)
        .transform_into_array::<String>()
        .unwrap();
    assert_eq!(
        result_vector[0][0], expected_file_name,
        "File should be listed in stage"
    );
}

#[test]
fn test_get() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_GET";
    let file_name = "test_get.csv";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = create_test_file(temp_dir.path(), file_name, "1,2,3\n");

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
        "GET @{stage_name}/{file_name} file://{}/",
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
    let file_name = "test_put_get_compress_false.csv";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = create_test_file(temp_dir.path(), file_name, "1,2,3\n");

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
        "GET @{stage_name}/{file_name} file://{}/",
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
    let file_name = "test_put_get_compress_true.csv";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = create_test_file(temp_dir.path(), file_name, "1,2,3\n");

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
        "GET @{stage_name}/{file_name} file://{}/",
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
    let file_name_base = "test_put_wildcard_question_mark";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();

    for i in 1..=5 {
        let file_name = format!("{file_name_base}_{i}.csv");
        create_test_file(temp_dir.path(), &file_name, "1,2,3\n");
    }

    // Create files that should NOT match the '?' wildcard pattern
    let non_matching_file1 = format!("{file_name_base}_10.csv"); // Two digits instead of one
    let non_matching_file2 = format!("{file_name_base}_abc.csv"); // Multiple characters
    create_test_file(temp_dir.path(), &non_matching_file1, "1,2,3\n");
    create_test_file(temp_dir.path(), &non_matching_file2, "1,2,3\n");

    let files_wildcard = format!(
        "{}/{file_name_base}_?.csv",
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
        let expected_file_name = format!(
            "{}/{}_{i}.csv.gz",
            stage_name.to_lowercase(),
            file_name_base
        );
        assert!(
            result_vector
                .iter()
                .any(|row| row.contains(&expected_file_name)),
            "File {expected_file_name} should be listed in stage"
        );
    }

    // Assert that non-matching files are NOT present
    let non_matching_file1_gz =
        format!("{}/{}_10.csv.gz", stage_name.to_lowercase(), file_name_base);
    let non_matching_file2_gz = format!(
        "{}/{}_abc.csv.gz",
        stage_name.to_lowercase(),
        file_name_base
    );

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
    let file_name_base = "test_put_wildcard_star";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();

    for i in 1..=5 {
        let file_name = format!("{file_name_base}_{i}{i}{i}.csv");
        create_test_file(temp_dir.path(), &file_name, "1,2,3\n");
    }

    // Create files that should NOT match the '*' wildcard pattern
    let non_matching_file1 = format!("{file_name_base}.csv"); // No underscore and suffix
    let non_matching_file2 = format!("{file_name_base}_test.txt"); // Different extension
    create_test_file(temp_dir.path(), &non_matching_file1, "1,2,3\n");
    create_test_file(temp_dir.path(), &non_matching_file2, "1,2,3\n");

    let files_wildcard = format!(
        "{}/{file_name_base}_*.csv",
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
        let expected_file_name = format!(
            "{}/{file_name_base}_{i}{i}{i}.csv.gz",
            stage_name.to_lowercase(),
        );
        assert!(
            result_vector
                .iter()
                .any(|row| row.contains(&expected_file_name)),
            "File {expected_file_name} should be listed in stage"
        );
    }

    // Assert that non-matching files are NOT present
    let non_matching_file1_gz = format!("{}/{}.csv.gz", stage_name.to_lowercase(), file_name_base);
    let non_matching_file2_gz = format!(
        "{}/{}_test.txt.gz",
        stage_name.to_lowercase(),
        file_name_base
    );

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
    let file_name_base = "data";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Setup stage
    client.create_temporary_stage(stage_name);

    // Create and upload test files that match the regexp pattern
    for i in 1..=5 {
        let file_name = format!("{file_name_base}_{i}.csv");
        let file_path = create_test_file(temp_dir.path(), &file_name, "1,2,3\n");
        let put_sql = format!(
            "PUT 'file://{}' @{stage_name}",
            file_path.to_str().unwrap().replace("\\", "/"),
        );
        client.execute_query(&put_sql);
    }

    // Create and upload files that should NOT match the regexp pattern
    let non_matching_file1 = format!("{file_name_base}_10.csv"); // Two digits instead of one
    let non_matching_file2 = format!("{file_name_base}_abc.csv"); // Multiple characters
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
    let get_pattern = format!(r".*/{file_name_base}_.\.csv\.gz");

    let get_sql = format!(
        "GET @{stage_name} file://{}/ PATTERN='{}'",
        download_dir.to_str().unwrap().replace("\\", "/"),
        get_pattern
    );
    client.execute_query(&get_sql);

    // Verify the downloaded files exist
    for i in 1..=5 {
        let expected_file_path = download_dir.join(format!("{file_name_base}_{i}.csv.gz"));
        assert!(
            expected_file_path.exists(),
            "Downloaded file should exist at {expected_file_path:?}",
        );
    }

    // Assert that non-matching files are NOT present
    let non_matching_file1_gz = download_dir.join(format!("{file_name_base}_10.csv.gz"));
    let non_matching_file2_gz = download_dir.join(format!("{file_name_base}_abc.csv.gz"));
    assert!(
        !non_matching_file1_gz.exists(),
        "Non-matching file should NOT exist at {non_matching_file1_gz:?}"
    );
    assert!(
        !non_matching_file2_gz.exists(),
        "Non-matching file should NOT exist at {non_matching_file2_gz:?}"
    );
}
