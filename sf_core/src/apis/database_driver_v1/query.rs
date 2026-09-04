use super::ColumnMetadata;
use super::connection::{Connection, RefreshContext};
use super::global_state::{PutGetResultsetFlavor, WrapperPresets};
use crate::apis::operation_ctx::OperationCtx;
use crate::arrow_utils::ArrowUtilsError;
use crate::arrow_utils::{boxed_arrow_reader, create_schema};
use crate::chunks::{
    ChunkError, PrefetchConfig, arrow_prefetch_reader, empty_reader, json_prefetch_reader,
    schema_only_reader, single_chunk_reader,
};
use crate::config::retry::RetryPolicy;
use crate::file_manager;
use crate::file_manager::{
    ByteSource, DownloadResult, SingleUploadData, StageInfoCache, StageInfoRefreshError,
    StageInfoSnapshot, UploadResult, download_files, upload_files, upload_prepared_source,
};
use crate::query_types::RowType;
use crate::rest;
use crate::utils::sync::MutexRecoverExt;
use arrow::array::{Array, Int64Array, RecordBatchReader, StringArray};
use arrow::datatypes::DataType;
use arrow::error::ArrowError;
use futures::FutureExt as _;
use futures::future::{BoxFuture, Shared};
use reqwest::Client;
use rest::snowflake::query_response::{self, QueryResponseError, RowsetData};
use snafu::{IntoError, Location, OptionExt, ResultExt, Snafu};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const PUT_GET_ROWSET_TEXT_LENGTH: u64 = 10000;
const PUT_GET_ROWSET_FIXED_LENGTH: u64 = 64;

/// Constant value of the PUT result's `encryption` column (same for every row).
/// Matches legacy ODBC and snowflake-jdbc.
const PUT_ENCRYPTION_LITERAL: &str = "ENCRYPTED";

/// Constant value of the GET result's `encryption` column. See `PUT_ENCRYPTION_LITERAL`.
const GET_ENCRYPTION_LITERAL: &str = "DECRYPTED";

/// Whether the PUT/GET result set carries the `encryption` column (between
/// `status` and `message`). ODBC and JDBC do; Python does not.
fn emits_encryption_column(flavor: &PutGetResultsetFlavor) -> bool {
    matches!(
        flavor,
        PutGetResultsetFlavor::Odbc | PutGetResultsetFlavor::Jdbc
    )
}

/// Inputs the refresher needs to re-issue the original PUT/GET SQL against GS.
///
/// The connection handle is held instead of a snapshot session token: a long
/// upload batch can outlive its session, and reading the token freshly per
/// refresh (via `RefreshContext::execute_with_refresh`) lets PR #1137's
/// session-renewal path heal a 390112 transparently.
#[derive(Clone)]
pub struct StageInfoRefreshContext {
    pub sql: String,
    pub query_parameters: crate::config::rest_parameters::QueryParameters,
    pub conn: Arc<Mutex<Connection>>,
}

/// Executes a PUT/GET file transfer, returning the results as a `RowsetData`.
///
/// When `stage_info_refresh_context` is `Some`, recoverable stage-info-expiry
/// errors re-issue the original PUT/GET SQL for a fresh `StageInfoSnapshot` and
/// retry the operation; non-PUT/GET callers pass `None`.
///
/// `operation_ctx` is what the transfer registers its cancellation cleanup against; `None`
/// makes it uncancellable.
#[allow(clippy::too_many_arguments)]
pub(super) async fn perform_put_get_transfer(
    operation_ctx: Option<&OperationCtx>,
    command: &str,
    data: &query_response::Data,
    wrapper_presets: &WrapperPresets,
    retry_policy: &RetryPolicy,
    stage_info_refresh_context: Option<StageInfoRefreshContext>,
    use_s3_regional_url_session_param: bool,
    skip_upload_on_content_match: bool,
    put_fastfail: bool,
    get_fastfail: bool,
    unsafe_file_write: bool,
    tls_config: crate::tls::config::TlsConfig,
    proxy_config: crate::tls::config::ProxyConfig,
    crl_worker: crate::crl::worker::SharedCrlWorker,
) -> Result<RowsetData, QueryResponseProcessingError> {
    // Seed the refresher's cache with the initial snapshot.
    let initial_snapshot = data
        .stage_info_snapshot()
        .context(FileTransferPreparationSnafu)?;
    let refresher =
        stage_info_refresh_context
            .zip(initial_snapshot)
            .map(|(stage_refresh_ctx, initial)| {
                SnowflakeStageInfoRefresher::new(stage_refresh_ctx, initial)
            });
    let refresher_handle = refresher
        .as_ref()
        .map(|r| r as &dyn file_manager::StageInfoRefresher);
    let transfer_ctx = file_manager::TransferCtx::new(
        refresher_handle,
        operation_ctx.map(OperationCtx::cleanup_scope),
    );
    // Bundled once and threaded into whichever converter the running arm
    // calls below — see `file_manager::StageTransport`.
    let transport = file_manager::StageTransport {
        tls_config,
        proxy_config,
        crl_worker,
    };

    match command {
        "UPLOAD" => {
            let file_upload_data = data
                .to_file_upload_data(
                    wrapper_presets.put_get_resultset_flavor.clone(),
                    wrapper_presets.legacy_odbc_compression_autodetect,
                    skip_upload_on_content_match,
                    use_s3_regional_url_session_param,
                    put_fastfail,
                    &transport,
                )
                .context(FileTransferPreparationSnafu)?;
            let upload_results = upload_files(&file_upload_data, retry_policy, transfer_ctx)
                .await
                .context(FileUploadSnafu)?;
            Ok(RowsetData::Upload(upload_results))
        }
        "DOWNLOAD" => match data.to_file_download_data(
            &wrapper_presets.put_get_resultset_flavor,
            use_s3_regional_url_session_param,
            unsafe_file_write,
            get_fastfail,
            &transport,
        ) {
            Ok(file_download_data) => {
                let download_results =
                    download_files(file_download_data, retry_policy, transfer_ctx)
                        .await
                        .context(FileDownloadSnafu)?;
                Ok(RowsetData::Download(download_results))
            }
            Err(e) if e.is_missing_source_locations() => {
                if wrapper_presets.legacy_empty_get_on_missing {
                    Ok(RowsetData::Download(Vec::new()))
                } else {
                    RemoteFileNotFoundSnafu.fail()
                }
            }
            Err(e) => Err(FileTransferPreparationSnafu.into_error(e)),
        },
        _ => UnsupportedCommandSnafu {
            command: command.to_string(),
        }
        .fail(),
    }
}

