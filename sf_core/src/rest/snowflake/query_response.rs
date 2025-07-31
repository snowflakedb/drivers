use crate::file_manager;
use crate::rest::RestError;
use serde::Deserialize;
use std::collections::HashMap;

// TODO: Delete all unused fields when we are sure they are not needed

#[derive(Deserialize)]
pub struct Response {
    pub data: Data,
    #[serde(rename = "message")]
    pub message: Option<String>,
    #[serde(rename = "code")]
    _code: Option<String>,
    #[serde(rename = "success")]
    pub success: bool,
}

#[derive(Deserialize)]
pub struct Data {
    #[serde(rename = "rowset")]
    pub rowset: Option<Vec<Vec<Option<String>>>>,
    #[serde(rename = "rowsetBase64")]
    pub rowset_base64: Option<String>,
    #[serde(rename = "command")]
    pub command: Option<String>,

    // file transfer response data
    #[serde(rename = "src_locations")]
    src_locations: Option<Vec<String>>,
    #[serde(rename = "stageInfo")]
    stage_info: Option<StageInfo>,
    #[serde(rename = "encryptionMaterial")]
    encryption_material: Option<OneOrMany<EncryptionMaterial>>,

    //unused fields
    #[serde(rename = "parameters")]
    _parameters: Option<Vec<NameValueParameter>>,
    #[serde(rename = "rowType")]
    _row_type: Option<Vec<RowType>>,
    #[serde(rename = "total")]
    _total: Option<i64>,
    #[serde(rename = "returned")]
    _returned: Option<i64>,
    #[serde(rename = "queryId")]
    _query_id: Option<String>,
    #[serde(rename = "sqlState")]
    _sql_state: Option<String>,
    #[serde(rename = "databaseProvider")]
    _database_provider: Option<String>,
    #[serde(rename = "finalDatabaseName")]
    _final_database_name: Option<String>,
    #[serde(rename = "finalSchemaName")]
    _final_schema_name: Option<String>,
    #[serde(rename = "finalWarehouseName")]
    _final_warehouse_name: Option<String>,
    #[serde(rename = "finalRoleName")]
    _final_role_name: Option<String>,
    #[serde(rename = "numberOfBinds")]
    _number_of_binds: Option<i32>,
    #[serde(rename = "statementTypeId")]
    _statement_type_id: Option<i64>,
    #[serde(rename = "version")]
    _version: Option<i64>,
    #[serde(rename = "chunks")]
    _chunks: Option<Vec<Chunk>>,
    #[serde(rename = "qrmk")]
    _qrmk: Option<String>,
    #[serde(rename = "chunkHeaders")]
    _chunk_headers: Option<HashMap<String, String>>,
    #[serde(rename = "getResultUrl")]
    _get_result_url: Option<String>,
    #[serde(rename = "progressDesc")]
    _progress_desc: Option<String>,
    #[serde(rename = "queryAbortsAfterSecs")]
    _query_abort_timeout: Option<i64>,
    #[serde(rename = "resultIds")]
    _result_ids: Option<String>,
    #[serde(rename = "resultTypes")]
    _result_types: Option<String>,
    #[serde(rename = "queryResultFormat")]
    _query_result_format: Option<String>,
    #[serde(rename = "asyncResult")]
    _async_result: Option<SnowflakeResult>,
    #[serde(rename = "asyncRows")]
    _async_rows: Option<SnowflakeRows>,
    #[serde(rename = "uploadInfo")]
    _upload_info: Option<StageInfo>,
    #[serde(rename = "localLocation")]
    _local_location: Option<String>,
    #[serde(rename = "parallel")]
    _parallel: Option<i64>,
    #[serde(rename = "threshold")]
    _threshold: Option<i64>,
    #[serde(rename = "autoCompress")]
    _auto_compress: Option<bool>,
    #[serde(rename = "overwrite")]
    _overwrite: Option<bool>,
    #[serde(rename = "sourceCompression")]
    _source_compression: Option<String>,
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
    //unused fields
    #[serde(rename = "url")]
    _url: String,
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

#[derive(Debug, Deserialize)]
pub struct NameValueParameter {
    //unused fields
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "value")]
    _value: serde_json::Value,
}

#[derive(Deserialize)]
pub struct RowType {
    //unused fields
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "fields")]
    _fields: Option<Vec<FieldMetadata>>,
    #[serde(rename = "byteLength")]
    _byte_length: Option<i64>,
    #[serde(rename = "length")]
    _length: Option<i64>,
    #[serde(rename = "type")]
    _type_: String,
    #[serde(rename = "precision")]
    _precision: i64,
    #[serde(rename = "scale")]
    _scale: i64,
    #[serde(rename = "nullable")]
    _nullable: bool,
}

