use crate::compression_types::CompressionType;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};

// Dedicated file transfer types
#[derive(Debug)]
pub struct UploadData {
    pub src_location_pattern: String,
    pub stage_info: StageInfo,
    pub encryption_material: EncryptionMaterial,
    pub auto_compress: bool,
    pub source_compression: SourceCompressionParam,
    pub overwrite: bool,
}

pub struct SingleUploadData {
    pub file_path: String,
    pub filename: String,
    pub stage_info: StageInfo,
    pub encryption_material: EncryptionMaterial,
    pub auto_compress: bool,
    pub source_compression: SourceCompressionParam,
    pub overwrite: bool,
}

#[derive(Debug)]
pub struct DownloadData {
    pub src_locations: Vec<String>,
    pub local_location: String,
    pub stage_info: StageInfo,
    pub encryption_materials: Vec<EncryptionMaterial>,
}

#[derive(Debug)]
pub struct SingleDownloadData {
    pub src_location: String,
    pub local_location: String,
    pub stage_info: StageInfo,
    pub encryption_material: EncryptionMaterial,
}

#[derive(Debug, Clone)]
pub struct UploadMetadata {
    pub source: String,
    pub target: String,
    pub source_size: i64,
    pub target_size: i64,
    pub source_compression: CompressionType,
    pub target_compression: CompressionType,
}

// Result types for file operations
#[derive(Debug, Clone)]
pub struct UploadResult {
    pub source: String,
    pub target: String,
    pub source_size: i64,
    pub target_size: i64,
    pub source_compression: String,
    pub target_compression: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub file: String,
    pub size: i64,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum SourceCompressionParam {
    Gzip,
    Bzip2,
    Brotli,
    Zstd,
    Deflate,
    RawDeflate,
    None,
    AutoDetect,
}

#[derive(Debug, Clone)]
pub struct StageInfo {
    pub bucket: String,
    pub key_prefix: String,
    pub region: String,
    pub creds: Credentials,
    /// S3 endpoint provided by Snowflake (e.g. for FIPS or regional routing).
    /// When present, the S3 client uses this instead of the SDK default.
    pub end_point: Option<String>,
}

/// AWS credentials for S3 stage access.
///
/// Sensitive fields (aws_secret_key, aws_token) are wrapped to prevent accidental logging.
#[derive(Clone)]
pub struct Credentials {
    pub aws_key_id: String,
    pub aws_secret_key: SecretString,
    pub aws_token: SecretString,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("aws_key_id", &self.aws_key_id)
            .field("aws_secret_key", &"***")
            .field("aws_token", &"***")
            .finish()
    }
}

/// Encryption material for file transfer.
///
/// The master key is sensitive and wrapped to prevent accidental logging.
#[derive(Clone)]
pub struct EncryptionMaterial {
    pub query_stage_master_key: SecretString,
    pub query_id: String,
    pub smk_id: String,
}

impl std::fmt::Debug for EncryptionMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptionMaterial")
            .field("query_stage_master_key", &"***")
            .field("query_id", &self.query_id)
            .field("smk_id", &self.smk_id)
            .finish()
    }
}

// Result of encryption containing encrypted data and metadata
#[derive(Debug)]
pub struct EncryptionResult {
    pub data: Vec<u8>,
    pub metadata: EncryptedFileMetadata,
}

// Encrypted file metadata that gets bundled with the encrypted data
#[derive(Debug)]
pub struct EncryptedFileMetadata {
    pub encrypted_key: String, // Base64 encoded
    pub iv: String,            // Base64 encoded
    pub material_desc: MaterialDescription,
    pub digest: String, // SHA-256 digest of the encrypted data
}

// Material description structure for JSON serialization
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialDescription {
    #[serde(rename = "queryId")]
    pub query_id: String,
    #[serde(rename = "smkId")]
    pub smk_id: String,
    #[serde(rename = "keySize")]
    pub key_size: String,
}
