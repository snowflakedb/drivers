use super::types::{
    CloudCredentials, EncryptedFileMetadata, EncryptionResult, MaterialDescription, StageInfo,
};
use snafu::{Location, OptionExt, ResultExt, Snafu};

// AWS SDK imports
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};

const SNOWFLAKE_UPLOAD_PROVIDER: &str = "snowflake-upload";
const SNOWFLAKE_DOWNLOAD_PROVIDER: &str = "snowflake-download";
const CONTENT_TYPE_OCTET_STREAM: &str = "application/octet-stream";

// S3 metadata header names
const S3_META_SFC_DIGEST: &str = "sfc-digest";
const S3_META_IV: &str = "x-amz-iv";
const S3_META_KEY: &str = "x-amz-key";
const S3_META_MATDESC: &str = "x-amz-matdesc";

// Upload status constants
const STATUS_SKIPPED: &str = "SKIPPED";
const STATUS_UPLOADED: &str = "UPLOADED";

// HTTP status codes
const HTTP_STATUS_FORBIDDEN: u16 = 403;

// TODO: streaming instead of loading the whole file into memory

/// Build the S3 key by concatenating the stage prefix and filename.
fn build_s3_key(stage_info: &StageInfo, filename: &str) -> String {
    format!("{}{}", stage_info.key_prefix, filename)
}

/// Normalize endpoint URL to include https:// scheme if not present.
fn normalize_endpoint_url(endpoint: &str) -> String {
    if endpoint.starts_with("https://") || endpoint.starts_with("http://") {
        endpoint.to_string()
    } else {
        format!("https://{}", endpoint)
    }
}

/// Extract a required metadata field from S3 response metadata.
fn get_metadata_field(
    metadata_map: &std::collections::HashMap<String, String>,
    field_name: &str,
) -> Result<String, DownloadFileError> {
    metadata_map
        .get(field_name)
        .cloned()
        .context(MissingFileMetadataSnafu {
            field: field_name.to_string(),
        })
}

/// Uploads a file to S3, skipping if it already exists and `overwrite` is false.
pub async fn upload_to_s3_or_skip(
    encryption_result: EncryptionResult,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
) -> Result<String, UploadFileError> {
    // Extract AWS credentials
    let CloudCredentials::Aws {
        key_id,
        secret_key,
        token,
    } = &stage_info.creds
    else {
        return InvalidCredentialsUploadSnafu.fail();
    };

    // Check if the file already exists in S3
    let s3_client = create_s3_client(
        stage_info,
        key_id,
        secret_key,
        token,
        SNOWFLAKE_UPLOAD_PROVIDER,
    )
    .await?;
    let s3_key = build_s3_key(stage_info, filename);

    if !overwrite && check_if_file_exists(&s3_client, stage_info, &s3_key).await? {
        tracing::info!("File already exists in S3: {}", s3_key);
        return Ok(STATUS_SKIPPED.to_string());
    }

    // Proceed with upload if the file does not exist or overwrite is true
    upload_to_s3(encryption_result, &s3_client, stage_info, &s3_key).await?;
    Ok(STATUS_UPLOADED.to_string())
}

/// Returns true if the file exists in S3, false if it does not.
/// When the check cannot be performed due to 403 Forbidden (limited
/// temporary credentials that allow PUT but not HEAD), returns false
/// so the caller proceeds with upload.
async fn check_if_file_exists(
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
        Err(SdkError::ServiceError(ref err))
            if err.raw().status().as_u16() == HTTP_STATUS_FORBIDDEN =>
        {
            tracing::warn!(
                "Access denied when checking if file exists in S3 ({s3_key}), proceeding with upload"
            );
            Ok(false)
        }
        Err(e) => Err(aws_sdk_s3::Error::from(e)).context(S3HeadSnafu),
    }
}

async fn upload_to_s3(
    encryption_result: EncryptionResult,
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
) -> Result<(), UploadFileError> {
    // Serialize encryption metadata
    let mat_desc = serde_json::to_string(&encryption_result.metadata.material_desc)
        .context(SerializationSnafu)?;

    let put_object_request = s3_client
        .put_object()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .body(ByteStream::from(encryption_result.data))
        .content_type(CONTENT_TYPE_OCTET_STREAM)
        .metadata(S3_META_SFC_DIGEST, &encryption_result.metadata.digest)
        .metadata(S3_META_IV, &encryption_result.metadata.iv)
        .metadata(S3_META_KEY, &encryption_result.metadata.encrypted_key)
        .metadata(S3_META_MATDESC, mat_desc);

    tracing::trace!("PUT object request: {:?}", put_object_request);

    // Upload to S3 (with optional encryption metadata)
    let result = put_object_request
        .send()
        .await
        .map_err(aws_sdk_s3::Error::from)
        .context(S3UploadSnafu)?;

    tracing::debug!("S3 upload result: {:?}", result);

    Ok(())
}

