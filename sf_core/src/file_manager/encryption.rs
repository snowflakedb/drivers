use super::types::{
    ByteSource, EncryptedFileMetadata, EncryptionMaterial, MaterialDescription, PreparedUpload,
};
use snafu::{Location, ResultExt, Snafu};

use base64::{Engine, engine::general_purpose::STANDARD as BASE64_ENGINE};
use openssl::{
    error::ErrorStack as OpenSslErrorStack,
    hash::{Hasher, MessageDigest, hash},
    rand::rand_bytes,
    symm::{Cipher, Crypter, Mode, decrypt, encrypt},
};
use std::io::{Read, Write};

// Cryptographic constants
const AES_256_KEY_SIZE_IN_BYTES: usize = 32; // 256 bits
const AES_128_KEY_SIZE_IN_BYTES: usize = 16; // 128 bits
const AES_BLOCK_SIZE_IN_BYTES: usize = 16; // 128-bit block size for AES

const CRYPT_CHUNK_SIZE: usize = 64 * 1024;

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
    source: ByteSource,
    encryption_material: &EncryptionMaterial,
) -> Result<PreparedUpload, EncryptionError> {
    let master_key = BASE64_ENGINE
        .decode(encryption_material.query_stage_master_key.reveal())
        .context(Base64DecodingSnafu {
            context: "master key",
        })?;
    let cipher_suite = CipherSuite::from_key_len(master_key.len())?;

    let file_key = generate_random_bytes(cipher_suite.key_len).context(OpenSSLSnafu {
        operation: "generating file key",
    })?;
    let iv = generate_random_bytes(AES_BLOCK_SIZE_IN_BYTES).context(OpenSSLSnafu {
        operation: "generating initialization vector",
    })?;

    let (mut reader, capacity_hint): (Box<dyn Read>, usize) = match source {
        ByteSource::Path(p) => {
            let f = std::fs::File::open(&p).context(IoSnafu {
                operation: "opening source file for encryption",
            })?;
            // Capacity hint is just an optimisation; on metadata failure fall
            // back to one chunk's worth so we still avoid the first realloc.
            let hint = match f.metadata() {
                Ok(m) => m.len() as usize,
                Err(e) => {
                    tracing::debug!(
                        "metadata() failed on encryption source {}; using chunk-sized capacity hint: {}",
                        p.display(),
                        e
                    );
                    CRYPT_CHUNK_SIZE
                }
            };
            (Box::new(f), hint)
        }
        ByteSource::Bytes(b) => {
            let hint = b.len();
            (Box::new(std::io::Cursor::new(b)), hint)
        }
    };

    let mut crypter = Crypter::new(cipher_suite.cbc, Mode::Encrypt, &file_key, Some(&iv)).context(
        OpenSSLSnafu {
            operation: "initializing AES-CBC encryptor",
        },
    )?;
    crypter.pad(true);

    let mut hasher = Hasher::new(MessageDigest::sha256()).context(OpenSSLSnafu {
        operation: "initializing SHA-256 hasher for encryption",
    })?;

    let mut plaintext_buf = vec![0u8; CRYPT_CHUNK_SIZE];
    // PKCS#7 padding can append one extra block on finalize.
    let mut cipher_buf = vec![0u8; CRYPT_CHUNK_SIZE + AES_BLOCK_SIZE_IN_BYTES];
    let mut encrypted_data: Vec<u8> =
        Vec::with_capacity(capacity_hint.saturating_add(AES_BLOCK_SIZE_IN_BYTES));

    loop {
        let n = reader.read(&mut plaintext_buf).context(IoSnafu {
            operation: "reading plaintext for encryption",
        })?;
        if n == 0 {
            break;
        }
        // Digest is computed over the (compressed) plaintext, not the ciphertext.
        // Each upload uses a fresh random IV, so a ciphertext digest would change
        // every time and never match the stored header. Hashing the plaintext keeps
        // the digest stable across uploads and interoperable with other drivers.
        hasher.update(&plaintext_buf[..n]).context(OpenSSLSnafu {
            operation: "hashing plaintext chunk",
        })?;
        let written = crypter
            .update(&plaintext_buf[..n], &mut cipher_buf)
            .context(OpenSSLSnafu {
                operation: "encrypting data chunk with AES-CBC",
            })?;
        encrypted_data.extend_from_slice(&cipher_buf[..written]);
    }

    let tail_written = crypter.finalize(&mut cipher_buf).context(OpenSSLSnafu {
        operation: "finalizing AES-CBC encryption",
    })?;
    encrypted_data.extend_from_slice(&cipher_buf[..tail_written]);

    let digest_bytes = hasher.finish().context(OpenSSLSnafu {
        operation: "finalizing SHA-256 digest",
    })?;
    let digest = BASE64_ENGINE.encode(digest_bytes);

    let encrypted_file_key =
        encrypt(cipher_suite.ecb, &master_key, None, &file_key).context(OpenSSLSnafu {
            operation: "encrypting file key with AES-ECB",
        })?;

    let material_desc = MaterialDescription {
        query_id: encryption_material.query_id.clone(),
        smk_id: encryption_material.smk_id.clone(),
        key_size: (cipher_suite.key_len * 8).to_string(),
    };

    let metadata = EncryptedFileMetadata {
        encrypted_key: BASE64_ENGINE.encode(&encrypted_file_key),
        iv: BASE64_ENGINE.encode(&iv),
        material_desc,
    };

    Ok(PreparedUpload {
        // The plaintext is streamed through Crypter, but the ciphertext is
        // accumulated into one Vec because PUT bodies are randomly accessed
        // (signature, retries) and a Path destination would mean a temp file
        // per upload. Peak memory for an encrypted upload is therefore still
        // ~file_size; a future refactor that wants true streaming end-to-end
        // would spill the ciphertext to a temp file and return ByteSource::Path
        // here, which the cloud upload paths already know how to stream.
        data: ByteSource::Bytes(encrypted_data),
        digest,
        encryption_metadata: Some(metadata),
    })
}

