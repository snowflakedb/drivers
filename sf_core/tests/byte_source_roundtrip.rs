#[path = "common/mod.rs"]
mod common;

use sf_core::apis::database_driver_v1::PutGetResultsetFlavor;
use sf_core::config::param_registry::DEFAULT_PUT_GET_MAX_ATTEMPTS;
use sf_core::config::param_store::ParamStore;
use sf_core::config::retry::RetryPolicy;
use sf_core::file_manager::MultipartParams;
use sf_core::file_manager::types::{
    ByteSource, CloudCredentials, EncryptedFileMetadata, EncryptionMaterial, LocationType,
    SingleDownloadData, StageInfo,
};
use sf_core::sensitive::SensitiveString;
use std::io::Read;

fn test_encryption_material() -> EncryptionMaterial {
    use base64::Engine;
    let master_key_b64 = base64::engine::general_purpose::STANDARD.encode([0u8; 32]);
    EncryptionMaterial {
        query_stage_master_key: SensitiveString::from(master_key_b64),
        query_id: "test-query-id".to_string(),
        smk_id: "42".to_string(),
    }
}

/// Encrypts `source` through the production lazy path (`build_encryptor` +
/// `EncryptingReader`) and collects the ciphertext for the test. Returns the
/// ciphertext, the cloud encryption metadata, and the `sfc-digest` (computed
/// over the pre-encryption source, matching JDBC/ODBC).
fn encrypt_source(
    source: ByteSource,
    material: &EncryptionMaterial,
) -> (Vec<u8>, EncryptedFileMetadata, String) {
    use sf_core::file_manager::internal::{build_encryptor, compute_sha256_digest};

    let source_len = match &source {
        ByteSource::Bytes(b) => b.len() as i64,
        ByteSource::Path(p) => std::fs::metadata(p).expect("source metadata").len() as i64,
    };
    let digest = compute_sha256_digest(&source).expect("digest over source");
    let (encryptor, metadata) = build_encryptor(material, source_len).expect("build_encryptor");

    let reader: Box<dyn Read + Send> = match source {
        ByteSource::Bytes(b) => Box::new(std::io::Cursor::new(b)),
        ByteSource::Path(p) => Box::new(std::fs::File::open(p).expect("open source")),
    };
    let mut ciphertext = Vec::new();
    encryptor
        .encrypting_reader(reader)
        .expect("encrypting_reader")
        .read_to_end(&mut ciphertext)
        .expect("read ciphertext");

    (ciphertext, metadata, digest)
}

