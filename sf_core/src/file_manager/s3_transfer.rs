use super::types::{
    EncryptedFileMetadata, MaterialDescription, PreparedUpload, StageInfo, UploadStatus,
};
use snafu::{Location, ResultExt, Snafu};

// AWS SDK imports
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};

const SNOWFLAKE_UPLOAD_PROVIDER: &str = "snowflake-upload";
const SNOWFLAKE_DOWNLOAD_PROVIDER: &str = "snowflake-download";
const CONTENT_TYPE_OCTET_STREAM: &str = "application/octet-stream";

// TODO: streaming instead of loading the whole file into memory

/// Uploads a file to S3, skipping if it already exists and `overwrite` is false.
pub async fn upload_to_s3_or_skip(
    prepared: PreparedUpload,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
) -> Result<UploadStatus, UploadFileError> {
    // Check if the file already exists in S3
    let s3_client = create_s3_client(stage_info, SNOWFLAKE_UPLOAD_PROVIDER).await?;
    let s3_key = format!("{}{filename}", stage_info.key_prefix);

    if !overwrite && check_if_file_exists(&s3_client, stage_info, &s3_key).await? {
        tracing::info!("File already exists in S3: {}", s3_key);
        return Ok(UploadStatus::Skipped);
    }

    // Proceed with upload if the file does not exist or overwrite is true
    upload_to_s3(prepared, &s3_client, stage_info, &s3_key).await?;
    Ok(UploadStatus::Uploaded)
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
        Err(SdkError::ServiceError(ref err)) if err.raw().status().as_u16() == 403 => {
            tracing::warn!(
                "Access denied when checking if file exists in S3 ({s3_key}), proceeding with upload"
            );
            Ok(false)
        }
        Err(e) => Err(aws_sdk_s3::Error::from(e)).context(upload_file_error::S3HeadSnafu),
    }
}

async fn upload_to_s3(
    prepared: PreparedUpload,
    s3_client: &S3Client,
    stage_info: &StageInfo,
    s3_key: &str,
) -> Result<(), UploadFileError> {
    let mut put_object_request = s3_client
        .put_object()
        .bucket(stage_info.bucket.clone())
        .key(s3_key)
        .body(ByteStream::from(prepared.data))
        .content_type(CONTENT_TYPE_OCTET_STREAM)
        .metadata("sfc-digest", &prepared.digest);

    if let Some(ref enc_meta) = prepared.encryption_metadata {
        let mat_desc = serde_json::to_string(&enc_meta.material_desc)
            .context(upload_file_error::SerializationSnafu)?;
        put_object_request = put_object_request
            .metadata("x-amz-iv", &enc_meta.iv)
            .metadata("x-amz-key", &enc_meta.encrypted_key)
            .metadata("x-amz-matdesc", mat_desc);
    }

    tracing::trace!("PUT object request: {:?}", put_object_request);

    let result = put_object_request
        .send()
        .await
        .map_err(aws_sdk_s3::Error::from)
        .context(upload_file_error::S3UploadSnafu)?;

    tracing::debug!("S3 upload result: {:?}", result);

    Ok(())
}

/// Downloads a file from S3 and returns the data with optional encryption metadata.
/// For SSE stages the metadata headers will be absent and `None` is returned.
pub async fn download_from_s3(
    stage_info: &StageInfo,
    filename: &str,
) -> Result<(Vec<u8>, Option<String>, Option<EncryptedFileMetadata>), DownloadFileError> {
    let s3_client = create_s3_client(stage_info, SNOWFLAKE_DOWNLOAD_PROVIDER).await?;
    let s3_key = format!("{}{filename}", stage_info.key_prefix);

    let response = s3_client
        .get_object()
        .bucket(stage_info.bucket.clone())
        .key(&s3_key)
        .send()
        .await
        .map_err(aws_sdk_s3::Error::from)
        .context(download_file_error::S3DownloadSnafu)?;

    let metadata_map = response.metadata().cloned().unwrap_or_default();

    let digest = metadata_map.get("sfc-digest").cloned();

    let mat_desc = metadata_map.get("x-amz-matdesc");
    let encrypted_key = metadata_map.get("x-amz-key");
    let iv = metadata_map.get("x-amz-iv");

    let file_metadata = match (mat_desc, encrypted_key, iv) {
        (Some(mat_desc_str), Some(key), Some(iv_val)) => {
            let material_desc: MaterialDescription = serde_json::from_str(mat_desc_str)
                .context(download_file_error::DeserializationSnafu)?;
            Some(EncryptedFileMetadata {
                encrypted_key: key.to_owned(),
                iv: iv_val.to_owned(),
                material_desc,
            })
        }
        (None, None, None) => None,
        _ => {
            return download_file_error::MissingFileMetadataSnafu {
                field: "partial encryption headers (x-amz-matdesc, x-amz-key, x-amz-iv)"
                    .to_string(),
            }
            .fail();
        }
    };

    let data = response
        .body
        .collect()
        .await
        .context(download_file_error::ByteStreamSnafu)?
        .into_bytes()
        .to_vec();

    Ok((data, digest, file_metadata))
}

async fn create_s3_client(
    stage_info: &StageInfo,
    provider_name: &'static str,
) -> Result<S3Client, S3CredentialError> {
    let super::types::CloudCredentials::S3 {
        ref aws_key_id,
        ref aws_secret_key,
        ref aws_token,
    } = stage_info.creds
    else {
        return Err(S3CredentialError);
    };

    let credentials = Credentials::new(
        aws_key_id,
        aws_secret_key.reveal(),
        Some(aws_token.reveal().to_string()),
        None,
        provider_name,
    );

    let config = aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(credentials)
        .region(Region::new(stage_info.region.clone()))
        .load()
        .await;

    let mut s3_config = aws_sdk_s3::config::Builder::from(&config);
    if let Some(end_point) = &stage_info.end_point {
        let endpoint_url = if end_point.starts_with("https://") || end_point.starts_with("http://")
        {
            end_point.clone()
        } else {
            format!("https://{end_point}")
        };
        tracing::debug!("Using Snowflake-provided S3 endpoint: {endpoint_url}");
        s3_config = s3_config.endpoint_url(endpoint_url);
    }

    Ok(S3Client::from_conf(s3_config.build()))
}

/// Error returned when `create_s3_client` is called with non-S3 credentials.
#[derive(Debug)]
struct S3CredentialError;

impl From<S3CredentialError> for UploadFileError {
    fn from(_: S3CredentialError) -> Self {
        UploadFileError::MissingS3Credentials {
            location: Location::default(),
        }
    }
}

impl From<S3CredentialError> for DownloadFileError {
    fn from(_: S3CredentialError) -> Self {
        DownloadFileError::MissingS3Credentials {
            location: Location::default(),
        }
    }
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(module)]
pub enum UploadFileError {
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
    #[snafu(display("Failed to serialize metadata during file upload"))]
    Serialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing S3 credentials"))]
    MissingS3Credentials {
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(module)]
pub enum DownloadFileError {
    #[snafu(display("Failed to download file from S3"))]
    S3Download {
        #[snafu(source(from(aws_sdk_s3::Error, Box::new)))]
        source: Box<aws_sdk_s3::Error>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to deserialize metadata during file download"))]
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
    #[snafu(display("Missing S3 credentials"))]
    MissingS3Credentials {
        #[snafu(implicit)]
        location: Location,
    },
}