/// Decrypts `ciphertext` into `output`, verifying the SHA-256 digest at
/// finalize time. On `DigestMismatch`, partial plaintext may already have
/// been written — callers must discard the partial output.
pub fn decrypt_ciphertext_to_writer<R: Read, W: Write>(
    mut ciphertext: R,
    metadata: &EncryptedFileMetadata,
    digest: &str,
    encryption_material: &EncryptionMaterial,
    output: &mut W,
) -> Result<i64, EncryptionError> {
    let master_key = BASE64_ENGINE
        .decode(encryption_material.query_stage_master_key.reveal())
        .context(Base64DecodingSnafu {
            context: "master key",
        })?;
    let cipher_suite = CipherSuite::from_key_len(master_key.len())?;

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

    let file_key = decrypt(cipher_suite.ecb, &master_key, None, &encrypted_file_key).context(
        OpenSSLSnafu {
            operation: "decrypting file key with AES-ECB",
        },
    )?;

    let mut crypter = Crypter::new(cipher_suite.cbc, Mode::Decrypt, &file_key, Some(&iv)).context(
        OpenSSLSnafu {
            operation: "initializing AES-CBC decryptor",
        },
    )?;
    crypter.pad(true);

    // The digest stored on upload is the SHA-256 of the (compressed) plaintext,
    // not the ciphertext, so verification hashes the decrypted output.
    let mut hasher = Hasher::new(MessageDigest::sha256()).context(OpenSSLSnafu {
        operation: "initializing SHA-256 hasher for decryption",
    })?;

    let mut cipher_buf = vec![0u8; CRYPT_CHUNK_SIZE];
    let mut plain_buf = vec![0u8; CRYPT_CHUNK_SIZE + AES_BLOCK_SIZE_IN_BYTES];
    let mut output_byte_len: i64 = 0;

    loop {
        let n = ciphertext.read(&mut cipher_buf).context(IoSnafu {
            operation: "reading ciphertext for decryption",
        })?;
        if n == 0 {
            break;
        }
        let written = crypter
            .update(&cipher_buf[..n], &mut plain_buf)
            .context(OpenSSLSnafu {
                operation: "decrypting data chunk with AES-CBC",
            })?;
        if written > 0 {
            let plaintext = &plain_buf[..written];
            hasher.update(plaintext).context(OpenSSLSnafu {
                operation: "hashing plaintext chunk",
            })?;
            output.write_all(plaintext).context(IoSnafu {
                operation: "writing decrypted chunk to output",
            })?;
            output_byte_len += written as i64;
        }
    }

    let tail_written = crypter.finalize(&mut plain_buf).context(OpenSSLSnafu {
        operation: "finalizing AES-CBC decryption",
    })?;
    if tail_written > 0 {
        let plaintext = &plain_buf[..tail_written];
        hasher.update(plaintext).context(OpenSSLSnafu {
            operation: "hashing final plaintext block",
        })?;
        output.write_all(plaintext).context(IoSnafu {
            operation: "writing final decrypted block",
        })?;
        output_byte_len += tail_written as i64;
    }

    let computed_bytes = hasher.finish().context(OpenSSLSnafu {
        operation: "finalizing SHA-256 digest for verification",
    })?;
    let computed = BASE64_ENGINE.encode(computed_bytes);
    if computed != digest {
        return DigestMismatchSnafu.fail();
    }

    Ok(output_byte_len)
}

