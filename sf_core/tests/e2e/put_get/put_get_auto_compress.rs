use crate::common::file_utils::shared_test_data_dir;
use crate::common::put_get_common::assert_file_exists;
use crate::common::put_get_common::get_file_from_stage;
use crate::common::put_get_common::upload_to_stage_with_options;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use flate2::bufread::GzDecoder;
use std::fs;
use std::path::PathBuf;

#[test]
fn should_compress_the_file_before_uploading_to_stage_when_auto_compress_set_to_true() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_PUT_GET_AUTO_COMPRESS_TRUE";
    let (uncompressed_filename, uncompressed_reference_file_path) = uncompressed_test_file();
    let compressed_filename = format!("{uncompressed_filename}.gz");

    // When File is uploaded to stage with AUTO_COMPRESS set to true
    upload_to_stage_with_options(
        &client,
        stage_name,
        uncompressed_reference_file_path.to_str().unwrap(),
        "AUTO_COMPRESS=TRUE",
    );

    // Then Only compressed file should be downloaded
    let (_get_result, download_dir) =
        get_file_from_stage(&client, stage_name, &uncompressed_filename);
    assert_file_exists(&download_dir, &compressed_filename);
    assert_file_not_exist(&download_dir, &uncompressed_filename);

    // And Have correct content
    //
    // The gzip wire bytes faithfully reproduce the legacy Python file-PUT
    // shape (RFC 1952 §2.3.1.10, plus compress_file_with_gzip +
    // normalize_gzip_header):
    //   FLG = 0x08 (FNAME present)
    //   FNAME = `len(basename) + 2` 0x20 spaces, NUL-terminated
    //   MTIME = 0
    //   XFL = 2 (derived from Compression::best(), level 9)
    //   OS = 255 (CPython gzip.py hardcodes b'\xff')
    // Byte-equality against the reference .gz is intentionally avoided —
    // the reference is a 26-byte no-FNAME flate2-default fixture, not
    // the legacy file-PUT shape — so we assert the wire shape directly
    // and verify the payload via decompression.
    assert_downloaded_gzip_matches_python_legacy_shape(
        &download_dir,
        &compressed_filename,
        &uncompressed_filename,
        &uncompressed_reference_file_path,
    );
}

#[test]
fn should_not_compress_the_file_before_uploading_to_stage_when_auto_compress_set_to_false() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_PUT_GET_AUTO_COMPRESS_FALSE";
    let (uncompressed_filename, uncompressed_reference_file_path) = uncompressed_test_file();
    let compressed_filename = format!("{uncompressed_filename}.gz");

    // When File is uploaded to stage with AUTO_COMPRESS set to false
    upload_to_stage_with_options(
        &client,
        stage_name,
        uncompressed_reference_file_path.to_str().unwrap(),
        "AUTO_COMPRESS=FALSE",
    );

    // Then Only uncompressed file should be downloaded
    let (_get_result, download_dir) =
        get_file_from_stage(&client, stage_name, &uncompressed_filename);
    assert_file_exists(&download_dir, &uncompressed_filename);
    assert_file_not_exist(&download_dir, &compressed_filename);

    // And Have correct content
    assert_downloaded_content_matches_reference(
        &download_dir,
        &uncompressed_filename,
        &uncompressed_reference_file_path,
    );
}

fn uncompressed_test_file() -> (String, PathBuf) {
    (
        "test_data.csv".to_string(),
        shared_test_data_dir()
            .join("compression")
            .join("test_data.csv"),
    )
}

fn assert_file_not_exist(download_dir: &tempfile::TempDir, filename: &str) {
    let file_path = download_dir.path().join(filename);
    assert!(
        !file_path.exists(),
        "File should not exist at {file_path:?}",
    );
}

fn assert_downloaded_content_matches_reference(
    download_dir: &tempfile::TempDir,
    downloaded_filename: &str,
    reference_file_path: &std::path::Path,
) {
    let expected_file_path = download_dir.path().join(downloaded_filename);
    let downloaded_content = fs::read(&expected_file_path).unwrap();
    let reference_content = fs::read(reference_file_path).unwrap();
    assert_eq!(
        downloaded_content, reference_content,
        "Downloaded content should match reference content"
    );
}

/// Verify that a downloaded gzip file matches the legacy Python file-PUT
/// wire shape byte-for-byte in the metadata-bearing header bytes (FLG,
/// FNAME, MTIME, XFL, OS) and that the payload decompresses to the
/// reference CSV. The deflate-stream bytes themselves are not compared:
/// they vary deterministically with the source CSV, not with header
/// metadata, so payload equality after decompression is the right contract.
fn assert_downloaded_gzip_matches_python_legacy_shape(
    download_dir: &tempfile::TempDir,
    downloaded_filename: &str,
    source_basename: &str,
    uncompressed_reference: &std::path::Path,
) {
    use std::io::Read;
    let downloaded_path = download_dir.path().join(downloaded_filename);
    let bytes = fs::read(&downloaded_path).unwrap();
    assert!(bytes.len() >= 10, "gzip stream must include 10-byte header");
    assert_eq!((bytes[0], bytes[1]), (0x1f, 0x8b), "gzip magic");

    assert_ne!(
        bytes[3] & 0x08,
        0,
        "FLG byte 0x{:02x} should have FNAME (0x08) bit set",
        bytes[3],
    );
    assert_eq!(
        &bytes[4..8],
        &[0, 0, 0, 0],
        "MTIME should be zeroed (matches normalize_gzip_header)",
    );
    assert_eq!(
        bytes[8], 2,
        "XFL should be 2 (derived from Compression::best(), level 9)",
    );
    assert_eq!(
        bytes[9], 0xff,
        "OS should be 255 (CPython gzip.py hardcodes b'\\xff')",
    );

    let header_filename = GzDecoder::new(bytes.as_slice())
        .header()
        .and_then(|h| h.filename())
        .map(<[u8]>::to_vec)
        .expect("gzip header should carry a FNAME field");
    let expected_blanked = vec![b' '; source_basename.len() + 2];
    assert_eq!(
        header_filename, expected_blanked,
        "FNAME should be `len(basename) + 2` spaces (matches normalize_gzip_header)",
    );

    let mut decoder = GzDecoder::new(bytes.as_slice());
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap();
    let reference_bytes = fs::read(uncompressed_reference).unwrap();
    assert_eq!(
        decompressed, reference_bytes,
        "Decompressed downloaded content should match the original CSV",
    );
}
