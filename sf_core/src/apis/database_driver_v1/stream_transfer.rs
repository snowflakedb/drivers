//! Connection-level streaming file transfer handlers.
//!
//! Backs `ConnectionUploadStream` (JDBC `uploadStream`, Python `file_stream`)
//! and `ConnectionDownloadStream` (JDBC `downloadStream`).
//!
//! Upload contract: the caller hands us a fully-formed PUT SQL plus the bytes
//! it has already drained from its stream. We run the SQL through GS for stage
//! credentials + encryption material, then upload `ByteSource::Bytes(data)` via
//! the shared in-memory upload path. The caller shapes the SQL (AUTO_COMPRESS,
//! OVERWRITE, etc.); we don't second-guess it beyond requiring a PUT.
//!
//! Download contract: the caller passes structured fields (`stage_name`,
//! `source_filename`, `decompress`). We synthesize a GET SQL targeting a
//! tempdir, run `download_single_file`, read the resulting file, optionally
//! gunzip, and return the bytes. The asymmetry vs. upload reflects that
//! `download_single_file` writes to a path — switching it to an in-memory sink
//! is a separate refactor (the Python reference's `_download_stream` is itself
//! unimplemented).
//!
//! Both handlers reuse the connection-context + GS-execute helpers from
//! `statement.rs` so the retry/refresh plumbing lives in one place.

use std::sync::Arc;

use snafu::{OptionExt, ResultExt};
use tokio::sync::Mutex;
use tracing::Instrument;

use super::connection::{Connection, FinalSessionNames, RefreshContext};
use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::query::{StageInfoRefreshContext, perform_stream_upload, stream_stage_info_refresher};
use super::result_set::{ResultSetInfo, resolve_reader_ctx, response_to_descriptor};
use super::statement::{query_context, skip_leading_whitespace_and_comments};
use crate::config::rest_parameters::QueryParameters;
use crate::file_manager::{self, SingleDownloadData, download_single_file};
use crate::handle_manager::Handle;
use crate::rest::snowflake::{
    QueryExecutionMode, QueryInput, RestError, query_response, snowflake_query_with_client,
};

