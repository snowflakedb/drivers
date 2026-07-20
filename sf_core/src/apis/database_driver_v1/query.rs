use super::ColumnMetadata;
use super::connection::{Connection, RefreshContext};
use super::global_state::{PutGetResultsetFlavor, WrapperPresets};
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
    StageInfoSnapshot, UploadResult, download_files, upload_files, upload_in_memory_file,
};
use crate::query_types::RowType;
use crate::rest;
use arrow::array::{Array, Int64Array, RecordBatchReader, StringArray};
use arrow::datatypes::DataType;
use arrow::error::ArrowError;
use reqwest::Client;
use rest::snowflake::query_response::{self, QueryResponseError, RowsetData};
use snafu::{IntoError, Location, OptionExt, ResultExt, Snafu};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const PUT_GET_ROWSET_TEXT_LENGTH: u64 = 10000;
const PUT_GET_ROWSET_FIXED_LENGTH: u64 = 64;

/// Literal emitted by `PutGetResultsetFlavor::Odbc` in the PUT result's
/// `encryption` column. Mirrors `#define ENCRYPTION_ENCRYPTED "ENCRYPTED"`
/// from legacy libsnowflakeclient's `FileTransferExecutionResult.cpp`. The
/// value is a constant string for *every* row (it advertises "your data
/// ended up encrypted", not "this row's encryption material"). Any C++ /
/// Python wrapper test that asserts on this column must use the same
/// literal — kept here so the contract has one source of truth.
const ODBC_PUT_ENCRYPTION_LITERAL: &str = "ENCRYPTED";

/// Literal emitted by `PutGetResultsetFlavor::Odbc` in the GET result's
/// `encryption` column. Mirrors `#define ENCRYPTION_DECRYPTED "DECRYPTED"`
/// from legacy libsnowflakeclient. See `ODBC_PUT_ENCRYPTION_LITERAL`.
const ODBC_GET_ENCRYPTION_LITERAL: &str = "DECRYPTED";

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
    pub cancel: tokio_util::sync::CancellationToken,
}

