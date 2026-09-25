use super::types::{ByteSource, EncryptedFileMetadata, EncryptionMaterial, MaterialDescription};
use crate::sensitive::Sensitive;
use snafu::{Location, ResultExt, Snafu, ensure};

use aws_lc_rs::digest::{Context as Sha256Context, SHA256};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_ENGINE};
use md5::{Digest as _, Md5};
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
/// ciphertext — the plaintext `sfc-digest` stays valid across retries.
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
/// the ciphertext length is analytic (from `source_len`).
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

/// Whether `metadata`'s wrapped file key unwraps with `encryption_material` to
/// an AES key of the master-key length.
///
/// `Ok(false)`: the wrap is not a CSE key wrap for this master key (git-stage
/// dummy `"test-key"`, wrong ciphertext length). GET writes raw bytes.
///
/// `Err`: the wrap length matches a real CSE key wrap and unwrap still failed.
/// GET fails instead of writing ciphertext.
///
/// TODO(SNOW-4115038): `Ok(false)` still writes raw bytes. Legacy
/// Python/JDBC/Node fail the GET. Loud failure is the intended contract.
pub(super) fn wrapped_key_matches_material(
    metadata: &EncryptedFileMetadata,
    encryption_material: &EncryptionMaterial,
) -> Result<bool, EncryptionError> {
    let master_key = BASE64_ENGINE
        .decode(encryption_material.query_stage_master_key.reveal())
        .context(Base64DecodeSnafu {
            context: "master key",
        })?;
    let cipher_suite = CipherSuite::from_key_len(master_key.len())?;
    match BASE64_ENGINE.decode(&metadata.encrypted_key) {
        Ok(wrapped) if wrapped.len() == cipher_suite.key_len + AES_BLOCK_SIZE_IN_BYTES => {
            unwrap_file_key(metadata, encryption_material).map(|_| true)
        }
        _ => Ok(false),
    }
}

fn unwrap_file_key(
    metadata: &EncryptedFileMetadata,
    encryption_material: &EncryptionMaterial,
) -> Result<(Vec<u8>, CipherSuite), EncryptionError> {
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

    let file_key = decrypt(cipher_suite.ecb, &master_key, None, &encrypted_file_key).context(
        OpenSSLSnafu {
            operation: "decrypting file key with AES-ECB",
        },
    )?;
    ensure!(
        file_key.len() == cipher_suite.key_len,
        UnsupportedKeySizeSnafu {
            key_size: file_key.len()
        }
    );
    Ok((file_key, cipher_suite))
}

/// Decrypts `ciphertext` into `output`. When `digest` is present it is
/// verified at finalize; on `DigestMismatch`, callers must discard any
/// already-written output.
pub fn decrypt_ciphertext_to_writer<R: Read, W: Write>(
    mut ciphertext: R,
    metadata: &EncryptedFileMetadata,
    digest: Option<&str>,
    encryption_material: &EncryptionMaterial,
    output: &mut W,
) -> Result<i64, EncryptionError> {
    let (file_key, cipher_suite) = unwrap_file_key(metadata, encryption_material)?;
    let iv = BASE64_ENGINE
        .decode(&metadata.iv)
        .context(Base64DecodeSnafu {
            context: "initialization vector",
        })?;

    let mut crypter = Crypter::new(cipher_suite.cbc, Mode::Decrypt, &file_key, Some(&iv)).context(
        OpenSSLSnafu {
            operation: "initializing AES-CBC decryptor",
        },
    )?;
    crypter.pad(true);

    // The digest stored on upload is the SHA-256 of the (compressed) plaintext,
    // not the ciphertext, so verification hashes the decrypted output.
    let mut hasher = digest
        .map(|_| {
            Hasher::new(MessageDigest::sha256()).context(OpenSSLSnafu {
                operation: "initializing SHA-256 hasher for decryption",
            })
        })
        .transpose()?;

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
            if let Some(hasher) = hasher.as_mut() {
                hasher.update(plaintext).context(OpenSSLSnafu {
                    operation: "hashing plaintext chunk",
                })?;
            }
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
        if let Some(hasher) = hasher.as_mut() {
            hasher.update(plaintext).context(OpenSSLSnafu {
                operation: "hashing final plaintext block",
            })?;
        }
        output.write_all(plaintext).context(IoSnafu {
            operation: "writing final decrypted block",
        })?;
        output_byte_len += tail_written as i64;
    }

    if let (Some(expected), Some(mut hasher)) = (digest, hasher) {
        let computed_bytes = hasher.finish().context(OpenSSLSnafu {
            operation: "finalizing SHA-256 digest for verification",
        })?;
        let computed = BASE64_ENGINE.encode(computed_bytes);
        if computed != expected {
            return DigestMismatchSnafu.fail();
        }
    }

    Ok(output_byte_len)
}

