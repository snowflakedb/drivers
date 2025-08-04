mod compression;
mod encryption;
mod file_transfer;
mod test_utils;
pub mod types;

pub use self::types::*;

use crate::rest::error::RestError;
use compression::compress_data;
use encryption::{decrypt_file_data, encrypt_file_data};
use file_transfer::{download_from_s3, upload_to_s3};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub async fn upload_files(mut data: UploadData) -> Result<(), FileManagerError> {
    for file_location in data.src_locations {
        tracing::info!("Uploading file: {}", file_location);

        let encryption_material = data.encryption_materials.pop().ok_or_else(|| {
            RestError::Internal("No encryption material provided for upload".to_string())
        })?;

        let file_locations = expand_file_names(&file_location)?;

        for file_location in file_locations {
            tracing::info!("Expanded file location: {}", file_location);

            // TODO: We could experiment with references here for performance after we have working parallel implementation

            let single_upload_data = SingleUploadData {
                src_location: file_location,
                stage_info: data.stage_info.clone(),
                encryption_material: encryption_material.clone(),
                auto_compress: data.auto_compress,
            };

            upload_single_file(single_upload_data).await?;
        }
    }

    Ok(())
}

pub async fn upload_single_file(data: SingleUploadData) -> Result<(), FileManagerError> {
    let file_path = Path::new(&data.src_location);
    let mut input_file = File::open(file_path)?;
    let mut file_name_with_extension = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| RestError::Internal("Invalid file name".to_string()))?
        .to_string();

    let mut input_data = Vec::new();
    input_file.read_to_end(&mut input_data)?;

    // Compress the file data if automatic compression is enabled
    if data.auto_compress {
        tracing::info!("Compressing file data before upload");
        input_data = compress_data(input_data, file_name_with_extension.as_str())?;
        file_name_with_extension = format!("{file_name_with_extension}.gz");
    } else {
        tracing::info!("Skipping compression, auto_compress is disabled");
    }

    // Encrypt the compressed data using the provided encryption material
    let encryption_result = encrypt_file_data(&input_data, data.encryption_material)?;

    tracing::trace!("Encryption metadata: {:?}", encryption_result.metadata);

    upload_to_s3(
        encryption_result,
        &data.stage_info,
        file_name_with_extension.as_str(),
    )
    .await?;

    Ok(())
}

pub async fn download_file(mut data: DownloadData) -> Result<(), FileManagerError> {
    // TODO: Implement multiple files download

    // Validate and extract the single source file and encryption material
    let (file_name_with_extension, encryption_material) =
        validate_src_location_and_encryption_materials(
            &mut data.src_locations,
            &mut data.encryption_materials,
        )?;

    // Download encrypted data and metadata from S3
    let (encrypted_data, file_metadata) =
        download_from_s3(&data.stage_info, file_name_with_extension.as_str()).await?;

    // Decrypt the data (this gives us the compressed data)
    let compressed_data = decrypt_file_data(&encrypted_data, &file_metadata, &encryption_material)?;

    // Create the full output path: local_location/src_location/file_name_with_extension
    let output_path = Path::new(&data.local_location).join(&file_name_with_extension);

    // Save the compressed data to the constructed path
    let mut output_file = File::create(&output_path)?;
    output_file.write_all(&compressed_data)?;

    tracing::info!(
        "File successfully downloaded and decrypted, saved to '{}' ({} bytes)",
        output_path.display(),
        compressed_data.len()
    );

    Ok(())
}

fn validate_src_location_and_encryption_materials(
    src_locations: &mut Vec<String>,
    encryption_materials: &mut Vec<EncryptionMaterial>,
) -> Result<(String, EncryptionMaterial), FileManagerError> {
    if src_locations.len() != 1 {
        return Err(FileManagerError::from(RestError::InvalidSnowflakeResponse(
            format!(
                "Expected exactly 1 source file, got {}",
                src_locations.len()
            ),
        )));
    }

    if encryption_materials.len() != 1 {
        return Err(FileManagerError::from(RestError::InvalidSnowflakeResponse(
            format!(
                "Expected exactly 1 encryption material, got {}",
                encryption_materials.len()
            ),
        )));
    }

    Ok((
        src_locations.pop().unwrap(),
        encryption_materials.pop().unwrap(),
    ))
}

/// Expands file names using glob patterns, returning a list of valid file paths
fn expand_file_names(pattern: &str) -> Result<Vec<String>, FileManagerError> {
    let mut expanded_files = Vec::new();
    let paths = glob::glob(pattern)?;

    for path in paths {
        match path {
            Ok(p) => {
                if p.is_file() {
                    match p.to_str() {
                        Some(path_str) => expanded_files.push(path_str.to_string()),
                        None => {
                            return Err(FileManagerError::from(RestError::Internal(format!(
                                "Path '{}' contains invalid UTF-8",
                                p.display()
                            ))));
                        }
                    }
                } else {
                    return Err(FileManagerError::from(RestError::Internal(format!(
                        "Path '{}' is not a file",
                        p.display()
                    ))));
                }
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(expanded_files)
}
