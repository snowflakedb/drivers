use sf_core::apis::database_driver_v1::PutGetResultsetFlavor;
use sf_core::config::param_registry::DEFAULT_PUT_GET_MAX_ATTEMPTS;
use sf_core::config::param_store::ParamStore;
use sf_core::config::retry::RetryPolicy;
// Shared zero-backoff Azure test policy (the base put/get policy; the per-attempt
// policy is derived from it via `azure_403_fastfail_policy`). Aliased so call
// sites read `test_policy(..)`.
use sf_core::file_manager::internal::azure_test_retry_policy as test_policy;
use sf_core::file_manager::{
    AzureDownloadError, CloudCredentials, DownloadData, EncryptionMaterial, LocationType,
    MultipartParams, StageInfo, download_files,
};
use sf_core::sensitive::SensitiveString;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Helper to build a StageInfo pointing at the mock server.
///
/// Uses a scheme-based endpoint (`http://127.0.0.1:PORT`) so that
/// `build_azure_url` uses the mock server URL directly, matching
/// how GCS tests work with custom endpoints.
fn azure_stage(mock_uri: &str) -> StageInfo {
    StageInfo {
        location_type: LocationType::Azure,
        bucket: "test-container".to_string(),
        key_prefix: "prefix/".to_string(),
        region: "eastus2".to_string(),
        creds: CloudCredentials::Azure {
            sas_token: SensitiveString::from("sv=2021-08-06&sig=test-secret-sig&se=2099-01-01"),
        },
        endpoint: Some(mock_uri.to_string()),
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
        use_s3_regional_url: false,
        tls_config: sf_core::tls::config::TlsConfig::default(),
        crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
        storage_account: Some("test".to_string()),
    }
}

fn azure_response_headers() -> ResponseTemplate {
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
        .insert_header("x-ms-meta-sfcdigest", "test-digest")
        .insert_header("x-ms-meta-encryptiondata", enc_data.to_string().as_str())
        .insert_header("x-ms-meta-matdesc", mat_desc.to_string().as_str())
}

// ---------------------------------------------------------------
// Successful download returns encrypted data and metadata
// ---------------------------------------------------------------

#[tokio::test]
async fn azure_download_success_returns_data_and_metadata() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(azure_response_headers())
        .mount(&server)
        .await;

    let stage = azure_stage(&server.uri());
    let result = sf_core::file_manager::download_from_azure(
        &stage,
        "file.csv",
        &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        &mut None,
    )
    .await;

    let response = result.expect("download should succeed");
    assert_eq!(response.data, b"encrypted-data");
    assert_eq!(response.digest, Some("test-digest".to_string()));
    assert_eq!(
        response.cloud_byte_count,
        b"encrypted-data".len() as i64,
        "cloud_byte_count should equal the body length"
    );
    let metadata = response
        .file_metadata
        .expect("encryption metadata should be present");
    assert_eq!(metadata.encrypted_key, "dGVzdC1rZXk=");
    assert_eq!(metadata.iv, "dGVzdC1pdg==");
    assert_eq!(metadata.material_desc.query_id, "test-query");
    assert_eq!(metadata.material_desc.smk_id, "1");
}

// ---------------------------------------------------------------
// 403 fast-fails on GET (same as PUT) — not inline-retried
// A 403 surfaces immediately as AzureHttp{403} so the refresh layer
// or caller can act on it rather than burning the retry budget.
// ---------------------------------------------------------------

#[tokio::test]
async fn azure_download_403_is_not_inline_retried() {
    let server = MockServer::start().await;
    let attempt = Arc::new(AtomicU32::new(0));

    let attempt_clone = attempt.clone();
    Mock::given(method("GET"))
        .respond_with(move |_: &Request| {
            attempt_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(403).set_body_string("Forbidden")
        })
        .mount(&server)
        .await;

    let stage = azure_stage(&server.uri());
    let result = sf_core::file_manager::download_from_azure(
        &stage,
        "file.csv",
        &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        &mut None,
    )
    .await;

    let err = result.expect_err("403 must surface as an error");
    assert!(
        matches!(
            &err,
            AzureDownloadError::AzureHttp {
                status_code: 403,
                ..
            }
        ),
        "must be AzureHttp 403, not RetryExhausted (no inline retry of 403); got: {err:?}"
    );
    assert_eq!(
        attempt.load(Ordering::SeqCst),
        1,
        "exactly one GET attempt — 403 must NOT be inline-retried"
    );
}

