mod azure_transfer;
pub mod encryption;
mod gcs_transfer;
mod s3_transfer;

mod path_expansion;
pub mod types;

/// Re-exports of internal encryption helpers for integration tests and the
/// `test-utils` feature. This module is only compiled when running tests or
/// when the crate is built with `--features test-utils`.
#[cfg(any(test, feature = "test-utils"))]
pub mod internal {
    pub use super::encryption::{decrypt_ciphertext_to_writer, encrypt_file_data};
}

pub use self::types::*;
pub use azure_transfer::download_from_azure;
pub use gcs_transfer::download_from_gcs;

use crate::apis::database_driver_v1::PutGetResultsetFlavor;
use crate::compression::{CompressionError, compress_data};
use crate::compression_types::{CompressionType, CompressionTypeError, try_guess_compression_type};
use azure_transfer::{AzureDownloadError, AzureUploadError, upload_to_azure_or_skip};
use encryption::{
    EncryptionError, compute_sha256_digest, decrypt_ciphertext_to_writer, encrypt_file_data,
};
use gcs_transfer::{GcsDownloadError, GcsUploadError, upload_to_gcs_or_skip};
use openssl::error::ErrorStack as OpenSslErrorStack;
use path_expansion::{PathExpansionError, expand_filenames};
use s3_transfer::{DownloadFileError, UploadFileError, download_from_s3, upload_to_s3_or_skip};
use snafu::{Location, OptionExt, ResultExt, Snafu};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Message string emitted in the PUT result's `message` column when the
/// upload outcome is `Skipped` under `PutGetResultsetFlavor::Odbc`. Mirrors
/// `#define MESSAGE_SKIPPED "File with same name already exists. SKIPPED"`
/// from legacy libsnowflakeclient's `FileTransferExecutionResult.cpp`. The
/// `Python` flavor leaves the `message` column empty for skipped uploads,
/// matching the historical universal-driver behaviour.
const ODBC_PUT_MESSAGE_SKIPPED: &str = "File with same name already exists. SKIPPED";

pub async fn upload_files(
    data: &UploadData,
    mut refresher: Option<&mut dyn StageCredsRefresher>,
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

    // The refresher owns the latest stage credentials for the batch via its
    // shared `StageCredsCache`; per-file calls read from that cache, so
    // refreshed creds heal the remaining files automatically (matching
    // Python's shared `StorageCredential`). The refresher coalesces
    // rapid-fire refresh calls across files.
    for file_location in file_locations {
        let stage_info = current_stage_info(&data.stage_info, refresher.as_deref());
        let path = PathBuf::from(&file_location.path);
        let single_upload_data = SingleUploadData {
            source: ByteSource::Path(path),
            source_path_str: file_location.path,
            filename: file_location.filename,
            stage_info,
            encryption_material: data.encryption_material.clone(),
            auto_compress: data.auto_compress,
            source_compression: data.source_compression.clone(),
            overwrite: data.overwrite,
            flavor: data.flavor.clone(),
            legacy_odbc_compression_autodetect: data.legacy_odbc_compression_autodetect,
        };

        let result = upload_single_file(single_upload_data, &mut refresher).await?;
        results.push(result);
    }

    Ok(results)
}

/// Returns a copy of `base` with `creds` replaced by the refresher's current
/// snapshot, when a refresher is present. Without a refresher, `base` is
/// returned unchanged.
fn current_stage_info(base: &StageInfo, refresher: Option<&dyn StageCredsRefresher>) -> StageInfo {
    let mut info = base.clone();
    if let Some(r) = refresher {
        info.creds = r.cache().snapshot();
    }
    info
}

