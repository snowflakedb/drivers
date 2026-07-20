use bytes::Bytes;
use flate2::Compression;
use flate2::write::GzEncoder;
use sf_core::apis::database_driver_v1::PutGetResultsetFlavor;
use sf_core::config::param_registry::DEFAULT_PUT_GET_MAX_ATTEMPTS;
use sf_core::config::param_store::ParamStore;
use sf_core::config::retry::RetryPolicy;
// Zero-backoff policy shared with the in-crate unit tests via `internal`, so
// retry tests inject the real shape with backoff zeroed instead of sleeping.
use sf_core::file_manager::internal::gcs_test_retry_policy as test_policy;
use sf_core::file_manager::types::ByteSource;
use sf_core::file_manager::{
    CloudCredentials, DownloadData, GcsDownloadError, GcsUploadError, LocationType,
    MultipartParams, PreparedUpload, RefreshFuture, StageInfo, StageInfoCache, StageInfoRefresher,
    StageInfoSnapshot, download_files,
};
use sf_core::sensitive::SensitiveString;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Helper to build a StageInfo with a presigned URL pointing at the mock server.
fn gcs_stage_with_presigned_url(presigned_url: &str) -> StageInfo {
    StageInfo {
        location_type: LocationType::Gcs,
        bucket: "test-bucket".to_string(),
        key_prefix: "prefix/".to_string(),
        region: "us-central1".to_string(),
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        endpoint: None,
        presigned_url: Some(presigned_url.to_string()),
        use_virtual_url: false,
        use_regional_url: false,
        use_s3_regional_url: false,
        tls_config: sf_core::tls::config::TlsConfig::default(),
        crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
        storage_account: None,
    }
}

/// Helper to build a StageInfo with a bearer token and custom endpoint pointing at mock.
fn gcs_stage_with_token(endpoint: &str) -> StageInfo {
    StageInfo {
        location_type: LocationType::Gcs,
        bucket: "test-bucket".to_string(),
        key_prefix: "prefix/".to_string(),
        region: "us-central1".to_string(),
        creds: CloudCredentials::Gcs {
            gcs_access_token: Some(SensitiveString::from("test-bearer-token")),
        },
        endpoint: Some(endpoint.to_string()),
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
        use_s3_regional_url: false,
        tls_config: sf_core::tls::config::TlsConfig::default(),
        crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
        storage_account: None,
    }
}

fn gcs_response_headers() -> ResponseTemplate {
    let enc_data = serde_json::json!({
        "EncryptionMode": "FullBlob",
        "WrappedContentKey": {
            "KeyId": "symmKey1",
            "EncryptedKey": "dGVzdC1rZXk=",
            "Algorithm": "AES_CBC_256"
        },
        "ContentEncryptionIV": "dGVzdC1pdg=="
    });
    let mat_desc = serde_json::json!({
        "queryId": "test-query",
        "smkId": "1",
        "keySize": "256"
    });
    ResponseTemplate::new(200)
        .set_body_bytes(b"encrypted-data".to_vec())
        .insert_header("x-goog-meta-sfc-digest", "test-digest")
        .insert_header("x-goog-meta-encryptiondata", enc_data.to_string().as_str())
        .insert_header("x-goog-meta-matdesc", mat_desc.to_string().as_str())
}

// ---------------------------------------------------------------
// 401 → TokenExpired (matches JDBC error401RenewExpired,
//   Python test_get_gcp_file_object_http_recoverable_error_refresh_with_downscoped,
//   ODBC test_token_renew_*)
// ---------------------------------------------------------------

#[tokio::test]
async fn gcs_download_401_returns_token_expired() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/download"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthenticated"))
        .mount(&server)
        .await;

    let stage = gcs_stage_with_presigned_url(&format!("{}/download", server.uri()));
    let result = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, DEFAULT_PUT_GET_MAX_ATTEMPTS),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    let err = result.unwrap_err();
    assert!(
        matches!(err, GcsDownloadError::TokenExpired { .. }),
        "401 should produce TokenExpired error, got: {err:?}"
    );
}

// ---------------------------------------------------------------
// 403 is retryable (matches ODBC is_retryable_http_code,
//   JDBC RestRequestTest with retryHTTP403=true)
// ---------------------------------------------------------------

#[tokio::test]
async fn gcs_download_403_is_retried_then_succeeds() {
    let server = MockServer::start().await;
    let attempt = Arc::new(AtomicU32::new(0));

    let attempt_clone = attempt.clone();
    Mock::given(method("GET"))
        .and(path("/download"))
        .respond_with(move |_: &Request| {
            let n = attempt_clone.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                ResponseTemplate::new(403).set_body_string("Forbidden")
            } else {
                gcs_response_headers()
            }
        })
        .mount(&server)
        .await;

    let stage = gcs_stage_with_presigned_url(&format!("{}/download", server.uri()));
    let result = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, DEFAULT_PUT_GET_MAX_ATTEMPTS),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(result.is_ok(), "403 should be retried and succeed");
    assert_eq!(
        attempt.load(Ordering::SeqCst),
        2,
        "should have retried once"
    );
}

// ---------------------------------------------------------------
// 400 retryable only for presigned URLs
// (matches Python _has_expired_presigned_url)
// ---------------------------------------------------------------

#[tokio::test]
async fn gcs_download_400_with_presigned_url_is_retried() {
    let server = MockServer::start().await;
    let attempt = Arc::new(AtomicU32::new(0));

    let attempt_clone = attempt.clone();
    Mock::given(method("GET"))
        .and(path("/download"))
        .respond_with(move |_: &Request| {
            let n = attempt_clone.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                ResponseTemplate::new(400).set_body_string("Bad Request")
            } else {
                gcs_response_headers()
            }
        })
        .mount(&server)
        .await;

    let stage = gcs_stage_with_presigned_url(&format!("{}/download", server.uri()));
    let result = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, DEFAULT_PUT_GET_MAX_ATTEMPTS),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(
        result.is_ok(),
        "400 with presigned URL should be retried and succeed"
    );
    assert_eq!(attempt.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn gcs_download_400_without_presigned_url_is_not_retried() {
    let server = MockServer::start().await;
    let attempt = Arc::new(AtomicU32::new(0));

    let attempt_clone = attempt.clone();
    Mock::given(method("GET"))
        .and(path("/test-bucket/prefix/file.csv"))
        .respond_with(move |_: &Request| {
            attempt_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(400).set_body_string("Bad Request")
        })
        .mount(&server)
        .await;

    let stage = gcs_stage_with_token(&server.uri());
    let result = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(
        result.is_err(),
        "400 without presigned URL should fail immediately"
    );
    assert_eq!(
        attempt.load(Ordering::SeqCst),
        1,
        "should NOT retry 400 without presigned URL"
    );
}

// ---------------------------------------------------------------
// 404 is a hard failure (not retried)
// ---------------------------------------------------------------

#[tokio::test]
async fn gcs_download_404_is_not_retried() {
    let server = MockServer::start().await;
    let attempt = Arc::new(AtomicU32::new(0));

    let attempt_clone = attempt.clone();
    Mock::given(method("GET"))
        .and(path("/download"))
        .respond_with(move |_: &Request| {
            attempt_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(404).set_body_string("Not Found")
        })
        .mount(&server)
        .await;

    let stage = gcs_stage_with_presigned_url(&format!("{}/download", server.uri()));
    let result = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, DEFAULT_PUT_GET_MAX_ATTEMPTS),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(result.is_err(), "404 should be a hard failure");
    assert_eq!(attempt.load(Ordering::SeqCst), 1, "should NOT retry 404");
}

