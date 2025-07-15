use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// TODO: Delete all unused fields when we are sure they are not needed

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResultFormat {
    Json,
    Arrow,
}

#[derive(Serialize)]
pub struct ExecBindParameter {
    #[serde(rename = "type")]
    pub type_: String,
    pub value: serde_json::Value,
    #[serde(rename = "fmt", skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<BindingSchema>,
}

#[derive(Serialize)]
pub struct BindingSchema {}

#[derive(Serialize)]
pub struct ExecRequest {
    #[serde(rename = "sqlText")]
    pub sql_text: String,
    #[serde(rename = "asyncExec")]
    pub async_exec: bool,
    #[serde(rename = "sequenceId")]
    pub sequence_id: u64,
    #[serde(rename = "querySubmissionTime")]
    pub query_submission_time: i64,
    #[serde(rename = "isInternal")]
    pub is_internal: bool,
    #[serde(rename = "describeOnly", skip_serializing_if = "Option::is_none")]
    pub describe_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bindings: Option<HashMap<String, ExecBindParameter>>,
    #[serde(rename = "bindStage", skip_serializing_if = "Option::is_none")]
    pub bind_stage: Option<String>,
    #[serde(rename = "queryContextDTO")]
    pub query_context: RequestQueryContext,
}

#[derive(Serialize)]
pub struct RequestQueryContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<RequestQueryContextEntry>>,
}

#[derive(Serialize)]
pub struct RequestQueryContextEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ContextData>,
    pub id: i32,
    pub priority: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
}

#[derive(Serialize)]
pub struct ContextData {
    #[serde(rename = "base64Data", skip_serializing_if = "Option::is_none")]
    pub base64_data: Option<String>,
}

#[derive(Deserialize)]
pub struct ExecResponseRowType {
    #[serde(rename = "name")]
    pub _name: String,
    #[serde(rename = "fields")]
    pub _fields: Option<Vec<FieldMetadata>>,
    #[serde(rename = "byteLength")]
    pub _byte_length: Option<i64>,
    #[serde(rename = "length")]
    pub _length: Option<i64>,
    #[serde(rename = "type")]
    pub _type_: String,
    #[serde(rename = "precision")]
    pub _precision: i64,
    #[serde(rename = "scale")]
    pub _scale: i64,
    #[serde(rename = "nullable")]
    pub _nullable: bool,
}

#[derive(Deserialize)]
pub struct FieldMetadata {
    #[serde(rename = "name")]
    pub _name: Option<String>,
    #[serde(rename = "type")]
    pub _type_: String,
    #[serde(rename = "nullable")]
    pub _nullable: bool,
    #[serde(rename = "length")]
    pub _length: i32,
    #[serde(rename = "scale")]
    pub _scale: i32,
    #[serde(rename = "precision")]
    pub _precision: i32,
    #[serde(rename = "fields")]
    pub _fields: Option<Vec<FieldMetadata>>,
}

#[derive(Deserialize)]
pub struct ExecResponseChunk {
    #[serde(rename = "url")]
    pub _url: String,
    #[serde(rename = "rowCount")]
    pub _row_count: i32,
    #[serde(rename = "uncompressedSize")]
    pub _uncompressed_size: i64,
    #[serde(rename = "compressedSize")]
    pub _compressed_size: i64,
}

#[derive(Deserialize)]
pub struct ExecResponseCredentials {
    #[serde(rename = "AWS_KEY_ID")]
    pub _aws_key_id: Option<String>,
    #[serde(rename = "AWS_SECRET_KEY")]
    pub _aws_secret_key: Option<String>,
    #[serde(rename = "AWS_TOKEN")]
    pub _aws_token: Option<String>,
    #[serde(rename = "AWS_ID")]
    pub _aws_id: Option<String>,
    #[serde(rename = "AWS_KEY")]
    pub _aws_key: Option<String>,
    #[serde(rename = "AZURE_SAS_TOKEN")]
    pub _azure_sas_token: Option<String>,
    #[serde(rename = "GCS_ACCESS_TOKEN")]
    pub _gcs_access_token: Option<String>,
}

