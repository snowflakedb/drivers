use crate::chunks::ChunkDownloadData;
use crate::file_manager::SourceCompressionParam;
use crate::{file_manager, query_types};
use serde::Deserialize;
use snafu::{OptionExt, Snafu};
use std::collections::HashMap;
// TODO: Delete all unused fields when we are sure they are not needed

#[derive(Deserialize)]
pub struct Response {
    pub data: Data,
    #[serde(rename = "message")]
    pub message: Option<String>,
    #[serde(rename = "code")]
    pub code: Option<String>,
    #[serde(rename = "success")]
    pub success: bool,
}

#[derive(Deserialize)]
pub struct Data {
    #[serde(rename = "rowset")]
    pub rowset: Option<Vec<Vec<Option<String>>>>,
    #[serde(rename = "rowsetBase64")]
    pub rowset_base64: Option<String>,
    #[serde(rename = "rowtype")]
    pub(crate) row_type: Option<Vec<RowType>>,
    #[serde(rename = "command")]
    pub command: Option<String>,

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
    _result_ids: Option<String>,
    #[serde(rename = "resultTypes")]
    _result_types: Option<String>,
    #[serde(rename = "queryResultFormat")]
    query_result_format: Option<String>,
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
    #[serde(rename = "presignedUrls")]
    _presigned_urls: Option<serde_json::Value>,
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
    _id: i32,
    #[serde(rename = "timestamp")]
    _timestamp: i64,
    #[serde(rename = "priority")]
    _priority: i32,
    #[serde(rename = "context")]
    _context: String,
}

#[derive(Deserialize)]
pub struct Chunk {
    #[serde(rename = "url")]
    url: String,
    //unused fields
    #[serde(rename = "rowCount")]
    _row_count: i32,
    #[serde(rename = "uncompressedSize")]
    _uncompressed_size: i64,
    #[serde(rename = "compressedSize")]
    _compressed_size: i64,
}

#[derive(Deserialize)]
pub struct SnowflakeResult {}

#[derive(Deserialize)]
pub struct SnowflakeRows {}

/// Statistics for DML operations (INSERT, UPDATE, DELETE)
#[derive(Deserialize, Default)]
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

#[derive(Deserialize, Debug)]
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

    // unused fields
    #[serde(rename = "fields")]
    pub _fields: Option<Vec<FieldMetadata>>,
}

#[derive(Debug, Deserialize)]
pub struct FieldMetadata {
    //unused fields
    #[serde(rename = "name")]
    _name: Option<String>,
    #[serde(rename = "type")]
    _type_: String,
    #[serde(rename = "nullable")]
    _nullable: bool,
    #[serde(rename = "length")]
    _length: i32,
    #[serde(rename = "scale")]
    _scale: i32,
    #[serde(rename = "precision")]
    _precision: i32,
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
    end_point: Option<String>,

    #[serde(rename = "locationType")]
    location_type: Option<String>,
    #[serde(rename = "presignedUrl")]
    presigned_url: Option<String>,

    // unused fields
    #[serde(rename = "path")]
    _path: Option<String>,
    #[serde(rename = "storageAccount")]
    _storage_account: Option<String>,
    #[serde(rename = "isClientSideEncrypted")]
    _is_client_side_encrypted: Option<bool>,
    #[serde(rename = "useS3RegionalUrl")]
    _use_s3_regional_url: Option<bool>,
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

