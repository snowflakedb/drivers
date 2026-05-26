//! Round-trip test for the `ByteSource::Bytes` upload/download path.
//!
//! This is the foundation that PR-3 (JDBC `uploadStream`) and PR-4
//! (Python `file_stream`) will build their wrapper tests on. It validates:
//!
//! 1. `encrypt_file_data` accepts `ByteSource::Bytes` and produces a
//!    `PreparedUpload` whose `data` is also `ByteSource::Bytes`.
//! 2. `decrypt_ciphertext_to_writer` fully round-trips the ciphertext back to
//!    the original plaintext.
//! 3. The SHA-256 digest in the `PreparedUpload` matches what
//!    `decrypt_ciphertext_to_writer` verifies internally.
//! 4. A tampered digest causes `DigestMismatch` — *after* some plaintext has
//!    been written (behavioral-change note from the streaming refactor).

use sf_core::file_manager::types::{ByteSource, EncryptionMaterial};
use sf_core::sensitive::SensitiveString;

/// Builds minimal `EncryptionMaterial` for testing. Uses a fixed 32-byte
/// (256-bit) master key so the test is deterministic and exercises AES-256-CBC.
fn test_encryption_material() -> EncryptionMaterial {
    use base64::Engine;
    // 32 zero bytes, base64-encoded.
    let master_key_b64 = base64::engine::general_purpose::STANDARD.encode([0u8; 32]);
    EncryptionMaterial {
        query_stage_master_key: SensitiveString::from(master_key_b64),
        query_id: "test-query-id".to_string(),
        smk_id: "42".to_string(),
    }
}

#[test]
fn bytes_source_encrypt_decrypt_roundtrip() {
    let plaintext = b"Hello, ByteSource::Bytes round-trip test!".to_vec();
    let material = test_encryption_material();

    // Encrypt via the ByteSource::Bytes path.
    let prepared = sf_core::file_manager::internal::encrypt_file_data(
        ByteSource::Bytes(plaintext.clone()),
        &material,
    )
    .expect("encryption must succeed");

    // The output must be a Bytes variant (not a Path).
    let ciphertext = match prepared.data {
        ByteSource::Bytes(ref b) => b.clone(),
        ByteSource::Path(_) => panic!("expected ByteSource::Bytes from encrypt_file_data"),
    };

    // Ciphertext must be non-empty and different from plaintext.
    assert!(!ciphertext.is_empty(), "ciphertext must not be empty");
    assert_ne!(
        ciphertext, plaintext,
        "ciphertext must differ from plaintext"
    );

    // Metadata must be present (client-side encryption).
    let enc_meta = prepared
        .encryption_metadata
        .as_ref()
        .expect("encryption metadata present");

    // Decrypt back to plaintext via the streaming writer.
    let mut output = Vec::<u8>::new();
    let written = sf_core::file_manager::internal::decrypt_ciphertext_to_writer(
        ciphertext.as_slice(),
        enc_meta,
        &prepared.digest,
        &material,
        &mut output,
    )
    .expect("decryption must succeed");

    assert_eq!(written, plaintext.len() as i64, "byte count must match");
    assert_eq!(output, plaintext, "decrypted content must match original");
}

#[test]
fn bytes_source_encrypt_decrypt_large_payload() {
    // 256 KiB of repeating data — exercises multiple CRYPT_CHUNK_SIZE (64 KiB)
    // iterations in the streaming Crypter loop.
    let plaintext: Vec<u8> = (0..256 * 1024).map(|i| (i % 251) as u8).collect();
    let material = test_encryption_material();

    let prepared = sf_core::file_manager::internal::encrypt_file_data(
        ByteSource::Bytes(plaintext.clone()),
        &material,
    )
    .expect("encryption must succeed");

    let ciphertext = match prepared.data {
        ByteSource::Bytes(ref b) => b.clone(),
        ByteSource::Path(_) => panic!("expected ByteSource::Bytes"),
    };

    let enc_meta = prepared.encryption_metadata.as_ref().unwrap();
    let mut output = Vec::<u8>::new();
    let written = sf_core::file_manager::internal::decrypt_ciphertext_to_writer(
        ciphertext.as_slice(),
        enc_meta,
        &prepared.digest,
        &material,
        &mut output,
    )
    .expect("decryption must succeed");

    assert_eq!(written, plaintext.len() as i64);
    assert_eq!(output, plaintext);
}

#[test]
fn bytes_source_decrypt_detects_tampered_digest() {
    let plaintext = b"tampered digest test".to_vec();
    let material = test_encryption_material();

    let prepared = sf_core::file_manager::internal::encrypt_file_data(
        ByteSource::Bytes(plaintext.clone()),
        &material,
    )
    .expect("encryption must succeed");

    let ciphertext = match prepared.data {
        ByteSource::Bytes(ref b) => b.clone(),
        ByteSource::Path(_) => panic!("expected ByteSource::Bytes"),
    };

    let enc_meta = prepared.encryption_metadata.as_ref().unwrap();
    let bad_digest = "AAAA"; // wrong digest

    let mut output = Vec::<u8>::new();
    let result = sf_core::file_manager::internal::decrypt_ciphertext_to_writer(
        ciphertext.as_slice(),
        enc_meta,
        bad_digest,
        &material,
        &mut output,
    );

    assert!(
        matches!(
            result,
            Err(sf_core::file_manager::encryption::EncryptionError::DigestMismatch { .. })
        ),
        "tampered digest must yield DigestMismatch, got: {result:?}",
    );
}
