use super::types::{EncryptionResult, FileUploadError, StageInfo};
use crate::rest::error::RestError;

// AWS SDK imports
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};

// TODO: streaming instead of loading the whole file into memory

pub async fn upload_to_s3(
    encryption_result: EncryptionResult,
    stage_info: &StageInfo,
    file_name: &str,
) -> Result<(), FileUploadError> {
    // Extract AWS credentials from stage info
    let creds = &stage_info.creds;

    // Create AWS credentials
    let credentials = Credentials::new(
        &creds.aws_key_id,
        &creds.aws_secret_key,
        Some(creds.aws_token.clone()),
        None,
        "snowflake-upload",
    );

    // Configure AWS client
    let config = aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(credentials)
        .region(Region::new(stage_info.region.clone()))
        .load()
        .await;

    let s3_client = S3Client::new(&config);

    // Extract S3 bucket and key from location
    let location = &stage_info.location;

    // Parse bucket and key prefix from location (format: "bucket-name/path/")
    let bucket_separator = location.find('/').ok_or(RestError::Internal(format!(
        "Invalid location in stage info {location}"
    )))?;

    let bucket = &location[..bucket_separator];
    let key_prefix = &location[bucket_separator + 1..]; // Everything after bucket/

    // Create S3 key: key_prefix + filename.gz
    let s3_key = format!("{key_prefix}{file_name}.gz");

    // Serialize encryption metadata
    let mat_desc = serde_json::to_string(&encryption_result.metadata.material_desc)?;

    let put_object_request = s3_client
        .put_object()
        .bucket(bucket)
        .key(&s3_key)
        .body(ByteStream::from(encryption_result.data))
        .content_type("application/octet-stream")
        .metadata("sfc-digest", &encryption_result.metadata.digest)
        .metadata("x-amz-iv", &encryption_result.metadata.iv)
        .metadata("x-amz-key", &encryption_result.metadata.encrypted_key)
        .metadata("x-amz-matdesc", mat_desc);

    tracing::debug!("PUT object request: {:?}", put_object_request);

    // Upload to S3 (with optional encryption metadata)
    let result = put_object_request.send().await?;

    tracing::info!(
        "Successfully uploaded file to S3: s3://{}/{}",
        bucket,
        s3_key
    );
    tracing::debug!("S3 upload result: {:?}", result);

    Ok(())
}