/// Generates a vector of random bytes of a specified size.
fn generate_random_bytes(size: usize) -> Result<Vec<u8>, OpenSslErrorStack> {
    let mut buffer = vec![0; size];
    rand_bytes(&mut buffer)?;
    Ok(buffer)
}

fn compute_upload_sha256(
    source: &ByteSource,
    encryptor: Option<&Encryptor>,
    on_body_chunk: impl FnMut(&[u8]),
) -> Result<String, EncryptionError> {
    let reader = source.open().context(IoSnafu {
        operation: "opening source for digest computation",
    })?;
    compute_upload_sha256_from_reader(reader, encryptor, on_body_chunk)
}

fn compute_upload_sha256_from_reader<R: Read>(
    source: R,
    encryptor: Option<&Encryptor>,
    mut on_body_chunk: impl FnMut(&[u8]),
) -> Result<String, EncryptionError> {
    struct Sha256Reader<'a, R> {
        source: R,
        hasher: &'a mut Sha256Context,
    }

    impl<R: Read> Read for Sha256Reader<'_, R> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let n = self.source.read(buf)?;
            self.hasher.update(&buf[..n]);
            Ok(n)
        }
    }

    enum DigestBody<'a, R: Read> {
        Plain(Sha256Reader<'a, R>),
        Encrypted(EncryptingReader<Sha256Reader<'a, R>>),
    }

    impl<R: Read> Read for DigestBody<'_, R> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            match self {
                Self::Plain(reader) => reader.read(buf),
                Self::Encrypted(reader) => reader.read(buf),
            }
        }
    }

    let mut sha256_hasher = Sha256Context::new(&SHA256);
    {
        let sha256_reader = Sha256Reader {
            source,
            hasher: &mut sha256_hasher,
        };
        let mut body = match encryptor {
            Some(encryptor) => DigestBody::Encrypted(encryptor.encrypting_reader(sha256_reader)?),
            None => DigestBody::Plain(sha256_reader),
        };
        let mut buf = vec![0u8; CRYPT_CHUNK_SIZE];
        loop {
            let n = body.read(&mut buf).context(IoSnafu {
                operation: "reading source for digest computation",
            })?;
            if n == 0 {
                break;
            }
            on_body_chunk(&buf[..n]);
        }
    }

    Ok(BASE64_ENGINE.encode(sha256_hasher.finish().as_ref()))
}

pub(super) fn compute_azure_blob_digests(
    source: &ByteSource,
    encryptor: Option<&Encryptor>,
) -> Result<(String, String), EncryptionError> {
    let mut md5_hasher = Md5::new();
    let sha256 = compute_upload_sha256(source, encryptor, |chunk| md5_hasher.update(chunk))?;
    Ok((sha256, BASE64_ENGINE.encode(md5_hasher.finalize())))
}

