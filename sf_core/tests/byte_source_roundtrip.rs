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
// Every cloud goes through `spawn_download_stream_pipeline` and the same
// sync-channel bridge into the sync decryptor; only the producer (S3's
// AWS-SDK `ByteStream` vs. GCS/Azure's reqwest body) and the wire-level
// metadata-header names differ, while the body bytes round-trip identically.
// The `Cloud` flavour drives both the mock's CSE metadata headers
// (`insert_cse_headers`) and the `StageInfo` shape (`cloud_stage`), so one
// fixture can run once per cloud and an Azure regression in this layer can't
// masquerade as "the S3/GCS test still passes".
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum Cloud {
    S3,
    Gcs,
    Azure,
}

/// Inserts the Azure-format `encryptiondata` JSON blob plus digest/matdesc
/// headers shared by GCS and Azure — only the meta-header prefixes differ
/// between the two clouds.
fn insert_blob_cse_headers(
    tpl: wiremock::ResponseTemplate,
    enc_meta: &EncryptedFileMetadata,
    digest: &str,
    h_digest: &str,
    h_enc: &str,
    h_mat: &str,
) -> wiremock::ResponseTemplate {
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
    tpl.insert_header(h_digest, digest)
        .insert_header(h_enc, enc_data_json.to_string().as_str())
        .insert_header(h_mat, mat_desc_json.to_string().as_str())
}

