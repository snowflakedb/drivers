use crate::apis::database_driver_v1::PutGetResultsetFlavor;
use crate::chunks::ChunkDownloadData;
use crate::file_manager::SourceCompressionParam;
use crate::{file_manager, query_types};
use serde::Deserialize;
use snafu::{OptionExt, Snafu};
use std::collections::HashMap;
// TODO: Delete all unused fields when we are sure they are not needed

/// Snowflake's default VARCHAR length (16 MB in characters).
/// Used as fallback when the server omits length metadata for TEXT columns.
/// See: https://docs.snowflake.com/en/sql-reference/data-types-text
///   "If no length is specified, the default is 16777216."
const DEFAULT_TEXT_LENGTH: u64 = 16_777_216;

/// Multiplier to derive byte_length from character length.
/// The SQL API returns byteLength equal to length for default TEXT columns, e.g.:
///   {"type":"text", "length":16777216, "byteLength":16777216}
/// See: https://docs.snowflake.com/en/developer-guide/sql-api/reference
const DEFAULT_TEXT_BYTE_LENGTH_MULTIPLIER: u64 = 1;

/// Response from the `POST /queries/{qid}/abort-request` endpoint.
///
/// The endpoint carries no payload beyond the standard envelope, so we use
/// `serde_json::Value` for `T`. `#[serde(default)]` on the envelope makes the
/// absent `data` field parse to `Value::Null`.
pub type AbortQueryResponse = crate::rest::snowflake::SnowflakeResponse<serde_json::Value>;

pub type Response = crate::rest::snowflake::SnowflakeResponse<Data>;

#[derive(Deserialize, Default)]
pub struct Data {
    #[serde(rename = "rowset")]
    pub rowset: Option<Vec<Vec<Option<String>>>>,
    #[serde(rename = "rowsetBase64")]
    pub rowset_base64: Option<String>,
    #[serde(rename = "rowtype")]
    pub(crate) row_type: Option<Vec<RowType>>,
    #[serde(rename = "command")]
    pub command: Option<String>,
    /// Original SQL text. Populated on responses from
    /// `monitoring/queries/{queryId}/result` (the async-PUT/GET retrieval
    /// path); also echoed back on some synchronous PUT/GET responses. Used
    /// by `connection_get_query_result` to construct a
    /// `StageInfoRefreshContext` so async PUT/GET transfers can recover
    /// from stage-info expiry by re-issuing the original command. Absent
    /// on most synchronous-execution responses where the caller already
    /// knows the SQL.
    #[serde(rename = "sqlText")]
    pub sql_text: Option<String>,

    // file transfer response data
    #[serde(rename = "src_locations")]
    src_locations: Option<Vec<String>>,
    #[serde(rename = "stageInfo")]
    stage_info: Option<StageInfo>,
    #[serde(rename = "encryptionMaterial")]
    encryption_material: Option<OneOrMany<EncryptionMaterial>>,
    #[serde(rename = "localLocation")]
    local_location: Option<String>,
    #[serde(rename = "autoCompress")]
    auto_compress: Option<bool>,
    #[serde(rename = "overwrite")]
    overwrite: Option<bool>,
    #[serde(rename = "sourceCompression")]
    source_compression: Option<String>,

    // chunked query results
    #[serde(rename = "chunks")]
    pub chunks: Option<Vec<Chunk>>,
    #[serde(rename = "qrmk")]
    _qrmk: Option<String>,
    #[serde(rename = "chunkHeaders")]
    chunk_headers: Option<HashMap<String, String>>,

    #[serde(rename = "parameters")]
    pub parameters: Option<Vec<NameValueParameter>>,
    #[serde(rename = "total")]
    pub total: Option<i64>,
    #[serde(rename = "returned")]
    pub returned: Option<i64>,
    #[serde(rename = "queryId")]
    pub query_id: Option<String>,
    #[serde(rename = "sqlState")]
    pub sql_state: Option<String>,
    #[serde(rename = "databaseProvider")]
    _database_provider: Option<String>,
    #[serde(rename = "finalDatabaseName")]
    pub final_database_name: Option<String>,
    #[serde(rename = "finalSchemaName")]
    pub final_schema_name: Option<String>,
    #[serde(rename = "finalWarehouseName")]
    pub final_warehouse_name: Option<String>,
    #[serde(rename = "finalRoleName")]
    pub final_role_name: Option<String>,
    #[serde(rename = "numberOfBinds")]
    pub number_of_binds: Option<i32>,
    #[serde(rename = "arrayBindSupported")]
    pub array_bind_supported: Option<bool>,
    #[serde(rename = "statementTypeId")]
    pub statement_type_id: Option<i64>,
    #[serde(rename = "version")]
    _version: Option<i64>,
    #[serde(rename = "getResultUrl")]
    pub get_result_url: Option<String>,
    #[serde(rename = "progressDesc")]
    _progress_desc: Option<String>,
    #[serde(rename = "queryAbortsAfterSecs")]
    _query_abort_timeout: Option<i64>,
    #[serde(rename = "resultIds")]
    pub result_ids: Option<String>,
    #[serde(rename = "resultTypes")]
    pub result_types: Option<String>,
    #[serde(rename = "queryResultFormat")]
    pub query_result_format: Option<String>,
    #[serde(rename = "asyncResult")]
    _async_result: Option<SnowflakeResult>,
    #[serde(rename = "asyncRows")]
    _async_rows: Option<SnowflakeRows>,
    #[serde(rename = "uploadInfo")]
    _upload_info: Option<StageInfo>,
    #[serde(rename = "parallel")]
    _parallel: Option<i64>,
    #[serde(rename = "threshold")]
    _threshold: Option<i64>,
    #[serde(rename = "clientShowEncryptionParameter")]
    _show_encryption_parameter: Option<bool>,
    /// Per-file pre-signed URLs returned by GS for GCS GET in
    /// presigned-only mode (no `GCS_ACCESS_TOKEN`). GS emits one URL per
    /// source file, indexed against `src_locations[i]`. `None` when GS did
    /// not emit the field (PUT path, GET-with-token path, S3/Azure stages).
    /// Consumed by `to_file_download_data`, which aligns this list with
    /// `src_locations` (padding short lists with `None`, warning and
    /// truncating long ones), and routes it through `DownloadData`.
    #[serde(rename = "presignedUrls")]
    presigned_urls: Option<Vec<Option<String>>>,
    #[serde(rename = "kind")]
    _kind: Option<String>,
    #[serde(rename = "operation")]
    _operation: Option<String>,
    #[serde(rename = "queryContext")]
    _query_context: Option<QueryContext>,
    #[serde(rename = "stats")]
    pub stats: Option<Stats>,
}

#[derive(Deserialize)]
pub struct QueryContext {
    //unused fields
    #[serde(rename = "entries")]
    _entries: Option<Vec<QueryContextEntry>>,
}

#[derive(Deserialize)]
pub struct QueryContextEntry {
    //unused fields
    #[serde(rename = "id")]
    _id: i64,
    #[serde(rename = "timestamp")]
    _timestamp: i64,
    #[serde(rename = "priority")]
    _priority: i64,
    #[serde(rename = "context")]
    _context: Option<String>,
}

#[derive(Deserialize)]
pub struct Chunk {
    #[serde(rename = "url")]
    pub url: String,
    //unused fields
    #[serde(rename = "rowCount")]
    pub row_count: i32,
    #[serde(rename = "uncompressedSize")]
    pub uncompressed_size: i64,
    #[serde(rename = "compressedSize")]
    pub compressed_size: i64,
}

#[derive(Deserialize)]
pub struct SnowflakeResult {}

#[derive(Deserialize)]
pub struct SnowflakeRows {}

/// Statistics for DML operations (INSERT, UPDATE, DELETE)
#[derive(Deserialize, Default, Clone)]
pub struct Stats {
    #[serde(rename = "numRowsInserted")]
    pub num_rows_inserted: Option<i64>,
    #[serde(rename = "numRowsUpdated")]
    pub num_rows_updated: Option<i64>,
    #[serde(rename = "numRowsDeleted")]
    pub num_rows_deleted: Option<i64>,
    #[serde(rename = "numDmlDuplicates")]
    pub num_dml_duplicates: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct NameValueParameter {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "value")]
    pub value: serde_json::Value,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RowType {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "scale")]
    pub scale: Option<u64>,
    #[serde(rename = "nullable")]
    pub nullable: bool,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "byteLength")]
    pub byte_length: Option<u64>,
    #[serde(rename = "length")]
    pub length: Option<u64>,
    #[serde(rename = "precision")]
    pub precision: Option<u64>,

    #[serde(rename = "extTypeName")]
    pub ext_type_name: Option<String>,

    /// Number of elements in a VECTOR column. Only set for VECTOR columns.
    #[serde(rename = "vectorDimension")]
    pub vector_dimension: Option<u64>,

    #[serde(rename = "fields")]
    pub fields: Option<Vec<FieldMetadata>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FieldMetadata {
    #[serde(rename = "type")]
    pub type_: String,

    // unused fields
    #[serde(rename = "name")]
    _name: Option<String>,
    #[serde(rename = "nullable")]
    _nullable: bool,
    #[serde(rename = "length")]
    _length: Option<i32>,
    #[serde(rename = "scale")]
    _scale: Option<i32>,
    #[serde(rename = "precision")]
    _precision: Option<i32>,
    #[serde(rename = "fields")]
    _fields: Option<Vec<FieldMetadata>>,
}

#[derive(Deserialize)]
pub struct StageInfo {
    #[serde(rename = "creds")]
    creds: Option<Credentials>,
    #[serde(rename = "region")]
    region: Option<String>,
    #[serde(rename = "location")]
    location: Option<String>,

    #[serde(rename = "endPoint")]
    endpoint: Option<String>,

    #[serde(rename = "locationType")]
    location_type: Option<String>,
    #[serde(rename = "presignedUrl")]
    presigned_url: Option<String>,

    #[serde(rename = "storageAccount")]
    storage_account: Option<String>,

    // unused fields
    #[serde(rename = "path")]
    _path: Option<String>,
    #[serde(rename = "isClientSideEncrypted")]
    _is_client_side_encrypted: Option<bool>,
    #[serde(rename = "useS3RegionalUrl")]
    use_s3_regional_url: Option<bool>,
    #[serde(rename = "useRegionalUrl")]
    use_regional_url: Option<bool>,
    #[serde(rename = "useVirtualUrl")]
    use_virtual_url: Option<bool>,
}

#[derive(Deserialize)]
pub struct Credentials {
    #[serde(rename = "AWS_KEY_ID")]
    aws_key_id: Option<String>,
    #[serde(rename = "AWS_SECRET_KEY")]
    aws_secret_key: Option<String>,
    #[serde(rename = "AWS_TOKEN")]
    aws_token: Option<String>,

    #[serde(rename = "GCS_ACCESS_TOKEN")]
    gcs_access_token: Option<String>,

