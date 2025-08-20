pub mod common;
use common::test_utils::*;
use std::fs;

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
