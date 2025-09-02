use super::types::{
    EncryptedFileMetadata, EncryptionMaterial, EncryptionResult, MaterialDescription,
};
use snafu::{Location, ResultExt, Snafu};

use base64::{Engine, engine::general_purpose::STANDARD as BASE64_ENGINE};
use openssl::{
    error::ErrorStack as OpenSslErrorStack,
    hash::{MessageDigest, hash},
    rand::rand_bytes,
    symm::{Cipher, decrypt, encrypt},
};

// Cryptographic constants
const AES_256_KEY_SIZE_IN_BYTES: usize = 32; // 256 bits
const AES_128_KEY_SIZE_IN_BYTES: usize = 16; // 128 bits
const AES_BLOCK_SIZE_IN_BYTES: usize = 16; // 128-bit block size for AES

/// A container for the ciphers and key length determined by the master key.
struct CipherSuite {
    key_len: usize,
    cbc: Cipher,
    ecb: Cipher,
}

impl CipherSuite {
    fn from_key_len(key_len: usize) -> Result<Self, EncryptionError> {
        match key_len {
            AES_128_KEY_SIZE_IN_BYTES => Ok(Self {
                key_len,
                cbc: Cipher::aes_128_cbc(),
                ecb: Cipher::aes_128_ecb(),
            }),
            AES_256_KEY_SIZE_IN_BYTES => Ok(Self {
                key_len,
                cbc: Cipher::aes_256_cbc(),
                ecb: Cipher::aes_256_ecb(),
            }),
            _ => UnsupportedKeySizeSnafu { key_size: key_len }.fail(),
        }
    }
}

/// Encrypts file data using AES-CBC with PKCS#7 padding.
pub fn encrypt_file_data(
    file_data: &[u8],
    encryption_material: &EncryptionMaterial,
) -> Result<EncryptionResult, EncryptionError> {
    // 1. Decode master key and select the appropriate cipher suite.
    let master_key = BASE64_ENGINE
        .decode(&encryption_material.query_stage_master_key)
        .context(Base64DecodingSnafu {
            context: "master key",
        })?;
    let cipher_suite = CipherSuite::from_key_len(master_key.len())?;

    // 2. Generate a random data encryption key (file key) and initialization vector (IV).
    let file_key = generate_random_bytes(cipher_suite.key_len).context(OpenSSLSnafu {
        operation: "generating file key",
    })?;
    let iv = generate_random_bytes(AES_BLOCK_SIZE_IN_BYTES).context(OpenSSLSnafu {
        operation: "generating initialization vector",
    })?;

    // 3. Encrypt the file data using the file key and IV with AES-CBC.
    let encrypted_data =
        encrypt(cipher_suite.cbc, &file_key, Some(&iv), file_data).context(OpenSSLSnafu {
            operation: "encrypting file data with AES-CBC",
        })?;

    // 4. Encrypt the file key using the master key with AES-ECB.
    let encrypted_file_key =
        encrypt(cipher_suite.ecb, &master_key, None, &file_key).context(OpenSSLSnafu {
            operation: "encrypting file key with AES-ECB",
        })?;

    // 5. Prepare the metadata for the encrypted file.
    let material_desc = MaterialDescription {
        query_id: encryption_material.query_id.clone(),
        smk_id: encryption_material.smk_id.clone(),
        key_size: (cipher_suite.key_len * 8).to_string(),
    };

    let metadata = EncryptedFileMetadata {
        encrypted_key: BASE64_ENGINE.encode(&encrypted_file_key),
        iv: BASE64_ENGINE.encode(&iv),
        material_desc,
        digest: calculate_digest(&encrypted_data).context(OpenSSLSnafu {
            operation: "calculating SHA-256 digest",
        })?,
    };

    Ok(EncryptionResult {
        data: encrypted_data,
        metadata,
    })
}

/// Decrypts file data using AES-CBC with PKCS#7 padding.
pub fn decrypt_file_data(
    encrypted_data: &[u8],
    metadata: &EncryptedFileMetadata,
    encryption_material: &EncryptionMaterial,
) -> Result<Vec<u8>, EncryptionError> {
    // 1. Decode master key and select the appropriate cipher suite.
    let master_key = BASE64_ENGINE
        .decode(&encryption_material.query_stage_master_key)
        .context(Base64DecodingSnafu {
            context: "master key",
        })?;
    let cipher_suite = CipherSuite::from_key_len(master_key.len())?;

    // 2. Decode the encrypted file key and IV from metadata.
    let encrypted_file_key =
        BASE64_ENGINE
            .decode(&metadata.encrypted_key)
            .context(Base64DecodingSnafu {
                context: "encrypted file key",
            })?;
    let iv = BASE64_ENGINE
        .decode(&metadata.iv)
        .context(Base64DecodingSnafu {
            context: "initialization vector",
        })?;

    // 3. Verify the digest of encrypted data.
    let calculated_digest = calculate_digest(encrypted_data).context(OpenSSLSnafu {
        operation: "calculating SHA-256 digest for verification",
    })?;
    if calculated_digest != metadata.digest {
        return DigestMismatchSnafu.fail();
    }

    // 4. Decrypt the file key using the master key with AES-ECB.
    let file_key = decrypt(cipher_suite.ecb, &master_key, None, &encrypted_file_key).context(
        OpenSSLSnafu {
            operation: "decrypting file key with AES-ECB",
        },
    )?;

    // 5. Decrypt the file data using the file key and IV with AES-CBC.
    let decrypted_data =
        decrypt(cipher_suite.cbc, &file_key, Some(&iv), encrypted_data).context(OpenSSLSnafu {
            operation: "decrypting file data with AES-CBC",
        })?;

    Ok(decrypted_data)
}

/// Generates a vector of random bytes of a specified size.
fn generate_random_bytes(size: usize) -> Result<Vec<u8>, OpenSslErrorStack> {
    let mut buffer = vec![0; size];
    rand_bytes(&mut buffer)?;
    Ok(buffer)
}

/// Computes the SHA-256 digest of the data and returns it as a Base64 string.
fn calculate_digest(data: &[u8]) -> Result<String, OpenSslErrorStack> {
    let digest = hash(MessageDigest::sha256(), data)?;
    Ok(BASE64_ENGINE.encode(digest))
}

#[derive(Snafu, Debug)]
pub enum EncryptionError {
    #[snafu(display("OpenSSL cryptographic operation failed during {operation}"))]
    OpenSSL {
        operation: String,
        source: OpenSslErrorStack,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to decode Base64 encoded data: {context}"))]
    Base64Decoding {
        context: String,
        source: base64::DecodeError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Unsupported encryption key size: {key_size} bytes"))]
    UnsupportedKeySize {
        key_size: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Data integrity check failed: digest mismatch"))]
    DigestMismatch {
        #[snafu(implicit)]
        location: Location,
    },
}
