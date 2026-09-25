//! GCS single-PUT upload + ranged download, exercised against a stateful
//! wiremock GCS endpoint.
//!
//! No real account is needed: `MultipartParams` is injected with a low
//! threshold so a ~20 MiB file is above it. Upload is still one object `PUT`
//! (digest metadata on that request, no initiation `POST`, no `Content-Range`);
//! download still takes parallel ranged `GET`s. The test verifies the
//! round-trip is byte-identical and that both wire shapes actually fired.
//!
//! The GCS *range size* (8 MiB, `MultipartConfig::GCS.default_part`) is fixed
//! for files this small, so the payload must exceed it to split on GET — the
//! threshold only controls single-vs-ranged routing on download. Token-refresh
//! on a single PUT is covered in `gcs_retry.rs`.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use sf_core::apis::database_driver_v1::PutGetResultsetFlavor;
use sf_core::config::param_store::ParamStore;
use sf_core::config::retry::RetryPolicy;
use sf_core::file_manager::internal::{MultipartConfig, compute_sha256_digest};
use sf_core::file_manager::types::{
    ByteSource, CloudCredentials, LocationType, SingleDownloadData, SingleUploadData, StageInfo,
};
use sf_core::file_manager::{
    MultipartParams, SourceCompressionParam, TransferCtx, download_single_file, upload_single_file,
};
use sf_core::sensitive::SensitiveString;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use crate::http::multipart_test_support::{make_payload, parse_range};

/// GCS user-metadata header carrying Snowflake's SHA-256 digest. Mirrors the
/// private constant of the same name in `gcs_transfer.rs`.
const GCS_META_SFC_DIGEST: &str = "x-goog-meta-sfc-digest";

/// GCS ranged-GET default range size (`MultipartConfig::GCS.default_part`).
/// `compute_part_size` returns this for any file, since GCS has no range-count
/// cap to grow past it.
const PART_SIZE: usize = 8 * 1024 * 1024;
/// > 2 × `PART_SIZE`, so the download splits into 3 ranges — unambiguously ≥2.
const PAYLOAD_LEN: usize = 20 * 1024 * 1024;
/// Below `PAYLOAD_LEN`, so download takes the ranged path.
const THRESHOLD_BYTES: i64 = 8 * 1024 * 1024;

#[derive(Default)]
struct GcsMockState {
    payload: Vec<u8>,
    digest: String,
    uploaded: AtomicBool,
    head_calls: AtomicUsize,
    ranged_get_calls: AtomicUsize,
    full_get_calls: AtomicUsize,
}

#[derive(Clone)]
struct GcsMock {
    state: Arc<GcsMockState>,
}

impl Respond for GcsMock {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let s = &self.state;
        let method = request.method.as_str();

        match method {
            "PUT" => {
                s.uploaded.store(true, Ordering::Relaxed);
                ResponseTemplate::new(200)
            }
            "HEAD" => {
                s.head_calls.fetch_add(1, Ordering::Relaxed);
                if !s.uploaded.load(Ordering::Relaxed) {
                    return ResponseTemplate::new(404);
                }
                ResponseTemplate::new(200)
                    .insert_header(GCS_META_SFC_DIGEST, s.digest.as_str())
                    .insert_header("content-length", s.payload.len().to_string().as_str())
            }
            "GET" => match request.headers.get("range") {
                Some(range) => {
                    let (start, end) = parse_range(range.to_str().unwrap(), s.payload.len());
                    s.ranged_get_calls.fetch_add(1, Ordering::Relaxed);
                    ResponseTemplate::new(206)
                        .insert_header(
                            "Content-Range",
                            format!("bytes {start}-{end}/{}", s.payload.len()).as_str(),
                        )
                        .insert_header(GCS_META_SFC_DIGEST, s.digest.as_str())
                        .set_body_bytes(s.payload[start..=end].to_vec())
                }
                None => {
                    s.full_get_calls.fetch_add(1, Ordering::Relaxed);
                    ResponseTemplate::new(200)
                        .insert_header(GCS_META_SFC_DIGEST, s.digest.as_str())
                        .set_body_bytes(s.payload.clone())
                }
            },
            _ => ResponseTemplate::new(400),
        }
    }
}

