use crate::apis::database_driver_v1::PutGetResultsetFlavor;
use crate::compression_types::CompressionType;
use crate::sensitive::SensitiveString;
use crate::tls::config::TlsConfig;
use crate::utils::sync::RwLockRecoverExt;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use snafu::{Location, Snafu};
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tempfile::TempPath;

use super::encryption::Encryptor;
use super::multipart::MultipartParams;

/// Result of an upload-or-skip operation. A failed transfer is never
/// represented here — see `upload_files` for fail-fast vs collect-all handling.
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

    /// Opens the source as a fresh blocking `Read`. A `Path` re-opens the file;
    /// a `Bytes` is an O(1) refcount clone behind a `Cursor`. Cheap to call once
    /// per upload retry. A `Path` open failure surfaces here (before the request
    /// body streams) rather than mid-stream.
    pub(crate) fn open(&self) -> std::io::Result<Box<dyn std::io::Read + Send>> {
        match self {
            ByteSource::Path(p) => Ok(Box::new(std::fs::File::open(p)?)),
            ByteSource::Bytes(b) => Ok(Box::new(std::io::Cursor::new(b.clone()))),
        }
    }

    /// Async sibling of [`open`](Self::open) for the reqwest upload path. A
    /// `Path` is opened with `tokio::fs::File::open` so the syscall runs on
    /// tokio's blocking pool — a slow open on a networked filesystem (NFS, EBS)
    /// never stalls the runtime thread — then handed back as a blocking
    /// `std::fs::File` for the encrypting stream, which reads it on its own
    /// `spawn_blocking` task. A `Bytes` is the same O(1) `Cursor` clone, no
    /// syscall. The `Path` open failure still surfaces here, before the body
    /// streams, so it stays a clean non-retryable error rather than a mid-stream
    /// one.
    pub(crate) async fn open_async(&self) -> std::io::Result<Box<dyn std::io::Read + Send>> {
        match self {
            ByteSource::Path(p) => {
                let file = tokio::fs::File::open(p).await?;
                Ok(Box::new(file.into_std().await))
            }
            ByteSource::Bytes(b) => Ok(Box::new(std::io::Cursor::new(b.clone()))),
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
    /// When true, PUT skips re-uploading an object whose stored digest
    /// metadata (S3 `x-amz-meta-sfc-digest` / Azure `x-ms-meta-sfcdigest` /
    /// GCS `x-goog-meta-sfc-digest`) matches the locally-computed SHA-256.
    /// Mirrors Python's `_skip_upload_on_content_match` cursor kwarg
    /// (`storage_client.py:214-218`). Only consulted when the caller
    /// also passes `overwrite=true`; the existence-only branch
    /// (`!overwrite && exists`) short-circuits before this flag.
    pub skip_upload_on_content_match: bool,
    /// Server-resolved multipart knobs (`data.threshold` / `data.parallel`):
    /// the size at/above which the upload switches to multipart and the
    /// concurrent-part count. Defaults (200 MiB / 1) apply when the server
    /// omits them.
    pub multipart: MultipartParams,
    /// Resolved fail-fast (abort) vs collect-all flag; see
    /// [`WrapperPresets::put_get_fastfail_default`](crate::apis::database_driver_v1::WrapperPresets::put_get_fastfail_default).
    pub put_fastfail: bool,
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
    pub multipart: MultipartParams,
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
    /// Server-resolved multipart knobs (`data.threshold` / `data.parallel`):
    /// the size above which the download switches to parallel ranged GETs and
    /// the concurrent-part count. Defaults (200 MiB / 1) apply when the server
    /// omits them.
    pub multipart: MultipartParams,
    /// When `true`, downloaded files are created with the process-default umask
    /// permissions instead of the secure owner-only mode (`0o600`). Mirrors
    /// Python's `unsafe_file_write` connection parameter. Unix-only; no-op on
    /// Windows.
    pub unsafe_file_write: bool,
    /// Resolved fail-fast (abort) vs collect-all flag; see
    /// [`WrapperPresets::put_get_fastfail_default`](crate::apis::database_driver_v1::WrapperPresets::put_get_fastfail_default).
    pub get_fastfail: bool,
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
    pub multipart: MultipartParams,
    /// Forwarded from `DownloadData.unsafe_file_write`; see that field for
    /// semantics.
    pub unsafe_file_write: bool,
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

impl fmt::Display for LocationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocationType::S3 => f.write_str("S3"),
            LocationType::Gcs => f.write_str("GCS"),
            LocationType::Azure => f.write_str("Azure"),
        }
    }
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
    /// TLS configuration for storage HTTP clients (S3/GCS/Azure). Carried on
    /// `StageInfo` so every `create_*_client` site can honour CRL, custom root
    /// store, and the protocol-version window without threading them through
    /// the transfer call chain. Set from the connection's `TlsConfig` in
    /// `perform_put_get_transfer`; defaults to `TlsConfig::default()` elsewhere.
    pub tls_config: TlsConfig,
    /// Driver-owned lazy CRL worker for storage TLS clients. Set alongside
    /// `tls_config` in `perform_put_get_transfer`.
    pub crl_worker: crate::crl::worker::SharedCrlWorker,
    /// Explicit proxy config for the storage HTTP clients (S3/GCS/Azure); set
    /// alongside `tls_config` on the PUT/GET path, `ProxyConfig::default()`
    /// otherwise.
    pub proxy_config: crate::tls::config::ProxyConfig,
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
///
/// `source` is always the **pre-encryption source** (the gzip tempfile, the
/// original file, or an in-memory buffer) — never ciphertext. For client-side
/// encryption the body is encrypted lazily as the cloud SDK pulls it (see
/// `CseParams`); for SSE the source is uploaded as-is.
#[derive(Debug, Clone)]
pub struct PreparedUpload {
    /// The upload body source. A [`PreparedSource::GzipTempfile`] carries its
    /// own unlink guard, so the source can't be detached from — or outlive
    /// without — the tempfile it reads.
    pub(super) source: PreparedSource,
    /// SHA-256 digest of the pre-encryption source (the `sfc-digest`), always
    /// present for integrity verification and computed identically for CSE and
    /// SSE — matching the JDBC/ODBC convention.
    pub digest: String,
    /// Client-side-encryption parameters: `Some` on the CSE path, `None` for
    /// SSE. Bundling the cloud metadata and the encryptor into one `Option`
    /// makes the invalid "one set, the other unset" state unrepresentable —
    /// on the CSE path both are always present, on SSE neither is.
    pub(super) cse: Option<CseParams>,
}

