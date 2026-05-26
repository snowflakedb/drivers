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

/// Chunk size for streaming through the Crypter. 64 KiB.
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
///
/// The plaintext is streamed through `openssl::symm::Crypter` in
/// `CRYPT_CHUNK_SIZE` chunks, so the full plaintext and full ciphertext are
/// never resident in memory at the same time. The SHA-256 digest is computed
/// over the ciphertext in the same pass.
///
/// The returned `PreparedUpload.data` is a `ByteSource::Bytes` holding the
/// ciphertext. The S3 PUT path converts that to a `ByteStream` without an
/// additional copy.
pub fn encrypt_file_data(
    source: ByteSource,
    encryption_material: &EncryptionMaterial,
) -> Result<PreparedUpload, EncryptionError> {
    // 1. Decode master key and select the appropriate cipher suite.
    let master_key = BASE64_ENGINE
        .decode(encryption_material.query_stage_master_key.reveal())
        .context(Base64DecodingSnafu {
            context: "master key",
        })?;
    let cipher_suite = CipherSuite::from_key_len(master_key.len())?;

    // 2. Generate a random data encryption key (file key) and IV.
    let file_key = generate_random_bytes(cipher_suite.key_len).context(OpenSSLSnafu {
        operation: "generating file key",
    })?;
    let iv = generate_random_bytes(AES_BLOCK_SIZE_IN_BYTES).context(OpenSSLSnafu {
        operation: "generating initialization vector",
    })?;

    // 3. Open the plaintext source as a reader.
    let mut reader: Box<dyn Read> = match source {
        ByteSource::Path(p) => Box::new(std::fs::File::open(&p).context(IoSnafu {
            operation: "opening source file for encryption",
        })?),
        ByteSource::Bytes(b) => Box::new(std::io::Cursor::new(b)),
    };

    // 4. Stream plaintext through Crypter, collecting ciphertext + digest.
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
    // Output buffer needs room for one extra block due to CBC PKCS#7 padding.
    let mut cipher_buf = vec![0u8; CRYPT_CHUNK_SIZE + AES_BLOCK_SIZE_IN_BYTES];
    let mut encrypted_data: Vec<u8> = Vec::new();

    loop {
        let n = reader.read(&mut plaintext_buf).context(IoSnafu {
            operation: "reading plaintext for encryption",
        })?;
        if n == 0 {
            break;
        }
        let written = crypter
            .update(&plaintext_buf[..n], &mut cipher_buf)
            .context(OpenSSLSnafu {
                operation: "encrypting data chunk with AES-CBC",
            })?;
        let chunk = &cipher_buf[..written];
        hasher.update(chunk).context(OpenSSLSnafu {
            operation: "hashing ciphertext chunk",
        })?;
        encrypted_data.extend_from_slice(chunk);
    }

    // Finalize encryption (emits PKCS#7 padding block).
    let tail_written = crypter.finalize(&mut cipher_buf).context(OpenSSLSnafu {
        operation: "finalizing AES-CBC encryption",
    })?;
    let tail = &cipher_buf[..tail_written];
    hasher.update(tail).context(OpenSSLSnafu {
        operation: "hashing final ciphertext block",
    })?;
    encrypted_data.extend_from_slice(tail);

    let digest_bytes = hasher.finish().context(OpenSSLSnafu {
        operation: "finalizing SHA-256 digest",
    })?;
    let digest = BASE64_ENGINE.encode(digest_bytes);

    // 5. Wrap the file key with AES-ECB.
    let encrypted_file_key =
        encrypt(cipher_suite.ecb, &master_key, None, &file_key).context(OpenSSLSnafu {
            operation: "encrypting file key with AES-ECB",
        })?;

    // 6. Build metadata.
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
        data: ByteSource::Bytes(encrypted_data),
        digest,
        encryption_metadata: Some(metadata),
    })
}

/// Decrypts `ciphertext` (any `Read` source) into `output`, verifying the
/// SHA-256 digest at finalize time. Returns the number of plaintext bytes written.
///
/// **Behavioral change vs. the old `decrypt_file_data`**: The original function
/// verified the digest eagerly over the full in-memory ciphertext buffer before
/// any decryption occurred. Under streaming, the digest is computed over
/// ciphertext as it flows through and verified only after all bytes have been
/// decrypted and written to `output`. If `DigestMismatch` is returned, some
/// plaintext may already have been written — callers should discard the partial
/// output (e.g. delete the destination file).
pub fn decrypt_ciphertext_to_writer<R: Read, W: Write>(
    ciphertext: R,
    metadata: &EncryptedFileMetadata,
    digest: &str,
    encryption_material: &EncryptionMaterial,
    output: &mut W,
) -> Result<i64, EncryptionError> {
    // 1. Decode master key and select the appropriate cipher suite.
    let master_key = BASE64_ENGINE
        .decode(encryption_material.query_stage_master_key.reveal())
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

    // 3. Unwrap the file key using the master key with AES-ECB.
    let file_key = decrypt(cipher_suite.ecb, &master_key, None, &encrypted_file_key).context(
        OpenSSLSnafu {
            operation: "decrypting file key with AES-ECB",
        },
    )?;

    // 4. Set up CBC decryptor and SHA-256 hasher (over ciphertext).
    let mut crypter = Crypter::new(cipher_suite.cbc, Mode::Decrypt, &file_key, Some(&iv)).context(
        OpenSSLSnafu {
            operation: "initializing AES-CBC decryptor",
        },
    )?;
    crypter.pad(true);

    let mut hasher = Hasher::new(MessageDigest::sha256()).context(OpenSSLSnafu {
        operation: "initializing SHA-256 hasher for decryption",
    })?;

    // 5. Stream ciphertext through hasher + decryptor, writing plaintext.
    let mut ciphertext_reader = ciphertext;
    let mut cipher_buf = vec![0u8; CRYPT_CHUNK_SIZE];
    let mut plain_buf = vec![0u8; CRYPT_CHUNK_SIZE + AES_BLOCK_SIZE_IN_BYTES];
    let mut total_output: i64 = 0;

    loop {
        let n = ciphertext_reader.read(&mut cipher_buf).context(IoSnafu {
            operation: "reading ciphertext for decryption",
        })?;
        if n == 0 {
            break;
        }
        let chunk = &cipher_buf[..n];
        hasher.update(chunk).context(OpenSSLSnafu {
            operation: "hashing ciphertext chunk",
        })?;
        let written = crypter
            .update(chunk, &mut plain_buf)
            .context(OpenSSLSnafu {
                operation: "decrypting data chunk with AES-CBC",
            })?;
        if written > 0 {
            output.write_all(&plain_buf[..written]).context(IoSnafu {
                operation: "writing decrypted chunk to output",
            })?;
            total_output += written as i64;
        }
    }

    // Finalize decryption (strips PKCS#7 padding).
    let tail_written = crypter.finalize(&mut plain_buf).context(OpenSSLSnafu {
        operation: "finalizing AES-CBC decryption",
    })?;
    if tail_written > 0 {
        output
            .write_all(&plain_buf[..tail_written])
            .context(IoSnafu {
                operation: "writing final decrypted block",
            })?;
        total_output += tail_written as i64;
    }

    // 6. Verify the digest at finalize time (post-decryption).
    let computed_bytes = hasher.finish().context(OpenSSLSnafu {
        operation: "finalizing SHA-256 digest for verification",
    })?;
    let computed = BASE64_ENGINE.encode(computed_bytes);
    if computed != digest {
        return DigestMismatchSnafu.fail();
    }

    Ok(total_output)
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
