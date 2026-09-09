//! SNOW-4073008: a stage file downloaded from Azure Blob Storage must come
//! back byte-for-byte identical to what was staged, compressed or not.

use crate::common::file_utils::create_test_file;
use crate::common::put_get_common::{get_file_from_stage, upload_to_stage_with_options};
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::require_running_on_azure;
use std::fs;
use uuid::Uuid;

const CONTENT: &str = "id,name\n1,café\n2,naïve\n3,日本語\n";

fn random_stage_name(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4().simple())
}

#[test]
fn should_download_uncompressed_stage_file_byte_for_byte_from_azure() {
    require_running_on_azure!();

    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = random_stage_name("TEST_AZURE_GET_UNCOMPRESSED");

    // Given an uncompressed file is uploaded to an Azure stage
    let upload_dir = tempfile::TempDir::new().unwrap();
    let filename = "unicode_data.csv";
    let local_path = create_test_file(upload_dir.path(), filename, CONTENT);

    upload_to_stage_with_options(
        &client,
        &stage_name,
        local_path.to_str().unwrap(),
        "AUTO_COMPRESS=FALSE OVERWRITE=TRUE",
    );

    // When the file is downloaded from the Azure stage
    let (_get_result, download_dir) = get_file_from_stage(&client, &stage_name, filename);
    let downloaded_path = download_dir.path().join(filename);
    let downloaded_bytes = fs::read(&downloaded_path).expect("downloaded file should exist");

    // Then the downloaded bytes must be byte-for-byte identical to what was staged
    let downloaded_text = String::from_utf8(downloaded_bytes)
        .expect("downloaded bytes must be valid UTF-8, not corrupted/transport-decoded bytes");
    assert_eq!(
        downloaded_text, CONTENT,
        "downloaded content must exactly match what was staged"
    );
}

#[test]
fn should_download_compressed_stage_file_and_decompress_to_original_content_from_azure() {
    require_running_on_azure!();

    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = random_stage_name("TEST_AZURE_GET_COMPRESSED");

    // Given a file is uploaded to an Azure stage with default (gzip) compression
    let upload_dir = tempfile::TempDir::new().unwrap();
    let filename = "unicode_data.csv";
    let local_path = create_test_file(upload_dir.path(), filename, CONTENT);

    upload_to_stage_with_options(
        &client,
        &stage_name,
        local_path.to_str().unwrap(),
        "OVERWRITE=TRUE",
    );

    // When the compressed file is downloaded from the Azure stage
    let (_get_result, download_dir) = get_file_from_stage(&client, &stage_name, filename);
    let downloaded_path = download_dir.path().join(format!("{filename}.gz"));
    let decompressed = crate::common::file_utils::decompress_gzipped_file(&downloaded_path)
        .expect("downloaded file must be valid gzip");

    // Then decompressing it must reproduce the original content exactly
    assert_eq!(
        decompressed, CONTENT,
        "decompressed content must exactly match what was staged"
    );
}
