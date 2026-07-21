//! Azure block-blob multipart upload + ranged download, exercised against a
//! stateful wiremock Azure endpoint.
//!
//! No real account is needed: `MultipartParams` is injected directly with a low
//! threshold (the transfer functions take it as an argument — pure dependency
//! injection, no production code change), so a ~20 MiB file splits into multiple
//! blocks on upload and multiple byte ranges on download. The test verifies the
//! round-trip is byte-identical AND asserts the multipart/ranged protocol
//! actually fired (≥2 `Put Block` calls + one `Put Block List` commit, ≥2
//! ranged `GET`s, and no fallback to a single `Put Blob` / full `GET`). It also
//! pins the wire-level shapes documented in `azure_transfer.rs`: block ids are
//! fixed-width base64, the commit carries the `sfcdigest` metadata, and the
//! download issues a HEAD (Get Blob Properties) probe before any ranged GET.
//!
//! The Azure block *size* (4 MiB, `MultipartConfig::AZURE.default_part`) is
//! fixed for files this small, so the payload must exceed it to split — the
//! threshold only controls single-vs-multipart routing.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use base64::Engine as _;
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

/// Azure Blob Storage user-metadata header carrying Snowflake's SHA-256
/// digest. Mirrors the private constant of the same name in `azure_transfer.rs`.
const AZURE_META_SFC_DIGEST: &str = "x-ms-meta-sfcdigest";

/// Azure block-blob default block size (`MultipartConfig::AZURE.default_part`).
/// `compute_part_size` returns this for any file below the grow boundary
/// (past ~195 GiB it grows to keep the block count under 50 000).
const PART_SIZE: usize = 4 * 1024 * 1024;
/// > 4 × `PART_SIZE`, so the file splits into 5 blocks / ranges — unambiguously ≥2.
const PAYLOAD_LEN: usize = 20 * 1024 * 1024;
/// Below `PAYLOAD_LEN`, so both upload and download take the multipart path.
const THRESHOLD_BYTES: i64 = 4 * 1024 * 1024;

/// Deterministic, position-dependent payload (a tiny LCG) so that a mis-ordered
/// block or range on reassembly cannot still compare equal to the original.
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
struct AzureMockState {
    payload: Vec<u8>,
    digest: String,
    put_block_calls: AtomicUsize,
    put_block_list_calls: AtomicUsize,
    single_put_calls: AtomicUsize,
    head_calls: AtomicUsize,
    ranged_get_calls: AtomicUsize,
    full_get_calls: AtomicUsize,
}

#[derive(Clone)]
struct AzureMock {
    state: Arc<AzureMockState>,
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

impl Respond for AzureMock {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let s = &self.state;
        let method = request.method.as_str();
        let query = request.url.query().unwrap_or("");

        match method {
            // Put Block List (commit): PUT ...?comp=blocklist. Checked before the
            // `comp=block` arm below since "comp=blocklist" contains "comp=block"
            // as a substring.
            "PUT" if query.contains("comp=blocklist") => {
                s.put_block_list_calls.fetch_add(1, Ordering::Relaxed);
                ResponseTemplate::new(201)
            }
            // Put Block: PUT ...?comp=block&blockid=...
            "PUT" if query.contains("comp=block") => {
                s.put_block_calls.fetch_add(1, Ordering::Relaxed);
                ResponseTemplate::new(201)
            }
            // Single Put Blob — must NOT happen for a multipart-sized file.
            "PUT" => {
                s.single_put_calls.fetch_add(1, Ordering::Relaxed);
                ResponseTemplate::new(201)
            }
            // Get Blob Properties: reports size via Content-Length + sfcdigest.
            // download_from_azure_streaming reads the header, not the body, for
            // the size, so the body content itself is unused (zero-filled).
            "HEAD" => {
                s.head_calls.fetch_add(1, Ordering::Relaxed);
                ResponseTemplate::new(200)
                    .insert_header(AZURE_META_SFC_DIGEST, s.digest.as_str())
                    .insert_header("content-length", s.payload.len().to_string().as_str())
                    .set_body_bytes(vec![0u8; s.payload.len()])
            }
            // Get Blob — ranged (206) or full (200).
            "GET" => match request.headers.get("range") {
                Some(range) => {
                    let (start, end) = parse_range(range.to_str().unwrap(), s.payload.len());
                    s.ranged_get_calls.fetch_add(1, Ordering::Relaxed);
                    ResponseTemplate::new(206)
                        .insert_header(
                            "Content-Range",
                            format!("bytes {start}-{end}/{}", s.payload.len()).as_str(),
                        )
                        .insert_header(AZURE_META_SFC_DIGEST, s.digest.as_str())
                        .set_body_bytes(s.payload[start..=end].to_vec())
                }
                None => {
                    s.full_get_calls.fetch_add(1, Ordering::Relaxed);
                    ResponseTemplate::new(200)
                        .insert_header(AZURE_META_SFC_DIGEST, s.digest.as_str())
                        .set_body_bytes(s.payload.clone())
                }
            },
            _ => ResponseTemplate::new(400),
        }
    }
}