/// Generates a vector of random bytes of a specified size.
fn generate_random_bytes(size: usize) -> Result<Vec<u8>, OpenSslErrorStack> {
    let mut buffer = vec![0; size];
    rand_bytes(&mut buffer)?;
    Ok(buffer)
}

/// Computes the SHA-256 digest of the data and returns it as a Base64 string.
pub(super) fn compute_sha256_digest(data: &[u8]) -> Result<String, OpenSslErrorStack> {
    let digest = hash(MessageDigest::sha256(), data)?;
    Ok(BASE64_ENGINE.encode(digest))
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum EncryptionError {
    #[snafu(display("OpenSSL cryptographic operation failed during {operation}"))]
    OpenSSL {
        operation: String,
        source: OpenSslErrorStack,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("I/O error during {operation}"))]
    Io {
        operation: &'static str,
        source: std::io::Error,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensitive::SensitiveString;

    fn test_material() -> EncryptionMaterial {
        // 32-byte master key (AES-256), Base64-encoded as the wire format.
        let master_key = BASE64_ENGINE.encode([7u8; AES_256_KEY_SIZE_IN_BYTES]);
        EncryptionMaterial {
            query_stage_master_key: SensitiveString::from(master_key),
            query_id: "test-query-id".to_string(),
            smk_id: "123".to_string(),
        }
    }

    #[test]
    fn encrypt_digest_is_sha256_of_plaintext_not_ciphertext() {
        let plaintext = b"the quick brown fox jumps over the lazy dog";
        let material = test_material();

        let prepared = encrypt_file_data(ByteSource::Bytes(plaintext.to_vec()), &material).unwrap();

        let expected = compute_sha256_digest(plaintext).unwrap();
        assert_eq!(prepared.digest, expected);
        // The ciphertext digest must NOT be what we store, otherwise the
        // random per-upload IV would make the digest non-reproducible.
        let ciphertext = prepared.data.into_bytes().unwrap();
        let ciphertext_digest = compute_sha256_digest(&ciphertext).unwrap();
        assert_ne!(prepared.digest, ciphertext_digest);
    }

    #[test]
    fn encrypt_digest_is_stable_across_uploads_of_same_content() {
        let plaintext = b"identical content";
        let material = test_material();

        let first = encrypt_file_data(ByteSource::Bytes(plaintext.to_vec()), &material).unwrap();
        let second = encrypt_file_data(ByteSource::Bytes(plaintext.to_vec()), &material).unwrap();

        // Fresh IV per upload => different ciphertext bytes ...
        assert_ne!(
            first.data.into_bytes().unwrap(),
            second.data.into_bytes().unwrap()
        );
        // ... but identical plaintext digest, which is what enables the
        // content-match upload skip.
        assert_eq!(first.digest, second.digest);
    }

    #[test]
    fn encrypt_then_decrypt_round_trips_and_verifies_plaintext_digest() {
        let plaintext = b"round-trip payload";
        let material = test_material();

        let prepared = encrypt_file_data(ByteSource::Bytes(plaintext.to_vec()), &material).unwrap();
        let metadata = prepared.encryption_metadata.unwrap();

        let ciphertext = prepared.data.into_bytes().unwrap();
        let mut decrypted = Vec::new();
        decrypt_ciphertext_to_writer(
            &ciphertext[..],
            &metadata,
            &prepared.digest,
            &material,
            &mut decrypted,
        )
        .unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_rejects_mismatched_digest() {
        let plaintext = b"payload to tamper-check";
        let material = test_material();

        let prepared = encrypt_file_data(ByteSource::Bytes(plaintext.to_vec()), &material).unwrap();
        let metadata = prepared.encryption_metadata.unwrap();
        let wrong_digest = compute_sha256_digest(b"different content").unwrap();

        let ciphertext = prepared.data.into_bytes().unwrap();
        let mut output = Vec::new();
        let result = decrypt_ciphertext_to_writer(
            &ciphertext[..],
            &metadata,
            &wrong_digest,
            &material,
            &mut output,
        );

        assert!(matches!(
            result,
            Err(EncryptionError::DigestMismatch { .. })
        ));
    }
}
