use sf_core::file_manager::{CloudCredentials, LocationType, StageInfo};
use sf_core::sensitive::SensitiveString;
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
        end_point: None,
        presigned_url: Some(presigned_url.to_string()),
        use_virtual_url: false,
        use_regional_url: false,
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
        end_point: Some(endpoint.to_string()),
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
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