/// The body source for an upload, bundled with any temp-file guard the body
/// depends on. Replaces a bare `ByteSource` plus a parallel `Arc<TempPath>`
/// keep-alive: the guard can no longer be dropped (or forgotten) separately
/// from the path it protects, so a gzip-tempfile path with no live guard —
/// which would unlink the file mid-upload — is unrepresentable.
#[derive(Debug, Clone)]
pub(super) enum PreparedSource {
    /// In-memory buffer (an `auto_compress=false` in-memory payload, or test bytes).
    Bytes(Bytes),
    /// A user-provided file on disk; its lifetime is the caller's.
    Path(PathBuf),
    /// The streaming-gzip output tempfile. `_guard` unlinks it when the last
    /// clone drops, so it travels with the `path` that points at it; held as
    /// `Arc` so retry-clones of `PreparedUpload` share ownership. Never read —
    /// its sole job is the unlink-on-drop.
    GzipTempfile {
        path: PathBuf,
        _guard: Arc<TempPath>,
    },
}

impl PreparedSource {
    /// The wire-level [`ByteSource`] to stream as the upload body. The gzip
    /// tempfile is read through its path like any other file source; the guard
    /// stays held by `self`, keeping the file alive across retries.
    pub(super) fn byte_source(&self) -> ByteSource {
        match self {
            PreparedSource::Bytes(b) => ByteSource::Bytes(b.clone()),
            PreparedSource::Path(p) | PreparedSource::GzipTempfile { path: p, .. } => {
                ByteSource::Path(p.clone())
            }
        }
    }
}

impl From<ByteSource> for PreparedSource {
    /// For a source with no temp-file guard (a user file or an in-memory buffer).
    fn from(source: ByteSource) -> Self {
        match source {
            ByteSource::Bytes(b) => PreparedSource::Bytes(b),
            ByteSource::Path(p) => PreparedSource::Path(p),
        }
    }
}

