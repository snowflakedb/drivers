use crate::rest::error::RestError;
use base64::{Engine, engine::general_purpose};
use openssl::rand::rand_bytes;
use openssl::symm::{Cipher, encrypt};

// Cryptographic constants
const AES_256_KEY_SIZE: usize = 32; // 256 bits ÷ 8 = 32 bytes
const AES_128_KEY_SIZE: usize = 16; // 128 bits ÷ 8 = 16 bytes
const AES_BLOCK_SIZE: usize = 16; // 128 bits ÷ 8 = 16 bytes (for IV and padding)

// Encryption material structure for the encryption module (no JSON parsing)
#[derive(Debug, Clone)]
pub struct EncryptionMaterial {
    pub query_stage_master_key: String,
    pub query_id: String,
    pub smk_id: i64,
}

// Encrypted file metadata that gets bundled with the encrypted data
#[derive(Debug, Clone)]
pub struct EncryptedFileMetadata {
    pub encryption_material: EncryptionMaterial,
    pub encrypted_key: String, // Base64 encoded
    pub iv: String,            // Base64 encoded
    pub mat_desc: String,
}

// Result of encryption containing encrypted data and metadata
#[derive(Debug)]
pub struct EncryptionResult {
    pub encrypted_data: Vec<u8>,
    pub metadata: EncryptedFileMetadata,
}

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
    encryption_material: &EncryptionMaterial,
) -> Result<EncryptionResult, RestError> {
    // Step 1: Decode master key to determine key size and algorithms
    let master_key = general_purpose::STANDARD
        .decode(&encryption_material.query_stage_master_key)
        .map_err(|e| RestError::Internal(format!("Failed to decode master key: {e}")))?;

    let key_size = master_key.len();
    if key_size != AES_128_KEY_SIZE && key_size != AES_256_KEY_SIZE {
        return Err(RestError::Internal(format!(
            "Unsupported master key size: {} bytes. Expected {} or {} bytes",
            key_size, AES_128_KEY_SIZE, AES_256_KEY_SIZE
        )));
    }

    // Step 2: Generate random data encryption key (same size as master key)
    let mut file_key = vec![0u8; key_size];
    rand_bytes(&mut file_key)
        .map_err(|e| RestError::Internal(format!("Failed to generate file encryption key: {e}")))?;

    // Step 3: Generate random IV
    let mut iv = vec![0u8; AES_BLOCK_SIZE];
    rand_bytes(&mut iv).map_err(|e| RestError::Internal(format!("Failed to generate IV: {e}")))?;

    // Step 4: Encrypt the file data
    let encrypted_data = encrypt_aes_cbc_pkcs7(file_data, &file_key, &iv, key_size)?;

    // Step 5: Encrypt the data encryption key using the master key
    let encrypted_file_key = encrypt_key_with_master_key(&file_key, &master_key)?;

    let key_size_bits = key_size * 8;
    let mat_desc = format!(
        "{{\"queryId\":\"{}\",\"smkId\":\"{}\",\"keySize\":\"{}\"}}",
        encryption_material.query_id, encryption_material.smk_id, key_size_bits
    );

    let metadata = EncryptedFileMetadata {
        encryption_material: encryption_material.clone(),
        encrypted_key: general_purpose::STANDARD.encode(&encrypted_file_key),
        iv: general_purpose::STANDARD.encode(&iv),
        mat_desc,
    };

    Ok(EncryptionResult {
        encrypted_data,
        metadata,
    })
}

/// Encrypts data using AES-CBC with PKCS#7 padding using OpenSSL
/// Key size determines whether to use AES-128-CBC or AES-256-CBC
fn encrypt_aes_cbc_pkcs7(
    data: &[u8],
    key: &[u8],
    iv: &[u8],
    key_size: usize,
) -> Result<Vec<u8>, RestError> {
    if iv.len() != AES_BLOCK_SIZE {
        return Err(RestError::Internal(format!(
            "IV must be exactly {AES_BLOCK_SIZE} bytes, got {}",
            iv.len()
        )));
    }

    // Choose cipher based on key size
    let cipher = match key_size {
        AES_128_KEY_SIZE => Cipher::aes_128_cbc(),
        AES_256_KEY_SIZE => Cipher::aes_256_cbc(),
        _ => {
            return Err(RestError::Internal(format!(
                "Unsupported key size: {} bytes. Expected 16 or 32 bytes",
                key_size
            )));
        }
    };

    // Encrypt the data
    // PKCS#7 padding is applied by default
    let encrypted_data = encrypt(cipher, key, Some(iv), data)
        .map_err(|e| RestError::Internal(format!("Failed to encrypt data with AES-CBC: {e}")))?;

    Ok(encrypted_data)
}