pub fn compute_sha256_digest(source: &ByteSource) -> Result<String, EncryptionError> {
    compute_upload_sha256(source, None, |_| {})
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
    fn upload_digests_match_sha256_and_md5_vectors() {
        let (sha256, md5) = compute_azure_blob_digests(
            &ByteSource::Bytes(bytes::Bytes::from_static(b"hello world")),
            None,
        )
        .unwrap();

        assert_eq!(sha256, "uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek=");
        assert_eq!(md5, "XrY7u+Ae7tCTyyK7j1rNww==");
    }

    #[test]
    fn upload_md5_matches_ciphertext_when_encrypted() {
        let plaintext = b"the quick brown fox jumps over the lazy dog";
        let material = test_material();
        let (encryptor, _metadata) = build_encryptor(&material, plaintext.len() as i64).unwrap();
        let ciphertext = encrypt_to_vec(&encryptor, plaintext);

        let (sha256, md5) = compute_azure_blob_digests(
            &ByteSource::Bytes(bytes::Bytes::from_static(plaintext)),
            Some(&encryptor),
        )
        .unwrap();

        assert_eq!(sha256, digest_of(plaintext));
        assert_eq!(md5, BASE64_ENGINE.encode(Md5::digest(&ciphertext)));
        assert_ne!(md5, BASE64_ENGINE.encode(Md5::digest(plaintext)));
    }

    #[test]
    fn upload_digests_consume_non_seekable_source_once() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };

        struct CountingReader {
            data: &'static [u8],
            offset: usize,
            bytes_read: Arc<AtomicUsize>,
        }

        impl Read for CountingReader {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let remaining = &self.data[self.offset..];
                let n = remaining.len().min(buf.len());
                buf[..n].copy_from_slice(&remaining[..n]);
                self.offset += n;
                self.bytes_read.fetch_add(n, Ordering::Relaxed);
                Ok(n)
            }
        }

        let bytes_read = Arc::new(AtomicUsize::new(0));
        let reader = CountingReader {
            data: b"one-way upload source",
            offset: 0,
            bytes_read: bytes_read.clone(),
        };
        let mut md5_hasher = Md5::new();
        let sha256 =
            compute_upload_sha256_from_reader(reader, None, |chunk| md5_hasher.update(chunk))
                .unwrap();
        let md5 = BASE64_ENGINE.encode(md5_hasher.finalize());

        assert_eq!(sha256, digest_of(b"one-way upload source"));
        assert_eq!(
            md5,
            BASE64_ENGINE.encode(Md5::digest(b"one-way upload source"))
        );
        assert_eq!(
            bytes_read.load(Ordering::Relaxed),
            b"one-way upload source".len()
        );
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
            Some(&digest),
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
            Some(&wrong_digest),
            &material,
            &mut output,
        );

        assert!(matches!(
            result,
            Err(EncryptionError::DigestMismatch { .. })
        ));
    }

    #[test]
    fn decrypt_succeeds_without_digest_when_absent() {
        let plaintext = b"payload with no digest header";
        let material = test_material();

        let (enc, metadata) = build_encryptor(&material, plaintext.len() as i64).unwrap();
        let ciphertext = encrypt_to_vec(&enc, plaintext);

        let mut decrypted = Vec::new();
        decrypt_ciphertext_to_writer(&ciphertext[..], &metadata, None, &material, &mut decrypted)
            .unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrapped_key_matches_material_accepts_real_cse_wrap() {
        let material = test_material();
        let (_enc, metadata) = build_encryptor(&material, 8).unwrap();
        assert!(wrapped_key_matches_material(&metadata, &material).unwrap());
    }

    #[test]
    fn wrapped_key_matches_material_rejects_placeholder_key() {
        let material = test_material();
        let metadata = EncryptedFileMetadata {
            encrypted_key: BASE64_ENGINE.encode(b"test-key"),
            iv: BASE64_ENGINE.encode([0u8; AES_BLOCK_SIZE_IN_BYTES]),
            material_desc: MaterialDescription {
                query_id: material.query_id.clone(),
                smk_id: material.smk_id.clone(),
                key_size: "256".to_string(),
            },
        };
        assert!(!wrapped_key_matches_material(&metadata, &material).unwrap());
    }

    #[test]
    fn wrapped_key_matches_material_rejects_mismatched_key_length() {
        let material = test_material();
        let master_key = BASE64_ENGINE
            .decode(material.query_stage_master_key.reveal())
            .unwrap();
        let short_key = [9u8; AES_128_KEY_SIZE_IN_BYTES];
        let wrapped = encrypt(Cipher::aes_256_ecb(), &master_key, None, &short_key).unwrap();
        let metadata = EncryptedFileMetadata {
            encrypted_key: BASE64_ENGINE.encode(wrapped),
            iv: BASE64_ENGINE.encode([0u8; AES_BLOCK_SIZE_IN_BYTES]),
            material_desc: MaterialDescription {
                query_id: material.query_id.clone(),
                smk_id: material.smk_id.clone(),
                key_size: "256".to_string(),
            },
        };
        assert!(!wrapped_key_matches_material(&metadata, &material).unwrap());
    }

    #[test]
    fn wrapped_key_matches_material_errors_when_cse_shaped_wrap_is_corrupt() {
        let material = test_material();
        let (_enc, mut metadata) = build_encryptor(&material, 8).unwrap();
        let mut wrapped = BASE64_ENGINE.decode(&metadata.encrypted_key).unwrap();
        let last = wrapped.len() - 1;
        wrapped[last] ^= 0xff;
        metadata.encrypted_key = BASE64_ENGINE.encode(wrapped);
        assert!(wrapped_key_matches_material(&metadata, &material).is_err());
    }
}
