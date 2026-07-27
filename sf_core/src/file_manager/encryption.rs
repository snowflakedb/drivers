use super::types::{ByteSource, EncryptedFileMetadata, EncryptionMaterial, MaterialDescription};
use crate::sensitive::Sensitive;
use snafu::{Location, ResultExt, Snafu};

use base64::{Engine, engine::general_purpose::STANDARD as BASE64_ENGINE};
use openssl::{
    error::ErrorStack as OpenSslErrorStack,
    hash::{Hasher, MessageDigest},
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

/// AES-CBC/PKCS#7 ciphertext length for a `source_len`-byte plaintext: always
/// rounds up to the next full block, adding a whole padding block when
/// `source_len` is already a block multiple (PKCS#7 always pads).
fn cbc_ciphertext_len(source_len: i64) -> i64 {
    let block = AES_BLOCK_SIZE_IN_BYTES as i64;
    source_len + (block - source_len % block)
}

/// Everything needed to encrypt an upload body on demand: the per-file AES key,
/// the IV, and the exact ciphertext length (so the cloud `Content-Length` can be
/// set before the body streams). AES-CBC with a fixed key+IV is deterministic,
/// so a fresh [`EncryptingReader`] per retry reproduces byte-identical
/// ciphertext — the digest (computed over the *plaintext* source) stays valid.
///
/// The per-file key is held in [`Sensitive`] so it is zeroized on drop and
/// redacted from `Debug` (an `Encryptor` rides on the `Debug`-derived
/// `PreparedUpload`, e.g. via `tracing`). The IV is not secret — it travels in
/// the `x-amz-iv` / `encryptiondata` metadata header — so it is left bare.
#[derive(Debug, Clone)]
pub struct Encryptor {
    file_key: Sensitive<Vec<u8>>,
    iv: Vec<u8>,
    cipher_len: i64,
}

impl Encryptor {
    /// Exact length of the ciphertext this encryptor will produce.
    pub fn cipher_len(&self) -> i64 {
        self.cipher_len
    }

    /// Wraps `source` in a streaming AES-CBC encryptor — the sync analogue of
    /// JDBC's `CipherInputStream` / libsnowflakeclient's `CipherStreamBuf`.
    /// Ciphertext is produced lazily as the reader is pulled; nothing beyond
    /// `~CRYPT_CHUNK_SIZE` is buffered.
    pub fn encrypting_reader<R: Read>(
        &self,
        source: R,
    ) -> Result<EncryptingReader<R>, EncryptionError> {
        let cipher_suite = CipherSuite::from_key_len(self.file_key.reveal().len())?;
        let mut crypter = Crypter::new(
            cipher_suite.cbc,
            Mode::Encrypt,
            self.file_key.reveal(),
            Some(&self.iv),
        )
        .context(OpenSSLSnafu {
            operation: "initializing AES-CBC encryptor",
        })?;
        crypter.pad(true);
        Ok(EncryptingReader {
            source,
            crypter,
            chunk: vec![0u8; CRYPT_CHUNK_SIZE],
            staged: Vec::new(),
            staged_pos: 0,
            source_done: false,
            finalized: false,
        })
    }
}

/// Builds the per-file [`Encryptor`] and the `EncryptedFileMetadata` the cloud
/// needs (encrypted file key, IV, material description). Pure: no file I/O —
/// the ciphertext length is analytic (from `source_len`) and the `sfc-digest`
/// is computed separately over the source by [`compute_sha256_digest`].
pub fn build_encryptor(
    encryption_material: &EncryptionMaterial,
    source_len: i64,
) -> Result<(Encryptor, EncryptedFileMetadata), EncryptionError> {
    let master_key = BASE64_ENGINE
        .decode(encryption_material.query_stage_master_key.reveal())
        .context(Base64DecodeSnafu {
            context: "master key",
        })?;
    let cipher_suite = CipherSuite::from_key_len(master_key.len())?;

    let file_key = generate_random_bytes(cipher_suite.key_len).context(OpenSSLSnafu {
        operation: "generating file key",
    })?;
    let iv = generate_random_bytes(AES_BLOCK_SIZE_IN_BYTES).context(OpenSSLSnafu {
        operation: "generating initialization vector",
    })?;

    let encrypted_file_key =
        encrypt(cipher_suite.ecb, &master_key, None, &file_key).context(OpenSSLSnafu {
            operation: "encrypting file key with AES-ECB",
        })?;

    let metadata = EncryptedFileMetadata {
        encrypted_key: BASE64_ENGINE.encode(&encrypted_file_key),
        iv: BASE64_ENGINE.encode(&iv),
        material_desc: MaterialDescription {
            query_id: encryption_material.query_id.clone(),
            smk_id: encryption_material.smk_id.clone(),
            key_size: (cipher_suite.key_len * 8).to_string(),
        },
    };

    let encryptor = Encryptor {
        file_key: file_key.into(),
        iv,
        cipher_len: cbc_ciphertext_len(source_len),
    };
    Ok((encryptor, metadata))
}

/// Streaming AES-CBC/PKCS#7 encryptor over an arbitrary `Read` source. Each
/// `read` drains previously-produced ciphertext, then encrypts the next
/// `CRYPT_CHUNK_SIZE` plaintext chunk (or emits the final padded block at EOF).
/// Peak resident memory is `~CRYPT_CHUNK_SIZE`, independent of file size.
pub struct EncryptingReader<R: Read> {
    source: R,
    crypter: Crypter,
    /// Reused plaintext read buffer.
    chunk: Vec<u8>,
    /// Ciphertext produced but not yet handed to the caller.
    staged: Vec<u8>,
    staged_pos: usize,
    source_done: bool,
    finalized: bool,
}

impl<R: Read> Read for EncryptingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            // 1. Hand back any staged ciphertext first.
            if self.staged_pos < self.staged.len() {
                let n = (self.staged.len() - self.staged_pos).min(buf.len());
                buf[..n].copy_from_slice(&self.staged[self.staged_pos..self.staged_pos + n]);
                self.staged_pos += n;
                return Ok(n);
            }
            if self.finalized {
                return Ok(0);
            }

            self.staged.clear();
            self.staged_pos = 0;

            // 2. At source EOF, emit the final padded block exactly once.
            if self.source_done {
                self.staged.resize(AES_BLOCK_SIZE_IN_BYTES * 2, 0);
                let w = self
                    .crypter
                    .finalize(&mut self.staged)
                    .map_err(std::io::Error::other)?;
                self.staged.truncate(w);
                self.finalized = true;
                continue;
            }

            // 3. Encrypt the next plaintext chunk. `update` may yield 0 bytes
            //    (CBC buffers a partial block) — loop and read more.
            let n = self.source.read(&mut self.chunk)?;
            if n == 0 {
                self.source_done = true;
                continue;
            }
            self.staged.resize(n + AES_BLOCK_SIZE_IN_BYTES, 0);
            let w = self
                .crypter
                .update(&self.chunk[..n], &mut self.staged)
                .map_err(std::io::Error::other)?;
            self.staged.truncate(w);
        }
    }
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
        .context(Base64DecodeSnafu {
            context: "master key",
        })?;
    let cipher_suite = CipherSuite::from_key_len(master_key.len())?;

    let encrypted_file_key =
        BASE64_ENGINE
            .decode(&metadata.encrypted_key)
            .context(Base64DecodeSnafu {
                context: "encrypted file key",
            })?;
    let iv = BASE64_ENGINE
        .decode(&metadata.iv)
        .context(Base64DecodeSnafu {
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

/// SHA-256 of `source` as Base64 — the `sfc-digest` over the pre-encryption
/// bytes (matching JDBC/ODBC). `Path` streams 64 KiB chunks; `Bytes` hashes in
/// place. Used by **both** the CSE and SSE upload paths so the source is never
/// materialized as a `Vec<u8>` just to compute a digest.
pub fn compute_sha256_digest(source: &ByteSource) -> Result<String, EncryptionError> {
    let mut hasher = Hasher::new(MessageDigest::sha256()).context(OpenSSLSnafu {
        operation: "initializing SHA-256 hasher",
    })?;
    match source {
        ByteSource::Path(p) => {
            let mut f = std::fs::File::open(p).context(IoSnafu {
                operation: "opening source for SHA-256 digest",
            })?;
            let mut buf = vec![0u8; CRYPT_CHUNK_SIZE];
            loop {
                let n = f.read(&mut buf).context(IoSnafu {
                    operation: "reading source for SHA-256 digest",
                })?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]).context(OpenSSLSnafu {
                    operation: "hashing data chunk",
                })?;
            }
        }
        ByteSource::Bytes(b) => {
            hasher.update(b).context(OpenSSLSnafu {
                operation: "hashing in-memory bytes",
            })?;
        }
    }
    let digest = hasher.finish().context(OpenSSLSnafu {
        operation: "finalizing SHA-256 digest",
    })?;
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
    Base64Decode {
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

    /// Encrypts `plaintext` through the lazy `EncryptingReader` and collects
    /// the ciphertext — the production upload body, materialized for the test.
    fn encrypt_to_vec(enc: &Encryptor, plaintext: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        enc.encrypting_reader(std::io::Cursor::new(plaintext.to_vec()))
            .unwrap()
            .read_to_end(&mut out)
            .unwrap();
        out
    }

    fn digest_of(bytes: &[u8]) -> String {
        compute_sha256_digest(&ByteSource::Bytes(bytes::Bytes::copy_from_slice(bytes))).unwrap()
    }

    #[test]
    fn sfc_digest_is_sha256_of_plaintext_not_ciphertext() {
        let plaintext = b"the quick brown fox jumps over the lazy dog";
        let material = test_material();

        let (enc, _meta) = build_encryptor(&material, plaintext.len() as i64).unwrap();
        let ciphertext = encrypt_to_vec(&enc, plaintext);

        // The `sfc-digest` is over the plaintext; the ciphertext digest must
        // differ (the random per-upload IV would otherwise make it unstable).
        assert_ne!(digest_of(plaintext), digest_of(&ciphertext));
    }

    /// The lazy `EncryptingReader` must produce exactly the same ciphertext as a
    /// one-shot `openssl::symm::encrypt` with the same key+IV, at and around the
    /// chunk/block boundaries — and must be deterministic across rebuilds, which
    /// is what makes upload retries (re-encryption) safe.
    #[test]
    fn encrypting_reader_matches_one_shot_and_is_deterministic() {
        let material = test_material();
        for len in [
            0usize,
            1,
            15,
            16,
            17,
            CRYPT_CHUNK_SIZE - 1,
            CRYPT_CHUNK_SIZE + 5,
        ] {
            let plaintext = vec![0xABu8; len];
            let (enc, _meta) = build_encryptor(&material, len as i64).unwrap();

            let lazy = encrypt_to_vec(&enc, &plaintext);

            let cbc = CipherSuite::from_key_len(enc.file_key.reveal().len())
                .unwrap()
                .cbc;
            let one_shot = encrypt(cbc, enc.file_key.reveal(), Some(&enc.iv), &plaintext)
                .expect("one-shot encrypt");
            assert_eq!(
                lazy, one_shot,
                "lazy ciphertext must match one-shot (len {len})"
            );
            assert_eq!(
                enc.cipher_len(),
                lazy.len() as i64,
                "analytic cipher_len must match actual (len {len})",
            );

            let again = encrypt_to_vec(&enc, &plaintext);
            assert_eq!(
                lazy, again,
                "re-encryption must be byte-identical (len {len})"
            );
        }
    }

    /// Reading the `EncryptingReader` through a buffer smaller than a staged
    /// ciphertext block exercises the partial-drain branch (`staged_pos`) that
    /// the bulk `read_to_end` path never hits. Output must still equal one-shot.
    #[test]
    fn encrypting_reader_partial_reads_into_small_buffer() {
        let material = test_material();
        let plaintext = vec![0x42u8; CRYPT_CHUNK_SIZE + 100];
        let (enc, _meta) = build_encryptor(&material, plaintext.len() as i64).unwrap();

        let mut reader = enc
            .encrypting_reader(std::io::Cursor::new(plaintext.clone()))
            .unwrap();
        let mut out = Vec::new();
        let mut buf = [0u8; 7]; // deliberately tiny, not a block multiple
        loop {
            let n = reader.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n]);
        }

        let cbc = CipherSuite::from_key_len(enc.file_key.reveal().len())
            .unwrap()
            .cbc;
        let one_shot = encrypt(cbc, enc.file_key.reveal(), Some(&enc.iv), &plaintext).unwrap();
        assert_eq!(out, one_shot);
    }

    #[test]
    fn encrypt_then_decrypt_round_trips_and_verifies_plaintext_digest() {
        let plaintext = b"round-trip payload";
        let material = test_material();

        let (enc, metadata) = build_encryptor(&material, plaintext.len() as i64).unwrap();
        let ciphertext = encrypt_to_vec(&enc, plaintext);
        let digest = digest_of(plaintext);

        let mut decrypted = Vec::new();
        decrypt_ciphertext_to_writer(
            &ciphertext[..],
            &metadata,
            &digest,
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

        let (enc, metadata) = build_encryptor(&material, plaintext.len() as i64).unwrap();
        let ciphertext = encrypt_to_vec(&enc, plaintext);
        let wrong_digest = digest_of(b"different content");

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
