mod azure_transfer;
mod encryption;
mod gcs_transfer;
mod s3_transfer;

mod path_expansion;
pub mod types;

pub use self::types::*;
pub use azure_transfer::download_from_azure;
pub use gcs_transfer::{
    GcsDownloadError, GcsUploadError, download_from_gcs, upload_to_gcs_or_skip,
};

use crate::apis::database_driver_v1::PutGetResultsetFlavor;
use crate::compression::{CompressionError, compress_data};
use crate::compression_types::{CompressionType, CompressionTypeError, try_guess_compression_type};
use crate::config::param_registry::DEFAULT_PUT_GET_MAX_ATTEMPTS;
use azure_transfer::{AzureDownloadError, AzureUploadError, upload_to_azure_or_skip};
use encryption::{EncryptionError, compute_sha256_digest, decrypt_file_data, encrypt_file_data};
use openssl::error::ErrorStack as OpenSslErrorStack;
use path_expansion::{PathExpansionError, expand_filenames};
use s3_transfer::{DownloadFileError, UploadFileError, download_from_s3, upload_to_s3_or_skip};
use snafu::{Location, ResultExt, Snafu};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Message string emitted in the PUT result's `message` column when the
/// upload outcome is `Skipped` under `PutGetResultsetFlavor::Odbc`. Mirrors
/// `#define MESSAGE_SKIPPED "File with same name already exists. SKIPPED"`
/// from legacy libsnowflakeclient's `FileTransferExecutionResult.cpp`. The
/// `Python` flavor leaves the `message` column empty for skipped uploads,
/// matching the historical universal-driver behaviour.
const ODBC_PUT_MESSAGE_SKIPPED: &str = "File with same name already exists. SKIPPED";