impl DatabaseDriverV1 {
    /// Execute a PUT SQL using `data` as the upload source. See module docs
    /// for the contract. Returns a `ResultSetInfo` whose handle/descriptor
    /// have the same shape as a normal `statement_execute_query` on PUT.
    pub async fn connection_upload_stream(
        &self,
        conn_handle: Handle,
        sql: String,
        data: Vec<u8>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<ResultSetInfo, ApiError> {
        let session_id = self.session_id_for_conn(conn_handle).await;
        async {
            let conn_ptr = self
                .connections
                .get_obj(conn_handle)
                .context(InvalidArgumentSnafu {
                    argument: "Connection handle not found",
                })?;

            if !is_put_sql(&sql) {
                return InvalidArgumentSnafu {
                    argument:
                        "ConnectionUploadStream requires a PUT SQL statement (SQL does not begin with PUT)"
                            .to_string(),
                }
                .fail();
            }

            let (query_parameters, http_client, retry_policy) = query_context(&conn_ptr).await?;

            let response = run_sql_against_gs(
                &conn_ptr,
                &http_client,
                &query_parameters,
                &retry_policy,
                sql.clone(),
                cancel.clone(),
            )
            .await?;

            // Update session parameter cache (mirrors the normal PUT path).
            if response.success {
                let conn = conn_ptr.lock().await;
                conn.update_session_params_cache(
                    &sql,
                    response.data.parameters.as_ref(),
                    &FinalSessionNames {
                        database: response.data.final_database_name.clone(),
                        schema: response.data.final_schema_name.clone(),
                        warehouse: response.data.final_warehouse_name.clone(),
                        role: response.data.final_role_name.clone(),
                    },
                )
                .await;
            }

            let gs_data = response.data;
            let refresh_ctx = StageInfoRefreshContext {
                sql: sql.clone(),
                query_parameters: query_parameters.clone(),
                conn: conn_ptr.clone(),
                cancel: cancel.clone(),
            };
            let use_s3_regional_url = conn_ptr
                .lock()
                .await
                .use_s3_regional_url_session_param()
                .await;

            // The file transfer itself uses the put/get retry policy (distinct
            // from the query policy that drove the GS PUT above).
            let put_get_policy = {
                let conn = conn_ptr.lock().await;
                crate::config::retry::RetryPolicy::put_get(&conn.connection_seed)
            };

            let rowset_data = perform_stream_upload(
                &gs_data,
                &self.wrapper_presets,
                Some(refresh_ctx),
                use_s3_regional_url,
                &put_get_policy,
                data,
                cancel,
            )
            .await
            .context(QueryResponseProcessSnafu)?;

            let descriptor = response_to_descriptor(&gs_data, &self.wrapper_presets);
            let reader_ctx = resolve_reader_ctx(&conn_ptr).await?;
            let handle = self.create_result_set(descriptor.clone(), rowset_data, reader_ctx);
            Ok(ResultSetInfo { handle, descriptor })
        }
        .instrument(crate::snowflake_op_span!(
            "connection_upload_stream",
            session_id
        ))
        .await
    }

    /// Download a file from a stage and return its bytes (optionally gunzipped).
    /// See module docs for the contract.
    pub async fn connection_download_stream(
        &self,
        conn_handle: Handle,
        stage_name: &str,
        source_filename: &str,
        decompress: bool,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<Vec<u8>, ApiError> {
        let session_id = self.session_id_for_conn(conn_handle).await;
        async {
            let conn_ptr = self
                .connections
                .get_obj(conn_handle)
                .context(InvalidArgumentSnafu {
                    argument: "Connection handle not found",
                })?;

            let stage_path = build_stage_path(stage_name, source_filename);
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
            // GET syntax does not support parameterized bindings for stage paths
            // or local locations; stage_name and source_filename are caller-supplied
            // (mirroring the file-path GET), and local_dir_url is internally generated
            // by tempfile::tempdir().
            let get_sql = format!("GET {stage_path} {local_dir_url}");

            let (query_parameters, http_client, retry_policy) = query_context(&conn_ptr).await?;

            let response = run_sql_against_gs(
                &conn_ptr,
                &http_client,
                &query_parameters,
                &retry_policy,
                get_sql.clone(),
                cancel.clone(),
            )
            .await?;

            if !response.success {
                return InvalidArgumentSnafu {
                    argument: response
                        .message
                        .unwrap_or_else(|| "GET command rejected by server".to_string()),
                }
                .fail();
            }

            let gs_data = response.data;
            let (use_s3_regional_url, unsafe_file_write) = {
                let conn = conn_ptr.lock().await;
                let unsafe_file_write = conn.unsafe_file_write();
                let use_s3_regional_url = conn.use_s3_regional_url_session_param().await;
                (use_s3_regional_url, unsafe_file_write)
            };

            let download_data = gs_data
                .to_file_download_data(
                    &self.wrapper_presets.put_get_resultset_flavor,
                    use_s3_regional_url,
                    unsafe_file_write,
                )
                .map_err(|e| {
                    InvalidArgumentSnafu {
                        argument: format!("Failed to parse GET response: {e}"),
                    }
                    .build()
                })?;

            if download_data.src_locations.is_empty() {
                return InvalidArgumentSnafu {
                    argument: format!("File not found on stage: {source_filename}"),
                }
                .fail();
            }

            let initial_snapshot = gs_data
                .stage_info_snapshot()
                .map_err(|e| {
                    InvalidArgumentSnafu {
                        argument: format!("Failed to extract stage info from GET response: {e}"),
                    }
                    .build()
                })?
                .ok_or_else(|| {
                    InvalidArgumentSnafu {
                        argument: "GET response missing stage credentials".to_string(),
                    }
                    .build()
                })?;

            let refresh_ctx = StageInfoRefreshContext {
                sql: get_sql,
                query_parameters,
                conn: conn_ptr.clone(),
                cancel: cancel.clone(),
            };
            let mut refresher = stream_stage_info_refresher(refresh_ctx, initial_snapshot);

            let put_get_policy = {
                let conn = conn_ptr.lock().await;
                crate::config::retry::RetryPolicy::put_get(&conn.connection_seed)
            };

            let single_download = SingleDownloadData {
                // SAFETY: `src_locations` is guaranteed non-empty by the
                // `is_empty()` check above, so `next()` always yields.
                src_location: download_data.src_locations.into_iter().next().unwrap(),
                local_location: tmp_dir.path().to_str().unwrap_or("/tmp").to_string(),
                stage_info: download_data.stage_info,
                encryption_material: download_data
                    .encryption_materials
                    .into_iter()
                    .next()
                    .flatten(),
                presigned_url: download_data.presigned_urls.into_iter().next().flatten(),
                flavor: download_data.flavor,
                multipart: download_data.multipart,
                unsafe_file_write: download_data.unsafe_file_write,
            };

            let mut refresher_dyn: Option<&mut dyn file_manager::StageInfoRefresher> =
                Some(&mut refresher);
            download_single_file(
                single_download,
                &put_get_policy,
                0,
                &mut refresher_dyn,
                cancel,
            )
            .await
            .map_err(|e| {
                if e.is_cancelled() {
                    return CancelledSnafu.build();
                }
                InvalidArgumentSnafu {
                    argument: format!("Download failed: {e}"),
                }
                .build()
            })?;

            // The downloaded file lives at `<tmp_dir>/<basename(source_filename)>`.
            // Read the first regular file we find — there will be exactly one.
            // `read_dir`, the file read, and the (CPU-bound) gzip inflate are all
            // blocking and the payload can be arbitrarily large, so run the whole
            // post-download step on the blocking pool rather than the async
            // executor thread.
            let dir_path = tmp_dir.path().to_path_buf();
            let raw_bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, ApiError> {
                let path = std::fs::read_dir(&dir_path)
                    .map_err(|e| {
                        InvalidArgumentSnafu {
                            argument: format!("Failed to read temp directory: {e}"),
                        }
                        .build()
                    })?
                    .next()
                    .and_then(|r| r.ok())
                    .map(|e| e.path())
                    .ok_or_else(|| {
                        InvalidArgumentSnafu {
                            argument: "Downloaded file not found in temp directory".to_string(),
                        }
                        .build()
                    })?;
                let bytes = std::fs::read(&path).map_err(|e| {
                    InvalidArgumentSnafu {
                        argument: format!("Failed to read downloaded file: {e}"),
                    }
                    .build()
                })?;
                if decompress {
                    crate::compression::decompress_data(&bytes).map_err(|e| {
                        InvalidArgumentSnafu {
                            argument: format!("Decompression failed: {e}"),
                        }
                        .build()
                    })
                } else {
                    Ok(bytes)
                }
            })
            .await
            .map_err(|e| {
                InvalidArgumentSnafu {
                    argument: format!("Download post-processing task failed: {e}"),
                }
                .build()
            })??;
            drop(tmp_dir);

            Ok(raw_bytes)
        }
        .instrument(crate::snowflake_op_span!(
            "connection_download_stream",
            session_id
        ))
        .await
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Run `sql` through the GS query path with master-token refresh on each retry,
/// matching the loop `statement.rs` uses for blocking PUT/GET execution.
async fn run_sql_against_gs(
    conn_ptr: &Arc<Mutex<Connection>>,
    http_client: &reqwest::Client,
    query_parameters: &QueryParameters,
    retry_policy: &crate::config::retry::RetryPolicy,
    sql: String,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<query_response::Response, ApiError> {
    let query_input = QueryInput::new(sql);

    let mut ctx = RefreshContext::from_arc(conn_ptr).await?;
    let mut last_error: Option<RestError> = None;
    loop {
        let session_token = ctx.refresh_token(last_error).await?;
        match snowflake_query_with_client(
            http_client,
            query_parameters.clone(),
            session_token.reveal(),
            query_input.clone(),
            retry_policy,
            QueryExecutionMode::Blocking,
            cancel.clone(),
        )
        .await
        {
            Ok(result) => return Ok(result),
            Err(e) => last_error = Some(e),
        }
    }
}

/// Returns `true` when `sql` (after stripping leading whitespace/comments)
/// starts with `PUT` followed by whitespace or a comment marker.
fn is_put_sql(sql: &str) -> bool {
    let s = skip_leading_whitespace_and_comments(sql);
    if s.len() < 4 {
        return false;
    }
    let prefix = &s[..3];
    let next_char = s.as_bytes()[3];
    prefix.eq_ignore_ascii_case("PUT")
        && (next_char.is_ascii_whitespace() || next_char == b'/' || next_char == b'-')
}

/// Builds the full stage path for GET: `<stage_name>/<source_filename>`,
/// avoiding a double slash if `stage_name` already ends with `/`.
fn build_stage_path(stage_name: &str, source_filename: &str) -> String {
    let stage = stage_name.trim_end_matches('/');
    format!("{stage}/{source_filename}")
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn is_put_sql_basic() {
        assert!(is_put_sql("PUT file://x @s"));
        assert!(is_put_sql("put file://x @s"));
        assert!(is_put_sql("  PUT file://x @s"));
        assert!(is_put_sql("/* hi */ PUT file://x @s"));
        assert!(is_put_sql("-- ok\nPUT file://x @s"));
    }

    #[test]
    fn is_put_sql_rejects_non_put() {
        assert!(!is_put_sql("SELECT 1"));
        assert!(!is_put_sql("GET @s file:///tmp"));
        assert!(!is_put_sql("PUTS"));
        assert!(!is_put_sql(""));
        assert!(!is_put_sql("PUT"));
    }
}
