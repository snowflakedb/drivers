mod compression;
mod encryption;
mod file_transfer;
mod test_utils;
pub mod types;

pub use self::types::*;

use crate::rest::error::RestError;
use compression::compress_data;
use encryption::encrypt_file_data;
use file_transfer::upload_to_s3;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub async fn transfer_file(mut data: Data) -> Result<(), FileManagerError> {
    // TODO: Implement multiple files transfer

    // Validate and extract the single source file and encryption material
    let src_location = if data.src_locations.len() == 1 {
        data.src_locations.pop().unwrap()
    } else {
        return Err(FileManagerError::from(RestError::Internal(format!(
            "Expected exactly 1 source file, got {}",
            data.src_locations.len()
        ))));
    };

    let encryption_material = if data.encryption_materials.len() == 1 {
        data.encryption_materials.pop().unwrap()
    } else {
        return Err(FileManagerError::from(RestError::Internal(format!(
            "Expected exactly 1 encryption material, got {}",
            data.encryption_materials.len()
        ))));
    };

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
