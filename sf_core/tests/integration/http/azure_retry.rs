use sf_core::file_manager::{CloudCredentials, LocationType, StageInfo};
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
        end_point: Some(mock_uri.to_string()),
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
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
    let result = sf_core::file_manager::download_from_azure(&stage, "file.csv").await;

    let (data, digest, metadata) = result.expect("download should succeed");
    assert_eq!(data, b"encrypted-data");
    assert_eq!(digest, Some("test-digest".to_string()));
    let metadata = metadata.expect("encryption metadata should be present");
    assert_eq!(metadata.encrypted_key, "dGVzdC1rZXk=");
    assert_eq!(metadata.iv, "dGVzdC1pdg==");
    assert_eq!(metadata.material_desc.query_id, "test-query");
    assert_eq!(metadata.material_desc.smk_id, "1");
}

// ---------------------------------------------------------------
// 403 is retryable for Azure (SAS token clock skew / replication)
// (matches JDBC/ODBC behavior)
// ---------------------------------------------------------------

#[tokio::test]
async fn azure_download_403_is_retried_then_succeeds() {
    let server = MockServer::start().await;
    let attempt = Arc::new(AtomicU32::new(0));

    let attempt_clone = attempt.clone();
    Mock::given(method("GET"))
        .respond_with(move |_: &Request| {
            let n = attempt_clone.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                ResponseTemplate::new(403).set_body_string("Forbidden")
            } else {
                azure_response_headers()
            }
        })
        .mount(&server)
        .await;

    let stage = azure_stage(&server.uri());
    let result = sf_core::file_manager::download_from_azure(&stage, "file.csv").await;

    assert!(result.is_ok(), "403 should be retried and succeed");
    assert_eq!(
        attempt.load(Ordering::SeqCst),
        2,
        "should have retried once"
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
    let result = sf_core::file_manager::download_from_azure(&stage, "file.csv").await;

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
    let result = sf_core::file_manager::download_from_azure(&stage, "file.csv").await;

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
    let result = sf_core::file_manager::download_from_azure(&stage, "file.csv").await;

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
        end_point: Some(format!("http://{addr}")),
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
        storage_account: Some("test".to_string()),
    };

    let result = sf_core::file_manager::download_from_azure(&stage, "file.csv").await;

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

    let result = sf_core::file_manager::download_from_azure(&stage, "file.csv").await;

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
    stage.end_point = Some("blob.core.windows.net".to_string());

    let result = sf_core::file_manager::download_from_azure(&stage, "file.csv").await;

    let err = result.unwrap_err();
    let err_str = format!("{err}");
    assert!(
        err_str.contains("storage_account"),
        "Should report missing storage_account, got: {err_str}"
    );
}
