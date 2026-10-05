use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::file_utils::{create_test_file, decompress_gzipped_file};
use crate::common::put_get_common::{assert_file_exists, get_file_from_stage, upload_to_stage};
use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_select_on_a_fips_build() {
    // Given the linked AWS-LC module reports FIPS mode
    assert_fips_mode();

    // And a default connection is logged in with CRL off
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When SELECT 1 is executed
    let rows = select_one(&client);

    // Then the query returns 1
    assert_eq!(rows, vec![vec![1]]);
}

#[test]
fn should_select_with_crl_enabled_on_a_fips_build() {
    // Given the linked AWS-LC module reports FIPS mode
    assert_fips_mode();

    // And CRL is enabled before the connection is established
    let client = SnowflakeTestClient::with_default_jwt_auth_params();
    client.set_connection_option("crl_check_mode", "ENABLED");
    client.connect().expect("CRL-enabled login failed");

    // When SELECT 1 is executed
    let rows = select_one(&client);

    // Then the query returns 1
    assert_eq!(rows, vec![vec![1]]);
}

#[test]
fn should_put_and_get_on_a_fips_build() {
    // Given the linked AWS-LC module reports FIPS mode
    assert_fips_mode();

    // And a default connection is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let temp_dir = tempfile::TempDir::new().expect("create temp dir");
    let filename = "fips_smoke.txt";
    let content = "fips-smoke\n";
    let file_path = create_test_file(temp_dir.path(), filename, content);
    let stage_name = "FIPS_SMOKE";

    // When a small file is uploaded to a unique stage and downloaded
    upload_to_stage(
        &client,
        stage_name,
        file_path.to_str().expect("test file path is utf-8"),
    );
    let (_get_result, download_dir) = get_file_from_stage(&client, stage_name, filename);

    // Then the downloaded file matches the upload
    let gzipped_filename = format!("{filename}.gz");
    assert_file_exists(&download_dir, &gzipped_filename);
    let downloaded = decompress_gzipped_file(download_dir.path().join(&gzipped_filename))
        .expect("decompress downloaded file");
    assert_eq!(downloaded, content);
}

fn assert_fips_mode() {
    assert!(
        aws_lc_rs::try_fips_mode().is_ok(),
        "aws-lc is linked but not in FIPS mode"
    );
}

fn select_one(client: &SnowflakeTestClient) -> Vec<Vec<i64>> {
    let result = client.execute_query("SELECT 1");
    ArrowResultHelper::from_result(result)
        .transform_into_array::<i64>()
        .expect("SELECT 1 should return rows")
}