/// Uploads `payload` (already reassembled from the caller's stream — either
/// an in-memory buffer or a spooled temp file, see `SpooledBuffer`) to the
/// stage described by a GS PUT response, returning a single-row
/// `RowsetData::Upload`. Mirrors the UPLOAD arm of [`perform_put_get_transfer`]
/// but sources the data from `payload` instead of expanding a local glob —
/// backs the chunked `connection_upload_stream_{begin,chunk,finish}` RPCs
/// (JDBC `uploadStream`, Python `file_stream`).
///
/// The destination filename is the basename of the PUT command's `file://`
/// token (echoed back by GS as `src_location_pattern`); auto-compress,
/// overwrite, and encryption all follow the GS response, exactly as a normal
/// file-path PUT.
pub(super) async fn build_and_upload_stream(
    data: &query_response::Data,
    wrapper_presets: &WrapperPresets,
    stage_info_refresh_context: Option<StageInfoRefreshContext>,
    use_s3_regional_url_session_param: bool,
    put_get_policy: &RetryPolicy,
    transport: &file_manager::StageTransport,
    payload: ByteSource,
) -> Result<RowsetData, QueryResponseProcessingError> {
    // Streaming PUT builds `StageInfo` outside `perform_put_get_transfer`, so
    // the connection's TLS, proxy, and CRL settings are threaded here via the
    // same `StageTransport` bundle that function's UPLOAD arm uses. `crl_worker`
    // is threaded for parity with the non-streaming PUT/GET path; the
    // hermetic/live proxy tests disable CRL checking, so it isn't exercised.
    let upload_data = data
        .to_file_upload_data(
            wrapper_presets.put_get_resultset_flavor.clone(),
            wrapper_presets.legacy_odbc_compression_autodetect,
            // Stream PUT never skips on content match: the API has no cursor
            // kwarg to opt into it and always uploads the supplied source.
            false,
            use_s3_regional_url_session_param,
            // Single-file, in-memory PUT: it builds one `SingleUploadData` and
            // never enters the `upload_files` batch loop, so `put_fastfail` is
            // inert here — seed it from the wrapper preset for consistency.
            wrapper_presets.put_get_fastfail_default,
            transport,
        )
        .context(FileTransferPreparationSnafu)?;

    let filename = std::path::Path::new(&upload_data.src_location_pattern)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&upload_data.src_location_pattern)
        .to_string();

    // Seed the refresher with the initial snapshot so a mid-upload cred/URL
    // expiry can re-issue the PUT SQL — identical machinery to the file path.
    let initial_snapshot = data
        .stage_info_snapshot()
        .context(FileTransferPreparationSnafu)?;
    let refresher =
        stage_info_refresh_context
            .zip(initial_snapshot)
            .map(|(stage_refresh_ctx, initial)| {
                SnowflakeStageInfoRefresher::new(stage_refresh_ctx, initial)
            });
    let refresher_handle = refresher
        .as_ref()
        .map(|r| r as &dyn file_manager::StageInfoRefresher);

    let single = SingleUploadData {
        // `upload_prepared_source` reads from `source` (below), not this
        // field; this placeholder only satisfies the struct. It also drives
        // `upload_result_source`'s Windows+ODBC "full local path" branch,
        // which is intentionally skipped here — a stream/chunked upload has
        // no meaningful local path (memory or a core-internal spool file) —
        // so the result's `source` column falls back to `filename` on every
        // (flavor, host) combination, matching the pre-chunking behavior.
        source: ByteSource::Bytes(bytes::Bytes::new()),
        filename,
        stage_info: upload_data.stage_info,
        encryption_material: upload_data.encryption_material,
        auto_compress: upload_data.auto_compress,
        source_compression: upload_data.source_compression,
        overwrite: upload_data.overwrite,
        flavor: upload_data.flavor,
        legacy_odbc_compression_autodetect: upload_data.legacy_odbc_compression_autodetect,
        skip_upload_on_content_match: upload_data.skip_upload_on_content_match,
        multipart: upload_data.multipart,
    };

    // No cleanup scope, and no cancellation coverage on this path at all — stated
    // plainly because the shape invites the opposite assumption.
    //
    // None of the `ConnectionUploadStream*` RPCs are marked `async_first`, so no
    // operation token reaches this layer and there is nothing to register against.
    // `connection_upload_stream_abort` is not a substitute: it only deletes local
    // handle state (and unlinks the spool file), issues no cloud-side abort, and
    // structurally cannot — `connection_upload_stream_finish` deletes the handle
    // before the upload starts, so by the time a multipart upload is in flight there
    // is no handle left for it to act on.
    //
    // So a cancelled *streaming* PUT can still orphan the S3/GCS debris that the
    // file-path PUT now aborts. Deliberately out of scope here; closing it means
    // marking those RPCs `async_first` and threading `operation_ctx` through them.
    // TODO: needs a tracking ticket before this PR merges — see the PR discussion.

    // No scheduler joined: a streamed PUT is a batch of one, so the cloud upload
    // leaf sizes its own budget (see `file_manager::scheduler_for`).
    let transfer_ctx = match refresher_handle {
        Some(refresher) => file_manager::TransferCtx::with_refresher(refresher),
        None => file_manager::TransferCtx::default(),
    };
    let result = upload_prepared_source(payload, single, put_get_policy, transfer_ctx)
        .await
        .context(FileUploadSnafu)?;

    Ok(RowsetData::Upload(vec![result]))
}

/// Builds the stage-info refresher used by `download_stream_begin`. Exposed
/// so `stream_transfer.rs` can drive a streaming GET with the same
/// cred/URL-refresh machinery as the file-path path, without re-exposing the
/// private `SnowflakeStageInfoRefresher` type.
pub(super) fn stream_stage_info_refresher(
    stage_refresh_ctx: StageInfoRefreshContext,
    initial: StageInfoSnapshot,
) -> impl file_manager::StageInfoRefresher {
    SnowflakeStageInfoRefresher::new(stage_refresh_ctx, initial)
}

/// Window during which repeated `refresh()` calls return without hitting GS.
/// Matches ODBC's `FileTransferAgent.cpp` `m_lastRefreshTokenSec` gate (10
/// minutes), which coalesces rapid-fire refreshes from concurrent uploads.
///
/// Applies only to `refresh` (cred-style: S3 STS expiry, GCS 401). Per-file
/// URL refresh (`refresh_url`, GCS 400) intentionally bypasses this window:
/// a single batch upload of 1000 files may carry up to 1000 distinct
/// per-object presigned URLs, and coalescing would lock all subsequent
/// expiries to the first-refreshed URL.
const REFRESH_COALESCE_WINDOW: Duration = Duration::from_secs(10 * 60);

/// A single in-flight GS stage-info refresh, cloned by all coalescing callers so
/// N concurrent 403/STS-expiry callers collapse into one GS fetch.
type InflightRefresh = Shared<BoxFuture<'static, Result<Instant, StageInfoRefreshError>>>;

/// Injectable fetch (re-executes the PUT/GET SQL and stores the result).
/// Production defaults to `fetch_and_store`; coordinator tests substitute a
/// counting stub via `new_with_fetch_fn` so the single-flight leader path is
/// driven by tests without a live GS call.
type StageInfoFetchFn = Arc<
    dyn Fn(
            StageInfoRefreshContext,
            StageInfoCache,
        ) -> BoxFuture<'static, Result<Instant, StageInfoRefreshError>>
        + Send
        + Sync,
>;

/// Refreshes stage info by re-executing the original PUT/GET SQL against GS and
/// storing the fresh `StageInfoSnapshot` in the shared cache. Single-flight
/// coordinator: N concurrent 403 callers on the same generation share one GS
/// fetch. (`refresh_url` bypasses the window for GCS per-file URL expiry.)
struct SnowflakeStageInfoRefresher {
    stage_refresh_ctx: StageInfoRefreshContext,
    cache: StageInfoCache,
    last_refresh_at: std::sync::Mutex<Option<Instant>>,
    /// In-flight fetch shared across concurrent 403/STS-expiry callers. Only
    /// the "leader" (the first caller that wins the `inflight` lock while the
    /// slot is empty) starts the actual GS fetch; every "follower" clones the
    /// `Shared` future and awaits it alongside the leader.
    inflight: tokio::sync::Mutex<Option<InflightRefresh>>,
    /// Injectable fetch implementation. Production uses `fetch_and_store`;
    /// tests may substitute a fake via `new_with_fetch_fn`.
    fetch_fn: StageInfoFetchFn,
}

