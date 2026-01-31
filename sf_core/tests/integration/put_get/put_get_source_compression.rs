use crate::common::file_utils::repo_root;
use crate::common::mocks;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use wiremock::MockServer;

// TODO: Consider transitioning to AsyncSnowflakeTestClient once it supports
// full statement execution (including PUT file processing). Currently using
// sync client with manual runtime for wiremock setup because the compression
// detection error is generated client-side during PUT file processing.
#[test]
fn should_return_error_for_unsupported_compression_type() {
    // Given Snowflake client is logged in
    let rt = tokio::runtime::Runtime::new().unwrap();
    let server = rt.block_on(async {
        let server = MockServer::start().await;
        let repo_root_str = repo_root().to_str().unwrap().to_string();

        mocks::auth::mount_jwt_login_success(&server).await;
        mocks::put_get::mount_unsupported_compression(&server, &repo_root_str).await;
        server
    });

    let client = SnowflakeTestClient::connect_integration_test(Some(&server.uri()));
    let stage_name = "TEST_STAGE_UNSUPPORTED";

    // And File compressed with unsupported format
    let filename = "test_data.csv.xz";
    let workspace_root = repo_root();
    let test_file_path = workspace_root
        .join("tests")
        .join("test_data")
        .join("generated_test_data")
        .join("compression")
        .join(filename);

    // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );

    // Then Unsupported compression error is thrown
    let result = client.execute_query_no_unwrap(&put_sql);
    assert!(
        matches!(
            &result,
            Err(e) if e.contains("Unsupported compression type")
        ),
        "Expected unsupported compression error, got: {result:?}"
    );
}
