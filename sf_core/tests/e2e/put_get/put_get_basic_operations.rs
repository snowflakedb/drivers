use super::super::common::test_utils::*;
use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::put_get_common::{
    GetResult, PutResult, assert_file_exists, get_file_from_stage, upload_file_to_stage,
};
use arrow::datatypes::Field;
use std::fs;

#[test]
fn should_select_data_from_file_uploaded_to_stage() {
    // Given File is uploaded to stage
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_SELECT";
    let (_filename, test_file_path) = test_file();
    upload_file_to_stage(&client, stage_name, &test_file_path);

    // When File data is queried using Select command
    let select_sql = format!("select $1, $2, $3 from @{stage_name}");
    let result = client.execute_query(&select_sql);

    // Then File data should be correctly returned
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper.assert_equals_single_row(vec!["1".to_string(), "2".to_string(), "3".to_string()]);
}

#[test]
fn should_list_file_uploaded_to_stage() {
    // Given File is uploaded to stage
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_LS";
    let (filename, test_file_path) = test_file();
    upload_file_to_stage(&client, stage_name, &test_file_path);

    // When Stage content is listed using LS command
    let ls_result = client.execute_query(&format!("LS @{stage_name}"));

    // Then File should be listed with correct filename
    let result_vector = ArrowResultHelper::from_result(ls_result)
        .transform_into_array::<String>()
        .unwrap();
    let expected_filename = format!("{}/{filename}.gz", stage_name.to_lowercase());
    assert_eq!(
        result_vector[0][0], expected_filename,
        "File should be listed in stage"
    );
}

#[test]
fn should_get_file_uploaded_to_stage() {
    // Given File is uploaded to stage
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_GET";
    let (filename, test_file_path) = test_file();
    upload_file_to_stage(&client, stage_name, &test_file_path);

    // When File is downloaded using GET command
    let (_get_result, download_dir) = get_file_from_stage(&client, stage_name, &filename);

    // Then File should be downloaded
    let gzipped_filename = format!("{filename}.gz");
    assert_file_exists(&download_dir, &gzipped_filename);

    // And Have correct content
    let expected_file_path = download_dir.path().join(&gzipped_filename);
    let decompressed_content =
        decompress_gzipped_file(&expected_file_path).expect("Failed to decompress downloaded file");
    let original_content = fs::read_to_string(&test_file_path).unwrap();
    assert_eq!(
        decompressed_content, original_content,
        "Downloaded and decompressed content should match original"
    );
}

#[test]
fn should_return_correct_rowset_for_put() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When File is uploaded to stage
    let stage_name = "TEST_STAGE_PUT_ROWSET";
    let (_filename, test_file_path) = test_file();
    let put_data = upload_file_to_stage(&client, stage_name, &test_file_path);

    // Then Rowset for PUT command should be correct
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);
    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    assert_eq!(put_result.source, "test_data.csv");
    assert_eq!(put_result.target, "test_data.csv.gz");
    assert_eq!(put_result.source_size, 6);
    assert_eq!(put_result.target_size, 32);
    assert_eq!(put_result.source_compression, "NONE");
    assert_eq!(put_result.target_compression, "GZIP");
    assert_eq!(put_result.status, "UPLOADED");
    assert_eq!(put_result.message, "");
}

#[test]
fn should_return_correct_rowset_for_get() {
    // Given File is uploaded to stage
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_GET_ROWSET";
    let (filename, test_file_path) = test_file();
    upload_file_to_stage(&client, stage_name, &test_file_path);

    // When File is downloaded using GET command
    let (get_result, _download_dir) = get_file_from_stage(&client, stage_name, &filename);

    // Then Rowset for GET command should be correct
    let mut arrow_helper = ArrowResultHelper::from_result(get_result);
    let get_result: GetResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch GET result");

    assert_eq!(get_result.file, "test_data.csv.gz");
    assert_eq!(get_result.size, 26);
    assert_eq!(get_result.status, "DOWNLOADED");
    assert_eq!(get_result.message, "");
}

#[test]
fn should_return_correct_column_metadata_for_put() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When File is uploaded to stage
    let stage_name = "TEST_STAGE_PUT_COLUMN_METADATA";
    let (_filename, test_file_path) = test_file();
    let put_data = upload_file_to_stage(&client, stage_name, &test_file_path);

    // Then Column metadata for PUT command should be correct
    let arrow_helper = ArrowResultHelper::from_result(put_data);
    let schema = arrow_helper.schema();
    let fields = schema.fields();
    assert_eq!(fields.len(), 8);
    check_text_field(&fields[0], "source");
    check_text_field(&fields[1], "target");
    check_fixed_field(&fields[2], "source_size");
    check_fixed_field(&fields[3], "target_size");
    check_text_field(&fields[4], "source_compression");
    check_text_field(&fields[5], "target_compression");
    check_text_field(&fields[6], "status");
    check_text_field(&fields[7], "message");
}

#[test]
fn should_return_correct_column_metadata_for_get() {
    // Given File is uploaded to stage
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_GET_COLUMN_METADATA";
    let (filename, test_file_path) = test_file();
    upload_file_to_stage(&client, stage_name, &test_file_path);

    // When File is downloaded using GET command
    let (get_result, _download_dir) = get_file_from_stage(&client, stage_name, &filename);

    // Then Column metadata for GET command should be correct
    let arrow_helper = ArrowResultHelper::from_result(get_result);
    let schema = arrow_helper.schema();
    let fields = schema.fields();
    assert_eq!(fields.len(), 4);
    check_text_field(&fields[0], "file");
    check_fixed_field(&fields[1], "size");
    check_text_field(&fields[2], "status");
    check_text_field(&fields[3], "message");
}

fn test_file() -> (String, std::path::PathBuf) {
    (
        "test_data.csv".to_string(),
        shared_test_data_dir().join("basic").join("test_data.csv"),
    )
}

fn check_text_field(field: &Field, name: &str) {
    let rowset_text_length = "10000";

    assert_eq!(field.name(), name);
    let m0 = field.metadata();
    assert_eq!(m0.get("logicalType"), Some(&"TEXT".to_string()));
    assert_eq!(m0.get("physicalType"), Some(&"LOB".to_string()));
    assert_eq!(m0.get("charLength"), Some(&rowset_text_length.to_string()));
    assert_eq!(m0.get("byteLength"), Some(&rowset_text_length.to_string()));
}

fn check_fixed_field(field: &Field, name: &str) {
    assert_eq!(field.name(), name);
    let m0 = field.metadata();
    assert_eq!(m0.get("logicalType"), Some(&"FIXED".to_string()));
    assert_eq!(m0.get("scale"), Some(&"0".to_string()));
    assert_eq!(m0.get("precision"), Some(&"64".to_string()));
    assert_eq!(m0.get("physicalType"), Some(&"SB8".to_string()));
}
