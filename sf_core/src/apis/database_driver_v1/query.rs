use super::ColumnMetadata;
use super::connection::{Connection, RefreshContext};
use super::global_state::{PutGetResultsetFlavor, WrapperPresets};
use crate::arrow_utils::ArrowUtilsError;
use crate::arrow_utils::{boxed_arrow_reader, create_schema};
use crate::chunks::{
    ChunkError, PrefetchConfig, arrow_prefetch_reader, empty_reader, json_prefetch_reader,
    schema_only_reader, single_chunk_reader,
};
use crate::file_manager;
use crate::file_manager::{
    CloudCredentials, DownloadResult, StageCredsCache, StageCredsRefreshError, UploadResult,
    download_files, upload_files,
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
pub struct StageCredsRefreshContext {
    pub sql: String,
    pub query_parameters: crate::config::rest_parameters::QueryParameters,
    pub conn: Arc<Mutex<Connection>>,
}

/// Executes a PUT/GET file transfer and returns a `RowsetData` variant holding the results.
///
/// When `stage_creds_refresh_context` is `Some`, an S3 `ExpiredToken` during a
/// file transfer triggers a re-issue of the original PUT/GET SQL to obtain fresh
/// STS credentials and the operation is retried. Non-PUT/GET callers pass `None`.
///
/// `use_s3_regional_url_session_param` is the resolved value of the
/// `ENABLE_STAGE_S3_PRIVATELINK_FOR_US_EAST_1` session parameter (read at the
/// dispatch site via `read_use_s3_regional_url_session_param`). When `true`,
/// it ORs into the S3 regional-URL decision, matching the Python connector,
/// JDBC, and libsnowflakeclient behavior.
pub(super) async fn perform_put_get_transfer(
    command: &str,
    data: &query_response::Data,
    wrapper_presets: &WrapperPresets,
    stage_creds_refresh_context: Option<StageCredsRefreshContext>,
    use_s3_regional_url_session_param: bool,
) -> Result<RowsetData, QueryResponseProcessingError> {
    // Seed the refresher's cache with the initial creds.
    let initial_creds = data
        .stage_info_creds()
        .context(FileTransferPreparationSnafu)?;
    let mut refresher = stage_creds_refresh_context
        .zip(initial_creds)
        .map(|(ctx, initial_creds)| SnowflakeStageCredsRefresher::new(ctx, initial_creds));
    let refresher_handle = refresher
        .as_mut()
        .map(|r| r as &mut dyn file_manager::StageCredsRefresher);

    match command {
        "UPLOAD" => {
            let file_upload_data = data
                .to_file_upload_data(
                    wrapper_presets.put_get_resultset_flavor.clone(),
                    wrapper_presets.legacy_odbc_compression_autodetect,
                    use_s3_regional_url_session_param,
                )
                .context(FileTransferPreparationSnafu)?;
            let upload_results = upload_files(&file_upload_data, refresher_handle)
                .await
                .context(FileUploadSnafu)?;
            Ok(RowsetData::Upload(upload_results))
        }
        "DOWNLOAD" => {
            let file_download_data = data
                .to_file_download_data(
                    &wrapper_presets.put_get_resultset_flavor,
                    use_s3_regional_url_session_param,
                )
                .map_err(|e| {
                    if e.to_string().contains("source locations") {
                        RemoteFileNotFoundSnafu.build()
                    } else {
                        FileTransferPreparationSnafu.into_error(e)
                    }
                })?;
            let download_results = download_files(file_download_data, refresher_handle)
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

/// Window during which repeated `refresh()` calls return without hitting GS.
/// Matches ODBC's `FileTransferAgent.cpp` `m_lastRefreshTokenSec` gate (10
/// minutes), which coalesces rapid-fire refreshes from concurrent uploads.
const REFRESH_COALESCE_WINDOW: Duration = Duration::from_secs(10 * 60);

/// Refreshes stage credentials by re-executing the original PUT/GET SQL
/// against Snowflake GS, matching Python's `StorageCredential.update` and
/// ODBC's `FileTransferAgent::renewToken`. GS returns a brand-new `stageInfo`
/// per query, so we take only the creds and leave bucket/region/key_prefix
/// untouched. The new creds land in the shared `StageCredsCache` so every
/// in-flight transfer in the batch picks them up on its next attempt.
///
/// The refresh re-issues the PUT/GET SQL through `RefreshContext::execute_with_refresh`
/// — if the session token has itself expired by the time we reach this point
/// (e.g. a long batch upload), the 390112 detection from PR #1137 transparently
/// renews the session before retrying the SQL.
///
/// A 10-minute coalescing window short-circuits subsequent refresh calls
/// without re-issuing the SQL, keeping us well-behaved against a burst of
/// `ExpiredToken` responses (long batch upload, concurrent parts in a future
/// parallel implementation) without either capping retries artificially or
/// hammering GS.
struct SnowflakeStageCredsRefresher {
    ctx: StageCredsRefreshContext,
    cache: StageCredsCache,
    last_refresh_at: Option<Instant>,
}

impl SnowflakeStageCredsRefresher {
    fn new(ctx: StageCredsRefreshContext, initial_creds: CloudCredentials) -> Self {
        Self {
            ctx,
            cache: StageCredsCache::new(initial_creds),
            last_refresh_at: None,
        }
    }
}

/// Returns `true` if a refresh recorded at `last` is still considered fresh
/// at `now` and a new fetch should be coalesced. Extracted so the
/// time-window logic can be unit-tested without a real `Instant::now()`.
fn should_coalesce(last: Option<Instant>, now: Instant) -> bool {
    last.is_some_and(|at| now.saturating_duration_since(at) < REFRESH_COALESCE_WINDOW)
}

impl file_manager::StageCredsRefresher for SnowflakeStageCredsRefresher {
    fn refresh(&mut self) -> file_manager::RefreshFuture<'_> {
        Box::pin(async move {
            // Coalesce rapid-fire refreshes: if we already fetched creds
            // within the window, the cache still holds them — nothing to do.
            if should_coalesce(self.last_refresh_at, Instant::now()) {
                tracing::debug!("Stage creds refresh coalesced; cache holds recent creds");
                return Ok(());
            }

            tracing::info!("Refreshing stage credentials by re-executing PUT/GET SQL");
            let creds = fetch_fresh_stage_creds(&self.ctx).await?;
            self.cache.store(creds);
            self.last_refresh_at = Some(Instant::now());
            Ok(())
        })
    }

    fn cache(&self) -> &StageCredsCache {
        &self.cache
    }
}

/// Re-issues the original PUT/GET SQL through `RefreshContext::execute_with_refresh`
/// and extracts the fresh `stageInfo.creds` from the response. Going through
/// `execute_with_refresh` means a session-token expiry mid-batch is healed
/// transparently by PR #1137's 390112 detection before the SQL is retried.
async fn fetch_fresh_stage_creds(
    ctx: &StageCredsRefreshContext,
) -> Result<CloudCredentials, StageCredsRefreshError> {
    use crate::file_manager::types::stage_creds_refresh_error::*;

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

    let query_input = rest::snowflake::QueryInput::new(ctx.sql.clone());
    let response = refresh_ctx
        .execute_with_refresh(|session_token| {
            let http_client = http_client.clone();
            let query_parameters = ctx.query_parameters.clone();
            let query_input = query_input.clone();
            async move {
                rest::snowflake::snowflake_query_with_client(
                    &http_client,
                    query_parameters,
                    session_token.reveal(),
                    query_input,
                    &crate::config::retry::RetryPolicy::default(),
                    rest::snowflake::QueryExecutionMode::Blocking,
                )
                .await
            }
        })
        .await
        .context(QueryFailedSnafu)?;

    if !response.success {
        return Err(ServerRejectedSnafu {
            message: response
                .message
                .unwrap_or_else(|| "Unknown error".to_string()),
        }
        .build());
    }

    // The re-issued PUT/GET carries the fresh stageInfo on the response.
    response
        .data
        .stage_info_creds()
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
) -> Result<Box<dyn RecordBatchReader + Send>, QueryResponseProcessingError> {
    match data {
        RowsetData::Upload(results) => {
            upload_results_reader(results, wrapper_presets).context(UploadResultsConversionSnafu)
        }
        RowsetData::Download(results) => download_results_reader(results, wrapper_presets)
            .context(DownloadResultsConversionSnafu),
        _ => read_batches(data, http_client, prefetch_config)
            .await
            .context(BatchReadingSnafu),
    }
}

pub(super) async fn read_batches(
    data: &RowsetData,
    http_client: Client,
    prefetch_config: &PrefetchConfig,
) -> Result<Box<dyn RecordBatchReader + Send>, ReadBatchesError> {
    tracing::debug!("read_batches called {:?}", data);
    match data {
        RowsetData::ArrowSingleChunk { chunk_base64 } => {
            single_chunk_reader(chunk_base64).context(ChunkReadingSnafu)
        }
        RowsetData::ArrowMultiChunk {
            initial_base64_opt,
            chunk_download_data,
        } => arrow_prefetch_reader(
            initial_base64_opt.as_deref(),
            chunk_download_data.clone().into(),
            http_client.clone(),
            prefetch_config,
        )
        .await
        .context(ChunkReadingSnafu),
        RowsetData::SchemaOnly { rowtype } => {
            let row_types = parse_row_types(rowtype)?;
            schema_only_reader(&row_types).context(ChunkReadingSnafu)
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
            .context(ChunkReadingSnafu)
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
            .context(ChunkReadingSnafu)
        }
        RowsetData::NoData | RowsetData::Upload(_) | RowsetData::Download(_) => Ok(empty_reader()),
    }
}

fn parse_row_types(rowtype: &[query_response::RowType]) -> Result<Vec<RowType>, ReadBatchesError> {
    rowtype
        .iter()
        .map(|rt| rt.try_into())
        .collect::<Result<Vec<_>, _>>()
        .context(RowTypeParsingSnafu)
}

fn validate_column_count(
    rowset: &[Vec<Option<String>>],
    row_types: &[RowType],
) -> Result<(), ReadBatchesError> {
    if !rowset.is_empty() {
        let num_columns_rowset = rowset.first().unwrap().len();
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
    BatchReading {
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
    RowTypeParsing {
        source: QueryResponseError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to decode base64 rowset"))]
    Base64Decoding {
        source: base64::DecodeError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read chunks"))]
    ChunkReading {
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
}