// ---------------------------------------------------------------
// 404 is a hard failure (not retried)
// ---------------------------------------------------------------

#[tokio::test]
async fn azure_download_404_is_not_retried() {
    let server = MockServer::start().await;
    let attempt = Arc::new(AtomicU32::new(0));

    let attempt_clone = attempt.clone();
    Mock::given(method("GET"))
        .respond_with(move |_: &Request| {
            attempt_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(404).set_body_string("Not Found")
        })
        .mount(&server)
        .await;

    let stage = azure_stage(&server.uri());
    let result = sf_core::file_manager::download_from_azure(
        &stage,
        "file.csv",
        &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        &mut None,
    )
    .await;

    assert!(result.is_err(), "404 should be a hard failure");
    assert_eq!(attempt.load(Ordering::SeqCst), 1, "should NOT retry 404");
}

// ---------------------------------------------------------------
// Standard retryable codes (503) are retried
// (matches all drivers)
// ---------------------------------------------------------------

#[tokio::test]
async fn azure_download_503_is_retried_then_succeeds() {
    let server = MockServer::start().await;
    let attempt = Arc::new(AtomicU32::new(0));

    let attempt_clone = attempt.clone();
    Mock::given(method("GET"))
        .respond_with(move |_: &Request| {
            let n = attempt_clone.fetch_add(1, Ordering::SeqCst);
            if n < 2 {
                ResponseTemplate::new(503).set_body_string("Service Unavailable")
            } else {
                azure_response_headers()
            }
        })
        .mount(&server)
        .await;

    let stage = azure_stage(&server.uri());
    let result = sf_core::file_manager::download_from_azure(
        &stage,
        "file.csv",
        &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        &mut None,
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
// Error body is sanitized (SAS tokens redacted)
// ---------------------------------------------------------------

#[tokio::test]
async fn azure_error_response_redacts_sas_token() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(400).set_body_string("Error: sig=secret123&se=2026 is invalid"),
        )
        .mount(&server)
        .await;

    let stage = azure_stage(&server.uri());
    let result = sf_core::file_manager::download_from_azure(
        &stage,
        "file.csv",
        &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        &mut None,
    )
    .await;

    let err = result.unwrap_err();
    let err_str = format!("{err}");
    assert!(
        !err_str.contains("secret123"),
        "SAS signature should be redacted from error, got: {err_str}"
    );
    assert!(
        err_str.contains("sig=REDACTED"),
        "Should contain redacted marker, got: {err_str}"
    );
}

// ---------------------------------------------------------------
// Transport errors do NOT leak SAS tokens
// ---------------------------------------------------------------

#[tokio::test]
async fn azure_transport_error_does_not_leak_sas_token() {
    // Bind a TCP listener, get its address, then drop it so the port is closed.
    // Connecting to this port produces a deterministic "connection refused" transport
    // error without any real DNS lookups.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let stage = StageInfo {
        location_type: LocationType::Azure,
        bucket: "test-container".to_string(),
        key_prefix: "prefix/".to_string(),
        region: "eastus2".to_string(),
        creds: CloudCredentials::Azure {
            sas_token: SensitiveString::from("sv=2021-08-06&sig=test-secret-sig&se=2099-01-01"),
        },
        endpoint: Some(format!("http://{addr}")),
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
        use_s3_regional_url: false,
        tls_config: sf_core::tls::config::TlsConfig::default(),
        crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
        storage_account: Some("test".to_string()),
    };

    let result = sf_core::file_manager::download_from_azure(
        &stage,
        "file.csv",
        &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        &mut None,
    )
    .await;

    let err = result.unwrap_err();
    let err_display = format!("{err}");
    let err_debug = format!("{err:?}");

    assert!(
        !err_display.contains("test-secret-sig"),
        "Display should not contain SAS signature, got: {err_display}"
    );
    assert!(
        !err_debug.contains("test-secret-sig"),
        "Debug should not contain SAS signature, got: {err_debug}"
    );
}

// ---------------------------------------------------------------
// Missing credentials / storage account produce clear errors
// ---------------------------------------------------------------

#[tokio::test]
async fn azure_download_with_wrong_creds_type_fails() {
    let server = MockServer::start().await;
    let mut stage = azure_stage(&server.uri());
    stage.creds = CloudCredentials::Gcs {
        gcs_access_token: None,
    };

    let result = sf_core::file_manager::download_from_azure(
        &stage,
        "file.csv",
        &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        &mut None,
    )
    .await;

    let err = result.unwrap_err();
    let err_str = format!("{err}");
    assert!(
        err_str.contains("Missing Azure credentials"),
        "Should report missing credentials, got: {err_str}"
    );
}

