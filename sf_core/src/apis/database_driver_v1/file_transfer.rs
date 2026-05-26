//! Connection-level streaming file transfer (gap 4 / gap 16).
//!
//! `connection_upload_stream` and `connection_download_stream` implement the
//! JDBC `uploadStream`/`downloadStream` API. The implementation follows
//! Option A from the PR design doc: a new RPC that receives in-memory bytes
//! across the JNI boundary, synthesizes a PUT/GET SQL to obtain stage
//! credentials and encryption material from GS, then calls the existing
//! `upload_single_file` / `download_single_file` helpers with those
//! credentials + `ByteSource::Bytes(...)` / a temp file path.
//!
//! Behavioral notes (all from snowflake-jdbc parity analysis):
//! - `compress_data = true` (JDBC default) causes the payload to be
//!   gzip-compressed before upload; the target filename gains a ".gz" suffix.
//! - `dest_prefix` is prepended to `dest_filename` with a "/" separator.
//!   An empty/absent prefix means stage root.
//! - The caller owns the stream/bytes; we do not close or free them.
//! - Errors are propagated as `ApiError` so the JNI bridge converts them to
//!   `SnowflakeSQLException` using the same path as every other RPC.

use super::connection::RefreshContext;
use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::query::StageCredsRefreshContext;
use crate::file_manager::{
    ByteSource, SingleDownloadData, SingleUploadData, SourceCompressionParam, StageCredsRefresher,
    download_single_file, upload_single_file,
};
use crate::handle_manager::Handle;
use crate::rest::snowflake::{QueryInput, snowflake_query_with_client};
use snafu::OptionExt;
use std::sync::atomic::Ordering;

/// Result returned to the protobuf dispatch layer for upload.
pub struct UploadStreamResult {
    pub status: String,
    pub target_filename: String,
}

/// Result returned to the protobuf dispatch layer for download.
pub struct DownloadStreamResult {
    pub data: Vec<u8>,
}

