//! S3 GET must decrypt a client-side-encrypted object whenever the CSE
//! key-wrap headers (`x-amz-key`/`x-amz-iv`/`x-amz-matdesc`) are present, even
//! when `sfc-digest` is absent — the exact signature a server-side
//! `COPY INTO ... SINGLE=TRUE` unload onto a CSE-enabled internal stage
//! leaves on the S3 object (GS's own unloader never sets `sfc-digest`, unlike
//! a client-driven PUT). Before this fix, `download_single_file` required all
//! three of `encryption_material`/CSE-headers/digest to decrypt and silently
//! wrote the still-encrypted ciphertext to disk otherwise — reproduced
//! against a real `sfctest0` account via `COPY INTO` + `GET`.
//!
//! Real S3 git-stage objects (the case the fallback was originally added
//! for, #117) carry none of the four headers (`sfc-digest`, `x-amz-matdesc`,
//! `x-amz-key`, `x-amz-iv`) at all — that raw-bytes fallback is exercised
//! here too and must stay intact.

use sf_core::apis::database_driver_v1::PutGetResultsetFlavor;
use sf_core::config::param_store::ParamStore;
use sf_core::config::retry::RetryPolicy;
use sf_core::file_manager::internal::build_encryptor;
use sf_core::file_manager::types::{
    CloudCredentials, EncryptionMaterial, LocationType, SingleDownloadData, StageInfo,
};
use sf_core::file_manager::{MultipartParams, TransferCtx, download_single_file};
use sf_core::sensitive::SensitiveString;
use std::io::Read;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

fn s3_stage(endpoint: &str) -> StageInfo {
    StageInfo {
        location_type: LocationType::S3,
        bucket: "test-bucket".to_string(),
        key_prefix: String::new(),
        region: "us-east-1".to_string(),
        creds: CloudCredentials::S3 {
            aws_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
            aws_secret_key: SensitiveString::from("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"),
            aws_token: SensitiveString::from(""),
        },
        endpoint: Some(endpoint.to_string()),
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
        use_s3_regional_url: false,
        storage_account: None,
        tls_config: sf_core::tls::config::TlsConfig::default(),
        crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
        proxy_config: sf_core::tls::config::ProxyConfig::default(),
    }
}

fn test_material() -> EncryptionMaterial {
    use base64::Engine;
    let master_key_b64 = base64::engine::general_purpose::STANDARD.encode([0u8; 32]);
    EncryptionMaterial {
        query_stage_master_key: SensitiveString::from(master_key_b64),
        query_id: "test-query-id".to_string(),
        smk_id: "42".to_string(),
    }
}

/// Encrypts `plaintext` through the production lazy path (`build_encryptor` +
/// `EncryptingReader`), returning the ciphertext plus the CSE metadata
/// headers a real PUT would attach to the object.
fn encrypt(plaintext: &[u8], material: &EncryptionMaterial) -> (Vec<u8>, String, String, String) {
    let (encryptor, metadata) =
        build_encryptor(material, plaintext.len() as i64).expect("build_encryptor");
    let mut ciphertext = Vec::new();
    encryptor
        .encrypting_reader(std::io::Cursor::new(plaintext.to_vec()))
        .expect("encrypting_reader")
        .read_to_end(&mut ciphertext)
        .expect("read ciphertext");
    let mat_desc_json =
        serde_json::to_string(&metadata.material_desc).expect("serialize material_desc");
    (
        ciphertext,
        metadata.encrypted_key,
        metadata.iv,
        mat_desc_json,
    )
}

/// Serves a fixed HEAD/GET response pair: HEAD returns `head_headers` over a
/// same-length zero body (matching real S3, which never sends a HEAD body);
/// GET returns `body` unconditionally.
struct FixedS3Object {
    body: Vec<u8>,
    extra_headers: Vec<(String, String)>,
}

impl Respond for FixedS3Object {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let mut template = match request.method.as_str() {
            "HEAD" => ResponseTemplate::new(200).set_body_bytes(vec![0u8; self.body.len()]),
            _ => ResponseTemplate::new(200).set_body_bytes(self.body.clone()),
        };
        for (name, value) in &self.extra_headers {
            template = template.insert_header(name.as_str(), value.as_str());
        }
        template
    }
}

async fn download_from_mock(
    server: &MockServer,
    encryption_material: Option<EncryptionMaterial>,
) -> std::path::PathBuf {
    let output_dir = tempfile::tempdir().unwrap();
    let download = SingleDownloadData {
        src_location: "object.csv".to_string(),
        local_location: output_dir.path().to_str().unwrap().to_string(),
        stage_info: s3_stage(&server.uri()),
        encryption_material,
        presigned_url: None,
        flavor: PutGetResultsetFlavor::Python,
        multipart: MultipartParams::default(),
        unsafe_file_write: false,
    };
    download_single_file(
        download,
        &RetryPolicy::put_get(&ParamStore::new()),
        0,
        TransferCtx::default(),
    )
    .await
    .expect("download should succeed");

    output_dir.keep().join("object.csv")
}

#[tokio::test(flavor = "multi_thread")]
async fn s3_download_decrypts_when_cse_headers_present_but_digest_absent() {
    let plaintext = b"one,two\nthree,four\n".to_vec();
    let material = test_material();
    let (ciphertext, encrypted_key, iv, mat_desc_json) = encrypt(&plaintext, &material);

    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .respond_with(FixedS3Object {
            body: ciphertext.clone(),
            extra_headers: vec![
                ("x-amz-meta-x-amz-key".to_string(), encrypted_key.clone()),
                ("x-amz-meta-x-amz-iv".to_string(), iv.clone()),
                (
                    "x-amz-meta-x-amz-matdesc".to_string(),
                    mat_desc_json.clone(),
                ),
                // No x-amz-meta-sfc-digest: the server-side COPY INTO unload signature.
            ],
        })
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(FixedS3Object {
            body: ciphertext,
            extra_headers: vec![
                ("x-amz-meta-x-amz-key".to_string(), encrypted_key),
                ("x-amz-meta-x-amz-iv".to_string(), iv),
                ("x-amz-meta-x-amz-matdesc".to_string(), mat_desc_json),
            ],
        })
        .mount(&server)
        .await;

    let output_path = download_from_mock(&server, Some(material)).await;
    let downloaded = std::fs::read(&output_path).expect("read downloaded file");

    assert_eq!(
        downloaded, plaintext,
        "the driver must decrypt the object using the CSE key-wrap headers even \
         though sfc-digest is absent — writing the ciphertext through unmodified \
         (the pre-fix behaviour) would fail this assertion"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn s3_download_returns_raw_bytes_when_all_cse_headers_absent() {
    // Real S3 git-stage objects (#117): encryption_material is non-null on the
    // GET response, but the object itself carries none of sfc-digest /
    // x-amz-matdesc / x-amz-key / x-amz-iv. This must still return raw bytes,
    // not attempt (and fail) a decrypt.
    let raw_bytes = b"raw git-stage file contents".to_vec();
    let material = test_material();

    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .respond_with(FixedS3Object {
            body: raw_bytes.clone(),
            extra_headers: vec![],
        })
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(FixedS3Object {
            body: raw_bytes.clone(),
            extra_headers: vec![],
        })
        .mount(&server)
        .await;

    let output_path = download_from_mock(&server, Some(material)).await;
    let downloaded = std::fs::read(&output_path).expect("read downloaded file");

    assert_eq!(
        downloaded, raw_bytes,
        "with no CSE headers at all, the driver must pass the object through \
         verbatim rather than attempting to decrypt it"
    );
}