// ---------------------------------------------------------------
// Standard retryable codes (408, 429, 500, 503) are retried
// (matches all drivers)
// ---------------------------------------------------------------

#[tokio::test]
async fn gcs_download_503_is_retried_then_succeeds() {
    let server = MockServer::start().await;
    let attempt = Arc::new(AtomicU32::new(0));

    let attempt_clone = attempt.clone();
    Mock::given(method("GET"))
        .and(path("/download"))
        .respond_with(move |_: &Request| {
            let n = attempt_clone.fetch_add(1, Ordering::SeqCst);
            if n < 2 {
                ResponseTemplate::new(503).set_body_string("Service Unavailable")
            } else {
                gcs_response_headers()
            }
        })
        .mount(&server)
        .await;

    let stage = gcs_stage_with_presigned_url(&format!("{}/download", server.uri()));
    let result = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, DEFAULT_PUT_GET_MAX_ATTEMPTS),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(
        result.is_ok(),
        "503 should be retried and eventually succeed"
    );
    assert_eq!(
        attempt.load(Ordering::SeqCst),
        3,
        "should have retried twice"
    );
}

// ---------------------------------------------------------------
// Response-side gzip auto-decode is disabled on the GCS client
// (matches JDBC `HttpUtil.disableContentCompression()` at
//   `HttpUtil.java:420`, used by `SnowflakeGCSClient.java:237,:432`;
//  Python `remove_content_encoding` hook at `storage_client.py:54-59`
//   — see `--gcp--/2.6-response_gzip_workaround.md`).
//
// External tooling (`gsutil cp -Z`, BigQuery exports, customer ETL)
// can land objects on a stage whose stored metadata advertises
// `Content-Encoding: gzip` while the body is the raw payload (or, for
// CSE stages, ciphertext). The driver must hand the body to the caller
// verbatim — otherwise CSE decrypt and the SHA-256/Content-Length
// checks (gaps 2.3, 2.5) silently fail.
// ---------------------------------------------------------------

fn gzip_encode(payload: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(payload).expect("gzip encode write");
    encoder.finish().expect("gzip encode finish")
}

/// A GCS response that claims `Content-Encoding: gzip` but ships a
/// non-gzip body. With reqwest auto-decompression on, the body reader
/// would either error (gunzip on non-gzip bytes) or return decoded
/// garbage; either way the caller wouldn't see the wire bytes.
#[tokio::test]
async fn gcs_download_content_encoding_gzip_with_non_gzip_body_is_returned_verbatim() {
    let server = MockServer::start().await;

    let payload: &[u8] = b"hello world (raw plaintext, NOT gzip)";

    Mock::given(method("GET"))
        .and(path("/download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(payload.to_vec())
                .insert_header("content-encoding", "gzip")
                .insert_header("x-goog-meta-sfc-digest", "test-digest"),
        )
        .mount(&server)
        .await;

    let stage = gcs_stage_with_presigned_url(&format!("{}/download", server.uri()));
    let result = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, 0),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    let response = result.expect(
        "download must succeed: reqwest auto-gunzip must be disabled on the GCS client \
         (otherwise the body reader errors on non-gzip bytes)",
    );
    assert_eq!(
        response.data, payload,
        "wire body bytes must reach the caller verbatim (no auto-decode)"
    );
    assert_eq!(
        response.cloud_byte_count,
        payload.len() as i64,
        "cloud_byte_count must reflect the wire bytes"
    );
}

/// Even when the body is *valid* gzip and the header says `gzip`, the
/// driver must hand the caller the compressed wire bytes — proving the
/// auto-decoder is off (positive byte-equality, not just "did not
/// error"). This is the case that matters for CSE: ciphertext that
/// happens to follow the gzip magic must not be re-decoded.
#[tokio::test]
async fn gcs_download_content_encoding_gzip_with_gzip_body_is_not_decoded() {
    let server = MockServer::start().await;

    let raw_payload: &[u8] = b"raw bytes that were gzipped on the way in";
    let gzipped = gzip_encode(raw_payload);
    assert_ne!(
        gzipped, raw_payload,
        "sanity: gzip output should differ from input"
    );

    let body_for_mock = gzipped.clone();
    Mock::given(method("GET"))
        .and(path("/download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(body_for_mock)
                .insert_header("content-encoding", "gzip")
                .insert_header("x-goog-meta-sfc-digest", "test-digest"),
        )
        .mount(&server)
        .await;

    let stage = gcs_stage_with_presigned_url(&format!("{}/download", server.uri()));
    let response = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, 0),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("download must succeed");

    assert_eq!(
        response.data, gzipped,
        "driver must return the gzipped wire bytes, NOT the decoded payload"
    );
    assert_ne!(
        response.data, raw_payload,
        "if this fires, reqwest auto-gunzip ran — the .no_gzip() fix has regressed"
    );
}

/// Regression guard: a response with no `Content-Encoding` header
/// behaves identically to the pre-fix happy path.
#[tokio::test]
async fn gcs_download_without_content_encoding_header_is_unchanged() {
    let server = MockServer::start().await;

    let payload: &[u8] = b"plain body, no Content-Encoding header";

    Mock::given(method("GET"))
        .and(path("/download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(payload.to_vec())
                .insert_header("x-goog-meta-sfc-digest", "test-digest"),
        )
        .mount(&server)
        .await;

    let stage = gcs_stage_with_presigned_url(&format!("{}/download", server.uri()));
    let response = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, 0),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("happy-path download must still succeed");

    assert_eq!(response.data, payload);
}

// ---------------------------------------------------------------
// Content-Length verification (gap 2.5)
//
// download_from_gcs compares the response Content-Length header
// against the actual body byte count. The check is skipped when
// Content-Length is absent, Content-Encoding is present, or the
// header value is malformed.
// ---------------------------------------------------------------

#[tokio::test]
async fn gcs_download_content_length_match_succeeds() {
    let server = MockServer::start().await;
    let body = b"hello world";

    Mock::given(method("GET"))
        .and(path("/download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(body.to_vec())
                .insert_header("x-goog-meta-sfc-digest", "test-digest"),
        )
        .mount(&server)
        .await;

    let stage = gcs_stage_with_presigned_url(&format!("{}/download", server.uri()));
    let response = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, 0),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("matching Content-Length must succeed");

    assert_eq!(response.data, body);
    assert_eq!(response.cloud_byte_count, body.len() as i64);
}

/// When the server announces more bytes in Content-Length than it sends,
/// reqwest/hyper detects the truncation (IncompleteMessage) before our
/// code gets a chance to compare lengths. The mismatch surfaces as an
/// HTTP transport error. Our `ContentLengthMismatch` variant is the
/// safety net for cases hyper does not catch (e.g. HTTP/2 framing
/// edge cases, or future library changes).
#[tokio::test]
async fn gcs_download_content_length_mismatch_truncated_body_is_http_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf).await;
        let response = "HTTP/1.1 200 OK\r\n\
             Content-Length: 100\r\n\
             x-goog-meta-sfc-digest: test-digest\r\n\
             Connection: close\r\n\r\n\
             only-14-bytes!";
        stream.write_all(response.as_bytes()).await.unwrap();
        let _ = stream.shutdown().await;
    });

    let presigned_url = format!("http://{addr}/download");
    let stage = gcs_stage_with_presigned_url(&presigned_url);
    let result = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, 0),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    server.await.unwrap();
    assert!(
        result.is_err(),
        "truncated body (Content-Length > actual bytes) must fail"
    );
}