/// Encrypts the file key using the master key with ECB mode
/// Uses AES-128-ECB or AES-256-ECB depending on master key size
fn encrypt_key_with_master_key(file_key: &[u8], master_key: &[u8]) -> Result<Vec<u8>, RestError> {
    let key_size = master_key.len();

    // Choose cipher based on master key size
    let cipher = match key_size {
        AES_128_KEY_SIZE => Cipher::aes_128_ecb(),
        AES_256_KEY_SIZE => Cipher::aes_256_ecb(),
        _ => {
            return Err(RestError::Internal(format!(
                "Unsupported master key size: {} bytes. Expected 16 or 32 bytes",
                key_size
            )));
        }
    };

    // Encrypt the padded file key using ECB mode (no IV needed)
    // PKCS#7 padding is applied by default
    let encrypted_key = encrypt(cipher, master_key, None, &file_key)
        .map_err(|e| RestError::Internal(format!("Failed to encrypt file key: {e}")))?;

    Ok(encrypted_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function for hex conversion
    fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
            .collect()
    }

    // Two following tests are based on working examples from the Python connector
    #[test]
    fn test_encrypt_aes_cbc_with_known_vectors() {
        // Test values
        let iv_hex = "0123456789abcdef0123456789abcdef";
        let key_hex = "000102030405060708090a0b0c0d0e0f";

        // Compressed and normalized file
        let content_hex = "1f8b08080000000002ff2020202020202020202020202020202020202020200033d431d231e602002eb41e0506000000";

        // Expected encrypted output
        let expected_hex = "1d5b84dffa21340044d35631c7c6e5821aa104005658156c4c80f76f6d439472278102ad55a1ecf488ada0125fa2fe3fdc9311273d7d64aba19a23d8bcfaa3be";

        let iv = hex_to_bytes(iv_hex).expect("Invalid IV hex");
        let key = hex_to_bytes(key_hex).expect("Invalid key hex");
        let content = hex_to_bytes(content_hex).expect("Invalid content hex");
        let expected = hex_to_bytes(expected_hex).expect("Invalid expected hex");

        // Encrypt the content
        let result = encrypt_aes_cbc_pkcs7(
            &content,
            &key,
            &iv,
            key.len(), // AES-128 since key is 16 bytes
        )
        .expect("Encryption should succeed");

        assert_eq!(
            result, expected,
            "Encrypted output doesn't match expected result."
        );

        assert_eq!(
            result.len() % 16,
            0,
            "Encrypted output should be a multiple of AES block size (16 bytes)"
        );
    }

    #[test]
    fn test_encrypt_key_with_master_key() {
        // Test values
        let file_key_hex = "000102030405060708090a0b0c0d0e0f";
        let master_key_hex = "5cbdafea1cd0c6c84a084ac26fabcd3b";
        let expected_encrypted_base64 = "r3D64+C5K7tHzVWdhUt9Ui7xmAauO3KYvpOjzPrGkjc=";

        let file_key = hex_to_bytes(file_key_hex).expect("Invalid file key hex");
        let master_key = hex_to_bytes(master_key_hex).expect("Invalid master key hex");

        // Encrypt the file key with the master key
        let result =
            encrypt_key_with_master_key(&file_key, &master_key).expect("Encryption should succeed");

        let result_base64 = general_purpose::STANDARD.encode(&result);

        assert_eq!(
            result_base64, expected_encrypted_base64,
            "Encrypted output doesn't match expected result"
        );

        assert_eq!(
            result.len() % AES_BLOCK_SIZE,
            0,
            "Encrypted output should be a multiple of AES block size (16 bytes)"
        );
    }
}
