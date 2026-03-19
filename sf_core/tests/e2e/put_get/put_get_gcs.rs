//! GCS-specific PUT/GET e2e tests.
//!
//! These tests only run against GCP-backed Snowflake accounts. They are
//! automatically skipped when the test account uses a different cloud provider.

use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::file_utils::{create_test_file, decompress_gzipped_file, shared_test_data_dir};
use crate::common::put_get_common::{
    GetResult, PutResult, assert_file_exists, get_file_from_stage, upload_to_stage,
    upload_to_stage_with_options,
};
use crate::common::snowflake_test_client::SnowflakeTestClient;
use std::fs;

/// Detects whether the test account is on GCP by checking the host URL.
fn is_gcp_account(client: &SnowflakeTestClient) -> bool {
    // Primary: check host from test parameters
    if let Some(ref host) = client.parameters.host
        && host.contains(".gcp.")
    {
        return true;
    }
    if let Some(ref url) = client.parameters.server_url
        && url.contains(".gcp.")
    {
        return true;
    }

    // Fallback: query current_account_url()
    if let Ok(result) = client.execute_query_no_unwrap("SELECT CURRENT_ACCOUNT_URL()") {
        let mut helper = ArrowResultHelper::from_result(result);
        if let Ok(rows) = helper.transform_into_array::<String>()
            && !rows.is_empty()
            && !rows[0].is_empty()
        {
            return rows[0][0].contains(".gcp.");
        }
    }
    false
}

/// Skips the test if not running on GCP.
macro_rules! require_gcp {
    ($client:expr) => {
        if !is_gcp_account(&$client) {
            eprintln!("Skipping GCS test: account is not on GCP");
            return;
        }
    };
}

// ---------------------------------------------------------------
// Basic GCS PUT/GET round-trip
// (mirrors ODBC test_simple_put_gcs_with_token / test_simple_get_gcs_with_token)
// ---------------------------------------------------------------

#[test]
fn gcs_should_put_and_get_file() {
    // Given GCP-backed Snowflake account
    let client = SnowflakeTestClient::connect_with_default_auth();
    require_gcp!(client);

    // When File is uploaded and downloaded via GCS stage
    let stage_name = "TEST_GCS_PUT_GET";
    let (filename, test_file_path) = test_file();
    upload_to_stage(&client, stage_name, test_file_path.to_str().unwrap());

    let (get_result, download_dir) = get_file_from_stage(&client, stage_name, &filename);

    // Then Round-trip content should match
    let gzipped_filename = format!("{filename}.gz");
    assert_file_exists(&download_dir, &gzipped_filename);

    let downloaded = decompress_gzipped_file(download_dir.path().join(&gzipped_filename))
        .expect("Failed to decompress");
    let original = fs::read_to_string(&test_file_path).unwrap();
    assert_eq!(downloaded, original, "Round-trip content should match");

    let mut helper = ArrowResultHelper::from_result(get_result);
    let get: GetResult = helper.fetch_one().expect("Failed to parse GET result");
    assert_eq!(get.status, "DOWNLOADED");
}

// ---------------------------------------------------------------
// PUT/GET with overwrite
// (mirrors JDBC testPutGetGcsDownscopedCredential)
// ---------------------------------------------------------------

#[test]
fn gcs_should_put_overwrite_and_get() {
    // Given GCP-backed Snowflake account
    let client = SnowflakeTestClient::connect_with_default_auth();
    require_gcp!(client);

    // When File is uploaded twice with overwrite
    let stage_name = "TEST_GCS_OVERWRITE";
    let (filename, test_file_path) = test_file();
    upload_to_stage(&client, stage_name, test_file_path.to_str().unwrap());

    let put_result = upload_to_stage_with_options(
        &client,
        stage_name,
        test_file_path.to_str().unwrap(),
        "OVERWRITE=TRUE",
    );

    // Then Second upload should succeed with UPLOADED status
    let mut helper = ArrowResultHelper::from_result(put_result);
    let put: PutResult = helper.fetch_one().expect("Failed to parse PUT result");
    assert_eq!(put.status, "UPLOADED", "Overwrite should re-upload");

    let (_get_result, download_dir) = get_file_from_stage(&client, stage_name, &filename);
    let gzipped_filename = format!("{filename}.gz");
    assert_file_exists(&download_dir, &gzipped_filename);
}

// ---------------------------------------------------------------
// PUT without overwrite should skip
// ---------------------------------------------------------------