#[test]
fn bytes_source_encrypt_decrypt_roundtrip() {
    let plaintext = b"Hello, ByteSource::Bytes round-trip test!".to_vec();
    let material = test_encryption_material();

    let (ciphertext, enc_meta, digest) =
        encrypt_source(ByteSource::Bytes(plaintext.clone().into()), &material);

    // Ciphertext must be non-empty and different from plaintext.
    assert!(!ciphertext.is_empty(), "ciphertext must not be empty");
    assert_ne!(
        ciphertext, plaintext,
        "ciphertext must differ from plaintext"
    );

    // Decrypt back to plaintext via the streaming writer.
    let mut output = Vec::<u8>::new();
    let written = sf_core::file_manager::internal::decrypt_ciphertext_to_writer(
        ciphertext.as_slice(),
        &enc_meta,
        &digest,
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

    let (ciphertext, enc_meta, digest) =
        encrypt_source(ByteSource::Bytes(plaintext.clone().into()), &material);

    let mut output = Vec::<u8>::new();
    let written = sf_core::file_manager::internal::decrypt_ciphertext_to_writer(
        ciphertext.as_slice(),
        &enc_meta,
        &digest,
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

    let (ciphertext, enc_meta, _digest) =
        encrypt_source(ByteSource::Bytes(plaintext.into()), &material);
    let bad_digest = "AAAA";

    let mut output = Vec::<u8>::new();
    let result = sf_core::file_manager::internal::decrypt_ciphertext_to_writer(
        ciphertext.as_slice(),
        &enc_meta,
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

    let (ciphertext, enc_meta, digest) =
        encrypt_source(ByteSource::Path(plaintext_path.clone()), &material);

    // Decrypt directly into an output file (the production GET pattern).
    let output_path = dir.path().join("decrypted.bin");
    let mut output_file = std::fs::File::create(&output_path).expect("create output");
    let written = sf_core::file_manager::internal::decrypt_ciphertext_to_writer(
        ciphertext.as_slice(),
        &enc_meta,
        &digest,
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
    let (ciphertext, enc_meta, _digest) =
        encrypt_source(ByteSource::Bytes(plaintext.into()), &material);
    let mat_desc_json = serde_json::to_string(&enc_meta.material_desc).unwrap();

    // Mock S3: return the valid ciphertext but with a deliberately wrong digest.
    // S3 HEADs first (for size + metadata) then GETs the body; the tampered
    // digest rides on both so the decrypt step sees it.
    let mock_server = MockServer::start().await;
    let cipher_len = ciphertext.len();
    Mock::given(method("HEAD"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-length", cipher_len.to_string())
                .insert_header("x-amz-meta-sfc-digest", "BAADBAADBAADBAAD")
                .insert_header("x-amz-meta-x-amz-matdesc", mat_desc_json.as_str())
                .insert_header("x-amz-meta-x-amz-key", enc_meta.encrypted_key.as_str())
                .insert_header("x-amz-meta-x-amz-iv", enc_meta.iv.as_str()),
        )
        .mount(&mock_server)
        .await;
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
            tls_config: sf_core::tls::config::TlsConfig::default(),
            crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
            storage_account: None,
        },
        encryption_material: Some(material),
        // GCS-only; ignored by the S3 download branch exercised here.
        presigned_url: None,
        flavor: PutGetResultsetFlavor::Python,
        multipart: MultipartParams::default(),
        unsafe_file_write: false,
    };

    let result = sf_core::file_manager::download_single_file(
        data,
        &RetryPolicy::put_get(&ParamStore::new()),
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

// ---------------------------------------------------------------------------
// PR-2 streaming round-trip: encrypt → mock cloud server → streaming decrypt
//
// Both GCS and Azure go through `cloud_http::spawn_byte_stream_producer` and
// the same sync-channel bridge into the sync decryptor. The wire-level
// metadata-header names differ, but the body bytes round-trip identically.
// We share one fixture and run it twice, once per `Cloud` flavour, so an
// Azure regression in this layer can't masquerade as "the GCS test still
// passes".
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum Cloud {
    Gcs,
    Azure,
}

impl Cloud {
    /// (digest header, encryption-data header, mat-desc header).
    fn meta_headers(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Cloud::Gcs => (
                "x-goog-meta-sfc-digest",
                "x-goog-meta-encryptiondata",
                "x-goog-meta-matdesc",
            ),
            Cloud::Azure => (
                "x-ms-meta-sfcdigest",
                "x-ms-meta-encryptiondata",
                "x-ms-meta-matdesc",
            ),
        }
    }
}

/// Encrypt → serve from a mock cloud HTTP server (GCS or Azure) → stream
/// download through the sync-`Read` bridge → decrypt → assert plaintext.
async fn streaming_roundtrip_for(cloud: Cloud) {
    use sf_core::file_manager::types::{ByteSource, CloudCredentials, LocationType, StageInfo};
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    // --- 1. Encrypt a small plaintext ---
    let plaintext = format!("{cloud:?} streaming round-trip test payload, PR-2!").into_bytes();
    let material = test_encryption_material();

    let (ciphertext, enc_meta, digest) =
        encrypt_source(ByteSource::Bytes(plaintext.clone().into()), &material);

    // --- 2. Start a mock cloud server that serves the ciphertext ---
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

    let (h_digest, h_enc, h_mat) = cloud.meta_headers();
    let cipher_len = ciphertext.len();
    // Azure HEADs the blob first (Get Blob Properties) for size + metadata, so
    // mock it with the metadata headers and a Content-Length-bearing body. GCS
    // never HEADs, so this mock is harmless on that path.
    Mock::given(method("HEAD"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(vec![0u8; cipher_len])
                .insert_header(h_digest, digest.as_str())
                .insert_header(h_enc, enc_data_json.to_string().as_str())
                .insert_header(h_mat, mat_desc_json.to_string().as_str()),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(ciphertext)
                .insert_header(h_digest, digest.as_str())
                .insert_header(h_enc, enc_data_json.to_string().as_str())
                .insert_header(h_mat, mat_desc_json.to_string().as_str()),
        )
        .mount(&server)
        .await;

    // --- 3. Build a stage that points at the mock server and download ---
    // GCS uses `presigned_url`; Azure uses `endpoint` (an `http://`-prefixed
    // value triggers the test-friendly direct-URL branch in
    // `build_azure_url`).
    let dl = match cloud {
        Cloud::Gcs => {
            let stage = StageInfo {
                location_type: LocationType::Gcs,
                bucket: "test-bucket".to_string(),
                key_prefix: "".to_string(),
                region: "us-central1".to_string(),
                creds: CloudCredentials::Gcs {
                    gcs_access_token: None,
                },
                endpoint: None,
                presigned_url: Some(format!("{}/gcs-object", server.uri())),
                use_virtual_url: false,
                use_regional_url: false,
                use_s3_regional_url: false,
                tls_config: sf_core::tls::config::TlsConfig::default(),
                crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
                storage_account: None,
            };
            sf_core::file_manager::internal::download_from_gcs_streaming(
                &stage,
                "gcs-object",
                None,
                // Success-path roundtrip; no retries exercised, so a default
                // zero-backoff policy is sufficient.
                &sf_core::file_manager::internal::gcs_test_retry_policy(
                    false,
                    DEFAULT_PUT_GET_MAX_ATTEMPTS,
                ),
                0,
                MultipartParams::default(),
                &mut None,
                sf_core::file_manager::internal::CloudSpillTarget::Temp(
                    std::env::temp_dir().as_path(),
                ),
            )
            .await
            .expect("GCS streaming download must succeed")
        }
        Cloud::Azure => {
            let stage = StageInfo {
                location_type: LocationType::Azure,
                bucket: "test-container".to_string(),
                key_prefix: "".to_string(),
                region: "eastus2".to_string(),
                creds: CloudCredentials::Azure {
                    sas_token: SensitiveString::from("sv=2021&sig=fake"),
                },
                // http://-prefixed endpoint short-circuits to direct URL,
                // exactly the Azurite test path in build_azure_url.
                endpoint: Some(server.uri()),
                presigned_url: None,
                use_virtual_url: false,
                use_regional_url: false,
                use_s3_regional_url: false,
                tls_config: sf_core::tls::config::TlsConfig::default(),
                crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
                storage_account: Some("mystorageaccount".to_string()),
            };
            sf_core::file_manager::internal::download_from_azure_streaming(
                &stage,
                "azure-blob",
                MultipartParams::default(),
                // Success-path roundtrip; no retries exercised, so the default
                // policy is sufficient.
                &RetryPolicy {
                    max_attempts: DEFAULT_PUT_GET_MAX_ATTEMPTS,
                    ..RetryPolicy::default()
                },
                sf_core::file_manager::internal::CloudSpillTarget::Temp(
                    std::env::temp_dir().as_path(),
                ),
            )
            .await
            .expect("Azure streaming download must succeed")
        }
    };

    let cse = dl
        .cse_info
        .expect("CSE info (metadata + digest) must be present");
    let reader = dl.body.into_reader().expect("into_reader");
    let mat_clone = material.clone();

    // --- 4. Decrypt in spawn_blocking (mirrors mod.rs) ---
    let decrypted = tokio::task::spawn_blocking(move || -> Vec<u8> {
        let mut output = Vec::new();
        sf_core::file_manager::internal::decrypt_ciphertext_to_writer(
            reader,
            &cse.metadata,
            &cse.digest,
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
        "{cloud:?} streaming decrypt must reproduce the original plaintext"
    );
}

/// Tests the full GCS streaming download path:
/// encrypt with ByteSource::Bytes → serve ciphertext from a mock GCS server →
/// `download_from_gcs_streaming` + `decrypt_ciphertext_to_writer` →
/// plaintext matches the original.
///
/// This exercises the `mpsc::sync_channel` bridge (`StreamReader`) between
/// the async reqwest body stream and the sync AES-CBC decryptor.
#[tokio::test]
async fn gcs_streaming_bytes_source_encrypt_decrypt_roundtrip() {
    streaming_roundtrip_for(Cloud::Gcs).await;
}

/// Azure twin of `gcs_streaming_bytes_source_encrypt_decrypt_roundtrip` —
/// identical fixture, exercises the parallel Azure download path through
/// the unified `cloud_http::CloudStreamingDownload`.
///
/// Catches regressions like the Azure SSE branch returning the wrong
/// `output_byte_len` (the Content-Length hint instead of the actually-
/// written byte count), which the GCS test wouldn't have caught.
#[tokio::test]
async fn azure_streaming_bytes_source_encrypt_decrypt_roundtrip() {
    streaming_roundtrip_for(Cloud::Azure).await;
}

// ---------------------------------------------------------------------------
// Mid-body disconnect: the streaming retry loop only covers up to *header*
// receipt. Once `download_from_gcs_streaming` hands back the reader, a
// transport failure mid-body surfaces to the consumer as an `io::Error` with
// no retry and no Range-resume — a deliberate behaviour change vs. the
// buffered path (which collected the whole body inside the retry loop). This
// pins the NOTE on `cloud_http::spawn_byte_stream_producer`.
//
// The fixture is a raw TCP server that returns a 200 with a 1 MiB
// `Content-Length`, writes only 16 body bytes, then closes the socket. reqwest
// (hyper) flags the truncated body as an error, which propagates out of the
// `StreamReader`.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
async fn gcs_streaming_mid_body_disconnect_surfaces_error() {
    use std::io::Read as _;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        // Drain the request — we only need the GET to have arrived before we
        // reply; the exact bytes are irrelevant.
        let mut req = [0u8; 1024];
        let _ = sock.read(&mut req).await;
        // Declare a 1 MiB body, then send only 16 bytes and hang up.
        sock.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1048576\r\n\r\n")
            .await
            .unwrap();
        sock.write_all(&[0u8; 16]).await.unwrap();
        sock.flush().await.unwrap();
        // `sock` dropped here → connection closed mid-body.
    });

    let stage = StageInfo {
        location_type: LocationType::Gcs,
        bucket: "test-bucket".to_string(),
        key_prefix: "".to_string(),
        region: "us-central1".to_string(),
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        endpoint: None,
        presigned_url: Some(format!("http://{addr}/gcs-object")),
        use_virtual_url: false,
        use_regional_url: false,
        use_s3_regional_url: false,
        tls_config: sf_core::tls::config::TlsConfig::default(),
        crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
        storage_account: None,
    };

    // Header phase succeeds — the retry loop saw a 200 with headers before the
    // body was truncated.
    let dl = tokio::time::timeout(
        Duration::from_secs(30),
        sf_core::file_manager::internal::download_from_gcs_streaming(
            &stage,
            "gcs-object",
            None,
            // Success-path roundtrip; no retries exercised, so a default
            // zero-backoff policy is sufficient.
            &sf_core::file_manager::internal::gcs_test_retry_policy(
                false,
                DEFAULT_PUT_GET_MAX_ATTEMPTS,
            ),
            0,
            MultipartParams::default(),
            &mut None,
            sf_core::file_manager::internal::CloudSpillTarget::Temp(std::env::temp_dir().as_path()),
        ),
    )
    .await
    .expect("header phase must not hang")
    .expect("header phase must succeed (200 received before disconnect)");

    // Reading the body must error, and there is no retry — the failure
    // propagates straight out of the reader.
    let reader = dl.body.into_reader().expect("into_reader");
    let read_result = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::task::spawn_blocking(move || {
            let mut reader = reader;
            let mut sink = Vec::new();
            reader.read_to_end(&mut sink)
        }),
    )
    .await
    .expect("body read must not hang")
    .expect("spawn_blocking join");

    assert!(
        read_result.is_err(),
        "mid-body disconnect must surface as an io::Error from the reader, got Ok({:?} bytes)",
        read_result.ok(),
    );

    server.await.unwrap();
}

// Auto-compress + CSE preprocessing flow: the streaming gzip tempfile is the
// lazy encryptor's source (no ciphertext tempfile). Decrypt then decompress
// must reproduce the original; the gzip tempfile must unlink once its guard
// drops.
#[test]
fn auto_compress_then_encrypt_decrypt_decompress_roundtrip() {
    use flate2::read::GzDecoder;

    let plaintext: Vec<u8> = (0..256 * 1024).map(|i| (i % 251) as u8).collect();
    let material = test_encryption_material();

    let (gzip_path, gzip_guard) = sf_core::file_manager::internal::compress_to_tempfile(
        &ByteSource::Bytes(plaintext.clone().into()),
    )
    .expect("compress to tempfile");
    assert!(
        gzip_path.exists(),
        "gzip tempfile must exist before encrypt"
    );

    // Encrypt the gzip tempfile lazily (the production CSE source) — no
    // ciphertext file is produced.
    let (ciphertext, enc_meta, digest) =
        encrypt_source(ByteSource::Path(gzip_path.clone()), &material);

    let mut compressed_back = Vec::<u8>::new();
    sf_core::file_manager::internal::decrypt_ciphertext_to_writer(
        ciphertext.as_slice(),
        &enc_meta,
        &digest,
        &material,
        &mut compressed_back,
    )
    .expect("decrypt ciphertext");

    let mut decompressed = Vec::new();
    GzDecoder::new(compressed_back.as_slice())
        .read_to_end(&mut decompressed)
        .expect("decompress decrypted output");
    assert_eq!(decompressed, plaintext, "round-trip must match input");

    drop(gzip_guard);
    assert!(
        !gzip_path.exists(),
        "gzip tempfile must be unlinked once its guard drops",
    );
}
