use crate::rest::error::RestError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::thrift_gen::database_driver_v1::{DriverException, StatusCode};
use aws_sdk_s3::error as s3_error;
use aws_sdk_s3::operation::get_object as s3_get_object;
use aws_sdk_s3::operation::put_object as s3_put_object;

// Dedicated file transfer types
#[derive(Debug)]
pub struct UploadData {
    pub src_locations: Vec<String>,
    pub stage_info: StageInfo,
    pub encryption_materials: Vec<EncryptionMaterial>,
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
pub struct StageInfo {
    pub location: String,
    pub region: String,
    pub creds: Credentials,
}

#[derive(Debug)]
pub struct Credentials {
    pub aws_key_id: String,
    pub aws_secret_key: String,
    pub aws_token: String,
}

#[derive(Debug)]
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
    FileTransfer(#[from] FileTransferError),
    #[error("Rest error: {0}")]
    Rest(#[from] RestError),
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
pub enum CompressionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Compression error: {0}")]
    Compression(#[from] flate2::CompressError),
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
pub enum FileTransferError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("S3 upload error: {0}")]
    // Boxed error to avoid large size difference between error types
    // TODO: Remove this once we have SDK-less file transfer
    S3Upload(#[from] Box<s3_error::SdkError<s3_put_object::PutObjectError>>),
    #[error("S3 download error: {0}")]
    S3Download(#[from] Box<s3_error::SdkError<s3_get_object::GetObjectError>>),
    #[error("ByteStream error: {0}")]
    ByteStream(String),
    #[error("Rest error: {0}")]
    Rest(#[from] RestError),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("File metadata error: {0}")]
    FileMetadata(String),
}

// Manual implementation of From since boxing breaks #[from]
impl From<s3_error::SdkError<s3_put_object::PutObjectError>> for FileTransferError {
    fn from(err: s3_error::SdkError<s3_put_object::PutObjectError>) -> Self {
        FileTransferError::S3Upload(Box::new(err))
    }
}

impl From<s3_error::SdkError<s3_get_object::GetObjectError>> for FileTransferError {
    fn from(err: s3_error::SdkError<s3_get_object::GetObjectError>) -> Self {
        FileTransferError::S3Download(Box::new(err))
    }
}