#[tokio::test]
async fn azure_download_with_missing_storage_account_fails() {
    let server = MockServer::start().await;
    let mut stage = azure_stage(&server.uri());
    stage.storage_account = None;
    // Remove scheme so it falls through to the standard URL path
    stage.endpoint = Some("blob.core.windows.net".to_string());

    let result = sf_core::file_manager::download_from_azure(
        &stage,
        "file.csv",
        &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        &mut None,
    )
    .await;

    let err = result.unwrap_err();
    let err_str = format!("{err}");
    assert!(
        err_str.contains("storage_account"),
        "Should report missing storage_account, got: {err_str}"
    );
}

// ---------------------------------------------------------------
// Persistent 403 with no refresher terminates without recovery
// (JDBC null-session analog: no credential source → terminal on first 403,
// symmetric with PUT which never inline-retried 403 either)
// ---------------------------------------------------------------

#[tokio::test]
async fn azure_download_persistent_403_with_no_refresher_terminates() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(|_: &Request| {
            ResponseTemplate::new(403).set_body_string(
                "<?xml version=\"1.0\"?><Error><Code>AuthenticationFailed</Code>\
                 <Message>Server failed to authenticate the request.</Message></Error>",
            )
        })
        .mount(&server)
        .await;

    let stage = azure_stage(&server.uri());
    // No refresher: the driver has no credential source to recover with.
    let result = sf_core::file_manager::download_from_azure(
        &stage,
        "file.csv",
        &test_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        &mut None,
    )
    .await;

    let err = result.expect_err("persistent 403 with no refresher must be a terminal error");
    assert!(
        matches!(
            &err,
            AzureDownloadError::AzureHttp {
                status_code: 403,
                ..
            }
        ),
        "terminal error must be AzureHttp 403 (no recovery loop, no inline retry, no panic); got: {err:?}"
    );
}

// ---------------------------------------------------------------
// Git stage objects: encryptiondata present but sfcdigest absent
// ---------------------------------------------------------------

#[tokio::test]
async fn azure_git_stage_download_succeeds_without_sfcdigest() {
    // Git stage objects on Azure carry CSE key-wrap headers (encryptiondata,
    // matdesc) but no sfcdigest — the object was uploaded by Snowflake's git
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
    // No x-ms-meta-sfcdigest header — matches what Snowflake's git integration uploads.
    // The streaming download path issues a HEAD (Get Blob Properties) first to learn
    // size + metadata, then a GET for the bytes. The CSE metadata is parsed from the
    // HEAD response, so the git-stage headers must live there; Content-Length stays
    // below the multipart threshold so the single streamed GET path runs.
    let raw_bytes = b"raw-git-file-bytes".to_vec();
    Mock::given(method("HEAD"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(vec![0u8; raw_bytes.len()])
                .insert_header("x-ms-meta-encryptiondata", enc_data.to_string().as_str())
                .insert_header("x-ms-meta-matdesc", mat_desc.to_string().as_str()),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(raw_bytes.clone())
                .insert_header("x-ms-meta-encryptiondata", enc_data.to_string().as_str())
                .insert_header("x-ms-meta-matdesc", mat_desc.to_string().as_str()),
        )
        .mount(&server)
        .await;

    let tmp = tempfile::tempdir().expect("tempdir");
    let local_location = tmp.path().to_string_lossy().to_string();

    let data = DownloadData {
        src_locations: vec!["file.txt".to_string()],
        local_location: local_location.clone(),
        stage_info: azure_stage(&server.uri()),
        encryption_materials: vec![Some(EncryptionMaterial {
            query_stage_master_key: SensitiveString::from("dGVzdC1tYXN0ZXIta2V5"),
            query_id: "test-query".to_string(),
            smk_id: "1".to_string(),
        })],
        presigned_urls: vec![None],
        flavor: PutGetResultsetFlavor::Python,
        multipart: MultipartParams::default(),
        unsafe_file_write: false,
    };

    let results = download_files(data, &RetryPolicy::put_get(&ParamStore::new()), None)
        .await
        .expect("git stage download should succeed even without sfcdigest");

    assert_eq!(results.len(), 1);
    let written = std::fs::read(std::path::Path::new(&local_location).join("file.txt"))
        .expect("downloaded file should exist");
    assert_eq!(written, b"raw-git-file-bytes");
}
