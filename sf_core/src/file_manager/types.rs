use crate::apis::database_driver_v1::PutGetResultsetFlavor;
use crate::compression_types::CompressionType;
use crate::sensitive::SensitiveString;
use serde::{Deserialize, Serialize};
use snafu::{Location, Snafu};
use std::fmt;
use std::sync::{Arc, RwLock};

/// Result of an upload-or-skip operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadStatus {
    Uploaded,
    Skipped,
}

impl fmt::Display for UploadStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UploadStatus::Uploaded => f.write_str("UPLOADED"),
            UploadStatus::Skipped => f.write_str("SKIPPED"),
        }
    }
}

// Dedicated file transfer types
#[derive(Debug)]
pub struct UploadData {
    pub src_location_pattern: String,
    pub stage_info: StageInfo,
    pub encryption_material: Option<EncryptionMaterial>,
    pub auto_compress: bool,
    pub source_compression: SourceCompressionParam,
    pub overwrite: bool,
    /// Wrapper-specific shape of the PUT result set. Forwarded into each
    /// `SingleUploadData` so that `upload_single_file` can populate the
    /// `message` column according to the active wrapper's contract.
    pub flavor: PutGetResultsetFlavor,
    /// When true, unsupported compression formats (e.g. `.xz`, `.lz`)
    /// detected during PUT auto-detect are treated as uncompressed
    /// instead of producing an error. Mirrors legacy libsnowflakeclient
    /// behavior; required for ODBC backward compatibility.
    pub treat_unsupported_compression_as_uncompressed: bool,
}

pub struct SingleUploadData {
    pub file_path: String,
    pub filename: String,
    pub stage_info: StageInfo,
    pub encryption_material: Option<EncryptionMaterial>,
    pub auto_compress: bool,
    pub source_compression: SourceCompressionParam,
    pub overwrite: bool,
    pub flavor: PutGetResultsetFlavor,
    pub treat_unsupported_compression_as_uncompressed: bool,
}

#[derive(Debug)]
pub struct DownloadData {
    pub src_locations: Vec<String>,
    pub local_location: String,
    pub stage_info: StageInfo,
    pub encryption_materials: Vec<Option<EncryptionMaterial>>,
    /// Wrapper-specific shape of the GET result set. Forwarded into each
    /// `SingleDownloadData` so that `download_single_file` can populate the
    /// `size` column according to the active wrapper's contract (cloud
    /// pre-decryption byte count for ODBC vs. post-decryption length for
    /// Python).
    pub flavor: PutGetResultsetFlavor,
}

#[derive(Debug)]
pub struct SingleDownloadData {
    pub src_location: String,
    pub local_location: String,
    pub stage_info: StageInfo,
    pub encryption_material: Option<EncryptionMaterial>,
    pub flavor: PutGetResultsetFlavor,
}

/// Bytes plus metadata returned by the cloud transfer layer for a single
/// downloaded blob.
///
/// `cloud_byte_count` is the on-cloud (pre-decryption) byte count of the
/// blob — typically the value reported by the storage layer's
/// `Content-Length` header. For `PutGetResultsetFlavor::Odbc` it becomes
/// the value of the GET result's `size` column (legacy `srcFileSize`
/// parity); for `Python` we keep reporting the post-decryption buffer
/// length out of `download_single_file`.
#[derive(Debug)]
pub struct DownloadResponse {
    pub data: Vec<u8>,
    pub digest: Option<String>,
    pub file_metadata: Option<EncryptedFileMetadata>,
    pub cloud_byte_count: i64,
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

/// Cloud storage location type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationType {
    S3,
    Gcs,
    Azure,
}

