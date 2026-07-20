//! S3 multipart upload + ranged download, exercised against a stateful wiremock
//! S3 endpoint.
//!
//! No real account is needed: `MultipartParams` is injected directly with a low
//! threshold (the transfer functions take it as an argument — pure dependency
//! injection, no production code change), so a ~20 MiB file splits into multiple
//! parts on upload and multiple byte ranges on download. The test verifies the
//! round-trip is byte-identical AND asserts the multipart/ranged protocol
//! actually fired (≥2 `UploadPart` calls, ≥2 `Range` GETs, and no fallback to a
//! single `PutObject` / full `GET`).
//!
//! The S3 *part size* (8 MiB) is fixed by `MultipartConfig::S3`, so the payload
//! must exceed it to split — the threshold only controls single-vs-multipart.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use sf_core::apis::database_driver_v1::PutGetResultsetFlavor;
use sf_core::config::param_store::ParamStore;
use sf_core::config::retry::RetryPolicy;
use sf_core::file_manager::internal::compute_sha256_digest;
use sf_core::file_manager::types::{
    ByteSource, CloudCredentials, LocationType, SingleDownloadData, SingleUploadData, StageInfo,
};
use sf_core::file_manager::{
    MultipartParams, SourceCompressionParam, download_single_file, upload_single_file,
};
use sf_core::sensitive::SensitiveString;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

/// S3 default part size (`MultipartConfig::S3.default_part`). `compute_part_size`
/// returns this for any file below the grow boundary.
const PART_SIZE: usize = 8 * 1024 * 1024;
/// > 2 × `PART_SIZE`, so the file splits into 3 parts / ranges — unambiguously ≥2.
const PAYLOAD_LEN: usize = 20 * 1024 * 1024;
/// Below `PAYLOAD_LEN`, so both upload and download take the multipart path.
const THRESHOLD_BYTES: i64 = 8 * 1024 * 1024;

/// Deterministic, position-dependent payload (a tiny LCG) so that a mis-ordered
/// part or range on reassembly cannot still compare equal to the original.
fn make_payload(len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut state: u32 = 0x9e37_79b9;
    for _ in 0..len {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        out.push((state >> 24) as u8);
    }
    out
}

#[derive(Default)]
struct S3MockState {
    payload: Vec<u8>,
    digest: String,
    create_calls: AtomicUsize,
    upload_part_calls: AtomicUsize,
    complete_calls: AtomicUsize,
    single_put_calls: AtomicUsize,
    ranged_get_calls: AtomicUsize,
    full_get_calls: AtomicUsize,
}

#[derive(Clone)]
struct S3Mock {
    state: Arc<S3MockState>,
}

/// Parse an inclusive `Range: bytes=START-END` header against `total`.
fn parse_range(value: &str, total: usize) -> (usize, usize) {
    let spec = value.trim().trim_start_matches("bytes=");
    let mut it = spec.split('-');
    let start: usize = it.next().unwrap().trim().parse().unwrap();
    let end: usize = it
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().unwrap())
        .unwrap_or(total - 1);
    (start, end.min(total - 1))
}

impl Respond for S3Mock {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let s = &self.state;
        let method = request.method.as_str();
        let query = request.url.query().unwrap_or("");