    #[serde(rename = "AZURE_SAS_TOKEN")]
    azure_sas_token: Option<String>,

    // unused fields
    #[serde(rename = "AWS_ID")]
    _aws_id: Option<String>,
    #[serde(rename = "AWS_KEY")]
    _aws_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EncryptionMaterial {
    #[serde(rename = "queryStageMasterKey")]
    query_stage_master_key: String,
    #[serde(rename = "queryId")]
    query_id: String,
    #[serde(rename = "smkId")]
    smk_id: String,
}

impl Data {
    /// Returns the full `StageInfoSnapshot` (creds + `presignedUrl` +
    /// `presignedUrls[]`) embedded in the response's `stageInfo`, or `None`
    /// if the response carries no `stageInfo` block (non-PUT/GET). Used by
    /// the stage-info refresh path (`SnowflakeStageInfoRefresher`), which
    /// writes the snapshot into the shared `StageInfoCache` after every
    /// re-issue of the original PUT/GET SQL.
    ///
    /// S3 / Azure responses populate only `.creds`; both URL fields are
    /// `None`. GCS responses populate `.creds`, `.presigned_url` (PUT-side
    /// single slot) and/or `.presigned_urls` (GET-side per-file list)
    /// according to whether the stage is in presigned mode and whether the
    /// command is a PUT or GET.
    pub fn stage_info_snapshot(
        &self,
    ) -> Result<Option<file_manager::StageInfoSnapshot>, QueryResponseError> {
        let Some(stage_info) = self.stage_info.as_ref() else {
            return Ok(None);
        };
        let converted: file_manager::StageInfo = stage_info.try_into()?;
        Ok(Some(file_manager::StageInfoSnapshot {
            creds: converted.creds,
            presigned_url: converted.presigned_url,
            // `presigned_urls[]` lives at the `Data` level (it's aligned
            // with `src_locations`), not on the inner `stageInfo` block.
            presigned_urls: self.presigned_urls.clone(),
        }))
    }

    /// Copies the fields necessary for file transfer.
    /// Encryption material is optional — SSE stages omit it from the response.
    ///
    /// `flavor` selects the wrapper-specific shape of the resulting PUT
    /// result set; it is forwarded into `SingleUploadData` so that
    /// `file_manager::upload_single_file` can populate the `message` column
    /// per `BehaviorDifferences.yaml` BD#3.
    /// `legacy_odbc_compression_autodetect` opts the
    /// PUT auto-detect path into the libsnowflakeclient-parity behaviors
    /// (short-prefix magic-byte detection plus error-swallowing on
    /// unsupported formats). See `WrapperPresets` for the full doc-comment.
    pub fn to_file_upload_data(
        &self,
        flavor: PutGetResultsetFlavor,
        legacy_odbc_compression_autodetect: bool,
        use_s3_regional_url_session_param: bool,
    ) -> Result<file_manager::UploadData, QueryResponseError> {
        let src_locations = self.src_locations.as_ref().context(MissingParameterSnafu {
            parameter: "source locations",
        })?;

        if src_locations.len() != 1 {
            InvalidFormatSnafu {
                message: "Expected exactly one source location for upload".to_string(),
            }
            .fail()?;
        }

        let src_location = src_locations
            .first()
            .context(MissingParameterSnafu {
                parameter: "source location",
            })?
            .clone();

        let mut stage_info: file_manager::StageInfo = self
            .stage_info
            .as_ref()
            .context(MissingParameterSnafu {
                parameter: "stage info",
            })?
            .try_into()?;

        if use_s3_regional_url_session_param {
            stage_info.use_s3_regional_url = true;
        }

        let encryption_material: Option<file_manager::EncryptionMaterial> = match &self
            .encryption_material
        {
            Some(materials) => {
                let converted: Vec<file_manager::EncryptionMaterial> = materials.into();
                match converted.len() {
                    0 => None,
                    1 => converted.into_iter().next(),
                    _ => InvalidFormatSnafu {
                        message: "Expected exactly one encryption material for upload".to_string(),
                    }
                    .fail()?,
                }
            }
            None => None,
        };

        let auto_compress = self.auto_compress.context(MissingParameterSnafu {
            parameter: "auto compress",
        })?;

        let source_compression_string = self
            .source_compression
            .as_ref()
            .context(MissingParameterSnafu {
                parameter: "source compression",
            })?
            .clone();

        let source_compression = match source_compression_string.to_uppercase().as_str() {
            "AUTO_DETECT" => SourceCompressionParam::AutoDetect,
            "GZIP" => SourceCompressionParam::Gzip,
            "BZIP2" => SourceCompressionParam::Bzip2,
            "BROTLI" => SourceCompressionParam::Brotli,
            "ZSTD" => SourceCompressionParam::Zstd,
            "DEFLATE" => SourceCompressionParam::Deflate,
            "RAW_DEFLATE" => SourceCompressionParam::RawDeflate,
            "PARQUET" => SourceCompressionParam::Parquet,
            "ORC" => SourceCompressionParam::Orc,
            "NONE" => SourceCompressionParam::None,
            _ => InvalidFormatSnafu {
                message: format!("Unknown source compression type: {source_compression_string}"),
            }
            .fail()?,
        };

        let overwrite = self.overwrite.unwrap_or(false);

        Ok(file_manager::UploadData {
            src_location_pattern: src_location,
            stage_info,
            encryption_material,
            auto_compress,
            source_compression,
            overwrite,
            flavor,
            legacy_odbc_compression_autodetect,
        })
    }

    /// Build a `SingleUploadData` for an internal `SYSTEM$BIND` CSV upload.
    ///
    /// Unlike user-facing PUT commands, `SYSTEM$BIND` uploads are internal
    /// driver infrastructure: the upload result is never surfaced to the
    /// application, so no wrapper flavor or compression-detection preset is
    /// needed. Only `stage_info`, `encryption_material`, and `overwrite` are
    /// read from the server response; everything else is fixed for this path:
    /// - `filename` / `source` are always `"0"` (one CSV file per request)
    /// - `auto_compress` is `true` (stage binding uses server-side compress)
    /// - `source_compression` is `None` (CSV payload is sent uncompressed)
    pub fn to_bind_stage_upload_data(
        &self,
        use_s3_regional_url_session_param: bool,
    ) -> Result<file_manager::SingleUploadData, QueryResponseError> {
        let mut stage_info: file_manager::StageInfo = self
            .stage_info
            .as_ref()
            .context(MissingParameterSnafu {
                parameter: "stage info",
            })?
            .try_into()?;

        if use_s3_regional_url_session_param {
            stage_info.use_s3_regional_url = true;
        }

        let encryption_material: Option<file_manager::EncryptionMaterial> = match &self
            .encryption_material
        {
            Some(materials) => {
                let converted: Vec<file_manager::EncryptionMaterial> = materials.into();
                match converted.len() {
                    0 => None,
                    1 => converted.into_iter().next(),
                    _ => InvalidFormatSnafu {
                        message: "Expected exactly one encryption material for upload".to_string(),
                    }
                    .fail()?,
                }
            }
            None => None,
        };

        let overwrite = self.overwrite.unwrap_or(false);

        Ok(file_manager::SingleUploadData {
            // In-memory upload: `upload_in_memory_file` supplies the CSV bytes
            // directly and ignores `source`; this only labels the logical source.
            source: file_manager::ByteSource::Path("0".into()),
            filename: "0".to_string(),
            stage_info,
            encryption_material,
            auto_compress: true,
            source_compression: SourceCompressionParam::None,
            overwrite,
            flavor: PutGetResultsetFlavor::default(),
            legacy_odbc_compression_autodetect: false,
        })
    }

    /// Encryption material is optional — SSE stages omit it from the response.
    ///
    /// `flavor` selects the wrapper-specific shape of the resulting GET
    /// result set; it is forwarded into `SingleDownloadData` so that
    /// `file_manager::download_single_file` can populate the `size`
    /// column per `BehaviorDifferences.yaml` BD#4. Taken by reference so
    /// callers don't have to clone — the single `clone` happens at the
    /// `DownloadData` storage point below.
    pub fn to_file_download_data(
        &self,
        flavor: &PutGetResultsetFlavor,
        use_s3_regional_url_session_param: bool,
    ) -> Result<file_manager::DownloadData, QueryResponseError> {
        let src_locations = self
            .src_locations
            .as_ref()
            .context(MissingParameterSnafu {
                parameter: "source locations",
            })?
            .clone();

        if src_locations.is_empty() {
            MissingParameterSnafu {
                parameter: "source locations",
            }
            .fail()?;
        }

        let mut stage_info: file_manager::StageInfo = self
            .stage_info
            .as_ref()
            .context(MissingParameterSnafu {
                parameter: "stage info",
            })?
            .try_into()?;

        if use_s3_regional_url_session_param {
            stage_info.use_s3_regional_url = true;
        }

        let encryption_materials: Vec<Option<file_manager::EncryptionMaterial>> =
            match &self.encryption_material {
                Some(materials) => {
                    let converted: Vec<file_manager::EncryptionMaterial> = materials.into();
                    if converted.is_empty() {
                        vec![None; src_locations.len()]
                    } else if src_locations.len() != converted.len() {
                        InvalidFormatSnafu {
                        message:
                            "Number of source locations must match number of encryption materials"
                                .to_string(),
                    }
                    .fail()?
                    } else {
                        converted.into_iter().map(Some).collect()
                    }
                }
                None => vec![None; src_locations.len()],
            };

        let (over_long, under_long) = presigned_url_policies(flavor);
        let presigned_urls = align_presigned_urls(
            self.presigned_urls.as_deref(),
            &src_locations,
            over_long,
            under_long,
        );

        let local_location: String = self
            .local_location
            .as_ref()
            .context(MissingParameterSnafu {
                parameter: "local location",
            })?
            .clone();

        Ok(file_manager::DownloadData {
            src_locations,
            local_location,
            stage_info,
            encryption_materials,
            presigned_urls,
            flavor: flavor.clone(),
        })
    }

    /// Converts to `RowsetData` by moving fields out of `Data`.
    pub fn into_rowset_data(self) -> RowsetData {
        let chunk_download_data = self.to_chunk_download_data();
        let initial_base64_opt = self.rowset_base64.filter(|v| !v.is_empty());

        match self.query_result_format.as_deref() {
            Some("arrow") => match (initial_base64_opt, chunk_download_data, self.row_type) {
                (initial_base64_opt, Some(chunk_download_data), _) => RowsetData::ArrowMultiChunk {
                    initial_base64_opt,
                    chunk_download_data,
                },
                (Some(chunk_base64), None, _) => RowsetData::ArrowSingleChunk { chunk_base64 },
                (None, None, Some(rowtype)) => RowsetData::SchemaOnly { rowtype },
                _ => {
                    tracing::error!(
                        "Initial base64 and/or chunk download data are missing for Arrow result format"
                    );
                    RowsetData::NoData
                }
            },
            Some("json") => match (self.rowset, self.row_type) {
                (Some(rowset), Some(rowtype)) => match chunk_download_data {
                    Some(chunk_download_data) => RowsetData::JsonMultiChunk {
                        rowset,
                        rowtype,
                        chunk_download_data,
                    },
                    None => RowsetData::JsonRowset { rowset, rowtype },
                },
                _ => {
                    tracing::error!("Rowset and/or rowtype are missing for JSON result format");
                    RowsetData::NoData
                }
            },
            Some(other) => {
                tracing::error!("Unsupported query result format: {other}");
                RowsetData::NoData
            }
            None => RowsetData::NoData,
        }
    }

