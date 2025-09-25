use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::put_get_common::PutResult;
use crate::common::put_get_common::upload_file_to_stage;
use crate::common::put_get_common::upload_file_to_stage_with_options;
use crate::common::test_utils::*;
use std::path::PathBuf;

#[test]
fn should_overwrite_file_when_overwrite_is_set_to_true() {
    // Given File is uploaded to stage
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_PUT_GET_OVERWRITE_TRUE";
    upload_original_file(&client, stage_name);

    // When Updated file is uploaded with OVERWRITE set to true
    let (filename, updated_file_path) = updated_test_file();
    let overwrite_put_result = upload_file_to_stage_with_options(
        &client,
        stage_name,
        &updated_file_path,
        "OVERWRITE=TRUE",
    );

    // Then UPLOADED status is returned
    assert_put_result_status(overwrite_put_result, &filename, "UPLOADED");

    // And File was overwritten
    assert_stage_content(&client, stage_name, "updated");
}

#[test]
fn should_not_overwrite_file_when_overwrite_is_set_to_false() {
    // Given File is uploaded to stage
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_PUT_GET_OVERWRITE_FALSE";
    upload_original_file(&client, stage_name);

    // When Updated file is uploaded with OVERWRITE set to false
    let (filename, updated_file_path) = updated_test_file();
    let no_overwrite_put_result = upload_file_to_stage_with_options(
        &client,
        stage_name,
        &updated_file_path,
        "OVERWRITE=FALSE",
    );

    // Then SKIPPED status is returned
    assert_put_result_status(no_overwrite_put_result, &filename, "SKIPPED");

    // And File was not overwritten
    assert_stage_content(&client, stage_name, "original");
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

fn assert_stage_content(
    client: &SnowflakeTestClient,
    stage_name: &str,
    expected_first_column: &str,
) {
    let select_sql = format!("select $1, $2, $3 from @{stage_name}");
    let result = client.execute_query(&select_sql);
    let data_vector = ArrowResultHelper::from_result(result)
        .transform_into_array::<String>()
        .unwrap();

    assert_eq!(
        data_vector,
        vec![vec![
            expected_first_column.to_string(),
            "test".to_string(),
            "data".to_string(),
        ]]
    );
}

fn upload_original_file(client: &SnowflakeTestClient, stage_name: &str) {
    let (filename, original_reference_file_path) = original_test_file();
    let original_put_results =
        upload_file_to_stage(client, stage_name, &original_reference_file_path);
    let mut arrow_helper = ArrowResultHelper::from_result(original_put_results);
    let first_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch original PUT result");
    assert_eq!(first_result.source, filename);
    assert_eq!(first_result.status, "UPLOADED");
}

fn assert_put_result_status(
    put_result: sf_core::protobuf_gen::database_driver_v1::ExecuteResult,
    expected_filename: &str,
    expected_status: &str,
) {
    let mut arrow_helper = ArrowResultHelper::from_result(put_result);
    let result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");
    assert_eq!(result.source, expected_filename);
    assert_eq!(result.status, expected_status);
}