#[tokio::test]
async fn gcs_download_no_content_length_header_succeeds() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"chunked body".to_vec())
                .insert_header("transfer-encoding", "chunked")
                .insert_header("x-goog-meta-sfc-digest", "test-digest"),
        )
        .mount(&server)
        .await;

    let stage = gcs_stage_with_presigned_url(&format!("{}/download", server.uri()));
    let result = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, 0),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(
        result.is_ok(),
        "absent Content-Length must skip the check: {result:?}"
    );
}

/// When `Content-Encoding` is present the Content-Length check must be
/// skipped — even when Content-Length and body size match. This pins the
/// interaction with the step 3 `.no_gzip()` fix: after disabling
/// auto-decode, `Content-Encoding` stays visible in the response headers
/// and our guard suppresses the length comparison (which would be
/// meaningless if a future code path re-enabled decoding).
#[tokio::test]
async fn gcs_download_content_encoding_present_skips_length_check() {
    let server = MockServer::start().await;
    let body = b"ten bytes!";

    Mock::given(method("GET"))
        .and(path("/download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(body.to_vec())
                .insert_header("content-encoding", "gzip")
                .insert_header("x-goog-meta-sfc-digest", "test-digest"),
        )
        .mount(&server)
        .await;

    let stage = gcs_stage_with_presigned_url(&format!("{}/download", server.uri()));
    let result = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, 0),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(
        result.is_ok(),
        "Content-Encoding present must skip the length check: {result:?}"
    );
    let response = result.unwrap();
    assert_eq!(response.data, body);
}

#[tokio::test]
async fn gcs_download_zero_byte_file_succeeds() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(Vec::new())
                .insert_header("content-length", "0")
                .insert_header("x-goog-meta-sfc-digest", "test-digest"),
        )
        .mount(&server)
        .await;

    let stage = gcs_stage_with_presigned_url(&format!("{}/download", server.uri()));
    let response = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, 0),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("zero-byte file must succeed");

    assert!(response.data.is_empty());
    assert_eq!(response.cloud_byte_count, 0);
}

/// Malformed Content-Length is rejected by hyper at the HTTP protocol layer
/// before our code sees the response headers. Our defensive parse-to-u64
/// fallback (`None` → skip check) exists for future-proofing but is
/// currently unreachable. This test documents that behavior.
#[tokio::test]
async fn gcs_download_malformed_content_length_rejected_by_http_layer() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf).await;
        let response = "HTTP/1.1 200 OK\r\n\
             Content-Length: not-a-number\r\n\
             x-goog-meta-sfc-digest: test-digest\r\n\
             Connection: close\r\n\r\nsome data";
        stream.write_all(response.as_bytes()).await.unwrap();
        let _ = stream.shutdown().await;
    });

    let presigned_url = format!("http://{addr}/download");
    let stage = gcs_stage_with_presigned_url(&presigned_url);
    let result = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, 0),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    server.await.unwrap();
    assert!(
        result.is_err(),
        "malformed Content-Length is rejected by hyper before our code runs"
    );
}

/// The GCS download path must not advertise `Accept-Encoding: gzip` on
/// the wire either. `.no_gzip()` on reqwest also suppresses the
/// automatic `Accept-Encoding` header injection — mirroring libcurl's
/// default (no opt-in) and JDBC's `disableContentCompression`. This
/// guards against a future regression where someone calls `.gzip(true)`
/// or removes `.no_gzip()` and only the auto-decoder check is asserted.
#[tokio::test]
async fn gcs_download_does_not_advertise_gzip_accept_encoding() {
    let server = MockServer::start().await;

    let payload: &[u8] = b"plain body";

    Mock::given(method("GET"))
        .and(path("/download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(payload.to_vec())
                .insert_header("x-goog-meta-sfc-digest", "test-digest"),
        )
        .mount(&server)
        .await;

    let stage = gcs_stage_with_presigned_url(&format!("{}/download", server.uri()));
    sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, 0),
        0,
        &mut None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("download must succeed");

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "exactly one GET expected");
    let accept_encoding = requests[0]
        .headers
        .get("accept-encoding")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        !accept_encoding.to_ascii_lowercase().contains("gzip"),
        "GCS GET must not advertise gzip in Accept-Encoding (reqwest .no_gzip() also \
         suppresses the auto-injected header); got: {accept_encoding:?}"
    );
}

// ---------------------------------------------------------------
// Server-supplied per-file pre-signed URL list on multi-file GET
// (gap 2.2 — see `--gcp--/2.2-server_supplied_presigned_url_list_on_download.md`)
// ---------------------------------------------------------------

/// Stage info for presigned-only multi-file GET: no token, no PUT-side
/// `presigned_url`; the URLs come from `DownloadData.presigned_urls`.
fn gcs_stage_presigned_only_no_stage_url() -> StageInfo {
    StageInfo {
        location_type: LocationType::Gcs,
        bucket: "test-bucket".to_string(),
        key_prefix: "prefix/".to_string(),
        region: "us-central1".to_string(),
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        endpoint: None,
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
        use_s3_regional_url: false,
        tls_config: sf_core::tls::config::TlsConfig::default(),
        crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
        storage_account: None,
    }
}

/// SSE response template (no encryption metadata headers): the body is
/// written to disk verbatim, so the test can read it back to verify
/// per-file routing.
fn gcs_sse_response(body: &'static [u8]) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_bytes(body.to_vec())
        .insert_header("x-goog-meta-sfc-digest", "test-digest")
}