    // unused fields
    #[serde(rename = "AWS_ID")]
    _aws_id: Option<String>,
    #[serde(rename = "AWS_KEY")]
    _aws_key: Option<String>,
    #[serde(rename = "AZURE_SAS_TOKEN")]
    _azure_sas_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EncryptionMaterial {
    #[serde(rename = "queryStageMasterKey")]
    query_stage_master_key: String,
    #[serde(rename = "queryId")]
    query_id: String,
    #[serde(rename = "smkId")]
    smk_id: i64,
}

impl Data {
    /// Copies the fields necessary for file transfer.
    pub fn to_file_upload_data(&self) -> Result<file_manager::UploadData, QueryResponseError> {
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

        let stage_info: file_manager::StageInfo = self
            .stage_info
            .as_ref()
            .context(MissingParameterSnafu {
                parameter: "stage info",
            })?
            .try_into()?;

        let encryption_materials: Vec<_> = self
            .encryption_material
            .as_ref()
            .context(MissingParameterSnafu {
                parameter: "encryption material",
            })?
            .into();

        if encryption_materials.len() != 1 {
            InvalidFormatSnafu {
                message: "Expected exactly one encryption material for upload".to_string(),
            }
            .fail()?;
        }

        let encryption_material = encryption_materials
            .first()
            .context(MissingParameterSnafu {
                parameter: "encryption material",
            })?
            .clone();

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

        // TODO: We should support other names for existing compression types that were supported in Python Connector,
        // like "BR" and "X-BR" for Brotli etc.
        let source_compression = match source_compression_string.to_uppercase().as_str() {
            "AUTO_DETECT" => SourceCompressionParam::AutoDetect,
            "GZIP" => SourceCompressionParam::Gzip,
            "BZIP2" => SourceCompressionParam::Bzip2,
            "BROTLI" => SourceCompressionParam::Brotli,
            "ZSTD" => SourceCompressionParam::Zstd,
            "DEFLATE" => SourceCompressionParam::Deflate,
            "RAW_DEFLATE" => SourceCompressionParam::RawDeflate,
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
        })
    }

    pub fn to_file_download_data(&self) -> Result<file_manager::DownloadData, QueryResponseError> {
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

        let stage_info: file_manager::StageInfo = self
            .stage_info
            .as_ref()
            .context(MissingParameterSnafu {
                parameter: "stage info",
            })?
            .try_into()?;

        let encryption_materials: Vec<_> = self
            .encryption_material
            .as_ref()
            .context(MissingParameterSnafu {
                parameter: "encryption material",
            })?
            .into();

        if src_locations.len() != encryption_materials.len() {
            InvalidFormatSnafu {
                message: "Number of source locations must match number of encryption materials"
                    .to_string(),
            }
            .fail()?;
        }

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
        })
    }

    pub fn to_rowset_data<'a>(&'a self) -> RowsetData<'a> {
        match self.query_result_format.as_deref() {
            Some("arrow") => {
                match (
                    self.to_initial_base64_opt(),
                    self.to_chunk_download_data(),
                    self.row_type.as_ref(),
                ) {
                    (initial_base64_opt, Some(chunk_download_data), _) => {
                        RowsetData::ArrowMultiChunk {
                            initial_base64_opt,
                            chunk_download_data,
                        }
                    }
                    (Some(chunk_base64), None, _) => RowsetData::ArrowSingleChunk { chunk_base64 },
                    (None, None, Some(rowtype)) => RowsetData::SchemaOnly { rowtype },
                    _ => {
                        tracing::error!(
                            "Initial base64 and/or chunk download data are missing for Arrow result format"
                        );
                        RowsetData::NoData
                    }
                }
            }
            Some("json") => {
                if let Some((rowset, rowtype)) = self.to_json_rowset() {
                    match self.to_chunk_download_data() {
                        Some(chunk_download_data) => RowsetData::JsonMultiChunk {
                            rowset,
                            rowtype,
                            chunk_download_data,
                        },
                        None => RowsetData::JsonRowset { rowset, rowtype },
                    }
                } else {
                    tracing::error!("Rowset and/or rowtype are missing for JSON result format");
                    RowsetData::NoData
                }
            }
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
                    .map(|chunk| ChunkDownloadData::new(&chunk.url, chunk_headers))
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

#[derive(Debug)]
pub enum RowsetData<'a> {
    SchemaOnly {
        rowtype: &'a Vec<RowType>,
    },
    ArrowMultiChunk {
        initial_base64_opt: Option<&'a str>,
        chunk_download_data: Vec<ChunkDownloadData>,
    },
    ArrowSingleChunk {
        chunk_base64: &'a str,
    },
    JsonRowset {
        rowset: &'a Vec<Vec<Option<String>>>,
        rowtype: &'a Vec<RowType>,
    },
    JsonMultiChunk {
        rowset: &'a Vec<Vec<Option<String>>>,
        rowtype: &'a Vec<RowType>,
        chunk_download_data: Vec<ChunkDownloadData>,
    },
    NoData,
}

