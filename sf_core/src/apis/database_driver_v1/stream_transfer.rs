//! Connection-level streaming file transfer handlers.
//!
//! Backs `ConnectionUploadStream` / the chunked `ConnectionUploadStream{Begin,
//! Chunk,Finish,Abort}` RPCs (JDBC `uploadStream`, Python `file_stream`) and
//! `ConnectionDownloadStream` (JDBC `downloadStream`).
//!
//! Upload contract: bytes arrive whole (`ConnectionUploadStream`) or in
//! chunks (`ConnectionUploadStreamChunk`); either way we reassemble them into
//! a re-readable [`file_manager::ByteSource`] — in memory, or spooled to a
//! temp file past [`file_manager::SpooledBuffer`]'s threshold — then run the
//! PUT through the normal file-transfer path (`build_and_upload_stream` →
//! `upload_prepared_source`). Deliberate store-and-forward tradeoff: the
//! whole payload lands on local disk before upload starts (no incremental
//! progress, and needs disk headroom roughly equal to the payload size past
//! the spool threshold), in exchange for digest/retry/CSE/auto-compress/
//! multipart for free by reusing the file-path PUT pipeline — matching JDBC's
//! `FileBackedOutputStream` reference. Genuinely-streaming multipart upload
//! is a possible future follow-up, not implemented here. The caller shapes
//! the SQL (AUTO_COMPRESS, OVERWRITE, etc.); we only require it to start with
//! PUT.
//!
//! Chunked RPCs only round-trip to GS in `connection_upload_stream_finish`:
//! `begin` validates the SQL and opens a session, `chunk` appends to that
//! session's `SpooledBuffer` — bounding wrapper memory to ~one chunk.
//!
//! Session cleanup: `finish`/`abort` free the session and unlink any spooled
//! temp file. If neither is ever called (e.g. the wrapper process dies), the
//! session leaks until process exit — no per-connection reaping yet.
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
use super::query::{StageInfoRefreshContext, build_and_upload_stream, stream_stage_info_refresher};
use super::result_set::{ResultSetInfo, resolve_reader_ctx, response_to_descriptor};
use super::statement::{query_context, skip_leading_whitespace_and_comments};
use crate::config::rest_parameters::QueryParameters;
use crate::file_manager::{
    self, ByteSource, SPOOL_MEM_THRESHOLD, SingleDownloadData, SpooledBuffer, download_single_file,
};
use crate::handle_manager::Handle;
use crate::rest::snowflake::{
    QueryExecutionMode, QueryInput, RestError, query_response, snowflake_query_with_client,
};

/// Rejection message shared by `run_put_stream_via_gs` and
/// `connection_upload_stream_begin`'s PUT-SQL validation, so both stay in sync.
const UPLOAD_STREAM_REQUIRES_PUT_SQL: &str =
    "Upload stream requires a PUT SQL statement (SQL does not begin with PUT)";

/// A pending chunked upload: PUT SQL plus bytes received so far via
/// `ConnectionUploadStreamChunk`. Set up by `begin`, filled by `chunk`,
/// consumed by `finish` (or dropped by `abort`). See module docs for lifetime.
pub(super) struct UploadStreamSession {
    pub conn_handle: Handle,
    pub sql: String,
    pub buffer: Mutex<SpooledBuffer>,
    /// Mem-to-file spill threshold for `buffer`. Always `SPOOL_MEM_THRESHOLD`
    /// in production; tests inject a smaller value to exercise the mem→file
    /// flip without allocating that many bytes.
    pub(crate) spill_threshold: usize,
}

impl DatabaseDriverV1 {
    /// Executes a PUT SQL with already-drained `data` as the upload source.
    /// Kept for callers not yet on the chunked RPCs (JDBC still uses this).
    /// Delegates to [`Self::run_put_stream_via_gs`].
    pub async fn connection_upload_stream(
        &self,
        conn_handle: Handle,
        sql: String,
        data: Vec<u8>,
    ) -> Result<ResultSetInfo, ApiError> {
        self.run_put_stream_via_gs(conn_handle, sql, ByteSource::Bytes(data.into()))
            .await
    }

