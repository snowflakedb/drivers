use serde::Deserialize;

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

// Translation functions to convert from query types to file transfer types
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

/// File transfer conversion logic for ExecResponseData
pub fn to_file_transfer_data(
    src_locations: &Option<Vec<String>>,
    stage_info: &Option<ExecResponseStageInfo>,
    encryption_material: &Option<
        Vec<super::query_encryption_types::ExecResponseEncryptionMaterial>,
    >,
) -> Result<crate::api_server::file_transfer::FileTransferData, crate::rest::error::RestError> {
    let src_locations = src_locations
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

    let stage_info = stage_info
        .as_ref()
        .ok_or_else(|| {
            crate::rest::error::RestError::Internal("Stage info not found in response".to_string())
        })?
        .to_file_transfer_stage_info()?;

    let encryption_materials = encryption_material
        .as_ref()
        .ok_or_else(|| {
            crate::rest::error::RestError::Internal(
                "Encryption material not found in response".to_string(),
            )
        })?
        .iter()
        .map(|mat| mat.to_encryption_material())
        .collect::<Vec<_>>();

    Ok(crate::api_server::file_transfer::FileTransferData {
        src_locations,
        stage_info,
        encryption_materials,
    })
}
