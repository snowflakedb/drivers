pub use super::arrow_deserialize::ArrowDeserialize;
use super::test_utils::SnowflakeTestClient;
use sf_core::protobuf_gen::database_driver_v1::ExecuteResult;
use std::path::Path;
use tempfile::TempDir;

// Structured types for Snowflake command results using manual ArrowDeserialize
#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
pub struct GetResult {
    pub file: String,
    pub size: i64,
    pub status: String,
    pub message: String,
}

impl ArrowDeserialize for PutResult {
    fn deserialize_one(
        batch: &arrow::record_batch::RecordBatch,
        row_index: usize,
    ) -> Result<Self, String> {
        let col = |name: &str| -> Result<arrow::array::ArrayRef, String> {
            // Prefer exact match; fall back to lowercase name to tolerate schema variations
            let schema = batch.schema();
            let index_result = schema
                .index_of(name)
                .or_else(|_| schema.index_of(&name.to_ascii_lowercase()));
            let idx = index_result.map_err(|e| e.to_string())?;
            Ok(batch.column(idx).clone())
        };
        let str_at = |array: &arrow::array::ArrayRef| -> Result<String, String> {
            array
                .as_any()
                .downcast_ref::<arrow::array::StringArray>()
                .ok_or_else(|| "Expected StringArray".to_string())
                .map(|a| a.value(row_index).to_string())
        };
        let i64_at = |array: &arrow::array::ArrayRef| -> Result<i64, String> {
            array
                .as_any()
                .downcast_ref::<arrow::array::Int64Array>()
                .ok_or_else(|| "Expected Int64Array".to_string())
                .map(|a| a.value(row_index))
        };
        Ok(PutResult {
            source: str_at(&col("SOURCE")?)?,
            target: str_at(&col("TARGET")?)?,
            source_size: i64_at(&col("SOURCE_SIZE")?)?,
            target_size: i64_at(&col("TARGET_SIZE")?)?,
            source_compression: str_at(&col("SOURCE_COMPRESSION")?)?,
            target_compression: str_at(&col("TARGET_COMPRESSION")?)?,
            status: str_at(&col("STATUS")?)?,
            message: str_at(&col("MESSAGE")?)?,
        })
    }
}

impl ArrowDeserialize for GetResult {
    fn deserialize_one(
        batch: &arrow::record_batch::RecordBatch,
        row_index: usize,
    ) -> Result<Self, String> {
        let col = |name: &str| -> Result<arrow::array::ArrayRef, String> {
            let schema = batch.schema();
            let index_result = schema
                .index_of(name)
                .or_else(|_| schema.index_of(&name.to_ascii_lowercase()));
            let idx = index_result.map_err(|e| e.to_string())?;
            Ok(batch.column(idx).clone())
        };
        let str_at = |array: &arrow::array::ArrayRef| -> Result<String, String> {
            array
                .as_any()
                .downcast_ref::<arrow::array::StringArray>()
                .ok_or_else(|| "Expected StringArray".to_string())
                .map(|a| a.value(row_index).to_string())
        };
        let i64_at = |array: &arrow::array::ArrayRef| -> Result<i64, String> {
            array
                .as_any()
                .downcast_ref::<arrow::array::Int64Array>()
                .ok_or_else(|| "Expected Int64Array".to_string())
                .map(|a| a.value(row_index))
        };
        Ok(GetResult {
            file: str_at(&col("FILE")?)?,
            size: i64_at(&col("SIZE")?)?,
            status: str_at(&col("STATUS")?)?,
            message: str_at(&col("MESSAGE")?)?,
        })
    }
}

/// Upload a file to a stage using the PUT command and return the execution result
pub fn upload_file_to_stage(
    client: &SnowflakeTestClient,
    stage_name: &str,
    file_path: &Path,
) -> ExecuteResult {
    let file_uri = file_path.to_str().unwrap().replace("\\", "/");
    let put_sql = format!("PUT 'file://{file_uri}' @{stage_name}");
    client.execute_query(&put_sql)
}

/// Upload a file to a stage with additional options (e.g., AUTO_COMPRESS=TRUE)
pub fn upload_file_to_stage_with_options(
    client: &SnowflakeTestClient,
    stage_name: &str,
    file_path: &Path,
    options: &str,
) -> ExecuteResult {
    let file_uri = file_path.to_str().unwrap().replace("\\", "/");
    let put_sql = format!("PUT 'file://{file_uri}' @{stage_name} {options}");
    client.execute_query(&put_sql)
}

/// Download a single file from a stage into a temporary directory and return (GET rowset, download_dir)
pub fn get_file_from_stage(
    client: &SnowflakeTestClient,
    stage_name: &str,
    filename: &str,
) -> (ExecuteResult, TempDir) {
    let download_dir = tempfile::tempdir().unwrap();
    let download_path_str = download_dir.path().to_str().unwrap().replace("\\", "/");
    // Allow either plain or gz suffix when downloading
    let pattern = format!("'^{filename}(\\.gz)?$'");
    let get_sql = format!("GET @{stage_name} file://{download_path_str} PATTERN={pattern}");
    let result = client.execute_query(&get_sql);
    (result, download_dir)
}

/// Assert that a file exists in the provided temporary download directory
pub fn assert_file_exists(download_dir: &TempDir, filename: &str) {
    let file_path = download_dir.path().join(filename);
    assert!(file_path.exists(), "File should exist at {file_path:?}");
}
