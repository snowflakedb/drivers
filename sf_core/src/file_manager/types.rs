use crate::compression_types::CompressionType;
use crate::sensitive::SensitiveString;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use snafu::{Location, Snafu};
use std::fmt;
use std::sync::Arc;

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

// Dedicated file transfer types.
//
// `UploadData`/`DownloadData` carry an optional `Arc<dyn CredentialRefresher>`,
// which has no `Debug` impl; we follow the codebase pattern (see
// `DriverProviders` in `apis/database_driver_v1/global_state.rs`) and omit
// `Debug` on these structs rather than hand-writing a redacting impl.
pub struct UploadData {
    pub src_location_pattern: String,
    pub stage_info: StageInfo,
    pub encryption_material: Option<EncryptionMaterial>,
    pub auto_compress: bool,
    pub source_compression: SourceCompressionParam,
    pub overwrite: bool,
    /// Re-fetches storage credentials when the cloud provider reports an
    /// expired token. `None` makes expired tokens terminal.
    pub credential_refresher: Option<Arc<dyn CredentialRefresher>>,
}

pub struct SingleUploadData {
    pub file_path: String,
    pub filename: String,
    pub stage_info: StageInfo,
    pub encryption_material: Option<EncryptionMaterial>,
    pub auto_compress: bool,
    pub source_compression: SourceCompressionParam,
    pub overwrite: bool,
}

pub struct DownloadData {
    pub src_locations: Vec<String>,
    pub local_location: String,
    pub stage_info: StageInfo,
    pub encryption_materials: Vec<Option<EncryptionMaterial>>,
    pub credential_refresher: Option<Arc<dyn CredentialRefresher>>,
}

pub struct SingleDownloadData {
    pub src_location: String,
    pub local_location: String,
    pub stage_info: StageInfo,
    pub encryption_material: Option<EncryptionMaterial>,
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
    pub end_point: Option<String>,
    /// Presigned URL for GCS operations (when access tokens are not available).
    pub presigned_url: Option<String>,
    /// Whether to use virtual-hosted-style URLs for GCS.
    pub use_virtual_url: bool,
    /// Whether to use regional GCS endpoints.
    pub use_regional_url: bool,
    /// Azure storage account name (required for Azure Blob Storage).
    pub storage_account: Option<String>,
}

/// Error returned by a `CredentialRefresher` when re-fetching storage
/// credentials fails. The underlying error chain is preserved so
/// `error_trace::ErrorTrace` can render it.
#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub))]
pub enum CredentialRefreshError {
    #[snafu(display("Failed to refresh storage credentials"))]
    RefreshFailed {
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Re-fetches cloud storage credentials when the cloud provider reports an
/// expired token mid-transfer. Implementations typically re-execute the
/// originating PUT/GET SQL command and parse fresh `stageInfo.creds`.
///
/// One refresher instance is shared across every file in a PUT/GET batch
/// (`Arc<dyn CredentialRefresher>`): a refresh triggered by file #1 is
/// visible to files #2..N without another server round-trip, and concurrent
/// refresh calls coalesce via the `cache_version` / `seen_version` pairing.
///
/// The trait is dyn-compat, so `refresh` returns a `BoxFuture` — matching
/// `CallerIdentityProvider` in `telemetry::platform_detection::aws`.
pub trait CredentialRefresher: Send + Sync + 'static {
    /// Returns the latest refreshed credentials if any refresh has succeeded,
    /// else `None`. Storage clients call this before each file's first attempt
    /// so a refresh by an earlier file is picked up without an `ExpiredToken`
    /// round-trip.
    fn cached_creds(&self) -> Option<CloudCredentials>;

    /// Version of the in-process credential cache; incremented on every
    /// successful refresh. Paired with `refresh` to coalesce concurrent
    /// expired-token events: a caller that sees `ExpiredToken`, reads version
    /// `N`, and calls `refresh(N)` triggers a server round-trip; other
    /// callers that held `N` but arrive after the refresh completed get the
    /// cached value without hitting the server. "Version" rather than "epoch"
    /// to avoid collision with the Snowflake session epoch.
    fn cache_version(&self) -> u64;

    /// Produces refreshed credentials, skipping the server round-trip if
    /// another caller has already advanced past `seen_version`.
    fn refresh(
        &self,
        seen_version: u64,
    ) -> BoxFuture<'_, Result<CloudCredentials, CredentialRefreshError>>;
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
///
/// `Clone` is used by the S3 credential-refresh retry loop, which re-submits
/// the payload under fresh credentials on `ExpiredToken`.
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