impl DatabaseDriverV1 {
    /// Upload `data` bytes to `stage_name` as `dest_filename`.
    ///
    /// Steps:
    /// 1. Build a synthetic `PUT file://<dest_filename> <stage_name>` SQL and
    ///    execute it through the existing GS query path to obtain encryption
    ///    material + stage credentials.
    /// 2. Call `upload_single_file` with `ByteSource::Bytes(data)` and the
    ///    stage info / encryption material from step 1.
    ///
    /// The `compress_data` parameter maps directly to the `UploadStreamConfig`
    /// `compressData` field. When `true`, `preprocess_file_before_upload` (inside
    /// `upload_single_file`) will gzip the bytes and append ".gz" to the target
    /// filename, matching snowflake-jdbc behavior.
    pub async fn connection_upload_stream(
        &self,
        conn_handle: Handle,
        stage_name: &str,
        dest_filename: &str,
        dest_prefix: Option<&str>,
        data: Vec<u8>,
        compress_data: bool,
    ) -> Result<UploadStreamResult, ApiError> {
        let conn_ptr = self
            .connections
            .get_obj(conn_handle)
            .context(InvalidArgumentSnafu {
                argument: "Connection handle not found",
            })?;

        // Build the effective destination path: prefix/filename or just filename.
        let effective_filename = build_dest_filename(dest_prefix, dest_filename);

        // Synthesize PUT SQL.  The `file://` placeholder path must match
        // `effective_filename` so that GS echoes the right `src_locations` entry
        // back.  `OVERWRITE = TRUE` matches the reference JDBC default for stream
        // uploads (uploadStream always uses overwrite semantics).
        let put_sql = format!("PUT file://{effective_filename} {stage_name} OVERWRITE = TRUE");

        let (query_parameters, http_client, retry_policy) = {
            let conn = conn_ptr.lock().await;
            if conn.is_closed.load(Ordering::SeqCst) {
                return Err(ConnectionClosedSnafu {}.build());
            }
            (
                conn.query_transport_parameters()?,
                conn.http_client
                    .clone()
                    .context(ConnectionNotInitializedSnafu)?,
                conn.retry_policy.clone(),
            )
        };

        // Execute the PUT SQL to get stage info from GS.
        let gs_response = {
            let mut ctx = RefreshContext::from_arc(&conn_ptr).await?;
            let mut last_error = None;
            loop {
                let session_token = ctx.refresh_token(last_error).await?;
                match snowflake_query_with_client(
                    &http_client,
                    query_parameters.clone(),
                    session_token.reveal(),
                    QueryInput::new(put_sql.clone()),
                    &retry_policy,
                    crate::rest::snowflake::QueryExecutionMode::Blocking,
                )
                .await
                {
                    Ok(r) => break Ok(r),
                    Err(e) => last_error = Some(e),
                }
            }
        }?;

        if !gs_response.success {
            return Err(InvalidArgumentSnafu {
                argument: gs_response
                    .message
                    .unwrap_or_else(|| "PUT command rejected by server".to_string()),
            }
            .build());
        }

        let gs_data = gs_response.data;

        // Build the stage-creds refresher (same pattern as statement.rs).
        let refresh_ctx = StageCredsRefreshContext {
            sql: put_sql,
            query_parameters,
            conn: conn_ptr.clone(),
        };
        let use_s3_regional_url = conn_ptr
            .lock()
            .await
            .use_s3_regional_url_session_param()
            .await;

        // Obtain the UploadData from GS response (gives us stage info + encryption
        // material). The `src_location_pattern` from this is the path placeholder
        // GS echoed back; we replace the actual data source with ByteSource::Bytes.
        let upload_data = gs_data
            .to_file_upload_data(
                self.wrapper_presets.put_get_resultset_flavor.clone(),
                self.wrapper_presets.legacy_odbc_compression_autodetect,
                use_s3_regional_url,
            )
            .map_err(|e| {
                InvalidArgumentSnafu {
                    argument: format!("Failed to parse PUT response: {e}"),
                }
                .build()
            })?;

        // Seed the refresher cache and build a stage-creds refresher.
        let initial_creds = gs_data
            .stage_info_creds()
            .map_err(|e| {
                InvalidArgumentSnafu {
                    argument: format!("Failed to extract stage creds from PUT response: {e}"),
                }
                .build()
            })?
            .ok_or_else(|| {
                InvalidArgumentSnafu {
                    argument: "PUT response missing stage credentials".to_string(),
                }
                .build()
            })?;

        let mut refresher =
            crate::apis::database_driver_v1::query::SnowflakeStageCredsRefresherPub::new(
                refresh_ctx,
                initial_creds,
            );

        let single_upload = SingleUploadData {
            source: ByteSource::Bytes(data),
            // The source column in PUT results uses the display name.
            source_path_str: effective_filename.clone(),
            filename: effective_filename.clone(),
            stage_info: upload_data.stage_info,
            encryption_material: upload_data.encryption_material,
            // The caller's `compress_data` flag overrides whatever GS returned
            // in `auto_compress`.  Reference JDBC ignores the GS-returned flag
            // for stream uploads and uses the per-call option exclusively.
            auto_compress: compress_data,
            source_compression: SourceCompressionParam::None,
            overwrite: upload_data.overwrite,
            flavor: upload_data.flavor,
            legacy_odbc_compression_autodetect: upload_data.legacy_odbc_compression_autodetect,
        };

        let mut refresher_dyn: Option<&mut dyn StageCredsRefresher> = Some(&mut refresher);
        let result = upload_single_file(single_upload, &mut refresher_dyn)
            .await
            .map_err(|e| {
                InvalidArgumentSnafu {
                    argument: format!("Upload failed: {e}"),
                }
                .build()
            })?;

        Ok(UploadStreamResult {
            status: result.status,
            target_filename: result.target,
        })
    }

