use crate::compression::CompressionError;
use crate::rest::error::RestError;
use glob;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::thrift_gen::database_driver_v1::{DriverException, StatusCode};
use aws_sdk_s3::primitives::ByteStreamError;

// Dedicated file transfer types
#[derive(Debug)]
pub struct UploadData {
    pub src_location: String,
    pub stage_info: StageInfo,
    pub encryption_material: EncryptionMaterial,
    pub auto_compress: bool,
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
pub struct StageInfo {
    pub location: String,
    pub region: String,
    pub creds: Credentials,
}

#[derive(Debug, Clone)]
pub struct Credentials {
    pub aws_key_id: String,
    pub aws_secret_key: String,
    pub aws_token: String,
}

#[derive(Debug, Clone)]
pub struct EncryptionMaterial {
    pub query_stage_master_key: String,
    pub query_id: String,
    pub smk_id: String,
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

// Error types for file manager operations
#[derive(Error, Debug)]
pub enum FileManagerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Encryption error: {0}")]
    Encryption(#[from] EncryptionError),
    #[error("Compression error: {0}")]
    Compression(#[from] CompressionError),
    #[error("S3 upload error: {0}")]
    S3Upload(#[from] UploadFileError),
    #[error("S3 download error: {0}")]
    S3Download(#[from] DownloadFileError),
    #[error("Rest error: {0}")]
    Rest(#[from] RestError),
    #[error("Path error: {0}")]
    Path(#[from] PathError),
}

impl From<FileManagerError> for DriverException {
    fn from(err: FileManagerError) -> Self {
        DriverException::new(
            format!("FileManager error: {err}"),
            StatusCode::UNKNOWN,
            None,
            None,
            None,
        )
    }
}

#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Rest error: {0}")]
    Rest(#[from] RestError),
    #[error("OpenSSL error: {0}")]
    OpenSsl(#[from] openssl::error::ErrorStack),
    #[error("Base64 decoding error: {0}")]
    Base64Decode(#[from] base64::DecodeError),
}

#[derive(Error, Debug)]
pub enum UploadFileError {
    #[error("S3 error: {0}")]
    S3(#[from] Box<aws_sdk_s3::Error>), // Box to avoid large enum size
    #[error("Rest error: {0}")]
    Rest(#[from] RestError),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum DownloadFileError {
    #[error("S3 error: {0}")]
    S3(#[from] Box<aws_sdk_s3::Error>), // Box to avoid large enum size
    #[error("Rest error: {0}")]
    Rest(#[from] RestError),
    #[error("Deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),
    #[error("File metadata error: {0}")]
    FileMetadata(String),
    #[error("ByteStream error: {0}")]
    ByteStream(#[from] ByteStreamError),
}

#[derive(Error, Debug)]
pub enum PathError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Pattern error: {0}")]
    Pattern(#[from] glob::PatternError),
    #[error("Glob error: {0}")]
    Glob(#[from] glob::GlobError),
}