pub async fn upload_files(
    data: &UploadData,
    put_get_max_attempts: u32,
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
        let single_upload_data = SingleUploadData {
            file_path: file_location.path,
            filename: file_location.filename,
            stage_info,
            encryption_material: data.encryption_material.clone(),
            auto_compress: data.auto_compress,
            source_compression: data.source_compression.clone(),
            overwrite: data.overwrite,
            flavor: data.flavor.clone(),
            legacy_odbc_compression_autodetect: data.legacy_odbc_compression_autodetect,
        };

        let result =
            upload_single_file(single_upload_data, put_get_max_attempts, &mut refresher).await?;
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
    put_get_max_attempts: u32,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<UploadResult, FileManagerError> {
    let mut input_file = File::open(&data.file_path).context(IoSnafu)?;

    let mut file_buffer = Vec::new();
    input_file.read_to_end(&mut file_buffer).context(IoSnafu)?;

    upload_prepared_buffer(file_buffer, data, put_get_max_attempts, refresher).await
}

/// Uploads an in-memory byte buffer to the stage location described by
/// `data`. Skips the disk read that [`upload_single_file`] performs and
/// delegates to the shared cloud-upload path, so encryption, compression,
/// SHA-256 digesting, and the per-cloud (S3 / GCS / Azure) dispatch behave
/// identically.
///
/// `data.file_path` is consulted by `preprocess_file_before_upload` only to
/// fill the `source` column of the upload result on the legacy
/// `PutGetResultsetFlavor::Odbc + Windows` path; callers that do not surface
/// the upload result back to the user (notably the large-bindings stage
/// uploader) may set it to the same value as `data.filename`.
pub async fn upload_in_memory_file(
    buffer: Vec<u8>,
    data: SingleUploadData,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<UploadResult, FileManagerError> {
    // The in-memory path serves the bind-stage uploader, which is not driven
    // by the session-level `put_get_max_attempts` knob; it keeps the default
    // attempt count (its payloads are small and fast — see `upload_blob`).
    upload_prepared_buffer(buffer, data, DEFAULT_PUT_GET_MAX_ATTEMPTS, refresher).await
}

/// Shared core of the upload path used by both `upload_single_file` (file
/// source) and `upload_in_memory_file` (in-memory source). Splitting the
/// disk read off lets both callers reuse the same preprocess + cloud
/// dispatch with no behavior drift.
async fn upload_prepared_buffer(
    buffer: Vec<u8>,
    data: SingleUploadData,
    put_get_max_attempts: u32,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<UploadResult, FileManagerError> {
    let (prepared, file_metadata) = preprocess_file_before_upload(buffer, &data)?;

    let status = match data.stage_info.location_type {
        LocationType::S3 => upload_to_s3_or_skip(
            prepared,
            &data.stage_info,
            file_metadata.target.as_str(),
            data.overwrite,
            put_get_max_attempts,
            refresher,
        )
        .await
        .context(S3UploadSnafu)?,
        LocationType::Gcs => upload_to_gcs_or_skip(
            prepared,
            &data.stage_info,
            file_metadata.target.as_str(),
            data.overwrite,
            put_get_max_attempts,
            refresher,
        )
        .await
        .context(GcsUploadSnafu)?,
        LocationType::Azure => upload_to_azure_or_skip(
            prepared,
            &data.stage_info,
            file_metadata.target.as_str(),
            data.overwrite,
            put_get_max_attempts,
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
fn preprocess_file_before_upload(
    mut file_buffer: Vec<u8>,
    data: &SingleUploadData,
) -> Result<(PreparedUpload, UploadMetadata), FileManagerError> {
    let source_size = file_buffer.len() as i64;

    let source_compression = get_source_compression(
        data.filename.as_str(),
        file_buffer.as_slice(),
        &data.source_compression,
        data.legacy_odbc_compression_autodetect,
    )
    .context(CompressionTypeSnafu)?;

    let source = upload_result_source(
        data.file_path.as_str(),
        data.filename.as_str(),
        &data.flavor,
        cfg!(windows),
    );
    let mut target = data.filename.clone();

    let target_compression = if data.auto_compress && source_compression == CompressionType::None {
        file_buffer = compress_data(file_buffer).context(CompressionSnafu)?;
        target = format!("{}.gz", data.filename);
        CompressionType::Gzip
    } else {
        source_compression.clone()
    };

    let prepared = match &data.encryption_material {
        Some(material) => {
            encrypt_file_data(file_buffer.as_slice(), material).context(EncryptionSnafu)?
        }
        None => {
            let digest = compute_sha256_digest(&file_buffer).context(DigestComputationSnafu)?;
            PreparedUpload {
                data: file_buffer,
                digest,
                encryption_metadata: None,
            }
        }
    };

    let target_size = prepared.data.len() as i64;

    Ok((
        prepared,
        UploadMetadata {
            source,
            target,
            source_size,
            source_compression,
            target_size,
            target_compression,
        },
    ))
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
    put_get_max_attempts: u32,
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
        };

        let result = download_single_file(
            single_download_data,
            put_get_max_attempts,
            index,
            &mut refresher,
        )
        .await?;
        results.push(result);
    }

    Ok(results)
}

/// Downloads one file. See `upload_single_file` for the refresh semantics.
///
/// `per_file_index` is the file's index inside the GET batch — i.e. its
/// position in `DownloadData.presigned_urls` / `DownloadData.src_locations`.
/// The GCS branch uses it to re-pick `presigned_urls[i]` from the refresher
/// cache after a 400-triggered URL refresh. Non-GCS branches ignore it.
pub async fn download_single_file(
    data: SingleDownloadData,
    put_get_max_attempts: u32,
    per_file_index: usize,
    refresher: &mut Option<&mut dyn StageInfoRefresher>,
) -> Result<DownloadResult, FileManagerError> {
    let DownloadResponse {
        data: raw_data,
        digest,
        file_metadata,
        cloud_byte_count,
    } = match data.stage_info.location_type {
        LocationType::S3 => {
            // `data.presigned_url` is GCS-only; S3 ignores it (uses STS creds).
            download_from_s3(
                &data.stage_info,
                data.src_location.as_str(),
                put_get_max_attempts,
                refresher,
            )
            .await
            .context(S3DownloadSnafu)?
        }
        LocationType::Gcs => download_from_gcs(
            &data.stage_info,
            data.src_location.as_str(),
            data.presigned_url.as_deref(),
            put_get_max_attempts,
            per_file_index,
            refresher,
        )
        .await
        .context(GcsDownloadSnafu)?,
        LocationType::Azure => {
            // `data.presigned_url` is GCS-only; Azure ignores it (uses SAS).
            download_from_azure(
                &data.stage_info,
                data.src_location.as_str(),
                put_get_max_attempts,
            )
            .await
            .context(AzureDownloadSnafu)?
        }
    };

    let output_data = match data.encryption_material.as_ref() {
        Some(enc_material) => match (file_metadata, digest.as_deref()) {
            (Some(enc_metadata), Some(d)) => {
                decrypt_file_data(&raw_data, &enc_metadata, d, enc_material)
                    .context(DecryptionSnafu)?
            }
            // The server advertises encryption material but the object carries no
            // client-side-encryption headers (e.g. git stage objects on S3).
            // Fall through to raw bytes, matching legacy connector behaviour.
            _ => {
                tracing::debug!(
                    "encryption_material present but S3 encryption headers absent; \
                     returning raw bytes"
                );
                raw_data
            }
        },
        None => raw_data,
    };

    let filename = Path::new(&data.src_location)
        .file_name()
        .unwrap_or(std::ffi::OsStr::new(&data.src_location));
    let output_path = Path::new(&data.local_location).join(filename);

    let mut output_file = File::create(&output_path).context(IoSnafu)?;
    output_file.write_all(&output_data).context(IoSnafu)?;

    tracing::info!(
        "File downloaded to '{}' ({} bytes)",
        output_path.display(),
        output_data.len()
    );

    Ok(DownloadResult {
        file: data.src_location,
        size: download_result_size(cloud_byte_count, output_data.len() as i64, &data.flavor),
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

        let (prepared, metadata) = preprocess_file_before_upload(payload.clone(), &data).unwrap();

        assert_eq!(metadata.target, "data.parquet", "no .gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Parquet);
        assert_eq!(metadata.source_compression, CompressionType::Parquet);
        assert_eq!(
            prepared.data, payload,
            "payload must pass through bit-identical (no gzip wrap)",
        );
    }

    #[test]
    fn preprocess_orc_passthrough_under_auto_compress() {
        let payload = b"ORC\x00\x01\x02some-orc-bytes-go-here".to_vec();
        let data = passthrough_upload_data("data.orc", PutGetResultsetFlavor::Python, false);

        let (prepared, metadata) = preprocess_file_before_upload(payload.clone(), &data).unwrap();

        assert_eq!(metadata.target, "data.orc", "no .gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Orc);
        assert_eq!(metadata.source_compression, CompressionType::Orc);
        assert_eq!(
            prepared.data, payload,
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

        let (prepared, metadata) = preprocess_file_before_upload(payload.clone(), &data).unwrap();

        assert_eq!(metadata.target, "data.parquet", "no .gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Parquet);
        assert_eq!(metadata.source_compression, CompressionType::Parquet);
        assert_eq!(
            prepared.data, payload,
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

        let (prepared, metadata) = preprocess_file_before_upload(payload.clone(), &data).unwrap();

        assert_eq!(metadata.target, "data.orc", "no .gz suffix expected");
        assert_eq!(metadata.target_compression, CompressionType::Orc);
        assert_eq!(metadata.source_compression, CompressionType::Orc);
        assert_eq!(
            prepared.data, payload,
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

        let (prepared, metadata) = preprocess_file_before_upload(payload.clone(), &data).unwrap();

        assert_eq!(metadata.target, "data.parquet");
        assert_eq!(metadata.target_compression, CompressionType::Parquet);
        assert_eq!(prepared.data, payload);
    }

    #[test]
    fn preprocess_orc_passthrough_when_unsupported_compression_swallowed() {
        let payload = b"ORC\x00\x01\x02more-bytes".to_vec();
        let data = passthrough_upload_data("data.orc", PutGetResultsetFlavor::Odbc, true);

        let (prepared, metadata) = preprocess_file_before_upload(payload.clone(), &data).unwrap();

        assert_eq!(metadata.target, "data.orc");
        assert_eq!(metadata.target_compression, CompressionType::Orc);
        assert_eq!(prepared.data, payload);
    }

    fn passthrough_upload_data(
        filename: &str,
        flavor: PutGetResultsetFlavor,
        legacy_odbc_compression_autodetect: bool,
    ) -> SingleUploadData {
        SingleUploadData {
            file_path: format!("/tmp/{filename}"),
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
