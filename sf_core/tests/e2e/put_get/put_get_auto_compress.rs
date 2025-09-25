use crate::common::put_get_common::assert_file_exists;
use crate::common::put_get_common::get_file_from_stage;
use crate::common::put_get_common::upload_file_to_stage_with_options;
use crate::common::test_utils::*;
use std::fs;
use std::path::PathBuf;

#[test]
fn should_compress_the_file_before_uploading_to_stage_when_auto_compress_set_to_true() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_PUT_GET_AUTO_COMPRESS_TRUE";
    let (uncompressed_filename, uncompressed_reference_file_path) = uncompressed_test_file();
    let (compressed_filename, compressed_reference_file_path) = compressed_test_file();

    // When File is uploaded to stage with AUTO_COMPRESS set to true
    upload_file_to_stage_with_options(
        &client,
        stage_name,
        &uncompressed_reference_file_path,
        "AUTO_COMPRESS=TRUE",
    );

    // Then Only compressed file should be downloaded
    let (_get_result, download_dir) =
        get_file_from_stage(&client, stage_name, &uncompressed_filename);
    assert_file_exists(&download_dir, &compressed_filename);
    assert_file_not_exist(&download_dir, &uncompressed_filename);

    // And Have correct content
    assert_downloaded_content_matches_reference(
        &download_dir,
        &compressed_filename,
        &compressed_reference_file_path,
    );
}

#[test]
fn should_not_compress_the_file_before_uploading_to_stage_when_auto_compress_set_to_false() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_PUT_GET_AUTO_COMPRESS_FALSE";
    let (uncompressed_filename, uncompressed_reference_file_path) = uncompressed_test_file();
    let (compressed_filename, _compressed_reference_file_path) = compressed_test_file();

    // When File is uploaded to stage with AUTO_COMPRESS set to false
    upload_file_to_stage_with_options(
        &client,
        stage_name,
        &uncompressed_reference_file_path,
        "AUTO_COMPRESS=FALSE",
    );

    // Then Only uncompressed file should be downloaded
    let (_get_result, download_dir) =
        get_file_from_stage(&client, stage_name, &uncompressed_filename);
    assert_file_exists(&download_dir, &uncompressed_filename);
    assert_file_not_exist(&download_dir, &compressed_filename);

    // And Have correct content
    assert_downloaded_content_matches_reference(
        &download_dir,
        &uncompressed_filename,
        &uncompressed_reference_file_path,
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

fn assert_file_not_exist(download_dir: &tempfile::TempDir, filename: &str) {
    let file_path = download_dir.path().join(filename);
    assert!(
        !file_path.exists(),
        "File should not exist at {file_path:?}",
    );
}

fn assert_downloaded_content_matches_reference(
    download_dir: &tempfile::TempDir,
    downloaded_filename: &str,
    reference_file_path: &std::path::Path,
) {
    let expected_file_path = download_dir.path().join(downloaded_filename);
    let downloaded_content = fs::read(&expected_file_path).unwrap();
    let reference_content = fs::read(reference_file_path).unwrap();
    assert_eq!(
        downloaded_content, reference_content,
        "Downloaded content should match reference content"
    );
}