impl SnowflakeStageInfoRefresher {
    fn new(stage_refresh_ctx: StageInfoRefreshContext, initial: StageInfoSnapshot) -> Self {
        Self {
            stage_refresh_ctx,
            cache: StageInfoCache::new(initial),
            last_refresh_at: std::sync::Mutex::new(None),
            inflight: tokio::sync::Mutex::new(None),
            fetch_fn: Arc::new(|stage_refresh_ctx, cache| {
                fetch_and_store(stage_refresh_ctx, cache).boxed()
            }),
        }
    }

    /// Test-only constructor that injects a custom fetch function instead of
    /// calling the real GS endpoint.
    #[cfg(test)]
    fn new_with_fetch_fn(
        stage_refresh_ctx: StageInfoRefreshContext,
        initial: StageInfoSnapshot,
        fetch_fn: impl Fn(
            StageInfoRefreshContext,
            StageInfoCache,
        ) -> BoxFuture<'static, Result<Instant, StageInfoRefreshError>>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            stage_refresh_ctx,
            cache: StageInfoCache::new(initial),
            last_refresh_at: std::sync::Mutex::new(None),
            inflight: tokio::sync::Mutex::new(None),
            fetch_fn: Arc::new(fetch_fn),
        }
    }
}

/// Returns `true` if a refresh recorded at `last` is still considered fresh
/// at `now` and a new fetch should be coalesced. Extracted so the
/// time-window logic can be unit-tested without a real `Instant::now()`.
fn within_coalesce_window(last: Option<Instant>, now: Instant) -> bool {
    last.is_some_and(|at| now.saturating_duration_since(at) < REFRESH_COALESCE_WINDOW)
}

/// Fetches fresh stage info from GS and stores it in `cache`. Returns the new
/// `cache.cached_at()` generation. Used as the leader's work unit in the
/// single-flight coordinator.
async fn fetch_and_store(
    stage_refresh_ctx: StageInfoRefreshContext,
    cache: StageInfoCache,
) -> Result<Instant, StageInfoRefreshError> {
    let snapshot = fetch_fresh_stage_info(&stage_refresh_ctx).await?;
    cache.store(snapshot);
    Ok(cache.cached_at())
}

impl file_manager::StageInfoRefresher for SnowflakeStageInfoRefresher {
    fn refresh(&self, observed: Instant) -> file_manager::RefreshFuture<'_> {
        Box::pin(async move {
            // Fast path: cache already holds a newer generation.
            let cur = self.cache.cached_at();
            if cur > observed {
                return Ok(cur);
            }

            // Acquire the inflight slot. Double-check under the lock.
            let mut inflight_slot = self.inflight.lock().await;
            let cur = self.cache.cached_at();
            if cur > observed {
                return Ok(cur);
            }

            let fut: Shared<BoxFuture<'static, Result<Instant, StageInfoRefreshError>>> =
                match &*inflight_slot {
                    Some(f) => {
                        // Follower: join the in-flight fetch started by the leader.
                        // Do NOT re-check the coalescing window — the leader already
                        // decided a fetch was needed.
                        f.clone()
                    }
                    None => {
                        // Leader: decide whether to fetch or coalesce.
                        let last = *self.last_refresh_at.lock_recover();
                        // Final generation re-check: another task may have stored a
                        // new generation between the double-check above and acquiring
                        // the last_refresh_at lock.
                        let cur = self.cache.cached_at();
                        if cur > observed {
                            return Ok(cur);
                        }
                        if within_coalesce_window(last, Instant::now()) {
                            // Sequential within-window terminal: the cache is still
                            // considered fresh; tell the caller the refresh was a
                            // no-op by returning the current (unchanged) generation.
                            tracing::debug!(
                                "Stage info refresh coalesced; cache holds recent snapshot"
                            );
                            return Ok(observed);
                        }
                        tracing::info!("Refreshing stage info by re-executing PUT/GET SQL");
                        let shared =
                            (self.fetch_fn)(self.stage_refresh_ctx.clone(), self.cache.clone())
                                .shared();
                        *inflight_slot = Some(shared.clone());
                        shared
                    }
                };

            // Drop the inflight lock BEFORE awaiting — never hold a lock
            // across an .await (liveness: the leader's future must be able to
            // complete even while followers are already awaiting it).
            drop(inflight_slot);

            let refresh_result = fut.clone().await;

            // Re-acquire the inflight slot to clear it. Stamp the coalesce
            // window BEFORE clearing the slot, inside the still-held critical
            // section, so a straggler that observes the cleared slot also sees
            // the fresh timestamp and coalesces instead of launching a second
            // GS fetch. Only stamp on success (a failed refresh must re-fetch).
            let mut inflight_slot = self.inflight.lock().await;
            if refresh_result.is_ok() {
                *self.last_refresh_at.lock_recover() = Some(Instant::now());
            }
            if let Some(ref slot_fut) = *inflight_slot
                && Shared::ptr_eq(slot_fut, &fut)
            {
                *inflight_slot = None;
            }
            drop(inflight_slot);

            refresh_result
        })
    }

    fn refresh_url(&self, current_upload_file: Option<&str>) -> file_manager::RefreshFuture<'_> {
        // `current_upload_file` is a `&str` borrow; we must own the value
        // inside the async block (which is `'_` but moves into a Box::pin).
        let current_upload_file = current_upload_file.map(str::to_string);
        Box::pin(async move {
            // For a PUT, re-issue the SQL rewritten to target the single file
            // currently uploading so GS returns *that* file's presigned URL —
            // re-issuing the original glob SQL would return the first matched
            // file's URL and misroute the upload. For a GET, the call site
            // re-picks `presignedUrls[per_file_index]`, so re-issue unchanged.
            let sql = match current_upload_file.as_deref() {
                Some(dst) => match rewrite_put_command_for_file(&self.stage_refresh_ctx.sql, dst) {
                    Some(rewritten) => rewritten,
                    // PUT command with no parseable `file://` token: refuse to
                    // re-issue the unchanged SQL (it would misroute) and let
                    // the GCS call site surface PresignedUrlExpired.
                    None => {
                        use crate::file_manager::types::stage_info_refresh_error::PresignedUrlRefreshSkippedSnafu;
                        return PresignedUrlRefreshSkippedSnafu.fail();
                    }
                },
                None => self.stage_refresh_ctx.sql.clone(),
            };
            // Per-file URL refresh: bypass the coalescing window and the
            // single-flight gate. Each file may carry a distinct per-object
            // presigned URL, so collapsing refresh calls would lock subsequent
            // files to a stale URL. The GCS call site enforces a two-strike guard.
            tracing::info!(
                "Refreshing stage info (presigned URLs) by re-executing PUT/GET SQL — \
                 bypassing 10-min coalesce window for per-file URL expiry"
            );
            let snapshot = fetch_fresh_stage_info_with_sql(&self.stage_refresh_ctx, &sql).await?;
            self.cache.store(snapshot);
            // Stamp last_refresh_at so a subsequent token-style refresh honors
            // the window — the snapshot we just wrote carries fresh creds too.
            *self.last_refresh_at.lock_recover() = Some(Instant::now());
            Ok(self.cache.cached_at())
        })
    }

    fn cache(&self) -> &StageInfoCache {
        &self.cache
    }
}

