use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::file_utils::shared_test_data_dir;
use crate::common::put_get_common::PutResult;
use crate::common::put_get_common::upload_to_stage;
use crate::common::put_get_common::upload_to_stage_with_options;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use std::path::PathBuf;

#[test]
fn should_overwrite_file_when_overwrite_is_set_to_true() {
    // Given File is uploaded to stage
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_PUT_GET_OVERWRITE_TRUE";
    upload_original_file(&client, stage_name);

    // When Updated file is uploaded with OVERWRITE set to true
    let (filename, updated_file_path) = updated_test_file();
    let overwrite_put_result = upload_to_stage_with_options(
        &client,
        stage_name,
        updated_file_path.to_str().unwrap(),
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
    let no_overwrite_put_result = upload_to_stage_with_options(
        &client,
        stage_name,
        updated_file_path.to_str().unwrap(),
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

/// GCS-only: when the same file is uploaded twice with `OVERWRITE=TRUE`,
/// the digest-skip branch fires on the second attempt because the remote
/// `x-goog-meta-sfc-digest` already matches the local SHA-256.
///
/// The digest is the SHA-256 of the (compressed) plaintext, so the skip
/// fires for both SSE and CSE stages. Not applicable to S3/Azure where no
/// digest-skip exists — the test self-skips (via `current_cloud()`) when the
/// connected account is not GCS-backed, since the `gcs_e2e` feature only gates
/// compilation, not the runtime cloud.
#[cfg(feature = "gcs_e2e")]
#[test]
fn should_skip_upload_when_content_matches_under_overwrite_true_on_gcs_encryption_type() {
    use crate::common::put_get_common::build_put_command;
    use crate::common::snowflake_test_client::CloudProvider;

    let client = SnowflakeTestClient::connect_with_default_auth();

    // The digest-skip optimisation only exists on the GCS upload path
    // (`upload_to_gcs_or_skip`); S3/Azure have existence-skip only. Internal
    // stages live in the deployment cloud, so on a non-GCS account (e.g. CI's
    // default AWS account, decoded by `scripts/decode_secrets.sh` with no
    // argument) the second PUT would re-upload. The `gcs_e2e` feature only
    // gates *compilation*, not the runtime cloud — self-skip here rather than
    // assert wrong-cloud behaviour.
    let cloud = client.current_cloud();
    if cloud != CloudProvider::Gcp {
        eprintln!("Skipping GCS digest-skip test: connected account is {cloud:?}, not GCP");
        return;
    }

    for (stage_name, encryption_type) in [
        ("TEST_PUT_GCS_DIGEST_SKIP_SSE", "SNOWFLAKE_SSE"),
        ("TEST_PUT_GCS_DIGEST_SKIP_CSE", "SNOWFLAKE_FULL"),
    ] {
        // Given File is uploaded to a GCS <encryption_type> stage
        client.execute_sql(&format!(
            "CREATE TEMPORARY STAGE IF NOT EXISTS {stage_name} \
             ENCRYPTION = (TYPE = '{encryption_type}')"
        ));
        let (filename, file_path) = original_test_file();
        let file_path_str = file_path.to_str().unwrap();
        let first_result = client.execute_query(&build_put_command(stage_name, file_path_str, ""));
        assert_put_result_status(first_result, &filename, "UPLOADED");

        // When Same file is uploaded again with OVERWRITE set to true
        let second_result = client.execute_query(&build_put_command(
            stage_name,
            file_path_str,
            "OVERWRITE=TRUE",
        ));

        // Then SKIPPED status is returned
        assert_put_result_status(second_result, &filename, "SKIPPED");
    }
}

fn upload_original_file(client: &SnowflakeTestClient, stage_name: &str) {
    let (filename, original_reference_file_path) = original_test_file();
    let original_put_results = upload_to_stage(
        client,
        stage_name,
        original_reference_file_path.to_str().unwrap(),
    );
    let mut arrow_helper = ArrowResultHelper::from_result(original_put_results);
    let first_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch original PUT result");
    assert_eq!(first_result.source, filename);
    assert_eq!(first_result.status, "UPLOADED");
}

fn assert_put_result_status(
    put_result: sf_core::protobuf::generated::database_driver_v1::ResultSetGetStreamResponse,
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
