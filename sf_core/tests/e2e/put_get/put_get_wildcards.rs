use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::put_get_common::{assert_file_exists, upload_to_stage};
use crate::common::test_utils::*;
use sf_core::protobuf_gen::database_driver_v1::ExecuteResult;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn should_upload_files_that_match_wildcard_question_mark_pattern() {
    // Given Files matching wildcard pattern
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_WILDCARD_QUESTION_MARK";
    let base_file_name = "test_put_wildcard_question_mark";
    let temp_dir = TempDir::new().unwrap();
    let _matching_files = create_matching_files(&temp_dir, base_file_name);

    // And Files not matching wildcard pattern
    let non_matching_files = vec![
        format!("{base_file_name}_10.csv"),  // Two digits instead of one
        format!("{base_file_name}_abc.csv"), // Multiple characters
    ];
    create_test_files(&temp_dir, &non_matching_files);

    // When Files are uploaded using command with question mark wildcard
    let files_wildcard = format!(
        "{}/{base_file_name}_?.csv",
        temp_dir.path().to_str().unwrap().replace("\\", "/"),
    );
    upload_to_stage(&client, stage_name, &files_wildcard);

    // Then Files matching wildcard pattern are uploaded
    let result_vector = get_stage_listing_results(&client, stage_name);
    assert_matching_files_in_stage(&result_vector, stage_name, base_file_name);

    // And Files not matching wildcard pattern are not uploaded
    assert_non_matching_files_not_in_stage(&result_vector, stage_name, &non_matching_files);
}

#[test]
fn should_upload_files_that_match_wildcard_star_pattern() {
    // Given Files matching wildcard pattern
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_WILDCARD_STAR";
    let base_file_name = "test_put_wildcard_star";
    let temp_dir = TempDir::new().unwrap();
    let _matching_files = create_matching_files(&temp_dir, base_file_name);

    // And Files not matching wildcard pattern
    let non_matching_files = vec![
        format!("{base_file_name}.csv"),      // No underscore and suffix
        format!("{base_file_name}_test.txt"), // Different extension
    ];
    create_test_files(&temp_dir, &non_matching_files);

    // When Files are uploaded using command with star wildcard
    let files_wildcard = format!(
        "{}/{base_file_name}_*.csv",
        temp_dir.path().to_str().unwrap().replace("\\", "/"),
    );
    upload_to_stage(&client, stage_name, &files_wildcard);

    // Then Files matching wildcard pattern are uploaded
    let result_vector = get_stage_listing_results(&client, stage_name);
    assert_matching_files_in_stage(&result_vector, stage_name, base_file_name);

    // And Files not matching wildcard pattern are not uploaded
    assert_non_matching_files_not_in_stage(&result_vector, stage_name, &non_matching_files);
}

// This test's purpose is to check if download of multiple files is working correctly.
// Regular expression handling is the job of Snowflake's backend.
// Escaping in the regexp does not seem to work correctly, it should be taken care of in the future.
#[test]
fn should_download_files_that_are_matching_wildcard_pattern() {
    // Given Files matching wildcard pattern are uploaded
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_GET_REGEXP";
    let base_file_name = "test_get";
    let temp_dir = TempDir::new().unwrap();
    let matching_files = create_matching_files(&temp_dir, base_file_name);
    for filename in &matching_files {
        let file_path = temp_dir.path().join(filename);
        upload_to_stage(&client, stage_name, file_path.to_str().unwrap());
    }

    // And Files not matching wildcard pattern are uploaded
    let non_matching_files = vec![
        format!("{base_file_name}_10.csv"),  // Two digits instead of one
        format!("{base_file_name}_abc.csv"), // Multiple characters
    ];
    create_test_files(&temp_dir, &non_matching_files);
    for non_matching_file in &non_matching_files {
        let file_path = temp_dir.path().join(non_matching_file);
        upload_to_stage(&client, stage_name, file_path.to_str().unwrap());
    }

    // When Files are downloaded using command with wildcard
    let download_temp_dir = TempDir::new().unwrap();
    let get_pattern = format!(r".*/{base_file_name}_.\.csv\.gz"); // The last two dots are escaped to match literal ".csv.gz"
    let _get_result =
        get_from_stage_with_pattern(&client, stage_name, &get_pattern, download_temp_dir.path());

    // Then Files matching wildcard pattern are downloaded
    assert_matching_files_downloaded(&download_temp_dir, base_file_name);

    // And Files not matching wildcard pattern are not downloaded
    assert_non_matching_files_not_downloaded(&download_temp_dir, &non_matching_files);
}

