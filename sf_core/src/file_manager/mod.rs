mod azure_transfer;
mod cloud_http;
mod encryption;
mod gcs_transfer;
mod multipart;
mod s3_transfer;

mod path_expansion;
pub mod types;

#[cfg(any(test, feature = "test-utils"))]
pub mod internal {
    pub use super::azure_transfer::download_from_azure_streaming;
    pub use super::cloud_http::{CloudStreamingDownload, CseDownloadInfo, StreamReader};
    pub use super::encryption::{
        EncryptingReader, EncryptionError, Encryptor, build_encryptor, compute_sha256_digest,
        decrypt_ciphertext_to_writer,
    };
    pub use super::gcs_transfer::download_from_gcs_streaming;
    pub use crate::compression::compress_to_tempfile;

    /// Builds a base put/get retry policy with the given `max_attempts`
    /// (zero backoff for instant test runs).
    fn base_policy(max_attempts: u32) -> crate::config::retry::RetryPolicy {
        use crate::config::retry::{BackoffConfig, Jitter, RetryPolicy};
        use std::time::Duration;
        let mut p = RetryPolicy::put_get(&test_params(max_attempts));
        p.backoff = BackoffConfig {
            base: Duration::ZERO,
            factor: 1.0,
            cap: Duration::ZERO,
            jitter: Jitter::None,
        };
        p
    }

    /// Zero-backoff variant of the production Azure retry policy, for tests.
    pub fn azure_test_retry_policy(max_attempts: u32) -> crate::config::retry::RetryPolicy {
        super::azure_transfer::azure_retry_policy(&base_policy(max_attempts))
    }

    /// Zero-backoff variant of the production GCS retry policy, for tests.
    pub fn gcs_test_retry_policy(
        using_presigned_url: bool,
        max_attempts: u32,
    ) -> crate::config::retry::RetryPolicy {
        super::gcs_transfer::gcs_retry_policy(using_presigned_url, &base_policy(max_attempts))
    }

    /// Builds a [`ParamStore`] with only `put_get_max_attempts` set.
    pub fn test_params(max_attempts: u32) -> crate::config::param_store::ParamStore {
        use crate::config::param_registry::param_names;
        use crate::config::settings::Setting;
        let mut params = crate::config::param_store::ParamStore::new();
        params.insert(
            param_names::PUT_GET_MAX_ATTEMPTS.as_str().to_string(),
            Setting::Int(max_attempts as i64),
        );
        params
    }
}

pub use self::types::*;
pub use azure_transfer::download_from_azure;
pub use gcs_transfer::{
    GcsDownloadError, GcsUploadError, download_from_gcs, upload_to_gcs_or_skip,
};
pub use multipart::{FileTooLargeError, MultipartParams, MultipartThreshold};

use crate::apis::database_driver_v1::PutGetResultsetFlavor;
use crate::compression::{CompressionError, compress_to_tempfile};
use crate::compression_types::{CompressionType, CompressionTypeError, try_guess_compression_type};
use crate::config::retry::RetryPolicy;
use azure_transfer::{
    AzureDownloadError, AzureUploadError, azure_retry_policy, download_from_azure_streaming,
    upload_to_azure_or_skip,
};
use encryption::{
    EncryptionError, build_encryptor, compute_sha256_digest, decrypt_ciphertext_to_writer,
};
use gcs_transfer::{download_from_gcs_streaming, gcs_retry_policy};
use path_expansion::{PathExpansionError, expand_filenames};
use s3_transfer::{
    DownloadFileError, S3Download, S3DownloadBody, SpillTarget, SpilledBody, UploadFileError,
    download_from_s3, upload_to_s3_or_skip,
};
use snafu::{Location, ResultExt, Snafu};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Message string emitted in the PUT result's `message` column when the
/// upload outcome is `Skipped` under `PutGetResultsetFlavor::Odbc`. Mirrors
/// `#define MESSAGE_SKIPPED "File with same name already exists. SKIPPED"`
/// from legacy libsnowflakeclient's `FileTransferExecutionResult.cpp`. The
/// `Python` flavor leaves the `message` column empty for skipped uploads,
/// matching the historical universal-driver behaviour.
const ODBC_PUT_MESSAGE_SKIPPED: &str = "File with same name already exists. SKIPPED";

/// Bytes read from the source for compression auto-detection. Every
/// `CompressionType` we currently detect has its magic at offset 0 (gzip 0–1,
/// bzip2 0–2, zstd 0–3, parquet/ORC 0–3), so 16 bytes would suffice today.
/// The 512-byte buffer is future-proofing: the `infer` crate's archive
/// matchers read up to ~265 bytes (e.g. tar's `ustar` at offset 257), so if
/// we ever map one of those archive kinds to a `CompressionType` the buffer
/// already covers it. 512 is O(1) regardless of file size.
const COMPRESSION_DETECT_PREFIX_LEN: usize = 512;

pub async fn upload_files(
    data: &UploadData,
    policy: &RetryPolicy,
    mut refresher: Option<&mut dyn StageInfoRefresher>,
) -> Result<Vec<UploadResult>, FileManagerError> {
    let file_locations =
        expand_filenames(&data.src_location_pattern).context(PathExpansionSnafu)?;

    if file_locations.is_empty() {
        return NoFilesMatchedSnafu {
            pattern: data.src_location_pattern.clone(),
        }
        .fail();
    }

    let mut results = Vec::with_capacity(file_locations.len());

    // The refresher owns the latest stage info (creds + presigned URLs) for
    // the batch via its shared `StageInfoCache`; per-file calls read from
    // that cache, so refreshed creds/URLs heal the remaining files
    // automatically (matching Python's shared `StorageCredential`). The
    // refresher coalesces rapid-fire token refresh calls across files; URL
    // refresh is intentionally not coalesced (each file may carry its own
    // presigned URL).
    for file_location in file_locations {
        let stage_info = current_stage_info(&data.stage_info, refresher.as_deref());
        let path = PathBuf::from(&file_location.path);
        let single_upload_data = SingleUploadData {
            source: ByteSource::Path(path),
            filename: file_location.filename,
            stage_info,
            encryption_material: data.encryption_material.clone(),
            auto_compress: data.auto_compress,
            source_compression: data.source_compression.clone(),
            overwrite: data.overwrite,
            flavor: data.flavor.clone(),
            legacy_odbc_compression_autodetect: data.legacy_odbc_compression_autodetect,
            skip_upload_on_content_match: data.skip_upload_on_content_match,
            multipart: data.multipart,
        };

        let result = upload_single_file(single_upload_data, policy, &mut refresher).await?;
        results.push(result);
    }

    Ok(results)
}

/// Returns a copy of `base` with `creds` and `presigned_url` overlaid from
/// the refresher's current `StageInfoSnapshot`, when a refresher is present.
/// Without a refresher, `base` is returned unchanged.
///
/// The snapshot's `presigned_urls[]` lives on `DownloadData` (not
/// `StageInfo`); the per-file GCS GET path reads it directly from the
/// refresher cache at the call site (see `download_from_gcs`).
fn current_stage_info(base: &StageInfo, refresher: Option<&dyn StageInfoRefresher>) -> StageInfo {
    refresher.map_or_else(
        || base.clone(),
        |r| base.with_snapshot(r.cache().snapshot()),
    )
}