/// The two client-side-encryption artifacts an upload needs, always produced
/// together by [`super::encryption::build_encryptor`]:
/// - `metadata` — the encrypted file key / IV / material description the cloud
///   stores as object metadata headers.
/// - `encryptor` — the lazy AES-CBC encryptor applied to `source` while building
///   the upload body. Carries the ciphertext length so `Content-Length` can be
///   set before the body streams; AES-CBC is deterministic, so each retry
///   re-encrypts to byte-identical ciphertext.
#[derive(Debug, Clone)]
pub(super) struct CseParams {
    pub(super) metadata: EncryptedFileMetadata,
    pub(super) encryptor: Encryptor,
}

impl PreparedUpload {
    /// Test-only constructor for an unencrypted (SSE) prepared upload — the
    /// `encryptor` injection seam is `pub(super)`, so out-of-crate tests can't
    /// use a struct literal. Production builds `PreparedUpload` directly in
    /// `preprocess_file_before_upload`.
    #[cfg(feature = "test-utils")]
    pub fn new_unencrypted_for_test(data: ByteSource, digest: String) -> Self {
        Self {
            source: data.into(),
            digest,
            cse: None,
        }
    }

    /// Test-only constructor for a client-side-encrypted prepared upload. Pair
    /// the `encryptor` + `encryption_metadata` from `encryption::build_encryptor`.
    #[cfg(feature = "test-utils")]
    pub fn new_encrypted_for_test(
        data: ByteSource,
        digest: String,
        encryption_metadata: EncryptedFileMetadata,
        encryptor: Encryptor,
    ) -> Self {
        Self {
            source: data.into(),
            digest,
            cse: Some(CseParams {
                metadata: encryption_metadata,
                encryptor,
            }),
        }
    }
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
    /// The snapshot and its monotonic store marker live under ONE lock
    /// (threading the marker through the cache's *existing* `Arc<RwLock>`), so a
    /// reader can never observe the marker and the creds disagreeing. The
    /// `Instant` advances on every `store()`; a coalesced refresh stores nothing,
    /// leaving it unchanged. Shared across cache clones via `Arc`, so every
    /// holder sees the same snapshot + marker. A refresher compares the marker it
    /// started with against the current one to tell a real re-fetch (marker
    /// moved) from a coalesced no-op — WITHOUT putting the SAS into an equality
    /// or log path.
    inner: Arc<RwLock<(StageInfoSnapshot, Instant)>>,
}

impl StageInfoCache {
    pub fn new(snapshot: StageInfoSnapshot) -> Self {
        Self {
            inner: Arc::new(RwLock::new((snapshot, Instant::now()))),
        }
    }

    /// Convenience constructor used by S3 / Azure callers that only ever
    /// populate `creds`.
    pub fn new_with_creds(creds: CloudCredentials) -> Self {
        Self::new(StageInfoSnapshot::creds_only(creds))
    }

    /// Returns a clone of the current snapshot.
    pub fn snapshot(&self) -> StageInfoSnapshot {
        self.inner.read_recover().0.clone()
    }

    /// Replaces the full snapshot, advancing the store marker atomically (same
    /// lock) so a refresher can detect that a new snapshot landed. Called by
    /// `StageInfoRefresher::refresh` and `refresh_url` after a PUT/GET re-issue.
    pub fn store(&self, new: StageInfoSnapshot) {
        let mut guard = self.inner.write_recover();
        // Guarantee strict advance even on coarse-resolution clocks: the new
        // marker is always > the previous one, so `new_gen > observed` is
        // unambiguous after a real store.
        let next = Instant::now().max(guard.1 + Duration::from_nanos(1));
        *guard = (new, next);
    }

    /// The monotonic store marker. Lets a refresher tell "a new snapshot was
    /// stored since I last looked" (marker advanced) from "the refresh coalesced
    /// / stored nothing" (marker unchanged), without comparing the credential
    /// value itself.
    ///
    /// Uses `read_recover()` (utils::sync, #651): a poisoned lock is recovered
    /// and logged rather than propagated or `.expect()`-panicked, so the read
    /// never fails.
    pub fn cached_at(&self) -> Instant {
        self.inner.read_recover().1
    }
}

pub type RefreshFuture<'a> = std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<std::time::Instant, StageInfoRefreshError>>
            + Send
            + 'a,
    >,
>;

