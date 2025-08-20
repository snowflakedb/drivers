mod encryption;
mod file_transfer;
pub mod types;

pub use self::types::*;

use crate::compression::compress_data;
use crate::compression_types::{CompressionType, CompressionTypeError, try_guess_compression_type};
use crate::rest::error::RestError;
use encryption::{decrypt_file_data, encrypt_file_data};
use file_transfer::{download_from_s3, upload_to_s3_or_skip};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub async fn upload_files(data: &UploadData) -> Result<Vec<UploadResult>, FileManagerError> {
    let file_locations = expand_filenames(&data.src_location)?;
    let mut results = Vec::new();

    for file_location in file_locations {
        tracing::info!("Expanded file location: {}", file_location);

        // TODO: We could experiment with references here for performance after we have working parallel implementation

        let single_upload_data = UploadData {
            src_location: file_location,
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

pub async fn upload_single_file(data: UploadData) -> Result<UploadResult, FileManagerError> {
    let file_path = Path::new(&data.src_location);
    let mut input_file = File::open(file_path)?;
    let filename = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| RestError::Internal("Invalid file name".to_string()))?
        .to_string();

    let mut file_buffer = Vec::new();
    input_file.read_to_end(&mut file_buffer)?;

    let (encryption_result, file_metadata) =
        preprocess_file_before_upload(&filename, file_buffer, &data)?;

    let status = upload_to_s3_or_skip(
        encryption_result,
        &data.stage_info,
        file_metadata.target.as_str(),
        data.overwrite,
    )
    .await?;

    // TODO: Right now empty message is hardcoded, because any error in the upload process will
    // result in an error before this point and an ERROR status is never returned.
    // We should adjust this after we have more tests in different wrappers to ensure error handling is consistent.
    Ok(UploadResult {
        source: file_metadata.source,
        target: file_metadata.target,
        source_size: file_metadata.source_size,
        target_size: file_metadata.target_size,
        source_compression: file_metadata.source_compression.to_string(),
        target_compression: file_metadata.target_compression.to_string(),
        status,
        message: "".to_string(),
    })
}

/// Sets file metadata, compresses the file if needed, and encrypts the data before uploading it to S3.
fn preprocess_file_before_upload(
    filename: &str,
    mut file_buffer: Vec<u8>,
    data: &UploadData,
) -> Result<(EncryptionResult, UploadMetadata), FileManagerError> {
    let source_size = file_buffer.len() as i64;

    let source_compression =
        get_source_compression(filename, file_buffer.as_slice(), &data.source_compression)?;

    let source = filename.to_string();
    let mut target = filename.to_string();

    // Compress the data if needed
    let target_compression = if data.auto_compress && source_compression == CompressionType::None {
        file_buffer = compress_data(file_buffer, filename)?;
        target = format!("{filename}.gz");
        CompressionType::Gzip
    } else {
        source_compression.clone()
    };

    // Encrypt the data
    let encryption_result = encrypt_file_data(file_buffer.as_slice(), &data.encryption_material)?;

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
    if data.src_locations.len() != data.encryption_materials.len() {
        return Err(FileManagerError::from(RestError::Internal(
            "Number of source locations must match number of encryption materials".to_string(),
        )));
    }

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
        download_from_s3(&data.stage_info, data.src_location.as_str()).await?;

    // Decrypt the data (this gives us the compressed data)
    let compressed_data =
        decrypt_file_data(&encrypted_data, &file_metadata, &data.encryption_material)?;

    // Create the full output path: local_location/src_location
    let output_path = Path::new(&data.local_location).join(&data.src_location);

    // Save the compressed data to the constructed path
    let mut output_file = File::create(&output_path)?;
    output_file.write_all(&compressed_data)?;

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

/// Expands file names using glob patterns, returning a list of valid file paths
fn expand_filenames(pattern: &str) -> Result<Vec<String>, PathError> {
    let mut expanded_files = Vec::new();
    let paths = glob::glob(pattern)?;

    for path in paths {
        match path {
            Ok(p) => {
                if p.is_file() {
                    match p.to_str() {
                        Some(path_str) => expanded_files.push(path_str.to_string()),
                        None => {
                            return Err(PathError::InvalidPath(format!(
                                "Path '{}' contains invalid UTF-8",
                                p.display()
                            )));
                        }
                    }
                } else {
                    return Err(PathError::InvalidPath(format!(
                        "Path '{}' is not a file",
                        p.display()
                    )));
                }
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(expanded_files)
}
