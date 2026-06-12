use sf_core::apis::database_driver_v1::PutGetResultsetFlavor;
use sf_core::config::param_registry::DEFAULT_PUT_GET_MAX_ATTEMPTS;
use sf_core::file_manager::types::{
    ByteSource, CloudCredentials, EncryptionMaterial, LocationType, SingleDownloadData, StageInfo,
};
use sf_core::sensitive::SensitiveString;

fn test_encryption_material() -> EncryptionMaterial {
    use base64::Engine;
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
    let bad_digest = "AAAA";

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
            Err(sf_core::file_manager::internal::EncryptionError::DigestMismatch { .. })
        ),
        "tampered digest must yield DigestMismatch, got: {result:?}",
    );
}

#[test]
fn path_source_encrypt_decrypt_roundtrip() {
    use std::io::Write;

    // Multi-chunk payload to exercise the streaming Crypter path with a real file.
    let plaintext: Vec<u8> = (0..100 * 1024).map(|i| (i % 251) as u8).collect();
    let material = test_encryption_material();

    let dir = tempfile::tempdir().expect("tempdir");
    let plaintext_path = dir.path().join("plain.bin");
    {
        let mut f = std::fs::File::create(&plaintext_path).expect("create plaintext");
        f.write_all(&plaintext).expect("write plaintext");
    }

    let prepared = sf_core::file_manager::internal::encrypt_file_data(
        ByteSource::Path(plaintext_path.clone()),
        &material,
    )
    .expect("encryption from Path must succeed");

    let ciphertext = match prepared.data {
        ByteSource::Bytes(ref b) => b.clone(),
        ByteSource::Path(_) => panic!("expected ByteSource::Bytes from encrypt_file_data"),
    };

    let enc_meta = prepared.encryption_metadata.as_ref().unwrap();

    // Decrypt directly into an output file (the production GET pattern).
    let output_path = dir.path().join("decrypted.bin");
    let mut output_file = std::fs::File::create(&output_path).expect("create output");
    let written = sf_core::file_manager::internal::decrypt_ciphertext_to_writer(
        ciphertext.as_slice(),
        enc_meta,
        &prepared.digest,
        &material,
        &mut output_file,
    )
    .expect("decryption must succeed");
    drop(output_file);

    assert_eq!(written, plaintext.len() as i64);
    let on_disk = std::fs::read(&output_path).expect("read decrypted");
    assert_eq!(on_disk, plaintext, "round-tripped file must match original");
}

// End-to-end atomic-rename contract: when `download_single_file` decrypts a
// file whose `sfc-digest` header doesn't match the ciphertext SHA-256, the
// final output path must NOT exist on disk. This pins the guarantee that the
// `.part` + `rename` pattern added in this PR actually prevents a partial
// plaintext from appearing at the user-visible destination on failure.
#[tokio::test(flavor = "multi_thread")]
async fn download_single_file_tampered_digest_leaves_no_output() {
    use sf_core::file_manager::FileManagerError;
    use sf_core::file_manager::internal::EncryptionError;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let plaintext = b"download tampered-digest test payload".to_vec();
    let material = test_encryption_material();

    // Encrypt to get valid ciphertext + the matching enc-metadata headers.
    let prepared =
        sf_core::file_manager::internal::encrypt_file_data(ByteSource::Bytes(plaintext), &material)
            .expect("encryption must succeed");

    let ciphertext = match prepared.data {
        ByteSource::Bytes(ref b) => b.clone(),
        ByteSource::Path(_) => panic!("expected Bytes from encrypt_file_data"),
    };
    let enc_meta = prepared.encryption_metadata.as_ref().unwrap();
    let mat_desc_json = serde_json::to_string(&enc_meta.material_desc).unwrap();

    // Mock S3: return the valid ciphertext but with a deliberately wrong digest.
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-amz-meta-sfc-digest", "BAADBAADBAADBAAD")
                .insert_header("x-amz-meta-x-amz-matdesc", mat_desc_json.as_str())
                .insert_header("x-amz-meta-x-amz-key", enc_meta.encrypted_key.as_str())
                .insert_header("x-amz-meta-x-amz-iv", enc_meta.iv.as_str())
                .set_body_bytes(ciphertext),
        )
        .mount(&mock_server)
        .await;

    let output_dir = tempfile::tempdir().unwrap();
    let src_location = "test_file.bin";
    let data = SingleDownloadData {
        src_location: src_location.to_string(),
        local_location: output_dir.path().to_str().unwrap().to_string(),
        stage_info: StageInfo {
            location_type: LocationType::S3,
            bucket: "test-bucket".to_string(),
            key_prefix: "".to_string(),
            region: "us-east-1".to_string(),
            creds: CloudCredentials::S3 {
                aws_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
                aws_secret_key: SensitiveString::from("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"),
                aws_token: SensitiveString::from(""),
            },
            endpoint: Some(mock_server.uri()),
            presigned_url: None,
            use_virtual_url: false,
            use_regional_url: false,
            use_s3_regional_url: false,
            storage_account: None,
        },
        encryption_material: Some(material),
        // GCS-only; ignored by the S3 download branch exercised here.
        presigned_url: None,
        flavor: PutGetResultsetFlavor::Python,
    };

    let result = sf_core::file_manager::download_single_file(
        data,
        DEFAULT_PUT_GET_MAX_ATTEMPTS,
        0,
        &mut None,
    )
    .await;

    assert!(
        matches!(
            result,
            Err(FileManagerError::Decryption {
                source: EncryptionError::DigestMismatch { .. },
                ..
            })
        ),
        "tampered digest must yield Decryption(DigestMismatch), got: {result:?}",
    );

    // The atomic rename must not have fired — no output at the user-visible path.
    let output_path = output_dir.path().join(src_location);
    assert!(
        !output_path.exists(),
        "output file must NOT exist after DigestMismatch: {output_path:?}",
    );
}
