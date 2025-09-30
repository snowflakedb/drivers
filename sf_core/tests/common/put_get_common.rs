pub use super::arrow_deserialize::ArrowDeserialize;
pub use super::test_utils::SnowflakeTestClient;
use sf_core::protobuf_gen::database_driver_v1::ExecuteResult;

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
) -> ExecuteResult {
    upload_to_stage_with_options(client, stage_name, file_pattern, "")
}

pub fn upload_to_stage_with_options(
    client: &SnowflakeTestClient,
    stage_name: &str,
    file_pattern: &str,
    options: &str,
) -> ExecuteResult {
    client.create_temporary_stage(stage_name);
    let put_sql = build_put_command(stage_name, file_pattern, options);
    client.execute_query(&put_sql)
}

pub fn get_file_from_stage(
    client: &SnowflakeTestClient,
    stage_name: &str,
    filename: &str,
) -> (ExecuteResult, tempfile::TempDir) {
    let download_dir = tempfile::TempDir::new().unwrap();
    let get_sql = format!(
        "GET @{stage_name}/{filename} file://{}/",
        download_dir.path().to_str().unwrap().replace("\\", "/")
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
    let mut put_sql = format!(
        "PUT 'file://{}' @{stage_name}",
        file_path_or_pattern.replace("\\", "/")
    );

    if !options.is_empty() {
        put_sql.push_str(&format!(" {options}"));
    }
    put_sql
}