fn gcs_stage(endpoint: &str) -> StageInfo {
    StageInfo {
        location_type: LocationType::Gcs,
        bucket: "test-bucket".to_string(),
        key_prefix: String::new(),
        region: "us-central1".to_string(),
        creds: CloudCredentials::Gcs {
            gcs_access_token: Some(SensitiveString::from("test-bearer-token")),
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

#[tokio::test(flavor = "multi_thread")]
async fn should_upload_via_single_put_and_download_via_ranged_get() {
    let payload = make_payload(PAYLOAD_LEN);
    let digest =
        compute_sha256_digest(&ByteSource::Bytes(payload.clone().into())).expect("compute digest");
    let expected_ranges = MultipartConfig::GCS
        .expected_part_count(PAYLOAD_LEN as u64)
        .expect("payload within GCS object limits");
    assert!(
        expected_ranges >= 2,
        "payload must split into >=2 download ranges"
    );
    assert_eq!(expected_ranges, PAYLOAD_LEN.div_ceil(PART_SIZE) as u64);

    let server = MockServer::start().await;
    let state = Arc::new(GcsMockState {
        payload: payload.clone(),
        digest: digest.clone(),
        ..Default::default()
    });
    Mock::given(any())
        .respond_with(GcsMock {
            state: Arc::clone(&state),
        })
        .mount(&server)
        .await;

    let multipart = MultipartParams::from_server(Some(THRESHOLD_BYTES), Some(4));

    let upload = SingleUploadData {
        source: ByteSource::Bytes(payload.clone().into()),
        filename: "bigfile.bin".to_string(),
        stage_info: gcs_stage(&server.uri()),
        encryption_material: None,
        auto_compress: false,
        source_compression: SourceCompressionParam::None,
        overwrite: true,
        flavor: PutGetResultsetFlavor::Python,
        legacy_odbc_compression_autodetect: false,
        skip_upload_on_content_match: false,
        multipart,
        put_compress_level: 9,
        put_tempdir: None,
    };
    let upload_result = upload_single_file(
        upload,
        &RetryPolicy::put_get(&ParamStore::new()),
        TransferCtx::default(),
    )
    .await
    .expect("upload should succeed");
    assert_eq!(upload_result.status, "UPLOADED");

    let received = server
        .received_requests()
        .await
        .expect("wiremock request log should be enabled");
    assert!(
        received.iter().all(|r| r.method.as_str() != "POST"),
        "a single-PUT upload must not initiate a resumable session"
    );
    let puts: Vec<&Request> = received
        .iter()
        .filter(|r| r.method.as_str() == "PUT")
        .collect();
    assert_eq!(
        puts.len(),
        1,
        "a payload above the multipart threshold still uploads as one object PUT"
    );
    let put = puts[0];
    assert_eq!(
        put.body.as_slice(),
        payload.as_slice(),
        "the object PUT must carry the uploaded bytes"
    );
    assert_eq!(
        put.headers
            .get(GCS_META_SFC_DIGEST)
            .and_then(|v| v.to_str().ok()),
        Some(digest.as_str()),
        "digest metadata on the object PUT must match the source digest"
    );
    assert!(
        put.headers.get("content-range").is_none(),
        "a whole-object PUT must not carry Content-Range"
    );

    let output_dir = tempfile::tempdir().unwrap();
    let download = SingleDownloadData {
        src_location: "bigfile.bin".to_string(),
        local_location: output_dir.path().to_str().unwrap().to_string(),
        stage_info: gcs_stage(&server.uri()),
        encryption_material: None,
        presigned_url: None,
        flavor: PutGetResultsetFlavor::Python,
        multipart,
        unsafe_file_write: false,
    };
    download_single_file(
        download,
        &RetryPolicy::put_get(&ParamStore::new()),
        /* per_file_index */ 0,
        TransferCtx::default(),
    )
    .await
    .expect("download should succeed");

    assert!(
        state.head_calls.load(Ordering::Relaxed) >= 1,
        "download must probe object size with HEAD before ranged GETs"
    );
    assert_eq!(
        state.ranged_get_calls.load(Ordering::Relaxed) as u64,
        expected_ranges,
        "one ranged GET per range"
    );
    assert_eq!(
        state.full_get_calls.load(Ordering::Relaxed),
        0,
        "must not fall back to a single full GET"
    );

    let downloaded = std::fs::read(output_dir.path().join("bigfile.bin")).expect("read output");
    assert_eq!(downloaded.len(), payload.len(), "downloaded length matches");
    assert!(downloaded == payload, "downloaded bytes match the original");
}