/// Refreshes the stage info shared via `StageInfoCache` in response to a
/// recoverable cloud-storage error.
///
/// Two recovery paths fire through this trait, distinguished by the entry
/// point — both re-issue the original PUT/GET SQL against GS and write the
/// full `StageInfoSnapshot` (creds + presigned URLs) back into the cache:
///
/// - [`refresh`](Self::refresh) — used for `ExpiredToken`-class errors
///   (S3 STS, GCS 401 bearer). Takes `observed: Instant` (the `cached_at`
///   the caller saw before the error) and returns the new generation
///   (`cache().cached_at()` after the refresh). Implementations coalesce
///   rapid-fire calls: if the cache already holds a generation newer than
///   `observed`, that generation is returned immediately without hitting GS
///   (matches ODBC's `m_lastRefreshTokenSec` gate and Python's
///   `StorageCredential.update` thread-lock).
/// - [`refresh_url`](Self::refresh_url) — used for the GCS 400
///   presigned-URL-expiry path. Takes the destination file name for PUT
///   (rewrites the SQL) or `None` for GET. Each invocation re-issues the
///   SQL bypassing the coalescing window — refreshing for file N must
///   produce a fresh `presignedUrls[]` even if file N-1 refreshed seconds
///   ago. The call site enforces a two-strike guard
///   (`GcsRequestError::PresignedUrlExpired`) to prevent looping.
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
    /// Coalesced refresh — used for `ExpiredToken`-class errors. Returns the
    /// new cache generation (a monotonically advancing `Instant`); if the
    /// cache already holds a generation newer than `observed`, that
    /// generation is returned without hitting GS.
    fn refresh(&self, observed: Instant) -> RefreshFuture<'_>;

    /// Non-coalesced URL refresh — used for GCS 400 per-file presigned-URL
    /// expiry. `current_upload_file` is the destination object name for PUT
    /// (used to rewrite the SQL for that file's URL); pass `None` for GET.
    /// Returns the new cache generation after the SQL re-issue.
    fn refresh_url(&self, current_upload_file: Option<&str>) -> RefreshFuture<'_>;

    /// The snapshot cache shared between the refresher and the file-transfer
    /// layer. After either refresh method succeeds, callers read the new
    /// value from here.
    fn cache(&self) -> &StageInfoCache;
}

#[derive(Debug, Clone, Snafu, error_trace::ErrorTrace)]
#[snafu(module, visibility(pub(crate)))]
pub enum StageInfoRefreshError {
    /// The Snowflake query that re-issues PUT/GET to obtain new stage
    /// info failed. Carries the underlying `ApiError` straight from
    /// `RefreshContext::execute_with_refresh` (or the connection lookup that
    /// precedes it) so error_trace keeps the full chain.
    #[snafu(display("Failed to re-execute PUT/GET SQL during stage info refresh"))]
    QueryFailed {
        // Arc (not Box): ApiError/QueryResponseError aren't Clone, but
        // StageInfoRefreshError must be Clone for the coordinator's Shared
        // single-flight future — Box would not compile.
        #[snafu(source(from(crate::apis::database_driver_v1::ApiError, Arc::new)))]
        source: Arc<crate::apis::database_driver_v1::ApiError>,
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
        // Arc (not Box): ApiError/QueryResponseError aren't Clone, but
        // StageInfoRefreshError must be Clone for the coordinator's Shared
        // single-flight future — Box would not compile.
        #[snafu(source(from(
            crate::rest::snowflake::query_response::QueryResponseError,
            Arc::new
        )))]
        source: Arc<crate::rest::snowflake::query_response::QueryResponseError>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_snapshot() -> StageInfoSnapshot {
        StageInfoSnapshot::creds_only(CloudCredentials::Azure {
            sas_token: "tok".into(),
        })
    }

    #[test]
    fn store_advances_generation_strictly() {
        let cache = StageInfoCache::new(dummy_snapshot());
        let gen0 = cache.cached_at();
        cache.store(dummy_snapshot());
        let gen1 = cache.cached_at();
        cache.store(dummy_snapshot());
        let gen2 = cache.cached_at();
        assert!(
            gen1 > gen0,
            "first store must strictly advance the generation"
        );
        assert!(
            gen2 > gen1,
            "second store must strictly advance the generation"
        );
    }
}
