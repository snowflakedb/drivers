use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::file_utils::shared_test_data_dir;
use crate::common::put_get_common::PutResult;
use crate::common::put_get_common::upload_to_stage_with_options;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use sf_core::protobuf_gen::database_driver_v1::ExecuteResult;
use std::path::PathBuf;
use test_case::test_case;

#[test_case("GZIP", "test_data.csv.gz"; "gzip")]
#[test_case("BZIP2", "test_data.csv.bz2"; "bzip2")]
#[test_case("BROTLI", "test_data.csv.br"; "brotli")]
#[test_case("ZSTD", "test_data.csv.zst"; "zstd")]
#[test_case("DEFLATE", "test_data.csv.deflate"; "deflate")]
// RAW_DEFLATE is currently not auto-detected as it is not auto-detected in any existing drivers
// TODO: Revisit when we test more drivers, especially Go driver
fn should_auto_detect_standard_compression_types_when_source_compression_set_to_auto_detect(
    expected_compression: &str,
    filename: &str,
) {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = format!("TEST_STAGE_{}", expected_compression);

    // And File with standard type (GZIP, BZIP2, BROTLI, ZSTD, DEFLATE)
    let test_file_path = get_test_data_path(filename);

    // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT
    let put_data = upload_to_stage_with_options(
        &client,
        &stage_name,
        test_file_path.to_str().unwrap(),
        "SOURCE_COMPRESSION=AUTO_DETECT",
    );

    // Then Target compression has correct type and all PUT results are correct
    assert_put_results(
        put_data,
        filename,
        expected_compression,
        filename,
        expected_compression,
    );
}

fn get_test_data_path(filename: &str) -> PathBuf {
    shared_test_data_dir().join("compression").join(filename)
}

#[test_case("GZIP", "test_data.csv.gz"; "gzip")]
#[test_case("BZIP2", "test_data.csv.bz2"; "bzip2")]
#[test_case("BROTLI", "test_data.csv.br"; "brotli")]
#[test_case("ZSTD", "test_data.csv.zst"; "zstd")]
#[test_case("DEFLATE", "test_data.csv.deflate"; "deflate")]
#[test_case("RAW_DEFLATE", "test_data.csv.raw_deflate"; "raw_deflate")]
fn should_upload_compressed_files_with_source_compression_set_to_explicit_types(
    compression: &str,
    filename: &str,
) {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = format!("TEST_STAGE_{}", compression);

    // And File with standard type (GZIP, BZIP2, BROTLI, ZSTD, DEFLATE, RAW_DEFLATE)
    let test_file_path = get_test_data_path(filename);

    // When File is uploaded with SOURCE_COMPRESSION set to explicit type
    let put_result = upload_to_stage_with_options(
        &client,
        &stage_name,
        test_file_path.to_str().unwrap(),
        &format!("SOURCE_COMPRESSION={}", compression),
    );

    // Then Target compression has correct type and all PUT results are correct
    assert_put_results(put_result, filename, compression, filename, compression);
}

#[test]
fn should_not_compress_file_when_source_compression_set_to_auto_detect_and_auto_compress_set_to_false()
 {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_NONE_NO_AUTO_COMPRESS";

    // And Uncompressed file
    let filename = "test_data.csv";
    let test_file_path = get_test_data_path(filename);

    // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to FALSE
    let put_result = upload_to_stage_with_options(
        &client,
        stage_name,
        test_file_path.to_str().unwrap(),
        "SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=FALSE",
    );

    // Then File is not compressed
    assert_put_results(put_result, filename, "NONE", filename, "NONE");
}

#[test]
fn should_not_compress_file_when_source_compression_set_to_none_and_auto_compress_set_to_false() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_NONE_NO_AUTO_COMPRESS";

    // And Uncompressed file
    let filename = "test_data.csv";
    let test_file_path = get_test_data_path(filename);

    // When File is uploaded with SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to FALSE
    let put_result = upload_to_stage_with_options(
        &client,
        stage_name,
        test_file_path.to_str().unwrap(),
        "SOURCE_COMPRESSION=NONE AUTO_COMPRESS=FALSE",
    );

    // Then File is not compressed
    assert_put_results(put_result, filename, "NONE", filename, "NONE");
}

#[test]
fn should_compress_uncompressed_file_when_source_compression_set_to_auto_detect_and_auto_compress_set_to_true()
 {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_COMPRESS";

    // And Uncompressed file
    let filename = "test_data.csv";
    let test_file_path = get_test_data_path(filename);

    // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to TRUE
    let put_result = upload_to_stage_with_options(
        &client,
        stage_name,
        test_file_path.to_str().unwrap(),
        "SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=TRUE",
    );

    // Then Target compression has GZIP type and all PUT results are correct
    let expected_target = format!("{filename}.gz");
    assert_put_results(put_result, filename, "NONE", &expected_target, "GZIP");
}

#[test]
fn should_compress_uncompressed_file_when_source_compression_set_to_none_and_auto_compress_set_to_true()
 {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_NONE_AUTO_COMPRESS";

    // And Uncompressed file
    let filename = "test_data.csv";
    let test_file_path = get_test_data_path(filename);

    // When File is uploaded with SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to TRUE
    let put_result = upload_to_stage_with_options(
        &client,
        stage_name,
        test_file_path.to_str().unwrap(),
        "SOURCE_COMPRESSION=NONE AUTO_COMPRESS=TRUE",
    );

    // Then Target compression has GZIP type and all PUT results are correct
    let expected_target = format!("{filename}.gz");
    assert_put_results(put_result, filename, "NONE", &expected_target, "GZIP");
}

fn assert_put_results(
    put_data: ExecuteResult,
    expected_source_filename: &str,
    expected_source_compression: &str,
    expected_target_filename: &str,
    expected_target_compression: &str,
) {
    // Process Arrow result and extract PutResult
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);
    let put_result: PutResult = arrow_helper
        .fetch_one()
        .unwrap_or_else(|_| panic!("Failed to fetch PUT result for {expected_source_filename}"));

    assert_eq!(
        put_result.source, expected_source_filename,
        "Source filename should be '{expected_source_filename}', got '{}'",
        put_result.source
    );
    assert_eq!(
        put_result.target, expected_target_filename,
        "Target filename should be '{expected_target_filename}', got '{}'",
        put_result.target
    );
    assert_eq!(
        put_result.source_compression, expected_source_compression,
        "Source compression should be '{expected_source_compression}', got '{}'",
        put_result.source_compression
    );
    assert_eq!(
        put_result.target_compression, expected_target_compression,
        "Target compression should be '{expected_target_compression}', got '{}'",
        put_result.target_compression
    );
    assert_eq!(
        put_result.status, "UPLOADED",
        "Upload status should be 'UPLOADED', got '{}'",
        put_result.status
    );
}
