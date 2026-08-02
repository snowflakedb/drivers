//! GCS resumable-upload multipart upload + ranged download, exercised against
//! a stateful wiremock GCS endpoint.
//!
//! No real account is needed: `MultipartParams` is injected directly with a low
//! threshold (the transfer functions take it as an argument — pure dependency
//! injection, no production code change), so a ~20 MiB file splits into multiple
//! chunks on upload and multiple byte ranges on download. The test verifies the
//! round-trip is byte-identical AND asserts the resumable/ranged protocol
//! actually fired (≥2 chunk `PUT`s against the resumable session URL, ≥2 ranged
//! `GET`s, and no fallback to a single object `PUT` / full `GET`). It also pins
//! the wire-level shapes documented in `gcs_transfer.rs`: the digest metadata
//! rides on the resumable-session initiation `POST` (not on the chunk `PUT`s),
//! and the download issues a HEAD probe before any ranged GET.
//!
//! The GCS *chunk size* (8 MiB, `MultipartConfig::GCS.default_part`) is fixed
//! for files this small, so the payload must exceed it to split — the
//! threshold only controls single-vs-resumable routing.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use sf_core::apis::database_driver_v1::PutGetResultsetFlavor;
use sf_core::config::param_registry::DEFAULT_PUT_GET_MAX_ATTEMPTS;
use sf_core::config::param_store::ParamStore;
use sf_core::config::retry::RetryPolicy;
use sf_core::file_manager::internal::compute_sha256_digest;
use sf_core::file_manager::internal::gcs_test_retry_policy as test_policy;
use sf_core::file_manager::types::{
    ByteSource, CloudCredentials, LocationType, SingleDownloadData, SingleUploadData, StageInfo,
};
use sf_core::file_manager::{
    MultipartParams, RefreshFuture, SourceCompressionParam, StageInfoCache, StageInfoRefresher,
    StageInfoSnapshot, download_single_file, upload_single_file,
};
use sf_core::sensitive::SensitiveString;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use crate::http::multipart_test_support::{make_payload, parse_range};

/// GCS user-metadata header carrying Snowflake's SHA-256 digest. Mirrors the
/// private constant of the same name in `gcs_transfer.rs`.
const GCS_META_SFC_DIGEST: &str = "x-goog-meta-sfc-digest";

/// GCS resumable default chunk size (`MultipartConfig::GCS.default_part`).
/// `compute_part_size` returns this for any file, since GCS resumable has no
/// `max_parts` cap to grow past it.
const PART_SIZE: usize = 8 * 1024 * 1024;
/// > 2 × `PART_SIZE`, so the file splits into 3 chunks / ranges — unambiguously ≥2.
const PAYLOAD_LEN: usize = 20 * 1024 * 1024;
/// Below `PAYLOAD_LEN`, so both upload and download take the resumable/ranged path.
const THRESHOLD_BYTES: i64 = 8 * 1024 * 1024;

#[derive(Default)]
struct GcsMockState {
    payload: Vec<u8>,
    digest: String,
    /// Base URL of the mock server, used to mint the resumable session URL
    /// returned in the initiation response's `Location` header.
    base_url: String,
    /// Flips to `true` once the final chunk PUT completes the resumable
    /// session, mirroring real GCS: the object genuinely doesn't exist until
    /// then. Before that, HEAD must report 404 — otherwise the pre-upload
    /// existence/digest-match check in `upload_to_gcs_or_skip` (which HEADs
    /// the object first) would see a matching digest and skip the upload
    /// entirely (GCS skips even under `OVERWRITE=TRUE` on a digest match).
    uploaded: AtomicBool,
    initiate_calls: AtomicUsize,
    put_chunk_calls: AtomicUsize,
    single_put_calls: AtomicUsize,
    head_calls: AtomicUsize,
    ranged_get_calls: AtomicUsize,
    full_get_calls: AtomicUsize,
    /// The first N resumable-session initiation `POST`s return `503`
    /// (transient) instead of the normal 200+Location success. `0` (the
    /// default) means never fail.
    initiate_fail_first_n: AtomicUsize,
    /// The chunk `PUT` whose 1-based ordinal (across the whole test, i.e.
    /// counting every session's chunk PUTs) equals this value returns `401`
    /// instead of the normal 308/200. `0` (the default) means never fail.
    /// Because the ordinal is a running total, it fires exactly once even
    /// across a full session re-initiate.
    fail_chunk_put_number: AtomicUsize,
}

#[derive(Clone)]
struct GcsMock {
    state: Arc<GcsMockState>,
}