#[derive(Deserialize)]
pub struct ExecResponseStageInfo {
    #[serde(rename = "locationType")]
    pub _location_type: Option<String>,
    #[serde(rename = "location")]
    pub _location: Option<String>,
    #[serde(rename = "path")]
    pub _path: Option<String>,
    #[serde(rename = "region")]
    pub _region: Option<String>,
    #[serde(rename = "storageAccount")]
    pub _storage_account: Option<String>,
    #[serde(rename = "isClientSideEncrypted")]
    pub _is_client_side_encrypted: Option<bool>,
    #[serde(rename = "creds")]
    pub _creds: Option<ExecResponseCredentials>,
    #[serde(rename = "presignedUrl")]
    pub _presigned_url: Option<String>,
    #[serde(rename = "endPoint")]
    pub _end_point: Option<String>,
    #[serde(rename = "useS3RegionalUrl")]
    pub _use_s3_regional_url: Option<bool>,
    #[serde(rename = "useRegionalUrl")]
    pub _use_regional_url: Option<bool>,
    #[serde(rename = "useVirtualUrl")]
    pub _use_virtual_url: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct NameValueParameter {
    #[serde(rename = "name")]
    pub _name: String,
    #[serde(rename = "value")]
    pub _value: serde_json::Value,
}

#[derive(Deserialize)]
pub struct SnowflakeResult {}

#[derive(Deserialize)]
pub struct SnowflakeRows {}

#[derive(Deserialize)]
pub struct EncryptionWrapper {}

#[derive(Deserialize)]
pub struct ExecResponseData {
    #[serde(rename = "parameters")]
    pub _parameters: Option<Vec<NameValueParameter>>,
    #[serde(rename = "rowType")]
    pub _row_type: Option<Vec<ExecResponseRowType>>,
    #[serde(rename = "rowset")]
    pub _rowset: Option<Vec<Vec<Option<String>>>>,
    #[serde(rename = "rowsetBase64")]
    pub rowset_base64: Option<String>,
    #[serde(rename = "total")]
    pub _total: Option<i64>,
    #[serde(rename = "returned")]
    pub _returned: Option<i64>,
    #[serde(rename = "queryId")]
    pub _query_id: Option<String>,
    #[serde(rename = "sqlState")]
    pub _sql_state: Option<String>,
    #[serde(rename = "databaseProvider")]
    pub _database_provider: Option<String>,
    #[serde(rename = "finalDatabaseName")]
    pub _final_database_name: Option<String>,
    #[serde(rename = "finalSchemaName")]
    pub _final_schema_name: Option<String>,
    #[serde(rename = "finalWarehouseName")]
    pub _final_warehouse_name: Option<String>,
    #[serde(rename = "finalRoleName")]
    pub _final_role_name: Option<String>,
    #[serde(rename = "numberOfBinds")]
    pub _number_of_binds: Option<i32>,
    #[serde(rename = "statementTypeId")]
    pub _statement_type_id: Option<i64>,
    #[serde(rename = "version")]
    pub _version: Option<i64>,
    #[serde(rename = "chunks")]
    pub _chunks: Option<Vec<ExecResponseChunk>>,
    #[serde(rename = "qrmk")]
    pub _qrmk: Option<String>,
    #[serde(rename = "chunkHeaders")]
    pub _chunk_headers: Option<HashMap<String, String>>,
    #[serde(rename = "getResultUrl")]
    pub _get_result_url: Option<String>,
    #[serde(rename = "progressDesc")]
    pub _progress_desc: Option<String>,
    #[serde(rename = "queryAbortsAfterSecs")]
    pub _query_abort_timeout: Option<i64>,
    #[serde(rename = "resultIds")]
    pub _result_ids: Option<String>,
    #[serde(rename = "resultTypes")]
    pub _result_types: Option<String>,
    #[serde(rename = "queryResultFormat")]
    pub _query_result_format: Option<String>,
    #[serde(rename = "asyncResult")]
    pub _async_result: Option<SnowflakeResult>,
    #[serde(rename = "asyncRows")]
    pub _async_rows: Option<SnowflakeRows>,
    #[serde(rename = "uploadInfo")]
    pub _upload_info: Option<ExecResponseStageInfo>,
    #[serde(rename = "localLocation")]
    pub _local_location: Option<String>,
    #[serde(rename = "srcLocations")]
    pub _src_locations: Option<Vec<String>>,
    #[serde(rename = "parallel")]
    pub _parallel: Option<i64>,
    #[serde(rename = "threshold")]
    pub _threshold: Option<i64>,
    #[serde(rename = "autoCompress")]
    pub _auto_compress: Option<bool>,
    #[serde(rename = "overwrite")]
    pub _overwrite: Option<bool>,
    #[serde(rename = "sourceCompression")]
    pub _source_compression: Option<String>,
    #[serde(rename = "clientShowEncryptionParameter")]
    pub _show_encryption_parameter: Option<bool>,
    #[serde(rename = "encryptionMaterial")]
    pub _encryption_material: Option<EncryptionWrapper>,
    #[serde(rename = "presignedUrls")]
    pub _presigned_urls: Option<Vec<String>>,
    #[serde(rename = "stageInfo")]
    pub _stage_info: Option<ExecResponseStageInfo>,
    #[serde(rename = "command")]
    pub _command: Option<String>,
    #[serde(rename = "kind")]
    pub _kind: Option<String>,
    #[serde(rename = "operation")]
    pub _operation: Option<String>,
    #[serde(rename = "queryContext")]
    pub _query_context: Option<ResponseQueryContext>,
}

#[derive(Deserialize)]
pub struct ResponseQueryContext {
    #[serde(rename = "entries")]
    pub _entries: Option<Vec<ResponseQueryContextEntry>>,
}

#[derive(Deserialize)]
pub struct ResponseQueryContextEntry {
    #[serde(rename = "id")]
    pub _id: i32,
    #[serde(rename = "timestamp")]
    pub _timestamp: i64,
    #[serde(rename = "priority")]
    pub _priority: i32,
    #[serde(rename = "context")]
    pub _context: String,
}

#[derive(Deserialize)]
pub struct ExecResponse {
    pub data: ExecResponseData,
    #[serde(rename = "message")]
    pub message: Option<String>,
    #[serde(rename = "code")]
    pub _code: Option<String>,
    #[serde(rename = "success")]
    pub success: bool,
}
