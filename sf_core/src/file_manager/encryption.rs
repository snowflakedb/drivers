use super::types::{
    EncryptedFileMetadata, EncryptionError, EncryptionMaterial, EncryptionResult,
    MaterialDescription,
};
use crate::rest::error::RestError;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as base64_engine;
use openssl::hash::{MessageDigest, hash};
use openssl::rand::rand_bytes;
use openssl::symm::{Cipher, encrypt};

// Cryptographic constants
const AES_256_KEY_SIZE_IN_BYTES: usize = 32; // 256 bits ÷ 8 = 32 bytes
const AES_128_KEY_SIZE_IN_BYTES: usize = 16; // 128 bits ÷ 8 = 16 bytes
const AES_BLOCK_SIZE_IN_BYTES: usize = 16; // 128 bits ÷ 8 = 16 bytes

/// Encrypts file data using AES-CBC with PKCS#7 padding
///
/// Steps:
/// 1. Decode master key to determine its size and corresponding algorithms
/// 2. Generate random data encryption key (same size as master key)
/// 3. Generate random IV (AES_BLOCK_SIZE bytes for AES-CBC)
/// 4. Encrypt the file data using AES-CBC with PKCS#7 padding
/// 5. Encrypt the data encryption key using the master key with PKCS#7 padding
pub fn encrypt_file_data(
    file_data: &[u8],
    encryption_material: EncryptionMaterial,
) -> Result<EncryptionResult, EncryptionError> {
    // Step 1: Decode master key to determine key size and algorithms
    let master_key = base64_engine.decode(encryption_material.query_stage_master_key)?;

    let (master_key_len, cbc_cipher, ecb_cipher) = match master_key.len() {
        AES_128_KEY_SIZE_IN_BYTES => (
            AES_128_KEY_SIZE_IN_BYTES,
            Cipher::aes_128_cbc(),
            Cipher::aes_128_ecb(),
        ),
        AES_256_KEY_SIZE_IN_BYTES => (
            AES_256_KEY_SIZE_IN_BYTES,
            Cipher::aes_256_cbc(),
            Cipher::aes_256_ecb(),
        ),
        _ => {
            return Err(EncryptionError::from(RestError::InvalidSnowflakeResponse(
                format!("Unsupported master key size: {} bytes", master_key.len()),
            )));
        }
    };

    // Step 2: Generate random data encryption key (same size as master key)
    let mut file_key = vec![0u8; master_key_len];
    rand_bytes(&mut file_key)?;

    // Step 3: Generate random IV
    let mut iv = vec![0u8; AES_BLOCK_SIZE_IN_BYTES];
    rand_bytes(&mut iv)?;

    // Step 4: Encrypt the file data
    let encrypted_data = encrypt(cbc_cipher, &file_key, Some(&iv), file_data)?;

    // Step 5: Encrypt the data encryption key using the master key
    let encrypted_file_key = encrypt(ecb_cipher, &master_key, None, &file_key)?;

    let key_size_bits = master_key_len * 8;
    let material_desc = MaterialDescription {
        query_id: encryption_material.query_id,
        smk_id: encryption_material.smk_id,
        key_size: key_size_bits.to_string(),
    };

    let digest = calculate_digest(&encrypted_data)?;

    let metadata = EncryptedFileMetadata {
        encrypted_key: base64_engine.encode(&encrypted_file_key),
        iv: base64_engine.encode(&iv),
        material_desc,
        digest,
    };

    Ok(EncryptionResult {
        data: encrypted_data,
        metadata,
    })
}

fn calculate_digest(data: &[u8]) -> Result<String, EncryptionError> {
    let digest = hash(MessageDigest::sha256(), data)?;
    Ok(base64_engine.encode(digest))
}