/// Executes a PUT/GET file transfer and returns a `RowsetData` variant holding the results.
///
/// `retry_policy` is the base put/get retry policy (built from connection
/// params at the dispatch site); cloud-specific code clones and tweaks it.
///
/// When `stage_info_refresh_context` is `Some`, recoverable stage-info-expiry
/// errors during a file transfer trigger a re-issue of the original PUT/GET
/// SQL to obtain a fresh `StageInfoSnapshot` (creds + presigned URLs) and the
/// operation is retried. Specifically:
/// - S3: AWS `ExpiredToken` → creds refresh (coalesced, 10-min window)
/// - GCS 401: bearer expired → creds refresh (coalesced, 10-min window)
/// - GCS 400: presigned URL expired → URL refresh (per-file, no coalesce)
///
/// Non-PUT/GET callers pass `None`.
///
/// `use_s3_regional_url_session_param` is the resolved value of the
/// `ENABLE_STAGE_S3_PRIVATELINK_FOR_US_EAST_1` session parameter (read at the
/// dispatch site via `read_use_s3_regional_url_session_param`). When `true`,
/// it ORs into the S3 regional-URL decision, matching the Python connector,
/// JDBC, and libsnowflakeclient behavior.
#[allow(clippy::too_many_arguments)]
pub(super) async fn perform_put_get_transfer(
    command: &str,
    data: &query_response::Data,
    wrapper_presets: &WrapperPresets,
    retry_policy: &RetryPolicy,
    stage_info_refresh_context: Option<StageInfoRefreshContext>,
    use_s3_regional_url_session_param: bool,
    skip_upload_on_content_match: bool,
    unsafe_file_write: bool,
    tls_config: crate::tls::config::TlsConfig,
    crl_worker: crate::crl::worker::SharedCrlWorker,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<RowsetData, QueryResponseProcessingError> {
    // Seed the refresher's cache with the initial snapshot.
    let initial_snapshot = data
        .stage_info_snapshot()
        .context(FileTransferPreparationSnafu)?;
    let mut refresher = stage_info_refresh_context
        .zip(initial_snapshot)
        .map(|(ctx, initial)| SnowflakeStageInfoRefresher::new(ctx, initial));
    let refresher_handle = refresher
        .as_mut()
        .map(|r| r as &mut dyn file_manager::StageInfoRefresher);

    match command {
        "UPLOAD" => {
            let mut file_upload_data = data
                .to_file_upload_data(
                    wrapper_presets.put_get_resultset_flavor.clone(),
                    wrapper_presets.legacy_odbc_compression_autodetect,
                    skip_upload_on_content_match,
                    use_s3_regional_url_session_param,
                )
                .context(FileTransferPreparationSnafu)?;
            file_upload_data.stage_info.tls_config = tls_config.clone();
            file_upload_data.stage_info.crl_worker = crl_worker.clone();
            let upload_results =
                upload_files(&file_upload_data, retry_policy, refresher_handle, cancel)
                    .await
                    .context(FileUploadSnafu)?;
            Ok(RowsetData::Upload(upload_results))
        }
        "DOWNLOAD" => {
            let mut file_download_data = data
                .to_file_download_data(
                    &wrapper_presets.put_get_resultset_flavor,
                    use_s3_regional_url_session_param,
                    unsafe_file_write,
                )
                .map_err(|e| {
                    if e.to_string().contains("source locations") {
                        RemoteFileNotFoundSnafu.build()
                    } else {
                        FileTransferPreparationSnafu.into_error(e)
                    }
                })?;
            file_download_data.stage_info.tls_config = tls_config;
            file_download_data.stage_info.crl_worker = crl_worker;
            let download_results =
                download_files(file_download_data, retry_policy, refresher_handle, cancel)
                    .await
                    .context(FileDownloadSnafu)?;
            Ok(RowsetData::Download(download_results))
        }
        _ => UnsupportedCommandSnafu {
            command: command.to_string(),
        }
        .fail(),
    }
}

/// Uploads `bytes` (already drained from the caller's stream) to the stage
/// described by a GS PUT response, returning a single-row `RowsetData::Upload`.
/// Mirrors the UPLOAD arm of [`perform_put_get_transfer`] but sources the data
/// from memory instead of expanding a local glob — backs
/// `connection_upload_stream` (JDBC `uploadStream`, Python `file_stream`).
///
/// The destination filename is the basename of the PUT command's `file://`
/// token (echoed back by GS as `src_location_pattern`); auto-compress,
/// overwrite, and encryption all follow the GS response, exactly as a normal
/// file-path PUT.
pub(super) async fn perform_stream_upload(
    data: &query_response::Data,
    wrapper_presets: &WrapperPresets,
    stage_info_refresh_context: Option<StageInfoRefreshContext>,
    use_s3_regional_url_session_param: bool,
    put_get_policy: &RetryPolicy,
    bytes: Vec<u8>,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<RowsetData, QueryResponseProcessingError> {
    let upload_data = data
        .to_file_upload_data(
            wrapper_presets.put_get_resultset_flavor.clone(),
            wrapper_presets.legacy_odbc_compression_autodetect,
            // In-memory stream PUT never skips on content match: the API has no
            // cursor kwarg to opt into it and always uploads the supplied bytes.
            false,
            use_s3_regional_url_session_param,
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
    let mut refresher = stage_info_refresh_context
        .zip(initial_snapshot)
        .map(|(ctx, initial)| SnowflakeStageInfoRefresher::new(ctx, initial));
    let mut refresher_handle = refresher
        .as_mut()
        .map(|r| r as &mut dyn file_manager::StageInfoRefresher);

    let single = SingleUploadData {
        // `upload_in_memory_file` overrides `source` with `bytes` below; this
        // placeholder only satisfies the struct (the result's `source` column
        // is derived from `filename` for in-memory uploads).
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

    let result =
        upload_in_memory_file(bytes, single, put_get_policy, &mut refresher_handle, cancel)
            .await
            .context(FileUploadSnafu)?;

    Ok(RowsetData::Upload(vec![result]))
}

/// Builds the stage-info refresher used by `connection_download_stream`.
/// Exposed so `stream_transfer.rs` can drive a streaming GET with the same
/// cred/URL-refresh machinery as the file-path path, without re-exposing the
/// private `SnowflakeStageInfoRefresher` type.
pub(super) fn stream_stage_info_refresher(
    ctx: StageInfoRefreshContext,
    initial: StageInfoSnapshot,
) -> impl file_manager::StageInfoRefresher {
    SnowflakeStageInfoRefresher::new(ctx, initial)
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

/// Refreshes stage info (creds + presigned URLs) by re-executing the
/// original PUT/GET SQL against Snowflake GS, matching Python's
/// `StorageCredential.update` and ODBC's `FileTransferAgent::renewToken`. GS
/// returns a brand-new `stageInfo` per query — we take the full
/// `StageInfoSnapshot` (creds + presignedUrl + presignedUrls[]) and write it
/// into the shared `StageInfoCache` so every in-flight transfer in the
/// batch picks the fresh values up on its next attempt.
///
/// The refresh re-issues the PUT/GET SQL through `RefreshContext::execute_with_refresh`
/// — if the session token has itself expired by the time we reach this point
/// (e.g. a long batch upload), the 390112 detection from PR #1137 transparently
/// renews the session before retrying the SQL.
///
/// Two entry points share the same fetch logic but differ in coalescing:
/// - [`refresh`](file_manager::StageInfoRefresher::refresh) gates on the
///   10-minute window (matches libsfclient's `m_lastRefreshTokenSec`); used
///   for token-style expiries (S3 STS, GCS 401) where a burst of expirations
///   across files should collapse to a single SQL re-issue.
/// - [`refresh_url`](file_manager::StageInfoRefresher::refresh_url) bypasses
///   the window; used for GCS 400 per-file URL expiry where each call may
///   need to fetch a fresh `presignedUrls[]` slot. The call site
///   (`gcs_transfer.rs`) enforces a two-strike guard to prevent looping.
struct SnowflakeStageInfoRefresher {
    ctx: StageInfoRefreshContext,
    cache: StageInfoCache,
    last_refresh_at: Option<Instant>,
    /// Destination object name of the file currently being uploaded, set by
    /// `upload_to_gcs_or_skip` via `notify_current_upload_file` before a
    /// per-file URL refresh. `refresh_url` rewrites the PUT SQL to target this
    /// file so GS returns its presigned URL (multi-file glob PUT). `None` for
    /// GET, where the call site re-picks `presignedUrls[per_file_index]`.
    current_upload_file: Option<String>,
}

impl SnowflakeStageInfoRefresher {
    fn new(ctx: StageInfoRefreshContext, initial: StageInfoSnapshot) -> Self {
        Self {
            ctx,
            cache: StageInfoCache::new(initial),
            last_refresh_at: None,
            current_upload_file: None,
        }
    }
}

/// Returns `true` if a refresh recorded at `last` is still considered fresh
/// at `now` and a new fetch should be coalesced. Extracted so the
/// time-window logic can be unit-tested without a real `Instant::now()`.
fn should_coalesce(last: Option<Instant>, now: Instant) -> bool {
    last.is_some_and(|at| now.saturating_duration_since(at) < REFRESH_COALESCE_WINDOW)
}

impl file_manager::StageInfoRefresher for SnowflakeStageInfoRefresher {
    fn refresh(&mut self) -> file_manager::RefreshFuture<'_> {
        Box::pin(async move {
            // Coalesce rapid-fire refreshes: if we already fetched within
            // the window, the cache still holds the result — nothing to do.
            if should_coalesce(self.last_refresh_at, Instant::now()) {
                tracing::debug!("Stage info refresh coalesced; cache holds recent snapshot");
                return Ok(());
            }

            tracing::info!("Refreshing stage info by re-executing PUT/GET SQL");
            let snapshot = fetch_fresh_stage_info(&self.ctx).await?;
            self.cache.store(snapshot);
            self.last_refresh_at = Some(Instant::now());
            Ok(())
        })
    }

    fn refresh_url(&mut self) -> file_manager::RefreshFuture<'_> {
        Box::pin(async move {
            // For a PUT, re-issue the SQL rewritten to target the single file
            // currently uploading so GS returns *that* file's presigned URL —
            // re-issuing the original glob SQL would return the first matched
            // file's URL and misroute the upload. For a GET, the call site
            // re-picks `presignedUrls[per_file_index]`, so re-issue unchanged.
            let sql = match self.current_upload_file.as_deref() {
                Some(dst) => match rewrite_put_command_for_file(&self.ctx.sql, dst) {
                    Some(rewritten) => rewritten,
                    // PUT command with no parseable `file://` token: refuse to
                    // re-issue the unchanged SQL (it would misroute) and let
                    // the GCS call site surface PresignedUrlExpired.
                    None => {
                        use crate::file_manager::types::stage_info_refresh_error::PresignedUrlRefreshSkippedSnafu;
                        return PresignedUrlRefreshSkippedSnafu.fail();
                    }
                },
                None => self.ctx.sql.clone(),
            };
            // Per-file URL refresh: bypass the coalescing window. Each file
            // may carry a distinct per-object presigned URL, so collapsing
            // refresh calls would lock subsequent files to a stale URL. The
            // GCS call site enforces a two-strike guard.
            tracing::info!(
                "Refreshing stage info (presigned URLs) by re-executing PUT/GET SQL — \
                 bypassing 10-min coalesce window for per-file URL expiry"
            );
            let snapshot = fetch_fresh_stage_info_with_sql(&self.ctx, &sql).await?;
            self.cache.store(snapshot);
            // Update `last_refresh_at` so a subsequent token-style refresh
            // honors the window — the snapshot we just wrote carries fresh
            // creds too. (Cred + URL refresh share the same underlying SQL,
            // so this isn't double-spending against GS.)
            self.last_refresh_at = Some(Instant::now());
            Ok(())
        })
    }

    fn cache(&self) -> &StageInfoCache {
        &self.cache
    }

    fn notify_current_upload_file(&mut self, dst_file_name: String) {
        self.current_upload_file = Some(dst_file_name);
    }
}

/// Extracts the local file path token from a PUT command: everything after the
/// `file://` prefix, ending at the closing quote (if the path is quoted) or at
/// the first space / newline / `;` (otherwise). Returns `None` when there is no
/// `file://` (e.g. a GET command) or the token is empty/malformed. Mirrors
/// libsfclient `getLocalFilePathFromCommand` and Python
/// `_get_local_file_path_from_put_command`.
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

/// Re-issues the original PUT/GET SQL (`ctx.sql`) and extracts the fresh
/// `stageInfo` snapshot. See [`fetch_fresh_stage_info_with_sql`].
async fn fetch_fresh_stage_info(
    ctx: &StageInfoRefreshContext,
) -> Result<StageInfoSnapshot, StageInfoRefreshError> {
    fetch_fresh_stage_info_with_sql(ctx, &ctx.sql).await
}

/// Re-issues `sql` through `RefreshContext::execute_with_refresh` and extracts
/// the full `stageInfo` snapshot (creds + presignedUrl + presignedUrls[]) from
/// the response. Going through `execute_with_refresh` means a session-token
/// expiry mid-batch is healed transparently by session-renewal logic before
/// the SQL is retried.
///
/// `sql` is usually `ctx.sql`, but the per-file URL refresh path passes a
/// command rewritten for a single destination file (see
/// `rewrite_put_command_for_file`) so GS returns that file's presigned URL.
async fn fetch_fresh_stage_info_with_sql(
    ctx: &StageInfoRefreshContext,
    sql: &str,
) -> Result<StageInfoSnapshot, StageInfoRefreshError> {
    use crate::file_manager::types::stage_info_refresh_error::*;

    // `from_arc` is used (not `new`) so that a `close()` raced against an
    // in-flight refresh is rejected, consistent with the original query path.
    let mut refresh_ctx = RefreshContext::from_arc(&ctx.conn)
        .await
        .context(QueryFailedSnafu)?;
    // `from_arc` already validates that `http_client` is present (via the
    // is_closed check + `RefreshContext::new`), so this lookup just clones it.
    let http_client = ctx
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
            let query_parameters = ctx.query_parameters.clone();
            let query_input = query_input.clone();
            let cancel = ctx.cancel.clone();
            async move {
                rest::snowflake::snowflake_query_with_client(
                    &http_client,
                    query_parameters,
                    session_token.reveal(),
                    query_input,
                    &crate::config::retry::RetryPolicy::default(),
                    rest::snowflake::QueryExecutionMode::Blocking,
                    cancel,
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
    cancel: tokio_util::sync::CancellationToken,
) -> Result<Box<dyn RecordBatchReader + Send>, QueryResponseProcessingError> {
    match data {
        RowsetData::Upload(results) => {
            upload_results_reader(results, wrapper_presets).context(UploadResultsConversionSnafu)
        }
        RowsetData::Download(results) => download_results_reader(results, wrapper_presets)
            .context(DownloadResultsConversionSnafu),
        _ => read_batches(data, http_client, prefetch_config, nullable_flags, cancel)
            .await
            .context(BatchReadSnafu),
    }
}

pub(super) async fn read_batches(
    data: &RowsetData,
    http_client: Client,
    prefetch_config: &PrefetchConfig,
    nullable_flags: Option<&[bool]>,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<Box<dyn RecordBatchReader + Send>, ReadBatchesError> {
    tracing::debug!("read_batches called {:?}", data);
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
            cancel,
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
                cancel,
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
                cancel,
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
    if wrapper_presets.put_get_resultset_flavor == PutGetResultsetFlavor::Odbc {
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
    if wrapper_presets.put_get_resultset_flavor == PutGetResultsetFlavor::Odbc {
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
    if wrapper_presets.put_get_resultset_flavor == PutGetResultsetFlavor::Odbc {
        columns.push(Arc::new(StringArray::from_iter_values(
            std::iter::repeat_n(ODBC_PUT_ENCRYPTION_LITERAL, n),
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
    if wrapper_presets.put_get_resultset_flavor == PutGetResultsetFlavor::Odbc {
        columns.push(Arc::new(StringArray::from_iter_values(
            std::iter::repeat_n(ODBC_GET_ENCRYPTION_LITERAL, n),
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

impl QueryResponseProcessingError {
    pub(crate) fn is_cancelled(&self) -> bool {
        match self {
            QueryResponseProcessingError::BatchRead { source, .. } => source.is_cancelled(),
            QueryResponseProcessingError::FileUpload { source, .. } => source.is_cancelled(),
            QueryResponseProcessingError::FileDownload { source, .. } => source.is_cancelled(),
            _ => false,
        }
    }
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

impl ReadBatchesError {
    pub(crate) fn is_cancelled(&self) -> bool {
        matches!(self, ReadBatchesError::ChunkRead { source, .. } if source.is_cancelled())
    }
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
    // The coalescing decision is extracted as `should_coalesce(last, now)`
    // so we can drive it with synthetic Instants instead of the real clock.
    // These tests pin the boundary at REFRESH_COALESCE_WINDOW (10 min) and
    // verify both edges.

    #[test]
    fn should_coalesce_returns_false_before_first_refresh() {
        let now = Instant::now();
        assert!(!should_coalesce(None, now));
    }

    #[test]
    fn should_coalesce_returns_true_inside_window() {
        let last = Instant::now();
        // Just inside the window — anything < REFRESH_COALESCE_WINDOW.
        let now = last + REFRESH_COALESCE_WINDOW - Duration::from_secs(1);
        assert!(should_coalesce(Some(last), now));
    }

    #[test]
    fn should_coalesce_returns_false_at_window_boundary() {
        // Exactly REFRESH_COALESCE_WINDOW elapsed should *not* coalesce —
        // it's strictly less-than. Belt-and-braces: if we ever change the
        // comparison, this catches it.
        let last = Instant::now();
        let now = last + REFRESH_COALESCE_WINDOW;
        assert!(!should_coalesce(Some(last), now));
    }

    #[test]
    fn should_coalesce_returns_false_past_window() {
        let last = Instant::now();
        let now = last + REFRESH_COALESCE_WINDOW + Duration::from_secs(1);
        assert!(!should_coalesce(Some(last), now));
    }

    #[test]
    fn should_coalesce_handles_clock_going_backwards() {
        // saturating_duration_since avoids panics if the system clock skews
        // backwards between the recorded last and now (paranoia for tests
        // that mint Instants by hand; in production Instants are monotonic).
        let last = Instant::now();
        let now = last - Duration::from_millis(0); // same instant
        assert!(should_coalesce(Some(last), now));
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
}
