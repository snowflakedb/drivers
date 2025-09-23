pub mod common;
use common::put_get_common::*;
use common::test_utils::*;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_put_get_with_auto_compress_true() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_GET_COMPRESS_TRUE";

    // Use shared test data from repository
    let (uncompressed_filename, uncompressed_file_path) = uncompressed_test_file();
    let (compressed_filename, compressed_file_path) = compressed_test_file();

    // Setup stage and upload file
    client.create_temporary_stage(stage_name);

    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} AUTO_COMPRESS=TRUE",
        uncompressed_file_path.to_str().unwrap().replace("\\", "/")
    );
    client.execute_query(&put_sql);

    // Create directory for download
    let download_dir = tempfile::TempDir::new().unwrap();
    let download_dir_path = download_dir.path();

    let get_sql = format!(
        "GET @{stage_name}/{uncompressed_filename} file://{}/",
        download_dir_path.to_str().unwrap().replace("\\", "/")
    );
    client.execute_query(&get_sql);

    // Verify the downloaded file exists and content matches
    let expected_file_path = download_dir_path.join(compressed_filename);
    assert!(
        expected_file_path.exists(),
        "Downloaded gzipped file should exist at {expected_file_path:?}",
    );
    let not_expected_file_path = download_dir_path.join(uncompressed_filename);
    assert!(
        !not_expected_file_path.exists(),
        "Uncompressed file should not exist at {not_expected_file_path:?}",
    );

    // Verify content
    let downloaded_content = fs::read(&expected_file_path).unwrap();
    let reference_content = fs::read(&compressed_file_path).unwrap();
    assert_eq!(
        downloaded_content, reference_content,
        "Compressed content should match the reference"
    );
}

#[test]
fn test_put_get_with_auto_compress_false() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_GET_COMPRESS_FALSE";

    // Use shared test data from repository
    let (uncompressed_filename, uncompressed_file_path) = uncompressed_test_file();
    let (compressed_filename, _compressed_file_path) = compressed_test_file();

    // Setup stage and upload file
    client.create_temporary_stage(stage_name);

    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} AUTO_COMPRESS=FALSE",
        uncompressed_file_path.to_str().unwrap().replace("\\", "/")
    );
    client.execute_query(&put_sql);

    // Create directory for download
    let download_dir = tempfile::TempDir::new().unwrap();
    let download_dir_path = download_dir.path();

    // Download file using GET
    let get_sql = format!(
        "GET @{stage_name}/{uncompressed_filename} file://{}/",
        download_dir_path.to_str().unwrap().replace("\\", "/")
    );
    client.execute_query(&get_sql);

    // Verify the downloaded file exists and is uncompressed
    let expected_file_path = download_dir_path.join(uncompressed_filename);
    assert!(
        expected_file_path.exists(),
        "Downloaded file should exist at {expected_file_path:?}",
    );
    let not_expected_file_path = download_dir_path.join(compressed_filename);
    assert!(
        !not_expected_file_path.exists(),
        "Compressed file should not exist at {not_expected_file_path:?}",
    );

    // Verify content
    let downloaded_content = fs::read(&expected_file_path).unwrap();
    let original_content = fs::read(&uncompressed_file_path).unwrap();
    assert_eq!(
        downloaded_content, original_content,
        "Downloaded content should match original"
    );
}

fn uncompressed_test_file() -> (String, PathBuf) {
    (
        "test_data.csv".to_string(),
        shared_test_data_dir()
            .join("compression")
            .join("test_data.csv"),
    )
}

fn compressed_test_file() -> (String, PathBuf) {
    (
        "test_data.csv.gz".to_string(),
        shared_test_data_dir()
            .join("compression")
            .join("test_data.csv.gz"),
    )
}