impl Cloud {
    /// The object/blob name used in the mock GET. For GCS this must match the
    /// `presigned_url` suffix built in [`cloud_stage`].
    fn src_location(self) -> &'static str {
        match self {
            Cloud::S3 => "cse-object",
            Cloud::Gcs => "gcs-object",
            Cloud::Azure => "azure-blob",
        }
    }

    /// Inserts this cloud's client-side-encryption metadata headers onto a
    /// mock response. S3 carries key/iv/matdesc as discrete `x-amz-meta-*`
    /// headers; GCS and Azure carry an Azure-format `encryptiondata` JSON blob
    /// under their respective meta-header prefixes.
    fn insert_cse_headers(
        self,
        tpl: wiremock::ResponseTemplate,
        enc_meta: &EncryptedFileMetadata,
        digest: &str,
    ) -> wiremock::ResponseTemplate {
        match self {
            Cloud::S3 => {
                let mat_desc_json = serde_json::to_string(&enc_meta.material_desc).unwrap();
                tpl.insert_header("x-amz-meta-sfc-digest", digest)
                    .insert_header("x-amz-meta-x-amz-matdesc", mat_desc_json.as_str())
                    .insert_header("x-amz-meta-x-amz-key", enc_meta.encrypted_key.as_str())
                    .insert_header("x-amz-meta-x-amz-iv", enc_meta.iv.as_str())
            }
            Cloud::Gcs => insert_blob_cse_headers(
                tpl,
                enc_meta,
                digest,
                "x-goog-meta-sfc-digest",
                "x-goog-meta-encryptiondata",
                "x-goog-meta-matdesc",
            ),
            Cloud::Azure => insert_blob_cse_headers(
                tpl,
                enc_meta,
                digest,
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

    let cipher_len = ciphertext.len();
    // Azure HEADs the blob first (Get Blob Properties) for size + metadata, so
    // mock it with the metadata headers and a Content-Length-bearing body. GCS
    // never HEADs, so this mock is harmless on that path.
    Mock::given(method("HEAD"))
        .respond_with(cloud.insert_cse_headers(
            ResponseTemplate::new(200).set_body_bytes(vec![0u8; cipher_len]),
            &enc_meta,
            &digest,
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(cloud.insert_cse_headers(
            ResponseTemplate::new(200).set_body_bytes(ciphertext),
            &enc_meta,
            &digest,
        ))
        .mount(&server)
        .await;

    // --- 3. Build a stage that points at the mock server and download ---
    // GCS uses `presigned_url`; Azure uses `endpoint` (an `http://`-prefixed
    // value triggers the test-friendly direct-URL branch in
    // `build_azure_url`). S3 has no place here: this fixture exercises the
    // lower-level reqwest `download_from_{gcs,azure}_streaming` API, whereas
    // S3's streaming download is an AWS-SDK `ByteStream` (covered through the
    // shared pipeline in `open_download_stream_for_stage_s3_cse_roundtrip`).
    let dl = match cloud {
        Cloud::S3 => unreachable!(
            "streaming_roundtrip_for is GCS/Azure-only; S3 uses the AWS-SDK \
             ByteStream path — see open_download_stream_for_stage_s3_cse_roundtrip"
        ),
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
                false,
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
                false,
                sf_core::file_manager::internal::CloudSpillTarget::Temp(
                    std::env::temp_dir().as_path(),
                ),
                &mut None,
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
/// This exercises the `tokio::sync::mpsc` bridge (`StreamReader`) between
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
/// Spawns a raw TCP server that accepts one connection, drains whatever
/// request arrives, replies with a 200 declaring `Content-Length: 1048576`,
/// writes only 16 body bytes, then either drops the socket immediately
/// (`hang: false`, simulating a mid-body disconnect a retry loop can't
/// recover from) or holds the connection open indefinitely (`hang: true`,
/// simulating a stalled read for abort tests — the caller is expected to
/// abort its client-side tasks long before this would ever resolve on its
/// own). Shared by every cloud's mid-body-disconnect / abort test, since the
/// fixture itself is cloud-agnostic (it never inspects the request, only the
/// fact that one arrived).
async fn spawn_truncated_body_server(
    hang: bool,
) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        // Drain the request — we only need the GET to have arrived before we
        // reply; the exact bytes (path, auth headers, ...) are irrelevant.
        let mut req = [0u8; 4096];
        let _ = sock.read(&mut req).await;
        // Declare a 1 MiB body, then send only 16 bytes and hang up.
        sock.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1048576\r\n\r\n")
            .await
            .unwrap();
        sock.write_all(&[0u8; 16]).await.unwrap();
        sock.flush().await.unwrap();
        if hang {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
        // Otherwise `sock` drops here → connection closed mid-body.
    });

    (addr, handle)
}

#[tokio::test(flavor = "multi_thread")]
async fn gcs_streaming_mid_body_disconnect_surfaces_error() {
    use std::io::Read as _;
    use std::time::Duration;

    let (addr, server) = spawn_truncated_body_server(false).await;

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
            false,
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

// ---------------------------------------------------------------------------
// `open_s3_download_stream`: the zero-disk, chunked S3 GET path underneath
// `download_stream_begin`/`_chunk`/`_close`. These tests drain the public
// `DownloadStreamOpen::chunks` channel end to end, exercising the
// spawn_blocking decrypt/gunzip pipeline and the ChannelWriter bridge.
// ---------------------------------------------------------------------------

/// Builds an S3, GCS, or Azure `StageInfo` pointed at a mock server `uri`,
/// following the same shape as `streaming_roundtrip_for`'s inline stages
/// above. S3 and Azure are wired via `endpoint`; GCS via `presigned_url`
/// (Azure's `http://`-prefixed endpoint short-circuits to a direct URL in
/// `build_azure_url`, exactly like the Azurite test path). The S3 arm matches
/// `download_single_file_tampered_digest_leaves_no_output`'s inline stage.
fn cloud_stage(cloud: Cloud, uri: String) -> StageInfo {
    match cloud {
        Cloud::S3 => StageInfo {
            location_type: LocationType::S3,
            // AWS doc-convention bucket/prefix, matching the AKIA...EXAMPLE creds below.
            bucket: "examplebucket".to_string(),
            key_prefix: "photos/2024/".to_string(),
            region: "us-east-1".to_string(),
            creds: CloudCredentials::S3 {
                aws_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
                aws_secret_key: SensitiveString::from("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"),
                aws_token: SensitiveString::from(""),
            },
            endpoint: Some(uri),
            presigned_url: None,
            use_virtual_url: false,
            use_regional_url: false,
            use_s3_regional_url: false,
            tls_config: sf_core::tls::config::TlsConfig::default(),
            crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
            storage_account: None,
        },
        Cloud::Gcs => StageInfo {
            location_type: LocationType::Gcs,
            bucket: "test-bucket".to_string(),
            key_prefix: "".to_string(),
            region: "us-central1".to_string(),
            creds: CloudCredentials::Gcs {
                gcs_access_token: None,
            },
            endpoint: None,
            presigned_url: Some(format!("{uri}/gcs-object")),
            use_virtual_url: false,
            use_regional_url: false,
            use_s3_regional_url: false,
            tls_config: sf_core::tls::config::TlsConfig::default(),
            crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
            storage_account: None,
        },
        Cloud::Azure => StageInfo {
            location_type: LocationType::Azure,
            bucket: "test-container".to_string(),
            key_prefix: "".to_string(),
            region: "eastus2".to_string(),
            creds: CloudCredentials::Azure {
                sas_token: SensitiveString::from("sv=2021&sig=fake"),
            },
            endpoint: Some(uri),
            presigned_url: None,
            use_virtual_url: false,
            use_regional_url: false,
            use_s3_regional_url: false,
            tls_config: sf_core::tls::config::TlsConfig::default(),
            crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
            storage_account: Some("mystorageaccount".to_string()),
        },
    }
}

/// Drains a [`DownloadStreamOpen`]'s `chunks` channel to a single `Vec<u8>`,
/// propagating the first terminal error (if any) and joining the background
/// producer/pipeline task so a panic there fails the test loudly rather than
/// silently truncating the output.
async fn drain_download_stream(
    mut opened: sf_core::file_manager::DownloadStreamOpen,
) -> Result<Vec<u8>, sf_core::file_manager::FileManagerError> {
    let mut out = Vec::new();
    while let Some(item) = opened.chunks.recv().await {
        out.extend_from_slice(&item?);
    }
    opened.task.await.expect("pipeline task must not panic");
    Ok(out)
}

/// Zero-backoff put/get retry policy for the success-path streaming tests —
/// no retries are exercised, so the default backoff config would just add
/// unnecessary latency if a retry were ever (incorrectly) triggered.
fn zero_backoff_test_retry_policy() -> RetryPolicy {
    use sf_core::config::retry::{BackoffConfig, Jitter};
    use std::time::Duration;
    RetryPolicy {
        backoff: BackoffConfig {
            base: Duration::ZERO,
            factor: 1.0,
            cap: Duration::ZERO,
            jitter: Jitter::None,
        },
        ..RetryPolicy::put_get(&ParamStore::new())
    }
}

/// CSE round-trip through the chunked `open_download_stream_for_stage`
/// pipeline, shared by all three clouds: encrypt → serve ciphertext + the
/// cloud's CSE metadata headers from a mock GET → dispatch to that cloud's
/// zero-disk opener → drain the `DownloadStreamOpen::chunks` channel →
/// assert the decrypted plaintext matches. `decompress: false` — gzip is
/// covered separately below.
///
/// This is the single common CSE test across S3/GCS/Azure. The dispatcher and
/// `DownloadStreamOpen` are the one API layer all three clouds share (S3 via
/// the AWS-SDK `ByteStream` producer, GCS/Azure via the reqwest producer), so
/// a CSE regression in any one cloud's pipeline wiring surfaces here rather
/// than hiding behind a green test for the other two.
async fn open_download_stream_cse_roundtrip_for(cloud: Cloud) {
    use sf_core::file_manager::open_download_stream_for_stage;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    let plaintext = format!("{cloud:?} open_download_stream CSE round-trip payload").into_bytes();
    let material = test_encryption_material();
    let (ciphertext, enc_meta, digest) =
        encrypt_source(ByteSource::Bytes(plaintext.clone().into()), &material);

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(cloud.insert_cse_headers(
            ResponseTemplate::new(200).set_body_bytes(ciphertext),
            &enc_meta,
            &digest,
        ))
        .mount(&server)
        .await;

    let stage = cloud_stage(cloud, server.uri());
    let opened = open_download_stream_for_stage(
        &stage,
        cloud.src_location(),
        // GCS reads its presigned URL from the stage; no per-file override.
        None,
        &zero_backoff_test_retry_policy(),
        &mut None,
        Some(material),
        false,
    )
    .await
    .expect("open_download_stream_for_stage must succeed");

    let decrypted = drain_download_stream(opened)
        .await
        .expect("streaming decrypt must succeed");
    assert_eq!(
        decrypted, plaintext,
        "{cloud:?}: chunked download must reproduce the original plaintext"
    );
}

#[tokio::test]
async fn open_download_stream_for_stage_s3_cse_roundtrip() {
    open_download_stream_cse_roundtrip_for(Cloud::S3).await;
}

#[tokio::test]
async fn open_download_stream_for_stage_gcs_cse_roundtrip() {
    open_download_stream_cse_roundtrip_for(Cloud::Gcs).await;
}

#[tokio::test]
async fn open_download_stream_for_stage_azure_cse_roundtrip() {
    open_download_stream_cse_roundtrip_for(Cloud::Azure).await;
}

/// Minimal `StageInfoRefresher` for the Azure SAS-refresh test: counts
/// `refresh()` calls and rotates the shared cache to a fresh SAS on each, so
/// the post-403 retry runs with new credentials. Mirrors the creds-only
/// refresher shape S3/Azure use in production.
struct CountingSasRefresher {
    cache: sf_core::file_manager::types::StageInfoCache,
    calls: std::sync::Arc<std::sync::atomic::AtomicU32>,
    fresh: CloudCredentials,
}

impl sf_core::file_manager::types::StageInfoRefresher for CountingSasRefresher {
    fn refresh(&mut self) -> sf_core::file_manager::types::RefreshFuture<'_> {
        let calls = self.calls.clone();
        let cache = self.cache.clone();
        let fresh = self.fresh.clone();
        Box::pin(async move {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            cache.store(sf_core::file_manager::types::StageInfoSnapshot::creds_only(
                fresh,
            ));
            Ok(())
        })
    }

    fn refresh_url(&mut self) -> sf_core::file_manager::types::RefreshFuture<'_> {
        Box::pin(async { Ok(()) })
    }

    fn cache(&self) -> &sf_core::file_manager::types::StageInfoCache {
        &self.cache
    }
}

/// The Azure arm of `open_download_stream_for_stage` must thread the
/// `refresher` through so an already-expired SAS is rotated and retried — the
/// zero-disk twin of the buffered `download_from_azure_streaming` refresh
/// path. Pre-fix the Azure opener took no refresher, so a 403 surfaced
/// terminally and this download would fail. First GET fast-fails 403 →
/// SAS-refresh layer → `refresh()` rotates creds → the retry GET succeeds.
#[tokio::test]
async fn open_download_stream_for_stage_azure_refreshes_sas_on_403() {
    use sf_core::file_manager::open_download_stream_for_stage;
    use sf_core::file_manager::types::{StageInfoCache, StageInfoRefresher};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    let plaintext = b"azure SAS refresh after 403".to_vec();

    let server = MockServer::start().await;
    // First GET: 403 (expired SAS) — Azure maps this to SasExpired, routing to
    // the refresh layer. Exhausts after one call via up_to_n_times.
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(403)
                .set_body_string("<Error><Code>AuthenticationFailed</Code></Error>"),
        )
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    // Retry (post-refresh) GET: 200 with the body.
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(plaintext.clone()))
        .with_priority(2)
        .mount(&server)
        .await;

    let stage = cloud_stage(Cloud::Azure, server.uri());
    let mut refresher = CountingSasRefresher {
        cache: StageInfoCache::new_with_creds(stage.creds.clone()),
        calls: Arc::new(AtomicU32::new(0)),
        fresh: CloudCredentials::Azure {
            sas_token: SensitiveString::from("sv=2021&sig=refreshed"),
        },
    };
    let calls = refresher.calls.clone();
    let mut refresher_dyn: Option<&mut dyn StageInfoRefresher> = Some(&mut refresher);

    let opened = open_download_stream_for_stage(
        &stage,
        Cloud::Azure.src_location(),
        None,
        &zero_backoff_test_retry_policy(),
        &mut refresher_dyn,
        None,
        false,
    )
    .await
    .expect("Azure open must succeed after the SAS is refreshed");

    let out = drain_download_stream(opened)
        .await
        .expect("download must succeed via the refreshed SAS");
    assert_eq!(
        out, plaintext,
        "must round-trip the body served after refresh"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "expected exactly one SAS refresh triggered by the 403"
    );
}

/// Decompress-only path (no CSE): object is gzip-compressed, no client-side
/// encryption. `decompress: true` must gunzip in-flight with no decrypt step.
#[tokio::test]
async fn s3_open_download_stream_gunzip_only_roundtrip() {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use sf_core::file_manager::open_s3_download_stream;
    use std::io::Write as _;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    let plaintext: Vec<u8> = (0..64 * 1024).map(|i| (i % 251) as u8).collect();
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&plaintext).expect("gzip write");
    let gzip_bytes = encoder.finish().expect("gzip finish");

    let server = MockServer::start().await;
    // No CSE headers — hits the SSE/no-encryption arm of the pipeline match.
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(gzip_bytes))
        .mount(&server)
        .await;

    let stage = cloud_stage(Cloud::S3, server.uri());
    let opened = open_s3_download_stream(
        &stage,
        "gzip-object",
        &zero_backoff_test_retry_policy(),
        &mut None,
        None,
        true,
    )
    .await
    .expect("open_s3_download_stream must succeed");

    let decompressed = drain_download_stream(opened)
        .await
        .expect("streaming gunzip must succeed");
    assert_eq!(
        decompressed, plaintext,
        "chunked S3 download must reproduce the original plaintext after gunzip"
    );
}