#[tokio::test]
async fn gcs_download_files_routes_each_file_to_its_per_file_presigned_url() {
    // Pre-2.2 this fails on the first file with `MissingGcsCredentials`
    // because `DownloadData` carries no per-file URL slot. Post-2.2, GS's
    // `data.presignedUrls[i]` is preserved through the pipeline and each
    // file is fetched from its own URL — matching Python connector
    // (`gcs_storage_client.py:77`), libsfclient (`SnowflakeGCSClient.cpp:144`),
    // and JDBC (`SnowflakeFileTransferAgent.java:1762`).
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/presigned/a"))
        .respond_with(gcs_sse_response(b"alpha-bytes"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/presigned/b"))
        .respond_with(gcs_sse_response(b"beta-bytes"))
        .expect(1)
        .mount(&server)
        .await;

    let tmp = tempfile::tempdir().expect("tempdir");
    let local_location = tmp.path().to_string_lossy().to_string();

    let url_a = format!("{}/presigned/a", server.uri());
    let url_b = format!("{}/presigned/b", server.uri());

    let data = DownloadData {
        src_locations: vec!["a".to_string(), "b".to_string()],
        local_location: local_location.clone(),
        stage_info: gcs_stage_presigned_only_no_stage_url(),
        encryption_materials: vec![None, None],
        presigned_urls: vec![Some(url_a.clone()), Some(url_b.clone())],
        flavor: PutGetResultsetFlavor::Python,
        multipart: MultipartParams::default(),
        unsafe_file_write: false,
    };

    let results = download_files(
        data,
        &RetryPolicy::put_get(&ParamStore::new()),
        None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("multi-file presigned GET should succeed");

    assert_eq!(results.len(), 2);
    let dir = std::path::Path::new(&local_location);
    assert_eq!(
        std::fs::read(dir.join("a")).expect("read file a"),
        b"alpha-bytes"
    );
    assert_eq!(
        std::fs::read(dir.join("b")).expect("read file b"),
        b"beta-bytes"
    );
}

#[tokio::test]
async fn gcs_download_files_fails_with_missing_credentials_when_no_url_and_no_token() {
    // Pin the post-2.2 failure mode: the only path that still surfaces
    // `MissingGcsCredentials` is the genuinely degenerate one (no per-file
    // URL, no `stage_info.presigned_url`, no token). Guards against silent
    // regressions if a future change accidentally promotes a default URL.
    let tmp = tempfile::tempdir().expect("tempdir");
    let local_location = tmp.path().to_string_lossy().to_string();

    let data = DownloadData {
        src_locations: vec!["a".to_string()],
        local_location,
        stage_info: gcs_stage_presigned_only_no_stage_url(),
        encryption_materials: vec![None],
        presigned_urls: vec![None],
        flavor: PutGetResultsetFlavor::Python,
        multipart: MultipartParams::default(),
        unsafe_file_write: false,
    };

    let err = download_files(
        data,
        &RetryPolicy::put_get(&ParamStore::new()),
        None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect_err("download must fail when neither URL nor token is available");
    // Walk the error chain (snafu wraps the leaf `MissingGcsCredentials`
    // through `GcsDownloadError` → `FileManagerError`).
    let chain: Vec<String> =
        std::iter::successors(Some(&err as &dyn std::error::Error), |e| e.source())
            .map(|e| e.to_string())
            .collect();
    assert!(
        chain.iter().any(|m| m == "Missing GCS credentials"),
        "expected MissingGcsCredentials in error chain, got: {chain:?}"
    );
}

// ---------------------------------------------------------------
// Reactive recovery: 400 → URL refresh (gap 2.1) and 401 → token
// refresh (gap 2.4). Exercises StageInfoRefresher through
// download_from_gcs with a wiremock that flips response on a
// queued snapshot rotation.
// ---------------------------------------------------------------

/// A `StageInfoRefresher` for integration tests. The cache is rotated to
/// the queued snapshot on each `refresh()` / `refresh_url()` call (FIFO);
/// when the queue is empty, the cache is left untouched (simulating a
/// coalesced hit). Counts each kind of call so tests can assert which
/// path fired.
struct FakeRefresher {
    cache: StageInfoCache,
    refresh_queue: Mutex<Vec<StageInfoSnapshot>>,
    refresh_url_queue: Mutex<Vec<StageInfoSnapshot>>,
    refresh_calls: AtomicUsize,
    refresh_url_calls: AtomicUsize,
    /// Destination file names passed to `notify_current_upload_file`, in call
    /// order — lets tests assert the per-file PUT plumbing.
    notified_files: Mutex<Vec<String>>,
}

impl FakeRefresher {
    fn new(initial: StageInfoSnapshot) -> Self {
        Self {
            cache: StageInfoCache::new(initial),
            refresh_queue: Mutex::new(Vec::new()),
            refresh_url_queue: Mutex::new(Vec::new()),
            refresh_calls: AtomicUsize::new(0),
            refresh_url_calls: AtomicUsize::new(0),
            notified_files: Mutex::new(Vec::new()),
        }
    }

    fn arm_refresh(&self, snap: StageInfoSnapshot) {
        self.refresh_queue.lock().unwrap().push(snap);
    }

    fn arm_refresh_url(&self, snap: StageInfoSnapshot) {
        self.refresh_url_queue.lock().unwrap().push(snap);
    }
}

impl StageInfoRefresher for FakeRefresher {
    fn refresh(&mut self) -> RefreshFuture<'_> {
        self.refresh_calls.fetch_add(1, Ordering::SeqCst);
        let next = {
            let mut q = self.refresh_queue.lock().unwrap();
            if q.is_empty() {
                None
            } else {
                Some(q.remove(0))
            }
        };
        if let Some(snap) = next {
            self.cache.store(snap);
        }
        Box::pin(async { Ok(()) })
    }

    fn refresh_url(&mut self) -> RefreshFuture<'_> {
        self.refresh_url_calls.fetch_add(1, Ordering::SeqCst);
        let next = {
            let mut q = self.refresh_url_queue.lock().unwrap();
            if q.is_empty() {
                None
            } else {
                Some(q.remove(0))
            }
        };
        if let Some(snap) = next {
            self.cache.store(snap);
        }
        Box::pin(async { Ok(()) })
    }

    fn cache(&self) -> &StageInfoCache {
        &self.cache
    }

    fn notify_current_upload_file(&mut self, dst_file_name: String) {
        self.notified_files.lock().unwrap().push(dst_file_name);
    }
}

/// Gap 2.1: first 400 in presigned mode triggers `refresh_url()` (no
/// coalesce); the rotated `presigned_url` from the cache is used on the
/// retry. Matches Python's per-file refresh on 400 (`gcs_storage_client.py`).
#[tokio::test]
async fn gcs_download_400_triggers_url_refresh_and_succeeds() {
    let server = MockServer::start().await;
    let stale_hits = Arc::new(AtomicU32::new(0));
    let fresh_hits = Arc::new(AtomicU32::new(0));

    let stale_clone = stale_hits.clone();
    Mock::given(method("GET"))
        .and(path("/stale-url"))
        .respond_with(move |_: &Request| {
            stale_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(400).set_body_string("ExpiredToken")
        })
        .mount(&server)
        .await;
    let fresh_clone = fresh_hits.clone();
    Mock::given(method("GET"))
        .and(path("/fresh-url"))
        .respond_with(move |_: &Request| {
            fresh_clone.fetch_add(1, Ordering::SeqCst);
            gcs_response_headers()
        })
        .mount(&server)
        .await;

    let stale_url = format!("{}/stale-url", server.uri());
    let fresh_url = format!("{}/fresh-url", server.uri());
    let stage = gcs_stage_with_presigned_url(&stale_url);

    let mut fake = FakeRefresher::new(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: Some(stale_url.clone()),
        presigned_urls: None,
    });
    fake.arm_refresh_url(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: Some(fresh_url.clone()),
        presigned_urls: None,
    });

    let mut refresher_opt: Option<&mut dyn StageInfoRefresher> = Some(&mut fake);
    let result = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, 0),
        0,
        &mut refresher_opt,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(result.is_ok(), "400 → refresh_url → retry should succeed");
    assert_eq!(
        stale_hits.load(Ordering::SeqCst),
        1,
        "stale URL hit once before refresh"
    );
    assert_eq!(
        fresh_hits.load(Ordering::SeqCst),
        1,
        "fresh URL hit once on retry"
    );
    assert_eq!(
        fake.refresh_url_calls.load(Ordering::SeqCst),
        1,
        "refresh_url() called exactly once"
    );
    assert_eq!(
        fake.refresh_calls.load(Ordering::SeqCst),
        0,
        "coalesced refresh() must NOT fire on 400"
    );
}

