use crate::compression_types::CompressionType;
use crate::sensitive::SensitiveString;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

// Type alias for the credential refresh callback
/// Callback to re-execute the PUT command and obtain fresh stage credentials.
/// Returns a new StageInfo with refreshed credentials.
/// All three existing drivers (JDBC, Python, ODBC) implement this by holding
/// a reference to the connection/session and re-executing the original command.
pub type CredentialRefreshFn = Box<
    dyn Fn() -> Pin<
            Box<
                dyn Future<Output = Result<StageInfo, crate::file_manager::FileManagerError>>
                    + Send,
            >,
        > + Send
        + Sync,
>;

// Dedicated file transfer types
pub struct UploadData {
    pub src_location_pattern: String,
    pub stage_info: StageInfo,
    pub encryption_material: EncryptionMaterial,
    pub auto_compress: bool,
    pub source_compression: SourceCompressionParam,
    pub overwrite: bool,
    /// Optional callback to refresh expired stage credentials.
    /// When provided, enables automatic retry on 401 (token expired) errors.
    pub credential_refresh: Option<CredentialRefreshFn>,
}

impl std::fmt::Debug for UploadData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadData")
            .field("src_location_pattern", &self.src_location_pattern)
            .field("stage_info", &self.stage_info)
            .field("encryption_material", &"<redacted>")
            .field("auto_compress", &self.auto_compress)
            .field("source_compression", &self.source_compression)
            .field("overwrite", &self.overwrite)
            .field(
                "credential_refresh",
                &self.credential_refresh.as_ref().map(|_| "<function>"),
            )
            .finish()
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageLocationType {
    S3,
    Gcs,
    Azure,
}

#[derive(Debug, Clone)]
pub enum CloudCredentials {
    Aws {
        key_id: String,
        secret_key: SensitiveString,
        token: SensitiveString,
    },
    Gcs {
        access_token: SensitiveString,
    },
    Azure {
        sas_token: SensitiveString,
    },
}

#[derive(Debug, Clone)]
pub struct StageInfo {
    pub location_type: StageLocationType,
    pub bucket: String,
    pub key_prefix: String,
    pub region: Option<String>, // Optional for GCS
    pub creds: CloudCredentials,
    /// Cloud storage endpoint provided by Snowflake (e.g. for FIPS, regional routing, or custom GCS endpoints).
    /// When present, the client uses this instead of the default.
    pub end_point: Option<String>,
    // GCS-specific URL routing
    pub use_regional_url: bool,
    pub use_virtual_url: bool,
}

/// Encryption material for file transfer.
#[derive(Debug, Clone)]
pub struct EncryptionMaterial {
    pub query_stage_master_key: SensitiveString,
    pub query_id: String,
    pub smk_id: String,
}

// Result of encryption containing encrypted data and metadata
#[derive(Debug, Clone)]
pub struct EncryptionResult {
    pub data: Vec<u8>,
    pub metadata: EncryptedFileMetadata,
}

// Encrypted file metadata that gets bundled with the encrypted data
#[derive(Debug, Clone)]
pub struct EncryptedFileMetadata {
    pub encrypted_key: String, // Base64 encoded
    pub iv: String,            // Base64 encoded
    pub material_desc: MaterialDescription,
    pub digest: String, // SHA-256 digest of the encrypted data
}

// Material description structure for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDescription {
    #[serde(rename = "queryId")]
    pub query_id: String,
    #[serde(rename = "smkId")]
    pub smk_id: String,
    #[serde(rename = "keySize")]
    pub key_size: String,
}
