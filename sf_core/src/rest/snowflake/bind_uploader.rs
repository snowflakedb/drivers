//! Bind Stage Uploader for large array bindings
//!
//! When array bindings exceed a threshold, we upload them to a temporary stage
//! (SYSTEM$BIND) as CSV files instead of sending them inline in the JSON request.
//! This is the same approach used by the official Go and JDBC drivers.

use std::collections::HashMap;
use std::io::Write;

use chrono::Utc;
use snafu::{Location, ResultExt, Snafu};
use uuid::Uuid;

use super::QueryParameters;
use super::query_request::BindParameter;
use crate::file_manager::{self, EncryptionMaterial, SingleUploadData, SourceCompressionParam};

/// The name of the temporary bind stage
pub const BIND_STAGE_NAME: &str = "SYSTEM$BIND";

/// Default threshold for when to use bind stage upload (number of bind values)
/// If total bind values (columns * rows) exceeds this, use stage upload
pub const DEFAULT_BIND_STAGE_THRESHOLD: usize = 65536; // 64K values

/// Maximum size of a single CSV file chunk (10MB like JDBC)
const INPUT_STREAM_BUFFER_SIZE: usize = 10 * 1024 * 1024;

#[derive(Debug, Snafu)]
pub enum BindUploadError {
    #[snafu(display("Failed to create bind stage"))]
    StageCreation {
        source: super::RestError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to upload bind data: {message}"))]
    Upload {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to upload bind data"))]
    UploadRest {
        source: super::RestError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to serialize bind data to CSV"))]
    CsvSerialization {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Mismatched column lengths in bind data"))]
    ColumnLengthMismatch {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Empty bind data"))]
    EmptyBindData {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("File upload failed"))]
    FileUpload {
        source: file_manager::FileManagerError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("IO error"))]
    Io {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Result of bind stage upload
pub struct BindStageResult {
    /// The stage path to use in the query (e.g., "@SYSTEM$BIND/uuid")
    pub stage_path: String,
}

/// Upload array bindings to the bind stage
pub async fn upload_bindings_to_stage(
    client: &reqwest::Client,
    query_parameters: QueryParameters,
    session_token: String,
    bindings: &HashMap<String, BindParameter>,
    session_timezone: Option<String>,
) -> Result<BindStageResult, BindUploadError> {
    let request_id = Uuid::new_v4();
    let stage_path = format!("@{}/{}", BIND_STAGE_NAME, request_id);

    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_debug.log")
        .and_then(|mut f| {
            writeln!(
                f,
                "DEBUG bind_uploader: stage_path={}, session_timezone={:?}",
                stage_path, session_timezone
            )
        });

    // First, create the temporary stage if needed
    create_bind_stage(client, query_parameters.clone(), session_token.clone()).await?;

    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_debug.log")
        .and_then(|mut f| writeln!(f, "DEBUG bind_uploader: stage created successfully"));

    // Convert bindings to CSV rows, using session timezone for timestamp formatting
    let csv_data = bindings_to_csv(bindings, session_timezone.as_deref())?;

    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_debug.log")
        .and_then(|mut f| {
            writeln!(
                f,
                "DEBUG bind_uploader: csv_data size={} bytes",
                csv_data.len()
            )
        });

    // Upload CSV data in chunks
    upload_csv_chunks(
        client,
        query_parameters,
        session_token,
        &stage_path,
        csv_data,
    )
    .await?;

    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_debug.log")
        .and_then(|mut f| {
            writeln!(
                f,
                "DEBUG bind_uploader: upload complete, returning stage_path={}",
                stage_path
            )
        });

    Ok(BindStageResult { stage_path })
}

/// Create the SYSTEM$BIND temporary stage if it doesn't exist
async fn create_bind_stage(
    client: &reqwest::Client,
    query_parameters: QueryParameters,
    session_token: String,
) -> Result<(), BindUploadError> {
    let create_stage_sql = format!(
        "CREATE OR REPLACE TEMPORARY STAGE {} file_format=(type=csv field_optionally_enclosed_by='\"')",
        BIND_STAGE_NAME
    );

    let _ = super::snowflake_query_internal(
        client,
        query_parameters,
        session_token,
        create_stage_sql,
        None,
        None,
        false,
    )
    .await
    .context(StageCreationSnafu)?;

    Ok(())
}

/// Convert bind parameters to CSV format
/// Each row in the CSV corresponds to one set of parameter values
fn bindings_to_csv(
    bindings: &HashMap<String, BindParameter>,
    session_timezone: Option<&str>,
) -> Result<Vec<u8>, BindUploadError> {
    if bindings.is_empty() {
        return Err(BindUploadError::EmptyBindData {
            location: Location::default(),
        });
    }

    // Get the number of columns and rows
    let num_columns = bindings.len();

    // Sort keys to ensure consistent column order (1, 2, 3, ...)
    let mut keys: Vec<_> = bindings.keys().collect();
    keys.sort_by(|a, b| {
        let a_num: usize = a.parse().unwrap_or(0);
        let b_num: usize = b.parse().unwrap_or(0);
        a_num.cmp(&b_num)
    });

    // Determine number of rows from first column
    let first_binding = bindings.get(keys[0]).unwrap();
    let num_rows = match &first_binding.value {
        serde_json::Value::Array(arr) => arr.len(),
        _ => 1, // Single value means 1 row
    };

    // Validate all columns have the same number of rows
    for key in &keys {
        let binding = bindings.get(*key).unwrap();
        let col_rows = match &binding.value {
            serde_json::Value::Array(arr) => arr.len(),
            _ => 1,
        };
        if col_rows != num_rows {
            return Err(BindUploadError::ColumnLengthMismatch {
                location: Location::default(),
            });
        }
    }

    // Build CSV data
    let mut csv_buffer = Vec::with_capacity(num_rows * num_columns * 32);

    for row_idx in 0..num_rows {
        for (col_idx, key) in keys.iter().enumerate() {
            if col_idx > 0 {
                csv_buffer.push(b',');
            }

            let binding = bindings.get(*key).unwrap();
            let value = match &binding.value {
                serde_json::Value::Array(arr) => &arr[row_idx],
                single_value => single_value,
            };

            // Write escaped CSV value, converting epoch seconds to date/timestamp format if needed
            write_csv_value_with_type(&mut csv_buffer, value, &binding.type_, session_timezone);
        }
        csv_buffer.push(b'\n');
    }

    Ok(csv_buffer)
}

/// Write a JSON value as a CSV field (with proper escaping)
/// For bind stage upload, dates and timestamps need to be in ISO format, not epoch seconds
fn write_csv_value_with_type(
    buffer: &mut Vec<u8>,
    value: &serde_json::Value,
    type_: &str,
    session_timezone: Option<&str>,
) {
    match value {
        serde_json::Value::Null => {
            // Empty field for NULL
        }
        serde_json::Value::String(s) => {
            format_csv_string(buffer, s, type_, session_timezone);
        }
        serde_json::Value::Number(n) => {
            format_csv_string(buffer, &n.to_string(), type_, session_timezone);
        }
        serde_json::Value::Bool(b) => {
            buffer.extend_from_slice(if *b { b"true" } else { b"false" });
        }
        _ => {
            // For arrays/objects, serialize as JSON string
            let json_str = value.to_string();
            let escaped = escape_csv_string(&json_str);
            buffer.extend_from_slice(escaped.as_bytes());
        }
    }
}

fn format_csv_string(buffer: &mut Vec<u8>, s: &str, type_: &str, session_timezone: Option<&str>) {
    let formatted = match type_.to_uppercase().as_str() {
        "DATE" => {
            if s.contains('-') && s.len() >= 10 {
                s.to_string()
            } else if let Ok(days) = s.parse::<i64>() {
                let secs = days * 86400;
                let datetime =
                    chrono::DateTime::from_timestamp(secs, 0).unwrap_or_else(chrono::Utc::now);
                datetime.format("%Y-%m-%d").to_string()
            } else {
                s.to_string()
            }
        }
        "TIME" => {
            // Convert nanoseconds since midnight to time format "HH:MM:SS.nnnnnnnnn"
            if let Ok(nanos) = s.parse::<i64>() {
                let total_secs = nanos / 1_000_000_000;
                let remaining_nanos = (nanos % 1_000_000_000) as u32;
                let hours = total_secs / 3600;
                let minutes = (total_secs % 3600) / 60;
                let seconds = total_secs % 60;
                format!(
                    "{:02}:{:02}:{:02}.{:09}",
                    hours, minutes, seconds, remaining_nanos
                )
            } else {
                s.to_string()
            }
        }
        "TIMESTAMP_NTZ" => {
            if let Some((secs, nanos)) = parse_epoch_seconds(s) {
                format_timestamp_ntz_epoch(secs, nanos)
            } else {
                s.to_string()
            }
        }
        "TIMESTAMP_LTZ" | "TIMESTAMP_TZ" | "TIMESTAMP" => {
            if let Some((secs, nanos)) = parse_epoch_seconds(s) {
                format_timestamp_ltz_epoch(secs, nanos, session_timezone)
                    .unwrap_or_else(|| s.to_string())
            } else {
                s.to_string()
            }
        }
        _ => s.to_string(),
    };
    let escaped = escape_csv_string(&formatted);
    buffer.extend_from_slice(escaped.as_bytes());
}

/// Escape a string for CSV format
fn escape_csv_string(s: &str) -> String {
    // If the string contains quotes, commas, or newlines, we need to quote it
    let needs_quoting = s.contains('"') || s.contains(',') || s.contains('\n') || s.contains('\r');

    if needs_quoting {
        // Double any existing quotes and wrap in quotes
        let escaped = s.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        s.to_string()
    }
}

fn parse_epoch_seconds(value: &str) -> Option<(i64, u32)> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(dot_idx) = trimmed.find('.') {
        // Split into integer and fractional parts
        let secs_part = &trimmed[..dot_idx];
        let frac_part = &trimmed[dot_idx + 1..];
        if secs_part.is_empty() {
            return None;
        }
        let secs = secs_part.parse::<i64>().ok()?;
        let mut nanos_str: String = frac_part.chars().take(9).collect();
        while nanos_str.len() < 9 {
            nanos_str.push('0');
        }
        let nanos = nanos_str.parse::<u32>().ok()?;
        Some((secs, nanos))
    } else {
        let secs = trimmed.parse::<i64>().ok()?;
        Some((secs, 0))
    }
}

fn format_timestamp_ltz_epoch(
    secs: i64,
    nanos: u32,
    session_timezone: Option<&str>,
) -> Option<String> {
    let utc_dt = chrono::DateTime::from_timestamp(secs, nanos)?;
    if let Some(tz_name) = session_timezone {
        if let Ok(tz) = tz_name.parse::<chrono_tz::Tz>() {
            let local_dt = utc_dt.with_timezone(&tz);
            return Some(local_dt.format("%Y-%m-%d %H:%M:%S%.9f").to_string());
        }
    }
    Some(utc_dt.format("%Y-%m-%d %H:%M:%S%.9f").to_string())
}

fn format_timestamp_ntz_epoch(secs: i64, nanos: u32) -> String {
    if let Some(naive_dt) = chrono::NaiveDateTime::from_timestamp_opt(secs, nanos) {
        format!("{}", naive_dt.format("%Y-%m-%d %H:%M:%S%.9f"))
    } else {
        format!("{secs}.{nanos:09}")
    }
}

/// Upload CSV data to the stage in chunks
async fn upload_csv_chunks(
    client: &reqwest::Client,
    query_parameters: QueryParameters,
    session_token: String,
    stage_path: &str,
    csv_data: Vec<u8>,
) -> Result<(), BindUploadError> {
    // Split data into chunks if needed
    let mut file_count = 0;
    let mut start = 0;

    while start < csv_data.len() {
        let end = std::cmp::min(start + INPUT_STREAM_BUFFER_SIZE, csv_data.len());

        // Find a newline boundary to avoid splitting rows
        let chunk_end = if end < csv_data.len() {
            // Find the last newline before the end
            csv_data[start..end]
                .iter()
                .rposition(|&b| b == b'\n')
                .map(|pos| start + pos + 1)
                .unwrap_or(end)
        } else {
            end
        };

        file_count += 1;
        let chunk = &csv_data[start..chunk_end];

        // Upload this chunk using PUT command with file stream
        upload_chunk(
            client,
            query_parameters.clone(),
            session_token.clone(),
            stage_path,
            file_count,
            chunk,
        )
        .await?;

        start = chunk_end;
    }

    Ok(())
}

/// Upload a single chunk to the stage
async fn upload_chunk(
    client: &reqwest::Client,
    query_parameters: QueryParameters,
    session_token: String,
    stage_path: &str,
    file_number: usize,
    data: &[u8],
) -> Result<(), BindUploadError> {
    // Compress the data using gzip
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder
        .write_all(data)
        .map_err(|_| BindUploadError::CsvSerialization {
            location: Location::default(),
        })?;
    let compressed = encoder
        .finish()
        .map_err(|_| BindUploadError::CsvSerialization {
            location: Location::default(),
        })?;

    // Create a temporary file with the data
    let temp_path = format!("/tmp/bind_data_{}.csv.gz", file_number);
    std::fs::write(&temp_path, &compressed).context(IoSnafu)?;

    // Use PUT command to get stage info and encryption material
    let put_command = format!(
        "PUT 'file://{}' '{}' OVERWRITE=TRUE AUTO_COMPRESS=FALSE",
        temp_path, stage_path
    );

    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_debug.log")
        .and_then(|mut f| {
            writeln!(
                f,
                "DEBUG upload_chunk: PUT command={}, compressed_size={}, original_size={}",
                put_command,
                compressed.len(),
                data.len()
            )
        });

    // Execute the PUT command to get stage info
    let result = super::snowflake_query_internal(
        client,
        query_parameters,
        session_token,
        put_command,
        None,
        None,
        false,
    )
    .await
    .context(UploadRestSnafu)?;

    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_debug.log")
        .and_then(|mut f| {
            writeln!(
                f,
                "DEBUG upload_chunk: PUT result success={}, command={:?}",
                result.success, result.data.command
            )
        });

    if !result.success {
        let _ = std::fs::remove_file(&temp_path);
        return Err(BindUploadError::Upload {
            message: result
                .message
                .unwrap_or_else(|| "Unknown error".to_string()),
            location: Location::default(),
        });
    }

    // Check if this is an UPLOAD command - if so, we need to do the actual file transfer
    if result.data.command.as_deref() == Some("UPLOAD") {
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rust_debug.log")
            .and_then(|mut f| {
                writeln!(
                    f,
                    "DEBUG upload_chunk: Got UPLOAD command, performing actual file transfer"
                )
            });

        // Extract stage info and encryption material from response
        let stage_info: file_manager::StageInfo = result
            .data
            .stage_info
            .as_ref()
            .ok_or_else(|| BindUploadError::Upload {
                message: "Missing stage_info in PUT response".to_string(),
                location: Location::default(),
            })?
            .try_into()
            .map_err(|e| BindUploadError::Upload {
                message: format!("Failed to parse stage_info: {:?}", e),
                location: Location::default(),
            })?;

        let encryption_materials: Vec<EncryptionMaterial> = result
            .data
            .encryption_material
            .as_ref()
            .ok_or_else(|| BindUploadError::Upload {
                message: "Missing encryption_material in PUT response".to_string(),
                location: Location::default(),
            })?
            .into();

        let encryption_material = encryption_materials
            .first()
            .ok_or_else(|| BindUploadError::Upload {
                message: "Empty encryption_material in PUT response".to_string(),
                location: Location::default(),
            })?
            .clone();

        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rust_debug.log")
            .and_then(|mut f| {
                writeln!(
                    f,
                    "DEBUG upload_chunk: stage_info bucket={}, key_prefix={}",
                    stage_info.bucket, stage_info.key_prefix
                )
            });

        // Create upload data for the file
        let upload_data = SingleUploadData {
            file_path: temp_path.clone(),
            filename: format!("bind_data_{}.csv.gz", file_number),
            stage_info,
            encryption_material,
            auto_compress: false, // Already compressed
            source_compression: SourceCompressionParam::Gzip,
            overwrite: true,
        };

        // Perform the actual upload
        let upload_result = match file_manager::upload_single_file(upload_data).await {
            Ok(result) => result,
            Err(e) => {
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/rust_debug.log")
                    .and_then(|mut f| {
                        writeln!(f, "DEBUG upload_chunk: File upload FAILED: {:?}", e)
                    });
                return Err(e).context(FileUploadSnafu);
            }
        };

        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rust_debug.log")
            .and_then(|mut f| {
                writeln!(
                    f,
                    "DEBUG upload_chunk: File upload complete, status={}",
                    upload_result.status
                )
            });
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    Ok(())
}

/// Check if bindings should use stage upload based on size
pub fn should_use_bind_stage(bindings: &HashMap<String, BindParameter>, threshold: usize) -> bool {
    if bindings.is_empty() {
        return false;
    }

    // Count total number of bind values
    let total_values: usize = bindings
        .values()
        .map(|b| match &b.value {
            serde_json::Value::Array(arr) => arr.len(),
            _ => 1,
        })
        .sum();

    total_values > threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_csv_string_simple() {
        assert_eq!(escape_csv_string("hello"), "hello");
        assert_eq!(escape_csv_string("123"), "123");
    }

    #[test]
    fn test_escape_csv_string_with_comma() {
        assert_eq!(escape_csv_string("hello,world"), "\"hello,world\"");
    }

    #[test]
    fn test_escape_csv_string_with_quote() {
        assert_eq!(escape_csv_string("say \"hello\""), "\"say \"\"hello\"\"\"");
    }

    #[test]
    fn test_escape_csv_string_with_newline() {
        assert_eq!(escape_csv_string("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn test_escape_csv_string_with_carriage_return() {
        assert_eq!(escape_csv_string("line1\rline2"), "\"line1\rline2\"");
    }

    #[test]
    fn test_bindings_to_csv_simple() {
        let mut bindings = HashMap::new();
        bindings.insert(
            "1".to_string(),
            BindParameter {
                type_: "FIXED".to_string(),
                value: serde_json::json!(["1", "2", "3"]),
                format: None,
                schema: None,
            },
        );
        bindings.insert(
            "2".to_string(),
            BindParameter {
                type_: "TEXT".to_string(),
                value: serde_json::json!(["a", "b", "c"]),
                format: None,
                schema: None,
            },
        );

        let csv = bindings_to_csv(&bindings, None).unwrap();
        let csv_str = String::from_utf8(csv).unwrap();

        assert_eq!(csv_str, "1,a\n2,b\n3,c\n");
    }

    #[test]
    fn test_bindings_to_csv_with_nulls() {
        let mut bindings = HashMap::new();
        bindings.insert(
            "1".to_string(),
            BindParameter {
                type_: "FIXED".to_string(),
                value: serde_json::json!(["1", null, "3"]),
                format: None,
                schema: None,
            },
        );

        let csv = bindings_to_csv(&bindings, None).unwrap();
        let csv_str = String::from_utf8(csv).unwrap();

        assert_eq!(csv_str, "1\n\n3\n");
    }

    #[test]
    fn test_bindings_to_csv_with_special_chars() {
        let mut bindings = HashMap::new();
        bindings.insert(
            "1".to_string(),
            BindParameter {
                type_: "TEXT".to_string(),
                value: serde_json::json!(["hello,world", "say \"hi\"", "line1\nline2"]),
                format: None,
                schema: None,
            },
        );

        let csv = bindings_to_csv(&bindings, None).unwrap();
        let csv_str = String::from_utf8(csv).unwrap();

        assert_eq!(
            csv_str,
            "\"hello,world\"\n\"say \"\"hi\"\"\"\n\"line1\nline2\"\n"
        );
    }

    #[test]
    fn test_bindings_to_csv_single_value() {
        let mut bindings = HashMap::new();
        bindings.insert(
            "1".to_string(),
            BindParameter {
                type_: "FIXED".to_string(),
                value: serde_json::json!("42"),
                format: None,
                schema: None,
            },
        );

        let csv = bindings_to_csv(&bindings, None).unwrap();
        let csv_str = String::from_utf8(csv).unwrap();

        assert_eq!(csv_str, "42\n");
    }

    #[test]
    fn test_bindings_to_csv_empty() {
        let bindings = HashMap::new();
        let result = bindings_to_csv(&bindings, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_bindings_to_csv_column_mismatch() {
        let mut bindings = HashMap::new();
        bindings.insert(
            "1".to_string(),
            BindParameter {
                type_: "FIXED".to_string(),
                value: serde_json::json!(["1", "2", "3"]),
                format: None,
                schema: None,
            },
        );
        bindings.insert(
            "2".to_string(),
            BindParameter {
                type_: "TEXT".to_string(),
                value: serde_json::json!(["a", "b"]), // Only 2 values instead of 3
                format: None,
                schema: None,
            },
        );

        let result = bindings_to_csv(&bindings, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_should_use_bind_stage_below_threshold() {
        let mut bindings = HashMap::new();
        bindings.insert(
            "1".to_string(),
            BindParameter {
                type_: "FIXED".to_string(),
                value: serde_json::json!(["1", "2", "3"]),
                format: None,
                schema: None,
            },
        );

        // 3 values, threshold 10 -> should not use stage
        assert!(!should_use_bind_stage(&bindings, 10));
    }

    #[test]
    fn test_should_use_bind_stage_above_threshold() {
        let mut bindings = HashMap::new();
        bindings.insert(
            "1".to_string(),
            BindParameter {
                type_: "FIXED".to_string(),
                value: serde_json::json!(["1", "2", "3"]),
                format: None,
                schema: None,
            },
        );

        // 3 values, threshold 2 -> should use stage
        assert!(should_use_bind_stage(&bindings, 2));
    }

    #[test]
    fn test_should_use_bind_stage_empty() {
        let bindings = HashMap::new();
        assert!(!should_use_bind_stage(&bindings, 10));
    }

    #[test]
    fn test_should_use_bind_stage_multiple_columns() {
        let mut bindings = HashMap::new();
        bindings.insert(
            "1".to_string(),
            BindParameter {
                type_: "FIXED".to_string(),
                value: serde_json::json!(["1", "2"]),
                format: None,
                schema: None,
            },
        );
        bindings.insert(
            "2".to_string(),
            BindParameter {
                type_: "TEXT".to_string(),
                value: serde_json::json!(["a", "b"]),
                format: None,
                schema: None,
            },
        );

        // 4 total values (2 columns * 2 rows), threshold 3 -> should use stage
        assert!(should_use_bind_stage(&bindings, 3));
        // 4 total values, threshold 5 -> should not use stage
        assert!(!should_use_bind_stage(&bindings, 5));
    }

    #[test]
    fn test_write_csv_value_null() {
        let mut buffer = Vec::new();
        write_csv_value_with_type(&mut buffer, &serde_json::Value::Null, "TEXT", None);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_write_csv_value_string() {
        let mut buffer = Vec::new();
        write_csv_value_with_type(&mut buffer, &serde_json::json!("hello"), "TEXT", None);
        assert_eq!(String::from_utf8(buffer).unwrap(), "hello");
    }

    #[test]
    fn test_write_csv_value_number() {
        let mut buffer = Vec::new();
        write_csv_value_with_type(&mut buffer, &serde_json::json!(42), "FIXED", None);
        assert_eq!(String::from_utf8(buffer).unwrap(), "42");
    }

    #[test]
    fn test_write_csv_value_bool() {
        let mut buffer = Vec::new();
        write_csv_value_with_type(&mut buffer, &serde_json::json!(true), "BOOLEAN", None);
        assert_eq!(String::from_utf8(buffer).unwrap(), "true");

        let mut buffer = Vec::new();
        write_csv_value_with_type(&mut buffer, &serde_json::json!(false), "BOOLEAN", None);
        assert_eq!(String::from_utf8(buffer).unwrap(), "false");
    }

    #[test]
    fn test_write_csv_value_float() {
        let mut buffer = Vec::new();
        write_csv_value_with_type(&mut buffer, &serde_json::json!(3.14), "REAL", None);
        assert_eq!(String::from_utf8(buffer).unwrap(), "3.14");
    }

    #[test]
    fn test_write_csv_value_date() {
        let mut buffer = Vec::new();
        // 1449734400 seconds = 2015-12-10
        write_csv_value_with_type(
            &mut buffer,
            &serde_json::json!("1449734400.000000000"),
            "DATE",
            None,
        );
        assert_eq!(String::from_utf8(buffer).unwrap(), "2015-12-10");
    }

    #[test]
    fn test_write_csv_value_timestamp_utc() {
        let mut buffer = Vec::new();
        // 1198674855.123456789 seconds = 2007-12-26 13:14:15.123456789 UTC
        // With no session timezone, output is in UTC
        write_csv_value_with_type(
            &mut buffer,
            &serde_json::json!("1198674855.123456789"),
            "TIMESTAMP_LTZ",
            None,
        );
        let result = String::from_utf8(buffer).unwrap();
        assert_eq!(result, "2007-12-26 13:14:15.123456789");
    }

    #[test]
    fn test_write_csv_value_timestamp_with_timezone() {
        let mut buffer = Vec::new();
        // 1198674855 seconds = 2007-12-26 13:14:15 UTC = 2007-12-26 08:14:15 EST (America/New_York)
        write_csv_value_with_type(
            &mut buffer,
            &serde_json::json!("1198674855.123456789"),
            "TIMESTAMP_LTZ",
            Some("America/New_York"),
        );
        let result = String::from_utf8(buffer).unwrap();
        // Should be converted to EST (UTC-5)
        assert_eq!(result, "2007-12-26 08:14:15.123456789");
    }

    #[test]
    fn test_write_csv_value_time() {
        let mut buffer = Vec::new();
        // 47655123456789 nanoseconds = 13:14:15.123456789
        write_csv_value_with_type(
            &mut buffer,
            &serde_json::json!("47655123456789"),
            "TIME",
            None,
        );
        assert_eq!(String::from_utf8(buffer).unwrap(), "13:14:15.123456789");
    }

    #[test]
    fn test_parse_epoch_seconds_exact() {
        let (secs, nanos) = parse_epoch_seconds("1198674855.123456789").unwrap();
        assert_eq!(secs, 1_198_674_855);
        assert_eq!(nanos, 123_456_789);

        let (secs, nanos) = parse_epoch_seconds("-1.000000001").unwrap();
        assert_eq!(secs, -1);
        assert_eq!(nanos, 1);

        let (secs, nanos) = parse_epoch_seconds("42").unwrap();
        assert_eq!(secs, 42);
        assert_eq!(nanos, 0);
    }
}
