pub mod common;
use common::arrow_result_helper::ArrowResultHelper;
use common::put_get_common::*;
use common::test_utils::*;
use std::fs;

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