/// Gap 2.1 two-strike guard: if the refreshed URL also returns 400, we
/// stop after the second strike and surface `PresignedUrlExpired` rather
/// than looping. Matches Python's `gcs_storage_client.py` guard.
#[tokio::test]
async fn gcs_download_400_after_url_refresh_returns_presigned_url_expired() {
    let server = MockServer::start().await;
    let stale_hits = Arc::new(AtomicU32::new(0));
    let fresh_hits = Arc::new(AtomicU32::new(0));

    let stale_clone = stale_hits.clone();
    Mock::given(method("GET"))
        .and(path("/stale-url"))
        .respond_with(move |_: &Request| {
            stale_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(400).set_body_string("ExpiredToken")
        })
        .mount(&server)
        .await;
    let fresh_clone = fresh_hits.clone();
    Mock::given(method("GET"))
        .and(path("/also-stale-url"))
        .respond_with(move |_: &Request| {
            fresh_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(400).set_body_string("ExpiredToken")
        })
        .mount(&server)
        .await;

    let stale_url = format!("{}/stale-url", server.uri());
    let also_stale_url = format!("{}/also-stale-url", server.uri());
    let stage = gcs_stage_with_presigned_url(&stale_url);

    let mut fake = FakeRefresher::new(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: Some(stale_url.clone()),
        presigned_urls: None,
    });
    fake.arm_refresh_url(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: Some(also_stale_url.clone()),
        presigned_urls: None,
    });

    let mut refresher_opt: Option<&mut dyn StageInfoRefresher> = Some(&mut fake);
    let err = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(true, 0),
        0,
        &mut refresher_opt,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect_err("two consecutive 400s must fail fast");

    assert!(
        matches!(err, GcsDownloadError::PresignedUrlExpired { .. }),
        "second 400 must surface PresignedUrlExpired, got: {err:?}"
    );
    assert_eq!(
        fake.refresh_url_calls.load(Ordering::SeqCst),
        1,
        "refresh_url() fires exactly once (two-strike guard)"
    );
}

/// Gap 2.4: first 401 triggers coalesced `refresh()`; the rotated bearer
/// token from the cache is used on the retry. Matches Python's
/// `_handle_refresh_with_downscoped_*` paths in `gcs_storage_client.py` and
/// JDBC's `error401RenewExpired`.
#[tokio::test]
async fn gcs_download_401_triggers_token_refresh_and_succeeds() {
    let server = MockServer::start().await;
    let attempt = Arc::new(AtomicU32::new(0));
    let attempt_clone = attempt.clone();

    Mock::given(method("GET"))
        .and(path("/test-bucket/prefix/file.csv"))
        .respond_with(move |req: &Request| {
            let n = attempt_clone.fetch_add(1, Ordering::SeqCst);
            let auth = req
                .headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string();
            if n == 0 {
                // First request still carries the stale "Bearer stale-token".
                assert!(
                    auth.ends_with("stale-token"),
                    "first attempt should use stale token; got auth={auth}"
                );
                ResponseTemplate::new(401).set_body_string("Unauthenticated")
            } else {
                // Second request must carry the rotated bearer.
                assert!(
                    auth.ends_with("fresh-token"),
                    "retry should use rotated token; got auth={auth}"
                );
                gcs_response_headers()
            }
        })
        .mount(&server)
        .await;

    // Stage uses bearer-token mode (no presigned URL) pointing at the mock.
    let mut stage = gcs_stage_with_token(&server.uri());
    stage.creds = CloudCredentials::Gcs {
        gcs_access_token: Some(SensitiveString::from("stale-token")),
    };

    let mut fake = FakeRefresher::new(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: Some(SensitiveString::from("stale-token")),
        },
        presigned_url: None,
        presigned_urls: None,
    });
    fake.arm_refresh(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: Some(SensitiveString::from("fresh-token")),
        },
        presigned_url: None,
        presigned_urls: None,
    });

    let mut refresher_opt: Option<&mut dyn StageInfoRefresher> = Some(&mut fake);
    let result = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(false, 0),
        0,
        &mut refresher_opt,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(result.is_ok(), "401 → refresh → retry should succeed");
    assert_eq!(
        attempt.load(Ordering::SeqCst),
        2,
        "should hit GCS twice (stale, then fresh)"
    );
    assert_eq!(
        fake.refresh_calls.load(Ordering::SeqCst),
        1,
        "refresh() fires exactly once on 401"
    );
    assert_eq!(
        fake.refresh_url_calls.load(Ordering::SeqCst),
        0,
        "non-coalesced refresh_url() must NOT fire on 401"
    );
}

/// Gap 2.4 two-strike-equivalent: if the refresher returns the same token
/// (within its coalescing window), the GCS token refresher reports "no
/// rotation" and the original 401 is surfaced as `TokenExpired` rather
/// than spinning. Mirrors S3's STS-refresh "unchanged creds" behavior.
#[tokio::test]
async fn gcs_download_401_with_unchanged_token_returns_token_expired() {
    let server = MockServer::start().await;
    let attempt = Arc::new(AtomicU32::new(0));
    let attempt_clone = attempt.clone();

    Mock::given(method("GET"))
        .and(path("/test-bucket/prefix/file.csv"))
        .respond_with(move |_: &Request| {
            attempt_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(401).set_body_string("Unauthenticated")
        })
        .mount(&server)
        .await;

    let mut stage = gcs_stage_with_token(&server.uri());
    stage.creds = CloudCredentials::Gcs {
        gcs_access_token: Some(SensitiveString::from("stale-token")),
    };

    // No arming → refresh() leaves the cache holding the same "stale-token";
    // the GcsTokenRefresher must detect "no rotation" and not retry.
    let mut fake = FakeRefresher::new(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: Some(SensitiveString::from("stale-token")),
        },
        presigned_url: None,
        presigned_urls: None,
    });

    let mut refresher_opt: Option<&mut dyn StageInfoRefresher> = Some(&mut fake);
    let err = sf_core::file_manager::download_from_gcs(
        &stage,
        "file.csv",
        None,
        &test_policy(false, 0),
        0,
        &mut refresher_opt,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect_err("unchanged token must surface TokenExpired");

    assert!(
        matches!(err, GcsDownloadError::TokenExpired { .. }),
        "should surface TokenExpired when refresher returns unchanged creds, got: {err:?}"
    );
    assert_eq!(
        attempt.load(Ordering::SeqCst),
        1,
        "no retry when refresh returns unchanged token"
    );
    assert_eq!(fake.refresh_calls.load(Ordering::SeqCst), 1);
}

// ---------------------------------------------------------------
// Gap 2.4 — upload side
// (Python: _has_expired_token + StorageCredential.update;
//  JDBC: GCSDefaultAccessStrategy 401 → renewExpiredToken;
//  libsfclient: renewToken at FileTransferAgent.cpp:400)
// ---------------------------------------------------------------