    /// Download a single file from `stage_name` as `source_filename` and return
    /// its raw bytes (or decompressed bytes when `decompress = true`).
    ///
    /// Steps:
    /// 1. Synthesize `GET <stage_name>/<source_filename> file:///tmp` SQL to get
    ///    stage credentials and encryption material.
    /// 2. Download to a temp file via `download_single_file`.
    /// 3. Read the temp file into a `Vec<u8>`.
    /// 4. If `decompress`, gunzip the bytes.
    /// 5. Temp dir is dropped (auto-deleted) and we return the bytes.
    pub async fn connection_download_stream(
        &self,
        conn_handle: Handle,
        stage_name: &str,
        source_filename: &str,
        decompress: bool,
    ) -> Result<DownloadStreamResult, ApiError> {
        let conn_ptr = self
            .connections
            .get_obj(conn_handle)
            .context(InvalidArgumentSnafu {
                argument: "Connection handle not found",
            })?;

        // Snowflake GET syntax: GET @stage/path file:///local_dir
        // We construct the full stage path as "<stage_name>/<source_filename>".
        let stage_path = build_stage_path(stage_name, source_filename);

        // Create a temp directory for the download.
        let tmp_dir = tempfile::tempdir().map_err(|e| {
            InvalidArgumentSnafu {
                argument: format!("Failed to create temp directory: {e}"),
            }
            .build()
        })?;

        let local_dir_url = format!(
            "file://{}",
            tmp_dir.path().to_str().unwrap_or("/tmp").replace('\\', "/")
        );

        let get_sql = format!("GET {stage_path} {local_dir_url}");

        let (query_parameters, http_client, retry_policy) = {
            let conn = conn_ptr.lock().await;
            if conn.is_closed.load(Ordering::SeqCst) {
                return Err(ConnectionClosedSnafu {}.build());
            }
            (
                conn.query_transport_parameters()?,
                conn.http_client
                    .clone()
                    .context(ConnectionNotInitializedSnafu)?,
                conn.retry_policy.clone(),
            )
        };

        // Execute GET to get stage info from GS.
        let gs_response = {
            let mut ctx = RefreshContext::from_arc(&conn_ptr).await?;
            let mut last_error = None;
            loop {
                let session_token = ctx.refresh_token(last_error).await?;
                match snowflake_query_with_client(
                    &http_client,
                    query_parameters.clone(),
                    session_token.reveal(),
                    QueryInput::new(get_sql.clone()),
                    &retry_policy,
                    crate::rest::snowflake::QueryExecutionMode::Blocking,
                )
                .await
                {
                    Ok(r) => break Ok(r),
                    Err(e) => last_error = Some(e),
                }
            }
        }?;

        if !gs_response.success {
            return Err(InvalidArgumentSnafu {
                argument: gs_response
                    .message
                    .unwrap_or_else(|| "GET command rejected by server".to_string()),
            }
            .build());
        }

        let gs_data = gs_response.data;

        let refresh_ctx = StageCredsRefreshContext {
            sql: get_sql,
            query_parameters,
            conn: conn_ptr.clone(),
        };
        let use_s3_regional_url = conn_ptr
            .lock()
            .await
            .use_s3_regional_url_session_param()
            .await;

        let download_data = gs_data
            .to_file_download_data(
                &self.wrapper_presets.put_get_resultset_flavor,
                use_s3_regional_url,
            )
            .map_err(|e| {
                InvalidArgumentSnafu {
                    argument: format!("Failed to parse GET response: {e}"),
                }
                .build()
            })?;

        if download_data.src_locations.is_empty() {
            return Err(InvalidArgumentSnafu {
                argument: format!("File not found on stage: {source_filename}"),
            }
            .build());
        }

        let initial_creds = gs_data
            .stage_info_creds()
            .map_err(|e| {
                InvalidArgumentSnafu {
                    argument: format!("Failed to extract stage creds from GET response: {e}"),
                }
                .build()
            })?
            .ok_or_else(|| {
                InvalidArgumentSnafu {
                    argument: "GET response missing stage credentials".to_string(),
                }
                .build()
            })?;

        let mut refresher =
            crate::apis::database_driver_v1::query::SnowflakeStageCredsRefresherPub::new(
                refresh_ctx,
                initial_creds,
            );

        let single_download = SingleDownloadData {
            src_location: download_data.src_locations.into_iter().next().unwrap(),
            local_location: tmp_dir.path().to_str().unwrap_or("/tmp").to_string(),
            stage_info: download_data.stage_info,
            encryption_material: download_data
                .encryption_materials
                .into_iter()
                .next()
                .unwrap_or(None),
            flavor: download_data.flavor,
        };

        let mut refresher_dyn: Option<&mut dyn StageCredsRefresher> = Some(&mut refresher);
        let _result = download_single_file(single_download, &mut refresher_dyn)
            .await
            .map_err(|e| {
                InvalidArgumentSnafu {
                    argument: format!("Download failed: {e}"),
                }
                .build()
            })?;

        // Find the downloaded file in the temp directory: take the first entry.
        let dir_iter = std::fs::read_dir(tmp_dir.path()).map_err(|e| {
            InvalidArgumentSnafu {
                argument: format!("Failed to read temp directory: {e}"),
            }
            .build()
        })?;
        let downloaded_path = if let Some(entry) = dir_iter.into_iter().next() {
            let entry = entry.map_err(|e| {
                InvalidArgumentSnafu {
                    argument: format!("Failed to read directory entry: {e}"),
                }
                .build()
            })?;
            Some(entry.path())
        } else {
            None
        };

        let path = downloaded_path.ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "Downloaded file not found in temp directory".to_string(),
            }
            .build()
        })?;

        let raw_bytes = std::fs::read(&path).map_err(|e| {
            InvalidArgumentSnafu {
                argument: format!("Failed to read downloaded file: {e}"),
            }
            .build()
        })?;

        // Temp dir (and file) is dropped here, cleaning up automatically.
        drop(tmp_dir);

        let data = if decompress {
            crate::compression::decompress_data(&raw_bytes).map_err(|e| {
                InvalidArgumentSnafu {
                    argument: format!("Decompression failed: {e}"),
                }
                .build()
            })?
        } else {
            raw_bytes
        };

        Ok(DownloadStreamResult { data })
    }
}

