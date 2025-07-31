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

pub async fn upload_file(mut data: UploadData) -> Result<(), FileManagerError> {
    // TODO: Implement multiple files upload

    // Validate and extract the single source file and encryption material
    let (src_location, encryption_material) = validate_src_location_and_encryption_materials(
        &mut data.src_locations,
        &mut data.encryption_materials,
    )?;

    let file_path = Path::new(&src_location);
    let mut input_file = File::open(file_path)?;
    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| RestError::Internal("Invalid file name".to_string()))?;

    let mut input_data = Vec::new();
    input_file.read_to_end(&mut input_data)?;

    // Read and compress the file data
    let compressed_data = compress_data(input_data, file_name)?;

    // Encrypt the compressed data using the provided encryption material
    let encryption_result = encrypt_file_data(&compressed_data, encryption_material)?;

    tracing::trace!("Encryption metadata: {:?}", encryption_result.metadata);

    upload_to_s3(encryption_result, &data.stage_info, file_name).await?;

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