/// Gap 2.4 upload: first 401 triggers coalesced `refresh()`; the rotated
/// bearer token from the cache is used on the retry. Mirrors
/// `gcs_download_401_triggers_token_refresh_and_succeeds` for the PUT path.
#[tokio::test]
async fn gcs_upload_401_then_refresh_then_200() {
    let server = MockServer::start().await;
    let attempt = Arc::new(AtomicU32::new(0));
    let attempt_clone = attempt.clone();

    // The upload URL in token mode: {endpoint}/{bucket}/{key_prefix}{filename}
    // = {server.uri()}/test-bucket/prefix/file.csv
    Mock::given(method("PUT"))
        .and(path("/test-bucket/prefix/file.csv"))
        .respond_with(move |req: &Request| {
            let n = attempt_clone.fetch_add(1, Ordering::SeqCst);
            let auth = req
                .headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string();
            if n == 0 {
                assert!(
                    auth.ends_with("stale-token"),
                    "first PUT should use stale bearer; got auth={auth}"
                );
                ResponseTemplate::new(401).set_body_string("Unauthenticated")
            } else {
                assert!(
                    auth.ends_with("fresh-token"),
                    "retry PUT should use rotated bearer; got auth={auth}"
                );
                ResponseTemplate::new(200)
            }
        })
        .mount(&server)
        .await;

    let mut stage = gcs_stage_with_token(&server.uri());
    stage.creds = CloudCredentials::Gcs {
        gcs_access_token: Some(SensitiveString::from("stale-token")),
    };

    let mut fake = FakeRefresher::new(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: Some(SensitiveString::from("stale-token")),
        },
        presigned_url: None,
        presigned_urls: None,
    });
    fake.arm_refresh(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: Some(SensitiveString::from("fresh-token")),
        },
        presigned_url: None,
        presigned_urls: None,
    });

    let prepared = PreparedUpload::new_unencrypted_for_test(
        ByteSource::Bytes(Bytes::from_static(b"test-bytes")),
        "test-digest".to_string(),
    );
    let mut refresher_opt: Option<&mut dyn StageInfoRefresher> = Some(&mut fake);
    let result = sf_core::file_manager::upload_to_gcs_or_skip(
        prepared,
        &stage,
        "file.csv",
        true,
        &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
        &mut refresher_opt,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(
        result.is_ok(),
        "401 → refresh → retry PUT should succeed; got {result:?}"
    );
    assert_eq!(
        attempt.load(Ordering::SeqCst),
        2,
        "should hit GCS twice (stale, then fresh)"
    );
    assert_eq!(
        fake.refresh_calls.load(Ordering::SeqCst),
        1,
        "refresh() fires exactly once on 401"
    );
    assert_eq!(
        fake.refresh_url_calls.load(Ordering::SeqCst),
        0,
        "non-coalesced refresh_url() must NOT fire on 401"
    );
}

// ---------------------------------------------------------------
// Gap 2.1 — upload side (400 URL refresh for PUT)
// ---------------------------------------------------------------

/// Gap 2.1 upload: first 400 in presigned mode triggers `refresh_url()`;
/// the rotated `presigned_url` from the cache is used on the retry.
/// Mirrors `gcs_download_400_triggers_url_refresh_and_succeeds` for PUT.
#[tokio::test]
async fn gcs_upload_400_triggers_url_refresh_and_succeeds() {
    let server = MockServer::start().await;
    let stale_hits = Arc::new(AtomicU32::new(0));
    let fresh_hits = Arc::new(AtomicU32::new(0));

    let stale_clone = stale_hits.clone();
    Mock::given(method("PUT"))
        .and(path("/stale-upload"))
        .respond_with(move |_: &Request| {
            stale_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(400).set_body_string("ExpiredToken")
        })
        .mount(&server)
        .await;

    let fresh_clone = fresh_hits.clone();
    Mock::given(method("PUT"))
        .and(path("/fresh-upload"))
        .respond_with(move |_: &Request| {
            fresh_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200)
        })
        .mount(&server)
        .await;

    let stale_url = format!("{}/stale-upload", server.uri());
    let fresh_url = format!("{}/fresh-upload", server.uri());
    let stage = gcs_stage_with_presigned_url(&stale_url);

    let mut fake = FakeRefresher::new(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: Some(stale_url.clone()),
        presigned_urls: None,
    });
    fake.arm_refresh_url(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: Some(fresh_url.clone()),
        presigned_urls: None,
    });

    let prepared = PreparedUpload::new_unencrypted_for_test(
        ByteSource::Bytes(Bytes::from_static(b"test-bytes")),
        "test-digest".to_string(),
    );
    let mut refresher_opt: Option<&mut dyn StageInfoRefresher> = Some(&mut fake);
    let result = sf_core::file_manager::upload_to_gcs_or_skip(
        prepared,
        &stage,
        "file.csv",
        true,
        &test_policy(true, DEFAULT_PUT_GET_MAX_ATTEMPTS),
        &mut refresher_opt,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(
        result.is_ok(),
        "400 → refresh_url → retry PUT should succeed; got {result:?}"
    );
    assert_eq!(
        stale_hits.load(Ordering::SeqCst),
        1,
        "stale URL hit once before refresh"
    );
    assert_eq!(
        fresh_hits.load(Ordering::SeqCst),
        1,
        "fresh URL hit once on retry"
    );
    assert_eq!(
        fake.refresh_url_calls.load(Ordering::SeqCst),
        1,
        "refresh_url() called exactly once"
    );
    assert_eq!(
        fake.refresh_calls.load(Ordering::SeqCst),
        0,
        "coalesced refresh() must NOT fire on 400"
    );
}

/// The upload path must tell the refresher which destination file is being
/// uploaded (via `notify_current_upload_file`) so a per-file URL refresh can
/// rewrite the PUT SQL for that file. Asserts the dst name (incl. any
/// compression suffix) reaches the refresher before the 400-triggered refresh.
#[tokio::test]
async fn gcs_upload_notifies_dst_file_name_before_url_refresh() {
    let server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/stale-upload"))
        .respond_with(ResponseTemplate::new(400).set_body_string("ExpiredToken"))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/fresh-upload"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let stale_url = format!("{}/stale-upload", server.uri());
    let fresh_url = format!("{}/fresh-upload", server.uri());
    let stage = gcs_stage_with_presigned_url(&stale_url);

    let mut fake = FakeRefresher::new(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: Some(stale_url.clone()),
        presigned_urls: None,
    });
    fake.arm_refresh_url(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: Some(fresh_url.clone()),
        presigned_urls: None,
    });

    let prepared = PreparedUpload::new_unencrypted_for_test(
        ByteSource::Bytes(Bytes::from_static(b"test-bytes")),
        "test-digest".to_string(),
    );
    let mut refresher_opt: Option<&mut dyn StageInfoRefresher> = Some(&mut fake);
    let result = sf_core::file_manager::upload_to_gcs_or_skip(
        prepared,
        &stage,
        "part-01.csv.gz",
        true,
        &test_policy(true, DEFAULT_PUT_GET_MAX_ATTEMPTS),
        &mut refresher_opt,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(
        result.is_ok(),
        "upload should succeed after refresh; got {result:?}"
    );
    assert_eq!(
        *fake.notified_files.lock().unwrap(),
        vec!["part-01.csv.gz".to_string()],
        "refresher must be told the destination object name for per-file URL rewrite"
    );
}