// ---------------------------------------------------------------------------
// Mid-body disconnect, same tradeoff as
// `gcs_streaming_mid_body_disconnect_surfaces_error`: the AWS SDK's retry
// covers only opening the GET, so a body truncation surfaces as a terminal
// `Err` on `DownloadStreamOpen::chunks` with no retry and no Range-resume.
//
// Fixture: a raw TCP server sends a 200 with a 1 MiB Content-Length, writes
// 16 body bytes, then closes — same shape as the GCS fixture, for S3.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
async fn s3_open_download_stream_mid_body_disconnect_surfaces_error() {
    use sf_core::file_manager::open_s3_download_stream;

    let (addr, server) = spawn_truncated_body_server(false).await;

    let stage = cloud_stage(Cloud::S3, format!("http://{addr}"));
    let opened = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        open_s3_download_stream(
            &stage,
            "disconnect-object",
            &zero_backoff_test_retry_policy(),
            &mut None,
            None,
            false,
        ),
    )
    .await
    .expect("open must not hang")
    .expect("header phase must succeed (200 received before disconnect)");

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        drain_download_stream(opened),
    )
    .await
    .expect("drain must not hang");

    assert!(
        result.is_err(),
        "mid-body disconnect must surface as a terminal Err on the chunks channel, got Ok({:?} bytes)",
        result.ok().map(|v| v.len()),
    );

    server.await.unwrap();
}

