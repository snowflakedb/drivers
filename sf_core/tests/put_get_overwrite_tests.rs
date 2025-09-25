pub mod common;
use common::arrow_result_helper::ArrowResultHelper;
use common::put_get_common::*;
use std::path::PathBuf;

#[test]
fn test_put_overwrite_true() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_OVERWRITE_TRUE";

    let (filename, original_file_path) = original_test_file();

    // Setup stage and upload original file
    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name}",
        original_file_path.to_str().unwrap().replace("\\", "/")
    );

    let first_put_result = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(first_put_result);
    let first_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch first PUT result");

    // Verify first upload was successful
    assert_eq!(first_result.source, filename);
    assert_eq!(first_result.status, "UPLOADED");

    let (filename, updated_file_path) = updated_test_file();

    // Upload the same file again with OVERWRITE=TRUE
    let put_overwrite_sql = format!(
        "PUT 'file://{}' @{stage_name} OVERWRITE=TRUE",
        updated_file_path.to_str().unwrap().replace("\\", "/")
    );
    let overwrite_put_result = client.execute_query(&put_overwrite_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(overwrite_put_result);
    let overwrite_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch overwrite PUT result");

    // Verify the file was overwritten (uploaded again)
    assert_eq!(overwrite_result.source, filename);
    assert_eq!(overwrite_result.status, "UPLOADED");

    // Verify the content was actually updated by querying the file
    let select_sql = format!("select $1, $2, $3 from @{stage_name}");
    let result = client.execute_query(&select_sql);
    let data_vector = ArrowResultHelper::from_result(result)
        .transform_into_array()
        .unwrap();
    assert_updated_content(data_vector.as_ref());
}

#[test]
fn test_put_overwrite_false() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_OVERWRITE_FALSE";
    let (filename, original_file_path) = original_test_file();

    // Setup stage and upload original file
    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name}",
        original_file_path.to_str().unwrap().replace("\\", "/")
    );
    let first_put_result = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(first_put_result);
    let first_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch first PUT result");

    // Verify first upload was successful
    assert_eq!(first_result.source, filename);
    assert_eq!(first_result.status, "UPLOADED");

    let (filename, updated_file_path) = updated_test_file();

    // Try to upload the same file again with OVERWRITE=FALSE
    let put_no_overwrite_sql = format!(
        "PUT 'file://{}' @{stage_name} OVERWRITE=FALSE",
        updated_file_path.to_str().unwrap().replace("\\", "/")
    );
    let no_overwrite_put_result = client.execute_query(&put_no_overwrite_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(no_overwrite_put_result);
    let no_overwrite_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch no-overwrite PUT result");

    // Verify the file upload was skipped
    assert_eq!(no_overwrite_result.source, filename);
    assert_eq!(no_overwrite_result.status, "SKIPPED");

    // Verify the original content is still in the stage (not overwritten)
    let select_sql = format!("select $1, $2, $3 from @{stage_name}");
    let result = client.execute_query(&select_sql);
    let data_vector = ArrowResultHelper::from_result(result)
        .transform_into_array()
        .unwrap();

    assert_original_content(data_vector.as_ref());
}

fn original_test_file() -> (String, PathBuf) {
    (
        "test_data.csv".to_string(),
        shared_test_data_dir()
            .join("overwrite")
            .join("original/test_data.csv"),
    )
}

fn updated_test_file() -> (String, PathBuf) {
    (
        "test_data.csv".to_string(),
        shared_test_data_dir()
            .join("overwrite")
            .join("updated/test_data.csv"),
    )
}

fn assert_original_content(data: &Vec<Vec<String>>) {
    assert_eq!(
        data,
        &vec![vec![
            "original".to_string(),
            "test".to_string(),
            "data".to_string(),
        ]]
    );
}

fn assert_updated_content(data: &Vec<Vec<String>>) {
    assert_eq!(
        data,
        &vec![vec![
            "updated".to_string(),
            "test".to_string(),
            "data".to_string(),
        ]]
    );
}