    pub fn to_chunk_download_data(&self) -> Option<Vec<ChunkDownloadData>> {
        match (self.chunks.as_ref(), self.chunk_headers.as_ref()) {
            (Some(chunks), chunk_headers_opt) => {
                let empty_headers = HashMap::new();
                let chunk_headers = chunk_headers_opt.unwrap_or(&empty_headers);
                if chunk_headers_opt.is_none() {
                    tracing::warn!("Chunks found without chunk headers; using empty headers");
                }
                let chunk_download_data = chunks
                    .iter()
                    .map(|chunk| ChunkDownloadData::new(chunk, chunk_headers))
                    .collect();
                Some(chunk_download_data)
            }
            (None, Some(_)) => {
                tracing::error!("Chunk headers found but chunks are missing");
                None
            }
            _ => None,
        }
    }

    pub fn to_initial_base64_opt(&self) -> Option<&str> {
        let value = self.rowset_base64.as_deref()?;
        if value.is_empty() { None } else { Some(value) }
    }

    #[allow(clippy::type_complexity)]
    pub fn to_json_rowset(&self) -> Option<(&Vec<Vec<Option<String>>>, &Vec<RowType>)> {
        match (self.rowset.as_ref(), self.row_type.as_ref()) {
            (Some(rowset), Some(row_type)) => Some((rowset, row_type)),
            (Some(_), None) => {
                tracing::error!("Rowset found but rowtype is missing");
                None
            }
            (None, Some(_)) => {
                tracing::error!("Rowtype found but rowset is missing");
                None
            }
            _ => None,
        }
    }
}

/// Controls `align_presigned_urls` when `presignedUrls.len() > src_locations.len()`.
///
/// | Variant                 | Behaviour                                                            | Driver parity                      |
/// |-------------------------|----------------------------------------------------------------------|------------------------------------|
/// | `IgnoreExtras`          | Use the first `N` URLs positionally; extra entries are never read.   | Python, Go, Node.js, libsfc, ODBC  |
/// | `FallbackToCredentials` | Discard the entire list; every file downloads via the credential     | JDBC                               |
/// |                         | chain (`stage_info.presigned_url` → `gcs_access_token`).            |                                    |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlongPresignedUrlsPolicy {
    /// Use the first `N` URLs positionally; any entries beyond `src_locations.len()`
    /// are never read. Matches Python's `idx < len(presigned_urls)` guard and the
    /// loop-over-files approach used by Go, Node.js, libsnowflakeclient, and ODBC.
    IgnoreExtras,
    /// Discard the entire `presignedUrls` list and let every file fall back through
    /// `resolve_url_and_token`'s credential chain (`stage_info.presigned_url` →
    /// `gcs_access_token`). If neither credential is present, the download surfaces
    /// `MissingGcsCredentials`. Use this when positional alignment cannot be trusted
    /// (e.g. GS emitted extra URLs due to a race condition). Matches JDBC behavior.
    FallbackToCredentials,
}

/// Controls `align_presigned_urls` when `presignedUrls.len() < src_locations.len()`.
///
/// | Variant                 | Behaviour                                                            | Driver parity                      |
/// |-------------------------|----------------------------------------------------------------------|------------------------------------|
/// | `TokenFallback`         | Use URLs for present slots; missing slots fall back to credentials.  | Python, Go, Node.js, libsfc, ODBC  |
/// | `FallbackToCredentials` | Discard the entire list; every file downloads via the credential     | JDBC                               |
/// |                         | chain (`stage_info.presigned_url` → `gcs_access_token`).            |                                    |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnderlongPresignedUrlsPolicy {
    /// Use presigned URLs for the slots that are present; pad missing tail slots with
    /// `None` so those files fall back through `resolve_url_and_token`'s credential
    /// chain. Matches Python's `idx < len(presigned_urls)` guard and libsnowflakeclient's
    /// `size() > i` check.
    TokenFallback,
    /// Discard the entire `presignedUrls` list and let every file fall back through
    /// `resolve_url_and_token`'s credential chain (`stage_info.presigned_url` →
    /// `gcs_access_token`). If neither credential is present, the download surfaces
    /// `MissingGcsCredentials`. Use this when a partial list cannot be trusted to be
    /// positionally aligned. Matches JDBC behavior.
    FallbackToCredentials,
}

fn presigned_url_policies(
    flavor: &PutGetResultsetFlavor,
) -> (OverlongPresignedUrlsPolicy, UnderlongPresignedUrlsPolicy) {
    match flavor {
        // JDBC requires strict length equality; any mismatch discards the
        // entire list and falls back to credentials for all files.
        PutGetResultsetFlavor::Jdbc => (
            OverlongPresignedUrlsPolicy::FallbackToCredentials,
            UnderlongPresignedUrlsPolicy::FallbackToCredentials,
        ),
        // All other drivers (Python, ODBC, Go, Node.js, libsfc) use per-index
        // bounds checks: ignore extras, fall back to token for missing slots.
        _ => (
            OverlongPresignedUrlsPolicy::IgnoreExtras,
            UnderlongPresignedUrlsPolicy::TokenFallback,
        ),
    }
}

/// Aligns the optional server-supplied per-file pre-signed URL list to
/// `src_locations` for routing through `DownloadData`.
///
/// GS is expected to deliver one URL per source file in positional order.
/// When that invariant is violated, `over_long` and `under_long` govern
/// recovery. Pass `presigned_url_policies(flavor)` to select the behavior
/// appropriate for the active driver flavor.
///
/// # Parameters
///
/// * `presigned_urls` — the raw `presignedUrls[]` array from GS, or `None`
///   when the field was absent (PUT path, S3/Azure stages).
/// * `src_locations` — the file list from the same response.
/// * `over_long` — [`OverlongPresignedUrlsPolicy`] to apply when
///   `presigned_urls.len() > src_locations.len()`.
/// * `under_long` — [`UnderlongPresignedUrlsPolicy`] to apply when
///   `presigned_urls.len() < src_locations.len()`.
///
/// Returns a `Vec<Option<String>>` of length `src_locations.len()`. URLs
/// are not logged — they carry signed query strings with object-scope
/// credentials.
fn align_presigned_urls(
    presigned_urls: Option<&[Option<String>]>,
    src_locations: &[String],
    over_long: OverlongPresignedUrlsPolicy,
    under_long: UnderlongPresignedUrlsPolicy,
) -> Vec<Option<String>> {
    let target_len = src_locations.len();
    let Some(urls) = presigned_urls else {
        return vec![None; target_len];
    };

    if urls.len() == target_len {
        return urls.to_vec();
    }

    if urls.len() > target_len {
        match over_long {
            OverlongPresignedUrlsPolicy::IgnoreExtras => {
                tracing::warn!(
                    presigned_url_count = urls.len(),
                    src_location_count = target_len,
                    "Client backend presignedUrls[] longer than src_locations[]; ignoring extra entries"
                );
                urls.iter().take(target_len).cloned().collect()
            }
            OverlongPresignedUrlsPolicy::FallbackToCredentials => {
                tracing::warn!(
                    presigned_url_count = urls.len(),
                    src_location_count = target_len,
                    "Client backend presignedUrls[] longer than src_locations[]; \
                     falling back to credentials for all files"
                );
                vec![None; target_len]
            }
        }
    } else {
        match under_long {
            UnderlongPresignedUrlsPolicy::TokenFallback => {
                tracing::warn!(
                    presigned_url_count = urls.len(),
                    src_location_count = target_len,
                    "Client backend presignedUrls[] shorter than src_locations[]; \
                     missing slots will fall back to token credentials"
                );
                let mut out: Vec<Option<String>> = urls.to_vec();
                out.resize(target_len, None);
                out
            }
            UnderlongPresignedUrlsPolicy::FallbackToCredentials => {
                tracing::warn!(
                    presigned_url_count = urls.len(),
                    src_location_count = target_len,
                    "Client backend presignedUrls[] shorter than src_locations[]; \
                     falling back to credentials for all files"
                );
                vec![None; target_len]
            }
        }
    }
}

#[derive(Debug)]
pub enum RowsetData {
    SchemaOnly {
        rowtype: Vec<RowType>,
    },
    ArrowMultiChunk {
        initial_base64_opt: Option<String>,
        chunk_download_data: Vec<ChunkDownloadData>,
    },
    ArrowSingleChunk {
        chunk_base64: String,
    },
    JsonRowset {
        rowset: Vec<Vec<Option<String>>>,
        rowtype: Vec<RowType>,
    },
    JsonMultiChunk {
        rowset: Vec<Vec<Option<String>>>,
        rowtype: Vec<RowType>,
        chunk_download_data: Vec<ChunkDownloadData>,
    },
    /// PUT (UPLOAD) result: file transfer already executed, results stored.
    Upload(Vec<crate::file_manager::UploadResult>),
    /// GET (DOWNLOAD) result: file transfer already executed, results stored.
    Download(Vec<crate::file_manager::DownloadResult>),
    NoData,
}

/// Selects the storage representation for a `GEOGRAPHY` / `GEOMETRY` column
/// based on the underlying `type` field the server sends. The server routes
/// text output formats (GeoJSON → `object`, WKT/EWKT → `text`) and binary
/// output formats (WKB/EWKB → `binary`) through this field.
fn geo_representation(underlying_type: &str) -> query_types::GeoRepresentation {
    if underlying_type.eq_ignore_ascii_case("binary") {
        query_types::GeoRepresentation::Binary
    } else {
        query_types::GeoRepresentation::Text
    }
}

impl TryFrom<&RowType> for query_types::RowType {
    type Error = QueryResponseError;

    fn try_from(value: &RowType) -> Result<Self, Self::Error> {
        let name = value.name.clone();
        let nullable = value.nullable;
        let effective_type = value
            .ext_type_name
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&value.type_);