/// Extracts the local `file://` path token from a PUT command; returns `None`
/// for a GET command or empty/malformed input.
fn local_file_path_from_put_command(sql: &str) -> Option<&str> {
    const FILE_PROTOCOL: &str = "file://";
    let proto_idx = sql.find(FILE_PROTOCOL)?;
    let quoted = proto_idx > 0 && sql.as_bytes()[proto_idx - 1] == b'\'';
    let rest = &sql[proto_idx + FILE_PROTOCOL.len()..];
    let end = if quoted {
        rest.find('\'')?
    } else {
        rest.find([' ', '\n', ';']).unwrap_or(rest.len())
    };
    let path = &rest[..end];
    (!path.is_empty()).then_some(path)
}

/// Rewrites a PUT command so it targets a single destination file: the local
/// path token after `file://` is replaced with `dst_file_name` (GS resolves the
/// remote object from the trailing name, so the local prefix is dropped).
/// Returns `None` when the command has no parseable local path. Mirrors
/// libsfclient `getPresignedUrlForUploading` and Python `_update_presigned_url`.
fn rewrite_put_command_for_file(sql: &str, dst_file_name: &str) -> Option<String> {
    let local_path = local_file_path_from_put_command(sql)?;
    Some(sql.replace(local_path, dst_file_name))
}

/// Re-issues the original PUT/GET SQL (`stage_refresh_ctx.sql`) and extracts the fresh
/// `stageInfo` snapshot. See [`fetch_fresh_stage_info_with_sql`].
async fn fetch_fresh_stage_info(
    stage_refresh_ctx: &StageInfoRefreshContext,
) -> Result<StageInfoSnapshot, StageInfoRefreshError> {
    fetch_fresh_stage_info_with_sql(stage_refresh_ctx, &stage_refresh_ctx.sql).await
}

/// Re-issues `sql` via `RefreshContext::execute_with_refresh` and extracts the
/// fresh `stageInfo` snapshot from the response. `sql` is usually `stage_refresh_ctx.sql`; the
/// per-file URL-refresh path passes a command rewritten for one destination file.
async fn fetch_fresh_stage_info_with_sql(
    stage_refresh_ctx: &StageInfoRefreshContext,
    sql: &str,
) -> Result<StageInfoSnapshot, StageInfoRefreshError> {
    use crate::file_manager::types::stage_info_refresh_error::*;

    // `from_arc` is used (not `new`) so that a `close()` raced against an
    // in-flight refresh is rejected, consistent with the original query path.
    let mut refresh_ctx = RefreshContext::from_arc(&stage_refresh_ctx.conn)
        .await
        .context(QueryFailedSnafu)?;
    // `from_arc` already validates that `http_client` is present (via the
    // is_closed check + `RefreshContext::new`), so this lookup just clones it.
    let http_client = stage_refresh_ctx
        .conn
        .lock()
        .await
        .http_client
        .clone()
        .expect("http_client present after RefreshContext::from_arc succeeded");

    let query_input = rest::snowflake::QueryInput::new(sql.to_string());
    let response = refresh_ctx
        .execute_with_refresh(|session_token| {
            let http_client = http_client.clone();
            let query_parameters = stage_refresh_ctx.query_parameters.clone();
            let query_input = query_input.clone();
            async move {
                rest::snowflake::snowflake_query_with_client(
                    &http_client,
                    query_parameters,
                    session_token.reveal(),
                    query_input,
                    rest::snowflake::QueryOptions::default(),
                )
                .await
            }
        })
        .await
        .context(QueryFailedSnafu)?;

    if !response.success {
        return ServerRejectedSnafu {
            message: response
                .message
                .unwrap_or_else(|| "Unknown error".to_string()),
        }
        .fail();
    }

    // The re-issued PUT/GET carries the fresh stageInfo on the response.
    response
        .data
        .stage_info_snapshot()
        .context(InvalidStageInfoSnafu)?
        .context(MissingStageInfoSnafu)
}

/// Builds an Arrow `RecordBatchReader` from the stored `RowsetData`.
/// Called lazily by `result_set_get_stream`.
pub(super) async fn build_reader_from_rowset_data(
    data: &RowsetData,
    http_client: Client,
    prefetch_config: &PrefetchConfig,
    wrapper_presets: &WrapperPresets,
    nullable_flags: Option<&[bool]>,
) -> Result<Box<dyn RecordBatchReader + Send>, QueryResponseProcessingError> {
    match data {
        RowsetData::Upload(results) => {
            upload_results_reader(results, wrapper_presets).context(UploadResultsConversionSnafu)
        }
        RowsetData::Download(results) => download_results_reader(results, wrapper_presets)
            .context(DownloadResultsConversionSnafu),
        _ => read_batches(data, http_client, prefetch_config, nullable_flags)
            .await
            .context(BatchReadSnafu),
    }
}

pub(super) async fn read_batches(
    data: &RowsetData,
    http_client: Client,
    prefetch_config: &PrefetchConfig,
    nullable_flags: Option<&[bool]>,
) -> Result<Box<dyn RecordBatchReader + Send>, ReadBatchesError> {
    match data {
        RowsetData::ArrowSingleChunk { chunk_base64 } => {
            single_chunk_reader(chunk_base64, nullable_flags).context(ChunkReadSnafu)
        }
        RowsetData::ArrowMultiChunk {
            initial_base64_opt,
            chunk_download_data,
        } => arrow_prefetch_reader(
            initial_base64_opt.as_deref(),
            chunk_download_data.clone().into(),
            http_client.clone(),
            prefetch_config,
            nullable_flags,
        )
        .await
        .context(ChunkReadSnafu),
        RowsetData::SchemaOnly { rowtype } => {
            let row_types = parse_row_types(rowtype)?;
            schema_only_reader(&row_types).context(ChunkReadSnafu)
        }
        RowsetData::JsonRowset { rowset, rowtype } => {
            let row_types = parse_row_types(rowtype)?;
            validate_column_count(rowset, &row_types)?;
            json_prefetch_reader(
                rowset,
                row_types,
                Vec::new(),
                http_client.clone(),
                prefetch_config,
            )
            .await
            .context(ChunkReadSnafu)
        }
        RowsetData::JsonMultiChunk {
            rowset,
            rowtype,
            chunk_download_data,
        } => {
            let row_types = parse_row_types(rowtype)?;
            validate_column_count(rowset, &row_types)?;

            json_prefetch_reader(
                rowset,
                row_types,
                chunk_download_data.clone(),
                http_client.clone(),
                prefetch_config,
            )
            .await
            .context(ChunkReadSnafu)
        }
        RowsetData::NoData | RowsetData::Upload(_) | RowsetData::Download(_) => Ok(empty_reader()),
    }
}

fn parse_row_types(rowtype: &[query_response::RowType]) -> Result<Vec<RowType>, ReadBatchesError> {
    rowtype
        .iter()
        .map(|rt| rt.try_into())
        .collect::<Result<Vec<_>, _>>()
        .context(RowTypeParseSnafu)
}

fn validate_column_count(
    rowset: &[Vec<Option<String>>],
    row_types: &[RowType],
) -> Result<(), ReadBatchesError> {
    if let Some(first_row) = rowset.first() {
        let num_columns_rowset = first_row.len();
        let num_columns_rowtype = row_types.len();
        if num_columns_rowset != num_columns_rowtype {
            return ColumnCountMismatchSnafu {
                rowtype_count: num_columns_rowtype,
                rowset_count: num_columns_rowset,
            }
            .fail();
        }
    }
    Ok(())
}