// ---------------------------------------------------------------------------
// open_download_stream_for_stage: the dispatcher (mod.rs) that routes a
// download_stream_begin call to the right cloud's zero-disk opener based on
// `stage_info.location_type`. `LocationType` is exhaustive (S3/Gcs/Azure), so
// the match has no fallback arm — these three tests are the only coverage
// that actually drives the match itself (each cloud's opener already has its
// own direct-call coverage above/nearby, but never through the dispatcher).
// ---------------------------------------------------------------------------

/// Opens through `open_download_stream_for_stage` and drains to a `Vec<u8>`,
/// with no CSE and no decompression — just enough to prove the dispatch
/// match reached the right opener and that opener's body round-trips.
async fn dispatch_raw_roundtrip_for(
    stage: StageInfo,
    src_location: &str,
    per_file_presigned_url: Option<&str>,
) -> Vec<u8> {
    use sf_core::file_manager::open_download_stream_for_stage;

    let opened = open_download_stream_for_stage(
        &stage,
        src_location,
        per_file_presigned_url,
        &zero_backoff_test_retry_policy(),
        &mut None,
        None,
        false,
    )
    .await
    .expect("open_download_stream_for_stage must succeed");

    drain_download_stream(opened)
        .await
        .expect("streaming copy must succeed")
}

