//! Connection-level streaming file transfer handlers.
//!
//! Backs the chunked `ConnectionUploadStream{Begin,Chunk,Finish,Abort}` and
//! `ConnectionDownloadStream{Begin,Chunk,Close}` RPCs (JDBC `uploadStream` /
//! `downloadStream`, Python `file_stream`).
//!
//! Upload contract: the caller's bytes arrive via `ConnectionUploadStreamChunk`
//! and the core reassembles them into a re-readable [`file_manager::ByteSource`]
//! — in memory, or spooled to a temp file past [`file_manager::SpooledBuffer`]'s
//! threshold — then runs the PUT through the normal file-transfer path
//! (`build_and_upload_stream` → `upload_prepared_source`). Deliberate
//! store-and-forward tradeoff: the whole payload lands on local disk before
//! upload starts (no incremental progress, and needs disk headroom roughly
//! equal to the payload size past the spool threshold), in exchange for
//! digest/retry/CSE/auto-compress/multipart for free by reusing the file-path
//! PUT pipeline — matching JDBC's `FileBackedOutputStream` reference.
//! Genuinely-streaming multipart upload is a possible future follow-up, not
//! implemented here. The caller shapes the SQL (AUTO_COMPRESS, OVERWRITE,
//! etc.); we only require it to start with PUT.
//!
//! Chunked RPCs only round-trip to GS in `connection_upload_stream_finish`:
//! `begin` validates the SQL and opens a session, `chunk` appends to that
//! session's `SpooledBuffer` — bounding wrapper memory to ~one chunk.
//!
//! Session lifetime & cleanup: an `UploadStreamSession` is freed by
//! `connection_upload_stream_finish`/`_abort`; a download session by
//! `download_stream_close`. `reap_connection_streams` also frees both kinds
//! on `connection_close`, dropping an upload's temp file and aborting a
//! download's tasks. Only the graceful-close path is covered — a session on
//! a connection that's never closed leaks until process exit; see
//! `TODO(SNOW-3704961)` in `connection::cleanup_connection`.
//!
//! Download contract (chunked, `download_stream_{begin,chunk,close}`):
//! `download_stream_begin` opens a zero-disk streaming GET against cloud
//! storage (S3/GCS/Azure, dispatched on the stage's `location_type` via
//! `open_download_stream_for_stage`); `download_stream_chunk` drains its
//! plaintext channel on demand, capped at `DOWNLOAD_STREAM_MAX_CHUNK_LEN` per
//! call so only a few chunks are ever buffered.
//!
//! Integrity tradeoff vs. whole-file: plaintext reaches the channel as each
//! chunk decrypts, before the end-of-stream digest check
//! (`decrypt_ciphertext_to_writer`) confirms nothing was tampered with. A
//! caller consuming chunks as they arrive may already have used bad bytes by
//! the time a mismatch fails the final `download_stream_chunk` call. Callers
//! needing "no output on tamper" should use the whole-file download instead.
//! A mid-body transport failure is likewise terminal — no retry, no
//! Range-resume, same tradeoff across S3/GCS/Azure.
//!
//! Both handlers reuse the connection-context + GS-execute helpers from
//! `statement.rs` so the retry/refresh plumbing lives in one place.

use std::sync::Arc;

use bytes::Bytes;
use snafu::{OptionExt, ResultExt};
use tokio::sync::Mutex;
use tracing::Instrument;

use super::connection::{Connection, FinalSessionNames, RefreshContext};
use super::error::*;
use super::global_state::{DatabaseDriverV1, PutGetResultsetFlavor};
use super::query::{StageInfoRefreshContext, build_and_upload_stream, stream_stage_info_refresher};
use super::result_set::{ResultSetInfo, resolve_reader_ctx, response_to_descriptor};
use super::statement::{query_context, skip_leading_whitespace_and_comments};
use crate::config::rest_parameters::QueryParameters;
use crate::file_manager::{self, ByteSource, SPOOL_MEM_THRESHOLD, SpooledBuffer};
use crate::handle_manager::Handle;
use crate::rest::snowflake::{
    QueryExecutionMode, QueryInput, RestError, query_response, snowflake_query_with_client,
};

/// Rejection message shared by `run_put_stream_via_gs` and
/// `connection_upload_stream_begin`'s PUT-SQL validation, so both stay in sync.
const UPLOAD_STREAM_REQUIRES_PUT_SQL: &str =
    "Upload stream requires a PUT SQL statement (SQL does not begin with PUT)";

