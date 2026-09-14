//! S3 GET decrypts CSE objects that have key-wrap headers but no `sfc-digest`,
//! and returns raw bytes when those headers are absent or the wrap is not this
//! query-stage master key (git-stage objects).

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

/// HEAD gets a same-length zero body (S3 HEAD has no body); GET returns `body`.
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

async fn mount_object(server: &MockServer, body: Vec<u8>, extra_headers: Vec<(String, String)>) {
    Mock::given(method("HEAD"))
        .respond_with(FixedS3Object {
            body: body.clone(),
            extra_headers: extra_headers.clone(),
        })
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .respond_with(FixedS3Object {
            body,
            extra_headers,
        })
        .mount(server)
        .await;
}

async fn download_from_mock(
    server: &MockServer,
    encryption_material: Option<EncryptionMaterial>,
) -> tempfile::TempDir {
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

    output_dir
}

#[tokio::test(flavor = "multi_thread")]
async fn s3_download_decrypts_when_cse_headers_present_but_digest_absent() {
    let plaintext = b"one,two\nthree,four\n".to_vec();
    let material = test_material();
    let (ciphertext, encrypted_key, iv, mat_desc_json) = encrypt(&plaintext, &material);

    let server = MockServer::start().await;
    mount_object(
        &server,
        ciphertext,
        vec![
            ("x-amz-meta-x-amz-key".to_string(), encrypted_key),
            ("x-amz-meta-x-amz-iv".to_string(), iv),
            ("x-amz-meta-x-amz-matdesc".to_string(), mat_desc_json),
        ],
    )
    .await;

    let output_dir = download_from_mock(&server, Some(material)).await;
    let downloaded =
        std::fs::read(output_dir.path().join("object.csv")).expect("read downloaded file");

    assert_eq!(downloaded, plaintext);
}

#[tokio::test(flavor = "multi_thread")]
async fn s3_download_returns_raw_bytes_when_all_cse_headers_absent() {
    let raw_bytes = b"raw git-stage file contents".to_vec();
    let material = test_material();

    let server = MockServer::start().await;
    mount_object(&server, raw_bytes.clone(), vec![]).await;

    let output_dir = download_from_mock(&server, Some(material)).await;
    let downloaded =
        std::fs::read(output_dir.path().join("object.csv")).expect("read downloaded file");

    assert_eq!(downloaded, raw_bytes);
}

fn git_stage_headers(material: &EncryptionMaterial) -> Vec<(String, String)> {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use openssl::symm::{Cipher, encrypt};

    let master_key = STANDARD
        .decode(material.query_stage_master_key.reveal())
        .expect("master key");
    let short_key = [9u8; 16];
    let wrapped =
        encrypt(Cipher::aes_256_ecb(), &master_key, None, &short_key).expect("wrap short key");
    let mat_desc = serde_json::json!({
        "queryId": material.query_id,
        "smkId": material.smk_id,
        "keySize": "256",
    });
    vec![
        ("x-amz-meta-x-amz-key".to_string(), STANDARD.encode(wrapped)),
        (
            "x-amz-meta-x-amz-iv".to_string(),
            STANDARD.encode([0u8; 16]),
        ),
        ("x-amz-meta-x-amz-matdesc".to_string(), mat_desc.to_string()),
    ]
}

#[tokio::test(flavor = "multi_thread")]
async fn s3_download_returns_raw_bytes_when_cse_headers_are_git_stage_wrap() {
    let raw_bytes = b"git blob stored as plaintext\n".to_vec();
    let material = test_material();

    let server = MockServer::start().await;
    mount_object(&server, raw_bytes.clone(), git_stage_headers(&material)).await;

    let output_dir = download_from_mock(&server, Some(material)).await;
    let downloaded =
        std::fs::read(output_dir.path().join("object.csv")).expect("read downloaded file");

    assert_eq!(downloaded, raw_bytes);
}
