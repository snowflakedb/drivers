//! Multi-file GET on a GCS-backed Snowflake stage (presigned URL path).
//!
//! On GCP accounts (*.gcp.snowflakecomputing.com), Snowflake returns
//! `data.presignedUrls[i]` per source file instead of a bearer token.
//! Before gap 2.2, sf_core discarded this list and every file in a
//! multi-file GET failed with `MissingGcsCredentials` on the first file.

use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::file_utils::{create_test_file, path_to_sql_uri};
use crate::common::put_get_common::{GetResult, PutResult};
use crate::common::snowflake_test_client::SnowflakeTestClient;
use uuid::Uuid;

fn random_stage_name(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4().simple())
}

#[test]
fn should_get_multiple_files_from_stage_in_single_command() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = random_stage_name("TEST_GCS_MULTI_GET");
    client.execute_sql(&format!("CREATE TEMPORARY STAGE {stage_name}"));

    let upload_dir = tempfile::TempDir::new().unwrap();

    // Given Two files are uploaded to stage
    let file_alpha = create_test_file(upload_dir.path(), "alpha.txt", "contents of alpha\n");
    let file_beta = create_test_file(upload_dir.path(), "beta.txt", "contents of beta\n");

    let put_alpha = format!(
        "PUT 'file://{}' @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE",
        path_to_sql_uri(&file_alpha)
    );
    let result = client.execute_query(&put_alpha);
    let mut helper = ArrowResultHelper::from_result(result);
    let put: PutResult = helper.fetch_one().expect("PUT alpha failed");
    assert_eq!(put.status, "UPLOADED");

    let put_beta = format!(
        "PUT 'file://{}' @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE",
        path_to_sql_uri(&file_beta)
    );
    let result = client.execute_query(&put_beta);
    let mut helper = ArrowResultHelper::from_result(result);
    let put: PutResult = helper.fetch_one().expect("PUT beta failed");
    assert_eq!(put.status, "UPLOADED");

    // When All files are downloaded from stage using GET command
    let download_dir = tempfile::TempDir::new().unwrap();
    let get_sql = format!(
        "GET @{stage_name} 'file://{}/'",
        path_to_sql_uri(download_dir.path())
    );
    let get_result = client.execute_query(&get_sql);

    // Then All files should be downloaded
    let mut helper = ArrowResultHelper::from_result(get_result);
    let rows: Vec<GetResult> = helper.fetch_all().expect("Failed to fetch GET results");
    assert_eq!(
        rows.len(),
        2,
        "Expected 2 downloaded files, got {}",
        rows.len()
    );
    for row in &rows {
        assert_eq!(
            row.status, "DOWNLOADED",
            "File {} was not DOWNLOADED",
            row.file
        );
    }

    // And Each file should have correct content
    let downloaded_alpha = download_dir.path().join("alpha.txt");
    assert!(
        downloaded_alpha.exists(),
        "alpha.txt should exist in download dir"
    );
    let content_alpha = std::fs::read_to_string(&downloaded_alpha).unwrap();
    assert_eq!(content_alpha.trim(), "contents of alpha");

    let downloaded_beta = download_dir.path().join("beta.txt");
    assert!(
        downloaded_beta.exists(),
        "beta.txt should exist in download dir"
    );
    let content_beta = std::fs::read_to_string(&downloaded_beta).unwrap();
    assert_eq!(content_beta.trim(), "contents of beta");
}
