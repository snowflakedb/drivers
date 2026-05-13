mod azure_transfer;
mod encryption;
mod gcs_transfer;
mod s3_transfer;

mod path_expansion;
pub mod types;

pub use self::types::*;
pub use azure_transfer::download_from_azure;
pub use gcs_transfer::download_from_gcs;

use crate::apis::database_driver_v1::PutGetResultsetFlavor;
use crate::compression::{CompressionError, compress_data};
use crate::compression_types::{CompressionType, CompressionTypeError, try_guess_compression_type};
use azure_transfer::{AzureDownloadError, AzureUploadError, upload_to_azure_or_skip};
use encryption::{EncryptionError, compute_sha256_digest, decrypt_file_data, encrypt_file_data};
use gcs_transfer::{GcsDownloadError, GcsUploadError, upload_to_gcs_or_skip};
use openssl::error::ErrorStack as OpenSslErrorStack;
use path_expansion::{PathExpansionError, expand_filenames};
use s3_transfer::{DownloadFileError, UploadFileError, download_from_s3, upload_to_s3_or_skip};
use snafu::{Location, OptionExt, ResultExt, Snafu};
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
        let single_upload_data = SingleUploadData {
            file_path: file_location.path,
            filename: file_location.filename,
            stage_info,
            encryption_material: data.encryption_material.clone(),
            auto_compress: data.auto_compress,
            source_compression: data.source_compression.clone(),
            overwrite: data.overwrite,
            flavor: data.flavor.clone(),
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
    let mut input_file = File::open(&data.file_path).context(IoSnafu)?;

    let mut file_buffer = Vec::new();
    input_file.read_to_end(&mut file_buffer).context(IoSnafu)?;

    let (prepared, file_metadata) = preprocess_file_before_upload(file_buffer, &data)?;

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
fn upload_result_source<'a>(
    file_path: &'a str,
    filename: &'a str,
    flavor: &PutGetResultsetFlavor,
    is_windows: bool,
) -> &'a str {
    match (is_windows, flavor) {
        (true, PutGetResultsetFlavor::Odbc) => file_path,
        _ => filename,
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
        &data.flavor,
    )
    .context(CompressionTypeSnafu)?;

    let source = upload_result_source(
        data.file_path.as_str(),
        data.filename.as_str(),
        &data.flavor,
        cfg!(windows),
    )
    .to_string();
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
    flavor: &PutGetResultsetFlavor,
) -> Result<CompressionType, CompressionTypeError> {
    match source_compression {
        SourceCompressionParam::AutoDetect => {
            auto_detect_source_compression(filename, file_buffer, flavor)
        }
        SourceCompressionParam::None => Ok(CompressionType::None),
        SourceCompressionParam::Gzip => Ok(CompressionType::Gzip),
        SourceCompressionParam::Bzip2 => Ok(CompressionType::Bzip2),
        SourceCompressionParam::Brotli => Ok(CompressionType::Brotli),
        SourceCompressionParam::Zstd => Ok(CompressionType::Zstd),
        SourceCompressionParam::Deflate => Ok(CompressionType::Deflate),
        SourceCompressionParam::RawDeflate => Ok(CompressionType::RawDeflate),
    }
}