/// Gap 2.1 upload two-strike guard: if the refreshed presigned URL also returns
/// 400, we stop after the second strike and surface `PresignedUrlExpired` rather
/// than looping. Symmetric to `gcs_download_400_after_url_refresh_returns_presigned_url_expired`.
#[tokio::test]
async fn gcs_upload_400_after_url_refresh_returns_presigned_url_expired() {
    let server = MockServer::start().await;
    let stale_hits = Arc::new(AtomicU32::new(0));
    let also_stale_hits = Arc::new(AtomicU32::new(0));

    let stale_clone = stale_hits.clone();
    Mock::given(method("PUT"))
        .and(path("/stale-upload"))
        .respond_with(move |_: &Request| {
            stale_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(400).set_body_string("ExpiredToken")
        })
        .mount(&server)
        .await;
    let also_stale_clone = also_stale_hits.clone();
    Mock::given(method("PUT"))
        .and(path("/also-stale-upload"))
        .respond_with(move |_: &Request| {
            also_stale_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(400).set_body_string("ExpiredToken")
        })
        .mount(&server)
        .await;

    let stale_url = format!("{}/stale-upload", server.uri());
    let also_stale_url = format!("{}/also-stale-upload", server.uri());
    let stage = gcs_stage_with_presigned_url(&stale_url);

    let mut fake = FakeRefresher::new(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: Some(stale_url.clone()),
        presigned_urls: None,
    });
    fake.arm_refresh_url(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: Some(also_stale_url.clone()),
        presigned_urls: None,
    });

    let prepared = PreparedUpload::new_unencrypted_for_test(
        ByteSource::Bytes(Bytes::from_static(b"test-bytes")),
        "test-digest".to_string(),
    );
    let mut refresher_opt: Option<&mut dyn StageInfoRefresher> = Some(&mut fake);
    let err = sf_core::file_manager::upload_to_gcs_or_skip(
        prepared,
        &stage,
        "file.csv",
        true,
        &test_policy(true, DEFAULT_PUT_GET_MAX_ATTEMPTS),
        &mut refresher_opt,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect_err("two consecutive 400s on PUT must fail fast");

    assert!(
        matches!(err, GcsUploadError::PresignedUrlExpired { .. }),
        "second 400 on PUT must surface PresignedUrlExpired, got: {err:?}"
    );
    assert_eq!(
        fake.refresh_url_calls.load(Ordering::SeqCst),
        1,
        "refresh_url() fires exactly once (two-strike guard)"
    );
    assert_eq!(
        fake.refresh_calls.load(Ordering::SeqCst),
        0,
        "coalesced refresh() must NOT fire on 400"
    );
    assert_eq!(stale_hits.load(Ordering::SeqCst), 1, "stale URL hit once");
    assert_eq!(
        also_stale_hits.load(Ordering::SeqCst),
        1,
        "also-stale URL hit once on second strike"
    );
}

// ---------------------------------------------------------------
// Gap 2.1 — per-file URL refresh is not debounced
// ---------------------------------------------------------------