/// Uploads one file. On S3 stages, the `refresher` (if any) is used to refresh
/// STS credentials on `ExpiredToken`; see `s3_transfer::upload_to_s3_or_skip`
/// for the refresh semantics. Refreshed credentials are stored in the
/// refresher's `StageCredsCache` rather than returned here.
pub async fn upload_single_file(
    data: SingleUploadData,
    refresher: &mut Option<&mut dyn StageCredsRefresher>,
) -> Result<UploadResult, FileManagerError> {
    // Move `source` out before borrowing the rest of `data`. The partial move
    // is safe because `preprocess_file_before_upload` takes ownership of the
    // source while borrowing the metadata fields from `data`.
    let source = data.source.clone();
    let (prepared, file_metadata) = preprocess_file_before_upload(source, &data)?;

    let status = match data.stage_info.location_type {
        LocationType::S3 => upload_to_s3_or_skip(
            prepared,
            &data.stage_info,
            file_metadata.target.as_str(),
            data.overwrite,
            refresher,
        )
        .await
        .context(S3UploadSnafu)?,
        LocationType::Gcs => upload_to_gcs_or_skip(
            prepared,
            &data.stage_info,
            file_metadata.target.as_str(),
            data.overwrite,
        )
        .await
        .context(GcsUploadSnafu)?,
        LocationType::Azure => upload_to_azure_or_skip(
            prepared,
            &data.stage_info,
            file_metadata.target.as_str(),
            data.overwrite,
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
    file_path: &str,
    filename: &str,
    flavor: &PutGetResultsetFlavor,
    is_windows: bool,
) -> String {
    match (is_windows, flavor) {
        (true, PutGetResultsetFlavor::Odbc) => file_path.replace('\\', "/"),
        _ => filename.to_string(),
    }
}

/// Sets file metadata, compresses the file if needed, and optionally encrypts the data.
/// For SSE stages (no encryption material), the data is uploaded without client-side encryption.
///
/// For `ByteSource::Path`, this function opens the file twice: once to read a
/// 16-byte prefix for compression auto-detection, and once to stream the full
/// content through compression and/or encryption. This avoids buffering the
/// whole file in memory while preserving the auto-detect behavior.
fn preprocess_file_before_upload(
    source: ByteSource,
    data: &SingleUploadData,
) -> Result<(PreparedUpload, UploadMetadata), FileManagerError> {
    // Read a 16-byte prefix for compression auto-detection. For Path sources
    // we open the file briefly just for the prefix without buffering the rest.
    // Source size is also determined here for the result metadata.
    let (prefix, source_size) = read_prefix_and_size(&source)?;

    let source_compression = get_source_compression(
        data.filename.as_str(),
        &prefix,
        &data.source_compression,
        data.legacy_odbc_compression_autodetect,
    )
    .context(CompressionTypeSnafu)?;

    // `result_source` is the string value for the PUT result's `source` column.
    // It is distinct from the `source: ByteSource` data — the naming follows
    // the original `upload_result_source` helper.
    let result_source = upload_result_source(
        &data.source_path_str,
        data.filename.as_str(),
        &data.flavor,
        cfg!(windows),
    );
    let mut target = data.filename.clone();

    // Determine whether to compress and produce the final data source.
    let (upload_source, target_compression) =
        if data.auto_compress && source_compression == CompressionType::None {
            // Compress: read the full source into memory. The plaintext is
            // consumed without being kept alongside the compressed output.
            let compressed = compress_source(source, |e| {
                use snafu::IntoError as _;
                IoSnafu.into_error(e)
            })?;
            target = format!("{}.gz", data.filename);
            (ByteSource::Bytes(compressed), CompressionType::Gzip)
        } else {
            (source, source_compression.clone())
        };

    let prepared = match &data.encryption_material {
        Some(material) => encrypt_file_data(upload_source, material).context(EncryptionSnafu)?,
        None => {
            // No encryption: compute digest from the bytes.
            let bytes = upload_source.into_bytes().context(IoSnafu)?;
            let digest = compute_sha256_digest(&bytes).context(DigestComputationSnafu)?;
            PreparedUpload {
                data: ByteSource::Bytes(bytes),
                digest,
                encryption_metadata: None,
            }
        }
    };

    let target_size = prepared.data.len().unwrap_or(0) as i64;

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

/// Reads up to 16 bytes from the source for compression auto-detection, and
/// returns the source's full byte length for the upload metadata.
///
/// For `ByteSource::Path`, the file is opened, a short read is performed, and
/// the file is then closed again — the full content is never buffered here.
/// For `ByteSource::Bytes`, the prefix is sliced from the existing buffer.
fn read_prefix_and_size(source: &ByteSource) -> Result<(Vec<u8>, i64), FileManagerError> {
    match source {
        ByteSource::Path(p) => {
            let mut f = File::open(p).context(IoSnafu)?;
            let size = f.metadata().context(IoSnafu)?.len() as i64;
            let mut prefix = vec![0u8; 16];
            let n = f.read(&mut prefix).context(IoSnafu)?;
            prefix.truncate(n);
            Ok((prefix, size))
        }
        ByteSource::Bytes(b) => {
            let prefix = b[..b.len().min(16)].to_vec();
            Ok((prefix, b.len() as i64))
        }
    }
}

/// Reads the full source and compresses it with gzip (mtime=0).
///
/// Returns the compressed bytes or a `FileManagerError` that wraps the
/// underlying IO or compression failure.
fn compress_source(
    source: ByteSource,
    io_context: impl Fn(std::io::Error) -> FileManagerError,
) -> Result<Vec<u8>, FileManagerError> {
    let bytes = source.into_bytes().map_err(io_context)?;
    compress_data(bytes).context(CompressionSnafu)
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
    mut refresher: Option<&mut dyn StageCredsRefresher>,
) -> Result<Vec<DownloadResult>, FileManagerError> {
    let mut results = Vec::new();

    for (file_location, encryption_material) in data
        .src_locations
        .drain(..)
        .zip(data.encryption_materials.drain(..))
    {
        let stage_info = current_stage_info(&data.stage_info, refresher.as_deref());
        let single_download_data = SingleDownloadData {
            src_location: file_location,
            local_location: data.local_location.clone(),
            stage_info,
            encryption_material,
            flavor: data.flavor.clone(),
        };

        let result = download_single_file(single_download_data, &mut refresher).await?;
        results.push(result);
    }

    Ok(results)
}

/// Downloads one file. See `upload_single_file` for the refresh semantics.
pub async fn download_single_file(
    data: SingleDownloadData,
    refresher: &mut Option<&mut dyn StageCredsRefresher>,
) -> Result<DownloadResult, FileManagerError> {
    let DownloadResponse {
        data: raw_data,
        digest,
        file_metadata,
        cloud_byte_count,
    } = match data.stage_info.location_type {
        LocationType::S3 => {
            download_from_s3(&data.stage_info, data.src_location.as_str(), refresher)
                .await
                .context(S3DownloadSnafu)?
        }
        LocationType::Gcs => download_from_gcs(&data.stage_info, data.src_location.as_str())
            .await
            .context(GcsDownloadSnafu)?,
        LocationType::Azure => download_from_azure(&data.stage_info, data.src_location.as_str())
            .await
            .context(AzureDownloadSnafu)?,
    };

    let filename = Path::new(&data.src_location)
        .file_name()
        .unwrap_or(std::ffi::OsStr::new(&data.src_location));
    let output_path = Path::new(&data.local_location).join(filename);

    // The output byte length for the `Python` flavor's `size` column.

    let output_byte_len: i64 = match data.encryption_material.as_ref() {
        Some(enc_material) => {
            let enc_metadata = file_metadata.context(MissingDecryptionMetadataSnafu {
                detail: "encryption metadata headers missing from downloaded file",
            })?;
            let d = digest.as_deref().context(MissingDecryptionMetadataSnafu {
                detail: "digest header missing from downloaded file",
            })?;
            // Stream ciphertext through the Crypter directly into the output
            // file. The plaintext is never held as a full Vec<u8> — each
            // decrypted chunk is written immediately. The digest is verified
            // at finalize time (post-decryption — see behavioral-change note
            // in `decrypt_ciphertext_to_writer`).
            let mut output_file = File::create(&output_path).context(IoSnafu)?;
            decrypt_ciphertext_to_writer(
                raw_data.as_slice(),
                &enc_metadata,
                d,
                enc_material,
                &mut output_file,
            )
            .context(DecryptionSnafu)?
        }
        None => {
            // SSE stage: write the raw body bytes directly to the output file.
            let mut output_file = File::create(&output_path).context(IoSnafu)?;
            std::io::copy(&mut raw_data.as_slice(), &mut output_file).context(IoSnafu)?;
            raw_data.len() as i64
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
        source: OpenSslErrorStack,
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
    #[snafu(display("Missing decryption metadata: {detail}"))]
    MissingDecryptionMetadata {
        detail: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("File does not exist: {pattern}"))]
    NoFilesMatched {
        pattern: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

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
                WINDOWS_BACKSLASH_PATH,
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
                WINDOWS_MIXED_PATH,
                BASENAME,
                &PutGetResultsetFlavor::Odbc,
                true,
            ),
            WINDOWS_MIXED_PATH_NORMALISED,
        );
        // Already-normalised input must be returned unchanged.
        assert_eq!(
            upload_result_source(
                WINDOWS_FORWARD_SLASH_PATH,
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
                upload_result_source(full_path, BASENAME, &PutGetResultsetFlavor::Python, true),
                BASENAME,
                "Python on Windows must continue stripping directories from `{full_path}`",
            );
        }
    }

    #[test]
    fn upload_result_source_non_windows_returns_basename_for_both_flavors() {
        for flavor in [PutGetResultsetFlavor::Python, PutGetResultsetFlavor::Odbc] {
            assert_eq!(
                upload_result_source(UNIX_FULL_PATH, BASENAME, &flavor, false),
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
                    upload_result_source(BASENAME, BASENAME, &flavor, is_windows),
                    BASENAME,
                    "is_windows={is_windows}, flavor={flavor:?} must return {BASENAME}",
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
            preprocess_file_before_upload(ByteSource::Bytes(payload.clone()), &data).unwrap();

        assert_eq!(metadata.target, "data.parquet", "no .gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Parquet);
        assert_eq!(metadata.source_compression, CompressionType::Parquet);
        assert_eq!(
            prepared.data.into_bytes().unwrap(),
            payload,
            "payload must pass through bit-identical (no gzip wrap)",
        );
    }

    #[test]
    fn preprocess_orc_passthrough_under_auto_compress() {
        let payload = b"ORC\x00\x01\x02some-orc-bytes-go-here".to_vec();
        let data = passthrough_upload_data("data.orc", PutGetResultsetFlavor::Python, false);

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(payload.clone()), &data).unwrap();

        assert_eq!(metadata.target, "data.orc", "no .gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Orc);
        assert_eq!(metadata.source_compression, CompressionType::Orc);
        assert_eq!(
            prepared.data.into_bytes().unwrap(),
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
            preprocess_file_before_upload(ByteSource::Bytes(payload.clone()), &data).unwrap();

        assert_eq!(metadata.target, "data.parquet");
        assert_eq!(metadata.target_compression, CompressionType::Parquet);
        assert_eq!(prepared.data.into_bytes().unwrap(), payload);
    }

    #[test]
    fn preprocess_orc_passthrough_when_unsupported_compression_swallowed() {
        let payload = b"ORC\x00\x01\x02more-bytes".to_vec();
        let data = passthrough_upload_data("data.orc", PutGetResultsetFlavor::Odbc, true);

        let (prepared, metadata) =
            preprocess_file_before_upload(ByteSource::Bytes(payload.clone()), &data).unwrap();

        assert_eq!(metadata.target, "data.orc");
        assert_eq!(metadata.target_compression, CompressionType::Orc);
        assert_eq!(prepared.data.into_bytes().unwrap(), payload);
    }

    fn passthrough_upload_data(
        filename: &str,
        flavor: PutGetResultsetFlavor,
        legacy_odbc_compression_autodetect: bool,
    ) -> SingleUploadData {
        // Tests that call preprocess_file_before_upload directly pass a
        // ByteSource::Bytes so they don't depend on the filesystem.
        SingleUploadData {
            source: ByteSource::Bytes(Vec::new()),
            source_path_str: format!("/tmp/{filename}"),
            filename: filename.to_string(),
            stage_info: dummy_stage_info(),
            encryption_material: None,
            auto_compress: true,
            source_compression: SourceCompressionParam::AutoDetect,
            overwrite: false,
            flavor,
            legacy_odbc_compression_autodetect,
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
            storage_account: None,
        }
    }
}