/// Builds the destination filename on the stage, applying the prefix if any.
/// Empty/None prefix means the stage root (no prefix applied).
///
/// Matches reference JDBC: `destPrefix + "/" + destFileName` when prefix is set,
/// plain `destFileName` otherwise.
fn build_dest_filename(dest_prefix: Option<&str>, dest_filename: &str) -> String {
    match dest_prefix {
        Some(prefix) if !prefix.is_empty() => format!("{prefix}/{dest_filename}"),
        _ => dest_filename.to_string(),
    }
}

/// Builds the full stage path for GET: `<stage_name>/<source_filename>`.
/// Avoids double slashes if `stage_name` already ends with `/`.
fn build_stage_path(stage_name: &str, source_filename: &str) -> String {
    let stage = stage_name.trim_end_matches('/');
    format!("{stage}/{source_filename}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_dest_filename_no_prefix() {
        assert_eq!(build_dest_filename(None, "data.csv"), "data.csv");
        assert_eq!(build_dest_filename(Some(""), "data.csv"), "data.csv");
    }

    #[test]
    fn build_dest_filename_with_prefix() {
        assert_eq!(
            build_dest_filename(Some("mydir"), "data.csv"),
            "mydir/data.csv"
        );
        assert_eq!(
            build_dest_filename(Some("a/b/c"), "data.csv"),
            "a/b/c/data.csv"
        );
    }

    #[test]
    fn build_stage_path_basic() {
        assert_eq!(
            build_stage_path("@my_stage", "data.csv.gz"),
            "@my_stage/data.csv.gz"
        );
    }

    #[test]
    fn build_stage_path_trailing_slash() {
        assert_eq!(
            build_stage_path("@my_stage/", "data.csv"),
            "@my_stage/data.csv"
        );
    }
}
