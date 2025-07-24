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
    pub aws_key_id: Option<String>,
    #[serde(rename = "AWS_SECRET_KEY")]
    pub aws_secret_key: Option<String>,
    #[serde(rename = "AWS_TOKEN")]
    pub aws_token: Option<String>,
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
    pub location: Option<String>,
    #[serde(rename = "path")]
    pub _path: Option<String>,
    #[serde(rename = "region")]
    pub region: Option<String>,
    #[serde(rename = "storageAccount")]
    pub _storage_account: Option<String>,
    #[serde(rename = "isClientSideEncrypted")]
    pub _is_client_side_encrypted: Option<bool>,
    #[serde(rename = "creds")]
    pub creds: Option<ExecResponseCredentials>,
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
pub struct ExecResponseData {
    #[serde(rename = "parameters")]
    pub _parameters: Option<Vec<NameValueParameter>>,
    #[serde(rename = "rowType")]
    pub _row_type: Option<Vec<ExecResponseRowType>>,
    #[serde(rename = "rowset")]
    pub rowset: Option<Vec<Vec<Option<String>>>>,
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
    #[serde(rename = "src_locations")]
    pub src_locations: Option<Vec<String>>,
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
    pub _encryption_material: Option<serde_json::Value>,
    #[serde(rename = "presignedUrls")]
    pub _presigned_urls: Option<serde_json::Value>,
    #[serde(rename = "stageInfo")]
    pub stage_info: Option<ExecResponseStageInfo>,
    #[serde(rename = "command")]
    pub command: Option<String>,
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

// Translation functions to convert from query types to file transfer types
impl ExecResponseData {
    /// Convert ExecResponseData to FileTransferData, validating that all required fields are present
    pub fn to_file_transfer_data(
        &self,
    ) -> Result<crate::api_server::file_transfer::FileTransferData, crate::rest::error::RestError>
    {
        let src_locations = self
            .src_locations
            .as_ref()
            .ok_or_else(|| {
                crate::rest::error::RestError::Internal(
                    "Source locations not found in response".to_string(),
                )
            })?
            .clone();

        if src_locations.is_empty() {
            return Err(crate::rest::error::RestError::Internal(
                "No source locations found".to_string(),
            ));
        }

        let stage_info = self
            .stage_info
            .as_ref()
            .ok_or_else(|| {
                crate::rest::error::RestError::Internal(
                    "Stage info not found in response".to_string(),
                )
            })?
            .to_file_transfer_stage_info()?;

        Ok(crate::api_server::file_transfer::FileTransferData {
            src_locations,
            stage_info,
        })
    }
}

impl ExecResponseStageInfo {
    /// Convert ExecResponseStageInfo to FileTransferStageInfo, validating that all required fields are present
    pub fn to_file_transfer_stage_info(
        &self,
    ) -> Result<
        crate::api_server::file_transfer::FileTransferStageInfo,
        crate::rest::error::RestError,
    > {
        let location = self
            .location
            .as_ref()
            .ok_or_else(|| {
                crate::rest::error::RestError::InvalidSnowflakeResponse(
                    "S3 location not found in stage info".to_string(),
                )
            })?
            .clone();

        let region = self
            .region
            .as_ref()
            .ok_or_else(|| {
                crate::rest::error::RestError::InvalidSnowflakeResponse(
                    "Region not found in stage info".to_string(),
                )
            })?
            .clone();

        let creds = self
            .creds
            .as_ref()
            .ok_or_else(|| {
                crate::rest::error::RestError::InvalidSnowflakeResponse(
                    "Credentials not found in stage info".to_string(),
                )
            })?
            .to_file_transfer_credentials()?;

        Ok(crate::api_server::file_transfer::FileTransferStageInfo {
            location,
            region,
            creds,
        })
    }
}

impl ExecResponseCredentials {
    /// Convert ExecResponseCredentials to FileTransferCredentials, validating that all required fields are present
    pub fn to_file_transfer_credentials(
        &self,
    ) -> Result<
        crate::api_server::file_transfer::FileTransferCredentials,
        crate::rest::error::RestError,
    > {
        let aws_key_id = self
            .aws_key_id
            .as_ref()
            .ok_or_else(|| {
                crate::rest::error::RestError::InvalidSnowflakeResponse(
                    "AWS_KEY_ID not found in credentials".to_string(),
                )
            })?
            .clone();

        let aws_secret_key = self
            .aws_secret_key
            .as_ref()
            .ok_or_else(|| {
                crate::rest::error::RestError::InvalidSnowflakeResponse(
                    "AWS_SECRET_KEY not found in credentials".to_string(),
                )
            })?
            .clone();

        let aws_token = self
            .aws_token
            .as_ref()
            .ok_or_else(|| {
                crate::rest::error::RestError::InvalidSnowflakeResponse(
                    "AWS_TOKEN not found in credentials".to_string(),
                )
            })?
            .clone();

        Ok(crate::api_server::file_transfer::FileTransferCredentials {
            aws_key_id,
            aws_secret_key,
            aws_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::error::RestError;

    fn create_valid_credentials() -> ExecResponseCredentials {
        serde_json::from_str(
            r#"{
            "AWS_KEY_ID": "test_key_id",
            "AWS_SECRET_KEY": "test_secret_key",
            "AWS_TOKEN": "test_token"
        }"#,
        )
        .unwrap()
    }

    fn create_valid_stage_info() -> ExecResponseStageInfo {
        serde_json::from_str(
            r#"{
            "location": "test-bucket/path/",
            "region": "us-east-1",
            "creds": {
                "AWS_KEY_ID": "test_key_id",
                "AWS_SECRET_KEY": "test_secret_key",
                "AWS_TOKEN": "test_token"
            }
        }"#,
        )
        .unwrap()
    }

    fn create_valid_exec_response_data() -> ExecResponseData {
        serde_json::from_str(
            r#"{
            "src_locations": ["test_file.csv"],
            "stageInfo": {
                "location": "test-bucket/path/",
                "region": "us-east-1",
                "creds": {
                    "AWS_KEY_ID": "test_key_id",
                    "AWS_SECRET_KEY": "test_secret_key",
                    "AWS_TOKEN": "test_token"
                }
            },
            "command": "UPLOAD"
        }"#,
        )
        .unwrap()
    }