    /// Begins a chunked upload: validates `sql` is a PUT statement and opens
    /// a session that `connection_upload_stream_chunk` appends to. Does not
    /// round-trip to GS — that happens once, in
    /// `connection_upload_stream_finish`, after all bytes are in.
    pub async fn connection_upload_stream_begin(
        &self,
        conn_handle: Handle,
        sql: String,
    ) -> Result<Handle, ApiError> {
        self.connections
            .get_obj(conn_handle)
            .context(InvalidArgumentSnafu {
                argument: "Connection handle not found",
            })?;

        if !is_put_sql(&sql) {
            return InvalidArgumentSnafu {
                argument: UPLOAD_STREAM_REQUIRES_PUT_SQL,
            }
            .fail();
        }

        let session = UploadStreamSession {
            conn_handle,
            sql,
            buffer: Mutex::new(SpooledBuffer::default()),
            spill_threshold: SPOOL_MEM_THRESHOLD,
        };
        Ok(self.upload_streams.add_handle(session))
    }

    /// Appends one chunk to a pending upload started by
    /// `connection_upload_stream_begin`.
    pub async fn connection_upload_stream_chunk(
        &self,
        upload_handle: Handle,
        data: Vec<u8>,
    ) -> Result<(), ApiError> {
        let session = self
            .upload_streams
            .get_obj(upload_handle)
            .context(InvalidArgumentSnafu {
                argument: "Upload stream handle not found",
            })?;

        let mut buffer = session.buffer.lock().await;
        // Spilling to disk does sync file I/O; `block_in_place` keeps that
        // off the async executor thread. Safe: the core always runs on a
        // multi-thread tokio runtime (see `CApiState::runtime`).
        tokio::task::block_in_place(|| {
            buffer.write_all_with_threshold(&data, session.spill_threshold)
        })
        .context(SpoolBufferWriteSnafu)
    }

    /// Finishes a chunked upload: deregisters the session, reassembles its
    /// chunks into a `ByteSource`, and runs the normal upload path.
    ///
    /// `delete_handle` is first-caller-wins: if two callers race, only one
    /// gets `true`. The loser must bail with "handle not found" (like
    /// `abort`'s loser) instead of draining an already-taken, now-empty buffer.
    pub async fn connection_upload_stream_finish(
        &self,
        upload_handle: Handle,
    ) -> Result<ResultSetInfo, ApiError> {
        let session = self
            .upload_streams
            .get_obj(upload_handle)
            .context(InvalidArgumentSnafu {
                argument: "Upload stream handle not found",
            })?;
        if !self.upload_streams.delete_handle(upload_handle) {
            return InvalidArgumentSnafu {
                argument: "Upload stream handle not found",
            }
            .fail();
        }

        let buffer = {
            let mut guard = session.buffer.lock().await;
            std::mem::take(&mut *guard)
        };
        let (source, temp_path_guard) = buffer.into_source();

        // Must outlive the upload below: it unlinks the spooled temp file on
        // drop, and the upload streams the PUT body from its path.
        let result = self
            .run_put_stream_via_gs(session.conn_handle, session.sql.clone(), source)
            .await;
        drop(temp_path_guard);
        result
    }

    /// Aborts a pending chunked upload: deregisters the session without
    /// uploading. Any spooled temp file is unlinked when the buffer drops.
    pub async fn connection_upload_stream_abort(
        &self,
        upload_handle: Handle,
    ) -> Result<(), ApiError> {
        if self.upload_streams.delete_handle(upload_handle) {
            Ok(())
        } else {
            InvalidArgumentSnafu {
                argument: "Upload stream handle not found",
            }
            .fail()
        }
    }

