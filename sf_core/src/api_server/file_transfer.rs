use flate2::{Compression, GzBuilder};
use std::io::{Write, Read};
use std::fs::File;
use std::sync::{Arc, Mutex};
use crate::rest::snowflake::query::ExecResponseData;
use crate::rest::error::RestError;
use crate::driver::Connection;

// AWS SDK imports
use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;

pub fn transfer_file(_conn_ptr: &Arc<Mutex<Connection>>, data: &ExecResponseData) -> Result<(), RestError> {
    // Extract the source file path
    let file_path = data.src_locations
        .as_ref()
        .and_then(|locations| locations.first())
        .ok_or_else(|| RestError::Internal("Source file location not found in response".to_string()))?;

    let compressed_data = compress_and_normalize_gzip(file_path)
        .map_err(|e| RestError::Internal(format!("Failed to compress file: {}", e)))?;

    // Get stage info for S3 upload
    let stage_info = data._stage_info
        .as_ref()
        .ok_or_else(|| RestError::Internal("Stage info not found in response".to_string()))?;

    // Upload to S3 (without encryption for now)
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| RestError::Internal(format!("Failed to create async runtime: {}", e)))?;
    
    runtime.block_on(async {
        upload_to_s3_simple(&compressed_data, stage_info, file_path).await
    }).map_err(|e| RestError::Internal(format!("Failed to upload to S3: {}", e)))?;

    Ok(())
}

// TODO: streaming instead of loading the whole file into memory

pub fn compress_and_normalize_gzip(file_path: &str) -> Result<Vec<u8>, std::io::Error> {
    let mut input_file = File::open(file_path)?;
    let mut input_data = Vec::new();
    input_file.read_to_end(&mut input_data)?;

    // Use GzBuilder to create a normalized gzip encoder with controlled header
    let mut encoder = GzBuilder::new()
        .mtime(0) // Set timestamp to 0 for consistent normalization
        .write(Vec::new(), Compression::default());
    
    encoder.write_all(&input_data)?;
    let compressed_data = encoder.finish()?;
    
    Ok(compressed_data)
}

async fn upload_to_s3_simple(
    data: &[u8], 
    stage_info: &crate::rest::snowflake::query::ExecResponseStageInfo,
    file_path: &str
) -> Result<(), Box<dyn std::error::Error>> {
    // Extract AWS credentials from stage info
    let creds = stage_info._creds
        .as_ref()
        .ok_or("AWS credentials not found in stage info")?;
    
    let aws_key_id = creds._aws_key_id
        .as_ref()
        .ok_or("AWS_KEY_ID not found")?;
    
    let aws_secret_key = creds._aws_secret_key
        .as_ref()
        .ok_or("AWS_SECRET_KEY not found")?;
    
    let aws_token = creds._aws_token
        .as_ref()
        .ok_or("AWS_TOKEN not found")?;
    
    // Create AWS credentials
    let credentials = Credentials::new(
        aws_key_id,
        aws_secret_key,
        Some(aws_token.clone()),
        None,
        "snowflake-upload",
    );
    
    // Configure AWS client
    let region = stage_info._region
        .as_ref()
        .cloned()
        .unwrap_or_else(|| "us-west-2".to_string());
    
    let config = aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(credentials)
        .region(Region::new(region))
        .load()
        .await;
    
    let s3_client = S3Client::new(&config);
    
    // Extract S3 bucket and key from location
    let location = stage_info._location
        .as_ref()
        .ok_or("S3 location not found")?;
    
    // Parse bucket and key prefix from location (format: "bucket-name/path/")
    let parts: Vec<&str> = location.split('/').collect();
    if parts.is_empty() {
        return Err("Invalid S3 location format".into());
    }
    
    let bucket = parts[0];
    let key_prefix = if parts.len() > 1 {
        parts[1..].join("/")
    } else {
        String::new()
    };
    
    // Create S3 key: key_prefix + filename.gz
    let file_name = std::path::Path::new(file_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("Invalid file path")?;
    
    let s3_key = format!("{}{}.gz", key_prefix, file_name);
    
    // Upload to S3 (simple version without encryption)
    let result = s3_client
        .put_object()
        .bucket(bucket)
        .key(&s3_key)
        .body(ByteStream::from(data.to_vec()))
        .content_type("application/gzip")
        .send()
        .await?;
    
    tracing::info!("Successfully uploaded file to S3: s3://{}/{}", bucket, s3_key);
    tracing::debug!("S3 upload result: {:?}", result);
    
    Ok(())
}