#[derive(Debug, Clone)]
pub struct StageInfo {
    pub location_type: LocationType,
    pub bucket: String,
    pub key_prefix: String,
    pub region: String,
    pub creds: CloudCredentials,
    /// Cloud endpoint provided by Snowflake (e.g. for FIPS or regional routing).
    /// When present, the storage client uses this instead of the default.
    pub endpoint: Option<String>,
    /// Presigned URL for GCS operations (when access tokens are not available).
    pub presigned_url: Option<String>,
    /// Whether to use virtual-hosted-style URLs for GCS.
    pub use_virtual_url: bool,
    /// Whether to use regional GCS endpoints.
    pub use_regional_url: bool,
    /// Whether to use the S3 regional endpoint (`s3.<region>.amazonaws.com`)
    /// instead of the global one. Set by GS for PrivateLink-to-S3 accounts
    /// and Snowpipe Streaming. Computed as `useS3RegionalUrl || useRegionalUrl`
    /// from the response. Ignored when `endpoint` is set — FIPS / VPCE /
    /// custom endpoint takes precedence.
    pub use_s3_regional_url: bool,
    /// Azure storage account name (required for Azure Blob Storage).
    pub storage_account: Option<String>,
}

/// Cloud storage credentials.
#[derive(Debug, Clone)]
pub enum CloudCredentials {
    /// AWS S3 credentials (access key + secret + session token).
    S3 {
        aws_key_id: String,
        aws_secret_key: SensitiveString,
        aws_token: SensitiveString,
    },
    /// Google Cloud Storage credentials (OAuth2 Bearer token).
    /// Token is `None` when operating in presigned-URL-only mode.
    Gcs {
        gcs_access_token: Option<SensitiveString>,
    },
    /// Azure Blob Storage credentials (SAS token).
    Azure { sas_token: SensitiveString },
}

/// Encryption material for file transfer.
#[derive(Debug, Clone)]
pub struct EncryptionMaterial {
    pub query_stage_master_key: SensitiveString,
    pub query_id: String,
    pub smk_id: String,
}

/// Prepared file data ready for cloud upload.
/// For client-side encryption: contains encrypted data + encryption metadata.
/// For server-side encryption (SSE): contains raw data with no encryption metadata.
#[derive(Debug, Clone)]
pub struct PreparedUpload {
    pub data: Vec<u8>,
    /// SHA-256 digest of the data (always present for integrity verification).
    pub digest: String,
    /// Client-side encryption metadata. `None` for SSE stages.
    pub encryption_metadata: Option<EncryptedFileMetadata>,
}

/// Client-side encryption metadata that gets bundled with the uploaded data.
#[derive(Debug, Clone)]
pub struct EncryptedFileMetadata {
    pub encrypted_key: String, // Base64 encoded
    pub iv: String,            // Base64 encoded
    pub material_desc: MaterialDescription,
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

/// Encryption metadata envelope returned by cloud storage providers.
/// Matches the JSON format produced by `build_encryption_metadata_json`.
#[derive(Debug, Deserialize)]
pub(super) struct EncryptionData {
    #[serde(rename = "WrappedContentKey")]
    pub wrapped_content_key: WrappedContentKey,
    #[serde(rename = "ContentEncryptionIV")]
    pub content_encryption_iv: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct WrappedContentKey {
    #[serde(rename = "EncryptedKey")]
    pub encrypted_key: String,
}

/// Builds the Snowflake encryption metadata JSON envelope (shared across all cloud providers).
/// Matches the format used by JDBC/Python/ODBC drivers.
pub(super) fn build_encryption_metadata_json(
    metadata: &EncryptedFileMetadata,
) -> serde_json::Value {
    serde_json::json!({
        "EncryptionMode": "FullBlob",
        "WrappedContentKey": {
            "KeyId": "symmKey1",
            "EncryptedKey": metadata.encrypted_key,
            "Algorithm": "AES_CBC_256"
        },
        "EncryptionAgent": {
            "Protocol": "1.0",
            "EncryptionAlgorithm": "AES_CBC_256"
        },
        "ContentEncryptionIV": metadata.iv,
        "KeyWrappingMetadata": {
            "EncryptionLibrary": "Rust(OpenSSL)"
        }
    })
}

/// Percent-encode a URL path, preserving `/` separators.
/// Matches Python `urllib.parse.quote()` / ODBC `encodeUrlName()` behavior:
/// unreserved chars (RFC 3986) and `/` pass through, everything else is encoded.
pub(super) fn percent_encode_path(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                encoded.push(byte as char)
            }
            _ => {
                use std::fmt::Write;
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
    }
    encoded
}

/// Shared, mutable view of the stage credentials in use for a PUT/GET command.
///
/// The refresher and the file-transfer layer both hold a clone of this cache;
/// when the refresher fetches new credentials it writes them here, and every
/// subsequent S3 call (in this and any other in-flight file in the same
/// batch) reads the fresh value via `snapshot()`. The internal `Arc<RwLock>`
/// lets a future parallel-upload implementation share the same cache across
/// concurrent uploaders without API changes.
#[derive(Debug, Clone)]
pub struct StageCredsCache {
    inner: Arc<RwLock<CloudCredentials>>,
}

impl StageCredsCache {
    pub fn new(creds: CloudCredentials) -> Self {
        Self {
            inner: Arc::new(RwLock::new(creds)),
        }
    }