/// Hard cap on bytes returned per `download_stream_chunk` call, regardless of
/// the caller's `max_len` — an oversized `max_len` can't defeat the
/// bounded-memory guarantee. See the proto doc on
/// `ConnectionDownloadStreamChunkRequest`.
const DOWNLOAD_STREAM_MAX_CHUNK_LEN: usize = 8 * 1024 * 1024;

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

/// Mutable state for a chunked download: the producer's chunk channel plus a
/// leftover buffer so a `max_len` split doesn't drop bytes. Wrapped in a
/// `Mutex` by [`DownloadStream`] since every field changes per call.
pub(super) struct DownloadStreamSession {
    /// Chunks from the background download task, in order. A closed channel
    /// means clean EOF; a terminal `Err` is the producer's last item.
    rx: tokio::sync::mpsc::Receiver<Result<Vec<u8>, file_manager::FileManagerError>>,
    /// Bytes pulled off `rx` but not yet returned — left over when a
    /// previous call's `max_len` split a chunk.
    leftover: Bytes,
    /// Set on clean EOF from `rx`. `eof` is only reported once this is set
    /// and `leftover` is empty.
    done: bool,
}

/// Handles for the two background tasks behind a chunked download: the cloud
/// producer and the decrypt/gunzip pipeline. Always aborted together via
/// [`Self::abort`] — aborting only the pipeline could leave the producer
/// stuck forever on a stalled cloud read.
pub(super) struct DownloadAborter {
    producer: tokio::task::AbortHandle,
    pipeline: tokio::task::AbortHandle,
}

impl DownloadAborter {
    fn abort(&self) {
        self.producer.abort();
        self.pipeline.abort();
    }
}

/// A pending chunked download: registered by `download_stream_begin`,
/// drained by `download_stream_chunk`, torn down by `download_stream_close`.
/// `aborter` lives outside `session`'s `Mutex` so `close` (or the reaper) can
/// abort a stalled download without waiting on a lock `download_stream_chunk`
/// might be holding.
pub(super) struct DownloadStream {
    pub(super) conn_handle: Handle,
    pub(super) aborter: DownloadAborter,
    session: Mutex<DownloadStreamSession>,
}

impl DatabaseDriverV1 {
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
    /// PUT's. Used by
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

