use crate::apis::database_driver_v1::PutGetResultsetFlavor;
use crate::compression_types::CompressionType;
use crate::sensitive::SensitiveString;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use snafu::{Location, Snafu};
use std::fmt;
use std::path::PathBuf;
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

/// The source of bytes for a file upload. `Path` streams from disk;
/// `Bytes` is an in-memory buffer (used by tests and for ciphertext output
/// from the encryption stage).
#[derive(Debug, Clone)]
pub enum ByteSource {
    Path(PathBuf),
    Bytes(Bytes),
}

impl ByteSource {
    /// Reads the entire source into a `Vec<u8>`.
    pub fn into_bytes(self) -> std::io::Result<Vec<u8>> {
        match self {
            ByteSource::Path(p) => std::fs::read(p),
            ByteSource::Bytes(b) => Ok(b.to_vec()),
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
    /// When true, PUT auto-detect mirrors legacy libsnowflakeclient
    /// behavior: (1) unsupported compression formats are silently
    /// treated as uncompressed instead of erroring, and (2) magic-byte
    /// detection consults a short-prefix table (2-byte gzip, 2-byte
    /// zlib mapped to `Deflate`, 4-byte snowflake brotli marker) ahead
    /// of the `infer` crate.
    pub legacy_odbc_compression_autodetect: bool,
    /// When true, PUT skips re-uploading a blob whose stored
    /// `x-ms-meta-sfcdigest` matches the locally-computed SHA-256.
    /// Mirrors Python's `_skip_upload_on_content_match` cursor kwarg
    /// (`storage_client.py:214-218`). Only consulted when the caller
    /// also passes `overwrite=true`; the existence-only branch
    /// (`!overwrite && exists`) short-circuits before this flag.
    pub skip_upload_on_content_match: bool,
}

// TODO: SNOW-3643409 - decouple large bindings and PUT/GET interfaces
pub struct SingleUploadData {
    pub source: ByteSource,
    pub filename: String,
    pub stage_info: StageInfo,
    pub encryption_material: Option<EncryptionMaterial>,
    pub auto_compress: bool,
    pub source_compression: SourceCompressionParam,
    pub overwrite: bool,
    pub flavor: PutGetResultsetFlavor,
    pub legacy_odbc_compression_autodetect: bool,
    pub skip_upload_on_content_match: bool,
}

#[derive(Debug)]
pub struct DownloadData {
    pub src_locations: Vec<String>,
    pub local_location: String,
    pub stage_info: StageInfo,
    pub encryption_materials: Vec<Option<EncryptionMaterial>>,
    /// Server-supplied per-file pre-signed URLs, aligned by index against
    /// `src_locations`. `Some(url)` is the URL GS issued for that file on
    /// GCS GET in presigned-only mode (no access token); `None` means GS
    /// did not provide one for this file (PUT path, GET-with-token path,
    /// S3/Azure stages, or a partial list during stage reconfiguration).
    ///
    /// Invariant: `presigned_urls.len() == src_locations.len()`. The
    /// alignment is established in
    /// `query_response::Data::to_file_download_data` and preserved by the
    /// three-way zip in `download_files`. See
    /// `--gcp--/2.2-server_supplied_presigned_url_list_on_download.md`.
    pub presigned_urls: Vec<Option<String>>,
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
    /// Per-file pre-signed URL chosen by `download_files` from
    /// `DownloadData.presigned_urls`. `Some(url)` is preferred over
    /// `stage_info.presigned_url` (the PUT-only single slot) by the GCS
    /// branch in `download_single_file`. The S3 and Azure branches ignore
    /// this field — neither cloud uses a per-file URL list on GET.
    pub presigned_url: Option<String>,
    pub flavor: PutGetResultsetFlavor,
}

/// Bytes plus metadata returned by the cloud transfer layer for a single
/// downloaded blob. `data` holds the raw cloud bytes (ciphertext for
/// encrypted stages, plaintext for SSE). `cloud_byte_count` is the on-cloud
/// byte count — under `PutGetResultsetFlavor::Odbc` it becomes the GET
/// result's `size` column (legacy `srcFileSize` parity).
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
    Parquet,
    Orc,
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

impl StageInfo {
    /// Returns a copy of `self` with `creds` and `presigned_url` overlaid
    /// from `snapshot`. `presigned_url` is overlaid only when the snapshot
    /// carries one — this keeps S3/Azure callers from clobbering an inherited
    /// single-slot URL they received in the initial PUT response.
    pub(super) fn with_snapshot(&self, snapshot: StageInfoSnapshot) -> StageInfo {
        let mut info = self.clone();
        info.creds = snapshot.creds;
        if snapshot.presigned_url.is_some() {
            info.presigned_url = snapshot.presigned_url;
        }
        info
    }
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
    pub data: ByteSource,
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

/// Snapshot of the stage info pieces that GS re-emits on every PUT/GET
/// re-issue: `creds`, `presigned_url` (PUT-side single slot), and
/// `presigned_urls[]` (GET-side per-file list). All three are produced
/// together by one `StatementExecuteQuery` round-trip, so the refresher
/// stores them in a single cache slot to keep the read/write paths trivially
/// atomic from the file-transfer layer's perspective.
///
/// `presigned_url` and `presigned_urls` are `None` for non-GCS stages and
/// for the GET-with-token GCS path. S3 / Azure consumers project `creds`
/// out of the snapshot and ignore the URL fields.
#[derive(Debug, Clone)]
pub struct StageInfoSnapshot {
    pub creds: CloudCredentials,
    pub presigned_url: Option<String>,
    pub presigned_urls: Option<Vec<Option<String>>>,
}

impl StageInfoSnapshot {
    /// Convenience constructor for the creds-only callers (S3, Azure).
    /// Leaves both URL fields `None`.
    pub fn creds_only(creds: CloudCredentials) -> Self {
        Self {
            creds,
            presigned_url: None,
            presigned_urls: None,
        }
    }
}

/// Shared, mutable view of the stage info (creds + presigned URLs) in use
/// for a PUT/GET command.
///
/// The refresher and the file-transfer layer both hold a clone of this cache;
/// when the refresher re-issues the PUT/GET SQL it writes the resulting
/// `StageInfoSnapshot` here, and every subsequent transfer attempt (in this
/// and any other in-flight file in the same batch) reads the fresh value via
/// `snapshot()`. The internal `Arc<RwLock>` lets a future parallel-upload
/// implementation share the same cache across concurrent uploaders without
/// API changes. S3 / Azure callers project `creds` out of the snapshot and
/// ignore the GCS-only URL fields.
#[derive(Debug, Clone)]
pub struct StageInfoCache {
    inner: Arc<RwLock<StageInfoSnapshot>>,
}

impl StageInfoCache {
    pub fn new(snapshot: StageInfoSnapshot) -> Self {
        Self {
            inner: Arc::new(RwLock::new(snapshot)),
        }
    }

    /// Convenience constructor used by S3 / Azure callers that only ever
    /// populate `creds`.
    pub fn new_with_creds(creds: CloudCredentials) -> Self {
        Self::new(StageInfoSnapshot::creds_only(creds))
    }

    /// Returns a clone of the current snapshot.
    pub fn snapshot(&self) -> StageInfoSnapshot {
        self.inner
            .read()
            .expect("stage info cache poisoned")
            .clone()
    }

    /// Replaces the full snapshot. Called by `StageInfoRefresher::refresh`
    /// and `refresh_url` after a successful PUT/GET re-issue.
    pub fn store(&self, new: StageInfoSnapshot) {
        *self.inner.write().expect("stage info cache poisoned") = new;
    }
}

pub type RefreshFuture<'a> = std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<(), StageInfoRefreshError>> + Send + 'a>,
>;

/// Refreshes the stage info shared via `StageInfoCache` in response to a
/// recoverable cloud-storage error.
///
/// Two recovery paths fire through this trait, distinguished by the entry
/// point — both re-issue the original PUT/GET SQL against GS and write the
/// full `StageInfoSnapshot` (creds + presigned URLs) back into the cache:
///
/// - [`refresh`](Self::refresh) — used for `ExpiredToken`-class errors
///   (S3 STS, GCS 401 bearer). May be called many times per PUT/GET command
///   (long batch upload where the refreshed token also expires).
///   Implementations are expected to coalesce rapid-fire calls themselves:
///   the production implementation in this driver caches a successful
///   refresh for 10 minutes, matching ODBC's `m_lastRefreshTokenSec` gate
///   and Python's `StorageCredential.update` thread-lock.
/// - [`refresh_url`](Self::refresh_url) — used for the GCS 400
///   presigned-URL-expiry path. Each file in a multi-file GET may have its
///   own per-object URL, so this entry point intentionally bypasses the
///   coalescing window — refreshing for file N must produce a fresh
///   `presignedUrls[]` even if file N-1 refreshed seconds ago. The
///   call site enforces a two-strike guard (`GcsRequestError::PresignedUrlExpired`)
///   to prevent looping when the refreshed URL also fails.
///
/// On success either method writes the new snapshot into its `cache()`;
/// the file-transfer layer reads it back via `StageInfoCache::snapshot()`
/// on the next attempt. S3 / Azure callers project `.creds` out of the
/// snapshot; the GCS layer additionally reads `.presigned_url` /
/// `.presigned_urls`.
///
/// Callers that don't need refresh pass `None` and fall back to a single
/// pre-fetched snapshot with no retry on the recoverable errors.
///
/// # Encryption-material invariant
///
/// Neither `refresh` nor `refresh_url` may rotate `UploadData.encryption_material`
/// (`file_manager/mod.rs`). CSE state belongs to the client and rotating it
/// would corrupt in-flight encrypts. The cache only holds the three fields
/// of `StageInfoSnapshot` — there is no encryption-material slot here.
pub trait StageInfoRefresher: Send + Sync {
    /// Coalesced refresh — used for `ExpiredToken`-class errors.
    fn refresh(&mut self) -> RefreshFuture<'_>;

    /// Non-coalesced refresh — used for GCS 400 per-file presigned-URL
    /// expiry. Each invocation re-issues the SQL even if a previous refresh
    /// landed within the coalescing window.
    fn refresh_url(&mut self) -> RefreshFuture<'_>;

    /// The snapshot cache shared between the refresher and the file-transfer
    /// layer. After either refresh method succeeds, callers read the new
    /// value from here.
    fn cache(&self) -> &StageInfoCache;

    /// Informs the refresher of the destination object name of the file
    /// currently being uploaded, so `refresh_url` can rewrite the PUT SQL to
    /// fetch that file's presigned URL (multi-file glob PUT). Called per file
    /// by the GCS upload path before a refresh. Default: no-op (GET callers
    /// and non-refreshing paths).
    fn notify_current_upload_file(&mut self, _dst_file_name: String) {}
}

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(module, visibility(pub(crate)))]
pub enum StageInfoRefreshError {
    /// The Snowflake query that re-issues PUT/GET to obtain new stage
    /// info failed. Carries the underlying `ApiError` straight from
    /// `RefreshContext::execute_with_refresh` (or the connection lookup that
    /// precedes it) so error_trace keeps the full chain.
    #[snafu(display("Failed to re-execute PUT/GET SQL during stage info refresh"))]
    QueryFailed {
        #[snafu(source(from(crate::apis::database_driver_v1::ApiError, Box::new)))]
        source: Box<crate::apis::database_driver_v1::ApiError>,
        #[snafu(implicit)]
        location: Location,
    },
    /// GS responded with `success: false` — the SQL ran but the server
    /// rejected it.
    #[snafu(display("Stage info refresh query rejected by server: {message}"))]
    ServerRejected {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
    /// The refresh response carried a `stageInfo` block but it failed to
    /// parse into the file-manager shape (missing fields, malformed location,
    /// unknown location_type, etc.).
    #[snafu(display("Stage info refresh response has malformed stageInfo"))]
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
    #[snafu(display("Stage info refresh response missing stageInfo"))]
    MissingStageInfo {
        #[snafu(implicit)]
        location: Location,
    },
    /// A per-file PUT URL refresh could not be performed because the command
    /// has no parseable `file://` local path to rewrite for the target file.
    /// Re-issuing the unchanged SQL could return a different file's presigned
    /// URL, so the GCS call site fails fast with `PresignedUrlExpired` rather
    /// than misrouting the upload.
    #[snafu(display(
        "Presigned URL refresh skipped: PUT command has no parseable file:// local path \
         to rewrite for the target file"
    ))]
    PresignedUrlRefreshSkipped {
        #[snafu(implicit)]
        location: Location,
    },
}