fn azure_stage(endpoint: &str) -> StageInfo {
    StageInfo {
        location_type: LocationType::Azure,
        bucket: "test-container".to_string(),
        key_prefix: String::new(),
        region: "eastus2".to_string(),
        creds: CloudCredentials::Azure {
            sas_token: SensitiveString::from("sv=2021-08-06&sig=test-secret-sig&se=2099-01-01"),
        },
        endpoint: Some(endpoint.to_string()),
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
        use_s3_regional_url: false,
        storage_account: Some("test".to_string()),
        tls_config: sf_core::tls::config::TlsConfig::default(),
        crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn should_upload_and_download_via_azure_multipart_roundtrip() {
    let payload = make_payload(PAYLOAD_LEN);
    let digest =
        compute_sha256_digest(&ByteSource::Bytes(payload.clone().into())).expect("compute digest");
    let expected_chunks = PAYLOAD_LEN.div_ceil(PART_SIZE);
    assert!(expected_chunks >= 2, "payload must split into >=2 chunks");

    let state = Arc::new(AzureMockState {
        payload: payload.clone(),
        digest: digest.clone(),
        ..Default::default()
    });
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(AzureMock {
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
        stage_info: azure_stage(&server.uri()),
        encryption_material: None, // SSE stage: the upload body is the raw payload.
        auto_compress: false,
        source_compression: SourceCompressionParam::None,
        overwrite: true,
        flavor: PutGetResultsetFlavor::Python,
        legacy_odbc_compression_autodetect: false,
        skip_upload_on_content_match: false,
        multipart,
    };
    let upload_result =
        upload_single_file(upload, &RetryPolicy::put_get(&ParamStore::new()), &mut None)
            .await
            .expect("upload should succeed");
    assert_eq!(upload_result.status, "UPLOADED");

    assert_eq!(
        state.put_block_list_calls.load(Ordering::Relaxed),
        1,
        "exactly one Put Block List commit"
    );
    assert_eq!(
        state.single_put_calls.load(Ordering::Relaxed),
        0,
        "must not fall back to a single Put Blob"
    );
    assert_eq!(
        state.put_block_calls.load(Ordering::Relaxed),
        expected_chunks,
        "one Put Block per chunk"
    );

    // Pin the wire-level shapes documented in `azure_transfer.rs`: block ids are
    // fixed-width base64, and the digest metadata rides on the commit request.
    let received = server
        .received_requests()
        .await
        .expect("wiremock request log should be enabled");
    let block_ids: Vec<String> = received
        .iter()
        .filter(|r| {
            r.method.as_str() == "PUT"
                && r.url
                    .query()
                    .is_some_and(|q| q.contains("comp=block") && !q.contains("comp=blocklist"))
        })
        .map(|r| {
            r.url
                .query_pairs()
                .find(|(k, _)| k == "blockid")
                .map(|(_, v)| v.into_owned())
                .expect("every Put Block request must carry a blockid query param")
        })
        .collect();
    assert_eq!(
        block_ids.len(),
        expected_chunks,
        "one blockid per Put Block call"
    );
    let decoded_lens: Vec<usize> = block_ids
        .iter()
        .map(|id| {
            base64::engine::general_purpose::STANDARD
                .decode(id)
                .expect("block id must be valid base64")
                .len()
        })
        .collect();
    assert!(
        decoded_lens.iter().all(|&len| len == decoded_lens[0]),
        "every block id must decode to the same fixed width, got lengths {decoded_lens:?}"
    );

    let commit = received
        .iter()
        .find(|r| {
            r.method.as_str() == "PUT"
                && r.url.query().is_some_and(|q| q.contains("comp=blocklist"))
        })
        .expect("a Put Block List commit must be sent");
    assert!(
        commit.headers.get(AZURE_META_SFC_DIGEST).is_some(),
        "digest metadata must ride on the Put Block List commit"
    );

    // ---- Download (ranged) ----
    let output_dir = tempfile::tempdir().unwrap();
    let download = SingleDownloadData {
        src_location: "bigfile.bin".to_string(),
        local_location: output_dir.path().to_str().unwrap().to_string(),
        stage_info: azure_stage(&server.uri()),
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
    )
    .await
    .expect("download should succeed");

    assert!(
        state.head_calls.load(Ordering::Relaxed) >= 1,
        "download must probe blob properties with HEAD before ranged GETs"
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
