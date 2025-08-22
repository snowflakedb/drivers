pub mod common;
use common::arrow_result_helper::ArrowResultHelper;
use common::put_get_common::*;
use common::test_utils::*;

#[test]
fn test_put_overwrite_true() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_OVERWRITE_TRUE";
    let filename = "test_overwrite_true.csv";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();
    let original_content = "original,data,1\n";
    let updated_content = "updated,data,2\n";

    // Create original test file
    let test_file_path = create_test_file(temp_dir.path(), filename, original_content);

    // Setup stage and upload original file
    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name}",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    let first_put_result = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(first_put_result);
    let first_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch first PUT result");

    // Verify first upload was successful
    assert_eq!(first_result.status, "UPLOADED");
    assert_eq!(first_result.source, filename);

    // Create updated test file with different content
    let updated_file_path = create_test_file(temp_dir.path(), filename, updated_content);

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
    assert_eq!(overwrite_result.status, "UPLOADED");
    assert_eq!(overwrite_result.source, filename);

    // Verify the content was actually updated by querying the file
    let select_sql = format!("select $1, $2, $3 from @{stage_name}");
    let result = client.execute_query(&select_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper.assert_equals_single_row(vec![
        "updated".to_string(),
        "data".to_string(),
        "2".to_string(),
    ]);
}

#[test]
fn test_put_overwrite_false() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_OVERWRITE_FALSE";
    let filename = "test_overwrite_false.csv";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();
    let original_content = "original,data,1\n";
    let updated_content = "updated,data,2\n";

    // Create original test file
    let test_file_path = create_test_file(temp_dir.path(), filename, original_content);

    // Setup stage and upload original file
    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name}",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    let first_put_result = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(first_put_result);
    let first_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch first PUT result");

    // Verify first upload was successful
    assert_eq!(first_result.status, "UPLOADED");
    assert_eq!(first_result.source, filename);

    // Create updated test file with different content
    let updated_file_path = create_test_file(temp_dir.path(), filename, updated_content);

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
    assert_eq!(no_overwrite_result.status, "SKIPPED");
    assert_eq!(no_overwrite_result.source, filename);

    // Verify the original content is still in the stage (not overwritten)
    let select_sql = format!("select $1, $2, $3 from @{stage_name}");
    let result = client.execute_query(&select_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper.assert_equals_single_row(vec![
        "original".to_string(),
        "data".to_string(),
        "1".to_string(),
    ]);
}

#[test]
fn test_put_overwrite_false_multiple_files_mixed_status() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_OVERWRITE_MIXED";
    let base_filename = "test_overwrite_mixed";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Create multiple test files
    let file1_name = format!("{base_filename}_1.csv");
    let file2_name = format!("{base_filename}_2.csv");
    let file3_name = format!("{base_filename}_3.csv");

    let _file1_path = create_test_file(temp_dir.path(), &file1_name, "file1,content,1\n");
    let file2_path = create_test_file(temp_dir.path(), &file2_name, "file2,content,2\n");
    let _file3_path = create_test_file(temp_dir.path(), &file3_name, "file3,content,3\n");

    // Setup stage
    client.create_temporary_stage(stage_name);

    // Upload file2 first to make it exist in the stage
    let put_file2_sql = format!(
        "PUT 'file://{}' @{stage_name}",
        file2_path.to_str().unwrap().replace("\\", "/")
    );
    let file2_result = client.execute_query(&put_file2_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(file2_result);
    let initial_upload: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch initial file2 upload result");

    // Verify initial upload was successful
    assert_eq!(initial_upload.status, "UPLOADED");
    assert_eq!(initial_upload.source, file2_name);

    // Now upload all three files using wildcard pattern with OVERWRITE=FALSE
    // This should result in:
    // - file1: UPLOADED (new file)
    // - file2: SKIPPED (already exists)
    // - file3: UPLOADED (new file)
    let files_wildcard = format!(
        "{}/{base_filename}_*.csv",
        temp_dir.path().to_str().unwrap().replace("\\", "/"),
    );

    let put_pattern_sql = format!("PUT 'file://{files_wildcard}' @{stage_name} OVERWRITE=FALSE");
    let pattern_result = client.execute_query(&put_pattern_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(pattern_result);

    // The result should contain multiple rows - one for each file
    let results: Vec<PutResult> = arrow_helper
        .fetch_all()
        .expect("Failed to fetch pattern upload results");

    // Should have exactly 3 results
    assert_eq!(results.len(), 3, "Should have results for all 3 files");

    // Sort results by source filename for predictable testing
    let mut sorted_results = results;
    sorted_results.sort_by(|a, b| a.source.cmp(&b.source));

    // Verify each file result
    assert_eq!(sorted_results[0].source, file1_name);
    assert_eq!(
        sorted_results[0].status, "UPLOADED",
        "file1 should be uploaded (new file)"
    );

    assert_eq!(sorted_results[1].source, file2_name);
    assert_eq!(
        sorted_results[1].status, "SKIPPED",
        "file2 should be skipped (already exists)"
    );

    assert_eq!(sorted_results[2].source, file3_name);
    assert_eq!(
        sorted_results[2].status, "UPLOADED",
        "file3 should be uploaded (new file)"
    );

    // Verify that all files are actually in the stage by listing them
    let ls_result = client.execute_query(&format!("LS @{stage_name}"));
    let result_vector = ArrowResultHelper::from_result(ls_result)
        .transform_into_array::<String>()
        .unwrap();

    // All three files should be present in the stage
    let expected_files = [
        format!("{}/{file1_name}.gz", stage_name.to_lowercase()),
        format!("{}/{file2_name}.gz", stage_name.to_lowercase()),
        format!("{}/{file3_name}.gz", stage_name.to_lowercase()),
    ];

    for expected_file in &expected_files {
        assert!(
            result_vector.iter().any(|row| row.contains(expected_file)),
            "File {expected_file} should be present in stage"
        );
    }

    // Verify we can query data from all files to ensure they're all accessible
    let select_sql = format!("select $1, $2, $3 from @{stage_name} order by $1");
    let data_result = client.execute_query(&select_sql);
    let data_vector = ArrowResultHelper::from_result(data_result)
        .transform_into_array::<String>()
        .unwrap();

    // Should have 3 rows, one from each file
    assert_eq!(data_vector.len(), 3, "Should have data from all 3 files");

    // Verify the content is correct (sorted by first column)
    assert_eq!(
        data_vector[0],
        vec!["file1".to_string(), "content".to_string(), "1".to_string()]
    );
    assert_eq!(
        data_vector[1],
        vec!["file2".to_string(), "content".to_string(), "2".to_string()]
    );
    assert_eq!(
        data_vector[2],
        vec!["file3".to_string(), "content".to_string(), "3".to_string()]
    );
}
