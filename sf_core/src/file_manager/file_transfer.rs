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

/// Uploads a file to S3, skipping if it already exists and `overwrite` is false.
pub async fn upload_to_s3_or_skip(
    encryption_result: EncryptionResult,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
) -> Result<String, UploadFileError> {
    // Check if the file already exists in S3
    let s3_client = create_s3_client(stage_info, SNOWFLAKE_UPLOAD_PROVIDER).await;
    let s3_key = format!("{}{filename}", stage_info.key_prefix);

    if !overwrite && check_if_file_exists(&s3_client, stage_info, &s3_key).await? {
        tracing::info!("File already exists in S3: {}", s3_key);
        return Ok("SKIPPED".to_string());
    }

    // Proceed with upload if the file does not exist or overwrite is true
    upload_to_s3(encryption_result, &s3_client, stage_info, &s3_key).await?;
    Ok("UPLOADED".to_string())
}

/// Returns true if the file exists in S3, false if it does not.
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
        .metadata("sfc-digest", &encryption_result.metadata.digest)
        .metadata("x-amz-iv", &encryption_result.metadata.iv)
        .metadata("x-amz-key", &encryption_result.metadata.encrypted_key)
        .metadata("x-amz-matdesc", mat_desc);

    tracing::debug!("PUT object request: {:?}", put_object_request);

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
    let s3_client = create_s3_client(stage_info, SNOWFLAKE_DOWNLOAD_PROVIDER).await;
    let s3_key = format!("{}{filename}", stage_info.key_prefix);

    // Download from S3
    let response = s3_client
        .get_object()
        .bucket(stage_info.bucket.clone())
        .key(&s3_key)
        .send()
        .await
        .map_err(aws_sdk_s3::Error::from)
        .context(S3DownloadSnafu)?;

    // Extract metadata from S3 response and construct the metadata structure directly
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

    // Construct the metadata structure directly without intermediate variables
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

    // Read the encrypted data from the response body
    let encrypted_data = response
        .body
        .collect()
        .await
        .context(ByteStreamSnafu)?
        .into_bytes()
        .to_vec();

    Ok((encrypted_data, file_metadata))
}

async fn create_s3_client(stage_info: &StageInfo, provider_name: &'static str) -> S3Client {
    let credentials = Credentials::new(
        &stage_info.creds.aws_key_id,
        &stage_info.creds.aws_secret_key,
        Some(stage_info.creds.aws_token.clone()),
        None,
        provider_name,
    );

    let config = aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(credentials)
        .region(Region::new(stage_info.region.clone()))
        .load()
        .await;

    S3Client::new(&config)
}

#[derive(Snafu, Debug)]
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
}

#[derive(Snafu, Debug)]
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
}
