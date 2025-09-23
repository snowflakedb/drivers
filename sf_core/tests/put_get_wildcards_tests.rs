pub mod common;
use common::arrow_result_helper::ArrowResultHelper;
use common::test_utils::*;
use std::fs;

#[test]
fn test_put_ls_wildcard_question_mark() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_WILDCARD_QUESTION_MARK";
    let base_name = "test_put_wildcard_question_mark";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();

    for i in 1..=5 {
        let filename = format!("{base_name}_{i}.csv");
        create_test_file(temp_dir.path(), &filename, "1,2,3\n");
    }

    // Create files that should NOT match the '?' wildcard pattern
    let non_matching_file1 = format!("{base_name}_10.csv"); // Two digits instead of one
    let non_matching_file2 = format!("{base_name}_abc.csv"); // Multiple characters
    create_test_file(temp_dir.path(), &non_matching_file1, "1,2,3\n");
    create_test_file(temp_dir.path(), &non_matching_file2, "1,2,3\n");

    let files_wildcard = format!(
        "{}/{base_name}_?.csv",
        temp_dir.path().to_str().unwrap().replace("\\", "/"),
    );

    // Setup stage and upload files
    client.create_temporary_stage(stage_name);

    let put_sql = format!("PUT 'file://{files_wildcard}' @{stage_name}");
    client.execute_query(&put_sql);

    let ls_result = client.execute_query(&format!("LS @{stage_name}"));
    let result_vector = ArrowResultHelper::from_result(ls_result)
        .transform_into_array::<String>()
        .unwrap();

    for i in 1..=5 {
        let expected_filename = format!("{}/{}_{i}.csv.gz", stage_name.to_lowercase(), base_name);
        assert!(
            result_vector
                .iter()
                .any(|row| row.contains(&expected_filename)),
            "File {expected_filename} should be listed in stage"
        );
    }

    // Assert that non-matching files are NOT present
    let non_matching_file1_gz = format!("{}/{}_10.csv.gz", stage_name.to_lowercase(), base_name);
    let non_matching_file2_gz = format!("{}/{}_abc.csv.gz", stage_name.to_lowercase(), base_name);

    assert!(
        !result_vector
            .iter()
            .any(|row| row.contains(&non_matching_file1_gz)),
        "File {non_matching_file1_gz} should NOT be listed in stage (doesn't match '?' pattern)"
    );
    assert!(
        !result_vector
            .iter()
            .any(|row| row.contains(&non_matching_file2_gz)),
        "File {non_matching_file2_gz} should NOT be listed in stage (doesn't match '?' pattern)"
    );
}

#[test]
fn test_put_ls_wildcard_star() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_WILDCARD_STAR";
    let base_name = "test_put_wildcard_star";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();

    for i in 1..=5 {
        let filename = format!("{base_name}_{i}{i}{i}.csv");
        create_test_file(temp_dir.path(), &filename, "1,2,3\n");
    }

    // Create files that should NOT match the '*' wildcard pattern
    let non_matching_file1 = format!("{base_name}.csv"); // No underscore and suffix
    let non_matching_file2 = format!("{base_name}_test.txt"); // Different extension
    create_test_file(temp_dir.path(), &non_matching_file1, "1,2,3\n");
    create_test_file(temp_dir.path(), &non_matching_file2, "1,2,3\n");

    let files_wildcard = format!(
        "{}/{base_name}_*.csv",
        temp_dir.path().to_str().unwrap().replace("\\", "/"),
    );

    // Setup stage and upload files
    client.create_temporary_stage(stage_name);

    let put_sql = format!("PUT 'file://{files_wildcard}' @{stage_name}");
    client.execute_query(&put_sql);

    let ls_result = client.execute_query(&format!("LS @{stage_name}"));
    let result_vector = ArrowResultHelper::from_result(ls_result)
        .transform_into_array::<String>()
        .unwrap();

    for i in 1..=5 {
        let expected_filename =
            format!("{}/{base_name}_{i}{i}{i}.csv.gz", stage_name.to_lowercase(),);
        assert!(
            result_vector
                .iter()
                .any(|row| row.contains(&expected_filename)),
            "File {expected_filename} should be listed in stage"
        );
    }

    // Assert that non-matching files are NOT present
    let non_matching_file1_gz = format!("{}/{}.csv.gz", stage_name.to_lowercase(), base_name);
    let non_matching_file2_gz = format!("{}/{}_test.txt.gz", stage_name.to_lowercase(), base_name);

    assert!(
        !result_vector
            .iter()
            .any(|row| row.contains(&non_matching_file1_gz)),
        "File {non_matching_file1_gz} should NOT be listed in stage (doesn't match '*' pattern)"
    );
    assert!(
        !result_vector
            .iter()
            .any(|row| row.contains(&non_matching_file2_gz)),
        "File {non_matching_file2_gz} should NOT be listed in stage (doesn't match '*' pattern)"
    );
}