    #[test]
    fn test_credentials_conversion_success() {
        let creds = create_valid_credentials();
        let result = creds.to_file_transfer_credentials();

        assert!(result.is_ok());
        let file_creds = result.unwrap();
        assert_eq!(file_creds.aws_key_id, "test_key_id");
        assert_eq!(file_creds.aws_secret_key, "test_secret_key");
        assert_eq!(file_creds.aws_token, "test_token");
    }

    #[test]
    fn test_credentials_conversion_missing_key_id() {
        let creds: ExecResponseCredentials = serde_json::from_str(
            r#"{
            "AWS_SECRET_KEY": "test_secret_key",
            "AWS_TOKEN": "test_token"
        }"#,
        )
        .unwrap();

        let result = creds.to_file_transfer_credentials();
        assert!(result.is_err());

        if let Err(RestError::InvalidSnowflakeResponse(msg)) = result {
            assert!(msg.contains("AWS_KEY_ID not found"));
        } else {
            panic!("Expected InvalidSnowflakeResponse error");
        }
    }

    #[test]
    fn test_credentials_conversion_missing_secret_key() {
        let creds: ExecResponseCredentials = serde_json::from_str(
            r#"{
            "AWS_KEY_ID": "test_key_id",
            "AWS_TOKEN": "test_token"
        }"#,
        )
        .unwrap();

        let result = creds.to_file_transfer_credentials();
        assert!(result.is_err());

