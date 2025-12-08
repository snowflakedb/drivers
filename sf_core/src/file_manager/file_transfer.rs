use super::types::{EncryptedFileMetadata, EncryptionResult, MaterialDescription, StageInfo};
use snafu::{Location, OptionExt, ResultExt, Snafu};

// AWS SDK imports
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};

const SNOWFLAKE_UPLOAD_PROVIDER: &str = "snowflake-upload";
const SNOWFLAKE_DOWNLOAD_PROVIDER: &str = "snowflake-download";
const CONTENT_TYPE_OCTET_STREAM: &str = "application/octet-stream";

// TODO: streaming instead of loading the whole file into memory

/// Storage provider detection
#[derive(Debug, Clone, PartialEq)]
pub enum StorageProvider {
    S3,
    Azure,
    Gcs,
}

impl StageInfo {
    /// Detect which cloud storage provider based on credentials
    pub fn detect_provider(&self) -> StorageProvider {
        if !self.creds.azure_sas_token.is_empty() {
            StorageProvider::Azure
        } else if !self.creds.gcs_access_token.is_empty() {
            StorageProvider::Gcs
        } else {
            // Default to S3 if AWS credentials present
            StorageProvider::S3
        }
    }
}

/// Main upload router that delegates to the appropriate cloud provider
pub async fn upload_file_or_skip(
    encryption_result: EncryptionResult,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
) -> Result<String, UploadFileError> {
    match stage_info.detect_provider() {
        StorageProvider::S3 => {
            upload_to_s3_or_skip(encryption_result, stage_info, filename, overwrite).await
        }
        StorageProvider::Azure => {
            // TODO: Implement Azure Blob Storage
            tracing::warn!("Azure Blob Storage not yet implemented");
            UnsupportedUploadProviderSnafu {
                provider: "Azure".to_string(),
            }
            .fail()
        }
        StorageProvider::Gcs => {
            // TODO: Implement Google Cloud Storage
            tracing::warn!("Google Cloud Storage not yet implemented");
            UnsupportedUploadProviderSnafu {
                provider: "GCS".to_string(),
            }
            .fail()
        }
    }
}

/// Main download router that delegates to the appropriate cloud provider
pub async fn download_file(
    stage_info: &StageInfo,
    filename: &str,
) -> Result<(Vec<u8>, EncryptedFileMetadata), DownloadFileError> {
    match stage_info.detect_provider() {
        StorageProvider::S3 => download_from_s3(stage_info, filename).await,
        StorageProvider::Azure => {
            tracing::warn!("Azure Blob Storage not yet implemented");
            UnsupportedDownloadProviderSnafu {
                provider: "Azure".to_string(),
            }
            .fail()
        }
        StorageProvider::Gcs => {
            tracing::warn!("Google Cloud Storage not yet implemented");
            UnsupportedDownloadProviderSnafu {
                provider: "GCS".to_string(),
            }
            .fail()
        }
    }
}

// ============================================================================
// AWS S3 IMPLEMENTATION
// ============================================================================

/// Uploads a file to S3, skipping if it already exists and `overwrite` is false.
pub async fn upload_to_s3_or_skip(
    encryption_result: EncryptionResult,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
) -> Result<String, UploadFileError> {
    let s3_client = create_s3_client(stage_info, SNOWFLAKE_UPLOAD_PROVIDER).await;
    // Ensure there's a / between key_prefix and filename
    let key_prefix = if stage_info.key_prefix.ends_with('/') {
        stage_info.key_prefix.clone()
    } else {
        format!("{}/", stage_info.key_prefix)
    };
    let s3_key = format!("{}{filename}", key_prefix);

    if !overwrite && check_if_s3_file_exists(&s3_client, stage_info, &s3_key).await? {
        tracing::info!("File already exists in S3: {s3_key}");
        return Ok("SKIPPED".to_string());
    }

    upload_to_s3(encryption_result, &s3_client, stage_info, &s3_key).await?;
    Ok("UPLOADED".to_string())
}

async fn check_if_s3_file_exists(
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
) -> Result<bool, UploadFileError> {
    match s3_client
        .head_object()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .send()
        .await
    {
        Ok(_) => Ok(true),
        Err(SdkError::ServiceError(err)) if err.err().is_not_found() => Ok(false),
        Err(e) => Err(aws_sdk_s3::Error::from(e)).context(S3HeadSnafu),
    }
}

