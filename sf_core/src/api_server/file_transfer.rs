use crate::api_server::encryption::{EncryptedFileMetadata, EncryptionMaterial, encrypt_file_data};
use crate::rest::error::RestError;
use base64::{Engine, engine::general_purpose};
use flate2::{Compression, GzBuilder};
use openssl::hash::{MessageDigest, hash};
use std::fs::File;
use std::io::{Read, Write};

// AWS SDK imports
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};

// Dedicated file transfer types
#[derive(Debug, Clone)]
pub struct FileTransferData {
    pub src_locations: Vec<String>,
    pub stage_info: FileTransferStageInfo,
    pub encryption_materials: Vec<EncryptionMaterial>,
}

#[derive(Debug, Clone)]
pub struct FileTransferStageInfo {
    pub location: String,
    pub region: String,
    pub creds: FileTransferCredentials,
}

#[derive(Debug, Clone)]
pub struct FileTransferCredentials {
    pub aws_key_id: String,
    pub aws_secret_key: String,
    pub aws_token: String,
}

pub async fn transfer_file(data: &FileTransferData) -> Result<(), RestError> {
    // TODO: Implement multiple files transfer

    // Validate exactly 1 source file and 1 encryption material
    if data.src_locations.len() != 1 {
        return Err(RestError::Internal(format!(
            "Expected exactly 1 source file, got {}",
            data.src_locations.len()
        )));
    }
    if data.encryption_materials.len() != 1 {
        return Err(RestError::Internal(format!(
            "Expected exactly 1 encryption material, got {}",
            data.encryption_materials.len()
        )));
    }

    let file_path = &data.src_locations[0];
    let encryption_material = &data.encryption_materials[0];

    tracing::info!("Processing encrypted file: {}", file_path);

    // Read and compress the file data
    let compressed_data = compress_data(file_path)
        .map_err(|e| RestError::Internal(format!("Failed to compress file: {e}")))?;

    // Encrypt the data
    let encryption_result = encrypt_file_data(&compressed_data, encryption_material)?;

    tracing::debug!("Encryption metadata: {:?}", encryption_result.metadata);

    upload_to_s3_simple(
        encryption_result.encrypted_data,
        &data.stage_info,
        file_path,
        Some(&encryption_result.metadata),
    )
    .await?;

    Ok(())
}

// TODO: streaming instead of loading the whole file into memory

fn compress_data(file_path: &str) -> Result<Vec<u8>, std::io::Error> {
    let mut input_file = File::open(file_path)?;
    let mut input_data = Vec::new();
    input_file.read_to_end(&mut input_data)?;

    // Extract filename from path
    let original_filename = std::path::Path::new(file_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");

    // Python Connector adds a "_c" suffix to the filename and replaces it with spaces
    // To match that behavior, we replace the filename with spaces + 2
    let spaces_filename = " ".repeat(original_filename.len() + 2);

    // Use GzBuilder to create gzip with spaces filename and zeroed timestamp
    let mut encoder = GzBuilder::new()
        .mtime(0) // Set timestamp to 0 for consistent normalization
        .filename(spaces_filename)
        .write(Vec::new(), Compression::best());

    encoder.write_all(&input_data)?;
    let compressed_data = encoder.finish()?;

    Ok(compressed_data)
}

async fn upload_to_s3_simple(
    data: Vec<u8>,
    stage_info: &FileTransferStageInfo,
    file_path: &str,
    encryption_metadata: Option<&EncryptedFileMetadata>,
) -> Result<(), RestError> {
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

    // Calculate digest if we have encrypted data
    let mut put_object_request = s3_client
        .put_object()
        .bucket(bucket)
        .key(&s3_key)
        .body(ByteStream::from(data.clone()))
        .content_type("application/octet-stream");

    // Add encryption metadata headers if available
    if let Some(metadata) = encryption_metadata {
        // Calculate SHA256 digest of the encrypted data
        let digest = hash(MessageDigest::sha256(), &data)
            .map_err(|e| RestError::Internal(format!("Failed to calculate digest: {e}")))?;
        let digest_b64 = general_purpose::STANDARD.encode(&digest);

        put_object_request = put_object_request
            .metadata("sfc-digest", digest_b64)
            .metadata("x-amz-iv", &metadata.iv)
            .metadata("x-amz-key", &metadata.encrypted_key)
            .metadata("x-amz-matdesc", &metadata.mat_desc);

        tracing::debug!("Added encryption metadata headers to S3 request");
    }

    tracing::debug!("PUT object request: {:?}", put_object_request);

    // Upload to S3 (with optional encryption metadata)
    let result = put_object_request
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

    // Helper functions for hex conversion
    fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
            .collect()
    }

    fn bytes_to_hex(bytes: &[u8]) -> String {
        bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }

    #[test]
    fn test_compress_test_normal_put_csv() {
        // Content: "1,2,3\n" (note: this matches the hex 31 2c 32 2c 33 0a)
        let content = "1,2,3\n";

        // Expected content before compression (hex): 31 2c 32 2c 33 0a
        let expected_content_hex = "312c322c330a";

        // Expected content after compression (hex bytes):
        let expected_compressed_hex = "1f8b08080000000002ff2020202020202020202020202020202020202020200033d431d231e602002eb41e0506000000";

        // Create a temporary directory and file with exact name "test_normal_put.csv"
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_normal_put.csv");
        std::fs::write(&file_path, content.as_bytes()).unwrap();

        let file_path = file_path.to_str().unwrap();

        // Verify content before compression
        let content_hex = bytes_to_hex(content.as_bytes());
        println!("DEBUG: Content: {:?}", content);
        println!("DEBUG: Content (hex): {}", content_hex);
        println!("DEBUG: Expected content (hex): {}", expected_content_hex);

        // Verify content hex matches expected
        assert_eq!(
            content_hex, expected_content_hex,
            "Content hex should be 312c322c330a (1,2,3\\n)"
        );

        // Compress the file using our compress_data function
        let compressed_data = compress_data(file_path).expect("Compression should succeed");

        // Convert result to hex for comparison
        let result_hex = bytes_to_hex(&compressed_data);

        println!("DEBUG: Actual compressed (hex): {}", result_hex);
        println!(
            "DEBUG: Expected compressed (hex): {}",
            expected_compressed_hex
        );
        println!("DEBUG: Compressed size: {} bytes", compressed_data.len());

        // Convert expected hex to bytes for comparison
        let expected = hex_to_bytes(expected_compressed_hex).expect("Invalid expected hex");

        // Verify the compressed output matches exactly
        assert_eq!(
            compressed_data, expected,
            "Compressed output doesn't match expected result.\nExpected: {}\nActual:   {}",
            expected_compressed_hex, result_hex
        );

        println!("✅ Compression test successful!");
        println!("   - Input: test_normal_put.csv with content \"1,2,3\\n\"");
        println!(
            "   - Output: {} bytes matching expected hex dump",
            compressed_data.len()
        );
    }
}