    /// Returns a clone of the current credentials.
    pub fn snapshot(&self) -> CloudCredentials {
        self.inner
            .read()
            .expect("stage creds cache poisoned")
            .clone()
    }

    /// Stores fresh credentials. Called by `StageCredsRefresher::refresh`.
    pub fn store(&self, new: CloudCredentials) {
        *self.inner.write().expect("stage creds cache poisoned") = new;
    }
}

pub type RefreshFuture<'a> = std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<(), StageCredsRefreshError>> + Send + 'a>,
>;

/// Refreshes the stage credentials shared via `StageCredsCache` in response to
/// an AWS STS `ExpiredToken`.
///
/// `refresh` may be called multiple times per PUT/GET command (e.g. a long
/// batch upload where the *refreshed* token also expires). Implementations are
/// expected to coalesce rapid-fire calls themselves — the production
/// implementation in this driver caches a successful refresh for 10 minutes,
/// matching ODBC's `m_lastRefreshTokenSec` gate. On success the refresher
/// writes the new credentials into its `cache()`; the file-transfer layer
/// reads them back via `StageCredsCache::snapshot()` on the next attempt.
///
/// Callers that don't need refresh pass `None` and fall back to a single
/// pre-fetched credential set with no retry on `ExpiredToken`.
pub trait StageCredsRefresher: Send + Sync {
    fn refresh(&mut self) -> RefreshFuture<'_>;

    /// The credential cache shared between the refresher and the file-transfer
    /// layer. After `refresh` succeeds, callers read the new value from here.
    fn cache(&self) -> &StageCredsCache;
}

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(module, visibility(pub(crate)))]
pub enum StageCredsRefreshError {
    /// The Snowflake query that re-issues PUT/GET to obtain new stage
    /// credentials failed. Carries the underlying `ApiError` straight from
    /// `RefreshContext::execute_with_refresh` (or the connection lookup that
    /// precedes it) so error_trace keeps the full chain.
    #[snafu(display("Failed to re-execute PUT/GET SQL during stage credentials refresh"))]
    QueryFailed {
        #[snafu(source(from(crate::apis::database_driver_v1::ApiError, Box::new)))]
        source: Box<crate::apis::database_driver_v1::ApiError>,
        #[snafu(implicit)]
        location: Location,
    },
    /// GS responded with `success: false` — the SQL ran but the server
    /// rejected it.
    #[snafu(display("Stage credentials refresh query rejected by server: {message}"))]
    ServerRejected {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
    /// The refresh response carried a `stageInfo` block but it failed to
    /// parse into the file-manager shape (missing fields, malformed location,
    /// unknown location_type, etc.).
    #[snafu(display("Stage credentials refresh response has malformed stageInfo"))]
    InvalidStageInfo {
        #[snafu(source(from(
            crate::rest::snowflake::query_response::QueryResponseError,
            Box::new
        )))]
        source: Box<crate::rest::snowflake::query_response::QueryResponseError>,
        #[snafu(implicit)]
        location: Location,
    },
    /// The refresh response did not contain a usable `stageInfo` block.
    #[snafu(display("Stage credentials refresh response missing stageInfo"))]
    MissingStageInfo {
        #[snafu(implicit)]
        location: Location,
    },
}