/// Helper macro to create string arrays from field accessors
macro_rules! string_array {
    ($data:expr, $field:ident) => {
        Arc::new(StringArray::from(
            $data.iter().map(|r| r.$field.as_str()).collect::<Vec<_>>(),
        ))
    };
}

/// Helper macro to create int64 arrays from field accessors
macro_rules! int64_array {
    ($data:expr, $field:ident) => {
        Arc::new(Int64Array::from(
            $data.iter().map(|r| r.$field).collect::<Vec<_>>(),
        ))
    };
}

fn upload_row_types(wrapper_presets: &WrapperPresets) -> Vec<(RowType, DataType)> {
    let mut row_types = vec![
        build_generic_text_rowtype("source"),
        build_generic_text_rowtype("target"),
        build_generic_fixed_rowtype("source_size"),
        build_generic_fixed_rowtype("target_size"),
        build_generic_text_rowtype("source_compression"),
        build_generic_text_rowtype("target_compression"),
        build_generic_text_rowtype("status"),
    ];
    if emits_encryption_column(&wrapper_presets.put_get_resultset_flavor) {
        row_types.push(build_generic_text_rowtype("encryption"));
    }
    row_types.push(build_generic_text_rowtype("message"));
    row_types
}

fn download_row_types(wrapper_presets: &WrapperPresets) -> Vec<(RowType, DataType)> {
    let mut row_types = vec![
        build_generic_text_rowtype("file"),
        build_generic_fixed_rowtype("size"),
        build_generic_text_rowtype("status"),
    ];
    if emits_encryption_column(&wrapper_presets.put_get_resultset_flavor) {
        row_types.push(build_generic_text_rowtype("encryption"));
    }
    row_types.push(build_generic_text_rowtype("message"));
    row_types
}

/// Converts upload results to Arrow format
pub(super) fn upload_results_reader(
    upload_results: &[UploadResult],
    wrapper_presets: &WrapperPresets,
) -> Result<Box<dyn RecordBatchReader + Send>, ArrowError> {
    let schema = create_schema(&upload_row_types(wrapper_presets))
        .expect("Failed to create schema from RowTypes");

    let n = upload_results.len();
    let mut columns: Vec<Arc<dyn Array>> = vec![
        string_array!(upload_results, source),
        string_array!(upload_results, target),
        int64_array!(upload_results, source_size),
        int64_array!(upload_results, target_size),
        string_array!(upload_results, source_compression),
        string_array!(upload_results, target_compression),
        string_array!(upload_results, status),
    ];
    if emits_encryption_column(&wrapper_presets.put_get_resultset_flavor) {
        columns.push(Arc::new(StringArray::from_iter_values(
            std::iter::repeat_n(PUT_ENCRYPTION_LITERAL, n),
        )));
    }
    columns.push(string_array!(upload_results, message));

    boxed_arrow_reader(schema, columns)
}

/// Converts download results to Arrow format
pub(super) fn download_results_reader(
    download_results: &[DownloadResult],
    wrapper_presets: &WrapperPresets,
) -> Result<Box<dyn RecordBatchReader + Send>, ArrowError> {
    let schema = create_schema(&download_row_types(wrapper_presets))
        .expect("Failed to create schema from RowTypes");

    let n = download_results.len();
    let mut columns: Vec<Arc<dyn Array>> = vec![
        string_array!(download_results, file),
        int64_array!(download_results, size),
        string_array!(download_results, status),
    ];
    if emits_encryption_column(&wrapper_presets.put_get_resultset_flavor) {
        columns.push(Arc::new(StringArray::from_iter_values(
            std::iter::repeat_n(GET_ENCRYPTION_LITERAL, n),
        )));
    }
    columns.push(string_array!(download_results, message));

    boxed_arrow_reader(schema, columns)
}

fn build_generic_text_rowtype(name: &str) -> (RowType, DataType) {
    (
        RowType::text(
            name,
            false,
            PUT_GET_ROWSET_TEXT_LENGTH,
            PUT_GET_ROWSET_TEXT_LENGTH,
        ),
        DataType::Utf8,
    )
}

fn build_generic_fixed_rowtype(name: &str) -> (RowType, DataType) {
    (
        RowType::fixed_with_scale_zero(name, false, PUT_GET_ROWSET_FIXED_LENGTH),
        DataType::Int64,
    )
}

/// Convert an internal `RowType` to protobuf `ColumnMetadata`.
fn rowtype_to_column_metadata(rt: &RowType) -> ColumnMetadata {
    match rt {
        RowType::Text {
            name,
            nullable,
            length,
            byte_length,
        } => ColumnMetadata {
            name: name.clone(),
            r#type: "TEXT".to_string(),
            precision: None,
            scale: None,
            length: Some(*length as i64),
            byte_length: Some(*byte_length as i64),
            nullable: *nullable,
            dimension: None,
            fixed: false,
            column_src_database: String::new(),
            column_src_schema: String::new(),
            column_src_table: String::new(),
            is_auto_increment: false,
            ext_col_type_name: String::new(),
            udt_output_type: String::new(),
            fields: Vec::new(),
        },
        RowType::Fixed {
            name,
            nullable,
            precision,
            scale,
        } => ColumnMetadata {
            name: name.clone(),
            r#type: "FIXED".to_string(),
            precision: Some(*precision as i64),
            scale: Some(*scale as i64),
            length: None,
            byte_length: None,
            nullable: *nullable,
            dimension: None,
            fixed: true,
            column_src_database: String::new(),
            column_src_schema: String::new(),
            column_src_table: String::new(),
            is_auto_increment: false,
            ext_col_type_name: String::new(),
            udt_output_type: String::new(),
            fields: Vec::new(),
        },
        _ => todo!(),
    }
}

/// Build column metadata for PUT (UPLOAD) results.
pub fn upload_column_metadata(wrapper_presets: &WrapperPresets) -> Vec<ColumnMetadata> {
    upload_row_types(wrapper_presets)
        .iter()
        .map(|(r, _)| rowtype_to_column_metadata(r))
        .collect()
}