/// Returns the resolved compression type for the `AUTO_DETECT` path,
/// gated on the active wrapper flavor. Legacy libsnowflakeclient silently
/// treated unsupported compression formats (e.g. `.xz`, `.lz`, `.parquet`)
/// as uncompressed and continued the upload — the `Odbc` flavor restores
/// that behavior. The `Python` flavor (default) surfaces the error,
/// matching the current UD-Python contract. The recovery is keyed on the
/// `UnsupportedCompressionType` error variant, so it fires regardless of
/// whether the detection went through the filename extension or the
/// magic-bytes (infer crate) path.
fn auto_detect_source_compression(
    filename: &str,
    file_buffer: &[u8],
    flavor: &PutGetResultsetFlavor,
) -> Result<CompressionType, CompressionTypeError> {
    let detected = try_guess_compression_type(filename, file_buffer);
    match flavor {
        PutGetResultsetFlavor::Odbc => match detected {
            Err(CompressionTypeError::UnsupportedCompressionType { .. }) => {
                Ok(CompressionType::None)
            }
            other => other,
        },
        _ => detected,
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

    let output_data = match data.encryption_material.as_ref() {
        Some(enc_material) => {
            let enc_metadata = file_metadata.context(MissingDecryptionMetadataSnafu {
                detail: "encryption metadata headers missing from downloaded file",
            })?;
            let d = digest.as_deref().context(MissingDecryptionMetadataSnafu {
                detail: "digest header missing from downloaded file",
            })?;
            decrypt_file_data(&raw_data, &enc_metadata, d, enc_material).context(DecryptionSnafu)?
        }
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
    // verbatim under `Odbc` on Windows (replicating the legacy
    // libsnowflakeclient `PATH_SEP=='/'` bug) and the basename
    // everywhere else (matching the historical UD-Python behaviour).
    const WINDOWS_FULL_PATH: &str = r"C:\Users\test\test_data.csv";
    const WINDOWS_FULL_PATH_FORWARD_SLASH: &str = "C:/Users/test/test_data.csv";
    const UNIX_FULL_PATH: &str = "/home/test/test_data.csv";
    const BASENAME: &str = "test_data.csv";

    #[test]
    fn upload_result_source_windows_odbc_returns_full_path_verbatim() {
        for full_path in [WINDOWS_FULL_PATH, WINDOWS_FULL_PATH_FORWARD_SLASH] {
            assert_eq!(
                upload_result_source(full_path, BASENAME, &PutGetResultsetFlavor::Odbc, true),
                full_path,
                "Odbc on Windows must emit `{full_path}` as-is",
            );
        }
    }

    #[test]
    fn upload_result_source_windows_python_returns_basename() {
        for full_path in [WINDOWS_FULL_PATH, WINDOWS_FULL_PATH_FORWARD_SLASH] {
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
    // no compression. The `Odbc` flavor restores that behavior; the
    // `Python` flavor (default) keeps surfacing the error.
    const UNSUPPORTED_COMPRESSION_FILENAMES: &[&str] = &[
        "test.xz",
        "test.lzma",
        "test.lz",
        "test.lzo",
        "test.Z",
        "test.parquet",
        "test.orc",
    ];

    #[test]
    fn auto_detect_source_compression_odbc_falls_back_to_none_for_unsupported() {
        for filename in UNSUPPORTED_COMPRESSION_FILENAMES {
            let result =
                auto_detect_source_compression(filename, b"", &PutGetResultsetFlavor::Odbc);
            assert_eq!(
                result.unwrap(),
                CompressionType::None,
                "Odbc flavor must fall back to None for {filename}",
            );
        }
    }

    #[test]
    fn auto_detect_source_compression_python_surfaces_unsupported_error() {
        for filename in UNSUPPORTED_COMPRESSION_FILENAMES {
            let result =
                auto_detect_source_compression(filename, b"", &PutGetResultsetFlavor::Python);
            assert!(
                matches!(
                    result,
                    Err(CompressionTypeError::UnsupportedCompressionType { .. })
                ),
                "Python flavor must propagate the unsupported error for {filename}, got: {result:?}",
            );
        }
    }

    // Buffer-detection branch (infer crate): an extension-less file whose
    // magic bytes match an unsupported format must still trigger the
    // Odbc-flavor fallback. Locks in that the recovery is keyed on the
    // `UnsupportedCompressionType` error variant, not on the
    // filename-extension detection path.
    #[test]
    fn auto_detect_source_compression_odbc_falls_back_for_buffer_detected_unsupported() {
        let xz_magic = b"\xFD7zXZ\x00\x00\x01\x69\x22\xDE\x36";
        let result =
            auto_detect_source_compression("noext", xz_magic, &PutGetResultsetFlavor::Odbc);
        assert_eq!(result.unwrap(), CompressionType::None);
    }

    #[test]
    fn auto_detect_source_compression_python_surfaces_buffer_detected_unsupported() {
        let xz_magic = b"\xFD7zXZ\x00\x00\x01\x69\x22\xDE\x36";
        let result =
            auto_detect_source_compression("noext", xz_magic, &PutGetResultsetFlavor::Python);
        assert!(
            matches!(
                result,
                Err(CompressionTypeError::UnsupportedCompressionType { .. })
            ),
            "Python flavor must propagate the buffer-detected unsupported error, got: {result:?}",
        );
    }

    #[test]
    fn auto_detect_source_compression_recognizes_gzip_for_both_flavors() {
        for flavor in [PutGetResultsetFlavor::Python, PutGetResultsetFlavor::Odbc] {
            let result = auto_detect_source_compression("test.csv.gz", b"", &flavor);
            assert_eq!(
                result.unwrap(),
                CompressionType::Gzip,
                "{flavor:?} flavor must still recognize supported extensions",
            );
        }
    }

    #[test]
    fn auto_detect_source_compression_returns_none_for_uncompressed_under_both_flavors() {
        for flavor in [PutGetResultsetFlavor::Python, PutGetResultsetFlavor::Odbc] {
            let result = auto_detect_source_compression("test.csv", b"", &flavor);
            assert_eq!(
                result.unwrap(),
                CompressionType::None,
                "{flavor:?} flavor must report None for plain files",
            );
        }
    }

    #[test]
    fn get_source_compression_explicit_param_ignores_flavor() {
        // Explicit SOURCE_COMPRESSION=<known type> never goes through the
        // auto-detect path, so the flavor branch is a no-op here.
        for flavor in [PutGetResultsetFlavor::Python, PutGetResultsetFlavor::Odbc] {
            assert_eq!(
                get_source_compression("ignored.xz", b"", &SourceCompressionParam::Gzip, &flavor,)
                    .unwrap(),
                CompressionType::Gzip,
            );
            assert_eq!(
                get_source_compression("ignored.xz", b"", &SourceCompressionParam::None, &flavor,)
                    .unwrap(),
                CompressionType::None,
            );
        }
    }
}
