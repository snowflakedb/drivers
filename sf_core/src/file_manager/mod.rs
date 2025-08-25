mod encryption;
mod file_transfer;

mod path_expansion;
pub mod types;

pub use self::types::*;

use crate::compression::{CompressionError, compress_data};
use crate::compression_types::{CompressionType, CompressionTypeError, try_guess_compression_type};
use encryption::{EncryptionError, decrypt_file_data, encrypt_file_data};
use file_transfer::{DownloadFileError, UploadFileError, download_from_s3, upload_to_s3_or_skip};
use path_expansion::{PathExpansionError, expand_filenames};
use snafu::{Location, ResultExt, Snafu};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub async fn upload_files(data: &UploadData) -> Result<Vec<UploadResult>, FileManagerError> {
    let file_locations =
        expand_filenames(&data.src_location_pattern).context(PathExpansionSnafu)?;
    let mut results = Vec::new();

    for file_location in file_locations {
        // TODO: We could experiment with references here for performance after we have working parallel implementation

        let single_upload_data = SingleUploadData {
            file_path: file_location.path,
            filename: file_location.filename,
            stage_info: data.stage_info.clone(),
            encryption_material: data.encryption_material.clone(),
            auto_compress: data.auto_compress,
            source_compression: data.source_compression.clone(),
            overwrite: data.overwrite,
        };

        let result = upload_single_file(single_upload_data).await?;
        results.push(result);
    }

    Ok(results)
}

pub async fn upload_single_file(data: SingleUploadData) -> Result<UploadResult, FileManagerError> {
    let mut input_file = File::open(&data.file_path).context(IoSnafu)?;

    let mut file_buffer = Vec::new();
    input_file.read_to_end(&mut file_buffer).context(IoSnafu)?;

    let (encryption_result, file_metadata) = preprocess_file_before_upload(file_buffer, &data)?;

    let status = upload_to_s3_or_skip(
        encryption_result,
        &data.stage_info,
        file_metadata.target.as_str(),
        data.overwrite,
    )
    .await
    .context(S3UploadSnafu)?;

    // TODO: Right now empty message is hardcoded, because any error in the upload process will
    // result in an error before this point and an ERROR status is never returned.
    // We should adjust this after we have more tests in different wrappers to ensure error handling is consistent.
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
        status,
        message: "".to_string(),
    })
}

/// Sets file metadata, compresses the file if needed, and encrypts the data before uploading it to S3.
fn preprocess_file_before_upload(
    mut file_buffer: Vec<u8>,
    data: &SingleUploadData,
) -> Result<(EncryptionResult, UploadMetadata), FileManagerError> {
    let source_size = file_buffer.len() as i64;

    let source_compression = get_source_compression(
        data.filename.as_str(),
        file_buffer.as_slice(),
        &data.source_compression,
    )
    .context(CompressionTypeSnafu)?;

    let source = data.filename.clone();
    let mut target = data.filename.clone();

    // Compress the data if needed
    let target_compression = if data.auto_compress && source_compression == CompressionType::None {
        file_buffer =
            compress_data(file_buffer, data.filename.as_str()).context(CompressionSnafu)?;
        target = format!("{}.gz", data.filename);
        CompressionType::Gzip
    } else {
        source_compression.clone()
    };

    // Encrypt the data
    let encryption_result = encrypt_file_data(file_buffer.as_slice(), &data.encryption_material)
        .context(EncryptionSnafu)?;

    let target_size = encryption_result.data.len() as i64;

    Ok((
        encryption_result,
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
) -> Result<CompressionType, CompressionTypeError> {
    match source_compression {
        SourceCompressionParam::AutoDetect => try_guess_compression_type(filename, file_buffer),
        SourceCompressionParam::None => Ok(CompressionType::None),
        SourceCompressionParam::Gzip => Ok(CompressionType::Gzip),
        SourceCompressionParam::Bzip2 => Ok(CompressionType::Bzip2),
        SourceCompressionParam::Brotli => Ok(CompressionType::Brotli),
        SourceCompressionParam::Zstd => Ok(CompressionType::Zstd),
        SourceCompressionParam::Deflate => Ok(CompressionType::Deflate),
        SourceCompressionParam::RawDeflate => Ok(CompressionType::RawDeflate),
    }
}

pub async fn download_files(
    mut data: DownloadData,
) -> Result<Vec<DownloadResult>, FileManagerError> {
    let mut results = Vec::new();

    for (file_location, encryption_material) in data
        .src_locations
        .drain(..)
        .zip(data.encryption_materials.drain(..))
    {
        let single_download_data = SingleDownloadData {
            src_location: file_location,
            local_location: data.local_location.clone(),
            stage_info: data.stage_info.clone(),
            encryption_material,
        };

        let result = download_single_file(single_download_data).await?;
        results.push(result);
    }

    Ok(results)
}

pub async fn download_single_file(
    data: SingleDownloadData,
) -> Result<DownloadResult, FileManagerError> {
    // Download encrypted data and metadata from S3
    let (encrypted_data, file_metadata) =
        download_from_s3(&data.stage_info, data.src_location.as_str())
            .await
            .context(S3DownloadSnafu)?;

    // Decrypt the data (this gives us the compressed data)
    let compressed_data =
        decrypt_file_data(&encrypted_data, &file_metadata, &data.encryption_material)
            .context(DecryptionSnafu)?;

    // Create the full output path: local_location/src_location
    let output_path = Path::new(&data.local_location).join(&data.src_location);

    // Save the compressed data to the constructed path
    let mut output_file = File::create(&output_path).context(IoSnafu)?;
    output_file.write_all(&compressed_data).context(IoSnafu)?;

    tracing::info!(
        "File successfully downloaded and decrypted, saved to '{}' ({} bytes)",
        output_path.display(),
        compressed_data.len()
    );

    // TODO: Right now "DOWNLOADED" is hardcoded, because any error in the download process will result in an error before this point.
    // We should adjust this after we have more tests in different wrappers to ensure error handling is consistent.
    Ok(DownloadResult {
        file: data.src_location,
        size: compressed_data.len() as i64,
        status: "DOWNLOADED".to_string(),
        message: "".to_string(),
    })
}

// Error types for file manager operations
#[derive(Snafu, Debug)]
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
}
