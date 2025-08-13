mod encryption;
mod file_transfer;
pub mod types;

pub use self::types::*;

use crate::compression::compress_data;
use crate::rest::error::RestError;
use encryption::{decrypt_file_data, encrypt_file_data};
use file_transfer::{download_from_s3, upload_to_s3};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub async fn upload_files(data: UploadData) -> Result<Vec<UploadResult>, FileManagerError> {
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
        };

        let result = upload_single_file(single_upload_data).await?;
        results.push(result);
    }

    Ok(results)
}

pub async fn upload_single_file(data: UploadData) -> Result<UploadResult, FileManagerError> {
    let file_path = Path::new(&data.src_location);
    let mut input_file = File::open(file_path)?;
    let mut filename = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| RestError::Internal("Invalid file name".to_string()))?
        .to_string();

    let mut input_data = Vec::new();
    input_file.read_to_end(&mut input_data)?;
    let source_size = input_data.len() as i64;

    // TODO: Determine if source file is compressed and set this accordingly
    let source_compression = "NONE".to_string();

    let mut target_compression = "NONE".to_string();
    let original_filename = filename.clone();

    // Compress the file data if automatic compression is enabled
    if data.auto_compress {
        tracing::info!("Compressing file data before upload");
        input_data = compress_data(input_data, filename.as_str())?;
        target_compression = "GZIP".to_string();
        filename = format!("{filename}.gz");
    } else {
        tracing::info!("Skipping compression, auto_compress is disabled");
    }

    // Encrypt the compressed data using the provided encryption material
    let encryption_result = encrypt_file_data(&input_data, data.encryption_material)?;

    let target_size = encryption_result.data.len() as i64;

    tracing::trace!("Encryption metadata: {:?}", encryption_result.metadata);

    upload_to_s3(encryption_result, &data.stage_info, filename.as_str()).await?;

    // TODO: Right now "UPLOADED" is hardcoded, because any error in the upload process will result in an error before this point.
    // We should adjust this after we have more tests in different wrappers to ensure error handling is consistent.
    Ok(UploadResult {
        source: original_filename,
        target: filename,
        source_size,
        target_size,
        source_compression,
        target_compression,
        status: "UPLOADED".to_string(),
        message: "".to_string(),
    })
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