// This test's purpose is to check if download of multiple files is working correctly.
// Regular expression handling is the job of Snowflake's backend.
// Escaping in the regexp does not seem to work correctly, it should be taken care of in the future.

#[test]
fn test_put_get_regexp() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_PUT_GET_REGEXP";
    let base_name = "data";

    // Set up test environment
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Setup stage
    client.create_temporary_stage(stage_name);

    // Create and upload test files that match the regexp pattern
    for i in 1..=5 {
        let filename = format!("{base_name}_{i}.csv");
        let file_path = create_test_file(temp_dir.path(), &filename, "1,2,3\n");
        let put_sql = format!(
            "PUT 'file://{}' @{stage_name}",
            file_path.to_str().unwrap().replace("\\", "/"),
        );
        client.execute_query(&put_sql);
    }

    // Create and upload files that should NOT match the regexp pattern
    let non_matching_file1 = format!("{base_name}_10.csv"); // Two digits instead of one
    let non_matching_file2 = format!("{base_name}_abc.csv"); // Multiple characters
    create_test_file(temp_dir.path(), &non_matching_file1, "1,2,3\n");
    create_test_file(temp_dir.path(), &non_matching_file2, "1,2,3\n");
    client.execute_query(&format!(
        "PUT 'file://{}/{}' @{stage_name}",
        temp_dir.path().to_str().unwrap().replace("\\", "/"),
        non_matching_file1
    ));
    client.execute_query(&format!(
        "PUT 'file://{}/{}' @{stage_name}",
        temp_dir.path().to_str().unwrap().replace("\\", "/"),
        non_matching_file2
    ));

    // Create directory for download
    let download_dir = temp_dir.path().join("download");
    fs::create_dir_all(&download_dir).unwrap();

    // The last two dots are escaped to match literal ".csv.gz"
    let get_pattern = format!(r".*/{base_name}_.\.csv\.gz");

    let get_sql = format!(
        "GET @{stage_name} file://{}/ PATTERN='{}'",
        download_dir.to_str().unwrap().replace("\\", "/"),
        get_pattern
    );
    client.execute_query(&get_sql);

    // Verify the downloaded files exist
    for i in 1..=5 {
        let expected_file_path = download_dir.join(format!("{base_name}_{i}.csv.gz"));
        assert!(
            expected_file_path.exists(),
            "Downloaded file should exist at {expected_file_path:?}",
        );
    }

    // Assert that non-matching files are NOT present
    let non_matching_file1_gz = download_dir.join(format!("{base_name}_10.csv.gz"));
    let non_matching_file2_gz = download_dir.join(format!("{base_name}_abc.csv.gz"));
    assert!(
        !non_matching_file1_gz.exists(),
        "Non-matching file should NOT exist at {non_matching_file1_gz:?}"
    );
    assert!(
        !non_matching_file2_gz.exists(),
        "Non-matching file should NOT exist at {non_matching_file2_gz:?}"
    );
}