    /// Shared core: runs `sql` (a PUT) through GS for stage credentials +
    /// encryption material, then uploads `source` via the normal
    /// file-transfer path. Returns a `ResultSetInfo` shaped like a normal
    /// PUT's. Used by both `connection_upload_stream` and
    /// `connection_upload_stream_finish`.
    pub(super) async fn run_put_stream_via_gs(
        &self,
        conn_handle: Handle,
        sql: String,
        source: ByteSource,
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
                    argument: UPLOAD_STREAM_REQUIRES_PUT_SQL,
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

            let rowset_data = build_and_upload_stream(
                &gs_data,
                &self.wrapper_presets,
                Some(refresh_ctx),
                use_s3_regional_url,
                &put_get_policy,
                source,
            )
            .await
            .context(QueryResponseProcessSnafu)?;

            let descriptor = response_to_descriptor(&gs_data, &self.wrapper_presets);
            let reader_ctx = resolve_reader_ctx(&conn_ptr).await?;
            let handle = self.create_result_set(descriptor.clone(), rowset_data, reader_ctx);
            Ok(ResultSetInfo { handle, descriptor })
        }
        .instrument(crate::snowflake_op_span!(
            "run_put_stream_via_gs",
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
            download_single_file(single_download, &put_get_policy, 0, &mut refresher_dyn)
                .await
                .map_err(|e| {
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
            None,
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

    /// Registers a bare, never-connected `Connection` and returns its handle —
    /// enough for handle-lookup + SQL-shape validation, which run before any I/O.
    fn register_bare_connection(driver: &DatabaseDriverV1) -> Handle {
        driver.connections.add_handle(Mutex::new(Connection::new()))
    }

    /// A `Handle` never registered with any `HandleManager` — exercises the
    /// "handle not found" branches.
    fn bogus_handle() -> Handle {
        Handle {
            id: 999_999,
            magic: 42,
        }
    }

    #[tokio::test]
    async fn upload_stream_begin_rejects_unknown_connection() {
        let driver = DatabaseDriverV1::new();
        let result = driver
            .connection_upload_stream_begin(bogus_handle(), "PUT file://x @s".into())
            .await;
        assert!(
            result.is_err(),
            "unknown connection handle must be rejected"
        );
    }

    #[tokio::test]
    async fn upload_stream_begin_rejects_non_put_sql() {
        let driver = DatabaseDriverV1::new();
        let conn_handle = register_bare_connection(&driver);
        let result = driver
            .connection_upload_stream_begin(conn_handle, "SELECT 1".into())
            .await;
        assert!(result.is_err(), "non-PUT SQL must be rejected");
    }

    #[tokio::test]
    async fn upload_stream_begin_registers_a_session_for_valid_put_sql() {
        let driver = DatabaseDriverV1::new();
        let conn_handle = register_bare_connection(&driver);
        let upload_handle = driver
            .connection_upload_stream_begin(conn_handle, "PUT file://x @s".into())
            .await
            .expect("valid PUT SQL against a known connection must succeed");

        let session = driver
            .upload_streams
            .get_obj(upload_handle)
            .expect("session must be registered under the returned handle");
        assert_eq!(session.sql, "PUT file://x @s");
        assert_eq!(session.conn_handle, conn_handle);
    }

    #[tokio::test]
    async fn upload_stream_chunk_rejects_unknown_handle() {
        let driver = DatabaseDriverV1::new();
        let result = driver
            .connection_upload_stream_chunk(bogus_handle(), b"data".to_vec())
            .await;
        assert!(result.is_err(), "unknown upload handle must be rejected");
    }

    // `block_in_place` panics on a current-thread runtime; match production.
    #[tokio::test(flavor = "multi_thread")]
    async fn upload_stream_chunk_appends_to_the_session_buffer() {
        let driver = DatabaseDriverV1::new();
        let conn_handle = register_bare_connection(&driver);
        let upload_handle = driver
            .connection_upload_stream_begin(conn_handle, "PUT file://x @s".into())
            .await
            .unwrap();

        driver
            .connection_upload_stream_chunk(upload_handle, b"hello ".to_vec())
            .await
            .unwrap();
        driver
            .connection_upload_stream_chunk(upload_handle, b"world".to_vec())
            .await
            .unwrap();

        let session = driver.upload_streams.get_obj(upload_handle).unwrap();
        let buffer = session.buffer.lock().await;
        assert!(matches!(&*buffer, SpooledBuffer::Mem(b) if b == b"hello world"));
    }

    // Same reason as above: `block_in_place` needs a multi-thread runtime.
    #[tokio::test(flavor = "multi_thread")]
    async fn upload_stream_chunk_spills_to_disk_past_the_injected_threshold() {
        // Registering the session directly with a small `spill_threshold`
        // exercises the mem-to-file flip without allocating 128 MiB.
        let driver = DatabaseDriverV1::new();
        let conn_handle = register_bare_connection(&driver);
        let upload_handle = driver.upload_streams.add_handle(UploadStreamSession {
            conn_handle,
            sql: "PUT file://x @s".into(),
            buffer: Mutex::new(SpooledBuffer::default()),
            spill_threshold: 16,
        });

        // Varying chunk sizes, crossing the 16-byte threshold partway through.
        let chunks: Vec<Vec<u8>> = (0u8..10).map(|i| vec![i; (i as usize % 5) + 1]).collect();
        let expected: Vec<u8> = chunks.iter().flatten().copied().collect();
        for chunk in &chunks {
            driver
                .connection_upload_stream_chunk(upload_handle, chunk.clone())
                .await
                .unwrap();
        }

        let session = driver.upload_streams.get_obj(upload_handle).unwrap();
        let buffer = {
            let mut guard = session.buffer.lock().await;
            std::mem::take(&mut *guard)
        };
        assert!(
            matches!(buffer, SpooledBuffer::File(_)),
            "expected the session buffer to have spilled to disk given the small threshold"
        );

        let (source, _temp_path) = buffer.into_source();
        match source {
            ByteSource::Path(path) => {
                let contents = std::fs::read(&path).unwrap();
                assert_eq!(contents, expected);
            }
            ByteSource::Bytes(_) => panic!("expected ByteSource::Path after spilling to disk"),
        }
    }

    #[tokio::test]
    async fn upload_stream_abort_rejects_unknown_handle() {
        let driver = DatabaseDriverV1::new();
        let result = driver.connection_upload_stream_abort(bogus_handle()).await;
        assert!(result.is_err(), "unknown upload handle must be rejected");
    }

    #[tokio::test]
    async fn upload_stream_abort_deregisters_the_session() {
        let driver = DatabaseDriverV1::new();
        let conn_handle = register_bare_connection(&driver);
        let upload_handle = driver
            .connection_upload_stream_begin(conn_handle, "PUT file://x @s".into())
            .await
            .unwrap();

        driver
            .connection_upload_stream_abort(upload_handle)
            .await
            .unwrap();

        assert!(
            driver.upload_streams.get_obj(upload_handle).is_none(),
            "session must be deregistered after abort"
        );
        // Aborting twice must fail — the session is already gone.
        assert!(
            driver
                .connection_upload_stream_abort(upload_handle)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn upload_stream_finish_rejects_unknown_handle() {
        let driver = DatabaseDriverV1::new();
        let result = driver.connection_upload_stream_finish(bogus_handle()).await;
        assert!(result.is_err(), "unknown upload handle must be rejected");
    }

    // Calls `connection_upload_stream_chunk` below — requires a multi-thread
    // runtime (see `block_in_place` above).
    #[tokio::test(flavor = "multi_thread")]
    async fn upload_stream_finish_deregisters_the_session_even_on_failure() {
        // A bare `Connection::new()` has no transport, so the upload is
        // guaranteed to fail — this only checks that `finish` still
        // deregisters the handle regardless.
        let driver = DatabaseDriverV1::new();
        let conn_handle = register_bare_connection(&driver);
        let upload_handle = driver
            .connection_upload_stream_begin(conn_handle, "PUT file://x @s".into())
            .await
            .unwrap();
        driver
            .connection_upload_stream_chunk(upload_handle, b"payload".to_vec())
            .await
            .unwrap();

        let result = driver.connection_upload_stream_finish(upload_handle).await;
        assert!(
            result.is_err(),
            "a never-connected Connection cannot complete the GS round trip"
        );
        assert!(
            driver.upload_streams.get_obj(upload_handle).is_none(),
            "session must be deregistered by finish even when the upload itself fails"
        );
    }

    // Sequential version of the race test below. Can't distinguish fixed vs.
    // buggy code by itself (get_obj already returns None either way by the
    // second call) — it just pins the exact "handle not found" message.
    #[tokio::test(flavor = "multi_thread")]
    async fn upload_stream_finish_rejects_a_second_finish_on_the_same_handle() {
        let driver = DatabaseDriverV1::new();
        let conn_handle = register_bare_connection(&driver);
        let upload_handle = driver
            .connection_upload_stream_begin(conn_handle, "PUT file://x @s".into())
            .await
            .unwrap();
        driver
            .connection_upload_stream_chunk(upload_handle, b"payload".to_vec())
            .await
            .unwrap();

        // Fails for lack of transport, but still deregisters the session.
        let _ = driver.connection_upload_stream_finish(upload_handle).await;

        let second = driver.connection_upload_stream_finish(upload_handle).await;
        // `ResultSetInfo` (the `Ok` type) isn't `Debug`, so match rather than
        // `expect_err`.
        let err = match second {
            Ok(_) => panic!("a second finish on an already-finished handle must fail"),
            Err(e) => e,
        };
        assert!(
            err.to_string().contains("Upload stream handle not found"),
            "expected the same \"handle not found\" rejection abort gives its \
             loser, got: {err}"
        );
    }

    // Regression test: concurrent `finish` calls on the same handle must not
    // all fall through to drain-and-upload. `delete_handle` is
    // first-caller-wins, so at most one racer can see `true`; losers must
    // bail with "handle not found" instead of draining an already-taken buffer.
    //
    // Many racers + a `Barrier` best-effort widen the race window on a real
    // multi-thread runtime, without adding a synchronization seam to
    // production code (rule 1, code-review-design-discipline.md). The
    // invariant asserted — at most one racer proceeds past delete_handle —
    // holds regardless of whether the window is actually hit on a given run.
    #[tokio::test(flavor = "multi_thread")]
    async fn upload_stream_finish_race_only_one_caller_proceeds_past_delete() {
        const RACERS: usize = 16;

        let driver = Arc::new(DatabaseDriverV1::new());
        let conn_handle = register_bare_connection(&driver);
        let upload_handle = driver
            .connection_upload_stream_begin(conn_handle, "PUT file://x @s".into())
            .await
            .unwrap();
        driver
            .connection_upload_stream_chunk(upload_handle, b"payload".to_vec())
            .await
            .unwrap();

        let barrier = Arc::new(tokio::sync::Barrier::new(RACERS));
        let mut tasks = Vec::with_capacity(RACERS);
        for _ in 0..RACERS {
            let driver = Arc::clone(&driver);
            let barrier = Arc::clone(&barrier);
            tasks.push(tokio::spawn(async move {
                barrier.wait().await;
                driver.connection_upload_stream_finish(upload_handle).await
            }));
        }

        let mut errors = Vec::with_capacity(RACERS);
        for task in tasks {
            // `ResultSetInfo` (the `Ok` type) isn't `Debug`, so match rather
            // than `expect_err`.
            match task.await.expect("racer task must not panic") {
                Ok(_) => panic!("a never-connected Connection cannot complete the GS round trip"),
                Err(e) => errors.push(e.to_string()),
            }
        }

        let proceeded = errors
            .iter()
            .filter(|e| !e.contains("Upload stream handle not found"))
            .count();
        assert_eq!(
            proceeded, 1,
            "exactly one of {RACERS} racing finishes may proceed past the \
             delete_handle check to attempt the upload (the rest must be \
             rejected as \"handle not found\"); got {proceeded} that \
             proceeded: {errors:?}"
        );
    }
}