#[tokio::test]
async fn open_download_stream_for_stage_dispatches_to_s3() {
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    let plaintext = b"dispatch-s3-payload".to_vec();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(plaintext.clone()))
        .mount(&server)
        .await;

    let out =
        dispatch_raw_roundtrip_for(cloud_stage(Cloud::S3, server.uri()), "object", None).await;
    assert_eq!(
        out, plaintext,
        "S3 arm of the dispatch match must round-trip raw bytes"
    );
}

#[tokio::test]
async fn open_download_stream_for_stage_dispatches_to_gcs() {
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    let plaintext = b"dispatch-gcs-payload".to_vec();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(plaintext.clone()))
        .mount(&server)
        .await;

    let stage = cloud_stage(Cloud::Gcs, server.uri());
    let out = dispatch_raw_roundtrip_for(stage, "gcs-object", None).await;
    assert_eq!(
        out, plaintext,
        "GCS arm of the dispatch match must round-trip raw bytes"
    );
}

#[tokio::test]
async fn open_download_stream_for_stage_dispatches_to_azure() {
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    let plaintext = b"dispatch-azure-payload".to_vec();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(plaintext.clone()))
        .mount(&server)
        .await;

    let stage = cloud_stage(Cloud::Azure, server.uri());
    let out = dispatch_raw_roundtrip_for(stage, "azure-blob", None).await;
    assert_eq!(
        out, plaintext,
        "Azure arm of the dispatch match must round-trip raw bytes"
    );
}

