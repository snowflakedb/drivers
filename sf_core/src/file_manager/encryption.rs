use super::types::{ByteSource, EncryptedFileMetadata, EncryptionMaterial, MaterialDescription};
use crate::sensitive::Sensitive;
use snafu::{Location, ResultExt, Snafu, ensure};

use aws_lc_rs::cipher::{
    AES_128, AES_256, Algorithm, DecryptionContext, EncryptionContext, PaddedBlockDecryptingKey,
    PaddedBlockEncryptingKey, StreamingDecryptingKey, StreamingEncryptingKey, UnboundCipherKey,
};
use aws_lc_rs::digest;
use aws_lc_rs::iv::{FixedLength, IV_LEN_128_BIT};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_ENGINE};
use md5::{Digest as _, Md5};
use std::io::{Read, Write};

// Cryptographic constants
const AES_256_KEY_SIZE_IN_BYTES: usize = 32; // 256 bits
const AES_128_KEY_SIZE_IN_BYTES: usize = 16; // 128 bits
const AES_BLOCK_SIZE_IN_BYTES: usize = 16; // 128-bit block size for AES

const CRYPT_CHUNK_SIZE: usize = 64 * 1024;

/// The AES algorithm selected by the master key's length. CBC and ECB share
/// one [`Algorithm`]; the mode comes from the key constructor
/// (`*_cbc_pkcs7` / `*_ecb_pkcs7`) rather than from a second value.
struct CipherSuite {
    key_len: usize,
    algorithm: &'static Algorithm,
}

impl CipherSuite {
    fn from_key_len(key_len: usize) -> Result<Self, EncryptionError> {
        match key_len {
            AES_128_KEY_SIZE_IN_BYTES => Ok(Self {
                key_len,
                algorithm: &AES_128,
            }),
            AES_256_KEY_SIZE_IN_BYTES => Ok(Self {
                key_len,
                algorithm: &AES_256,
            }),
            _ => UnsupportedKeySizeSnafu { key_size: key_len }.fail(),
        }
    }

    fn unbound_key(&self, key: &[u8]) -> Result<UnboundCipherKey, EncryptionError> {
        UnboundCipherKey::new(self.algorithm, key).context(CryptoSnafu {
            operation: "binding AES key",
        })
    }
}

