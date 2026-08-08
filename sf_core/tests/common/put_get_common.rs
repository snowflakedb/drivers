pub use super::arrow_deserialize::ArrowDeserialize;
use crate::common::file_utils::path_to_sql_uri;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use sf_core::file_manager::internal::compute_sha256_digest;
use sf_core::file_manager::types::ByteSource;
use sf_core::protobuf::generated::database_driver_v1::ResultSetGetStreamResponse;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Just over the server's 200 MiB PUT/GET threshold, so a file this size
/// forces cloud multipart on upload and parallel ranged GETs on download.
/// Shared by the multipart-roundtrip e2e tests (plain and live-mitm-proxied).
pub const MULTIPART_FILE_LEN: u64 = 210 * 1024 * 1024;

/// Writes `len` deterministic, position-dependent bytes (a tiny LCG) so a
/// mis-ordered part/range on reassembly would change the digest.
pub fn write_payload(path: &Path, len: u64) {
    let file = std::fs::File::create(path).expect("create payload file");
    let mut writer = BufWriter::new(file);
    let mut buf = vec![0u8; 1024 * 1024];
    let mut state: u32 = 0x9e37_79b9;
    let mut remaining = len;
    while remaining > 0 {
        let chunk = remaining.min(buf.len() as u64) as usize;
        for b in buf.iter_mut().take(chunk) {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *b = (state >> 24) as u8;
        }
        writer.write_all(&buf[..chunk]).expect("write payload");
        remaining -= chunk as u64;
    }
    writer.flush().expect("flush payload");
}

/// SHA-256 digest of the file at `path`.
pub fn file_digest(path: &Path) -> String {
    compute_sha256_digest(&ByteSource::Path(path.to_path_buf())).expect("digest")
}

// Structured types for Snowflake command results using our arrow_deserialize macro
#[derive(ArrowDeserialize, Debug, PartialEq)]
pub struct PutResult {
    pub source: String,
    pub target: String,
    pub source_size: i64,
    pub target_size: i64,
    pub source_compression: String,
    pub target_compression: String,
    pub status: String,
    pub message: String,
}

#[derive(ArrowDeserialize, Debug, PartialEq)]
pub struct GetResult {
    pub file: String,
    pub size: i64,
    pub status: String,
    pub message: String,
}

pub fn upload_to_stage(
    client: &SnowflakeTestClient,
    stage_name: &str,
    file_pattern: &str,
) -> ResultSetGetStreamResponse {
    upload_to_stage_with_options(client, stage_name, file_pattern, "")
}

pub fn upload_to_stage_with_options(
    client: &SnowflakeTestClient,
    stage_name: &str,
    file_pattern: &str,
    options: &str,
) -> ResultSetGetStreamResponse {
    client.create_temporary_stage(stage_name);
    let put_sql = build_put_command(stage_name, file_pattern, options);
    client.execute_query(&put_sql)
}

pub fn get_file_from_stage(
    client: &SnowflakeTestClient,
    stage_name: &str,
    filename: &str,
) -> (ResultSetGetStreamResponse, tempfile::TempDir) {
    let download_dir = tempfile::TempDir::new().unwrap();
    let get_sql = format!(
        "GET @{stage_name}/{filename} file://{}/",
        path_to_sql_uri(download_dir.path())
    );
    let get_result = client.execute_query(&get_sql);
    (get_result, download_dir)
}

pub fn assert_file_exists(download_dir: &tempfile::TempDir, filename: &str) {
    let file_path = download_dir.path().join(filename);
    assert!(
        file_path.exists(),
        "Downloaded file should exist at {file_path:?}",
    );
}

pub fn build_put_command(stage_name: &str, file_path_or_pattern: &str, options: &str) -> String {
    let resolved = path_to_sql_uri(std::path::Path::new(file_path_or_pattern));
    let mut put_sql = format!("PUT 'file://{resolved}' @{stage_name}");

    if !options.is_empty() {
        put_sql.push_str(&format!(" {options}"));
    }
    put_sql
}
