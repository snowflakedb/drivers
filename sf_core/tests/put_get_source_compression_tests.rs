pub mod common;
use crate::common::test_utils::shared_test_data_dir;
use common::arrow_result_helper::ArrowResultHelper;
use common::put_get_common::*;
use std::path::PathBuf;

#[test]
fn test_put_source_compression_auto_detect_standard_types() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_STANDARD";

    // Setup stage once for all tests
    client.create_temporary_stage(stage_name);

    // Test cases for standard compression types
    // RAW_DEFLATE is currently not auto-detected as it is not auto-detected in any existing drivers
    // TODO: Revisit while when we test more drivers, especially Go driver
    let test_cases = ["GZIP", "BZIP2", "BROTLI", "ZSTD", "DEFLATE"];

    for expected_compression in test_cases {
        let (filename, file_path) = test_file(expected_compression);

        // Upload file with AUTO_DETECT
        let put_sql = format!(
            "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT",
            file_path.to_str().unwrap().replace("\\", "/")
        );
        let put_data = client.execute_query(&put_sql);
        let mut arrow_helper = ArrowResultHelper::from_result(put_data);

        let put_result: PutResult = arrow_helper
            .fetch_one()
            .unwrap_or_else(|_| panic!("Failed to fetch PUT result for {filename}"));

        // Verify results
        assert_eq!(
            put_result.source, filename,
            "Source filename mismatch for {filename}"
        );
        assert_eq!(
            put_result.target, filename,
            "Target should equal source when file is already compressed for {filename}"
        );
        assert_eq!(
            put_result.source_compression, expected_compression,
            "Source compression type mismatch for {filename}"
        );
        assert_eq!(
            put_result.target_compression, expected_compression,
            "Target compression should match source for {filename}"
        );
        assert_eq!(
            put_result.status, "UPLOADED",
            "Upload status mismatch for {filename}"
        );
    }
}

#[test]
fn test_put_source_compression_auto_detect_none_no_auto_compress() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_NONE_NO_AUTO_COMPRESS";

    let (filename, file_path) = test_file("NONE");

    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=FALSE",
        file_path.to_str().unwrap().replace("\\", "/")
    );
    let put_data = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);

    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    assert_eq!(put_result.source, filename);
    assert_eq!(put_result.target, filename);
    assert_eq!(put_result.source_compression, "NONE");
    assert_eq!(put_result.target_compression, "NONE");
    assert_eq!(put_result.status, "UPLOADED");
}

#[test]
fn test_put_source_compression_auto_detect_none_with_auto_compress() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_NONE_WITH_AUTO_COMPRESS";

    let (filename, file_path) = test_file("NONE");

    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=TRUE",
        file_path.to_str().unwrap().replace("\\", "/")
    );
    let put_data = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);

    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    assert_eq!(put_result.source, filename);
    assert_eq!(put_result.target, format!("{filename}.gz"));
    assert_eq!(put_result.source_compression, "NONE");
    assert_eq!(put_result.target_compression, "GZIP");
    assert_eq!(put_result.status, "UPLOADED");
}

#[test]
fn test_put_source_compression_auto_detect_unsupported() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_UNSUPPORTED";

    let (_filename, file_path) = test_file("LZMA");

    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT",
        file_path.to_str().unwrap().replace("\\", "/")
    );

    // This should fail because LZMA compression type is not supported, but should be detected
    let result = client.execute_query_no_unwrap(&put_sql);

    assert!(
        matches!(
            &result,
            Err(e) if format!("{e:?}").contains("Unsupported compression type")
        ),
        "Expected unsupported compression error, got: {result:?}"
    );
}