/// Build column metadata for GET (DOWNLOAD) results.
pub fn download_column_metadata(wrapper_presets: &WrapperPresets) -> Vec<ColumnMetadata> {
    download_row_types(wrapper_presets)
        .iter()
        .map(|(r, _)| rowtype_to_column_metadata(r))
        .collect()
}

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
pub enum QueryResponseProcessingError {
    #[snafu(display("Failed to convert upload results to Arrow format"))]
    UploadResultsConversion {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to convert download results to Arrow format"))]
    DownloadResultsConversion {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to upload files"))]
    FileUpload {
        source: file_manager::FileManagerError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to download files"))]
    FileDownload {
        source: file_manager::FileManagerError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read batches from query response"))]
    BatchRead {
        source: ReadBatchesError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Unsupported command in query response: {command}"))]
    UnsupportedCommand {
        command: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to prepare file transfer data"))]
    FileTransferPreparation {
        source: QueryResponseError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("While getting file(s) there was an error: the file does not exist"))]
    RemoteFileNotFound {
        #[snafu(implicit)]
        location: Location,
    },
}

pub(crate) fn remote_file_not_found() -> QueryResponseProcessingError {
    RemoteFileNotFoundSnafu.build()
}

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
pub enum ReadBatchesError {
    #[snafu(display(
        "Column count mismatch: rowtype has {rowtype_count} columns, but rowset has {rowset_count} columns"
    ))]
    ColumnCountMismatch {
        rowtype_count: usize,
        rowset_count: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Rowset or rowtype not found in the response"))]
    MissingRowsetOrRowtype {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse rowtype"))]
    RowTypeParse {
        source: QueryResponseError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to decode base64 rowset"))]
    Base64Decode {
        source: base64::DecodeError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read chunks"))]
    ChunkRead {
        source: ChunkError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to convert rowset to Arrow format"))]
    RowsetConversion {
        source: ArrowUtilsError,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Test-only building block for the coordinator tests (in this module) and for
/// other modules' tests that drive the REAL single-flight coordinator. Only the
/// injected `fetch_fn` runs, so `sql`/`query_parameters` are never consumed —
/// stub them with minimal valid values.
#[cfg(test)]
fn stub_ctx() -> StageInfoRefreshContext {
    use crate::config::rest_parameters::{ClientInfo, QueryParameters};
    use crate::crl::config::CrlConfig;
    use crate::tls::config::TlsConfig;
    StageInfoRefreshContext {
        sql: "PUT file://x @s".into(),
        query_parameters: QueryParameters {
            server_url: "https://stub.example.com".into(),
            client_info: ClientInfo {
                client_app_id: "stub".into(),
                application: "stub".into(),
                version: "0.0.0".into(),
                os: "linux".into(),
                os_version: "0".into(),
                ocsp_mode: None,
                runtime_name: None,
                runtime_version: None,
                compiler: None,
                release_type: None,
                crl_config: CrlConfig::default(),
                tls_config: TlsConfig::default(),
                proxy_config: Default::default(),
                platforms: Vec::new(),
                os_details: None,
            },
            log_max_query_length: 100,
            log_query_text: false,
            log_query_parameters: false,
        },
        conn: Arc::new(Mutex::new(Connection::new())),
    }
}

/// A real [`SnowflakeStageInfoRefresher`] whose GS fetch is stubbed to store
/// `on_fetch` and count invocations. Returns `(refresher, fetch_call_counter)`.
///
/// Exposed `pub(crate)` (test-only) so `file_manager::azure_transfer` tests can
/// drive the REAL single-flight coordinator — proving N concurrent block 403s
/// collapse into exactly one GS fetch — without rebuilding a
/// `StageInfoRefreshContext` or reaching the private coordinator type directly.
#[cfg(test)]
pub(crate) fn test_counting_coordinator(
    initial: file_manager::StageInfoSnapshot,
    on_fetch: file_manager::StageInfoSnapshot,
) -> (
    impl file_manager::StageInfoRefresher,
    std::sync::Arc<std::sync::atomic::AtomicU32>,
) {
    use std::sync::atomic::Ordering;
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let counter_in = counter.clone();
    let refresher =
        SnowflakeStageInfoRefresher::new_with_fetch_fn(stub_ctx(), initial, move |_ctx, cache| {
            let counter = counter_in.clone();
            let on_fetch = on_fetch.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                // Brief simulated GS round-trip: hold the cache on the stale
                // snapshot long enough that all concurrent callers observe it,
                // fail, and coalesce onto this single fetch before it stores the
                // fresh snapshot (otherwise a fast leader could rotate the cache
                // before a peer even reads it).
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                cache.store(on_fetch);
                Ok(cache.cached_at())
            }
            .boxed()
        });
    (refresher, counter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_column_metadata_has_correct_structure_python() {
        let columns = upload_column_metadata(&WrapperPresets::python());

        assert_eq!(columns.len(), 8, "PUT (Python) should have 8 columns");

        assert_eq!(columns[0].name, "source");
        assert_eq!(columns[0].r#type, "TEXT");
        assert!(!columns[0].nullable);

        assert_eq!(columns[1].name, "target");
        assert_eq!(columns[1].r#type, "TEXT");

        assert_eq!(columns[2].name, "source_size");
        assert_eq!(columns[2].r#type, "FIXED");
        assert_eq!(
            columns[2].precision,
            Some(PUT_GET_ROWSET_FIXED_LENGTH as i64)
        );
        assert_eq!(columns[2].scale, Some(0));

        assert_eq!(columns[3].name, "target_size");
        assert_eq!(columns[3].r#type, "FIXED");

        assert_eq!(columns[4].name, "source_compression");
        assert_eq!(columns[4].r#type, "TEXT");

        assert_eq!(columns[5].name, "target_compression");
        assert_eq!(columns[5].r#type, "TEXT");

        assert_eq!(columns[6].name, "status");
        assert_eq!(columns[6].r#type, "TEXT");

        assert_eq!(columns[7].name, "message");
        assert_eq!(columns[7].r#type, "TEXT");
    }

    #[test]
    fn upload_column_metadata_has_correct_structure_odbc() {
        let columns = upload_column_metadata(&WrapperPresets::odbc());

        assert_eq!(
            columns.len(),
            9,
            "PUT (ODBC) should have 9 columns including encryption"
        );

        assert_eq!(columns[0].name, "source");
        assert_eq!(columns[0].r#type, "TEXT");
        assert!(!columns[0].nullable);

        assert_eq!(columns[1].name, "target");
        assert_eq!(columns[1].r#type, "TEXT");

        assert_eq!(columns[2].name, "source_size");
        assert_eq!(columns[2].r#type, "FIXED");
        assert_eq!(
            columns[2].precision,
            Some(PUT_GET_ROWSET_FIXED_LENGTH as i64)
        );
        assert_eq!(columns[2].scale, Some(0));

        assert_eq!(columns[3].name, "target_size");
        assert_eq!(columns[3].r#type, "FIXED");

        assert_eq!(columns[4].name, "source_compression");
        assert_eq!(columns[4].r#type, "TEXT");

        assert_eq!(columns[5].name, "target_compression");
        assert_eq!(columns[5].r#type, "TEXT");

        assert_eq!(columns[6].name, "status");
        assert_eq!(columns[6].r#type, "TEXT");

        assert_eq!(columns[7].name, "encryption");
        assert_eq!(columns[7].r#type, "TEXT");

        assert_eq!(columns[8].name, "message");
        assert_eq!(columns[8].r#type, "TEXT");
    }

    #[test]
    fn download_column_metadata_has_correct_structure_python() {
        let columns = download_column_metadata(&WrapperPresets::python());

        assert_eq!(columns.len(), 4, "GET (Python) should have 4 columns");

        assert_eq!(columns[0].name, "file");
        assert_eq!(columns[0].r#type, "TEXT");
        assert!(!columns[0].nullable);

        assert_eq!(columns[1].name, "size");
        assert_eq!(columns[1].r#type, "FIXED");
        assert_eq!(
            columns[1].precision,
            Some(PUT_GET_ROWSET_FIXED_LENGTH as i64)
        );
        assert_eq!(columns[1].scale, Some(0));

        assert_eq!(columns[2].name, "status");
        assert_eq!(columns[2].r#type, "TEXT");

        assert_eq!(columns[3].name, "message");
        assert_eq!(columns[3].r#type, "TEXT");
    }

    #[test]
    fn download_column_metadata_has_correct_structure_odbc() {
        let columns = download_column_metadata(&WrapperPresets::odbc());

        assert_eq!(
            columns.len(),
            5,
            "GET (ODBC) should have 5 columns including encryption"
        );

        assert_eq!(columns[0].name, "file");
        assert_eq!(columns[0].r#type, "TEXT");
        assert!(!columns[0].nullable);

        assert_eq!(columns[1].name, "size");
        assert_eq!(columns[1].r#type, "FIXED");
        assert_eq!(
            columns[1].precision,
            Some(PUT_GET_ROWSET_FIXED_LENGTH as i64)
        );
        assert_eq!(columns[1].scale, Some(0));

        assert_eq!(columns[2].name, "status");
        assert_eq!(columns[2].r#type, "TEXT");

        assert_eq!(columns[3].name, "encryption");
        assert_eq!(columns[3].r#type, "TEXT");

        assert_eq!(columns[4].name, "message");
        assert_eq!(columns[4].r#type, "TEXT");
    }

    #[test]
    fn upload_column_metadata_has_correct_structure_jdbc() {
        // JDBC carries the `encryption` column like ODBC.
        let columns = upload_column_metadata(&WrapperPresets::jdbc());

        assert_eq!(
            columns.len(),
            9,
            "PUT (JDBC) should have 9 columns including encryption"
        );
        assert_eq!(columns[6].name, "status");
        assert_eq!(columns[7].name, "encryption");
        assert_eq!(columns[7].r#type, "TEXT");
        assert_eq!(columns[8].name, "message");
    }

    #[test]
    fn download_column_metadata_has_correct_structure_jdbc() {
        let columns = download_column_metadata(&WrapperPresets::jdbc());

        assert_eq!(
            columns.len(),
            5,
            "GET (JDBC) should have 5 columns including encryption"
        );
        assert_eq!(columns[2].name, "status");
        assert_eq!(columns[3].name, "encryption");
        assert_eq!(columns[3].r#type, "TEXT");
        assert_eq!(columns[4].name, "message");
    }

    #[test]
    fn text_column_metadata_has_correct_fields() {
        let rt = build_generic_text_rowtype("test_col");
        let meta = rowtype_to_column_metadata(&rt.0);

        assert_eq!(meta.name, "test_col");
        assert_eq!(meta.r#type, "TEXT");
        assert_eq!(meta.length, Some(PUT_GET_ROWSET_TEXT_LENGTH as i64));
        assert_eq!(meta.byte_length, Some(PUT_GET_ROWSET_TEXT_LENGTH as i64));
        assert_eq!(meta.precision, None);
        assert_eq!(meta.scale, None);
        assert!(!meta.nullable);

        assert_eq!(rt.1, DataType::Utf8);
    }

    #[test]
    fn fixed_column_metadata_has_correct_fields() {
        let rt = build_generic_fixed_rowtype("test_col");
        let meta = rowtype_to_column_metadata(&rt.0);

        assert_eq!(meta.name, "test_col");
        assert_eq!(meta.r#type, "FIXED");
        assert_eq!(meta.precision, Some(PUT_GET_ROWSET_FIXED_LENGTH as i64));
        assert_eq!(meta.scale, Some(0));
        assert_eq!(meta.length, None);
        assert_eq!(meta.byte_length, None);
        assert!(!meta.nullable);

        assert_eq!(rt.1, DataType::Int64);
    }

    // --- Stage-creds coalescing window ---
    //
    // The coalescing decision is extracted as `within_coalesce_window(last, now)`
    // so we can drive it with synthetic Instants instead of the real clock.
    // These tests pin the boundary at REFRESH_COALESCE_WINDOW (10 min) and
    // verify both edges.

    #[test]
    fn should_coalesce_returns_false_before_first_refresh() {
        let now = Instant::now();
        assert!(!within_coalesce_window(None, now));
    }

    #[test]
    fn should_coalesce_returns_true_inside_window() {
        let last = Instant::now();
        // Just inside the window — anything < REFRESH_COALESCE_WINDOW.
        let now = last + REFRESH_COALESCE_WINDOW - Duration::from_secs(1);
        assert!(within_coalesce_window(Some(last), now));
    }

    #[test]
    fn should_coalesce_returns_false_at_window_boundary() {
        // Exactly REFRESH_COALESCE_WINDOW elapsed should *not* coalesce —
        // it's strictly less-than. Belt-and-braces: if we ever change the
        // comparison, this catches it.
        let last = Instant::now();
        let now = last + REFRESH_COALESCE_WINDOW;
        assert!(!within_coalesce_window(Some(last), now));
    }

    #[test]
    fn should_coalesce_returns_false_past_window() {
        let last = Instant::now();
        let now = last + REFRESH_COALESCE_WINDOW + Duration::from_secs(1);
        assert!(!within_coalesce_window(Some(last), now));
    }

    #[test]
    fn should_coalesce_handles_clock_going_backwards() {
        // saturating_duration_since avoids panics if the system clock skews
        // backwards between the recorded last and now (paranoia for tests
        // that mint Instants by hand; in production Instants are monotonic).
        let last = Instant::now();
        let now = last - Duration::from_millis(0); // same instant
        assert!(within_coalesce_window(Some(last), now));
    }

    #[test]
    fn local_file_path_unquoted_glob() {
        assert_eq!(
            local_file_path_from_put_command("PUT file://data/*.csv @stage"),
            Some("data/*.csv")
        );
    }

    #[test]
    fn local_file_path_unquoted_trailing_options() {
        assert_eq!(
            local_file_path_from_put_command(
                "PUT file://data/*.csv @stage AUTO_COMPRESS=TRUE OVERWRITE=FALSE"
            ),
            Some("data/*.csv")
        );
    }

    #[test]
    fn local_file_path_unquoted_to_end_of_string() {
        assert_eq!(
            local_file_path_from_put_command("PUT file://data/only.csv"),
            Some("data/only.csv")
        );
    }

    #[test]
    fn local_file_path_quoted() {
        assert_eq!(
            local_file_path_from_put_command("PUT 'file://data dir/*.csv' @stage"),
            Some("data dir/*.csv")
        );
    }

    #[test]
    fn local_file_path_quoted_unterminated_is_none() {
        // A quote opened before file:// with no closing quote is malformed —
        // refuse rather than guess at the path boundary.
        assert_eq!(
            local_file_path_from_put_command("PUT 'file://data/*.csv @stage"),
            None
        );
    }

    #[test]
    fn local_file_path_newline_and_semicolon_terminators() {
        assert_eq!(
            local_file_path_from_put_command("PUT file://data/*.csv\n@stage"),
            Some("data/*.csv")
        );
        assert_eq!(
            local_file_path_from_put_command("PUT file://data/*.csv;"),
            Some("data/*.csv")
        );
    }

    #[test]
    fn local_file_path_none_for_get_command() {
        assert_eq!(
            local_file_path_from_put_command("GET @stage file:///tmp/out"),
            Some("/tmp/out"),
            "GET also carries file://; the refresher only rewrites when an upload file is set"
        );
        assert_eq!(local_file_path_from_put_command("GET @stage"), None);
    }

    #[test]
    fn rewrite_put_command_replaces_glob_with_dst_name() {
        assert_eq!(
            rewrite_put_command_for_file("PUT file://data/*.csv @stage", "part-01.csv.gz"),
            Some("PUT file://part-01.csv.gz @stage".to_string()),
            "local path token is replaced with the dst name; file:// prefix is kept"
        );
    }

    #[test]
    fn rewrite_put_command_quoted_keeps_quotes() {
        assert_eq!(
            rewrite_put_command_for_file("PUT 'file://data dir/*.csv' @stage", "part-01.csv.gz"),
            Some("PUT 'file://part-01.csv.gz' @stage".to_string())
        );
    }

    #[test]
    fn rewrite_put_command_none_when_no_file_protocol() {
        assert_eq!(
            rewrite_put_command_for_file("GET @stage", "part-01.csv.gz"),
            None
        );
    }

    // -------------------------------------------------------------------
    // Single-flight coordinator concurrency tests.
    // These use `new_with_fetch_fn` to inject a fake fetch so no real
    // GS connection is needed.
    // -------------------------------------------------------------------

    fn stub_snapshot() -> file_manager::StageInfoSnapshot {
        file_manager::StageInfoSnapshot {
            creds: file_manager::CloudCredentials::Gcs {
                gcs_access_token: None,
            },
            presigned_url: None,
            presigned_urls: None,
        }
    }

    /// N concurrent callers that all observe the same `observed` generation
    /// must each receive the same `Ok(new_gen)` result — and the fetch function
    /// must be invoked exactly once (single-flight).
    #[tokio::test(flavor = "multi_thread")]
    async fn should_coalesce_n_concurrent_refresh_callers_into_one_fetch() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let fetch_calls = Arc::new(AtomicU32::new(0));
        let fetch_calls2 = fetch_calls.clone();

        let refresher = Arc::new(SnowflakeStageInfoRefresher::new_with_fetch_fn(
            stub_ctx(),
            stub_snapshot(),
            move |_ctx, cache| {
                let calls = fetch_calls2.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    // Simulate a brief async fetch.
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    cache.store(file_manager::StageInfoSnapshot {
                        creds: file_manager::CloudCredentials::Gcs {
                            gcs_access_token: None,
                        },
                        presigned_url: None,
                        presigned_urls: None,
                    });
                    Ok(cache.cached_at())
                }
                .boxed()
            },
        ));

        let observed = Instant::now();
        // Spawn 5 concurrent callers, all observing the same `observed`.
        let handles: Vec<_> = (0..5)
            .map(|_| {
                let r = refresher.clone();
                tokio::spawn(async move {
                    file_manager::StageInfoRefresher::refresh(r.as_ref(), observed).await
                })
            })
            .collect();

        let results: Vec<_> = futures::future::join_all(handles).await;
        let gens: Vec<Instant> = results
            .into_iter()
            .map(|h| h.expect("task panicked").expect("refresh failed"))
            .collect();

        // All callers must receive the same generation.
        let first = gens[0];
        for g in &gens {
            assert_eq!(
                *g, first,
                "all concurrent callers must share the same generation"
            );
        }
        // The fetch must have been called exactly once.
        assert_eq!(
            fetch_calls.load(Ordering::SeqCst),
            1,
            "single-flight: fetch must fire exactly once for N concurrent callers"
        );
        // The returned generation must be strictly after observed.
        assert!(
            first > observed,
            "new generation must be strictly after observed"
        );
    }

    /// A second `refresh()` call that arrives after the first completes and
    /// within the 10-min coalescing window must return the cached generation
    /// without re-fetching (terminal within-window coalesce).
    #[tokio::test]
    async fn should_coalesce_sequential_refresh_within_window() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let fetch_calls = Arc::new(AtomicU32::new(0));
        let fetch_calls2 = fetch_calls.clone();

        let refresher = SnowflakeStageInfoRefresher::new_with_fetch_fn(
            stub_ctx(),
            stub_snapshot(),
            move |_ctx, cache| {
                let calls = fetch_calls2.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    cache.store(file_manager::StageInfoSnapshot {
                        creds: file_manager::CloudCredentials::Gcs {
                            gcs_access_token: None,
                        },
                        presigned_url: None,
                        presigned_urls: None,
                    });
                    Ok(cache.cached_at())
                }
                .boxed()
            },
        );

        let observed1 = Instant::now();
        // First call — goes through to the fetch.
        let new_gen1 = file_manager::StageInfoRefresher::refresh(&refresher, observed1)
            .await
            .expect("first refresh failed");
        assert!(
            new_gen1 > observed1,
            "first refresh must advance generation"
        );
        assert_eq!(fetch_calls.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Second call with observed = new_gen1 — coalesced because last_refresh_at
        // is now set and within the window. Pass new_gen1 (not observed1) so the
        // cache's cached_at equals observed rather than exceeding it; the fast path
        // would short-circuit if we passed observed1 (cache already > observed1).
        let new_gen2 = file_manager::StageInfoRefresher::refresh(&refresher, new_gen1)
            .await
            .expect("second refresh failed");
        assert_eq!(
            new_gen2, new_gen1,
            "within-window sequential call must return the cached generation unchanged (coalesced)"
        );
        assert_eq!(
            fetch_calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "no second fetch within the coalescing window"
        );
    }

    /// A fetch that fails must propagate the error to all concurrent waiters.
    #[tokio::test(flavor = "multi_thread")]
    async fn should_propagate_fetch_failure_to_all_concurrent_waiters() {
        let refresher = Arc::new(SnowflakeStageInfoRefresher::new_with_fetch_fn(
            stub_ctx(),
            stub_snapshot(),
            |_ctx, _cache| {
                async move {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    Err(StageInfoRefreshError::ServerRejected {
                        message: "injected failure".into(),
                        location: snafu::Location::default(),
                    })
                }
                .boxed()
            },
        ));

        let observed = Instant::now();
        let handles: Vec<_> = (0..3)
            .map(|_| {
                let r = refresher.clone();
                tokio::spawn(async move {
                    file_manager::StageInfoRefresher::refresh(r.as_ref(), observed).await
                })
            })
            .collect();

        let results: Vec<_> = futures::future::join_all(handles).await;
        for result in results {
            let err = result
                .expect("task panicked")
                .expect_err("fetch failure must propagate");
            assert!(
                matches!(err, StageInfoRefreshError::ServerRejected { .. }),
                "all waiters must receive the error variant, got: {err:?}"
            );
        }
    }

    /// A failed refresh must NOT stamp the coalesce window: the error
    /// propagates, and a subsequent `refresh()` performs a fresh fetch (does
    /// not coalesce onto the failed attempt) and can succeed.
    #[tokio::test]
    async fn should_refetch_after_failed_refresh_without_coalescing() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let fetch_calls = Arc::new(AtomicU32::new(0));
        let fetch_calls2 = fetch_calls.clone();

        let refresher = SnowflakeStageInfoRefresher::new_with_fetch_fn(
            stub_ctx(),
            stub_snapshot(),
            move |_ctx, cache| {
                let calls = fetch_calls2.clone();
                async move {
                    // First attempt fails; every later attempt succeeds.
                    if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                        return Err(StageInfoRefreshError::ServerRejected {
                            message: "injected first-attempt failure".into(),
                            location: snafu::Location::default(),
                        });
                    }
                    cache.store(file_manager::StageInfoSnapshot {
                        creds: file_manager::CloudCredentials::Gcs {
                            gcs_access_token: None,
                        },
                        presigned_url: None,
                        presigned_urls: None,
                    });
                    Ok(cache.cached_at())
                }
                .boxed()
            },
        );

        let observed = Instant::now();

        // First refresh fails and surfaces the error.
        let err = file_manager::StageInfoRefresher::refresh(&refresher, observed)
            .await
            .expect_err("first refresh must surface the injected failure");
        assert!(
            matches!(err, StageInfoRefreshError::ServerRejected { .. }),
            "first refresh must surface the injected failure, got: {err:?}"
        );

        // Second refresh must re-fetch (the failed attempt left the window
        // unstamped) and succeed.
        let new_gen = file_manager::StageInfoRefresher::refresh(&refresher, observed)
            .await
            .expect("second refresh must re-fetch and succeed");
        assert!(
            new_gen > observed,
            "successful re-fetch must advance the generation"
        );
        assert_eq!(
            fetch_calls.load(Ordering::SeqCst),
            2,
            "failed refresh must not stamp the window; the second call must re-fetch"
        );
    }
}