        match method {
            // CreateMultipartUpload: POST .../key?uploads
            "POST" if query.contains("uploads") && !query.contains("uploadId") => {
                s.create_calls.fetch_add(1, Ordering::Relaxed);
                ResponseTemplate::new(200).set_body_raw(
                    br#"<?xml version="1.0" encoding="UTF-8"?><InitiateMultipartUploadResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Bucket>test-bucket</Bucket><Key>bigfile.bin</Key><UploadId>mock-upload-id</UploadId></InitiateMultipartUploadResult>"#.to_vec(),
                    "application/xml",
                )
            }
            // CompleteMultipartUpload: POST .../key?uploadId=...
            "POST" if query.contains("uploadId") => {
                s.complete_calls.fetch_add(1, Ordering::Relaxed);
                ResponseTemplate::new(200).set_body_raw(
                    br#"<?xml version="1.0" encoding="UTF-8"?><CompleteMultipartUploadResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Location>http://test-bucket/bigfile.bin</Location><Bucket>test-bucket</Bucket><Key>bigfile.bin</Key><ETag>"mock-complete-etag"</ETag></CompleteMultipartUploadResult>"#.to_vec(),
                    "application/xml",
                )
            }
            // UploadPart: PUT .../key?partNumber=N&uploadId=...
            "PUT" if query.contains("partNumber") => {
                let n = s.upload_part_calls.fetch_add(1, Ordering::Relaxed) + 1;
                ResponseTemplate::new(200).insert_header("ETag", format!("\"etag-{n}\"").as_str())
            }
            // Single PutObject — must NOT happen for a multipart-sized file.
            "PUT" => {
                s.single_put_calls.fetch_add(1, Ordering::Relaxed);
                ResponseTemplate::new(200).insert_header("ETag", "\"single-etag\"")
            }
            // HeadObject: size (Content-Length derived from the body) + sfc-digest.
            "HEAD" => ResponseTemplate::new(200)
                .insert_header("x-amz-meta-sfc-digest", s.digest.as_str())
                .set_body_bytes(vec![0u8; s.payload.len()]),
            // GetObject — ranged (206) or full (200).
            "GET" => match request.headers.get("range") {
                Some(range) => {
                    let (start, end) = parse_range(range.to_str().unwrap(), s.payload.len());
                    s.ranged_get_calls.fetch_add(1, Ordering::Relaxed);
                    ResponseTemplate::new(206)
                        .insert_header(
                            "Content-Range",
                            format!("bytes {start}-{end}/{}", s.payload.len()).as_str(),
                        )
                        .insert_header("x-amz-meta-sfc-digest", s.digest.as_str())
                        .set_body_bytes(s.payload[start..=end].to_vec())
                }
                None => {
                    s.full_get_calls.fetch_add(1, Ordering::Relaxed);
                    ResponseTemplate::new(200)
                        .insert_header("x-amz-meta-sfc-digest", s.digest.as_str())
                        .set_body_bytes(s.payload.clone())
                }
            },
            // AbortMultipartUpload / cleanup.
            "DELETE" => ResponseTemplate::new(204),
            _ => ResponseTemplate::new(400),
        }
    }
}

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
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn should_upload_and_download_via_s3_multipart_roundtrip() {
    let payload = make_payload(PAYLOAD_LEN);
    let digest =
        compute_sha256_digest(&ByteSource::Bytes(payload.clone().into())).expect("compute digest");
    let expected_chunks = PAYLOAD_LEN.div_ceil(PART_SIZE);
    assert!(expected_chunks >= 2, "payload must split into >=2 chunks");

    let state = Arc::new(S3MockState {
        payload: payload.clone(),
        digest: digest.clone(),
        ..Default::default()
    });
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(S3Mock {
            state: Arc::clone(&state),
        })
        .mount(&server)
        .await;

    // Inject a low threshold directly — no connection parameter, no global state.
    let multipart = MultipartParams::from_server(Some(THRESHOLD_BYTES), Some(4));

    // ---- Upload (multipart) ----
    let upload = SingleUploadData {
        source: ByteSource::Bytes(payload.clone().into()),
        filename: "bigfile.bin".to_string(),
        stage_info: s3_stage(&server.uri()),
        encryption_material: None, // SSE stage: the upload body is the raw payload.
        auto_compress: false,
        source_compression: SourceCompressionParam::None,
        overwrite: true,
        flavor: PutGetResultsetFlavor::Python,
        legacy_odbc_compression_autodetect: false,
        skip_upload_on_content_match: false,
        multipart,
    };
    let upload_result = upload_single_file(
        upload,
        &RetryPolicy::put_get(&ParamStore::new()),
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("upload should succeed");
    assert_eq!(upload_result.status, "UPLOADED");

    assert_eq!(
        state.create_calls.load(Ordering::Relaxed),
        1,
        "exactly one CreateMultipartUpload"
    );
    assert_eq!(
        state.complete_calls.load(Ordering::Relaxed),
        1,
        "exactly one CompleteMultipartUpload"
    );
    assert_eq!(
        state.single_put_calls.load(Ordering::Relaxed),
        0,
        "must not fall back to a single PutObject"
    );
    assert_eq!(
        state.upload_part_calls.load(Ordering::Relaxed),
        expected_chunks,
        "one UploadPart per chunk"
    );

    // ---- Download (ranged) ----
    let output_dir = tempfile::tempdir().unwrap();
    let download = SingleDownloadData {
        src_location: "bigfile.bin".to_string(),
        local_location: output_dir.path().to_str().unwrap().to_string(),
        stage_info: s3_stage(&server.uri()),
        encryption_material: None,
        presigned_url: None,
        flavor: PutGetResultsetFlavor::Python,
        multipart,
        unsafe_file_write: false,
    };
    download_single_file(
        download,
        &RetryPolicy::put_get(&ParamStore::new()),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("download should succeed");

    assert_eq!(
        state.ranged_get_calls.load(Ordering::Relaxed),
        expected_chunks,
        "one ranged GET per chunk"
    );
    assert_eq!(
        state.full_get_calls.load(Ordering::Relaxed),
        0,
        "must not fall back to a single full GET"
    );

    // ---- Compare ----
    let downloaded = std::fs::read(output_dir.path().join("bigfile.bin")).expect("read output");
    assert_eq!(downloaded.len(), payload.len(), "downloaded length matches");
    assert!(downloaded == payload, "downloaded bytes match the original");
}