pub async fn download_from_s3(
    stage_info: &StageInfo,
    filename: &str,
) -> Result<(Vec<u8>, EncryptedFileMetadata), DownloadFileError> {
    let CloudCredentials::Aws {
        key_id,
        secret_key,
        token,
    } = &stage_info.creds
    else {
        return InvalidCredentialsDownloadSnafu.fail();
    };

    let s3_client = create_s3_client_for_download(stage_info, key_id, secret_key, token).await?;
    let s3_key = build_s3_key(stage_info, filename);

    let response = s3_client
        .get_object()
        .bucket(stage_info.bucket.clone())
        .key(&s3_key)
        .send()
        .await
        .map_err(aws_sdk_s3::Error::from)
        .context(S3DownloadSnafu)?;

    let file_metadata = extract_metadata_from_response(&response)?;
    let encrypted_data = extract_data_from_response(response).await?;

    Ok((encrypted_data, file_metadata))
}

/// Create S3 client configured for download operations.
async fn create_s3_client_for_download(
    stage_info: &StageInfo,
    key_id: &str,
    secret_key: &crate::sensitive::SensitiveString,
    token: &crate::sensitive::SensitiveString,
) -> Result<S3Client, DownloadFileError> {
    let region_str = stage_info
        .region
        .as_ref()
        .context(MissingRegionDownloadSnafu)?;

    let credentials = Credentials::new(
        key_id,
        secret_key.reveal(),
        Some(token.reveal().to_string()),
        None,
        SNOWFLAKE_DOWNLOAD_PROVIDER,
    );

    let config = aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(credentials)
        .region(Region::new(region_str.to_string()))
        .load()
        .await;

    let mut s3_config = aws_sdk_s3::config::Builder::from(&config);
    if let Some(end_point) = &stage_info.end_point {
        let endpoint_url = normalize_endpoint_url(end_point);
        tracing::debug!("Using Snowflake-provided S3 endpoint: {endpoint_url}");
        s3_config = s3_config.endpoint_url(endpoint_url);
    }

    Ok(S3Client::from_conf(s3_config.build()))
}

/// Extract encrypted file metadata from S3 GetObject response.
fn extract_metadata_from_response(
    response: &aws_sdk_s3::operation::get_object::GetObjectOutput,
) -> Result<EncryptedFileMetadata, DownloadFileError> {
    let metadata_map = response.metadata().context(MissingFileMetadataSnafu {
        field: "All fields".to_string(),
    })?;

    let mat_desc_str = get_metadata_field(metadata_map, S3_META_MATDESC)?;
    let material_desc: MaterialDescription =
        serde_json::from_str(&mat_desc_str).context(DeserializationSnafu)?;

    let encrypted_key = get_metadata_field(metadata_map, S3_META_KEY)?;
    let iv = get_metadata_field(metadata_map, S3_META_IV)?;
    let digest = get_metadata_field(metadata_map, S3_META_SFC_DIGEST)?;

    Ok(EncryptedFileMetadata {
        encrypted_key,
        iv,
        material_desc,
        digest,
    })
}

/// Extract encrypted data bytes from S3 GetObject response.
async fn extract_data_from_response(
    response: aws_sdk_s3::operation::get_object::GetObjectOutput,
) -> Result<Vec<u8>, DownloadFileError> {
    let bytes = response
        .body
        .collect()
        .await
        .context(ByteStreamSnafu)?
        .into_bytes()
        .to_vec();
    Ok(bytes)
}

async fn create_s3_client(
    stage_info: &StageInfo,
    key_id: &str,
    secret_key: &crate::sensitive::SensitiveString,
    token: &crate::sensitive::SensitiveString,
    provider_name: &'static str,
) -> Result<S3Client, UploadFileError> {
    let credentials = Credentials::new(
        key_id,
        secret_key.reveal(),
        Some(token.reveal().to_string()),
        None,
        provider_name,
    );

    let region_str = stage_info
        .region
        .as_ref()
        .context(MissingRegionUploadSnafu)?;

    let config = aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(credentials)
        .region(Region::new(region_str.to_string()))
        .load()
        .await;

    let mut s3_config = aws_sdk_s3::config::Builder::from(&config);
    if let Some(end_point) = &stage_info.end_point {
        let endpoint_url = normalize_endpoint_url(end_point);
        tracing::debug!("Using Snowflake-provided S3 endpoint: {endpoint_url}");
        s3_config = s3_config.endpoint_url(endpoint_url);
    }

    Ok(S3Client::from_conf(s3_config.build()))
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum UploadFileError {
    #[snafu(display("Invalid credentials for S3"))]
    InvalidCredentialsUpload {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("S3 region is required but not provided"))]
    MissingRegionUpload {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to upload file to S3"))]
    S3Upload {
        #[snafu(source(from(aws_sdk_s3::Error, Box::new)))]
        source: Box<aws_sdk_s3::Error>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to check if file exists in S3"))]
    S3Head {
        #[snafu(source(from(aws_sdk_s3::Error, Box::new)))]
        source: Box<aws_sdk_s3::Error>,
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

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum DownloadFileError {
    #[snafu(display("Invalid credentials for S3"))]
    InvalidCredentialsDownload {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("S3 region is required but not provided"))]
    MissingRegionDownload {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to download file from S3"))]
    S3Download {
        #[snafu(source(from(aws_sdk_s3::Error, Box::new)))]
        source: Box<aws_sdk_s3::Error>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to deserialize metadata"))]
    Deserialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("File metadata missing: {field}"))]
    MissingFileMetadata {
        field: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read byte stream from S3"))]
    ByteStream {
        source: aws_sdk_s3::primitives::ByteStreamError,
        #[snafu(implicit)]
        location: Location,
    },
}
