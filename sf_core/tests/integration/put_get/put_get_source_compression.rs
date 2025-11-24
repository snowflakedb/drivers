use crate::common::file_utils::repo_root;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::wiremock_client::WiremockClient;

#[test]
fn should_return_error_for_unsupported_compression_type() {
    // Given Snowflake client is logged in
    let wiremock = WiremockClient::start();

    wiremock.add_mapping("auth/login_success_jwt.json", None);
    wiremock.add_mapping("put_get/put_unsupported_compression_type.json", None);

    let client = SnowflakeTestClient::connect_integration_test(Some(&wiremock.http_url()));
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
            Err(e) if format!("{e:?}").contains("Unsupported compression type")
        ),
        "Expected unsupported compression error, got: {result:?}"
    );
}