/// Spec 2.1 §5: per-file URL refresh must NOT use the coalescing window.
/// Two consecutive download calls (simulating two files in a batch), each
/// getting a 400, must each trigger `refresh_url()` independently — the
/// second call must not be swallowed by a debounce gate.
#[tokio::test]
async fn gcs_per_file_url_refresh_is_not_debounced() {
    let server = MockServer::start().await;

    // File 1: stale URL → 400, fresh URL → 200
    Mock::given(method("GET"))
        .and(path("/stale-1"))
        .respond_with(ResponseTemplate::new(400).set_body_string("ExpiredToken"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/fresh-1"))
        .respond_with(gcs_sse_response(b"file-1-bytes"))
        .mount(&server)
        .await;

    // File 2: stale URL → 400, fresh URL → 200
    Mock::given(method("GET"))
        .and(path("/stale-2"))
        .respond_with(ResponseTemplate::new(400).set_body_string("ExpiredToken"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/fresh-2"))
        .respond_with(gcs_sse_response(b"file-2-bytes"))
        .mount(&server)
        .await;

    let stale_1 = format!("{}/stale-1", server.uri());
    let fresh_1 = format!("{}/fresh-1", server.uri());
    let stale_2 = format!("{}/stale-2", server.uri());
    let fresh_2 = format!("{}/fresh-2", server.uri());

    // Stage has no presigned_url: the per-file URL drives both calls.
    let stage = gcs_stage_presigned_only_no_stage_url();

    // Arm two refresh_url slots — one for each file's expiry event.
    let mut fake = FakeRefresher::new(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: None,
        presigned_urls: None,
    });
    fake.arm_refresh_url(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: None,
        presigned_urls: Some(vec![Some(fresh_1.clone())]),
    });
    fake.arm_refresh_url(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: None,
        presigned_urls: Some(vec![Some(fresh_2.clone())]),
    });

    // Call download_from_gcs twice in sequence, simulating two files in a
    // batch. Each has its own stale per-file URL and its own per_file_index.
    let mut refresher_opt: Option<&mut dyn StageInfoRefresher> = Some(&mut fake);
    let r1 = sf_core::file_manager::download_from_gcs(
        &stage,
        "file1.csv",
        Some(&stale_1),
        &test_policy(true, 0),
        0,
        &mut refresher_opt,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;
    assert!(
        r1.is_ok(),
        "file 1 should succeed after URL refresh; got {r1:?}"
    );

    let r2 = sf_core::file_manager::download_from_gcs(
        &stage,
        "file2.csv",
        Some(&stale_2),
        &test_policy(true, 0),
        0,
        &mut refresher_opt,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;
    assert!(
        r2.is_ok(),
        "file 2 should succeed after URL refresh; got {r2:?}"
    );

    assert_eq!(
        fake.refresh_url_calls.load(Ordering::SeqCst),
        2,
        "refresh_url() must fire once per file with no debounce between files"
    );
    assert_eq!(
        fake.refresh_calls.load(Ordering::SeqCst),
        0,
        "coalesced refresh() must NOT fire during per-file URL refresh"
    );
}

/// End-to-end `download_files` batch test: each file has a stale presigned URL
/// that returns 400 on first attempt; after URL refresh the rotated
/// `presigned_urls[i]` is used and the download succeeds. This exercises the
/// `download_files` → `download_single_file` → `download_from_gcs` path (the
/// main motivation for per-file non-coalesced `refresh_url`).
#[tokio::test]
async fn gcs_download_files_batch_rotates_presigned_urls_across_files() {
    let server = MockServer::start().await;

    // File 1: stale URL → 400, fresh URL → 200 with body "bytes-for-a"
    Mock::given(method("GET"))
        .and(path("/stale-a"))
        .respond_with(ResponseTemplate::new(400).set_body_string("ExpiredToken"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/fresh-a"))
        .respond_with(gcs_sse_response(b"bytes-for-a"))
        .mount(&server)
        .await;

    // File 2: stale URL → 400, fresh URL → 200 with body "bytes-for-b"
    Mock::given(method("GET"))
        .and(path("/stale-b"))
        .respond_with(ResponseTemplate::new(400).set_body_string("ExpiredToken"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/fresh-b"))
        .respond_with(gcs_sse_response(b"bytes-for-b"))
        .mount(&server)
        .await;

    let stale_a = format!("{}/stale-a", server.uri());
    let fresh_a = format!("{}/fresh-a", server.uri());
    let stale_b = format!("{}/stale-b", server.uri());
    let fresh_b = format!("{}/fresh-b", server.uri());

    let tmp = tempfile::tempdir().expect("tempdir");
    let local_location = tmp.path().to_string_lossy().to_string();

    // Stage carries no presigned_url and no token; per-file URLs drive the fetch.
    let stage = gcs_stage_presigned_only_no_stage_url();

    // Arm two URL-rotation slots — one per file expiry event, each carrying
    // the refreshed `presigned_urls[0]` for that file's index.
    let mut fake = FakeRefresher::new(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: None,
        presigned_urls: None,
    });
    fake.arm_refresh_url(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: None,
        presigned_urls: Some(vec![Some(fresh_a.clone()), Some(stale_b.clone())]),
    });
    fake.arm_refresh_url(StageInfoSnapshot {
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        presigned_url: None,
        presigned_urls: Some(vec![Some(fresh_a.clone()), Some(fresh_b.clone())]),
    });

    let data = DownloadData {
        src_locations: vec!["a".to_string(), "b".to_string()],
        local_location: local_location.clone(),
        stage_info: stage,
        encryption_materials: vec![None, None],
        presigned_urls: vec![Some(stale_a.clone()), Some(stale_b.clone())],
        flavor: PutGetResultsetFlavor::Python,
        multipart: MultipartParams::default(),
        unsafe_file_write: false,
    };

    let refresher_opt: Option<&mut dyn StageInfoRefresher> = Some(&mut fake);
    let results = download_files(
        data,
        &RetryPolicy::put_get(&ParamStore::new()),
        refresher_opt,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("batch download should succeed after per-file URL refresh");

    assert_eq!(results.len(), 2, "both files must be reported");
    let dir = std::path::Path::new(&local_location);
    assert_eq!(
        std::fs::read(dir.join("a")).expect("read file a"),
        b"bytes-for-a"
    );
    assert_eq!(
        std::fs::read(dir.join("b")).expect("read file b"),
        b"bytes-for-b"
    );
    assert_eq!(
        fake.refresh_url_calls.load(Ordering::SeqCst),
        2,
        "refresh_url() must fire once per expired file (two files, two calls)"
    );
    assert_eq!(
        fake.refresh_calls.load(Ordering::SeqCst),
        0,
        "coalesced refresh() must NOT fire during per-file URL refresh"
    );
}

// A client-side-encrypted GCS upload streams a lazily-encrypting `reqwest`
// body of unknown length, so the driver must set `Content-Length` to the
// analytic ciphertext length explicitly. This pins that contract end-to-end:
// the mock requires the exact `Content-Length` and rejects a chunked body
// (no `Transfer-Encoding`), so a regression to chunked or a wrong length
// fails the request — which a download-only test could never catch.
#[tokio::test]
async fn gcs_cse_upload_sets_exact_content_length_and_is_not_chunked() {
    let plaintext = b"client-side-encrypted GCS upload body".to_vec();

    // Build the encryptor the way preprocessing does: digest over the source,
    // analytic ciphertext length on the encryptor.
    let material = sf_core::file_manager::types::EncryptionMaterial {
        query_stage_master_key: SensitiveString::from({
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode([0u8; 32])
        }),
        query_id: "q".to_string(),
        smk_id: "1".to_string(),
    };
    let digest = sf_core::file_manager::internal::compute_sha256_digest(&ByteSource::Bytes(
        plaintext.clone().into(),
    ))
    .unwrap();
    let (encryptor, enc_meta) =
        sf_core::file_manager::internal::build_encryptor(&material, plaintext.len() as i64)
            .unwrap();
    let expected_len = encryptor.cipher_len();

    let server = MockServer::start().await;
    let seen_len = Arc::new(AtomicU32::new(0));
    let seen_chunked = Arc::new(AtomicU32::new(0));
    let seen_len_c = seen_len.clone();
    let seen_chunked_c = seen_chunked.clone();
    Mock::given(method("PUT"))
        .and(path("/test-bucket/prefix/file.bin"))
        .respond_with(move |req: &Request| {
            if let Some(cl) = req
                .headers
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u32>().ok())
            {
                seen_len_c.store(cl, Ordering::SeqCst);
            }
            if req.headers.contains_key("transfer-encoding") {
                seen_chunked_c.fetch_add(1, Ordering::SeqCst);
            }
            ResponseTemplate::new(200)
        })
        .mount(&server)
        .await;

    let prepared = PreparedUpload::new_encrypted_for_test(
        ByteSource::Bytes(plaintext.into()),
        digest,
        enc_meta,
        encryptor,
    );
    let mut refresher_opt: Option<&mut dyn StageInfoRefresher> = None;
    let result = sf_core::file_manager::upload_to_gcs_or_skip(
        prepared,
        &gcs_stage_with_token(&server.uri()),
        "file.bin",
        true,
        &test_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
        &mut refresher_opt,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(
        result.is_ok(),
        "CSE GCS upload should succeed; got {result:?}"
    );
    assert_eq!(
        seen_len.load(Ordering::SeqCst) as i64,
        expected_len,
        "Content-Length must equal the analytic ciphertext length",
    );
    assert_eq!(
        seen_chunked.load(Ordering::SeqCst),
        0,
        "encrypted body must be sent with Content-Length, not Transfer-Encoding: chunked",
    );
}

// ---------------------------------------------------------------
// Git stage objects: encryptiondata present but sfc-digest absent
// ---------------------------------------------------------------

#[tokio::test]
async fn gcs_git_stage_download_succeeds_without_sfc_digest() {
    // Git stage objects on GCS carry CSE key-wrap headers (encryptiondata,
    // matdesc) but no sfc-digest — the object was uploaded by Snowflake's git
    // integration. download_files must succeed and write the raw bytes.
    let server = MockServer::start().await;

    let enc_data = serde_json::json!({
        "EncryptionMode": "FullBlob",
        "WrappedContentKey": {
            "KeyId": "symmKey1",
            "EncryptedKey": "dGVzdC1rZXk=",
            "Algorithm": "AES_CBC_256"
        },
        "ContentEncryptionIV": "dGVzdC1pdg=="
    });
    let mat_desc = serde_json::json!({
        "queryId": "test-query",
        "smkId": "1",
        "keySize": "256"
    });
    // No x-goog-meta-sfc-digest header — matches what Snowflake's git integration uploads.
    let presigned_url = format!("{}/presigned/git-file.txt", server.uri());
    Mock::given(method("GET"))
        .and(wiremock::matchers::path("/presigned/git-file.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"raw-git-file-bytes".to_vec())
                .insert_header("x-goog-meta-encryptiondata", enc_data.to_string().as_str())
                .insert_header("x-goog-meta-matdesc", mat_desc.to_string().as_str()),
        )
        .mount(&server)
        .await;

    let tmp = tempfile::tempdir().expect("tempdir");
    let local_location = tmp.path().to_string_lossy().to_string();

    let data = DownloadData {
        src_locations: vec!["git-file.txt".to_string()],
        local_location: local_location.clone(),
        stage_info: gcs_stage_with_presigned_url(&presigned_url),
        encryption_materials: vec![Some(sf_core::file_manager::EncryptionMaterial {
            query_stage_master_key: SensitiveString::from("dGVzdC1tYXN0ZXIta2V5"),
            query_id: "test-query".to_string(),
            smk_id: "1".to_string(),
        })],
        presigned_urls: vec![Some(presigned_url)],
        flavor: PutGetResultsetFlavor::Python,
        multipart: MultipartParams::default(),
        unsafe_file_write: false,
    };

    let results = download_files(
        data,
        &RetryPolicy::put_get(&ParamStore::new()),
        None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("git stage download should succeed even without sfc-digest");

    assert_eq!(results.len(), 1);
    let written = std::fs::read(std::path::Path::new(&local_location).join("git-file.txt"))
        .expect("downloaded file should exist");
    assert_eq!(written, b"raw-git-file-bytes");
}
