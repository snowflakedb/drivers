use crate::rest::error::RestError;
use crate::rest::snowflake::query::ExecResponseData;
use flate2::{Compression, GzBuilder};
use std::fs::File;
use std::io::{Read, Write};

// AWS SDK imports
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};

// TODO: Encrypt the file before uploading to S3

pub fn transfer_file(data: &ExecResponseData) -> Result<(), RestError> {
    // Extract the source file path
    let file_path = data
        .src_locations
        .as_ref()
        .and_then(|locations| locations.first())
        .ok_or_else(|| {
            RestError::Internal("Source file location not found in response".to_string())
        })?;

    let compressed_data = compress_data(file_path)
        .map_err(|e| RestError::Internal(format!("Failed to compress file: {e}")))?;

    // Get stage info for S3 upload
    let stage_info = data
        .stage_info
        .as_ref()
        .ok_or_else(|| RestError::Internal("Stage info not found in response".to_string()))?;

    // Upload to S3 (without encryption for now)
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| RestError::Internal(format!("Failed to create async runtime: {e}")))?;

    runtime
        .block_on(async { upload_to_s3_simple(&compressed_data, stage_info, file_path).await })
        .map_err(|e| RestError::Internal(format!("Failed to upload to S3: {e}")))?;

    Ok(())
}

// TODO: streaming instead of loading the whole file into memory

fn compress_data(file_path: &str) -> Result<Vec<u8>, std::io::Error> {
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
    file_path: &str,
) -> Result<(), RestError> {
    // Extract AWS credentials from stage info
    let creds = stage_info
        .creds
        .as_ref()
        .ok_or("AWS credentials not found in stage")
        .map_err(|e| RestError::InvalidSnowflakeResponse(e.to_string()))?;

    let aws_key_id = creds
        .aws_key_id
        .as_ref()
        .ok_or("AWS_KEY_ID not found")
        .map_err(|e| RestError::InvalidSnowflakeResponse(e.to_string()))?;

    let aws_secret_key = creds
        .aws_secret_key
        .as_ref()
        .ok_or("AWS_SECRET_KEY not found")
        .map_err(|e| RestError::InvalidSnowflakeResponse(e.to_string()))?;

    let aws_token = creds
        .aws_token
        .as_ref()
        .ok_or("AWS_TOKEN not found")
        .map_err(|e| RestError::InvalidSnowflakeResponse(e.to_string()))?;

    // Create AWS credentials
    let credentials = Credentials::new(
        aws_key_id,
        aws_secret_key,
        Some(aws_token.clone()),
        None,
        "snowflake-upload",
    );

    // Configure AWS client
    let region = stage_info
        .region
        .as_ref()
        .cloned()
        .ok_or("Region not found")
        .map_err(|e| RestError::InvalidSnowflakeResponse(e.to_string()))?;

    let config = aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(credentials)
        .region(Region::new(region))
        .load()
        .await;

    let s3_client = S3Client::new(&config);

    // Extract S3 bucket and key from location
    let location = stage_info
        .location
        .as_ref()
        .ok_or("S3 location not found in Snowflake stage info")
        .map_err(|e| RestError::InvalidSnowflakeResponse(e.to_string()))?;

    // Parse bucket and key prefix from location (format: "bucket-name/path/")
    let bucket_separator = location
        .find('/')
        .ok_or("Invalid S3 location format: missing bucket separator")
        .map_err(|e| RestError::InvalidSnowflakeResponse(e.to_string()))?;

    let bucket = &location[..bucket_separator];
    let key_prefix = &location[bucket_separator + 1..]; // Everything after bucket/

    // Create S3 key: key_prefix + filename.gz
    let file_name = std::path::Path::new(file_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("Invalid file path")
        .map_err(|e| RestError::Internal(e.to_string()))?;

    let s3_key = format!("{key_prefix}{file_name}.gz");

    // Upload to S3 (simple version without encryption)
    let result = s3_client
        .put_object()
        .bucket(bucket)
        .key(&s3_key)
        .body(ByteStream::from(data.to_vec()))
        .content_type("application/gzip")
        .send()
        .await
        .map_err(|e| RestError::Internal(format!("Failed to upload to S3: {e}")))?;

    tracing::info!(
        "Successfully uploaded file to S3: s3://{}/{}",
        bucket,
        s3_key
    );
    tracing::debug!("S3 upload result: {:?}", result);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;
    use std::collections::HashSet;
    use std::io::{Read, Write};
    use tempfile::NamedTempFile;

    #[test]
    fn test_gzip_normalization_comprehensive() {
        let test_content = "Test content for comprehensive gzip normalization.\nLine 2\nLine 3\n";
        let mut compressed_outputs = HashSet::new();

        // Create and compress the same content multiple times with slight delays
        for _i in 0..3 {
            // Create a temporary file with the test content
            let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
            temp_file
                .write_all(test_content.as_bytes())
                .expect("Failed to write to temp file");
            temp_file.flush().expect("Failed to flush temp file");

            // Add a small delay to ensure different timestamps
            std::thread::sleep(std::time::Duration::from_secs(2));

            let compressed = compress_data(temp_file.path().to_str().expect("Invalid path"))
                .expect("Failed to compress file");

            compressed_outputs.insert(compressed);
        }

        // All compressed outputs should be identical (only one unique output)
        assert_eq!(
            compressed_outputs.len(),
            1,
            "Gzip normalization failed: {} different outputs for identical content",
            compressed_outputs.len()
        );

        // Get the single compressed output and verify it's valid
        let compressed = compressed_outputs.into_iter().next().unwrap();

        // Verify decompression works
        let mut decoder = GzDecoder::new(&compressed[..]);
        let mut decompressed = String::new();
        decoder
            .read_to_string(&mut decompressed)
            .expect("Failed to decompress");

        assert_eq!(decompressed, test_content);
    }
}