        match effective_type.to_uppercase().as_str() {
            "TEXT" => {
                // Use Snowflake's default VARCHAR max length when the server omits
                // length metadata. This happens when the server returns DECFLOAT
                // columns as TEXT type for clients it doesn't recognize as
                // DECFLOAT-capable (e.g. JSON format fallback).
                let length = value.length.unwrap_or(DEFAULT_TEXT_LENGTH);
                let byte_length = value
                    .byte_length
                    .unwrap_or(length.saturating_mul(DEFAULT_TEXT_BYTE_LENGTH_MULTIPLIER));

                Ok(query_types::RowType::text(
                    &name,
                    nullable,
                    length,
                    byte_length,
                ))
            }
            "FIXED" => {
                let precision = value.precision.context(MissingParameterSnafu {
                    parameter: format!(
                        "row type -> precision for FIXED/NUMBER/NUMERIC/DECIMAL column '{name}'"
                    ),
                })?;

                let scale = value.scale.context(MissingParameterSnafu {
                    parameter: format!(
                        "row type -> scale for FIXED/NUMBER/NUMERIC/DECIMAL column '{name}'"
                    ),
                })?;

                Ok(query_types::RowType::fixed(
                    &name, nullable, precision, scale,
                ))
            }
            "REAL" => Ok(query_types::RowType::real(&name, nullable)),
            "DATE" => Ok(query_types::RowType::date(&name, nullable)),
            "TIMESTAMP_NTZ" => {
                let scale = value.scale.unwrap_or(9);
                Ok(query_types::RowType::timestamp_ntz(&name, nullable, scale))
            }
            "TIMESTAMP_LTZ" => {
                let scale = value.scale.unwrap_or(9);
                Ok(query_types::RowType::timestamp_ltz(&name, nullable, scale))
            }
            "TIMESTAMP_TZ" => {
                let scale = value.scale.unwrap_or(9);
                Ok(query_types::RowType::timestamp_tz(&name, nullable, scale))
            }
            "BOOLEAN" => Ok(query_types::RowType::boolean(&name, nullable)),
            "TIME" => {
                let scale = value.scale.unwrap_or(9);
                Ok(query_types::RowType::time(&name, nullable, scale))
            }
            "BINARY" => {
                let length = value.length.context(MissingParameterSnafu {
                    parameter: format!("row type -> length for BINARY column '{name}'"),
                })?;

                let byte_length = value.byte_length.context(MissingParameterSnafu {
                    parameter: format!("row type -> byte length for BINARY column '{name}'"),
                })?;

                Ok(query_types::RowType::binary(
                    &name,
                    nullable,
                    length,
                    byte_length,
                ))
            }
            "DECFLOAT" => {
                let precision = value.precision.unwrap_or(38);
                Ok(query_types::RowType::decfloat(&name, nullable, precision))
            }
            "OBJECT" => Ok(query_types::RowType::object(&name, nullable)),
            "ARRAY" => Ok(query_types::RowType::array(&name, nullable)),
            "VARIANT" => Ok(query_types::RowType::variant(&name, nullable)),
            "INTERVAL_YEAR_MONTH" => {
                let precision = value.precision.context(MissingParameterSnafu {
                    parameter: format!(
                        "row type -> precision for INTERVAL_YEAR_MONTH column '{name}'"
                    ),
                })?;
                let scale = value.scale.context(MissingParameterSnafu {
                    parameter: format!("row type -> scale for INTERVAL_YEAR_MONTH column '{name}'"),
                })?;
                Ok(query_types::RowType::interval_year_month(
                    &name, nullable, precision, scale,
                ))
            }
            "INTERVAL_DAY_SECOND" | "INTERVAL_DAY_TIME" => {
                let precision = value.precision.context(MissingParameterSnafu {
                    parameter: format!(
                        "row type -> precision for INTERVAL_DAY_TIME/DAY_SECOND column '{name}'"
                    ),
                })?;
                let scale = value.scale.context(MissingParameterSnafu {
                    parameter: format!(
                        "row type -> scale for INTERVAL_DAY_TIME/DAY_SECOND column '{name}'"
                    ),
                })?;
                Ok(query_types::RowType::interval_day_second(
                    &name, nullable, precision, scale,
                ))
            }
            "GEOGRAPHY" => Ok(query_types::RowType::geography(
                &name,
                nullable,
                geo_representation(&value.type_),
            )),
            "GEOMETRY" => Ok(query_types::RowType::geometry(
                &name,
                nullable,
                geo_representation(&value.type_),
            )),
            "VECTOR" => parse_vector_row_type(&name, nullable, value),
            other => InvalidFormatSnafu {
                message: format!("Unsupported column type '{other}' for column '{name}'"),
            }
            .fail(),
        }
    }
}

/// Parses a `VECTOR` row type. The server must send both `vectorDimension` and a
/// single-element `fields` array describing the element type (`FIXED` or `REAL`).
fn parse_vector_row_type(
    name: &str,
    nullable: bool,
    value: &RowType,
) -> Result<query_types::RowType, QueryResponseError> {
    let raw_dim = value.vector_dimension.context(MissingParameterSnafu {
        parameter: format!("row type -> vectorDimension for VECTOR column '{name}'"),
    })?;
    // Snowflake VECTOR dimensions are bounded (<= 4096) and always fit in usize.
    // Cast via `as` to match the trust-the-server convention used elsewhere for
    // server-provided sizes; Arrow's FixedSizeListArray will reject an invalid
    // size when the array is finalised.
    let dimension = raw_dim as usize;

    let element_field =
        value
            .fields
            .as_ref()
            .and_then(|f| f.first())
            .context(MissingParameterSnafu {
                parameter: format!("row type -> fields for VECTOR column '{name}'"),
            })?;
    let element_type = if element_field.type_.eq_ignore_ascii_case("FIXED") {
        query_types::VectorElementType::Int32
    } else if element_field.type_.eq_ignore_ascii_case("REAL") {
        query_types::VectorElementType::Float32
    } else {
        return InvalidFormatSnafu {
            message: format!(
                "Unsupported VECTOR element type '{}' for column '{name}'",
                element_field.type_,
            ),
        }
        .fail();
    };

    Ok(query_types::RowType::vector(
        name,
        nullable,
        dimension,
        element_type,
    ))
}

/// Server-pushed session parameter that ORs into `StageInfo.use_s3_regional_url`.
/// All three reference drivers (Python connector, JDBC, libsnowflakeclient)
/// read this exact key from the login response. The canonical name on the
/// Rust side is `use_s3_regional_url`; this string is only the wire-level key
/// that GS uses.
const ENABLE_STAGE_S3_PRIVATELINK_SERVER_KEY: &str = "ENABLE_STAGE_S3_PRIVATELINK_FOR_US_EAST_1";

/// Reads the `ENABLE_STAGE_S3_PRIVATELINK_FOR_US_EAST_1` session parameter as
/// a boolean.
///
/// `session_parameters` keys are uppercased upstream — see the write sites in
/// `apis::database_driver_v1::connection` (`session_parameters.write()` at
/// login merge and post-query response) and the corresponding read in
/// `connection_get_parameter` which uppercases its lookup key. We therefore
/// do a direct `HashMap::get` on the canonical uppercase form rather than
/// scanning the map.
///
/// Accepted value forms: `"true"` (case-insensitive) and `"1"`. JSON `true`
/// and JSON `1` from GS land here as those exact strings after the
/// post-response stringification at `apis::database_driver_v1::connection`.
/// JDBC additionally accepts `"on"`; we don't currently, since GS doesn't
/// emit that form.
///
/// Called at the PUT/GET dispatch site rather than passing the whole
/// session-parameter map down: PUT/GET only needs this one key, and reading
/// it eagerly avoids cloning the entire `HashMap<String, String>` on every
/// transfer.
pub fn read_use_s3_regional_url_session_param(
    session_parameters: &HashMap<String, String>,
) -> bool {
    session_parameters
        .get(ENABLE_STAGE_S3_PRIVATELINK_SERVER_KEY)
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false)
}

impl TryFrom<&StageInfo> for file_manager::StageInfo {
    type Error = QueryResponseError;

    fn try_from(value: &StageInfo) -> Result<Self, Self::Error> {
        // Determine location type (default to S3 for backward compatibility)
        let location_type = match value.location_type.as_deref() {
            Some("GCS") => file_manager::LocationType::Gcs,
            Some("AZURE") => file_manager::LocationType::Azure,
            Some("S3") | None => file_manager::LocationType::S3,
            Some(other) => {
                return InvalidFormatSnafu {
                    message: format!("Unknown location type: {other}"),
                }
                .fail();
            }
        };

        let location = value
            .location
            .as_ref()
            .context(MissingParameterSnafu {
                parameter: "stage info -> location",
            })?
            .clone();

        let bucket_separator = location.find('/').context(InvalidFormatSnafu {
            message: format!("Invalid location format: {location}"),
        })?;

        let bucket = location[..bucket_separator].to_string();
        let mut key_prefix = location[bucket_separator + 1..].to_string();
        if !key_prefix.is_empty() && !key_prefix.ends_with('/') {
            key_prefix.push('/');
        }

        let region = value
            .region
            .as_ref()
            .context(MissingParameterSnafu {
                parameter: "stage info -> region",
            })?
            .clone();

        // Build credentials based on location type
        let creds_data = value.creds.as_ref().context(MissingParameterSnafu {
            parameter: "stage info -> credentials",
        })?;

        let creds = match location_type {
            file_manager::LocationType::S3 => file_manager::CloudCredentials::S3 {
                aws_key_id: creds_data
                    .aws_key_id
                    .as_ref()
                    .context(MissingParameterSnafu {
                        parameter: "credentials -> aws key id",
                    })?
                    .clone(),
                aws_secret_key: creds_data
                    .aws_secret_key
                    .as_ref()
                    .context(MissingParameterSnafu {
                        parameter: "credentials -> aws secret key",
                    })?
                    .clone()
                    .into(),
                aws_token: creds_data
                    .aws_token
                    .as_ref()
                    .context(MissingParameterSnafu {
                        parameter: "credentials -> aws token",
                    })?
                    .clone()
                    .into(),
            },
            file_manager::LocationType::Gcs => file_manager::CloudCredentials::Gcs {
                gcs_access_token: creds_data
                    .gcs_access_token
                    .as_ref()
                    .filter(|t| !t.is_empty())
                    .map(|t| t.clone().into()),
            },
            file_manager::LocationType::Azure => file_manager::CloudCredentials::Azure {
                sas_token: creds_data
                    .azure_sas_token
                    .as_ref()
                    .context(MissingParameterSnafu {
                        parameter: "credentials -> AZURE_SAS_TOKEN",
                    })?
                    .clone()
                    .into(),
            },
        };

        let endpoint = value.endpoint.as_ref().filter(|ep| !ep.is_empty()).cloned();

        let presigned_url = value
            .presigned_url
            .as_ref()
            .filter(|url| !url.is_empty())
            .cloned();

        // ME-CENTRAL2 always uses regional URLs, regardless of the flag
        let use_regional_url =
            value.use_regional_url.unwrap_or(false) || region.eq_ignore_ascii_case("me-central2");
        let use_virtual_url = value.use_virtual_url.unwrap_or(false);
        // S3 PrivateLink / Snowpipe Streaming: either useS3RegionalUrl or
        // useRegionalUrl forces the regional endpoint. Mirrors the OR
        // semantics in the reference Python (s3_storage_client.py:85-91),
        // JDBC (StorageClientFactory.java:55-58), and libsnowflakeclient
        // (SnowflakeS3Client.cpp:106-113) S3 paths.
        let use_s3_regional_url =
            value.use_s3_regional_url.unwrap_or(false) || value.use_regional_url.unwrap_or(false);

        let storage_account = match location_type {
            file_manager::LocationType::Azure => Some(
                value
                    .storage_account
                    .as_ref()
                    .filter(|sa| !sa.is_empty())
                    .context(MissingParameterSnafu {
                        parameter: "stage info -> storageAccount",
                    })?
                    .clone(),
            ),
            _ => value
                .storage_account
                .as_ref()
                .filter(|sa| !sa.is_empty())
                .cloned(),
        };

        Ok(file_manager::StageInfo {
            location_type,
            bucket,
            key_prefix,
            region,
            creds,
            endpoint,
            presigned_url,
            use_virtual_url,
            use_regional_url,
            use_s3_regional_url,
            storage_account,
        })
    }
}