    /// Begins a chunked, zero-disk download: resolves `stage_name` +
    /// `source_filename` against GS, but opens a streaming GET against cloud
    /// storage instead of writing to a tempdir, and registers a session for
    /// `download_stream_chunk` to drain. Dispatches on the stage's
    /// `location_type` (S3/GCS/Azure) via `file_manager::open_download_stream_for_stage`.
    ///
    /// Returns the new handle plus the on-cloud byte count
    /// (pre-decompression), if the cloud response reported one.
    pub async fn download_stream_begin(
        &self,
        conn_handle: Handle,
        stage_name: String,
        source_filename: String,
        decompress: bool,
    ) -> Result<(Handle, Option<i64>), ApiError> {
        let session_id = self.session_id_for_conn(conn_handle).await;
        async {
            let conn_ptr = self
                .connections
                .get_obj(conn_handle)
                .context(InvalidArgumentSnafu {
                    argument: "Connection handle not found",
                })?;

            let stage_path = build_stage_path(&stage_name, &source_filename);
            // GET requires a syntactically valid local target even though
            // the streaming path never writes to it — created and discarded
            // immediately.
            let tmp_dir = tempfile::tempdir().map_err(|e| {
                InvalidArgumentSnafu {
                    argument: format!("Failed to create temp directory: {e}"),
                }
                .build()
            })?;
            // GET doesn't support parameterized bindings for stage paths or
            // local locations. stage_name/source_filename are caller-supplied
            // (same as the file-path GET); the local dir comes from
            // tempfile::tempdir(), not the caller.
            let get_sql = build_get_sql(&stage_path, tmp_dir.path());
            drop(tmp_dir);

            let (query_parameters, http_client, retry_policy) = query_context(&conn_ptr).await?;

            let response = run_sql_against_gs(
                &conn_ptr,
                &http_client,
                &query_parameters,
                &retry_policy,
                get_sql.clone(),
            )
            .await?;

            let (use_s3_regional_url, unsafe_file_write) = {
                let conn = conn_ptr.lock().await;
                let unsafe_file_write = conn.unsafe_file_write();
                let use_s3_regional_url = conn.use_s3_regional_url_session_param().await;
                (use_s3_regional_url, unsafe_file_write)
            };

            let resolved = resolve_download_target(
                response,
                self.wrapper_presets.put_get_resultset_flavor.clone(),
                use_s3_regional_url,
                unsafe_file_write,
                &source_filename,
            )?;

            let refresh_ctx = StageInfoRefreshContext {
                sql: get_sql,
                query_parameters,
                conn: conn_ptr.clone(),
            };
            let mut refresher = stream_stage_info_refresher(refresh_ctx, resolved.initial_snapshot);

            let put_get_policy = {
                let conn = conn_ptr.lock().await;
                crate::config::retry::RetryPolicy::put_get(&conn.connection_seed)
            };

            // `refresher` only needs to cover opening the stream — it's
            // dropped when this block returns, before the background
            // producer (which has no refresher of its own) is spawned.
            let mut refresher_dyn: Option<&mut dyn file_manager::StageInfoRefresher> =
                Some(&mut refresher);
            let opened = file_manager::open_download_stream_for_stage(
                &resolved.stage_info,
                &resolved.src_location,
                resolved.presigned_url.as_deref(),
                &put_get_policy,
                &mut refresher_dyn,
                resolved.encryption_material,
                decompress,
            )
            .await
            .map_err(|e| {
                InvalidArgumentSnafu {
                    argument: format!("Failed to open download stream: {e}"),
                }
                .build()
            })?;

            let total_len = (opened.cloud_byte_count > 0).then_some(opened.cloud_byte_count);
            let stream = DownloadStream {
                conn_handle,
                aborter: DownloadAborter {
                    producer: opened.producer_abort,
                    pipeline: opened.task.abort_handle(),
                },
                session: Mutex::new(DownloadStreamSession {
                    rx: opened.chunks,
                    leftover: Bytes::new(),
                    done: false,
                }),
            };
            // `opened.task` runs detached: the pipeline drives itself via
            // `rx`/`ChannelWriter`, and is only cancelled through
            // `stream.aborter`'s `AbortHandle` (not this `JoinHandle`).
            let handle = self.download_streams.add_handle(stream);
            Ok((handle, total_len))
        }
        .instrument(crate::snowflake_op_span!(
            "download_stream_begin",
            session_id
        ))
        .await
    }

    /// Pulls up to `max_len` bytes from a session opened by
    /// `download_stream_begin`: drains `leftover` first, then pulls fresh
    /// chunks off the channel. `eof` is `true` only once the producer is
    /// done and every byte has been returned. `max_len <= 0` is a no-op
    /// peek — nothing is consumed. Always clamped to
    /// [`DOWNLOAD_STREAM_MAX_CHUNK_LEN`].
    pub async fn download_stream_chunk(
        &self,
        download_handle: Handle,
        max_len: i64,
    ) -> Result<(Vec<u8>, bool), ApiError> {
        async {
            let stream =
                self.download_streams
                    .get_obj(download_handle)
                    .context(InvalidArgumentSnafu {
                        argument: "Download stream handle not found",
                    })?;
            let mut session = stream.session.lock().await;

            let max_len = usize::try_from(max_len)
                .unwrap_or(0)
                .min(DOWNLOAD_STREAM_MAX_CHUNK_LEN);
            let mut out = Vec::with_capacity(max_len);
            loop {
                if !session.leftover.is_empty() {
                    let n = session
                        .leftover
                        .len()
                        .min(max_len.saturating_sub(out.len()));
                    out.extend_from_slice(&session.leftover[..n]);
                    session.leftover = session.leftover.slice(n..);
                }
                if out.len() >= max_len || session.done {
                    break;
                }
                match session.rx.recv().await {
                    Some(Ok(chunk)) => session.leftover = Bytes::from(chunk),
                    Some(Err(e)) => {
                        session.done = true;
                        return InvalidArgumentSnafu {
                            argument: format!("Download stream failed: {e}"),
                        }
                        .fail();
                    }
                    None => session.done = true,
                }
            }
            let eof = session.done && session.leftover.is_empty();
            Ok((out, eof))
        }
        .instrument(tracing::debug_span!(
            "download_stream_chunk",
            ?download_handle,
            max_len
        ))
        .await
    }