/// `per_file_presigned_url` must take precedence over `stage_info.
/// presigned_url` — this is the wrapper's per-file URL override path (used
/// when GS returns a distinct presigned URL for a specific file in a
/// multi-file GET response). Point `stage_info.presigned_url` at an
/// unreachable address and `per_file_presigned_url` at the real mock server;
/// the download only succeeds if the per-file URL actually won.
#[tokio::test]
async fn open_gcs_download_stream_per_file_presigned_url_takes_precedence() {
    use sf_core::file_manager::open_gcs_download_stream;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    let plaintext = b"per-file-presigned-url-wins".to_vec();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(plaintext.clone()))
        .mount(&server)
        .await;

    let mut stage = cloud_stage(Cloud::Gcs, server.uri());
    // Deliberately unreachable: if the stage-level URL were used instead of
    // the per-file one, this connection would fail (nothing listens there).
    stage.presigned_url = Some("http://127.0.0.1:1/unreachable-stage-url".to_string());

    let opened = open_gcs_download_stream(
        &stage,
        "gcs-object",
        Some(&format!("{}/gcs-object", server.uri())),
        &zero_backoff_test_retry_policy(),
        &mut None,
        None,
        false,
    )
    .await
    .expect("per_file_presigned_url must take precedence over stage_info.presigned_url");

    let out = drain_download_stream(opened)
        .await
        .expect("streaming copy must succeed");
    assert_eq!(
        out, plaintext,
        "must fetch from the per-file URL, not the unreachable stage-level one"
    );
}

// ---------------------------------------------------------------------------
// Abort must actually stop a chunked download whose producer is parked on a
// stalled read — mirrors `download_stream_close_deregisters_and_aborts_the_
// task` in stream_transfer.rs, but exercises the real GCS/Azure producer
// against a genuinely wedged connection instead of a fake `pending()` future.
// ---------------------------------------------------------------------------

async fn abort_stops_a_hanging_download_for(cloud: Cloud) {
    use sf_core::file_manager::open_download_stream_for_stage;

    let (addr, hang_server) = spawn_truncated_body_server(true).await;
    let stage = cloud_stage(cloud, format!("http://{addr}"));

    let opened = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        open_download_stream_for_stage(
            &stage,
            "object",
            stage.presigned_url.as_deref(),
            &zero_backoff_test_retry_policy(),
            &mut None,
            None,
            false,
        ),
    )
    .await
    .expect("open must not hang")
    .expect("header phase must succeed (200 received before the connection wedges)");

    opened.producer_abort.abort();
    opened.task.abort();

    // `task` is a `spawn_blocking` closure parked in a *synchronous*
    // `recv()`; aborting its `JoinHandle` doesn't force-interrupt the OS
    // thread mid-read (spawn_blocking can't be preempted). What actually
    // unblocks it is the producer's abort tearing down its task — dropping
    // the response body's `Sender` — which makes the pipeline's `recv()`
    // return an error and let the closure return on its own. That teardown
    // happens on a real OS thread, so give it real wall-clock time rather
    // than spinning `yield_now()` (which can burn through many iterations
    // faster than the other thread gets scheduled at all, without ever
    // actually waiting on it).
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < deadline
        && !(opened.producer_abort.is_finished() && opened.task.is_finished())
    {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        opened.producer_abort.is_finished(),
        "{cloud:?}: abort must stop the still-pending producer task"
    );
    assert!(
        opened.task.is_finished(),
        "{cloud:?}: abort must stop the still-pending pipeline task"
    );

    // The fixture's `hang` branch sleeps 3600s; without this the server task
    // would stay parked for the rest of the test run instead of dropping with
    // the connection once this test is done.
    hang_server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn open_download_stream_for_stage_abort_stops_a_hanging_gcs_download() {
    abort_stops_a_hanging_download_for(Cloud::Gcs).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn open_download_stream_for_stage_abort_stops_a_hanging_azure_download() {
    abort_stops_a_hanging_download_for(Cloud::Azure).await;
}