/// The stage IV as the fixed-width type AWS-LC's cipher contexts require.
fn iv_context(iv: &[u8]) -> Result<EncryptionContext, EncryptionError> {
    let iv: [u8; IV_LEN_128_BIT] = iv
        .try_into()
        .map_err(|_| InvalidIvLengthSnafu { actual: iv.len() }.build())?;
    Ok(EncryptionContext::Iv128(FixedLength::from(iv)))
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
        let unbound = cipher_suite.unbound_key(self.file_key.reveal())?;
        // `less_safe_` because the IV is ours: it is generated once in
        // `build_encryptor`, travels in the stage metadata, and must be reused
        // verbatim on every retry so re-encryption is byte-identical. Letting
        // AWS-LC generate a fresh IV here would break that determinism and the
        // plaintext digest along with it.
        let key = StreamingEncryptingKey::less_safe_cbc_pkcs7(unbound, iv_context(&self.iv)?)
            .context(CryptoSnafu {
                operation: "initializing AES-CBC encryptor",
            })?;
        Ok(EncryptingReader {
            source,
            key: Some(key),
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

    let file_key = generate_random_bytes(cipher_suite.key_len)?;
    let iv = generate_random_bytes(AES_BLOCK_SIZE_IN_BYTES)?;

    let encoded_file_key = wrap_file_key(&cipher_suite, &master_key, &file_key)?;

    let metadata = EncryptedFileMetadata {
        encrypted_key: encoded_file_key,
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

fn wrap_file_key(
    cipher_suite: &CipherSuite,
    master_key: &[u8],
    file_key: &[u8],
) -> Result<String, EncryptionError> {
    // The stage metadata requires a full PKCS#7 block even for aligned keys.
    let mut wrapped = file_key.to_vec();
    PaddedBlockEncryptingKey::ecb_pkcs7(cipher_suite.unbound_key(master_key)?)
        .context(CryptoSnafu {
            operation: "initializing AES-ECB key wrap",
        })?
        .encrypt(&mut wrapped)
        .context(CryptoSnafu {
            operation: "encrypting file key with AES-ECB",
        })?;
    Ok(BASE64_ENGINE.encode(wrapped))
}

/// Streaming AES-CBC/PKCS#7 encryptor over an arbitrary `Read` source. Each
/// `read` drains previously-produced ciphertext, then encrypts the next
/// `CRYPT_CHUNK_SIZE` plaintext chunk (or emits the final padded block at EOF).
/// Peak resident memory is `~CRYPT_CHUNK_SIZE`, independent of file size.
pub struct EncryptingReader<R: Read> {
    source: R,
    /// Taken at finalize time: `StreamingEncryptingKey::finish` consumes the
    /// key, so it cannot live behind `&mut self`.
    key: Option<StreamingEncryptingKey>,
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
                let key = self
                    .key
                    .take()
                    .ok_or_else(|| std::io::Error::other("AES-CBC encryptor already finalized"))?;
                self.staged.resize(AES_BLOCK_SIZE_IN_BYTES, 0);
                // Scoped so the `BufferUpdate` borrow of `staged` ends before
                // the truncate below.
                let written = {
                    let (_ctx, out) = key
                        .finish(&mut self.staged)
                        .map_err(std::io::Error::other)?;
                    out.written().len()
                };
                self.staged.truncate(written);
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
            // AWS-LC requires `input.len() + block_len - 1`; a whole extra
            // block is simpler and always sufficient.
            self.staged.resize(n + AES_BLOCK_SIZE_IN_BYTES, 0);
            let written = {
                let key = self
                    .key
                    .as_mut()
                    .ok_or_else(|| std::io::Error::other("AES-CBC encryptor already finalized"))?;
                let out = key
                    .update(&self.chunk[..n], &mut self.staged)
                    .map_err(std::io::Error::other)?;
                out.written().len()
            };
            self.staged.truncate(written);
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

    // Unwrapped in place: AWS-LC decrypts into `wrapped` and hands back the
    // unpadded prefix, so truncating to that length turns the buffer itself
    // into the file key. Copying it out instead would leave a second live
    // copy of the key sitting in `wrapped` until the function returns.
    let mut wrapped = encrypted_file_key;
    let file_key_len = PaddedBlockDecryptingKey::ecb_pkcs7(cipher_suite.unbound_key(&master_key)?)
        .context(CryptoSnafu {
            operation: "initializing AES-ECB key unwrap",
        })?
        .decrypt(&mut wrapped, DecryptionContext::None)
        .context(CryptoSnafu {
            operation: "decrypting file key with AES-ECB",
        })?
        .len();
    wrapped.truncate(file_key_len);
    let file_key = wrapped;
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

    // `unwrap_file_key` has already checked that the unwrapped key matches
    // this suite's width, so it describes the file key as well as the master.
    let mut key = StreamingDecryptingKey::cbc_pkcs7(
        cipher_suite.unbound_key(&file_key)?,
        iv_context(&iv)?.into(),
    )
    .context(CryptoSnafu {
        operation: "initializing AES-CBC decryptor",
    })?;

    // The digest stored on upload is the SHA-256 of the (compressed) plaintext,
    // not the ciphertext, so verification hashes the decrypted output.
    // Built only when there is a digest to check it against: an S3 CSE
    // download can arrive with no `sfc-digest` header at all (SNOW-4073008),
    // and hashing the whole plaintext to discard the result is pure cost.
    let mut hasher = digest.map(|_| digest::Context::new(&digest::SHA256));

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
        let out = key
            .update(&cipher_buf[..n], &mut plain_buf)
            .context(CryptoSnafu {
                operation: "decrypting data chunk with AES-CBC",
            })?;
        let plaintext = out.written();
        if !plaintext.is_empty() {
            if let Some(hasher) = hasher.as_mut() {
                hasher.update(plaintext);
            }
            output_byte_len += plaintext.len() as i64;
            output.write_all(plaintext).context(IoSnafu {
                operation: "writing decrypted chunk to output",
            })?;
        }
    }

    let out = key.finish(&mut plain_buf).context(CryptoSnafu {
        operation: "finalizing AES-CBC decryption",
    })?;
    let tail = out.written();
    if !tail.is_empty() {
        if let Some(hasher) = hasher.as_mut() {
            hasher.update(tail);
        }
        output_byte_len += tail.len() as i64;
        output.write_all(tail).context(IoSnafu {
            operation: "writing final decrypted block",
        })?;
    }

    if let (Some(expected), Some(hasher)) = (digest, hasher) {
        let computed = BASE64_ENGINE.encode(hasher.finish().as_ref());
        if computed != expected {
            return DigestMismatchSnafu.fail();
        }
    }

    Ok(output_byte_len)
}

/// Generates a vector of random bytes of a specified size.
fn generate_random_bytes(size: usize) -> Result<Vec<u8>, EncryptionError> {
    let mut buffer = vec![0; size];
    aws_lc_rs::rand::fill(&mut buffer).context(CryptoSnafu {
        operation: "generating random bytes",
    })?;
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
        hasher: &'a mut digest::Context,
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

    let mut sha256_hasher = digest::Context::new(&digest::SHA256);
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
    #[snafu(display("AWS-LC cryptographic operation failed during {operation}"))]
    Crypto {
        operation: String,
        source: aws_lc_rs::error::Unspecified,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "AES-CBC requires a {IV_LEN_128_BIT}-byte initialization vector, got {actual} bytes"
    ))]
    InvalidIvLength {
        actual: usize,
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

    fn vector_encryptor(len: usize) -> Encryptor {
        Encryptor {
            file_key: Sensitive::from((0u8..32).collect::<Vec<u8>>()),
            iv: (0u8..16).collect(),
            cipher_len: cbc_ciphertext_len(len as i64),
        }
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

    /// Fixed AES-256-CBC/PKCS#7 stage ciphertext vectors generated with
    /// OpenSSL 3.5.3 (`enc -aes-256-cbc -K 000102...1f -iv 000102...0f -nosalt`).
    /// The key, IV, and plaintext are the sequential bytes in the test.
    #[test]
    fn stage_ciphertext_matches_reference_vectors() {
        for (len, expected) in [
            (0, "e9c3ef8ab23453e6f0749cd636e7a88e"),
            (1, "cab6bf3990d9768d8e1262e8adcda55f"),
            (15, "62b5e1438a1f9d3523ac06b82b425cab"),
            (
                16,
                "f29000b62a499fd0a9f39a6add2e778053c8742d0ea29b2712f6c7af4048f4b4",
            ),
            (
                17,
                "f29000b62a499fd0a9f39a6add2e7780ae2c7b7cc6473eb497cabbd193c6174e",
            ),
        ] {
            let plaintext: Vec<u8> = (0..len).map(|n| n as u8).collect();
            let encryptor = vector_encryptor(len);
            let ciphertext = encrypt_to_vec(&encryptor, &plaintext);
            assert_eq!(hex::encode(&ciphertext), expected, "length {len}");
            assert_eq!(encryptor.cipher_len(), ciphertext.len() as i64);
        }
    }

    /// The fixed ciphertext SHA-256s were computed over OpenSSL 3.5.3's
    /// AES-256-CBC/PKCS#7 output for the same sequential key, IV, and plaintext.
    /// These lengths cross the reader's chunk and final-padding boundaries.
    #[test]
    fn encrypting_reader_preserves_stage_ciphertext_across_chunk_boundaries() {
        for (len, expected_sha256) in [
            (
                CRYPT_CHUNK_SIZE - 1,
                "b6e414dc3eb59038b86ae65a7405e7ccf237e3881c7c0f50c09f009c4eaaa406",
            ),
            (
                CRYPT_CHUNK_SIZE,
                "fc615aa8caf22bf00c92bd6a8bae347619412c1297663832b4f4604e7b5591d2",
            ),
            (
                CRYPT_CHUNK_SIZE + 1,
                "5207e5464f3e27916e5afed98544a2ee73f18e40ce32c95862cd41be09cf4cd1",
            ),
        ] {
            let plaintext: Vec<u8> = (0..len).map(|n| n as u8).collect();
            let encryptor = vector_encryptor(len);
            let ciphertext = encrypt_to_vec(&encryptor, &plaintext);
            assert_eq!(
                hex::encode(digest::digest(&digest::SHA256, &ciphertext).as_ref()),
                expected_sha256,
                "length {len}"
            );
            assert_eq!(encryptor.cipher_len(), ciphertext.len() as i64);
            assert_eq!(encrypt_to_vec(&encryptor, &plaintext), ciphertext);
        }
    }

    /// Small reads exercise the staged ciphertext's partial-drain branch.
    #[test]
    fn encrypting_reader_partial_reads_into_small_buffer() {
        let plaintext: Vec<u8> = (0..CRYPT_CHUNK_SIZE + 100).map(|n| n as u8).collect();
        let encryptor = vector_encryptor(plaintext.len());
        let mut reader = encryptor
            .encrypting_reader(std::io::Cursor::new(plaintext.clone()))
            .unwrap();
        let mut out = Vec::new();
        let mut buf = [0u8; 7];
        loop {
            let n = reader.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n]);
        }

        assert_eq!(
            hex::encode(digest::digest(&digest::SHA256, &out).as_ref()),
            "8faf1df050a6d959f7b87682fe12c825d114eab613a1de2b91e94ca0523a9351"
        );
        assert_eq!(out, encrypt_to_vec(&encryptor, &plaintext));
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
        // AES-256-ECB/PKCS#7 wrap of `[9; 16]` under the `[7; 32]` master key.
        let wrapped =
            hex::decode("07b9f822f532dc5fde60cff773c193d094f551ac1fc273d74d84fc638b2a9ef8")
                .unwrap();
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

    /// A block-aligned file key gains a whole PKCS#7 padding block in this
    /// AES-256-ECB wrap. OpenSSL 3.5.3 reference: key `[7; 32]`, input `[9; 32]`.
    #[test]
    fn wrapped_file_key_matches_reference_vector() {
        let master_key = [7u8; AES_256_KEY_SIZE_IN_BYTES];
        let file_key = [9u8; AES_256_KEY_SIZE_IN_BYTES];
        let cipher_suite = CipherSuite::from_key_len(master_key.len()).unwrap();
        let encoded = wrap_file_key(&cipher_suite, &master_key, &file_key).unwrap();

        assert_eq!(
            encoded,
            "B7n4IvUy3F/eYM/3c8GT0Ae5+CL1Mtxf3mDP93PBk9CU9VGsH8Jz102E/GOLKp74"
        );
    }

    /// The randomly generated stage file key is block-aligned, but its
    /// metadata wrap must include a full PKCS#7 block for other drivers.
    #[test]
    fn wrapped_file_key_in_metadata_has_full_padding_block() {
        let material = test_material();
        let (encryptor, metadata) = build_encryptor(&material, 0).unwrap();
        let wrapped = BASE64_ENGINE.decode(&metadata.encrypted_key).unwrap();

        assert_eq!(
            wrapped.len(),
            encryptor.file_key.reveal().len() + AES_BLOCK_SIZE_IN_BYTES,
            "PKCS#7 must add a full padding block to a block-multiple key"
        );
    }

    /// An IV of the wrong width is rejected rather than silently truncated or
    /// zero-extended, either of which would produce ciphertext no other driver
    /// could decrypt.
    #[test]
    fn wrong_length_iv_is_rejected() {
        let material = test_material();
        let (enc, _meta) = build_encryptor(&material, 0).unwrap();
        let short = Encryptor {
            file_key: enc.file_key.clone(),
            iv: vec![0u8; AES_BLOCK_SIZE_IN_BYTES - 1],
            cipher_len: enc.cipher_len(),
        };
        assert!(matches!(
            short.encrypting_reader(std::io::Cursor::new(Vec::new())),
            Err(EncryptionError::InvalidIvLength { .. })
        ));
    }
}