#[derive(Deserialize)]
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
    //unused fields
    #[serde(rename = "locationType")]
    _location_type: Option<String>,
    #[serde(rename = "path")]
    _path: Option<String>,
    #[serde(rename = "storageAccount")]
    _storage_account: Option<String>,
    #[serde(rename = "isClientSideEncrypted")]
    _is_client_side_encrypted: Option<bool>,
    #[serde(rename = "presignedUrl")]
    _presigned_url: Option<String>,
    #[serde(rename = "endPoint")]
    _end_point: Option<String>,
    #[serde(rename = "useS3RegionalUrl")]
    _use_s3_regional_url: Option<bool>,
    #[serde(rename = "useRegionalUrl")]
    _use_regional_url: Option<bool>,
    #[serde(rename = "useVirtualUrl")]
    _use_virtual_url: Option<bool>,
}

#[derive(Deserialize)]
pub struct Credentials {
    #[serde(rename = "AWS_KEY_ID")]
    aws_key_id: Option<String>,
    #[serde(rename = "AWS_SECRET_KEY")]
    aws_secret_key: Option<String>,
    #[serde(rename = "AWS_TOKEN")]
    aws_token: Option<String>,
    //unused fields
    #[serde(rename = "AWS_ID")]
    _aws_id: Option<String>,
    #[serde(rename = "AWS_KEY")]
    _aws_key: Option<String>,
    #[serde(rename = "AZURE_SAS_TOKEN")]
    _azure_sas_token: Option<String>,
    #[serde(rename = "GCS_ACCESS_TOKEN")]
    _gcs_access_token: Option<String>,
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
    pub fn to_file_transfer_data(&self) -> Result<file_manager::Data, RestError> {
        let src_locations = self
            .src_locations
            .as_ref()
            .ok_or_else(|| RestError::MissingParameter("source locations".to_string()))?
            .clone();

        if src_locations.is_empty() {
            return Err(RestError::MissingParameter("source locations".to_string()));
        }

        let stage_info: file_manager::StageInfo = self
            .stage_info
            .as_ref()
            .ok_or_else(|| RestError::MissingParameter("stage info".to_string()))?
            .try_into()?;

        let encryption_materials: Vec<_> = self
            .encryption_material
            .as_ref()
            .ok_or_else(|| RestError::MissingParameter("encryption material".to_string()))?
            .as_slice()
            .iter()
            .map(|em| em.into())
            .collect();

        Ok(file_manager::Data {
            src_locations,
            stage_info,
            encryption_materials,
        })
    }
}

impl TryFrom<&StageInfo> for file_manager::StageInfo {
    type Error = RestError;

    fn try_from(value: &StageInfo) -> Result<Self, Self::Error> {
        let location = value
            .location
            .as_ref()
            .ok_or_else(|| RestError::MissingParameter("stage info -> location".to_string()))?
            .clone();

        let region = value
            .region
            .as_ref()
            .ok_or_else(|| RestError::MissingParameter("stage info -> region".to_string()))?
            .clone();

        let creds: file_manager::Credentials = value
            .creds
            .as_ref()
            .ok_or_else(|| RestError::MissingParameter("stage info -> credentials".to_string()))?
            .try_into()?;

        Ok(file_manager::StageInfo {
            location,
            region,
            creds,
        })
    }
}

impl TryFrom<&Credentials> for file_manager::Credentials {
    type Error = RestError;

    fn try_from(value: &Credentials) -> Result<Self, Self::Error> {
        let aws_key_id = value
            .aws_key_id
            .as_ref()
            .ok_or_else(|| RestError::MissingParameter("credentials -> aws key id".to_string()))?
            .clone();

        let aws_secret_key = value
            .aws_secret_key
            .as_ref()
            .ok_or_else(|| {
                RestError::MissingParameter("credentials -> aws secret key".to_string())
            })?
            .clone();

        let aws_token = value
            .aws_token
            .as_ref()
            .ok_or_else(|| RestError::MissingParameter("credentials -> aws token".to_string()))?
            .clone();

        Ok(file_manager::Credentials {
            aws_key_id,
            aws_secret_key,
            aws_token,
        })
    }
}

impl From<&EncryptionMaterial> for file_manager::EncryptionMaterial {
    fn from(value: &EncryptionMaterial) -> Self {
        Self {
            query_stage_master_key: value.query_stage_master_key.clone(),
            query_id: value.query_id.clone(),
            // Snowflake sends smk_id as i64, but later expects it as a string
            smk_id: value.smk_id.to_string(),
        }
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
