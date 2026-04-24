use crate::common::snowflake_test_client::SnowflakeTestClient;
use uuid::Uuid;

#[test]
fn should_return_error_when_putting_nonexistent_local_file() {
    // Given A stage is created
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_ERR";
    client.create_temporary_stage(stage_name);

    // When PUT is executed with a path to a nonexistent local file
    let nonexistent = format!("/tmp/nonexistent_file_{}.csv", Uuid::new_v4());
    let sql = format!("PUT 'file://{nonexistent}' @{stage_name}");

    // Then An error is raised indicating the local file does not exist
    let result = client.execute_query_no_unwrap(&sql);
    assert!(result.is_err(), "Expected error for PUT nonexistent file");
    let err = result.unwrap_err();
    assert!(
        err.contains("File does not exist"),
        "Expected 'File does not exist' in error, got: {err}"
    );
}

#[test]
fn should_return_error_when_getting_nonexistent_file_from_stage() {
    // Given An empty stage is created
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_GET_ERR";
    client.create_temporary_stage(stage_name);

    // When GET is executed for a file that does not exist in stage
    let nonexistent = format!("nonexistent_file_{}.csv", Uuid::new_v4());
    let sql = format!("GET @{stage_name}/{nonexistent} 'file:///tmp/'");

    // Then An error is raised indicating the remote file does not exist
    let result = client.execute_query_no_unwrap(&sql);
    assert!(result.is_err(), "Expected error for GET nonexistent file");
    let err = result.unwrap_err();
    assert!(
        err.contains("the file does not exist"),
        "Expected 'the file does not exist' in error, got: {err}"
    );
}