impl From<&EncryptionMaterial> for file_manager::EncryptionMaterial {
    fn from(value: &EncryptionMaterial) -> Self {
        Self {
            query_stage_master_key: value.query_stage_master_key.clone().into(),
            query_id: value.query_id.clone(),
            smk_id: value.smk_id.clone(),
        }
    }
}

impl From<&OneOrMany<EncryptionMaterial>> for Vec<file_manager::EncryptionMaterial> {
    fn from(value: &OneOrMany<EncryptionMaterial>) -> Self {
        value.as_slice().iter().map(|em| em.into()).collect()
    }
}

// Snowflake API can return a single object or an array for some fields - for example EncryptionMaterial
#[derive(Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> OneOrMany<T> {
    /// Returns a slice of the items without consuming the enum.
    fn as_slice(&self) -> &[T] {
        match self {
            OneOrMany::One(item) => std::slice::from_ref(item),
            OneOrMany::Many(vec) => vec.as_slice(),
        }
    }
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum QueryResponseError {
    #[snafu(display("Missing parameter in Snowflake response: {parameter}"))]
    MissingParameter {
        parameter: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Invalid Snowflake response: {message}"))]
    InvalidFormat {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_rowset_with_null_values() {
        let json = r#"{
            "data": {
                "rowset": [["val1", null, "val3"], [null, "val2", null]],
                "queryResultFormat": "json",
                "rowtype": [
                    {"name": "c1", "type": "TEXT", "nullable": false, "scale": null, "byteLength": 64, "length": 16, "precision": null},
                    {"name": "c2", "type": "TEXT", "nullable": true, "scale": null, "byteLength": 64, "length": 16, "precision": null},
                    {"name": "c3", "type": "TEXT", "nullable": true, "scale": null, "byteLength": 64, "length": 16, "precision": null}
                ]
            },
            "success": true
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();
        assert!(response.success);

        let rowset = response.data.rowset.as_ref().unwrap();
        assert_eq!(rowset.len(), 2);

        // First row: "val1", null, "val3"
        assert_eq!(rowset[0][0], Some("val1".to_string()));
        assert_eq!(rowset[0][1], None);
        assert_eq!(rowset[0][2], Some("val3".to_string()));

        // Second row: null, "val2", null
        assert_eq!(rowset[1][0], None);
        assert_eq!(rowset[1][1], Some("val2".to_string()));
        assert_eq!(rowset[1][2], None);
    }

    #[test]
    fn test_deserialize_rowset_all_nulls() {
        let json = r#"{
            "data": {
                "rowset": [[null, null]],
                "queryResultFormat": "json",
                "rowtype": [
                    {"name": "c1", "type": "TEXT", "nullable": true, "scale": null, "byteLength": 64, "length": 16, "precision": null},
                    {"name": "c2", "type": "TEXT", "nullable": true, "scale": null, "byteLength": 64, "length": 16, "precision": null}
                ]
            },
            "success": true
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();
        let rowset = response.data.rowset.as_ref().unwrap();
        assert_eq!(rowset[0][0], None);
        assert_eq!(rowset[0][1], None);
    }

    #[test]
    fn test_to_json_rowset_with_nulls() {
        let json = r#"{
            "data": {
                "rowset": [["a", null], [null, "b"]],
                "queryResultFormat": "json",
                "rowtype": [
                    {"name": "c1", "type": "TEXT", "nullable": true, "scale": null, "byteLength": 64, "length": 16, "precision": null},
                    {"name": "c2", "type": "TEXT", "nullable": true, "scale": null, "byteLength": 64, "length": 16, "precision": null}
                ]
            },
            "success": true
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();
        let (rowset, row_types) = response.data.to_json_rowset().unwrap();
        assert_eq!(rowset.len(), 2);
        assert_eq!(row_types.len(), 2);
    }

    #[test]
    fn test_arrow_chunks_without_headers_still_build_chunk_download_data() {
        let json = r#"{
            "data": {
                "queryResultFormat": "arrow",
                "chunks": [
                    {
                        "url": "https://example.com/chunk-1",
                        "rowCount": 1,
                        "uncompressedSize": 16,
                        "compressedSize": 16
                    }
                ],
                "rowtype": [
                    {
                        "name": "c1",
                        "type": "TEXT",
                        "nullable": true,
                        "scale": null,
                        "byteLength": 64,
                        "length": 16,
                        "precision": null
                    }
                ]
            },
            "success": true
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();
        let chunk_download_data = response.data.to_chunk_download_data().unwrap();

        assert_eq!(chunk_download_data.len(), 1);
    }

    #[test]
    fn test_arrow_chunks_without_headers_use_multi_chunk_rowset_data() {
        let json = r#"{
            "data": {
                "queryResultFormat": "arrow",
                "chunks": [
                    {
                        "url": "https://example.com/chunk-1",
                        "rowCount": 1,
                        "uncompressedSize": 16,
                        "compressedSize": 16
                    }
                ],
                "rowtype": [
                    {
                        "name": "c1",
                        "type": "TEXT",
                        "nullable": true,
                        "scale": null,
                        "byteLength": 64,
                        "length": 16,
                        "precision": null
                    }
                ]
            },
            "success": true
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();

        assert!(matches!(
            response.data.into_rowset_data(),
            RowsetData::ArrowMultiChunk { .. }
        ));
    }

    #[test]
    fn test_object_type_maps_to_object() {
        let row_type = RowType {
            name: "obj_col".to_string(),
            type_: "OBJECT".to_string(),
            nullable: true,
            scale: None,
            precision: None,
            length: Some(1024),
            byte_length: Some(4096),
            ext_type_name: None,
            vector_dimension: None,
            fields: None,
        };

        let converted: crate::query_types::RowType = (&row_type).try_into().unwrap();
        assert!(matches!(
            converted,
            crate::query_types::RowType::Object {
                ref name,
                nullable: true,
            } if name == "obj_col"
        ));
    }

    #[test]
    fn test_variant_type_maps_to_variant() {
        let row_type = RowType {
            name: "var_col".to_string(),
            type_: "VARIANT".to_string(),
            nullable: false,
            scale: None,
            precision: None,
            length: None,
            byte_length: None,
            ext_type_name: None,
            vector_dimension: None,
            fields: None,
        };

        let converted: crate::query_types::RowType = (&row_type).try_into().unwrap();
        assert!(matches!(
            converted,
            crate::query_types::RowType::Variant {
                ref name,
                nullable: false,
            } if name == "var_col"
        ));
    }

    #[test]
    fn test_array_type_maps_to_array() {
        let row_type = RowType {
            name: "arr_col".to_string(),
            type_: "ARRAY".to_string(),
            nullable: true,
            scale: None,
            precision: None,
            length: Some(512),
            byte_length: Some(2048),
            ext_type_name: None,
            vector_dimension: None,
            fields: None,
        };

        let converted: crate::query_types::RowType = (&row_type).try_into().unwrap();
        assert!(matches!(
            converted,
            crate::query_types::RowType::Array {
                ref name,
                nullable: true,
            } if name == "arr_col"
        ));
    }

    // ---------------------------------------------------------------
    // Upload encryption material parsing (to_file_upload_data)
    // ---------------------------------------------------------------

    fn make_upload_json(encryption_material_fragment: &str) -> String {
        format!(
            r#"{{
                "src_locations": ["path/to/file.csv"],
                "stageInfo": {{
                    "locationType": "GCS",
                    "location": "bucket/prefix/",
                    "creds": {{ "GCS_ACCESS_TOKEN": "fake" }},
                    "region": "us-central1"
                }},
                {encryption_material_fragment}
                "autoCompress": false,
                "sourceCompression": "NONE",
                "overwrite": false
            }}"#
        )
    }

    #[test]
    fn upload_encryption_material_null_returns_none() {
        let json = make_upload_json(r#""encryptionMaterial": null,"#);
        let data: Data = serde_json::from_str(&json).unwrap();
        let upload = data
            .to_file_upload_data(PutGetResultsetFlavor::default(), false, false)
            .unwrap();
        assert!(upload.encryption_material.is_none());
    }

    #[test]
    fn upload_encryption_material_absent_returns_none() {
        let json = make_upload_json("");
        let data: Data = serde_json::from_str(&json).unwrap();
        let upload = data
            .to_file_upload_data(PutGetResultsetFlavor::default(), false, false)
            .unwrap();
        assert!(upload.encryption_material.is_none());
    }

    #[test]
    fn upload_encryption_material_empty_array_returns_none() {
        let json = make_upload_json(r#""encryptionMaterial": [],"#);
        let data: Data = serde_json::from_str(&json).unwrap();
        let upload = data
            .to_file_upload_data(PutGetResultsetFlavor::default(), false, false)
            .unwrap();
        assert!(upload.encryption_material.is_none());
    }

    #[test]
    fn upload_encryption_material_single_returns_some() {
        let json = make_upload_json(
            r#""encryptionMaterial": {"queryStageMasterKey": "a2V5","queryId": "qid-1","smkId": "42"},"#,
        );
        let data: Data = serde_json::from_str(&json).unwrap();
        let upload = data
            .to_file_upload_data(PutGetResultsetFlavor::default(), false, false)
            .unwrap();
        assert!(upload.encryption_material.is_some());
    }

    #[test]
    fn upload_encryption_material_array_of_one_returns_some() {
        let json = make_upload_json(
            r#""encryptionMaterial": [{"queryStageMasterKey": "a2V5","queryId": "qid-1","smkId": "42"}],"#,
        );
        let data: Data = serde_json::from_str(&json).unwrap();
        let upload = data
            .to_file_upload_data(PutGetResultsetFlavor::default(), false, false)
            .unwrap();
        assert!(upload.encryption_material.is_some());
    }

    #[test]
    fn upload_encryption_material_array_of_many_returns_error() {
        let json = make_upload_json(
            r#""encryptionMaterial": [
                {"queryStageMasterKey": "a2V5","queryId": "qid-1","smkId": "1"},
                {"queryStageMasterKey": "b3l6","queryId": "qid-2","smkId": "2"}
            ],"#,
        );
        let data: Data = serde_json::from_str(&json).unwrap();
        let result = data.to_file_upload_data(PutGetResultsetFlavor::default(), false, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Expected exactly one encryption material"),
            "Error should mention the constraint: {err_msg}"
        );
    }

    #[test]
    fn upload_data_forwards_legacy_odbc_compression_autodetect_false() {
        let json = make_upload_json("");
        let data: Data = serde_json::from_str(&json).unwrap();
        let upload = data
            .to_file_upload_data(PutGetResultsetFlavor::Python, false, false)
            .unwrap();
        assert_eq!(upload.flavor, PutGetResultsetFlavor::Python);
        assert!(!upload.legacy_odbc_compression_autodetect);
    }

    #[test]
    fn upload_data_forwards_legacy_odbc_compression_autodetect_true() {
        let json = make_upload_json("");
        let data: Data = serde_json::from_str(&json).unwrap();
        let upload = data
            .to_file_upload_data(PutGetResultsetFlavor::Odbc, true, false)
            .unwrap();
        assert_eq!(upload.flavor, PutGetResultsetFlavor::Odbc);
        assert!(upload.legacy_odbc_compression_autodetect);
    }

    // Explicit `SOURCE_COMPRESSION=PARQUET` / `=ORC` parses to the matching
    // `SourceCompressionParam` variant. Mirrors Python's
    // `file_compression_type.py` which lists both with `is_supported=True`.
    fn upload_json_with_source_compression(value: &str) -> String {
        serde_json::json!({
            "src_locations": ["path/to/file.csv"],
            "stageInfo": {
                "locationType": "GCS",
                "location": "bucket/prefix/",
                "creds": { "GCS_ACCESS_TOKEN": "fake" },
                "region": "us-central1"
            },
            "autoCompress": false,
            "sourceCompression": value,
            "overwrite": false
        })
        .to_string()
    }

    #[test]
    fn upload_data_parses_explicit_source_compression_parquet() {
        for value in ["PARQUET", "parquet", "Parquet"] {
            let json = upload_json_with_source_compression(value);
            let data: Data = serde_json::from_str(&json).unwrap();
            let upload = data
                .to_file_upload_data(PutGetResultsetFlavor::Python, false, false)
                .unwrap();
            assert!(
                matches!(upload.source_compression, SourceCompressionParam::Parquet),
                "value={value:?} must parse to SourceCompressionParam::Parquet, got: {:?}",
                upload.source_compression,
            );
        }
    }

    #[test]
    fn upload_data_parses_explicit_source_compression_orc() {
        for value in ["ORC", "orc", "Orc"] {
            let json = upload_json_with_source_compression(value);
            let data: Data = serde_json::from_str(&json).unwrap();
            let upload = data
                .to_file_upload_data(PutGetResultsetFlavor::Python, false, false)
                .unwrap();
            assert!(
                matches!(upload.source_compression, SourceCompressionParam::Orc),
                "value={value:?} must parse to SourceCompressionParam::Orc, got: {:?}",
                upload.source_compression,
            );
        }
    }

    fn make_download_json() -> String {
        r#"{
            "src_locations": ["path/to/file.csv.gz"],
            "stageInfo": {
                "locationType": "GCS",
                "location": "bucket/prefix/",
                "creds": {"GCS_ACCESS_TOKEN": "fake"},
                "region": "us-central1"
            },
            "localLocation": "/tmp/dl"
        }"#
        .to_string()
    }

    #[test]
    fn download_data_forwards_flavor_python() {
        let data: Data = serde_json::from_str(&make_download_json()).unwrap();
        let download = data
            .to_file_download_data(&PutGetResultsetFlavor::Python, false)
            .unwrap();
        assert_eq!(download.flavor, PutGetResultsetFlavor::Python);
    }

    #[test]
    fn download_data_forwards_flavor_odbc() {
        let data: Data = serde_json::from_str(&make_download_json()).unwrap();
        let download = data
            .to_file_download_data(&PutGetResultsetFlavor::Odbc, false)
            .unwrap();
        assert_eq!(download.flavor, PutGetResultsetFlavor::Odbc);
    }

    // ---- presignedUrls[] alignment (gap 2.2) ----
    //
    // Cross-driver parity: Python uses `idx < len(...)` to gate
    // `meta.presigned_url`; JDBC zips arrays in
    // `SnowflakeFileTransferAgent.java:994-999` and silently skips on
    // length mismatch. Our `align_presigned_urls` matches Python's
    // tolerance (pad short, truncate long with warn).

    fn make_download_json_multi_with_urls(presigned_urls_field: &str) -> String {
        format!(
            r#"{{
                "src_locations": ["a", "b", "c"],
                "stageInfo": {{
                    "locationType": "GCS",
                    "location": "bucket/prefix/",
                    "creds": {{}},
                    "region": "us-central1"
                }},
                "localLocation": "/tmp/dl"
                {presigned_urls_field}
            }}"#
        )
    }

    #[test]
    fn download_data_copies_presigned_urls_when_aligned() {
        // GS sent one URL per source file — the common GCS GET case
        // post-2.2. Each `presigned_urls[i]` must round-trip into
        // `DownloadData.presigned_urls[i]` exactly.
        let json = make_download_json_multi_with_urls(r#", "presignedUrls": ["u0", "u1", "u2"]"#);
        let data: Data = serde_json::from_str(&json).unwrap();
        let download = data
            .to_file_download_data(&PutGetResultsetFlavor::Python, false)
            .unwrap();
        assert_eq!(
            download.presigned_urls,
            vec![
                Some("u0".to_string()),
                Some("u1".to_string()),
                Some("u2".to_string())
            ]
        );
    }

    #[test]
    fn download_data_presigned_urls_none_when_field_absent() {
        // Pre-2.2 PUT-side / S3 / Azure responses omit `presignedUrls`.
        // The alignment helper still produces a `Vec<Option<String>>`
        // matched to `src_locations.len()` so the downstream zip in
        // `download_files` is well-defined.
        let json = make_download_json_multi_with_urls("");
        let data: Data = serde_json::from_str(&json).unwrap();
        let download = data
            .to_file_download_data(&PutGetResultsetFlavor::Python, false)
            .unwrap();
        assert_eq!(download.presigned_urls, vec![None, None, None]);
    }

    #[test]
    #[tracing_test::traced_test]
    fn download_data_short_presigned_urls_pads_with_none() {
        // Stage reconfiguration mid-batch can return a partial list. Match
        // Python's `idx < len(presigned_urls)` tolerance: pad the tail
        // with `None` so the un-URL'd files fall back to the token (when
        // present) or surface `MissingGcsCredentials` per-file. A warning
        // is emitted so the mismatch is observable in the captured log.
        let json = make_download_json_multi_with_urls(r#", "presignedUrls": ["u0", "u1"]"#);
        let data: Data = serde_json::from_str(&json).unwrap();
        let download = data
            .to_file_download_data(&PutGetResultsetFlavor::Python, false)
            .unwrap();
        assert_eq!(
            download.presigned_urls,
            vec![Some("u0".to_string()), Some("u1".to_string()), None]
        );
        assert!(
            logs_contain("shorter than src_locations"),
            "expected token-fallback warning to be logged"
        );
    }

    #[test]
    #[tracing_test::traced_test]
    fn download_data_long_presigned_urls_ignores_extras_with_warn() {
        // GS occasionally over-emits during stage reconfigurations. Default
        // (non-JDBC) flavor: ignore extras, but emit a structured warning so
        // the mismatch is observable in the captured log.
        let json = make_download_json_multi_with_urls(
            r#", "presignedUrls": ["u0", "u1", "u2", "u3", "u4"]"#,
        );
        let data: Data = serde_json::from_str(&json).unwrap();
        let download = data
            .to_file_download_data(&PutGetResultsetFlavor::Python, false)
            .unwrap();
        assert_eq!(
            download.presigned_urls,
            vec![
                Some("u0".to_string()),
                Some("u1".to_string()),
                Some("u2".to_string())
            ]
        );
        assert!(
            logs_contain("longer than src_locations"),
            "expected ignore-extras warning to be logged"
        );
        assert!(
            !logs_contain("u3"),
            "presigned URL contents must not be logged"
        );
        assert!(
            !logs_contain("u4"),
            "presigned URL contents must not be logged"
        );
    }

    #[test]
    fn download_data_empty_presigned_urls_pads_to_src_locations_length() {
        // GS may send `presignedUrls: []` on a non-presigned-only path.
        // Treat as fully-absent: every slot becomes `None`.
        let json = make_download_json_multi_with_urls(r#", "presignedUrls": []"#);
        let data: Data = serde_json::from_str(&json).unwrap();
        let download = data
            .to_file_download_data(&PutGetResultsetFlavor::Python, false)
            .unwrap();
        assert_eq!(download.presigned_urls, vec![None, None, None]);
    }

    #[test]
    fn align_presigned_urls_helper_returns_empty_when_no_src_locations() {
        // Sanity check on the helper: zero source files means zero slots,
        // even when GS sent a URL list (degenerate input — should not
        // panic on the ignore-extras path).
        let aligned = align_presigned_urls(
            Some(&[Some("u0".to_string()), Some("u1".to_string())]),
            &Vec::<String>::new(),
            OverlongPresignedUrlsPolicy::IgnoreExtras,
            UnderlongPresignedUrlsPolicy::TokenFallback,
        );
        assert!(aligned.is_empty());
    }

    #[test]
    fn download_data_null_elements_in_presigned_urls_treated_as_none() {
        // S3/SSE GET responses include `"presignedUrls": [null]` (one null
        // per file). Before this fix, `Vec<String>` rejected null with a
        // serde type error; `Vec<Option<String>>` silently promotes each
        // null to `None`, letting the file fall back to stage credentials.
        let json = make_download_json_multi_with_urls(r#", "presignedUrls": [null, "u1", null]"#);
        let data: Data = serde_json::from_str(&json).unwrap();
        let download = data
            .to_file_download_data(&PutGetResultsetFlavor::Python, false)
            .unwrap();
        assert_eq!(
            download.presigned_urls,
            vec![None, Some("u1".to_string()), None]
        );
    }

    #[test]
    fn download_data_all_null_presigned_urls_treated_as_all_none() {
        // The exact S3/SSE GET shape: every slot is null. The result must
        // be identical to the field-absent case — all `None` — so that
        // the downstream transfer code uses stage credentials uniformly.
        let json = make_download_json_multi_with_urls(r#", "presignedUrls": [null, null, null]"#);
        let data: Data = serde_json::from_str(&json).unwrap();
        let download = data
            .to_file_download_data(&PutGetResultsetFlavor::Python, false)
            .unwrap();
        assert_eq!(download.presigned_urls, vec![None, None, None]);
    }

    #[test]
    fn download_data_long_presigned_urls_fallback_to_credentials_for_jdbc() {
        let json = make_download_json_multi_with_urls(
            r#", "presignedUrls": ["u0", "u1", "u2", "u3", "u4"]"#,
        );
        let data: Data = serde_json::from_str(&json).unwrap();
        let download = data
            .to_file_download_data(&PutGetResultsetFlavor::Jdbc, false)
            .unwrap();
        assert_eq!(download.presigned_urls, vec![None, None, None]);
    }

    #[test]
    fn download_data_short_presigned_urls_fallback_to_credentials_for_jdbc() {
        let json = make_download_json_multi_with_urls(r#", "presignedUrls": ["u0", "u1"]"#);
        let data: Data = serde_json::from_str(&json).unwrap();
        let download = data
            .to_file_download_data(&PutGetResultsetFlavor::Jdbc, false)
            .unwrap();
        assert_eq!(download.presigned_urls, vec![None, None, None]);
    }

    #[test]
    #[tracing_test::traced_test]
    fn align_presigned_urls_fallback_to_credentials_over_long() {
        let urls: Vec<Option<String>> = (0..5).map(|i| Some(format!("u{i}"))).collect();
        let src = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let aligned = align_presigned_urls(
            Some(&urls),
            &src,
            OverlongPresignedUrlsPolicy::FallbackToCredentials,
            UnderlongPresignedUrlsPolicy::TokenFallback,
        );
        assert_eq!(aligned, vec![None, None, None]);
        assert!(
            logs_contain("falling back to credentials for all files"),
            "expected fallback warning to be logged"
        );
    }

    #[test]
    #[tracing_test::traced_test]
    fn align_presigned_urls_fallback_to_credentials_under_long() {
        let urls: Vec<Option<String>> = vec![Some("u0".to_string()), Some("u1".to_string())];
        let src = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let aligned = align_presigned_urls(
            Some(&urls),
            &src,
            OverlongPresignedUrlsPolicy::IgnoreExtras,
            UnderlongPresignedUrlsPolicy::FallbackToCredentials,
        );
        assert_eq!(aligned, vec![None, None, None]);
        assert!(
            logs_contain("falling back to credentials for all files"),
            "expected fallback warning to be logged"
        );
    }

    #[test]
    fn test_unsupported_column_type_returns_error() {
        let row_type = RowType {
            name: "bad_col".to_string(),
            type_: "UNSUPPORTED_TYPE_XYZ".to_string(),
            nullable: false,
            scale: None,
            precision: None,
            length: None,
            byte_length: None,
            ext_type_name: None,
            vector_dimension: None,
            fields: None,
        };

        let result: Result<crate::query_types::RowType, _> = (&row_type).try_into();
        match result {
            Err(err) => assert!(
                err.to_string().contains("UNSUPPORTED_TYPE_XYZ"),
                "Error should mention the unsupported type: {err}"
            ),
            Ok(_) => panic!("Expected error for unsupported column type UNSUPPORTED_TYPE_XYZ"),
        }
    }

    #[test]
    fn test_geography_type_is_supported() {
        let row_type = RowType {
            name: "col".to_string(),
            type_: "GEOGRAPHY".to_string(),
            nullable: true,
            scale: None,
            precision: None,
            length: None,
            byte_length: None,
            ext_type_name: None,
            vector_dimension: None,
            fields: None,
        };

        let result: crate::query_types::RowType = (&row_type).try_into().unwrap();
        assert!(matches!(
            result,
            crate::query_types::RowType::Geography { .. }
        ));
    }

    /// Server sends `type=text` + `extTypeName=GEOGRAPHY` for WKT / EWKT output.
    #[test]
    fn test_geography_text_representation_from_text_underlying_type() {
        let row_type = RowType {
            name: "geo_col".to_string(),
            type_: "text".to_string(),
            nullable: true,
            scale: None,
            precision: None,
            length: Some(134_217_728),
            byte_length: Some(134_217_728),
            ext_type_name: Some("GEOGRAPHY".to_string()),
            vector_dimension: None,
            fields: None,
        };
        let result: crate::query_types::RowType = (&row_type).try_into().unwrap();
        assert!(matches!(
            result,
            crate::query_types::RowType::Geography {
                representation: crate::query_types::GeoRepresentation::Text,
                ..
            }
        ));
    }

    /// Server sends `type=object` + `extTypeName=GEOGRAPHY` for GeoJSON output.
    #[test]
    fn test_geography_object_underlying_type_maps_to_text_representation() {
        let row_type = RowType {
            name: "geo_col".to_string(),
            type_: "object".to_string(),
            nullable: true,
            scale: None,
            precision: None,
            length: None,
            byte_length: None,
            ext_type_name: Some("GEOGRAPHY".to_string()),
            vector_dimension: None,
            fields: None,
        };
        let result: crate::query_types::RowType = (&row_type).try_into().unwrap();
        assert!(matches!(
            result,
            crate::query_types::RowType::Geography {
                representation: crate::query_types::GeoRepresentation::Text,
                ..
            }
        ));
    }

    /// Server sends `type=binary` + `extTypeName=GEOGRAPHY` for WKB / EWKB output.
    #[test]
    fn test_geography_binary_representation_from_binary_underlying_type() {
        let row_type = RowType {
            name: "geo_col".to_string(),
            type_: "binary".to_string(),
            nullable: true,
            scale: None,
            precision: None,
            length: Some(67_108_864),
            byte_length: Some(67_108_864),
            ext_type_name: Some("GEOGRAPHY".to_string()),
            vector_dimension: None,
            fields: None,
        };
        let result: crate::query_types::RowType = (&row_type).try_into().unwrap();
        assert!(matches!(
            result,
            crate::query_types::RowType::Geography {
                representation: crate::query_types::GeoRepresentation::Binary,
                ..
            }
        ));
    }

    #[test]
    fn test_geometry_type_is_supported() {
        let row_type = RowType {
            name: "col".to_string(),
            type_: "GEOMETRY".to_string(),
            nullable: true,
            scale: None,
            precision: None,
            length: None,
            byte_length: None,
            ext_type_name: None,
            vector_dimension: None,
            fields: None,
        };

        let result: crate::query_types::RowType = (&row_type).try_into().unwrap();
        assert!(matches!(
            result,
            crate::query_types::RowType::Geometry { .. }
        ));
    }

    fn make_row_type(name: &str, type_: &str, nullable: bool) -> RowType {
        RowType {
            name: name.to_string(),
            type_: type_.to_string(),
            nullable,
            scale: None,
            precision: None,
            length: None,
            byte_length: None,
            ext_type_name: None,
            vector_dimension: None,
            fields: None,
        }
    }

    fn vector_field_metadata(type_: &str) -> FieldMetadata {
        FieldMetadata {
            type_: type_.to_string(),
            _name: None,
            _nullable: true,
            _length: None,
            _scale: None,
            _precision: None,
            _fields: None,
        }
    }

    fn make_vector_row_type(dimension: Option<u64>, element_type: Option<&str>) -> RowType {
        let mut row = make_row_type("col", "VECTOR", true);
        row.vector_dimension = dimension;
        row.fields = element_type.map(|t| vec![vector_field_metadata(t)]);
        row
    }

    #[test]
    fn test_vector_int_type_carries_dimension_and_element() {
        let row_type = make_vector_row_type(Some(3), Some("FIXED"));
        let result: crate::query_types::RowType = (&row_type).try_into().unwrap();
        assert!(matches!(
            result,
            crate::query_types::RowType::Vector {
                dimension: 3,
                element_type: crate::query_types::VectorElementType::Int32,
                ..
            }
        ));
    }

    #[test]
    fn test_vector_float_type_carries_dimension_and_element() {
        let row_type = make_vector_row_type(Some(5), Some("REAL"));
        let result: crate::query_types::RowType = (&row_type).try_into().unwrap();
        assert!(matches!(
            result,
            crate::query_types::RowType::Vector {
                dimension: 5,
                element_type: crate::query_types::VectorElementType::Float32,
                ..
            }
        ));
    }

    #[test]
    fn test_vector_element_type_is_case_insensitive() {
        let row_type = make_vector_row_type(Some(2), Some("fixed"));
        let result: crate::query_types::RowType = (&row_type).try_into().unwrap();
        assert!(matches!(
            result,
            crate::query_types::RowType::Vector {
                element_type: crate::query_types::VectorElementType::Int32,
                ..
            }
        ));
    }

    #[test]
    fn test_vector_unsupported_element_type_returns_error() {
        let row_type = make_vector_row_type(Some(3), Some("TEXT"));
        let result: Result<crate::query_types::RowType, _> = (&row_type).try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_vector_missing_dimension_returns_error() {
        let row_type = make_vector_row_type(None, Some("FIXED"));
        let result: Result<crate::query_types::RowType, _> = (&row_type).try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_vector_missing_fields_returns_error() {
        let row_type = make_vector_row_type(Some(3), None);
        let result: Result<crate::query_types::RowType, _> = (&row_type).try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_fixed_missing_precision_returns_error() {
        let row_type = RowType {
            name: "num_col".to_string(),
            type_: "FIXED".to_string(),
            nullable: false,
            scale: Some(2),
            precision: None,
            length: None,
            byte_length: None,
            ext_type_name: None,
            vector_dimension: None,
            fields: None,
        };

        let result: Result<crate::query_types::RowType, _> = (&row_type).try_into();
        match result {
            Err(err) => assert!(
                err.to_string().contains("precision"),
                "Error should mention missing precision: {err}"
            ),
            Ok(_) => panic!("Expected error for FIXED column without precision"),
        }
    }

    #[test]
    fn test_fixed_missing_scale_returns_error() {
        let row_type = RowType {
            name: "num_col".to_string(),
            type_: "FIXED".to_string(),
            nullable: false,
            scale: None,
            precision: Some(38),
            length: None,
            byte_length: None,
            ext_type_name: None,
            vector_dimension: None,
            fields: None,
        };

        let result: Result<crate::query_types::RowType, _> = (&row_type).try_into();
        match result {
            Err(err) => assert!(
                err.to_string().contains("scale"),
                "Error should mention missing scale: {err}"
            ),
            Ok(_) => panic!("Expected error for FIXED column without scale"),
        }
    }

    #[test]
    fn test_binary_missing_length_returns_error() {
        let row_type = RowType {
            name: "bin_col".to_string(),
            type_: "BINARY".to_string(),
            nullable: false,
            scale: None,
            precision: None,
            length: None,
            byte_length: Some(100),
            ext_type_name: None,
            vector_dimension: None,
            fields: None,
        };

        let result: Result<crate::query_types::RowType, _> = (&row_type).try_into();
        match result {
            Err(err) => assert!(
                err.to_string().contains("length"),
                "Error should mention missing length: {err}"
            ),
            Ok(_) => panic!("Expected error for BINARY column without length"),
        }
    }

    #[test]
    fn test_binary_missing_byte_length_returns_error() {
        let row_type = RowType {
            name: "bin_col".to_string(),
            type_: "BINARY".to_string(),
            nullable: false,
            scale: None,
            precision: None,
            length: Some(100),
            byte_length: None,
            ext_type_name: None,
            vector_dimension: None,
            fields: None,
        };

        let result: Result<crate::query_types::RowType, _> = (&row_type).try_into();
        match result {
            Err(err) => assert!(
                err.to_string().contains("byte length"),
                "Error should mention missing byte length: {err}"
            ),
            Ok(_) => panic!("Expected error for BINARY column without byte_length"),
        }
    }

    #[test]
    fn test_ext_type_name_takes_precedence_over_type() {
        // Server sends type="object" but extTypeName="GEOGRAPHY" for geography columns
        let row_type = RowType {
            name: "geo_col".to_string(),
            type_: "object".to_string(),
            nullable: true,
            scale: None,
            precision: None,
            length: None,
            byte_length: None,
            ext_type_name: Some("GEOGRAPHY".to_string()),
            vector_dimension: None,
            fields: None,
        };

        let converted: crate::query_types::RowType = (&row_type).try_into().unwrap();
        assert!(matches!(
            converted,
            crate::query_types::RowType::Geography {
                ref name,
                nullable: true,
                ..
            } if name == "geo_col"
        ));
    }

    #[test]
    fn test_empty_ext_type_name_falls_back_to_type() {
        // Stored procedures may return ext_type_name="" with type="text"
        let row_type = RowType {
            name: "RESULT_COL".to_string(),
            type_: "text".to_string(),
            nullable: false,
            scale: None,
            precision: None,
            length: Some(100),
            byte_length: Some(400),
            ext_type_name: Some("".to_string()),
            vector_dimension: None,
            fields: None,
        };

        let converted: crate::query_types::RowType = (&row_type).try_into().unwrap();
        assert!(matches!(
            converted,
            crate::query_types::RowType::Text {
                ref name,
                nullable: false,
                ..
            } if name == "RESULT_COL"
        ));
    }

    #[test]
    fn test_query_context_entry_id_exceeding_i32_max() {
        let json = r#"{
            "data": {
                "queryContext": {
                    "entries": [
                        {"id": 3575747553, "timestamp": 1681400000, "priority": 0, "context": "some_ctx"}
                    ]
                }
            },
            "success": true
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();
        assert!(response.success);
    }

    #[test]
    fn test_query_context_entry_missing_context_field() {
        let json = r#"{
            "data": {
                "queryContext": {
                    "entries": [
                        {"id": 42, "timestamp": 1681400000, "priority": 1}
                    ]
                }
            },
            "success": true
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();
        assert!(response.success);
    }

    #[test]
    fn test_query_context_entry_with_all_large_values() {
        let json = r#"{
            "data": {
                "queryContext": {
                    "entries": [
                        {"id": 3575748941, "timestamp": 9999999999999, "priority": 3000000000, "context": "ctx"},
                        {"id": 3575748745, "timestamp": 1681400000, "priority": 0}
                    ]
                }
            },
            "success": true
        }"#;

        let response: Response = serde_json::from_str(json).unwrap();
        assert!(response.success);
    }

    #[test]
    fn test_text_type_without_length_uses_default() {
        // Regression: the server may return DECFLOAT columns as TEXT without
        // length/byteLength metadata when it doesn't recognize the client as
        // DECFLOAT-capable. The driver must use defaults instead of failing.
        let row_type = RowType {
            name: "TEST_VALUE".to_string(),
            type_: "TEXT".to_string(),
            nullable: true,
            scale: None,
            precision: None,
            length: None,
            byte_length: None,
            ext_type_name: None,
            vector_dimension: None,
            fields: None,
        };

        let result: crate::query_types::RowType = (&row_type)
            .try_into()
            .expect("TEXT column without length should use defaults, not fail");
        match result {
            crate::query_types::RowType::Text {
                length,
                byte_length,
                ..
            } => {
                assert_eq!(length, DEFAULT_TEXT_LENGTH);
                assert_eq!(
                    byte_length,
                    DEFAULT_TEXT_LENGTH.saturating_mul(DEFAULT_TEXT_BYTE_LENGTH_MULTIPLIER)
                );
            }
            _ => panic!("Expected RowType::Text"),
        }
    }

    // --- StageInfo regional URL plumbing ---
    //
    // Build a minimal S3 stage-info JSON, deserialize, and convert to the
    // public `file_manager::StageInfo`. Asserts the OR semantics for
    // `useS3RegionalUrl` / `useRegionalUrl` mirror the reference Python /
    // JDBC / libsnowflakeclient S3 paths.

    fn s3_stage_info_value(
        use_s3_regional: Option<bool>,
        use_regional: Option<bool>,
    ) -> serde_json::Value {
        let mut value = serde_json::json!({
            "locationType": "S3",
            "location": "my-bucket/some/prefix/",
            "region": "us-east-1",
            "endPoint": null,
            "creds": {
                "AWS_KEY_ID": "k",
                "AWS_SECRET_KEY": "s",
                "AWS_TOKEN": "t",
            },
        });
        let obj = value.as_object_mut().expect("stage-info JSON is an object");
        if let Some(b) = use_s3_regional {
            obj.insert("useS3RegionalUrl".to_string(), serde_json::Value::Bool(b));
        }
        if let Some(b) = use_regional {
            obj.insert("useRegionalUrl".to_string(), serde_json::Value::Bool(b));
        }
        value
    }

    fn parse_s3_stage_info(value: serde_json::Value) -> file_manager::StageInfo {
        let raw: super::StageInfo = serde_json::from_value(value).expect("parse stage info json");
        (&raw).try_into().expect("convert stage info")
    }

    #[test]
    fn use_s3_regional_url_propagates_when_only_s3_flag_set() {
        let info = parse_s3_stage_info(s3_stage_info_value(Some(true), Some(false)));
        assert!(info.use_s3_regional_url);
    }

    #[test]
    fn use_s3_regional_url_propagates_when_only_generic_regional_flag_set() {
        // Mirrors Python's `useS3RegionalUrl OR useRegionalUrl` semantics.
        let info = parse_s3_stage_info(s3_stage_info_value(Some(false), Some(true)));
        assert!(info.use_s3_regional_url);
    }

    #[test]
    fn use_s3_regional_url_false_when_neither_flag_set() {
        let info = parse_s3_stage_info(s3_stage_info_value(Some(false), Some(false)));
        assert!(!info.use_s3_regional_url);
    }

    #[test]
    fn use_s3_regional_url_false_when_both_flags_absent() {
        // Older GS responses may omit both fields. Default must be false so
        // we keep talking to the global `s3.amazonaws.com` endpoint.
        let info = parse_s3_stage_info(s3_stage_info_value(None, None));
        assert!(!info.use_s3_regional_url);
    }

    // --- Session-parameter lookup (ENABLE_STAGE_S3_PRIVATELINK_FOR_US_EAST_1) ---
    //
    // Tests `read_use_s3_regional_url_session_param` directly. The boolean
    // result is OR'd into `StageInfo.use_s3_regional_url` at the PUT/GET
    // dispatch site (see `to_file_upload_data` / `to_file_download_data`).
    // Mirrors the OR-with-session-parameter semantics implemented in the
    // Python connector, JDBC, and libsnowflakeclient.

    fn build_session_params(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn read_session_param_returns_true_when_set() {
        let params = build_session_params(&[("ENABLE_STAGE_S3_PRIVATELINK_FOR_US_EAST_1", "true")]);
        assert!(read_use_s3_regional_url_session_param(&params));
    }

    #[test]
    fn read_session_param_value_lookup_is_case_insensitive() {
        // GS may push the value as `TRUE` or `True`; we must accept all
        // common cases. Keys, by contrast, are uppercased upstream by
        // `apis::database_driver_v1::connection`, so the helper does a
        // direct `get` on the canonical uppercase key and we don't need
        // to test other key casings here.
        let params = build_session_params(&[("ENABLE_STAGE_S3_PRIVATELINK_FOR_US_EAST_1", "TRUE")]);
        assert!(read_use_s3_regional_url_session_param(&params));
    }

    #[test]
    fn read_session_param_lowercase_key_returns_false() {
        // Documents the upstream invariant: `session_parameters` keys are
        // uppercased by `connection.rs` write sites. A lowercase key must
        // not be matched here, because if it ever appears the bug is in
        // the upstream normalization, not in this lookup.
        let params = build_session_params(&[("enable_stage_s3_privatelink_for_us_east_1", "true")]);
        assert!(!read_use_s3_regional_url_session_param(&params));
    }

    #[test]
    fn read_session_param_accepts_numeric_one() {
        let params = build_session_params(&[("ENABLE_STAGE_S3_PRIVATELINK_FOR_US_EAST_1", "1")]);
        assert!(read_use_s3_regional_url_session_param(&params));
    }

    #[test]
    fn read_session_param_returns_false_when_absent() {
        assert!(!read_use_s3_regional_url_session_param(&HashMap::new()));
    }

    #[test]
    fn read_session_param_returns_false_when_explicitly_false() {
        let params =
            build_session_params(&[("ENABLE_STAGE_S3_PRIVATELINK_FOR_US_EAST_1", "false")]);
        assert!(!read_use_s3_regional_url_session_param(&params));
    }

    #[test]
    fn read_session_param_unrelated_keys_ignored() {
        let params = build_session_params(&[
            ("CLIENT_PREFETCH_THREADS", "8"),
            ("CLIENT_SESSION_KEEP_ALIVE", "true"),
        ]);
        assert!(!read_use_s3_regional_url_session_param(&params));
    }

    // Integration check: the boolean parameter wires through
    // `to_file_upload_data` and ORs into `StageInfo.use_s3_regional_url`.

    fn make_upload_data_for_s3_regional_url_test(
        stage_info_value: serde_json::Value,
        use_s3_regional_url_session_param: bool,
    ) -> file_manager::UploadData {
        let payload = serde_json::json!({
            "command": "UPLOAD",
            "src_locations": ["/tmp/upload.csv"],
            "stageInfo": stage_info_value,
            "autoCompress": true,
            "sourceCompression": "NONE",
        });
        let data: Data = serde_json::from_value(payload).expect("build upload Data");
        data.to_file_upload_data(
            PutGetResultsetFlavor::default(),
            false,
            use_s3_regional_url_session_param,
        )
        .expect("convert to UploadData")
    }

    #[test]
    fn upload_data_session_param_true_forces_regional_when_stage_info_false() {
        let upload = make_upload_data_for_s3_regional_url_test(
            s3_stage_info_value(Some(false), Some(false)),
            true,
        );
        assert!(upload.stage_info.use_s3_regional_url);
    }

    #[test]
    fn upload_data_session_param_false_does_not_mask_stage_info_true() {
        // Stage-info already true; session-parameter false must not flip it
        // back to false. The OR is one-directional.
        let upload = make_upload_data_for_s3_regional_url_test(
            s3_stage_info_value(Some(true), Some(false)),
            false,
        );
        assert!(upload.stage_info.use_s3_regional_url);
    }
}
