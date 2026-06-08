use flate2::Compression;
use flate2::write::GzEncoder;
use sf_core::file_manager::{CloudCredentials, LocationType, StageInfo};
use sf_core::sensitive::SensitiveString;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
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
    let result = sf_core::file_manager::download_from_gcs(&stage, "file.csv").await;

    let err = result.unwrap_err();
    let err_str = format!("{err}");
    assert!(
        err_str.contains("token expired"),
        "401 should produce TokenExpired error, got: {err_str}"
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
    let result = sf_core::file_manager::download_from_gcs(&stage, "file.csv").await;

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
    let result = sf_core::file_manager::download_from_gcs(&stage, "file.csv").await;

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
    let result = sf_core::file_manager::download_from_gcs(&stage, "file.csv").await;

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
    let result = sf_core::file_manager::download_from_gcs(&stage, "file.csv").await;

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
    let result = sf_core::file_manager::download_from_gcs(&stage, "file.csv").await;

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
    let result = sf_core::file_manager::download_from_gcs(&stage, "file.csv").await;

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
    let response = sf_core::file_manager::download_from_gcs(&stage, "file.csv")
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
    let response = sf_core::file_manager::download_from_gcs(&stage, "file.csv")
        .await
        .expect("happy-path download must still succeed");

    assert_eq!(response.data, payload);
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
    sf_core::file_manager::download_from_gcs(&stage, "file.csv")
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
