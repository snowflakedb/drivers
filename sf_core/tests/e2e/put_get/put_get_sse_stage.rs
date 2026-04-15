use crate::common::file_utils::{create_test_file, path_to_sql_uri};
use crate::common::put_get_common::{GetResult, PutResult};
use crate::common::snowflake_test_client::SnowflakeTestClient;
use uuid::Uuid;

fn random_stage_name(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4().simple())
}

fn create_sse_stage(client: &SnowflakeTestClient, stage_name: &str) {
    client.execute_query(&format!(
        "CREATE TEMPORARY STAGE IF NOT EXISTS {stage_name} ENCRYPTION = (TYPE = 'SNOWFLAKE_SSE')"
    ));
}

#[test]
fn should_put_and_get_file_on_sse_stage() {
    // Given Stage with server-side encryption (SNOWFLAKE_SSE)
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = random_stage_name("TEST_SSE_PUT_GET");
    create_sse_stage(&client, &stage_name);

    let upload_dir = tempfile::TempDir::new().unwrap();
    let test_file = create_test_file(upload_dir.path(), "sse_test.txt", "hello sse\n");

    // When File is uploaded using PUT command
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE",
        path_to_sql_uri(&test_file)
    );
    let result = client.execute_query(&put_sql);

    // Then File should be uploaded successfully
    let mut helper = crate::common::arrow_result_helper::ArrowResultHelper::from_result(result);
    let put: PutResult = helper.fetch_one().expect("Failed to fetch PUT result");
    assert_eq!(put.status, "UPLOADED");

    // When File is downloaded using GET command
    let download_dir = tempfile::TempDir::new().unwrap();
    let get_sql = format!(
        "GET @{stage_name}/sse_test.txt 'file://{}/'",
        path_to_sql_uri(download_dir.path())
    );
    let get_result = client.execute_query(&get_sql);

    // Then File should be downloaded
    let mut helper = crate::common::arrow_result_helper::ArrowResultHelper::from_result(get_result);
    let get: GetResult = helper.fetch_one().expect("Failed to fetch GET result");
    assert_eq!(get.status, "DOWNLOADED");

    // And Have correct content
    let downloaded = download_dir.path().join("sse_test.txt");
    assert!(downloaded.exists(), "Downloaded file should exist");
    let content = std::fs::read_to_string(&downloaded).unwrap();
    assert_eq!(content.trim(), "hello sse");
}

#[test]
fn should_put_and_get_file_on_sse_stage_with_directory_enabled() {
    // Given Stage with server-side encryption and DIRECTORY enabled
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = random_stage_name("TEST_SSE_DIR");
    client.execute_query(&format!(
        "CREATE TEMPORARY STAGE IF NOT EXISTS {stage_name} \
         ENCRYPTION = (TYPE = 'SNOWFLAKE_SSE') \
         DIRECTORY = (ENABLE = TRUE)"
    ));

    let upload_dir = tempfile::TempDir::new().unwrap();
    let test_file = create_test_file(upload_dir.path(), "test.txt", "Initial contents\n");

    // When File is uploaded using PUT command
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE",
        path_to_sql_uri(&test_file)
    );
    let result = client.execute_query(&put_sql);

    // Then File should be uploaded successfully
    let mut helper = crate::common::arrow_result_helper::ArrowResultHelper::from_result(result);
    let put: PutResult = helper.fetch_one().expect("Failed to fetch PUT result");
    assert_eq!(put.status, "UPLOADED");

    // When File is downloaded using GET command
    let download_dir = tempfile::TempDir::new().unwrap();
    let get_sql = format!(
        "GET @{stage_name}/test.txt 'file://{}/'",
        path_to_sql_uri(download_dir.path())
    );
    let get_result = client.execute_query(&get_sql);

    // Then File should be downloaded
    let mut helper = crate::common::arrow_result_helper::ArrowResultHelper::from_result(get_result);
    let get: GetResult = helper.fetch_one().expect("Failed to fetch GET result");
    assert_eq!(get.status, "DOWNLOADED");

    // And Have correct content
    let downloaded = download_dir.path().join("test.txt");
    assert!(downloaded.exists(), "Downloaded file should exist");
    let content = std::fs::read_to_string(&downloaded).unwrap();
    assert_eq!(content.trim(), "Initial contents");
}