#[test]
fn test_put_source_compression_explicit_standard_types() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_EXPLICIT_COMPRESSION";

    // Setup stage once for all tests
    client.create_temporary_stage(stage_name);

    // Test cases for explicitly specified compression types
    let test_cases = ["GZIP", "BZIP2", "BROTLI", "ZSTD", "DEFLATE", "RAW_DEFLATE"];

    for compression_type in test_cases {
        let (filename, test_file_path) = test_file(compression_type);

        // Upload file with explicit SOURCE_COMPRESSION
        let put_sql = format!(
            "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION={}",
            test_file_path.to_str().unwrap().replace("\\", "/"),
            compression_type
        );
        let put_data = client.execute_query(&put_sql);
        let mut arrow_helper = ArrowResultHelper::from_result(put_data);

        let put_result: PutResult = arrow_helper
            .fetch_one()
            .unwrap_or_else(|_| panic!("Failed to fetch PUT result for {filename}"));

        // Verify results - source and target should be the same (no double compression)
        assert_eq!(
            put_result.source, filename,
            "Source filename mismatch for {filename}"
        );
        assert_eq!(
            put_result.target, filename,
            "Target should equal source when file is already compressed for {filename}"
        );
        assert_eq!(
            put_result.source_compression, compression_type,
            "Source compression type mismatch for {filename}"
        );
        assert_eq!(
            put_result.target_compression, compression_type,
            "Target compression should match source for {filename}"
        );
        assert_eq!(
            put_result.status, "UPLOADED",
            "Upload status mismatch for {filename}"
        );
    }
}

#[test]
fn test_put_source_compression_explicit_none_no_auto_compress() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_EXPLICIT_NONE_NO_AUTO_COMPRESS";

    let (filename, file_path) = test_file("NONE");

    // Setup stage and upload file with explicit SOURCE_COMPRESSION=NONE
    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=NONE AUTO_COMPRESS=FALSE",
        file_path.to_str().unwrap().replace("\\", "/")
    );
    let put_data = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);

    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    // Verify no compression occurred
    assert_eq!(put_result.source, filename);
    assert_eq!(put_result.target, filename);
    assert_eq!(put_result.source_compression, "NONE");
    assert_eq!(put_result.target_compression, "NONE");
    assert_eq!(put_result.status, "UPLOADED");
}

#[test]
fn test_put_source_compression_explicit_with_auto_compress() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_EXPLICIT_WITH_AUTO_COMPRESS";

    let (filename, file_path) = test_file("NONE");

    // Setup stage and upload file with SOURCE_COMPRESSION=NONE and AUTO_COMPRESS=TRUE
    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=NONE AUTO_COMPRESS=TRUE",
        file_path.to_str().unwrap().replace("\\", "/")
    );
    let put_data = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);

    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    // Verify compression occurred due to AUTO_COMPRESS=TRUE
    assert_eq!(put_result.source, filename);
    assert_eq!(put_result.target, format!("{filename}.gz")); // Should be compressed
    assert_eq!(put_result.source_compression, "NONE");
    assert_eq!(put_result.target_compression, "GZIP"); // Should be compressed with GZIP
    assert_eq!(put_result.status, "UPLOADED");
}

fn compression_tests_dir() -> PathBuf {
    shared_test_data_dir().join("compression")
}

fn test_file(compression_type: &str) -> (String, PathBuf) {
    match compression_type {
        "GZIP" => (
            "test_data.csv.gz".to_string(),
            compression_tests_dir().join("test_data.csv.gz"),
        ),
        "BZIP2" => (
            "test_data.csv.bz2".to_string(),
            compression_tests_dir().join("test_data.csv.bz2"),
        ),
        "BROTLI" => (
            "test_data.csv.br".to_string(),
            compression_tests_dir().join("test_data.csv.br"),
        ),
        "ZSTD" => (
            "test_data.csv.zst".to_string(),
            compression_tests_dir().join("test_data.csv.zst"),
        ),
        "DEFLATE" => (
            "test_data.csv.deflate".to_string(),
            compression_tests_dir().join("test_data.csv.deflate"),
        ),
        "RAW_DEFLATE" => (
            "test_data.csv.raw_deflate".to_string(),
            compression_tests_dir().join("test_data.csv.raw_deflate"),
        ),
        "LZMA" => (
            "test_data.csv.xz".to_string(),
            compression_tests_dir().join("test_data.csv.xz"),
        ),
        "NONE" => (
            "test_data.csv".to_string(),
            compression_tests_dir().join("test_data.csv"),
        ),
        _ => panic!("Unsupported compression type: {compression_type}"),
    }
}
