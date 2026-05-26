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

// ---------------------------------------------------------------------------
// PR-2 GCS streaming round-trip: encrypt → mock GCS server → streaming decrypt
// ---------------------------------------------------------------------------

/// Tests the full GCS streaming download path:
/// encrypt with ByteSource::Bytes → serve ciphertext from a mock GCS server →
/// `download_from_gcs_streaming` + `decrypt_ciphertext_to_writer` →
/// plaintext matches the original.
///
/// This exercises the `mpsc::sync_channel` bridge (GcsStreamReader) between
/// the async reqwest body stream and the sync AES-CBC decryptor.
#[tokio::test]
async fn gcs_streaming_bytes_source_encrypt_decrypt_roundtrip() {
    use sf_core::file_manager::types::{
        ByteSource, CloudCredentials, EncryptedFileMetadata, LocationType, StageInfo,
    };
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    // --- 1. Encrypt a small plaintext ---
    let plaintext = b"GCS streaming round-trip test payload, PR-2!".to_vec();
    let material = test_encryption_material();

    let prepared = sf_core::file_manager::internal::encrypt_file_data(
        ByteSource::Bytes(plaintext.clone()),
        &material,
    )
    .expect("encryption must succeed");

    let ciphertext = match &prepared.data {
        ByteSource::Bytes(b) => b.clone(),
        ByteSource::Path(_) => panic!("expected Bytes from encrypt_file_data"),
    };
    let enc_meta = prepared.encryption_metadata.as_ref().unwrap();
    let digest = &prepared.digest;

    // --- 2. Start a mock GCS server that serves the ciphertext ---
    let server = MockServer::start().await;

    let enc_data_json = serde_json::json!({
        "EncryptionMode": "FullBlob",
        "WrappedContentKey": {
            "KeyId": "symmKey1",
            "EncryptedKey": enc_meta.encrypted_key,
            "Algorithm": "AES_CBC_256"
        },
        "EncryptionAgent": {
            "Protocol": "1.0",
            "EncryptionAlgorithm": "AES_CBC_256"
        },
        "ContentEncryptionIV": enc_meta.iv,
        "KeyWrappingMetadata": {
            "EncryptionLibrary": "Rust(OpenSSL)"
        }
    });
    let mat_desc_json = serde_json::json!({
        "queryId": enc_meta.material_desc.query_id,
        "smkId":   enc_meta.material_desc.smk_id,
        "keySize": "256"
    });

    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(ciphertext)
                .insert_header("x-goog-meta-sfc-digest", digest.as_str())
                .insert_header(
                    "x-goog-meta-encryptiondata",
                    enc_data_json.to_string().as_str(),
                )
                .insert_header("x-goog-meta-matdesc", mat_desc_json.to_string().as_str()),
        )
        .mount(&server)
        .await;

    // --- 3. Download via streaming path ---
    let presigned_url = format!("{}/gcs-object", server.uri());
    let stage = StageInfo {
        location_type: LocationType::Gcs,
        bucket: "test-bucket".to_string(),
        key_prefix: "".to_string(),
        region: "us-central1".to_string(),
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        endpoint: None,
        presigned_url: Some(presigned_url),
        use_virtual_url: false,
        use_regional_url: false,
        use_s3_regional_url: false,
        storage_account: None,
    };

    let dl = sf_core::file_manager::internal::download_from_gcs_streaming(&stage, "gcs-object")
        .await
        .expect("streaming download must succeed");

    let file_metadata: EncryptedFileMetadata =
        dl.file_metadata.expect("enc metadata must be present");
    let dl_digest = dl.digest.expect("digest must be present");
    let reader = dl.reader;
    let mat_clone = material.clone();

    // --- 4. Decrypt in spawn_blocking (mirrors mod.rs) ---
    let decrypted = tokio::task::spawn_blocking(move || -> Vec<u8> {
        let mut output = Vec::new();
        sf_core::file_manager::internal::decrypt_ciphertext_to_writer(
            reader,
            &file_metadata,
            &dl_digest,
            &mat_clone,
            &mut output,
        )
        .expect("decryption must succeed");
        output
    })
    .await
    .expect("spawn_blocking must complete");

    // --- 5. Assert round-trip ---
    assert_eq!(
        decrypted, plaintext,
        "GCS streaming decrypt must reproduce the original plaintext"
    );
}