#[test]
fn gcs_should_skip_existing_file_without_overwrite() {
    // Given GCP-backed Snowflake account
    let client = SnowflakeTestClient::connect_with_default_auth();
    require_gcp!(client);

    // When File is uploaded twice without overwrite
    let stage_name = "TEST_GCS_SKIP";
    let (_filename, test_file_path) = test_file();
    upload_to_stage(&client, stage_name, test_file_path.to_str().unwrap());

    let put_result = upload_to_stage(&client, stage_name, test_file_path.to_str().unwrap());

    // Then Second upload should return SKIPPED status
    let mut helper = ArrowResultHelper::from_result(put_result);
    let put: PutResult = helper.fetch_one().expect("Failed to parse PUT result");
    assert_eq!(put.status, "SKIPPED", "Should skip without overwrite");
}

// ---------------------------------------------------------------
// Multiple files PUT/GET (tests per-file handling)
// (mirrors JDBC testFileTransferMappingFromSourceFile,
//  ODBC test_simple_get_gcs_with_presignedurl)
// ---------------------------------------------------------------

#[test]
fn gcs_should_put_and_get_multiple_files() {
    // Given GCP-backed Snowflake account
    let client = SnowflakeTestClient::connect_with_default_auth();
    require_gcp!(client);

    // When Multiple files are uploaded and downloaded
    let stage_name = "TEST_GCS_MULTI";
    let temp_dir = tempfile::TempDir::new().unwrap();

    let _file1 = create_test_file(temp_dir.path(), "multi_1.csv", "a,b,c\n1,2,3\n");
    let _file2 = create_test_file(temp_dir.path(), "multi_2.csv", "x,y,z\n4,5,6\n");

    client.create_temporary_stage(stage_name);

    let pattern = temp_dir.path().join("multi_*.csv");
    let put_result =
        upload_to_stage_with_options(&client, stage_name, pattern.to_str().unwrap(), "");

    let mut helper = ArrowResultHelper::from_result(put_result);
    let puts: Vec<Vec<String>> = helper.transform_into_array().unwrap();

    // Then Each file should have correct content
    assert_eq!(puts.len(), 2, "Should upload 2 files");

    let (_result1, download_dir1) = get_file_from_stage(&client, stage_name, "multi_1.csv");
    assert_file_exists(&download_dir1, "multi_1.csv.gz");
    let content1 = decompress_gzipped_file(download_dir1.path().join("multi_1.csv.gz")).unwrap();
    assert_eq!(content1, "a,b,c\n1,2,3\n");

    let (_result2, download_dir2) = get_file_from_stage(&client, stage_name, "multi_2.csv");
    assert_file_exists(&download_dir2, "multi_2.csv.gz");
    let content2 = decompress_gzipped_file(download_dir2.path().join("multi_2.csv.gz")).unwrap();
    assert_eq!(content2, "x,y,z\n4,5,6\n");
}

// ---------------------------------------------------------------
// Verify GCS result metadata matches expected format
// (mirrors existing put_get_basic_operations rowset tests)
// ---------------------------------------------------------------

#[test]
fn gcs_should_return_correct_put_rowset() {
    // Given GCP-backed Snowflake account
    let client = SnowflakeTestClient::connect_with_default_auth();
    require_gcp!(client);

    // When File is uploaded to GCS stage
    let stage_name = "TEST_GCS_PUT_ROWSET";
    let (_filename, test_file_path) = test_file();
    let put_data = upload_to_stage(&client, stage_name, test_file_path.to_str().unwrap());

    // Then PUT rowset should have correct metadata
    let mut helper = ArrowResultHelper::from_result(put_data);
    let put: PutResult = helper.fetch_one().expect("Failed to parse PUT result");

    assert_eq!(put.source, "test_data.csv");
    assert_eq!(put.target, "test_data.csv.gz");
    assert_eq!(put.source_compression, "NONE");
    assert_eq!(put.target_compression, "GZIP");
    assert_eq!(put.status, "UPLOADED");
}

#[test]
fn gcs_should_return_correct_get_rowset() {
    // Given GCP-backed Snowflake account
    let client = SnowflakeTestClient::connect_with_default_auth();
    require_gcp!(client);

    // When File is downloaded from GCS stage
    let stage_name = "TEST_GCS_GET_ROWSET";
    let (filename, test_file_path) = test_file();
    upload_to_stage(&client, stage_name, test_file_path.to_str().unwrap());

    let (get_result, _download_dir) = get_file_from_stage(&client, stage_name, &filename);

    // Then GET rowset should have correct metadata
    let mut helper = ArrowResultHelper::from_result(get_result);
    let get: GetResult = helper.fetch_one().expect("Failed to parse GET result");

    assert_eq!(get.file, "test_data.csv.gz");
    assert_eq!(get.status, "DOWNLOADED");
    assert!(get.size > 0, "Downloaded file size should be positive");
}

// ---------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------

fn test_file() -> (String, std::path::PathBuf) {
    (
        "test_data.csv".to_string(),
        shared_test_data_dir().join("basic").join("test_data.csv"),
    )
}