    /// Closes a chunked download: deregisters the session and aborts both
    /// background tasks. Safe to call after natural EOF (abort is a no-op)
    /// or while `download_stream_chunk` is blocked on the session lock —
    /// abort goes through `stream.aborter`, not that lock.
    pub async fn download_stream_close(&self, download_handle: Handle) -> Result<(), ApiError> {
        async {
            let stream =
                self.download_streams
                    .get_obj(download_handle)
                    .context(InvalidArgumentSnafu {
                        argument: "Download stream handle not found",
                    })?;
            // Deregister first so no new call can find this session, then
            // abort. `stream` is a local `Arc` clone, so the aborter survives
            // `delete_handle` regardless of order. `abort()` is infallible and
            // idempotent, so it's safe even if the task already finished or
            // another close beat us to it.
            self.download_streams.delete_handle(download_handle);
            stream.aborter.abort();
            Ok(())
        }
        .instrument(tracing::debug_span!(
            "download_stream_close",
            ?download_handle
        ))
        .await
    }

    /// Reaps every pending upload/download stream for `conn`, called from
    /// `connection_close`. An `UploadStreamSession` is just dropped
    /// (unlinking its temp file, if any); a `DownloadStream` is aborted
    /// first, so its background tasks don't keep running against
    /// soon-to-be-invalid credentials.
    ///
    /// Only runs on explicit `connection_close` — a stream that's never
    /// drained/closed otherwise leaks until process exit (see module docs).
    pub(super) fn reap_connection_streams(&self, conn: Handle) {
        for _upload in self
            .upload_streams
            .drain_matching(|s| s.conn_handle == conn)
        {
            // Dropped here: releases the buffer, unlinking its spooled temp
            // file if any.
        }
        for download in self
            .download_streams
            .drain_matching(|s| s.conn_handle == conn)
        {
            download.aborter.abort();
        }
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

/// Everything `download_stream_begin` needs to fetch a file, resolved once by
/// [`resolve_download_target`] instead of duplicating GS-response parsing.
#[derive(Debug)]
struct ResolvedDownload {
    src_location: String,
    stage_info: file_manager::StageInfo,
    encryption_material: Option<file_manager::EncryptionMaterial>,
    presigned_url: Option<String>,
    initial_snapshot: file_manager::StageInfoSnapshot,
}

/// Parses a GET's GS `response` into a [`ResolvedDownload`]: rejects a
/// server-side failure, then picks out the source location and stage
/// credentials (always a single file, per the current one-file-per-GET
/// design). Used by `download_stream_begin`; callers fetch
/// `use_s3_regional_url`/`unsafe_file_write` themselves before calling in.
fn resolve_download_target(
    response: query_response::Response,
    flavor: PutGetResultsetFlavor,
    use_s3_regional_url: bool,
    unsafe_file_write: bool,
    source_filename: &str,
) -> Result<ResolvedDownload, ApiError> {
    if !response.success {
        return InvalidArgumentSnafu {
            argument: response
                .message
                .unwrap_or_else(|| "GET command rejected by server".to_string()),
        }
        .fail();
    }

    let gs_data = response.data;
    let download_data = gs_data
        // `get_fastfail` is inert here: this single-file stream path only
        // projects individual fields out of `download_data` into
        // `ResolvedDownload` and never runs the `download_files` batch loop.
        .to_file_download_data(&flavor, use_s3_regional_url, unsafe_file_write, false)
        .map_err(|e| {
            InvalidArgumentSnafu {
                argument: format!("Failed to parse GET response: {e}"),
            }
            .build()
        })?;

    let initial_snapshot = gs_data
        .stage_info_snapshot()
        .map_err(|e| {
            InvalidArgumentSnafu {
                argument: format!("Failed to extract stage info from GET response: {e}"),
            }
            .build()
        })?
        .context(InvalidArgumentSnafu {
            argument: "GET response missing stage credentials",
        })?;

    let src_location =
        download_data
            .src_locations
            .into_iter()
            .next()
            .context(InvalidArgumentSnafu {
                argument: format!("File not found on stage: {source_filename}"),
            })?;

    Ok(ResolvedDownload {
        src_location,
        stage_info: download_data.stage_info,
        encryption_material: download_data
            .encryption_materials
            .into_iter()
            .next()
            .flatten(),
        presigned_url: download_data.presigned_urls.into_iter().next().flatten(),
        initial_snapshot,
    })
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

/// Builds the `GET <stage_path> '<file://...>'` SQL that downloads a stage
/// file into `local_dir`.
///
/// The local path is single-quoted so special characters don't break GET
/// parsing — most importantly the tilde in a Windows 8.3 short path
/// (`C:\Users\RUNNER~1\...`), which Snowflake otherwise rejects with
/// `unexpected '~'`, and spaces. This matches the legacy Python connector and
/// the GET docs, which require quoting local paths with special characters and
/// forward slashes on Windows. Backslashes are normalized to `/` and any
/// embedded single quote is escaped as `\'`. `stage_path` is left unquoted,
/// matching the existing file-path GET convention.
fn build_get_sql(stage_path: &str, local_dir: &std::path::Path) -> String {
    let local_path = local_dir
        .to_str()
        .unwrap_or("/tmp")
        .replace('\\', "/")
        .replace('\'', "\\'");
    format!("GET {stage_path} 'file://{local_path}'")
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
    fn build_get_sql_quotes_local_path() {
        // A Windows 8.3 short path embeds a `~`; the space is another special
        // char. Both must sit inside single quotes so GET doesn't choke on
        // them, and backslashes must be normalized to `/`.
        let sql = build_get_sql(
            "@my_stage/data.csv",
            std::path::Path::new("C:\\Users\\RUNNER~1\\App Data\\tmp"),
        );
        assert_eq!(
            sql,
            "GET @my_stage/data.csv 'file://C:/Users/RUNNER~1/App Data/tmp'"
        );
    }

    #[test]
    fn build_get_sql_escapes_embedded_single_quote() {
        // A single quote in the local path would otherwise terminate the
        // quoted literal early; it must be escaped as \'.
        let sql = build_get_sql("@my_stage/f.csv", std::path::Path::new("/tmp/o'brien"));
        assert_eq!(sql, "GET @my_stage/f.csv 'file:///tmp/o\\'brien'");
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
    /// Feeds `items` into a fresh mpsc channel one at a time, standing in for
    /// the real cloud producer/pipeline task, so the bookkeeping tests below
    /// (leftover splitting, eof detection, error propagation) run without a
    /// live GS + cloud round trip.
    fn spawn_fake_download_producer(
        items: Vec<Result<Vec<u8>, file_manager::FileManagerError>>,
    ) -> (
        tokio::sync::mpsc::Receiver<Result<Vec<u8>, file_manager::FileManagerError>>,
        tokio::task::JoinHandle<()>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::channel(8);
        let task = tokio::spawn(async move {
            for item in items {
                if tx.send(item).await.is_err() {
                    break;
                }
            }
        });
        (rx, task)
    }

    /// Registers a `DownloadStream` backed by `spawn_fake_download_producer`,
    /// mirroring what `download_stream_begin` does after a successful
    /// `open_s3_download_stream`. `conn_handle` need not be registered with
    /// `driver.connections` — most callers here don't look it up. Unlike
    /// production's two tasks, one fake task's abort handle backs both
    /// `DownloadAborter` slots, which is harmless since aborting an
    /// already-finished task is a no-op.
    fn register_download_session(
        driver: &DatabaseDriverV1,
        conn_handle: Handle,
        items: Vec<Result<Vec<u8>, file_manager::FileManagerError>>,
    ) -> Handle {
        let (rx, task) = spawn_fake_download_producer(items);
        let abort_handle = task.abort_handle();
        let stream = DownloadStream {
            conn_handle,
            aborter: DownloadAborter {
                producer: abort_handle.clone(),
                pipeline: abort_handle,
            },
            session: Mutex::new(DownloadStreamSession {
                rx,
                leftover: Bytes::new(),
                done: false,
            }),
        };
        driver.download_streams.add_handle(stream)
    }

    #[tokio::test]
    async fn download_stream_begin_rejects_unknown_connection() {
        let driver = DatabaseDriverV1::new();
        let result = driver
            .download_stream_begin(bogus_handle(), "@my_stage".into(), "data.csv".into(), false)
            .await;
        assert!(
            result.is_err(),
            "unknown connection handle must be rejected before any GS round trip"
        );
    }

    #[tokio::test]
    async fn download_stream_chunk_rejects_unknown_handle() {
        let driver = DatabaseDriverV1::new();
        let result = driver.download_stream_chunk(bogus_handle(), 16).await;
        assert!(result.is_err(), "unknown download handle must be rejected");
    }

    #[tokio::test]
    async fn download_stream_close_rejects_unknown_handle() {
        let driver = DatabaseDriverV1::new();
        let result = driver.download_stream_close(bogus_handle()).await;
        assert!(result.is_err(), "unknown download handle must be rejected");
    }

    #[tokio::test]
    async fn download_stream_chunk_splits_a_single_chunk_across_calls() {
        let driver = DatabaseDriverV1::new();
        let handle =
            register_download_session(&driver, bogus_handle(), vec![Ok(b"hello world".to_vec())]);

        // First call asks for fewer bytes than the chunk holds — the
        // remainder must be buffered in `leftover`, not dropped, and `eof`
        // must not fire while bytes remain.
        let (first, eof) = driver.download_stream_chunk(handle, 5).await.unwrap();
        assert_eq!(first, b"hello");
        assert!(!eof, "bytes remain in leftover; eof must not fire yet");

        // Second call drains the leftover plus observes the channel close.
        let (second, eof) = driver.download_stream_chunk(handle, 100).await.unwrap();
        assert_eq!(second, b" world");
        assert!(
            eof,
            "channel closed with no leftover remaining; eof must fire"
        );
    }

    #[tokio::test]
    async fn download_stream_chunk_reassembles_multiple_producer_chunks() {
        let driver = DatabaseDriverV1::new();
        let handle = register_download_session(
            &driver,
            bogus_handle(),
            vec![
                Ok(b"foo".to_vec()),
                Ok(b"bar".to_vec()),
                Ok(b"baz".to_vec()),
            ],
        );

        // A single call with a generous max_len must pull chunks off the
        // channel until it hits eof, not stop at the first one.
        let (all, eof) = driver.download_stream_chunk(handle, 100).await.unwrap();
        assert_eq!(all, b"foobarbaz");
        assert!(eof);
    }

    #[tokio::test]
    async fn download_stream_chunk_non_positive_max_len_is_a_no_op_peek() {
        let driver = DatabaseDriverV1::new();
        let handle =
            register_download_session(&driver, bogus_handle(), vec![Ok(b"hello".to_vec())]);

        let (out, eof) = driver.download_stream_chunk(handle, 0).await.unwrap();
        assert!(out.is_empty(), "max_len <= 0 must consume no bytes");
        assert!(!eof, "nothing has been pulled off the channel yet");
    }

    #[tokio::test]
    async fn download_stream_chunk_propagates_a_terminal_producer_error() {
        let driver = DatabaseDriverV1::new();
        let io_err = file_manager::FileManagerError::Io {
            source: std::io::Error::other("simulated producer failure"),
            location: snafu::Location::default(),
        };
        let handle = register_download_session(
            &driver,
            bogus_handle(),
            vec![Ok(b"partial".to_vec()), Err(io_err)],
        );

        // The good chunk ahead of the error is delivered first...
        let (first, eof) = driver.download_stream_chunk(handle, 7).await.unwrap();
        assert_eq!(first, b"partial");
        assert!(!eof);

        // ...then the terminal error surfaces on the next call, and the
        // session is marked done so a caller that retries doesn't hang.
        let result = driver.download_stream_chunk(handle, 100).await;
        assert!(
            result.is_err(),
            "a terminal producer error must surface as an Err, not a truncated Ok"
        );
    }

    #[tokio::test]
    async fn download_stream_close_deregisters_and_aborts_the_task() {
        let driver = DatabaseDriverV1::new();
        // Both the producer and the pipeline are made to never finish on
        // their own — `close` must abort both rather than waiting either
        // out, since aborting only one would leave the other parked forever
        // (see `DownloadAborter`'s doc comment).
        let (_tx_keep_alive, rx) =
            tokio::sync::mpsc::channel::<Result<Vec<u8>, file_manager::FileManagerError>>(8);
        let producer_task = tokio::spawn(async {
            std::future::pending::<()>().await;
        });
        let pipeline_task = tokio::spawn(async {
            std::future::pending::<()>().await;
        });
        let producer_abort = producer_task.abort_handle();
        let pipeline_abort = pipeline_task.abort_handle();
        let stream = DownloadStream {
            conn_handle: bogus_handle(),
            aborter: DownloadAborter {
                producer: producer_abort.clone(),
                pipeline: pipeline_abort.clone(),
            },
            session: Mutex::new(DownloadStreamSession {
                rx,
                leftover: Bytes::new(),
                done: false,
            }),
        };
        let handle = driver.download_streams.add_handle(stream);

        driver.download_stream_close(handle).await.unwrap();

        assert!(
            driver.download_streams.get_obj(handle).is_none(),
            "session must be deregistered after close"
        );
        // `abort()` schedules cancellation; give the runtime a bounded number
        // of yields to actually observe it rather than asserting instantly.
        for _ in 0..1000 {
            if producer_abort.is_finished() && pipeline_abort.is_finished() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(
            producer_abort.is_finished(),
            "close must abort the still-pending producer task, not just deregister it"
        );
        assert!(
            pipeline_abort.is_finished(),
            "close must abort the still-pending pipeline task, not just deregister it"
        );
        // Closing twice must fail — the session is already gone.
        assert!(driver.download_stream_close(handle).await.is_err());
    }

    #[tokio::test]
    async fn download_stream_chunk_reassembles_across_many_small_round_trips() {
        let driver = DatabaseDriverV1::new();
        let handle = register_download_session(
            &driver,
            bogus_handle(),
            vec![
                Ok(b"ab".to_vec()),
                Ok(b"cde".to_vec()),
                Ok(b"f".to_vec()),
                Ok(b"ghij".to_vec()),
            ],
        );

        // Pull "abcdefghij" back out via small, irregular max_len calls that
        // never line up with the producer's chunk boundaries above — unlike
        // the single-split test, this exercises `leftover` surviving more
        // than one carry-over within a single payload.
        let mut collected = Vec::new();
        let mut eof = false;
        for max_len in [1, 3, 2, 100] {
            assert!(!eof, "must not need another call after eof already fired");
            let (bytes, hit_eof) = driver.download_stream_chunk(handle, max_len).await.unwrap();
            collected.extend_from_slice(&bytes);
            eof = hit_eof;
        }
        assert_eq!(collected, b"abcdefghij");
        assert!(eof, "the full payload must be drained by the last call");
    }

    #[test]
    fn resolve_download_target_rejects_a_server_side_failure_with_the_gs_message() {
        let response = query_response::Response {
            success: false,
            code: None,
            message: Some("Stage 'MISSING_STAGE' does not exist".to_string()),
            data: query_response::Data::default(),
        };

        match resolve_download_target(
            response,
            PutGetResultsetFlavor::Python,
            false,
            false,
            "data.csv",
        ) {
            Err(ApiError::InvalidArgument { argument, .. }) => {
                assert_eq!(argument, "Stage 'MISSING_STAGE' does not exist");
            }
            other => panic!("expected InvalidArgument echoing the GS message, got {other:?}"),
        }
    }

    #[test]
    fn resolve_download_target_defaults_the_rejection_message_when_gs_omits_one() {
        let response = query_response::Response {
            success: false,
            code: None,
            message: None,
            data: query_response::Data::default(),
        };

        match resolve_download_target(
            response,
            PutGetResultsetFlavor::Python,
            false,
            false,
            "data.csv",
        ) {
            Err(ApiError::InvalidArgument { argument, .. }) => {
                assert_eq!(argument, "GET command rejected by server");
            }
            other => panic!("expected the default rejection message, got {other:?}"),
        }
    }

    #[test]
    fn resolve_download_target_rejects_a_response_with_no_source_locations() {
        // `to_file_download_data` itself rejects an absent/empty
        // `src_locations` before `resolve_download_target`'s own "File not
        // found" fallback ever runs — that fallback is defensive and
        // unreachable through this entry point today. Asserted generically,
        // not pinned to "File not found", since the actual message comes
        // from `to_file_download_data`'s check.
        let response = query_response::Response {
            success: true,
            code: None,
            message: None,
            data: query_response::Data::default(),
        };

        let result = resolve_download_target(
            response,
            PutGetResultsetFlavor::Python,
            false,
            false,
            "missing.csv",
        );
        assert!(
            result.is_err(),
            "a GS response with no source locations must be rejected, not silently resolved"
        );
    }
}