/// Uploads one file. The `refresher` (if any) is used to refresh stage info
/// on recoverable errors:
/// - S3 stages: AWS `ExpiredToken` triggers a creds refresh
///   (`s3_transfer::upload_to_s3_or_skip`).
/// - GCS stages: 401 triggers a creds refresh; 400 in presigned-mode
///   triggers a URL refresh (`gcs_transfer::upload_to_gcs_or_skip`).
/// - Azure stages: SAS URL refresh is out of scope for the current gap stack.
///
/// Refreshed snapshots are stored in the refresher's `StageInfoCache` rather
/// than returned here.
pub async fn upload_single_file(
    data: SingleUploadData,
    policy: &RetryPolicy,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<UploadResult, FileManagerError> {
    // `preprocess_file_before_upload` reads a `ByteSource::Path` itself
    // (streaming), so `upload_single_file` no longer pre-reads the file.
    upload_prepared_source(data.source.clone(), data, policy, refresher).await
}

/// Uploads an in-memory byte buffer to the stage location described by
/// `data`. Skips the `ByteSource::Path` disk read that [`upload_single_file`]
/// delegates and instead wraps the buffer in `ByteSource::Bytes`, sharing the
/// same cloud-upload path so encryption, compression, SHA-256 digesting, and
/// the per-cloud (S3 / GCS / Azure) dispatch behave identically.
///
/// The upload result's `source` column is derived from `data.source` /
/// `data.filename` (see `upload_result_source`); callers that do not surface
/// the upload result back to the user (notably the large-bindings stage
/// uploader) need not set a meaningful `data.source`.
pub async fn upload_in_memory_file(
    buffer: Vec<u8>,
    data: SingleUploadData,
    policy: &RetryPolicy,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<UploadResult, FileManagerError> {
    upload_prepared_source(ByteSource::Bytes(buffer.into()), data, policy, refresher).await
}

/// Shared core of the upload path used by both `upload_single_file` (file
/// source) and `upload_in_memory_file` (in-memory source). Taking the
/// `ByteSource` as a parameter lets both callers reuse the same preprocess +
/// cloud dispatch with no behavior drift.
async fn upload_prepared_source(
    source: ByteSource,
    data: SingleUploadData,
    policy: &RetryPolicy,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<UploadResult, FileManagerError> {
    // `preprocess_file_before_upload` reads the source file from disk and
    // AES-encrypts it (blocking I/O + CPU-bound); run it off the async executor
    // via `spawn_blocking`. `data` is moved in and handed back out so the
    // cloud dispatch below can keep using it without cloning.
    let (data, preprocessed) = tokio::task::spawn_blocking(move || {
        let result = preprocess_file_before_upload(source, &data);
        (data, result)
    })
    .await
    .context(BlockingTaskSnafu)?;
    let (prepared, file_metadata) = preprocessed?;

    let status = match data.stage_info.location_type {
        LocationType::S3 => upload_to_s3_or_skip(
            prepared,
            &data.stage_info,
            file_metadata.target.as_str(),
            data.overwrite,
            policy,
            data.multipart,
            refresher,
        )
        .await
        .context(S3UploadSnafu)?,
        LocationType::Gcs => upload_to_gcs_or_skip(
            prepared,
            &data.stage_info,
            file_metadata.target.as_str(),
            data.overwrite,
            &gcs_retry_policy(data.stage_info.presigned_url.is_some(), policy),
            refresher,
        )
        .await
        .context(GcsUploadSnafu)?,
        LocationType::Azure => upload_to_azure_or_skip(
            prepared,
            &data.stage_info,
            file_metadata.target.as_str(),
            data.overwrite,
            data.skip_upload_on_content_match,
            &azure_retry_policy(policy),
        )
        .await
        .context(AzureUploadSnafu)?,
    };

    // TODO: Right now the message column is only populated for the `Skipped` outcome under
    // the ODBC wrapper preset. Any failure in the upload process today returns an error before
    // this point, so an `ERROR` status is never produced. Revisit when error handling is
    // unified across wrappers.
    Ok(UploadResult {
        source: file_metadata.source,
        target: file_metadata.target,
        source_size: file_metadata.source_size,
        target_size: file_metadata.target_size,
        source_compression: file_metadata
            .source_compression
            .get_snowflake_representation()
            .to_string(),
        target_compression: file_metadata
            .target_compression
            .get_snowflake_representation()
            .to_string(),
        message: upload_result_message(status, &data.flavor).to_string(),
        status: status.to_string(),
    })
}

/// Returns the `message` column value for a completed upload, gated on the
/// active wrapper flavor. Legacy ODBC always populates the message with
/// `ODBC_PUT_MESSAGE_SKIPPED` for skipped uploads (overwrite=false +
/// target already exists); every other (flavor, status) combination uses
/// an empty string.
fn upload_result_message(status: UploadStatus, flavor: &PutGetResultsetFlavor) -> &'static str {
    match (status, flavor) {
        (UploadStatus::Skipped, PutGetResultsetFlavor::Odbc) => ODBC_PUT_MESSAGE_SKIPPED,
        _ => "",
    }
}

/// Returns the `source` column value for a completed upload, gated on the
/// active wrapper flavor and host platform. Legacy driver provides full path
/// verbatim on Windows, the `Odbc` flavor restores that behaviour; every other
/// combination keeps the `Path::file_name()` basename that UD-Python has always
/// reported.
///
/// `is_windows` is parameterized rather than read from `cfg!(windows)`
/// inside the helper so the unit tests can exercise both branches on
/// any host.
fn upload_result_source(
    source: &ByteSource,
    filename: &str,
    flavor: &PutGetResultsetFlavor,
    is_windows: bool,
) -> String {
    match (is_windows, flavor, source) {
        // Windows ODBC parity: emit the original local path (with forward
        // slashes) for the result's `source` column. For in-memory uploads
        // there is no local path, so fall back to the basename like every
        // other (flavor, host) combination.
        (true, PutGetResultsetFlavor::Odbc, ByteSource::Path(p)) => {
            p.display().to_string().replace('\\', "/")
        }
        _ => filename.to_string(),
    }
}

/// Sets file metadata, compresses the file if needed, and optionally encrypts the data.
/// For SSE stages (no encryption material), the data is uploaded without client-side encryption.
fn preprocess_file_before_upload(
    source: ByteSource,
    data: &SingleUploadData,
) -> Result<(PreparedUpload, UploadMetadata), FileManagerError> {
    let (prefix, source_size) = read_prefix_and_size(&source)?;

    let source_compression = get_source_compression(
        data.filename.as_str(),
        &prefix,
        &data.source_compression,
        data.legacy_odbc_compression_autodetect,
    )
    .context(CompressionTypeSnafu)?;

    let result_source = upload_result_source(
        &data.source,
        data.filename.as_str(),
        &data.flavor,
        cfg!(windows),
    );
    let mut target = data.filename.clone();

    let (upload_source, target_compression, gzip_tempfile) =
        if data.auto_compress && source_compression == CompressionType::None {
            // Stream the gzip output to a tempfile instead of buffering it in
            // heap; that tempfile then becomes the upload source (read lazily
            // during the body stream), so it must outlive the upload.
            let (path, temp_path) = compress_to_tempfile(&source).context(CompressionSnafu)?;
            target = format!("{}.gz", data.filename);
            (
                ByteSource::Path(path),
                CompressionType::Gzip,
                Some(temp_path),
            )
        } else {
            (source, source_compression.clone(), None)
        };

    // The upload source after optional auto-compression: the gzip tempfile, the
    // original file, or in-memory bytes. Encryption (CSE) is applied lazily
    // while building the cloud body, so the source is what we measure and hash
    // here; ciphertext is never materialized.
    let source_len = match &upload_source {
        ByteSource::Bytes(b) => b.len() as i64,
        ByteSource::Path(p) => std::fs::metadata(p).context(IoSnafu)?.len() as i64,
    };

    // `sfc-digest` is the SHA-256 of the pre-encryption source for both CSE and
    // SSE (matching JDBC/ODBC), so it can be computed once, up front.
    let digest = compute_sha256_digest(&upload_source).context(DigestComputationSnafu)?;

    let cse = match &data.encryption_material {
        Some(material) => {
            let (encryptor, metadata) =
                build_encryptor(material, source_len).context(EncryptionSnafu)?;
            Some(CseParams {
                metadata,
                encryptor,
            })
        }
        None => None,
    };

    // What actually lands in the stage: ciphertext length for CSE (analytic,
    // from the encryptor), or the source length for SSE.
    let target_size = cse
        .as_ref()
        .map(|c| c.encryptor.cipher_len())
        .unwrap_or(source_len);

    // Bundle the body source with its tempfile guard (if any). For the gzip
    // path the tempfile *is* the source, so the guard travels with it; every
    // other source carries no guard.
    let source = match gzip_tempfile {
        Some(temp_path) => PreparedSource::GzipTempfile {
            path: temp_path.to_path_buf(),
            _guard: Arc::new(temp_path),
        },
        None => PreparedSource::from(upload_source),
    };

    let prepared = PreparedUpload {
        source,
        digest,
        cse,
    };

    Ok((
        prepared,
        UploadMetadata {
            source: result_source,
            target,
            source_size,
            source_compression,
            target_size,
            target_compression,
        },
    ))
}

/// Reads the first `COMPRESSION_DETECT_PREFIX_LEN` bytes for compression
/// auto-detect, plus the source's total byte count.
///
/// For `ByteSource::Path` this opens the file once for the prefix + metadata
/// read; the upload path opens it again later (to compute the digest, and again
/// per attempt to stream/encrypt the body). If the file changes between opens,
/// the `source_size` reported here — and, for CSE, the analytic `Content-Length`
/// derived from it — can disagree with the bytes actually produced, which the
/// cloud SDK rejects (a digest mismatch is the milder failure). This is inherent
/// to streaming a mutable on-disk source; the pre-streaming code did one atomic
/// `read_to_end`, at the cost of the entire memory bound.
fn read_prefix_and_size(source: &ByteSource) -> Result<(Vec<u8>, i64), FileManagerError> {
    match source {
        ByteSource::Path(p) => {
            let f = File::open(p).context(IoSnafu)?;
            let size = f.metadata().context(IoSnafu)?.len() as i64;
            let mut prefix = Vec::with_capacity(COMPRESSION_DETECT_PREFIX_LEN);
            f.take(COMPRESSION_DETECT_PREFIX_LEN as u64)
                .read_to_end(&mut prefix)
                .context(IoSnafu)?;
            Ok((prefix, size))
        }
        ByteSource::Bytes(b) => {
            let prefix = b[..b.len().min(COMPRESSION_DETECT_PREFIX_LEN)].to_vec();
            Ok((prefix, b.len() as i64))
        }
    }
}

/// Uses user-specified compression type or auto-detects the compression type based on the file name and content.
fn get_source_compression(
    filename: &str,
    file_buffer: &[u8],
    source_compression: &SourceCompressionParam,
    legacy_odbc_compression_autodetect: bool,
) -> Result<CompressionType, CompressionTypeError> {
    match source_compression {
        SourceCompressionParam::AutoDetect => auto_detect_source_compression(
            filename,
            file_buffer,
            legacy_odbc_compression_autodetect,
        ),
        SourceCompressionParam::None => Ok(CompressionType::None),
        SourceCompressionParam::Gzip => Ok(CompressionType::Gzip),
        SourceCompressionParam::Bzip2 => Ok(CompressionType::Bzip2),
        SourceCompressionParam::Brotli => Ok(CompressionType::Brotli),
        SourceCompressionParam::Zstd => Ok(CompressionType::Zstd),
        SourceCompressionParam::Deflate => Ok(CompressionType::Deflate),
        SourceCompressionParam::RawDeflate => Ok(CompressionType::RawDeflate),
        SourceCompressionParam::Parquet => Ok(CompressionType::Parquet),
        SourceCompressionParam::Orc => Ok(CompressionType::Orc),
    }
}

/// Returns the resolved compression type for the `AUTO_DETECT` path.
/// `legacy_odbc_compression_autodetect` (true) opts
/// into two libsnowflakeclient-parity behaviors at once (see
/// `WrapperPresets` for the full doc-comment):
///
/// 1. Short-prefix magic-byte table runs ahead of the `infer` crate,
///    detecting 2-byte gzip / 2-byte zlib (mapped to `Deflate`) / 4-byte
///    snowflake brotli marker that `infer` would miss.
/// 2. Unsupported formats (`.xz`, `.lz`, `.lzma`, `.lzo`, `.Z`, plus the
///    buffer-detected equivalents) are silently treated as uncompressed
///    instead of erroring. Recovery is keyed on the
///    `UnsupportedCompressionType` error variant, so it fires regardless
///    of whether detection went through the filename extension or the
///    magic-bytes path.
fn auto_detect_source_compression(
    filename: &str,
    file_buffer: &[u8],
    legacy_odbc_compression_autodetect: bool,
) -> Result<CompressionType, CompressionTypeError> {
    let detected =
        try_guess_compression_type(filename, file_buffer, legacy_odbc_compression_autodetect);
    if legacy_odbc_compression_autodetect {
        match detected {
            Err(CompressionTypeError::UnsupportedCompressionType { .. }) => {
                Ok(CompressionType::None)
            }
            other => other,
        }
    } else {
        detected
    }
}

pub async fn download_files(
    mut data: DownloadData,
    policy: &RetryPolicy,
    mut refresher: Option<&mut dyn StageInfoRefresher>,
) -> Result<Vec<DownloadResult>, FileManagerError> {
    let mut results = Vec::new();

    // Three-way zip: src_locations / encryption_materials / presigned_urls.
    // `presigned_urls` is built in `query_response::to_file_download_data` to
    // be the same length as `src_locations` (padded with `None` when GS
    // omitted entries) so the zip never silently drops a file. See
    // `DownloadData.presigned_urls` doc-comment for the alignment invariant.
    //
    // The per-file index (`enumerate`) is forwarded into `download_single_file`
    // so the GCS layer can re-resolve `presigned_urls[i]` from the refresher
    // cache after a 400-triggered URL refresh.
    let download_iter = data
        .src_locations
        .drain(..)
        .zip(data.encryption_materials.drain(..))
        .zip(data.presigned_urls.drain(..))
        .enumerate();
    for (index, ((file_location, encryption_material), presigned_url)) in download_iter {
        let stage_info = current_stage_info(&data.stage_info, refresher.as_deref());
        let single_download_data = SingleDownloadData {
            src_location: file_location,
            local_location: data.local_location.clone(),
            stage_info,
            encryption_material,
            presigned_url,
            flavor: data.flavor.clone(),
            multipart: data.multipart,
            unsafe_file_write: data.unsafe_file_write,
        };

        let result =
            download_single_file(single_download_data, policy, index, &mut refresher).await?;
        results.push(result);
    }

    Ok(results)
}

/// GET path guard layer 1 (CWE-73, SNOW-3663590; mirrors JDBC
/// `extractSafeDestFileName`): reduce the server-controlled `src_location` to a
/// single safe basename, rejecting empty / `.` / `..` / NUL / separators / `:`.
/// Replaces the old `file_name().unwrap_or(raw)` fallback that leaked the raw
/// (possibly absolute) tainted string into `Path::join`.
fn safe_download_file_name(src_location: &str) -> Result<&str, FileManagerError> {
    let name = match src_location.rfind(['/', '\\']) {
        Some(idx) => &src_location[idx + 1..],
        None => src_location,
    };

    let rejected =
        name.is_empty() || name == "." || name == ".." || name.contains(['\0', '/', '\\', ':']);
    if rejected {
        return DownloadPathRejectedSnafu {
            src_location: src_location.to_string(),
            local_location: String::new(),
        }
        .fail();
    }
    Ok(name)
}

/// GET path guard layer 2 (mirrors JDBC `assertWithinDirectory`): join the
/// safe basename onto the canonicalized `local_location` and confirm the result
/// stays inside it before any file is created. The caller creates
/// `local_location` first (SNOW-3704966), so `canonicalize` here also confirms it.
/// The containment check is defense-in-depth against future layer-1 changes and
/// catches a leaf that already exists as a symlink escaping `base_dir`.
fn resolve_validated_output_path(
    local_location: &str,
    src_location: &str,
) -> Result<PathBuf, FileManagerError> {
    let filename = safe_download_file_name(src_location)?;
    let base_dir = std::fs::canonicalize(local_location).context(IoSnafu)?;
    let output_path = base_dir.join(filename);
    // Layer 1 guarantees a separator-free basename, so the lexical join can only
    // be a direct child. But the leaf may already exist as a symlink pointing
    // outside `base_dir` (JDBC canonicalizes the full dest to catch this); if so,
    // resolve and re-check. A nonexistent leaf is the normal case — no escape.
    let resolved = std::fs::canonicalize(&output_path).unwrap_or_else(|_| output_path.clone());
    if !resolved.starts_with(&base_dir) {
        return DownloadPathRejectedSnafu {
            src_location: src_location.to_string(),
            local_location: local_location.to_string(),
        }
        .fail();
    }
    Ok(output_path)
}

/// Prepares the on-disk destination for one downloaded file: create
/// `local_location` recursively if missing (SNOW-3704966; matches Python
/// `os.makedirs` and JDBC), run the GET path guard, and derive the sibling
/// `<output>.part` temp path (downloads write there and `rename` on success, so
/// observers never see partial plaintext). Blocking; call inside `spawn_blocking`.
fn prepare_download_output_paths(
    local_location: &str,
    src_location: &str,
) -> Result<(PathBuf, PathBuf), FileManagerError> {
    std::fs::create_dir_all(local_location).context(IoSnafu)?;
    let output_path = resolve_validated_output_path(local_location, src_location)?;
    let partial_path = {
        let mut s = output_path.as_os_str().to_owned();
        s.push(".part");
        PathBuf::from(s)
    };
    Ok((output_path, partial_path))
}

/// Downloads one file. See `upload_single_file` for the refresh semantics.
///
/// `per_file_index` is the file's index inside the GET batch — i.e. its
/// position in `DownloadData.presigned_urls` / `DownloadData.src_locations`.
/// The GCS branch uses it to re-pick `presigned_urls[i]` from the refresher
/// cache after a 400-triggered URL refresh. Non-GCS branches ignore it.
///
/// For GCS and Azure, the response body is streamed directly into the
/// decrypt/write operation via `decrypt_ciphertext_to_writer` without buffering
/// the full ciphertext in memory. The blocking decrypt call runs in
/// `tokio::task::spawn_blocking` so the async runtime thread is free while the
/// blocking channel receive waits for the next chunk from the async producer.
///
/// For S3, a single buffered GET is used below the multipart threshold and
/// parallel ranged GETs into a tempfile above it. CSE objects decrypt the
/// ciphertext through a blocking `Read`; SSE objects skip decryption — a spilled
/// (ranged) download is renamed into place, an in-memory (small) one is copied
/// straight to the destination.
pub async fn download_single_file(
    mut data: SingleDownloadData,
    policy: &RetryPolicy,
    per_file_index: usize,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<DownloadResult, FileManagerError> {
    // Blocking FS syscalls (create_dir_all/canonicalize); keep off the async executor.
    let (output_path, partial_path) = {
        let local_location = data.local_location.clone();
        let src_location = data.src_location.clone();
        tokio::task::spawn_blocking(move || {
            prepare_download_output_paths(&local_location, &src_location)
        })
        .await
        .context(BlockingTaskSnafu)??
    };

    // CSE downloads decrypt the ciphertext through a blocking `Read`; SSE
    // downloads skip decryption and write the raw bytes. S3 buffers small blobs
    // in memory and spills large ranged downloads to a tempfile (renamed into
    // place on the SSE path); GCS/Azure stream from the network.
    //
    // CSE verifies the SHA-256 digest at finalize time rather than pre-checking
    // it: pre-verification would require buffering the full ciphertext, which
    // defeats the streaming refactor. The integrity guarantee is preserved (a
    // tampered byte still yields DigestMismatch); only the failure-mode timing
    // differs. Every branch writes to `partial_path` and renames on success — the
    // user-visible destination only ever appears as a complete artefact, even if
    // a concurrent FS observer is racing.
    // Extract enc_material and unsafe_file_write before the match so all three
    // arms can move them into their spawn_blocking closures
    // (EncryptionMaterial is not Clone).
    let enc_material = data.encryption_material.take();
    let unsafe_file_write = data.unsafe_file_write;
    let (cloud_byte_count, output_byte_len) = match data.stage_info.location_type {
        LocationType::S3 => {
            // Spill parallel ranged downloads next to the destination (not the
            // system temp dir) so the SSE finalize below is a same-filesystem
            // rename rather than a cross-device copy.
            // unwrap_or_else uses "." (current dir) rather than temp_dir so
            // the spill stays on the same filesystem as the destination,
            // keeping the subsequent rename cross-device-safe. temp_dir can
            // be on a different FS, which makes NamedTempFile::persist fail
            // with EXDEV. parent() is only None when output_path has no
            // directory component (a bare filename), in which case "." is
            // the correct implicit parent.
            let spill_dir = output_path
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            // A non-encrypted ranged download assembles straight into `.part`
            // (one rename to publish; any hard-kill leftover is a
            // self-overwriting `.part`). An encrypted (or git-stage) download
            // has `encryption_material`, so its ciphertext goes to a temp in
            // `spill_dir` and is decrypted into `.part` below.
            let spill_target = if enc_material.is_some() {
                SpillTarget::Temp(&spill_dir)
            } else {
                SpillTarget::Part(&partial_path)
            };
            let S3Download {
                body,
                digest,
                file_metadata,
                cloud_byte_count,
            } = download_from_s3(
                &data.stage_info,
                data.src_location.as_str(),
                policy,
                data.multipart,
                refresher,
                spill_target,
            )
            .await
            .context(S3DownloadSnafu)?;

            let partial_path2 = partial_path.clone();

            // Write to `<dst>.part` but do NOT rename inside spawn_blocking.
            // Rename happens after the `.await` (see below), so a cancelled/
            // dropped outer future cannot publish a file written by a detached
            // blocking task. The `.await` itself is the cancellation point.
            let (output_byte_len, spilled_temp) = tokio::task::spawn_blocking(
                move || -> Result<(i64, Option<tempfile::TempPath>), FileManagerError> {
                    match (enc_material, file_metadata, digest) {
                        // Client-side-encrypted object: decrypt the ciphertext
                        // (from the in-memory buffer or the spilled tempfile),
                        // verifying the SHA-256 digest at finalize time.
                        (Some(enc_material), Some(enc_metadata), Some(d)) => {
                            let reader = body.into_reader().context(IoSnafu)?;
                            let mut output_file =
                                create_output_file(&partial_path2, unsafe_file_write)
                                    .context(IoSnafu)?;
                            let result = decrypt_ciphertext_to_writer(
                                reader,
                                &enc_metadata,
                                d.as_str(),
                                &enc_material,
                                &mut output_file,
                            )
                            .context(DecryptionSnafu);
                            write_or_cleanup(output_file, &partial_path2, result).map(|n| (n, None))
                        }
                        // Non-decrypting cases — the cloud bytes are already the
                        // final plaintext:
                        //   * SSE stage — no `encryption_material` (server-side
                        //     decryption).
                        //   * `encryption_material` present but the object carries no
                        //     client-side-encryption headers (e.g. git-stage objects
                        //     on S3) — write raw bytes, matching legacy connector
                        //     behaviour (SNOW git-stage fix).
                        (maybe_enc, _, _) => {
                            if maybe_enc.is_some() {
                                tracing::debug!(
                                    "encryption_material present but S3 encryption headers absent; \
                                     writing raw bytes"
                                );
                            }
                            match body {
                                // Non-encrypted ranged download: the parallel GETs already
                                // assembled the whole object straight into `.part`. Nothing to
                                // copy — signal `None` so the post-await branch renames `.part`
                                // to the output (a single same-FS rename, no copy).
                                S3DownloadBody::Spilled(SpilledBody::Part(_)) => {
                                    Ok((cloud_byte_count, None))
                                }
                                // git-stage ranged download: raw bytes were assembled into a
                                // temp (chosen because `encryption_material` was present). Hand
                                // the TempPath out so the caller renames it straight to output.
                                S3DownloadBody::Spilled(SpilledBody::Temp(temp)) => {
                                    Ok((cloud_byte_count, Some(temp)))
                                }
                                // Small buffered download: copy the already-in-RAM
                                // bytes out (unavoidable and cheap).
                                S3DownloadBody::InMemory(bytes) => {
                                    let mut output_file =
                                        create_output_file(&partial_path2, unsafe_file_write)
                                            .context(IoSnafu)?;
                                    let result = std::io::copy(&mut &bytes[..], &mut output_file)
                                        .map(|n| n as i64)
                                        .context(IoSnafu);
                                    write_or_cleanup(output_file, &partial_path2, result)
                                        .map(|n| (n, None))
                                }
                            }
                        }
                    }
                },
            )
            .await
            .context(BlockingTaskSnafu)??;

            // Atomic publish: rename into place after the .await cancellation point.
            // `Some(temp)` (git-stage ranged download): the raw temp is renamed
            // directly to output — single same-FS rename.
            // `None` (CSE decrypt, InMemory copy, or non-encrypted ranged
            // download): the `.part` file is renamed to output via finalize_rename.
            // Running here — not inside spawn_blocking — means a dropped outer
            // future cannot publish; the blocking task may finish writing, but this
            // rename never executes unless the future reaches this point.
            match spilled_temp {
                Some(temp) => {
                    // git-stage ranged download: rename temp directly to output — single same-FS rename.
                    let output_for_rename = output_path.clone();
                    tokio::task::spawn_blocking(move || {
                        temp.persist(&output_for_rename)
                            .map(|_| ())
                            .map_err(|e| e.error)
                            .context(IoSnafu)
                    })
                    .await
                    .context(BlockingTaskSnafu)??;
                }
                None => {
                    // CSE / InMemory / non-encrypted-ranged path: rename `.part` into place.
                    let partial_for_rename = partial_path.clone();
                    let output_for_rename = output_path.clone();
                    tokio::task::spawn_blocking(move || {
                        finalize_rename(&partial_for_rename, &output_for_rename)
                    })
                    .await
                    .context(BlockingTaskSnafu)?
                    .context(IoSnafu)?;
                }
            }

            (cloud_byte_count, output_byte_len)
        }

        LocationType::Gcs => {
            // TODO(SNOW-3406377): GCS/Azure download arms below are ~70 lines each
            // and differ only in the streaming-download fn called and the error
            // context constructor — extract a shared helper, and fold the
            // `cloud_byte_count_hint > 0 ? hint : cloud_bytes_read` fallback onto
            // `CloudStreamingDownload`. Deferred from this PR to keep diff scoped.
            let dl = download_from_gcs_streaming(
                &data.stage_info,
                data.src_location.as_str(),
                data.presigned_url.as_deref(),
                // Build the policy here, where `using_presigned_url` is known
                // (per-file URL or stage URL), and pass it by reference so the
                // test seam can inject zero backoff.
                &gcs_retry_policy(
                    data.presigned_url.is_some() || data.stage_info.presigned_url.is_some(),
                    policy,
                ),
                per_file_index,
                refresher,
            )
            .await
            .context(GcsDownloadSnafu)?;

            let cloud_byte_count_hint = dl.cloud_byte_count;
            let cse_info = dl.cse_info;
            let reader = dl.reader;
            // Running total of on-cloud ciphertext bytes pulled off the wire,
            // read back after the blocking decrypt task joins. Used as the
            // `cloud_byte_count` fallback when Content-Length is absent.
            let cloud_bytes_read = reader.bytes_read_handle();
            let partial_path2 = partial_path.clone();
            let output_path2 = output_path.clone();
            // Blocking decrypt/write in a spawn_blocking task so the async runtime
            // thread is free to run the GCS producer that feeds the channel reader.
            let output_byte_len =
                tokio::task::spawn_blocking(move || -> Result<i64, FileManagerError> {
                    match (enc_material.as_ref(), cse_info) {
                        (Some(enc_material), Some(cse)) => {
                            let mut output_file =
                                create_output_file(&partial_path2, unsafe_file_write)
                                    .context(IoSnafu)?;
                            match decrypt_ciphertext_to_writer(
                                reader,
                                &cse.metadata,
                                &cse.digest,
                                enc_material,
                                &mut output_file,
                            ) {
                                Ok(n) => {
                                    drop(output_file);
                                    finalize_rename(&partial_path2, &output_path2)
                                        .context(IoSnafu)?;
                                    Ok(n)
                                }
                                Err(e) => {
                                    drop(output_file);
                                    warn_remove_partial(&partial_path2);
                                    Err(e).context(DecryptionSnafu)
                                }
                            }
                        }
                        // encryption_material present but the object carries no CSE
                        // headers (e.g. git-stage objects on GCS). Write raw bytes,
                        // matching legacy connector behaviour (SNOW git-stage fix).
                        (Some(_), None) => {
                            tracing::debug!(
                                "encryption_material present but GCS CSE headers absent; \
                                 writing raw bytes"
                            );
                            let mut output_file =
                                create_output_file(&partial_path2, unsafe_file_write)
                                    .context(IoSnafu)?;
                            match std::io::copy(&mut { reader }, &mut output_file) {
                                Ok(n) => {
                                    drop(output_file);
                                    finalize_rename(&partial_path2, &output_path2)
                                        .context(IoSnafu)?;
                                    Ok(n as i64)
                                }
                                Err(e) => {
                                    drop(output_file);
                                    warn_remove_partial(&partial_path2);
                                    Err(e).context(IoSnafu)
                                }
                            }
                        }
                        (None, _) => {
                            let mut output_file =
                                create_output_file(&partial_path2, unsafe_file_write)
                                    .context(IoSnafu)?;
                            match std::io::copy(&mut { reader }, &mut output_file) {
                                Ok(n) => {
                                    drop(output_file);
                                    finalize_rename(&partial_path2, &output_path2)
                                        .context(IoSnafu)?;
                                    Ok(n as i64)
                                }
                                Err(e) => {
                                    drop(output_file);
                                    warn_remove_partial(&partial_path2);
                                    Err(e).context(IoSnafu)
                                }
                            }
                        }
                    }
                })
                .await
                .context(BlockingTaskSnafu)??;

            // Use Content-Length hint as cloud_byte_count; if absent (chunked TE),
            // fall back to the on-cloud ciphertext bytes actually pulled off the
            // wire. We must NOT fall back to output_byte_len here: for CSE objects
            // that is the decrypted *plaintext* length, which under-reports the
            // on-cloud size by the AES-CBC PKCS#7 padding delta (1–16 bytes) and
            // violates the documented "on-cloud (pre-decryption) byte count" contract.
            let cloud_byte_count = if cloud_byte_count_hint > 0 {
                cloud_byte_count_hint
            } else {
                cloud_bytes_read.load(std::sync::atomic::Ordering::Relaxed) as i64
            };
            (cloud_byte_count, output_byte_len)
        }

        LocationType::Azure => {
            let dl = download_from_azure_streaming(
                &data.stage_info,
                data.src_location.as_str(),
                &azure_retry_policy(policy),
            )
            .await
            .context(AzureDownloadSnafu)?;

            let cloud_byte_count_hint = dl.cloud_byte_count;
            let cse_info = dl.cse_info;
            let reader = dl.reader;
            let cloud_bytes_read = reader.bytes_read_handle();
            let partial_path2 = partial_path.clone();
            let output_path2 = output_path.clone();

            let output_byte_len =
                tokio::task::spawn_blocking(move || -> Result<i64, FileManagerError> {
                    match (enc_material.as_ref(), cse_info) {
                        (Some(enc_material), Some(cse)) => {
                            let mut output_file =
                                create_output_file(&partial_path2, unsafe_file_write)
                                    .context(IoSnafu)?;
                            match decrypt_ciphertext_to_writer(
                                reader,
                                &cse.metadata,
                                &cse.digest,
                                enc_material,
                                &mut output_file,
                            ) {
                                Ok(n) => {
                                    drop(output_file);
                                    finalize_rename(&partial_path2, &output_path2)
                                        .context(IoSnafu)?;
                                    Ok(n)
                                }
                                Err(e) => {
                                    drop(output_file);
                                    warn_remove_partial(&partial_path2);
                                    Err(e).context(DecryptionSnafu)
                                }
                            }
                        }
                        // encryption_material present but the object carries no CSE
                        // headers (e.g. git-stage objects on Azure). Write raw bytes,
                        // matching legacy connector behaviour (SNOW git-stage fix).
                        (Some(_), None) => {
                            tracing::debug!(
                                "encryption_material present but Azure CSE headers absent; \
                                 writing raw bytes"
                            );
                            let mut output_file =
                                create_output_file(&partial_path2, unsafe_file_write)
                                    .context(IoSnafu)?;
                            match std::io::copy(&mut { reader }, &mut output_file) {
                                Ok(n) => {
                                    drop(output_file);
                                    finalize_rename(&partial_path2, &output_path2)
                                        .context(IoSnafu)?;
                                    Ok(n as i64)
                                }
                                Err(e) => {
                                    drop(output_file);
                                    warn_remove_partial(&partial_path2);
                                    Err(e).context(IoSnafu)
                                }
                            }
                        }
                        (None, _) => {
                            let mut output_file =
                                create_output_file(&partial_path2, unsafe_file_write)
                                    .context(IoSnafu)?;
                            match std::io::copy(&mut { reader }, &mut output_file) {
                                Ok(n) => {
                                    drop(output_file);
                                    finalize_rename(&partial_path2, &output_path2)
                                        .context(IoSnafu)?;
                                    Ok(n as i64)
                                }
                                Err(e) => {
                                    drop(output_file);
                                    warn_remove_partial(&partial_path2);
                                    Err(e).context(IoSnafu)
                                }
                            }
                        }
                    }
                })
                .await
                .context(BlockingTaskSnafu)??;

            let cloud_byte_count = if cloud_byte_count_hint > 0 {
                cloud_byte_count_hint
            } else {
                // Same CSE caveat as the GCS arm: fall back to on-cloud ciphertext
                // bytes, never the decrypted plaintext length (output_byte_len).
                cloud_bytes_read.load(std::sync::atomic::Ordering::Relaxed) as i64
            };
            (cloud_byte_count, output_byte_len)
        }
    };

    tracing::info!(
        "File downloaded to '{}' ({} bytes)",
        output_path.display(),
        output_byte_len
    );

    Ok(DownloadResult {
        file: data.src_location,
        size: download_result_size(cloud_byte_count, output_byte_len, &data.flavor),
        status: "DOWNLOADED".to_string(),
        message: "".to_string(),
    })
}

/// Creates the `.part` output file for a GET download, applying owner-only
/// permissions (`0o600`) on Unix when `unsafe_file_write` is `false`.
///
/// On Unix with `unsafe_file_write = false`, forces mode `0o600`; otherwise uses the process umask.
fn create_output_file(path: &Path, unsafe_file_write: bool) -> std::io::Result<File> {
    #[cfg(unix)]
    if !unsafe_file_write {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        // O_CREAT only sets the mode on newly-created files; if a stale .part
        // file exists its permissions are left untouched by truncate.  fchmod
        // (via set_permissions on the fd) covers that case.
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        return Ok(file);
    }
    let _ = unsafe_file_write;
    File::create(path)
}

/// Best-effort cleanup of the `<output_path>.part` temp file when an
/// atomic-rename download fails mid-stream. Logs (rather than ignoring) the
/// removal error so a subsequent disk-full failure on the same path is at
/// least diagnosable.
fn warn_remove_partial(partial_path: &Path) {
    if let Err(rm_err) = std::fs::remove_file(partial_path) {
        tracing::warn!(
            "failed to remove partial download {}: {}",
            partial_path.display(),
            rm_err
        );
    }
}

/// Write-only finalizer used where rename is handled separately (async arm).
/// Drops the file handle and cleans up `.part` on error.
fn write_or_cleanup(
    output_file: File,
    partial: &Path,
    write_result: Result<i64, FileManagerError>,
) -> Result<i64, FileManagerError> {
    drop(output_file);
    if write_result.is_err() {
        warn_remove_partial(partial);
    }
    write_result
}

/// Atomically promotes the verified `<output>.part` temp file to its final
/// destination. On rename failure (cross-device link, destination is a
/// directory, AV holding the handle on Windows, …) the partial is
/// best-effort-removed via [`warn_remove_partial`] so a failed finalize never
/// orphans a `.part` file beside the user-visible path. The rename error is
/// returned unchanged for the caller to `.context(IoSnafu)`.
fn finalize_rename(partial_path: &Path, output_path: &Path) -> std::io::Result<()> {
    std::fs::rename(partial_path, output_path).inspect_err(|_| warn_remove_partial(partial_path))
}

/// Returns the `size` column value for a completed download, gated on the
/// active wrapper flavor. Legacy ODBC reports the on-cloud
/// (pre-decryption) byte count via `srcFileSize`; Python keeps reporting
/// the post-decryption buffer length.
fn download_result_size(
    cloud_byte_count: i64,
    output_byte_len: i64,
    flavor: &PutGetResultsetFlavor,
) -> i64 {
    match flavor {
        PutGetResultsetFlavor::Odbc => cloud_byte_count,
        _ => output_byte_len,
    }
}

// Error types for file manager operations
#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum FileManagerError {
    #[snafu(display("Failed to read or write file"))]
    Io {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to encrypt data"))]
    Encryption {
        source: EncryptionError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to decrypt data"))]
    Decryption {
        source: EncryptionError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to compress data"))]
    Compression {
        source: CompressionError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to compute file digest"))]
    DigestComputation {
        source: EncryptionError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to upload file to S3"))]
    S3Upload {
        source: UploadFileError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to download file from S3"))]
    S3Download {
        source: DownloadFileError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to upload file to GCS"))]
    GcsUpload {
        source: GcsUploadError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to download file from GCS"))]
    GcsDownload {
        source: GcsDownloadError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to upload file to Azure"))]
    AzureUpload {
        source: AzureUploadError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to download file from Azure"))]
    AzureDownload {
        source: AzureDownloadError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to expand file paths"))]
    PathExpansion {
        source: PathExpansionError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to get compression type"))]
    CompressionType {
        source: CompressionTypeError,
        #[snafu(implicit)]
        location: Location,
        backtrace: snafu::Backtrace,
    },
    #[snafu(display("File does not exist: {pattern}"))]
    NoFilesMatched {
        pattern: String,
        #[snafu(implicit)]
        location: Location,
    },
    /// A GET download was refused because the resolved output path is not a
    /// safe, contained child of the target directory (CWE-73, SNOW-3663590).
    /// Kept distinct from `Io` so the security refusal is discriminable.
    #[snafu(display(
        "Refusing to write GET download outside the target directory \
         (src_location={src_location:?}, local_location={local_location:?})"
    ))]
    DownloadPathRejected {
        src_location: String,
        local_location: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Blocking task failed: {source}"))]
    BlockingTask {
        source: tokio::task::JoinError,
        #[snafu(implicit)]
        location: Location,
    },
}

impl FileManagerError {
    /// Whether this is a "file/object exceeds the cloud's max-object ceiling"
    /// error — an input error the proto boundary routes to `InvalidArgument`
    /// rather than `InternalError`. Defined here because the cloud `*FileError`
    /// enums are private to this module.
    pub(crate) fn is_file_too_large(&self) -> bool {
        matches!(
            self,
            FileManagerError::S3Upload {
                source: UploadFileError::FileTooLarge { .. },
                ..
            } | FileManagerError::S3Download {
                source: DownloadFileError::FileTooLarge { .. },
                ..
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[cfg(unix)]
    #[test]
    fn create_output_file_uses_owner_only_mode_when_unsafe_file_write_is_false() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_owned();
        drop(tmp); // remove so create_output_file creates it fresh

        create_output_file(&path, false).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn create_output_file_uses_owner_only_mode_on_stale_part_file() {
        use std::os::unix::fs::PermissionsExt;
        // Pre-create a .part file with loose permissions to simulate a stale
        // leftover from a previous failed download.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_owned();
        drop(tmp);
        let stale = File::create(&path).unwrap();
        stale
            .set_permissions(std::fs::Permissions::from_mode(0o644))
            .unwrap();
        drop(stale);

        create_output_file(&path, false).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn create_output_file_uses_umask_mode_when_unsafe_file_write_is_true() {
        use std::os::unix::fs::PermissionsExt;

        // Baseline: mode produced by standard File::create (umask-dependent).
        let tmp_base = tempfile::NamedTempFile::new().unwrap();
        let base_path = tmp_base.path().to_owned();
        drop(tmp_base);
        File::create(&base_path).unwrap();
        let baseline_mode = std::fs::metadata(&base_path).unwrap().permissions().mode() & 0o777;

        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_owned();
        drop(tmp);
        create_output_file(&path, true).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;

        // unsafe_file_write=true must use the same permissions as File::create,
        // not the forced 0o600 of the secure path.
        assert_eq!(mode, baseline_mode);
    }

    #[test]
    fn upload_result_message_odbc_skipped_uses_legacy_literal() {
        assert_eq!(
            upload_result_message(UploadStatus::Skipped, &PutGetResultsetFlavor::Odbc),
            ODBC_PUT_MESSAGE_SKIPPED,
        );
    }

    #[test]
    fn upload_result_message_python_skipped_is_empty() {
        assert_eq!(
            upload_result_message(UploadStatus::Skipped, &PutGetResultsetFlavor::Python),
            "",
        );
    }

    #[test]
    fn upload_result_message_odbc_uploaded_is_empty() {
        assert_eq!(
            upload_result_message(UploadStatus::Uploaded, &PutGetResultsetFlavor::Odbc),
            "",
        );
    }

    #[test]
    fn upload_result_message_python_uploaded_is_empty() {
        assert_eq!(
            upload_result_message(UploadStatus::Uploaded, &PutGetResultsetFlavor::Python),
            "",
        );
    }

    #[test]
    fn is_file_too_large_true_for_s3_over_ceiling_upload_and_download() {
        // `u64::MAX` is well past S3's `max_object`, so `compute_part_size`
        // yields the `FileTooLarge` inner error the transfer paths wrap.
        let inner = || {
            Box::new(
                multipart::compute_part_size(u64::MAX, &multipart::MultipartConfig::S3)
                    .unwrap_err(),
            )
        };

        let upload = FileManagerError::S3Upload {
            source: UploadFileError::FileTooLarge {
                source: inner(),
                location: Location::new(file!(), line!(), 0),
            },
            location: Location::new(file!(), line!(), 0),
        };
        assert!(upload.is_file_too_large());

        let download = FileManagerError::S3Download {
            source: DownloadFileError::FileTooLarge {
                source: inner(),
                location: Location::new(file!(), line!(), 0),
            },
            location: Location::new(file!(), line!(), 0),
        };
        assert!(download.is_file_too_large());
    }

    #[test]
    fn is_file_too_large_false_for_unrelated_errors() {
        let not_found = FileManagerError::NoFilesMatched {
            pattern: "no-such-file".to_string(),
            location: Location::new(file!(), line!(), 0),
        };
        assert!(!not_found.is_file_too_large());
    }

    #[test]
    fn odbc_put_message_skipped_matches_legacy_libsnowflakeclient() {
        // The exact string is part of the wrapper contract — every ODBC
        // application that parses the `message` column will key off this
        // value verbatim. Pinning it in a test prevents silent rewording.
        assert_eq!(
            ODBC_PUT_MESSAGE_SKIPPED,
            "File with same name already exists. SKIPPED",
        );
    }

    // BD#17 — `upload_result_source` must return the full source path
    // under `Odbc` on Windows with `\` normalised to `/` (matching the
    // legacy libsnowflakeclient wire-level value, whose `srcFileName`
    // came from the file:// URI parser and was therefore already
    // all-forward-slash), and the basename everywhere else (matching
    // the historical UD-Python behaviour).
    const WINDOWS_BACKSLASH_PATH: &str = r"C:\Users\test\test_data.csv";
    const WINDOWS_MIXED_PATH: &str = r"D:/a\universal-driver\tests\test_data.csv";
    const WINDOWS_FORWARD_SLASH_PATH: &str = "C:/Users/test/test_data.csv";
    const WINDOWS_BACKSLASH_PATH_NORMALISED: &str = "C:/Users/test/test_data.csv";
    const WINDOWS_MIXED_PATH_NORMALISED: &str = "D:/a/universal-driver/tests/test_data.csv";
    const UNIX_FULL_PATH: &str = "/home/test/test_data.csv";
    const BASENAME: &str = "test_data.csv";

    #[test]
    fn upload_result_source_windows_odbc_returns_full_path_with_forward_slashes() {
        // Pure backslash input — the form a path-like API surface might
        // produce; must be normalised to forward slashes to match legacy.
        assert_eq!(
            upload_result_source(
                &ByteSource::Path(PathBuf::from(WINDOWS_BACKSLASH_PATH)),
                BASENAME,
                &PutGetResultsetFlavor::Odbc,
                true,
            ),
            WINDOWS_BACKSLASH_PATH_NORMALISED,
        );
        // Mixed-separator input — the actual shape `glob` produces on
        // Windows when fed a file:// URI pattern (drive letter and first
        // segment as `/`, deeper segments rewritten to `\` during
        // filesystem traversal). This is the case that broke PR4 in CI.
        assert_eq!(
            upload_result_source(
                &ByteSource::Path(PathBuf::from(WINDOWS_MIXED_PATH)),
                BASENAME,
                &PutGetResultsetFlavor::Odbc,
                true,
            ),
            WINDOWS_MIXED_PATH_NORMALISED,
        );
        // Already-normalised input must be returned unchanged.
        assert_eq!(
            upload_result_source(
                &ByteSource::Path(PathBuf::from(WINDOWS_FORWARD_SLASH_PATH)),
                BASENAME,
                &PutGetResultsetFlavor::Odbc,
                true,
            ),
            WINDOWS_FORWARD_SLASH_PATH,
        );
    }

    #[test]
    fn upload_result_source_windows_python_returns_basename() {
        for full_path in [
            WINDOWS_BACKSLASH_PATH,
            WINDOWS_MIXED_PATH,
            WINDOWS_FORWARD_SLASH_PATH,
        ] {
            assert_eq!(
                upload_result_source(
                    &ByteSource::Path(PathBuf::from(full_path)),
                    BASENAME,
                    &PutGetResultsetFlavor::Python,
                    true,
                ),
                BASENAME,
                "Python on Windows must continue stripping directories from `{full_path}`",
            );
        }
    }

    #[test]
    fn upload_result_source_non_windows_returns_basename_for_both_flavors() {
        for flavor in [PutGetResultsetFlavor::Python, PutGetResultsetFlavor::Odbc] {
            assert_eq!(
                upload_result_source(
                    &ByteSource::Path(PathBuf::from(UNIX_FULL_PATH)),
                    BASENAME,
                    &flavor,
                    false,
                ),
                BASENAME,
                "{flavor:?} on non-Windows must always return the basename — \
                 legacy ODBC's `find_last_of('/')` worked correctly on Unix paths",
            );
        }
    }

    #[test]
    fn upload_result_source_basename_only_input_unchanged_for_all_combinations() {
        // When `file_path` already equals the basename (e.g. the user
        // passed a relative single-segment path) the two branches must
        // collapse to the same value regardless of host or flavor.
        // Backslash-free input guarantees the Odbc-on-Windows
        // normalisation is a no-op here.
        for is_windows in [false, true] {
            for flavor in [PutGetResultsetFlavor::Python, PutGetResultsetFlavor::Odbc] {
                assert_eq!(
                    upload_result_source(
                        &ByteSource::Path(PathBuf::from(BASENAME)),
                        BASENAME,
                        &flavor,
                        is_windows,
                    ),
                    BASENAME,
                    "is_windows={is_windows}, flavor={flavor:?} must return {BASENAME}",
                );
            }
        }
    }

    #[test]
    fn upload_result_source_bytes_source_falls_back_to_basename() {
        // For in-memory uploads there is no local path; even Windows ODBC
        // (the only flavor/host combo that would emit a path) must fall
        // back to the basename.
        for is_windows in [false, true] {
            for flavor in [PutGetResultsetFlavor::Python, PutGetResultsetFlavor::Odbc] {
                assert_eq!(
                    upload_result_source(
                        &ByteSource::Bytes(Bytes::new()),
                        BASENAME,
                        &flavor,
                        is_windows,
                    ),
                    BASENAME,
                );
            }
        }
    }

    // BD#4 — `download_single_file` must report the on-cloud
    // (pre-decryption) byte count under `Odbc` (matching legacy
    // libsnowflakeclient `srcFileSize`) and the post-decryption buffer
    // length under `Python` (current UD-Python contract).
    #[test]
    fn download_result_size_odbc_uses_cloud_byte_count() {
        let cloud_byte_count = 32;
        let output_byte_len = 26;
        assert_eq!(
            download_result_size(
                cloud_byte_count,
                output_byte_len,
                &PutGetResultsetFlavor::Odbc
            ),
            cloud_byte_count,
        );
    }

    #[test]
    fn download_result_size_python_uses_output_length() {
        let cloud_byte_count = 32;
        let output_byte_len = 26;
        assert_eq!(
            download_result_size(
                cloud_byte_count,
                output_byte_len,
                &PutGetResultsetFlavor::Python,
            ),
            output_byte_len,
        );
    }

    #[test]
    fn download_result_size_sse_branches_collapse_to_same_value() {
        // For SSE stages (no client-side encryption) the cloud byte
        // count and the post-decryption buffer length are identical, so
        // both wrapper flavors must report exactly `n`.
        for n in [0, 1, 1000] {
            assert_eq!(
                download_result_size(n, n, &PutGetResultsetFlavor::Odbc),
                n,
                "Odbc flavor must report n={n} when cloud == output",
            );
            assert_eq!(
                download_result_size(n, n, &PutGetResultsetFlavor::Python),
                n,
                "Python flavor must report n={n} when cloud == output",
            );
        }
    }

    // CWE-73 (SNOW-3663590) — GET download path guard. Layer 1 strips to a
    // safe basename; layer 2 confirms containment in the target dir. Mirrors
    // JDBC's `DownloadPathValidatorTest`.

    #[test]
    fn safe_download_file_name_plain_basename_passes() {
        assert_eq!(safe_download_file_name("file.csv").unwrap(), "file.csv");
    }

    #[test]
    fn safe_download_file_name_strips_forward_slash_dirs() {
        assert_eq!(safe_download_file_name("a/b/c.csv").unwrap(), "c.csv");
    }

    #[test]
    fn safe_download_file_name_strips_backslash_dirs() {
        assert_eq!(safe_download_file_name(r"a\b\c.csv").unwrap(), "c.csv");
    }

    #[test]
    fn safe_download_file_name_strips_absolute_path_to_basename() {
        // Pre-fix this leaked the raw path into `Path::join`; now only the
        // basename survives, so it can never escape `local_location`.
        assert_eq!(safe_download_file_name("/etc/passwd").unwrap(), "passwd");
    }

    #[test]
    fn safe_download_file_name_rejects_traversal_and_self_refs() {
        for bad in ["..", ".", "a/..", "a/.", "dir/"] {
            assert!(
                matches!(
                    safe_download_file_name(bad),
                    Err(FileManagerError::DownloadPathRejected { .. })
                ),
                "expected {bad:?} to be rejected",
            );
        }
    }

    #[test]
    fn safe_download_file_name_rejects_empty_and_bare_separators() {
        for bad in ["", "/", r"\"] {
            assert!(
                matches!(
                    safe_download_file_name(bad),
                    Err(FileManagerError::DownloadPathRejected { .. })
                ),
                "expected {bad:?} to be rejected",
            );
        }
    }

    #[test]
    fn safe_download_file_name_rejects_nul_and_colon() {
        // `:` guards Windows drive-letter / alternate-data-stream forms.
        for bad in ["evil\0.csv", "C:evil", "stream:ads"] {
            assert!(
                matches!(
                    safe_download_file_name(bad),
                    Err(FileManagerError::DownloadPathRejected { .. })
                ),
                "expected {bad:?} to be rejected",
            );
        }
    }

    #[test]
    fn resolve_validated_output_path_safe_name_stays_in_dir() {
        let dir = tempfile::tempdir().unwrap();
        let base = std::fs::canonicalize(dir.path()).unwrap();
        let out = resolve_validated_output_path(dir.path().to_str().unwrap(), "data.csv").unwrap();
        assert_eq!(out, base.join("data.csv"));
        assert!(out.starts_with(&base));
    }

    #[test]
    fn resolve_validated_output_path_absolute_src_cannot_escape() {
        // Server returns an absolute `src_location`; output must stay in the dir.
        let dir = tempfile::tempdir().unwrap();
        let base = std::fs::canonicalize(dir.path()).unwrap();
        let out = resolve_validated_output_path(dir.path().to_str().unwrap(), "/etc/cron.d/evil")
            .unwrap();
        assert_eq!(out, base.join("evil"));
        assert!(
            out.starts_with(&base),
            "absolute src_location must not escape the target dir: {out:?}",
        );
    }

    #[test]
    fn resolve_validated_output_path_rejects_traversal_src() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            resolve_validated_output_path(dir.path().to_str().unwrap(), "subdir/.."),
            Err(FileManagerError::DownloadPathRejected { .. })
        ));
    }

    #[test]
    fn resolve_validated_output_path_missing_dir_is_io_error() {
        // The guard still requires an existing dir (the GET flow creates it
        // upstream, SNOW-3704966); this pins the guard's standalone contract.
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        assert!(matches!(
            resolve_validated_output_path(missing.to_str().unwrap(), "data.csv"),
            Err(FileManagerError::Io { .. })
        ));
    }

    // SNOW-3704966: a missing destination dir is created recursively before write.
    #[test]
    fn prepare_download_output_paths_creates_missing_dir_recursively() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("nested").join("missing");
        assert!(
            !missing.exists(),
            "precondition: destination must not exist"
        );

        let (output_path, partial_path) =
            prepare_download_output_paths(missing.to_str().unwrap(), "data.csv")
                .expect("missing destination dir must be created, not rejected");

        assert!(
            missing.is_dir(),
            "GET must create the destination directory tree"
        );
        let base = std::fs::canonicalize(&missing).unwrap();
        assert_eq!(output_path, base.join("data.csv"));
        let mut expected_partial = output_path.clone().into_os_string();
        expected_partial.push(".part");
        assert_eq!(partial_path, PathBuf::from(expected_partial));
    }

    #[test]
    fn prepare_download_output_paths_existing_dir_is_ok() {
        // create_dir_all is a no-op when the directory already exists.
        let dir = tempfile::tempdir().unwrap();
        let base = std::fs::canonicalize(dir.path()).unwrap();
        let (output_path, _partial) =
            prepare_download_output_paths(dir.path().to_str().unwrap(), "f.bin").unwrap();
        assert_eq!(output_path, base.join("f.bin"));
    }

    // Mirrors JDBC `symlinkEscapeIsRejected`: a leaf that already exists as a
    // symlink out of the target dir must be refused, not silently followed.
    #[cfg(unix)]
    #[test]
    fn resolve_validated_output_path_rejects_symlink_leaf_escape() {
        let base = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("evil.bin");
        std::fs::write(&target, b"x").unwrap();
        let link = base.path().join("data.csv");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(matches!(
            resolve_validated_output_path(base.path().to_str().unwrap(), "data.csv"),
            Err(FileManagerError::DownloadPathRejected { .. })
        ));
    }

    // BD#6 — when SOURCE_COMPRESSION=AUTO_DETECT detects an unsupported
    // compression format, legacy libsnowflakeclient silently fell back to
    // no compression. ODBC (`legacy_odbc_compression_autodetect = true`)
    // restores that behavior; Python / JDBC (false) keep surfacing the
    // error. JDBC behavior verified equivalent to Python via
    // `SnowflakeFileTransferAgent.java:3163-3308`.
    #[rustfmt::skip]
    const UNSUPPORTED_COMPRESSION_FILENAMES: &[&str] = &[
        "test.xz",
        "test.lzma",
        "test.lz",
        "test.lzo",
        "test.Z",
    ];

    #[test]
    fn auto_detect_source_compression_legacy_flag_true_swallows_unsupported_error() {
        for filename in UNSUPPORTED_COMPRESSION_FILENAMES {
            let result = auto_detect_source_compression(filename, b"", true);
            assert_eq!(
                result.unwrap(),
                CompressionType::None,
                "legacy=true must fall back to None for {filename}",
            );
        }
    }

    #[test]
    fn auto_detect_source_compression_legacy_flag_false_propagates_unsupported_error() {
        for filename in UNSUPPORTED_COMPRESSION_FILENAMES {
            let result = auto_detect_source_compression(filename, b"", false);
            assert!(
                matches!(
                    result,
                    Err(CompressionTypeError::UnsupportedCompressionType { .. })
                ),
                "legacy=false must surface the unsupported error for {filename}, got: {result:?}",
            );
        }
    }

    // Buffer-detection branch (infer crate): an extension-less file whose
    // magic bytes match an unsupported format must still trigger the
    // legacy-flag fallback. Locks in that the recovery is keyed on the
    // `UnsupportedCompressionType` error variant, not on the
    // filename-extension detection path.
    #[test]
    fn auto_detect_source_compression_legacy_flag_true_swallows_buffer_detected_unsupported() {
        let xz_magic = b"\xFD7zXZ\x00\x00\x01\x69\x22\xDE\x36";
        let result = auto_detect_source_compression("noext", xz_magic, true);
        assert_eq!(result.unwrap(), CompressionType::None);
    }

    #[test]
    fn auto_detect_source_compression_legacy_flag_false_propagates_buffer_detected_unsupported() {
        let xz_magic = b"\xFD7zXZ\x00\x00\x01\x69\x22\xDE\x36";
        let result = auto_detect_source_compression("noext", xz_magic, false);
        assert!(
            matches!(
                result,
                Err(CompressionTypeError::UnsupportedCompressionType { .. })
            ),
            "legacy=false must surface the buffer-detected unsupported error, got: {result:?}",
        );
    }

    #[test]
    fn auto_detect_source_compression_recognizes_gzip_for_both_flag_values() {
        for legacy in [false, true] {
            let result = auto_detect_source_compression("test.csv.gz", b"", legacy);
            assert_eq!(
                result.unwrap(),
                CompressionType::Gzip,
                "legacy={legacy} must still recognize supported extensions",
            );
        }
    }

    #[test]
    fn auto_detect_source_compression_returns_none_for_uncompressed_for_both_flag_values() {
        for legacy in [false, true] {
            let result = auto_detect_source_compression("test.csv", b"", legacy);
            assert_eq!(
                result.unwrap(),
                CompressionType::None,
                "legacy={legacy} must report None for plain files",
            );
        }
    }

    #[test]
    fn auto_detect_source_compression_recognizes_parquet_regardless_of_flag() {
        for legacy in [false, true] {
            let result = auto_detect_source_compression("test.parquet", b"", legacy);
            assert_eq!(
                result.unwrap(),
                CompressionType::Parquet,
                "must recognize .parquet regardless of legacy flag (legacy={legacy})",
            );
        }
    }

    #[test]
    fn auto_detect_source_compression_recognizes_orc_regardless_of_flag() {
        for legacy in [false, true] {
            let result = auto_detect_source_compression("test.orc", b"", legacy);
            assert_eq!(
                result.unwrap(),
                CompressionType::Orc,
                "must recognize .orc regardless of legacy flag (legacy={legacy})",
            );
        }
    }

    #[test]
    fn auto_detect_source_compression_recognizes_parquet_magic_regardless_of_flag() {
        for legacy in [false, true] {
            let result = auto_detect_source_compression("noext", b"PAR1payload", legacy);
            assert_eq!(
                result.unwrap(),
                CompressionType::Parquet,
                "must recognize PAR1 magic regardless of legacy flag (legacy={legacy})",
            );
        }
    }

    #[test]
    fn auto_detect_source_compression_recognizes_orc_magic_regardless_of_flag() {
        for legacy in [false, true] {
            let result = auto_detect_source_compression("noext", b"ORCpayload", legacy);
            assert_eq!(
                result.unwrap(),
                CompressionType::Orc,
                "must recognize ORC magic regardless of legacy flag (legacy={legacy})",
            );
        }
    }

    // Partial-prefix detection: `\x1F\x8B` is the first 2 bytes of gzip's
    // 3-byte magic. With the legacy flag false (Python/JDBC default)
    // `infer` requires the full 3 bytes and returns `None` here. With the
    // legacy flag true (ODBC default), the short-prefix table matches
    // first and returns `Gzip`, mirroring `libsnowflakeclient`'s
    // `m_magicBytes = 2` for gzip.
    #[test]
    fn auto_detect_source_compression_legacy_flag_true_detects_2byte_gzip() {
        let two_byte_gzip: &[u8] = &[0x1F, 0x8B];
        let result = auto_detect_source_compression("noext", two_byte_gzip, true);
        assert_eq!(result.unwrap(), CompressionType::Gzip);
    }

    #[test]
    fn auto_detect_source_compression_legacy_flag_false_misses_2byte_gzip() {
        let two_byte_gzip: &[u8] = &[0x1F, 0x8B];
        let result = auto_detect_source_compression("noext", two_byte_gzip, false);
        assert_eq!(result.unwrap(), CompressionType::None);
    }

    #[test]
    fn get_source_compression_explicit_param_ignores_flag() {
        // Explicit SOURCE_COMPRESSION=<known type> never goes through the
        // auto-detect path, so the flag branch is a no-op here.
        for legacy in [false, true] {
            assert_eq!(
                get_source_compression("ignored.xz", b"", &SourceCompressionParam::Gzip, legacy)
                    .unwrap(),
                CompressionType::Gzip,
            );
            assert_eq!(
                get_source_compression("ignored.xz", b"", &SourceCompressionParam::None, legacy)
                    .unwrap(),
                CompressionType::None,
            );
        }
    }

    // Explicit SOURCE_COMPRESSION=PARQUET / =ORC short-circuits auto-detect:
    // user-specified compression is trusted, regardless of filename or
    // magic bytes. Mirrors Python `file_transfer_agent.py:1207`
    // (`current_file_compression_type = user_specified_source_compression`).
    #[test]
    fn get_source_compression_explicit_parquet_skips_autodetect() {
        for legacy in [false, true] {
            assert_eq!(
                get_source_compression(
                    "actually-not-parquet.csv",
                    b"some-csv,content",
                    &SourceCompressionParam::Parquet,
                    legacy,
                )
                .unwrap(),
                CompressionType::Parquet,
            );
        }
    }

    #[test]
    fn get_source_compression_explicit_orc_skips_autodetect() {
        for legacy in [false, true] {
            assert_eq!(
                get_source_compression(
                    "actually-not-orc.csv",
                    b"some-csv,content",
                    &SourceCompressionParam::Orc,
                    legacy,
                )
                .unwrap(),
                CompressionType::Orc,
            );
        }
    }

    // Upload-prep passthrough: a `.parquet` source under
    // `auto_compress = true` must NOT be re-wrapped in gzip. The target
    // filename keeps its original `.parquet` suffix (no `.gz` appended)
    // and `target_compression` is reported as `Parquet`. Asserting the
    // payload is bit-identical to the input distinguishes "didn't gzip"
    // from "gzipped a tiny buffer that happens to start with PAR1".
    #[test]
    fn preprocess_parquet_passthrough_under_auto_compress() {
        let payload = b"PAR1\x00\x01\x02\x03some-parquet-bytes-go-here".to_vec();
        let data = passthrough_upload_data("data.parquet", PutGetResultsetFlavor::Python, false);

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(Bytes::from(payload.clone())), &data)
                .unwrap();

        assert_eq!(metadata.target, "data.parquet", "no .gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Parquet);
        assert_eq!(metadata.source_compression, CompressionType::Parquet);
        assert_eq!(
            prepared.source.byte_source().into_bytes().unwrap(),
            payload,
            "payload must pass through bit-identical (no gzip wrap)",
        );
    }

    #[test]
    fn preprocess_orc_passthrough_under_auto_compress() {
        let payload = b"ORC\x00\x01\x02some-orc-bytes-go-here".to_vec();
        let data = passthrough_upload_data("data.orc", PutGetResultsetFlavor::Python, false);

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(Bytes::from(payload.clone())), &data)
                .unwrap();

        assert_eq!(metadata.target, "data.orc", "no .gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Orc);
        assert_eq!(metadata.source_compression, CompressionType::Orc);
        assert_eq!(
            prepared.source.byte_source().into_bytes().unwrap(),
            payload,
            "payload must pass through bit-identical"
        );
    }

    // Upload-prep passthrough on the explicit-param path: when the user
    // sets `SOURCE_COMPRESSION=PARQUET` / `=ORC`, the file must NOT be
    // re-wrapped in gzip even with `auto_compress = true`. Parallels the
    // auto-detect passthrough tests above; the difference is that the
    // compression type is taken from the user param rather than sniffed
    // from filename or magic bytes.
    #[test]
    fn preprocess_parquet_passthrough_under_explicit_param() {
        let payload = b"PAR1\x00\x01\x02\x03some-parquet-bytes-go-here".to_vec();
        let data = SingleUploadData {
            source_compression: SourceCompressionParam::Parquet,
            ..passthrough_upload_data("data.parquet", PutGetResultsetFlavor::Python, false)
        };

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(Bytes::from(payload.clone())), &data)
                .unwrap();

        assert_eq!(metadata.target, "data.parquet", "no .gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Parquet);
        assert_eq!(metadata.source_compression, CompressionType::Parquet);
        assert_eq!(
            prepared.source.byte_source().into_bytes().unwrap(),
            payload,
            "payload must pass through bit-identical (no gzip wrap)",
        );
    }

    #[test]
    fn preprocess_orc_passthrough_under_explicit_param() {
        let payload = b"ORC\x00\x01\x02some-orc-bytes-go-here".to_vec();
        let data = SingleUploadData {
            source_compression: SourceCompressionParam::Orc,
            ..passthrough_upload_data("data.orc", PutGetResultsetFlavor::Python, false)
        };

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(Bytes::from(payload.clone())), &data)
                .unwrap();

        assert_eq!(metadata.target, "data.orc", "no .gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Orc);
        assert_eq!(metadata.source_compression, CompressionType::Orc);
        assert_eq!(
            prepared.source.byte_source().into_bytes().unwrap(),
            payload,
            "payload must pass through bit-identical"
        );
    }

    // Locks in PR2 of Gap-12: parquet/orc detection is independent of the
    // unsupported-compression flag (ODBC sets the flag to true, matching
    // legacy libsnowflakeclient which detects PAR1/ORC magic via
    // FileCompressionType::PARQUET / ::ORC with isSupported=true).
    #[test]
    fn preprocess_parquet_passthrough_when_unsupported_compression_swallowed() {
        let payload = b"PAR1\x00\x01\x02\x03more-bytes".to_vec();
        let data = passthrough_upload_data("data.parquet", PutGetResultsetFlavor::Odbc, true);

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(Bytes::from(payload.clone())), &data)
                .unwrap();

        assert_eq!(metadata.target, "data.parquet");
        assert_eq!(metadata.target_compression, CompressionType::Parquet);
        assert_eq!(prepared.source.byte_source().into_bytes().unwrap(), payload);
    }

    #[test]
    fn preprocess_orc_passthrough_when_unsupported_compression_swallowed() {
        let payload = b"ORC\x00\x01\x02more-bytes".to_vec();
        let data = passthrough_upload_data("data.orc", PutGetResultsetFlavor::Odbc, true);

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(Bytes::from(payload.clone())), &data)
                .unwrap();

        assert_eq!(metadata.target, "data.orc");
        assert_eq!(metadata.target_compression, CompressionType::Orc);
        assert_eq!(prepared.source.byte_source().into_bytes().unwrap(), payload);
    }

    // Auto-compress of a plain (not-already-compressed) payload must stream the
    // gzip output to a tempfile and adopt it as the upload source: target gains
    // a `.gz` suffix, target compression is Gzip, and the source becomes a
    // `PreparedSource::GzipTempfile` whose `_guard` keeps the tempfile alive
    // (the lazily-read source must outlive the upload). Complements the
    // end-to-end `auto_compress_then_encrypt_decrypt_decompress_roundtrip` in
    // `tests/byte_source_roundtrip.rs`.
    #[test]
    fn preprocess_auto_compress_streams_gzip_to_tempfile() {
        let payload = b"plain csv payload that is not already compressed".to_vec();
        let data = passthrough_upload_data("data.csv", PutGetResultsetFlavor::Python, false);

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(Bytes::from(payload)), &data).unwrap();

        assert_eq!(metadata.target, "data.csv.gz", ".gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Gzip);
        assert!(
            matches!(prepared.source, PreparedSource::GzipTempfile { .. }),
            "auto-compress must make the gzip tempfile (which carries its own \
             unlink guard) the upload source",
        );
    }

    // Prefix-read coverage: the 512-byte prefix window must be wide enough
    // to cover non-zero-offset magic bytes (e.g. tar's `ustar` at offset 257).
    // No `CompressionType` currently uses a non-zero offset, but the constant
    // is sized for future archive matchers. This test pins the contract: a
    // file larger than 512 bytes yields a prefix of exactly
    // COMPRESSION_DETECT_PREFIX_LEN bytes, and that prefix covers at least
    // offset 257 so a future matcher with magic there would see it.
    #[test]
    fn read_prefix_and_size_covers_non_zero_offset_up_to_512_bytes() {
        use std::io::Write;

        let file_len = 600usize;
        let mut data = vec![0u8; file_len];
        // Write a sentinel at offset 257 (tar's `ustar` position) and at
        // offset COMPRESSION_DETECT_PREFIX_LEN - 1 (last byte of the window).
        data[257] = 0xAA;
        data[COMPRESSION_DETECT_PREFIX_LEN - 1] = 0xBB;
        // Byte just outside the window must NOT appear in the prefix.
        data[COMPRESSION_DETECT_PREFIX_LEN] = 0xCC;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.bin");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(&data)
            .unwrap();

        let (prefix, size) =
            read_prefix_and_size(&ByteSource::Path(path)).expect("read_prefix_and_size");

        assert_eq!(size, file_len as i64);
        assert_eq!(
            prefix.len(),
            COMPRESSION_DETECT_PREFIX_LEN,
            "prefix must be exactly COMPRESSION_DETECT_PREFIX_LEN bytes"
        );
        assert_eq!(
            prefix[257], 0xAA,
            "prefix must cover offset 257 (tar ustar position)"
        );
        assert_eq!(
            prefix[COMPRESSION_DETECT_PREFIX_LEN - 1],
            0xBB,
            "prefix must include the last byte of the window"
        );
        assert!(
            !prefix.contains(&0xCC),
            "prefix must not contain bytes beyond the window"
        );
    }

    #[test]
    fn read_prefix_and_size_bytes_source_truncates_to_window() {
        // ByteSource::Bytes also caps the prefix at COMPRESSION_DETECT_PREFIX_LEN.
        let data: Vec<u8> = (0..600u16).map(|i| (i % 251) as u8).collect();
        let (prefix, size) = read_prefix_and_size(&ByteSource::Bytes(Bytes::from(data.clone())))
            .expect("read_prefix_and_size for Bytes");

        assert_eq!(size, 600);
        assert_eq!(prefix.len(), COMPRESSION_DETECT_PREFIX_LEN);
        assert_eq!(&prefix[..], &data[..COMPRESSION_DETECT_PREFIX_LEN]);
    }

    // Determinism pin for `auto_compress = true`. The post-compression
    // SHA-256 digest is the value Snowflake stores as the remote
    // `x-ms-meta-sfcdigest` header (Azure) and the equivalent on GCS;
    // the skip-on-content-match optimization across UD and the legacy
    // Python connector compares this digest. If the gzip output is not
    // byte-stable across calls with identical input, the digest changes
    // every upload and the optimization silently never fires on the
    // default (auto_compress) path. This test pins both bytes and digest.
    #[test]
    fn preprocess_auto_compress_is_byte_deterministic_across_calls() {
        let payload = b"some payload that will be gzipped in preprocess".to_vec();
        let data = passthrough_upload_data("data.csv", PutGetResultsetFlavor::Python, false);

        let (a, meta_a) =
            preprocess_file_before_upload(ByteSource::Bytes(payload.clone().into()), &data)
                .unwrap();
        let (b, meta_b) =
            preprocess_file_before_upload(ByteSource::Bytes(payload.clone().into()), &data)
                .unwrap();

        assert_eq!(
            meta_a.target, "data.csv.gz",
            "auto_compress should produce a .gz target"
        );
        assert_eq!(meta_a.target_compression, CompressionType::Gzip);
        assert_eq!(meta_a.target, meta_b.target);
        assert_eq!(meta_a.target_compression, meta_b.target_compression);

        let bytes_a = a.source.byte_source().into_bytes().unwrap();
        let bytes_b = b.source.byte_source().into_bytes().unwrap();
        assert_eq!(
            bytes_a, bytes_b,
            "gzip output must be byte-identical across calls with the same input"
        );
        assert_eq!(
            a.digest, b.digest,
            "post-compression digest must be stable; otherwise content-match skip never fires"
        );
        assert_ne!(
            bytes_a, payload,
            "sanity: compressed bytes should differ from the raw payload (this test would be \
             vacuous on a passthrough path)"
        );
    }

    fn passthrough_upload_data(
        filename: &str,
        flavor: PutGetResultsetFlavor,
        legacy_odbc_compression_autodetect: bool,
    ) -> SingleUploadData {
        // Tests that call preprocess_file_before_upload directly pass a
        // ByteSource::Bytes so they don't depend on the filesystem.
        SingleUploadData {
            source: ByteSource::Bytes(Bytes::new()),
            filename: filename.to_string(),
            stage_info: dummy_stage_info(),
            encryption_material: None,
            auto_compress: true,
            source_compression: SourceCompressionParam::AutoDetect,
            overwrite: false,
            flavor,
            legacy_odbc_compression_autodetect,
            skip_upload_on_content_match: false,
            multipart: MultipartParams::default(),
        }
    }

    fn dummy_stage_info() -> StageInfo {
        StageInfo {
            location_type: LocationType::S3,
            bucket: "b".to_string(),
            key_prefix: "p".to_string(),
            region: "us-east-1".to_string(),
            creds: CloudCredentials::S3 {
                aws_key_id: String::new(),
                aws_secret_key: crate::sensitive::SensitiveString::from(String::new()),
                aws_token: crate::sensitive::SensitiveString::from(String::new()),
            },
            endpoint: None,
            presigned_url: None,
            use_virtual_url: false,
            use_regional_url: false,
            use_s3_regional_url: false,
            tls_config: crate::tls::config::TlsConfig::default(),
            crl_worker: crate::crl::worker::CrlWorker::new_lazy(),
            storage_account: None,
        }
    }

    // ---------------------------------------------------------------
    // Cross-wrapper result mapping for content-match skip
    //
    // Content-match skip under `overwrite=true` is a new path that produces
    // `(UploadStatus::Skipped, flavor)`. The `upload_result_message` unit
    // tests (above) cover the static mapping, but the END-TO-END behaviour
    // — that the path actually arrives at `Skipped` and the message column
    // gets populated correctly per wrapper — wasn't pinned. A future change
    // that splits content-match into a separate UploadStatus variant could
    // silently break the ODBC contract unless caught here.
    //
    // Drives `upload_single_file` against a wiremock Azure where HEAD
    // returns a matching digest, asserts the resulting `UploadResult` per
    // wrapper flavor.
    // ---------------------------------------------------------------

    use crate::sensitive::SensitiveString;
    use std::io::Write;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn run_content_match_skip(flavor: PutGetResultsetFlavor) -> UploadResult {
        // Real on-disk file so `upload_single_file`'s `File::open` works.
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let payload = b"hello-azure-cross-wrapper";
        // Disable auto_compress so the prepared source == payload and the digest
        // computed on the file matches what the test plants in the HEAD
        // response. With auto_compress=true the upload-prep would gzip the
        // bytes and the digest would be over the gzipped form.
        std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(tmp.path())
            .unwrap()
            .write_all(payload)
            .unwrap();

        let real_digest =
            encryption::compute_sha256_digest(&ByteSource::Bytes(payload.to_vec().into()))
                .expect("digest");

        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-ms-meta-sfcdigest", real_digest.as_str()),
            )
            .mount(&mock)
            .await;
        // Load-bearing: skip must fire (no Azure block-blob PUT) for this path.
        // Path-scoped to /test-container/ so stray S3 UploadPart requests from
        // concurrent tests don't spuriously trip the expect(0) assertion.
        Mock::given(method("PUT"))
            .and(path_regex("^/test-container/"))
            .respond_with(ResponseTemplate::new(201))
            .expect(0)
            .mount(&mock)
            .await;

        let stage_info = StageInfo {
            location_type: LocationType::Azure,
            bucket: "test-container".to_string(),
            key_prefix: "prefix/".to_string(),
            region: "eastus2".to_string(),
            creds: CloudCredentials::Azure {
                sas_token: SensitiveString::from("sv=test&sig=test&se=2099-01-01"),
            },
            endpoint: Some(mock.uri()),
            presigned_url: None,
            use_virtual_url: false,
            use_regional_url: false,
            use_s3_regional_url: false,
            tls_config: crate::tls::config::TlsConfig::default(),
            crl_worker: crate::crl::worker::CrlWorker::new_lazy(),
            storage_account: Some("test".to_string()),
        };

        let data = SingleUploadData {
            source: ByteSource::Path(tmp.path().to_str().unwrap().into()),
            filename: "f.dat".to_string(),
            stage_info,
            encryption_material: None,
            auto_compress: false,
            source_compression: SourceCompressionParam::None,
            overwrite: true,
            flavor,
            legacy_odbc_compression_autodetect: false,
            skip_upload_on_content_match: true,
            multipart: MultipartParams::default(),
        };

        let mut refresher: Option<&mut dyn StageInfoRefresher> = None;
        let policy = crate::config::retry::RetryPolicy::put_get(
            &crate::config::param_store::ParamStore::new(),
        );
        upload_single_file(data, &policy, &mut refresher)
            .await
            .expect("upload_single_file should succeed against the mock")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn content_match_skip_under_odbc_emits_legacy_message() {
        let result = run_content_match_skip(PutGetResultsetFlavor::Odbc).await;
        assert_eq!(result.status, "SKIPPED");
        assert_eq!(
            result.message, ODBC_PUT_MESSAGE_SKIPPED,
            "ODBC users who set OVERWRITE=TRUE and hit content-match must get the legacy SKIPPED message",
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn content_match_skip_under_python_emits_empty_message() {
        let result = run_content_match_skip(PutGetResultsetFlavor::Python).await;
        assert_eq!(result.status, "SKIPPED");
        assert_eq!(
            result.message, "",
            "Python flavor must keep the message column empty even when content-match fires",
        );
    }

    // ---------------------------------------------------------------
    // S3 / GCS scope pin: skip_upload_on_content_match is Azure-only
    // per gap-5 findings. These tests assert current no-op behaviour.
    // If a future change wires the flag for S3 or GCS, update findings.md
    // (cross-cloud parity scope) and this test deliberately.
    // ---------------------------------------------------------------

    fn write_local_payload(content: &[u8]) -> tempfile::NamedTempFile {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(tmp.path())
            .unwrap()
            .write_all(content)
            .unwrap();
        tmp
    }

    fn single_upload_data_for(
        location_type: LocationType,
        endpoint: &str,
        file_path: &str,
    ) -> SingleUploadData {
        let creds = match location_type {
            LocationType::S3 => CloudCredentials::S3 {
                aws_key_id: "AKIA-TEST".to_string(),
                aws_secret_key: SensitiveString::from("secret".to_string()),
                aws_token: SensitiveString::from("token".to_string()),
            },
            LocationType::Gcs => CloudCredentials::Gcs {
                gcs_access_token: Some(SensitiveString::from("test-bearer-token".to_string())),
            },
            LocationType::Azure => unreachable!("Azure path covered by content_match_skip tests"),
        };
        SingleUploadData {
            source: ByteSource::Path(file_path.into()),
            filename: "f.dat".to_string(),
            stage_info: StageInfo {
                location_type,
                bucket: "test-bucket".to_string(),
                key_prefix: "prefix/".to_string(),
                region: "us-east-1".to_string(),
                creds,
                endpoint: Some(endpoint.to_string()),
                presigned_url: None,
                use_virtual_url: false,
                use_regional_url: false,
                use_s3_regional_url: false,
                tls_config: crate::tls::config::TlsConfig::default(),
                crl_worker: crate::crl::worker::CrlWorker::new_lazy(),
                storage_account: None,
            },
            encryption_material: None,
            auto_compress: false,
            source_compression: SourceCompressionParam::None,
            overwrite: true,
            flavor: PutGetResultsetFlavor::Python,
            legacy_odbc_compression_autodetect: false,
            skip_upload_on_content_match: true,
            multipart: MultipartParams::default(),
        }
    }

    /// Pin: under `overwrite=true && skip_match=true`, S3 must NOT issue a
    /// HEAD probe (it doesn't read the flag). A regression where S3 begins
    /// honoring the flag would issue HEAD for digest comparison; this test
    /// catches that drift via `Mock::given(method("HEAD")).expect(0)`.
    ///
    /// No GCS sibling test: PR #57 (SNOW-3406389) made GCS issue HEAD
    /// unconditionally (the `upload_to_gcs_or_skip` signature drops the
    /// `skip_upload_on_content_match` kwarg entirely; HEAD fires whether
    /// the flag is set or not). Cross-cloud picture is therefore: S3
    /// no-ops the kwarg (this test), Azure honors it (azure_transfer.rs
    /// tests), GCS unconditionally probes (gcs_transfer.rs tests, layer
    /// below dispatch). S3 remains the only "no-op" cloud worth pinning
    /// here; tracked as a follow-up (S3 HEAD fail-OPEN clobber +
    /// skip-match parity).
    #[tokio::test(flavor = "multi_thread")]
    async fn skip_upload_on_content_match_is_no_op_on_s3() {
        let mock = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&mock)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock)
            .await;

        let tmp = write_local_payload(b"hello-s3-no-op");
        let data =
            single_upload_data_for(LocationType::S3, &mock.uri(), tmp.path().to_str().unwrap());

        let mut refresher: Option<&mut dyn StageInfoRefresher> = None;
        let policy = crate::config::retry::RetryPolicy::put_get(
            &crate::config::param_store::ParamStore::new(),
        );
        let result = upload_single_file(data, &policy, &mut refresher)
            .await
            .expect("S3 upload should succeed against the mock");
        assert_eq!(result.status, "UPLOADED");
    }
}