impl TryFrom<&RowType> for query_types::RowType {
    type Error = QueryResponseError;

    fn try_from(value: &RowType) -> Result<Self, Self::Error> {
        let name = value.name.clone();
        let nullable = value.nullable;

        match value.type_.to_uppercase().as_str() {
            "TEXT" => {
                let length = value.length.context(MissingParameterSnafu {
                    parameter: format!(
                        "row type -> length for TEXT/STRING/VARCHAR/CHAR column '{name}'"
                    ),
                })?;

                let byte_length = value.byte_length.context(MissingParameterSnafu {
                    parameter: format!(
                        "row type -> byte length for TEXT/STRING/VARCHAR/CHAR column '{name}'"
                    ),
                })?;

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
            "DECFLOAT" => Ok(query_types::RowType::decfloat(&name, nullable)),
            "OBJECT" => Ok(query_types::RowType::object(&name, nullable)),
            "ARRAY" => Ok(query_types::RowType::array(&name, nullable)),
            "VARIANT" => Ok(query_types::RowType::variant(&name, nullable)),
            other => InvalidFormatSnafu {
                message: format!("Unsupported column type '{other}' for column '{name}'"),
            }
            .fail(),
        }
    }
}

impl TryFrom<&StageInfo> for file_manager::StageInfo {
    type Error = QueryResponseError;

    fn try_from(value: &StageInfo) -> Result<Self, Self::Error> {
        // Determine location type (default to S3 for backward compatibility)
        let location_type = match value.location_type.as_deref() {
            Some("GCS") => file_manager::LocationType::Gcs,
            Some("AZURE") => {
                return UnsupportedStorageTypeSnafu {
                    storage_type: "Azure",
                }
                .fail();
            }
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
            file_manager::LocationType::Azure => unreachable!("Azure rejected above"),
        };

        let end_point = value
            .end_point
            .as_ref()
            .filter(|ep| !ep.is_empty())
            .cloned();

        let presigned_url = value
            .presigned_url
            .as_ref()
            .filter(|url| !url.is_empty())
            .cloned();

        // ME-CENTRAL2 always uses regional URLs, regardless of the flag
        let use_regional_url =
            value.use_regional_url.unwrap_or(false) || region.eq_ignore_ascii_case("me-central2");
        let use_virtual_url = value.use_virtual_url.unwrap_or(false);

        Ok(file_manager::StageInfo {
            location_type,
            bucket,
            key_prefix,
            region,
            creds,
            end_point,
            presigned_url,
            use_virtual_url,
            use_regional_url,
        })
    }
}

impl From<&EncryptionMaterial> for file_manager::EncryptionMaterial {
    fn from(value: &EncryptionMaterial) -> Self {
        Self {
            query_stage_master_key: value.query_stage_master_key.clone().into(),
            query_id: value.query_id.clone(),
            // Snowflake sends smk_id as i64, but later expects it as a string
            smk_id: value.smk_id.to_string(),
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
    #[snafu(display("Unsupported storage type: {storage_type}"))]
    UnsupportedStorageType {
        storage_type: &'static str,
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
            response.data.to_rowset_data(),
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
            _fields: None,
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
            _fields: None,
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
            _fields: None,
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

    #[test]
    fn test_unsupported_column_type_returns_error() {
        let row_type = RowType {
            name: "bad_col".to_string(),
            type_: "GEOGRAPHY".to_string(),
            nullable: false,
            scale: None,
            precision: None,
            length: None,
            byte_length: None,
            _fields: None,
        };

        let result: Result<crate::query_types::RowType, _> = (&row_type).try_into();
        match result {
            Err(err) => assert!(
                err.to_string().contains("GEOGRAPHY"),
                "Error should mention the unsupported type: {err}"
            ),
            Ok(_) => panic!("Expected error for unsupported column type GEOGRAPHY"),
        }
    }
}