/// Parse an outbound `Content-Range: bytes START-END/TOTAL` header (the shape
/// `gcs_put_one_chunk` sends on each resumable chunk PUT).
fn parse_content_range(value: &str) -> (u64, u64, u64) {
    let spec = value.trim().trim_start_matches("bytes ");
    let (range, total) = spec
        .split_once('/')
        .expect("Content-Range must have a total");
    let (start, end) = range
        .split_once('-')
        .expect("Content-Range must have a range");
    (
        start.parse().unwrap(),
        end.parse().unwrap(),
        total.parse().unwrap(),
    )
}

impl Respond for GcsMock {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let s = &self.state;
        let method = request.method.as_str();
        let path = request.url.path();

        match method {
            // Resumable session initiation: POST to the object URL itself
            // (`x-goog-resumable: start`). Mints a session URL under a distinct
            // path prefix so chunk PUTs are unambiguously routed below.
            "POST" => {
                let call_no = s.initiate_calls.fetch_add(1, Ordering::Relaxed);
                if call_no < s.initiate_fail_first_n.load(Ordering::Relaxed) {
                    return ResponseTemplate::new(503);
                }
                let session_url = format!("{}/upload/mock-session-id", s.base_url);
                ResponseTemplate::new(200).insert_header("Location", session_url.as_str())
            }
            // Resumable chunk PUT: PUT to the minted session URL. Reads the
            // Content-Range to decide whether this is the final chunk (200/201
            // ends the session) or an intermediate one (308 Resume Incomplete).
            "PUT" if path.starts_with("/upload/") => {
                let call_no = s.put_chunk_calls.fetch_add(1, Ordering::Relaxed) + 1;
                if call_no == s.fail_chunk_put_number.load(Ordering::Relaxed) {
                    return ResponseTemplate::new(401);
                }
                let content_range = request
                    .headers
                    .get("content-range")
                    .and_then(|v| v.to_str().ok())
                    .expect("every resumable chunk PUT must carry Content-Range");
                let (_start, end, total) = parse_content_range(content_range);
                if end + 1 == total {
                    s.uploaded.store(true, Ordering::Relaxed);
                    ResponseTemplate::new(200)
                } else {
                    ResponseTemplate::new(308)
                }
            }
            // Single object PUT — must NOT happen for a multipart-sized file.
            "PUT" => {
                s.single_put_calls.fetch_add(1, Ordering::Relaxed);
                ResponseTemplate::new(200)
            }
            // HEAD probe: 404 before the object is uploaded (the pre-upload
            // existence/digest-match check in `upload_to_gcs_or_skip` must see
            // "not found" so the upload actually proceeds); once uploaded,
            // reports size via Content-Length + sfc-digest for the download
            // path's size probe. Callers read only the headers, so the body
            // content itself is unused (zero-filled).
            "HEAD" => {
                s.head_calls.fetch_add(1, Ordering::Relaxed);
                if !s.uploaded.load(Ordering::Relaxed) {
                    return ResponseTemplate::new(404);
                }
                ResponseTemplate::new(200)
                    .insert_header(GCS_META_SFC_DIGEST, s.digest.as_str())
                    .insert_header("content-length", s.payload.len().to_string().as_str())
                    .set_body_bytes(vec![0u8; s.payload.len()])
            }
            // GET — ranged (206) or full (200).
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
            // Best-effort session abort on failure.
            "DELETE" => ResponseTemplate::new(204),
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
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn should_upload_and_download_via_gcs_multipart_roundtrip() {
    let payload = make_payload(PAYLOAD_LEN);
    let digest =
        compute_sha256_digest(&ByteSource::Bytes(payload.clone().into())).expect("compute digest");
    let expected_chunks = PAYLOAD_LEN.div_ceil(PART_SIZE);
    assert!(expected_chunks >= 2, "payload must split into >=2 chunks");

    let server = MockServer::start().await;
    let state = Arc::new(GcsMockState {
        payload: payload.clone(),
        digest: digest.clone(),
        base_url: server.uri(),
        ..Default::default()
    });
    Mock::given(any())
        .respond_with(GcsMock {
            state: Arc::clone(&state),
        })
        .mount(&server)
        .await;

    // Inject a low threshold directly — no connection parameter, no global state.
    let multipart = MultipartParams::from_server(Some(THRESHOLD_BYTES), Some(4));

    // ---- Upload (resumable) ----
    let upload = SingleUploadData {
        source: ByteSource::Bytes(payload.clone().into()),
        filename: "bigfile.bin".to_string(),
        stage_info: gcs_stage(&server.uri()),
        encryption_material: None, // SSE stage: the upload body is the raw payload.
        auto_compress: false,
        source_compression: SourceCompressionParam::None,
        overwrite: true,
        flavor: PutGetResultsetFlavor::Python,
        legacy_odbc_compression_autodetect: false,
        skip_upload_on_content_match: false,
        multipart,
    };
    let upload_result = upload_single_file(upload, &RetryPolicy::put_get(&ParamStore::new()), None)
        .await
        .expect("upload should succeed");
    assert_eq!(upload_result.status, "UPLOADED");

    assert_eq!(
        state.initiate_calls.load(Ordering::Relaxed),
        1,
        "exactly one resumable session initiation"
    );
    assert_eq!(
        state.single_put_calls.load(Ordering::Relaxed),
        0,
        "must not fall back to a single object PUT"
    );
    assert_eq!(
        state.put_chunk_calls.load(Ordering::Relaxed),
        expected_chunks,
        "one chunk PUT per chunk"
    );

    // Pin the wire-level shapes documented in `gcs_transfer.rs`: the digest
    // metadata rides on the initiation POST, not on the chunk PUTs.
    let received = server
        .received_requests()
        .await
        .expect("wiremock request log should be enabled");
    let initiate = received
        .iter()
        .find(|r| r.method.as_str() == "POST")
        .expect("a resumable session initiation POST must be sent");
    assert!(
        initiate.headers.get(GCS_META_SFC_DIGEST).is_some(),
        "digest metadata must ride on the resumable session initiation"
    );

    let chunk_puts: Vec<&Request> = received
        .iter()
        .filter(|r| r.method.as_str() == "PUT" && r.url.path().starts_with("/upload/"))
        .collect();
    assert_eq!(chunk_puts.len(), expected_chunks);
    assert!(
        chunk_puts
            .iter()
            .all(|r| r.headers.get(GCS_META_SFC_DIGEST).is_none()),
        "digest metadata must NOT ride on the per-chunk PUTs"
    );

    // ---- Download (ranged) ----
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
    download_single_file(download, &RetryPolicy::put_get(&ParamStore::new()), 0, None)
        .await
        .expect("download should succeed");

    assert!(
        state.head_calls.load(Ordering::Relaxed) >= 1,
        "download must probe object size with HEAD before ranged GETs"
    );
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

/// Gap: a transient 5xx on the resumable-session-initiation `POST` is
/// retried in place (via `gcs_request_with_retry`), rather than failing the
/// whole upload on the first hiccup.
#[tokio::test(flavor = "multi_thread")]
async fn should_retry_gcs_resumable_initiation_on_transient_5xx() {
    let payload = make_payload(PAYLOAD_LEN);
    let digest =
        compute_sha256_digest(&ByteSource::Bytes(payload.clone().into())).expect("compute digest");
    let expected_chunks = PAYLOAD_LEN.div_ceil(PART_SIZE);

    let server = MockServer::start().await;
    let state = Arc::new(GcsMockState {
        payload: payload.clone(),
        digest: digest.clone(),
        base_url: server.uri(),
        initiate_fail_first_n: 1.into(),
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
    };
    let upload_result = upload_single_file(
        upload,
        &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
        None,
    )
    .await
    .expect("upload should succeed after the initiate POST retries past the transient 503");
    assert_eq!(upload_result.status, "UPLOADED");

    assert_eq!(
        state.initiate_calls.load(Ordering::Relaxed),
        2,
        "one failed initiate POST (503) plus one retried initiate POST that succeeds"
    );
    assert_eq!(
        state.put_chunk_calls.load(Ordering::Relaxed),
        expected_chunks,
        "the initiate retry happens before any chunk PUT, so chunk PUTs are unaffected"
    );
}

/// A minimal `StageInfoRefresher` local to this file (mirrors
/// `gcs_retry.rs`'s private `FakeRefresher`, which cannot be imported across
/// the `mod gcs_retry;` boundary) — only `refresh()` is exercised by the
/// resumable/access-token upload path's 401 recovery.
struct GcsChunkFakeRefresher {
    cache: StageInfoCache,
    fresh_token: String,
    refresh_calls: AtomicUsize,
}

impl GcsChunkFakeRefresher {
    fn new(stale_token: &str, fresh_token: &str) -> Self {
        Self {
            cache: StageInfoCache::new(StageInfoSnapshot {
                creds: CloudCredentials::Gcs {
                    gcs_access_token: Some(SensitiveString::from(stale_token)),
                },
                presigned_url: None,
                presigned_urls: None,
            }),
            fresh_token: fresh_token.to_string(),
            refresh_calls: AtomicUsize::new(0),
        }
    }
}

impl StageInfoRefresher for GcsChunkFakeRefresher {
    fn refresh(&self, _observed: Instant) -> RefreshFuture<'_> {
        self.refresh_calls.fetch_add(1, Ordering::Relaxed);
        self.cache.store(StageInfoSnapshot {
            creds: CloudCredentials::Gcs {
                gcs_access_token: Some(SensitiveString::from(self.fresh_token.clone())),
            },
            presigned_url: None,
            presigned_urls: None,
        });
        let new_gen = self.cache.cached_at();
        Box::pin(async move { Ok(new_gen) })
    }

    fn refresh_url(&self, _current_upload_file: Option<&str>) -> RefreshFuture<'_> {
        unreachable!(
            "the GCS access-token resumable-upload path only calls refresh(), never refresh_url()"
        );
    }

    fn cache(&self) -> &StageInfoCache {
        &self.cache
    }
}

/// Gap: a 401 partway through the resumable chunk PUTs triggers a full
/// session re-initiate via the token-refresh loop (`run_gcs_with_token_refresh`
/// re-runs the *entire* attempt closure on `TokenExpired` — pre-upload HEAD
/// included — not just a resume of the half-finished session), then the
/// second, fully-fresh attempt commits on a terminal 2xx.
#[tokio::test(flavor = "multi_thread")]
async fn should_reinitiate_gcs_resumable_session_after_401_mid_chunk() {
    let payload = make_payload(PAYLOAD_LEN);
    let digest =
        compute_sha256_digest(&ByteSource::Bytes(payload.clone().into())).expect("compute digest");
    let expected_chunks = PAYLOAD_LEN.div_ceil(PART_SIZE);
    assert!(expected_chunks >= 2, "payload must split into >=2 chunks");

    let server = MockServer::start().await;
    let state = Arc::new(GcsMockState {
        payload: payload.clone(),
        digest: digest.clone(),
        base_url: server.uri(),
        // Fail the very first chunk PUT of the whole test (the first chunk
        // of the first, doomed session) with a 401, so the first attempt
        // sends exactly one chunk PUT before aborting.
        fail_chunk_put_number: 1.into(),
        ..Default::default()
    });
    Mock::given(any())
        .respond_with(GcsMock {
            state: Arc::clone(&state),
        })
        .mount(&server)
        .await;

    let mut stage = gcs_stage(&server.uri());
    stage.creds = CloudCredentials::Gcs {
        gcs_access_token: Some(SensitiveString::from("stale-token")),
    };

    let fake = GcsChunkFakeRefresher::new("stale-token", "fresh-token");

    let multipart = MultipartParams::from_server(Some(THRESHOLD_BYTES), Some(4));
    let upload = SingleUploadData {
        source: ByteSource::Bytes(payload.clone().into()),
        filename: "bigfile.bin".to_string(),
        stage_info: stage,
        encryption_material: None,
        auto_compress: false,
        source_compression: SourceCompressionParam::None,
        overwrite: true,
        flavor: PutGetResultsetFlavor::Python,
        legacy_odbc_compression_autodetect: false,
        skip_upload_on_content_match: false,
        multipart,
    };
    let refresher_opt = Some(&fake as &dyn StageInfoRefresher);
    let upload_result = upload_single_file(
        upload,
        &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
        refresher_opt,
    )
    .await
    .expect("401 mid-chunk should trigger a full re-initiate via token refresh and then succeed");
    assert_eq!(upload_result.status, "UPLOADED");

    assert_eq!(
        fake.refresh_calls.load(Ordering::Relaxed),
        1,
        "refresh() fires exactly once on the 401"
    );
    assert_eq!(
        state.initiate_calls.load(Ordering::Relaxed),
        2,
        "one aborted session (first attempt) plus one fresh session (second, successful attempt)"
    );
    assert_eq!(
        state.put_chunk_calls.load(Ordering::Relaxed),
        1 + expected_chunks,
        "one chunk PUT from the aborted first attempt (the one that got 401'd) \
         plus a full set of chunk PUTs from the second, successful attempt"
    );

    // Pin that the resumable path actually re-reads the rotated token, not
    // just that it blindly retries: the failed chunk PUT (ordinal ==
    // fail_chunk_put_number, i.e. the first one seen) must have carried the
    // stale token, and every chunk PUT after it must carry the fresh one.
    let received = server
        .received_requests()
        .await
        .expect("wiremock request log should be enabled");
    let chunk_put_auth_headers: Vec<String> = received
        .iter()
        .filter(|r| r.method.as_str() == "PUT" && r.url.path().starts_with("/upload/"))
        .map(|r| {
            r.headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    assert_eq!(chunk_put_auth_headers.len(), 1 + expected_chunks);
    assert!(
        chunk_put_auth_headers[0].ends_with("stale-token"),
        "the chunk PUT that got 401'd should have carried the stale bearer token"
    );
    assert!(
        chunk_put_auth_headers[1..]
            .iter()
            .all(|auth| auth.ends_with("fresh-token")),
        "every chunk PUT after the refresh should carry the rotated bearer token"
    );
}
