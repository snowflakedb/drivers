pub mod common;
use common::arrow_result_helper::ArrowResultHelper;
use common::put_get_common::*;
use common::test_utils::*;

use flate2::{
    Compression,
    write::{DeflateEncoder, GzEncoder},
};
use std::fs;
use std::io::Write;

#[test]
fn test_put_source_compression_auto_detect_standard_types() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_STANDARD";
    let temp_dir = tempfile::TempDir::new().unwrap();
    let content = "1,2,3\n";

    // Setup stage once for all tests
    client.create_temporary_stage(stage_name);

    // Test cases for standard compression types that follow the same pattern
    let test_cases = [
        ("test_gzip.csv.gz", "GZIP"),
        ("test_bzip2.csv.bz2", "BZIP2"),
        ("test_brotli.csv.br", "BROTLI"),
        ("test_zstd.csv.zst", "ZSTD"),
        ("test_deflate.csv.deflate", "DEFLATE"),
    ];

    for (filename, expected_compression) in test_cases {
        // Create compressed test file
        let test_file_path = temp_dir.path().join(filename);
        let file = fs::File::create(&test_file_path).unwrap();

        // Create appropriate encoder based on the compression type
        match expected_compression {
            "GZIP" => {
                let mut encoder = GzEncoder::new(file, Compression::default());
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            "BZIP2" => {
                let mut encoder = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            "BROTLI" => {
                let mut encoder = brotli::CompressorWriter::new(file, 4096, 6, 22);
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.flush().unwrap();
            }
            "ZSTD" => {
                let mut encoder = zstd::stream::write::Encoder::new(file, 3).unwrap();
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            "DEFLATE" => {
                let mut encoder = DeflateEncoder::new(file, Compression::default());
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            _ => panic!("Unsupported compression type in test: {expected_compression}"),
        }

        // Upload file with AUTO_DETECT
        let put_sql = format!(
            "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT",
            test_file_path.to_str().unwrap().replace("\\", "/")
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
fn test_put_source_compression_auto_detect_raw_deflate() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_RAW_DEFLATE";
    let filename = "test_raw_deflate.csv.raw_deflate";

    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    let compressed_data = {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all("1,2,3\n".as_bytes()).unwrap();
        encoder.finish().unwrap()
    };
    fs::write(&test_file_path, compressed_data).unwrap();

    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    let put_data = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);

    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    assert_eq!(put_result.source, filename);
    assert_eq!(put_result.target, filename);
    assert_eq!(put_result.source_compression, "RAW_DEFLATE");
    assert_eq!(put_result.target_compression, "RAW_DEFLATE");
    assert_eq!(put_result.status, "UPLOADED");
}

#[test]
fn test_put_source_compression_auto_detect_none_no_auto_compress() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_NONE_NO_AUTO_COMPRESS";
    let filename = "test_none.csv";

    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    fs::write(&test_file_path, "1,2,3\n").unwrap();

    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=FALSE",
        test_file_path.to_str().unwrap().replace("\\", "/")
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
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_NONE_WITH_AUTO_COMPRESS";
    let filename = "test_none.csv";

    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    fs::write(&test_file_path, "1,2,3\n").unwrap();

    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=TRUE",
        test_file_path.to_str().unwrap().replace("\\", "/")
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
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_UNSUPPORTED";
    let filename = "test_auto_detect.csv.lz";

    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    fs::write(&test_file_path, "1,2,3\n").unwrap();

    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );

    // This should fail because .lz extension indicates unsupported LZIP compression
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
fn test_put_source_compression_auto_detect_content_based() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_AUTO_DETECT_CONTENT";
    let filename = "test_auto_detect_no_extension";

    // Set up test environment - create a gzipped file without extension
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    let file = fs::File::create(&test_file_path).unwrap();
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder.write_all("1,2,3\n".as_bytes()).unwrap();
    encoder.finish().unwrap();

    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=AUTO_DETECT",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    let put_data = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);

    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    assert_eq!(put_result.source, filename);
    assert_eq!(put_result.target, filename);
    // Should detect GZIP based on file content since there's no extension
    assert_eq!(put_result.source_compression, "GZIP");
    assert_eq!(put_result.target_compression, "GZIP");
    assert_eq!(put_result.status, "UPLOADED");
}

// Tests for explicit SOURCE_COMPRESSION specification

#[test]
fn test_put_source_compression_explicit_standard_types() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_EXPLICIT_COMPRESSION";
    let temp_dir = tempfile::TempDir::new().unwrap();
    let content = "1,2,3\n";

    // Setup stage once for all tests
    client.create_temporary_stage(stage_name);

    // Test cases for explicitly specified compression types
    let test_cases = [
        ("test_explicit_gzip.dat", "GZIP"),
        ("test_explicit_bzip2.dat", "BZIP2"),
        ("test_explicit_brotli.dat", "BROTLI"),
        ("test_explicit_zstd.dat", "ZSTD"),
        ("test_explicit_deflate.dat", "DEFLATE"),
    ];

    for (filename, compression_type) in test_cases {
        // Create compressed test file (using .dat extension to avoid auto-detection)
        let test_file_path = temp_dir.path().join(filename);
        let file = fs::File::create(&test_file_path).unwrap();

        // Create appropriate encoder based on the compression type
        match compression_type {
            "GZIP" => {
                let mut encoder = GzEncoder::new(file, Compression::default());
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            "BZIP2" => {
                let mut encoder = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            "BROTLI" => {
                let mut encoder = brotli::CompressorWriter::new(file, 4096, 6, 22);
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.flush().unwrap();
            }
            "ZSTD" => {
                let mut encoder = zstd::stream::write::Encoder::new(file, 3).unwrap();
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            "DEFLATE" => {
                let mut encoder = DeflateEncoder::new(file, Compression::default());
                encoder.write_all(content.as_bytes()).unwrap();
                encoder.finish().unwrap();
            }
            _ => panic!("Unsupported compression type in test: {compression_type}"),
        }

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
fn test_put_source_compression_explicit_raw_deflate() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_EXPLICIT_RAW_DEFLATE";
    let filename = "test_explicit_raw_deflate.dat";

    // Set up test environment - raw deflate needs special handling
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    let compressed_data = {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all("1,2,3\n".as_bytes()).unwrap();
        encoder.finish().unwrap()
    };
    fs::write(&test_file_path, compressed_data).unwrap();

    // Setup stage and upload file with explicit SOURCE_COMPRESSION
    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=RAW_DEFLATE",
        test_file_path.to_str().unwrap().replace("\\", "/")
    );
    let put_data = client.execute_query(&put_sql);
    let mut arrow_helper = ArrowResultHelper::from_result(put_data);

    let put_result: PutResult = arrow_helper
        .fetch_one()
        .expect("Failed to fetch PUT result");

    // Verify no double compression occurred
    assert_eq!(put_result.source, filename);
    assert_eq!(put_result.target, filename);
    assert_eq!(put_result.source_compression, "RAW_DEFLATE");
    assert_eq!(put_result.target_compression, "RAW_DEFLATE");
    assert_eq!(put_result.status, "UPLOADED");
}

#[test]
fn test_put_source_compression_explicit_none_no_auto_compress() {
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_EXPLICIT_NONE_NO_AUTO_COMPRESS";
    let filename = "test_explicit_none.dat";

    // Set up test environment - uncompressed file
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    fs::write(&test_file_path, "1,2,3\n").unwrap();

    // Setup stage and upload file with explicit SOURCE_COMPRESSION=NONE
    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=NONE AUTO_COMPRESS=FALSE",
        test_file_path.to_str().unwrap().replace("\\", "/")
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
    let mut client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_EXPLICIT_WITH_AUTO_COMPRESS";
    let filename = "test_explicit_with_auto.dat";

    // Set up test environment - uncompressed file
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join(filename);
    fs::write(&test_file_path, "1,2,3\n").unwrap();

    // Setup stage and upload file with SOURCE_COMPRESSION=NONE and AUTO_COMPRESS=TRUE
    client.create_temporary_stage(stage_name);
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} SOURCE_COMPRESSION=NONE AUTO_COMPRESS=TRUE",
        test_file_path.to_str().unwrap().replace("\\", "/")
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