        if let Err(RestError::InvalidSnowflakeResponse(msg)) = result {
            assert!(msg.contains("AWS_SECRET_KEY not found"));
        } else {
            panic!("Expected InvalidSnowflakeResponse error");
        }
    }

    #[test]
    fn test_credentials_conversion_missing_token() {
        let creds: ExecResponseCredentials = serde_json::from_str(
            r#"{
            "AWS_KEY_ID": "test_key_id",
            "AWS_SECRET_KEY": "test_secret_key"
        }"#,
        )
        .unwrap();

        let result = creds.to_file_transfer_credentials();
        assert!(result.is_err());

        if let Err(RestError::InvalidSnowflakeResponse(msg)) = result {
            assert!(msg.contains("AWS_TOKEN not found"));
        } else {
            panic!("Expected InvalidSnowflakeResponse error");
        }
    }

    #[test]
    fn test_stage_info_conversion_success() {
        let stage_info = create_valid_stage_info();
        let result = stage_info.to_file_transfer_stage_info();

        assert!(result.is_ok());
        let file_stage_info = result.unwrap();
        assert_eq!(file_stage_info.location, "test-bucket/path/");
        assert_eq!(file_stage_info.region, "us-east-1");
        assert_eq!(file_stage_info.creds.aws_key_id, "test_key_id");
    }

    #[test]
    fn test_stage_info_conversion_missing_location() {
        let stage_info: ExecResponseStageInfo = serde_json::from_str(
            r#"{
            "region": "us-east-1",
            "creds": {
                "AWS_KEY_ID": "test_key_id",
                "AWS_SECRET_KEY": "test_secret_key",
                "AWS_TOKEN": "test_token"
            }
        }"#,
        )
        .unwrap();

        let result = stage_info.to_file_transfer_stage_info();
        assert!(result.is_err());

        if let Err(RestError::InvalidSnowflakeResponse(msg)) = result {
            assert!(msg.contains("S3 location not found"));
        } else {
            panic!("Expected InvalidSnowflakeResponse error");
        }
    }

    #[test]
    fn test_stage_info_conversion_missing_region() {
        let stage_info: ExecResponseStageInfo = serde_json::from_str(
            r#"{
            "location": "test-bucket/path/",
            "creds": {
                "AWS_KEY_ID": "test_key_id",
                "AWS_SECRET_KEY": "test_secret_key",
                "AWS_TOKEN": "test_token"
            }
        }"#,
        )
        .unwrap();

        let result = stage_info.to_file_transfer_stage_info();
        assert!(result.is_err());

        if let Err(RestError::InvalidSnowflakeResponse(msg)) = result {
            assert!(msg.contains("Region not found"));
        } else {
            panic!("Expected InvalidSnowflakeResponse error");
        }
    }

    #[test]
    fn test_stage_info_conversion_missing_credentials() {
        let stage_info: ExecResponseStageInfo = serde_json::from_str(
            r#"{
            "location": "test-bucket/path/",
            "region": "us-east-1"
        }"#,
        )
        .unwrap();

        let result = stage_info.to_file_transfer_stage_info();
        assert!(result.is_err());

        if let Err(RestError::InvalidSnowflakeResponse(msg)) = result {
            assert!(msg.contains("Credentials not found"));
        } else {
            panic!("Expected InvalidSnowflakeResponse error");
        }
    }

    #[test]
    fn test_exec_response_data_conversion_success() {
        let data = create_valid_exec_response_data();
        let result = data.to_file_transfer_data();

        assert!(result.is_ok());
        let file_data = result.unwrap();
        assert_eq!(file_data.src_locations.len(), 1);
        assert_eq!(file_data.src_locations[0], "test_file.csv");
        assert_eq!(file_data.stage_info.location, "test-bucket/path/");
        assert_eq!(file_data.stage_info.region, "us-east-1");
    }

    #[test]
    fn test_exec_response_data_conversion_missing_src_locations() {
        let data: ExecResponseData = serde_json::from_str(
            r#"{
            "stageInfo": {
                "location": "test-bucket/path/",
                "region": "us-east-1",
                "creds": {
                    "AWS_KEY_ID": "test_key_id",
                    "AWS_SECRET_KEY": "test_secret_key",
                    "AWS_TOKEN": "test_token"
                }
            },
            "command": "UPLOAD"
        }"#,
        )
        .unwrap();

        let result = data.to_file_transfer_data();
        assert!(result.is_err());

        if let Err(RestError::Internal(msg)) = result {
            assert!(msg.contains("Source locations not found"));
        } else {
            panic!("Expected Internal error");
        }
    }

    #[test]
    fn test_exec_response_data_conversion_empty_src_locations() {
        let data: ExecResponseData = serde_json::from_str(
            r#"{
            "src_locations": [],
            "stageInfo": {
                "location": "test-bucket/path/",
                "region": "us-east-1",
                "creds": {
                    "AWS_KEY_ID": "test_key_id",
                    "AWS_SECRET_KEY": "test_secret_key",
                    "AWS_TOKEN": "test_token"
                }
            },
            "command": "UPLOAD"
        }"#,
        )
        .unwrap();

        let result = data.to_file_transfer_data();
        assert!(result.is_err());

        if let Err(RestError::Internal(msg)) = result {
            assert!(msg.contains("No source locations found"));
        } else {
            panic!("Expected Internal error");
        }
    }

    #[test]
    fn test_exec_response_data_conversion_missing_stage_info() {
        let data: ExecResponseData = serde_json::from_str(
            r#"{
            "src_locations": ["test_file.csv"],
            "command": "UPLOAD"
        }"#,
        )
        .unwrap();

        let result = data.to_file_transfer_data();
        assert!(result.is_err());

        if let Err(RestError::Internal(msg)) = result {
            assert!(msg.contains("Stage info not found"));
        } else {
            panic!("Expected Internal error");
        }
    }

    #[test]
    fn test_exec_response_data_conversion_multiple_src_locations() {
        let data: ExecResponseData = serde_json::from_str(
            r#"{
            "src_locations": ["file1.csv", "file2.csv", "file3.csv"],
            "stageInfo": {
                "location": "test-bucket/path/",
                "region": "us-east-1",
                "creds": {
                    "AWS_KEY_ID": "test_key_id",
                    "AWS_SECRET_KEY": "test_secret_key",
                    "AWS_TOKEN": "test_token"
                }
            },
            "command": "UPLOAD"
        }"#,
        )
        .unwrap();

        let result = data.to_file_transfer_data();
        assert!(result.is_ok());

        let file_data = result.unwrap();
        assert_eq!(file_data.src_locations.len(), 3);
        assert_eq!(file_data.src_locations[0], "file1.csv");
        assert_eq!(file_data.src_locations[1], "file2.csv");
        assert_eq!(file_data.src_locations[2], "file3.csv");
    }

    #[test]
    fn test_stage_info_conversion_propagates_credential_errors() {
        let stage_info: ExecResponseStageInfo = serde_json::from_str(
            r#"{
            "location": "test-bucket/path/",
            "region": "us-east-1",
            "creds": {
                "AWS_SECRET_KEY": "test_secret_key",
                "AWS_TOKEN": "test_token"
            }
        }"#,
        )
        .unwrap();

        let result = stage_info.to_file_transfer_stage_info();
        assert!(result.is_err());

        if let Err(RestError::InvalidSnowflakeResponse(msg)) = result {
            assert!(msg.contains("AWS_KEY_ID not found"));
        } else {
            panic!("Expected InvalidSnowflakeResponse error from credentials");
        }
    }

    #[test]
    fn test_exec_response_data_conversion_propagates_stage_info_errors() {
        let data: ExecResponseData = serde_json::from_str(
            r#"{
            "src_locations": ["test_file.csv"],
            "stageInfo": {
                "region": "us-east-1",
                "creds": {
                    "AWS_KEY_ID": "test_key_id",
                    "AWS_SECRET_KEY": "test_secret_key",
                    "AWS_TOKEN": "test_token"
                }
            },
            "command": "UPLOAD"
        }"#,
        )
        .unwrap();

        let result = data.to_file_transfer_data();
        assert!(result.is_err());

        if let Err(RestError::InvalidSnowflakeResponse(msg)) = result {
            assert!(msg.contains("S3 location not found"));
        } else {
            panic!("Expected InvalidSnowflakeResponse error from stage info");
        }
    }
}