async fn upload_to_s3(
    encryption_result: EncryptionResult,
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
) -> Result<(), UploadFileError> {
    let mat_desc = serde_json::to_string(&encryption_result.metadata.material_desc)
        .context(SerializationSnafu)?;

    let put_object_request = s3_client
        .put_object()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .body(ByteStream::from(encryption_result.data))
        .content_type(CONTENT_TYPE_OCTET_STREAM)
        .metadata("sfc-digest", &encryption_result.metadata.digest)
        .metadata("x-amz-iv", &encryption_result.metadata.iv)
        .metadata("x-amz-key", &encryption_result.metadata.encrypted_key)
        .metadata("x-amz-matdesc", mat_desc);

    tracing::trace!("PUT object request: {put_object_request:?}");

    put_object_request
        .send()
        .await
        .map_err(aws_sdk_s3::Error::from)
        .context(S3UploadSnafu)?;

    tracing::debug!("S3 upload complete: {s3_key}");
    Ok(())
}

pub async fn download_from_s3(
    stage_info: &StageInfo,
    filename: &str,
) -> Result<(Vec<u8>, EncryptedFileMetadata), DownloadFileError> {
    let s3_client = create_s3_client(stage_info, SNOWFLAKE_DOWNLOAD_PROVIDER).await;
    let s3_key = format!("{}{filename}", stage_info.key_prefix);

    let response = s3_client
        .get_object()
        .bucket(stage_info.bucket.clone())
        .key(&s3_key)
        .send()
        .await
        .map_err(aws_sdk_s3::Error::from)
        .context(S3DownloadSnafu)?;

    let metadata_map = response.metadata().context(MissingFileMetadataSnafu {
        field: "All fields".to_string(),
    })?;

    let mat_desc_str = metadata_map
        .get("x-amz-matdesc")
        .context(MissingFileMetadataSnafu {
            field: "x-amz-matdesc".to_string(),
        })?;

    let material_desc: MaterialDescription =
        serde_json::from_str(mat_desc_str).context(DeserializationSnafu)?;

    let file_metadata = EncryptedFileMetadata {
        encrypted_key: metadata_map
            .get("x-amz-key")
            .context(MissingFileMetadataSnafu {
                field: "x-amz-key".to_string(),
            })?
            .to_owned(),
        iv: metadata_map
            .get("x-amz-iv")
            .context(MissingFileMetadataSnafu {
                field: "x-amz-iv".to_string(),
            })?
            .to_owned(),
        material_desc,
        digest: metadata_map
            .get("sfc-digest")
            .context(MissingFileMetadataSnafu {
                field: "sfc-digest".to_string(),
            })?
            .to_owned(),
    };

    let body_bytes = response
        .body
        .collect()
        .await
        .expect("Failed to collect body bytes");
    let encrypted_data = body_bytes.into_bytes().to_vec();

    tracing::debug!(
        "S3 download complete: {s3_key}, size: {}",
        encrypted_data.len()
    );
    Ok((encrypted_data, file_metadata))
}

async fn create_s3_client(stage_info: &StageInfo, provider: &'static str) -> S3Client {
    let creds = Credentials::new(
        &stage_info.creds.aws_key_id,
        &stage_info.creds.aws_secret_key,
        Some(stage_info.creds.aws_token.clone()),
        None,
        provider,
    );

    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(stage_info.region.clone()))
        .credentials_provider(creds)
        .load()
        .await;

    S3Client::new(&config)
}

// ============================================================================
// ERROR TYPES
// ============================================================================

#[derive(Snafu, Debug)]
#[snafu(visibility(pub))]
pub enum UploadFileError {
    #[snafu(display("Failed to upload file to S3"))]
    S3Upload {
        source: aws_sdk_s3::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to check if file exists in S3"))]
    S3Head {
        source: aws_sdk_s3::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported upload storage provider: {provider}"))]
    UnsupportedUploadProvider {
        provider: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to serialize metadata"))]
    Serialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Snafu, Debug)]
#[snafu(visibility(pub))]
pub enum DownloadFileError {
    #[snafu(display("Failed to download file from S3"))]
    S3Download {
        source: aws_sdk_s3::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported download storage provider: {provider}"))]
    UnsupportedDownloadProvider {
        provider: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Missing file metadata: {field}"))]
    MissingFileMetadata {
        field: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to deserialize metadata"))]
    Deserialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
}