fn create_matching_files(temp_dir: &TempDir, base_file_name: &str) -> Vec<String> {
    let matching_files: Vec<String> = (1..=5)
        .map(|i| format!("{base_file_name}_{i}.csv"))
        .collect();
    create_test_files(temp_dir, &matching_files);
    matching_files
}

fn assert_matching_files_in_stage(
    result_vector: &[Vec<String>],
    stage_name: &str,
    base_file_name: &str,
) {
    for i in 1..=5 {
        let expected_filename = format!(
            "{}/{}_{i}.csv.gz",
            stage_name.to_lowercase(),
            base_file_name
        );
        assert_file_in_stage(result_vector, &expected_filename);
    }
}

fn assert_file_in_stage(result_vector: &[Vec<String>], filename: &str) {
    let filename_string = filename.to_string();
    assert!(
        result_vector
            .iter()
            .any(|row| row.contains(&filename_string)),
        "File {filename} should be listed in stage"
    );
}

fn assert_non_matching_files_not_in_stage(
    result_vector: &[Vec<String>],
    stage_name: &str,
    non_matching_files: &[String],
) {
    for non_matching_file in non_matching_files {
        let non_matching_file_gz =
            format!("{}/{}.gz", stage_name.to_lowercase(), non_matching_file);
        assert_file_not_in_stage(result_vector, &non_matching_file_gz);
    }
}

fn assert_matching_files_downloaded(download_dir: &TempDir, base_file_name: &str) {
    for i in 1..=5 {
        let filename = format!("{base_file_name}_{i}.csv.gz");
        assert_file_exists(download_dir, &filename);
    }
}

fn assert_non_matching_files_not_downloaded(download_dir: &TempDir, non_matching_files: &[String]) {
    for non_matching_file in non_matching_files {
        let non_matching_file_gz = format!("{}.gz", non_matching_file);
        let file_path = download_dir.path().join(&non_matching_file_gz);
        assert!(
            !file_path.exists(),
            "Non-matching file should NOT exist at {file_path:?}"
        );
    }
}

fn create_test_files(temp_dir: &TempDir, filenames: &[String]) {
    for filename in filenames {
        create_test_file(temp_dir.path(), filename, "1,2,3\n");
    }
}

fn assert_file_not_in_stage(result_vector: &[Vec<String>], filename: &str) {
    let filename_string = filename.to_string();
    assert!(
        !result_vector
            .iter()
            .any(|row| row.contains(&filename_string)),
        "File {filename} should NOT be listed in stage"
    );
}

fn get_stage_listing_results(client: &SnowflakeTestClient, stage_name: &str) -> Vec<Vec<String>> {
    let ls_result = client.execute_query(&format!("LS @{stage_name}"));
    ArrowResultHelper::from_result(ls_result)
        .transform_into_array::<String>()
        .unwrap()
}

fn get_from_stage_with_pattern(
    client: &SnowflakeTestClient,
    stage_name: &str,
    pattern: &str,
    download_dir: &Path,
) -> ExecuteResult {
    let get_sql = format!(
        "GET @{stage_name} file://{}/ PATTERN='{}'",
        download_dir.to_str().unwrap().replace("\\", "/"),
        pattern
    );
    client.execute_query(&get_sql)
}
