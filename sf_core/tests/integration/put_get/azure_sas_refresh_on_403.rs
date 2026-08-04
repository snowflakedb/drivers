use sf_core::apis::database_driver_v1::PutGetResultsetFlavor;
use sf_core::config::param_registry::DEFAULT_PUT_GET_MAX_ATTEMPTS;
use sf_core::file_manager::internal::azure_test_retry_policy;
use sf_core::file_manager::{
    AzureDownloadError, AzureUploadError, CloudCredentials, DownloadData, DownloadResult,
    FileManagerError, LocationType, SourceCompressionParam, StageInfo, StageInfoRefresher,
    UploadData, UploadResult, download_files, upload_files,
};
use sf_core::sensitive::SensitiveString;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tempfile::TempDir;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

// =============================================================================
// Test fixtures shared across scenarios
// =============================================================================

const ORIGINAL_SIG: &str = "ORIGINAL-EXPIRED-SIG";
const REFRESHED_SIG: &str = "REFRESHED-FRESH-SIG";

const ORIGINAL_SAS: &str = "sv=2021-08-06&sig=ORIGINAL-EXPIRED-SIG&se=2099-01-01";
const REFRESHED_SAS: &str = "sv=2021-08-06&sig=REFRESHED-FRESH-SIG&se=2099-01-01";

/// Azure XML body for an `AuthenticationFailed` error — the most common body
/// returned by Azure for a stale or invalid SAS token.
const AZURE_AUTH_FAILED_BODY: &str = "<?xml version=\"1.0\"?><Error><Code>AuthenticationFailed</Code><Message>Server failed to authenticate the request.</Message></Error>";

/// Azure XML body for an `AuthorizationFailure` error — used when a 403 is
/// caused by a storage-level policy denial rather than a stale SAS token.
const AZURE_AUTHZ_FAILURE_BODY: &str = "<?xml version=\"1.0\"?><Error><Code>AuthorizationFailure</Code><Message>This request is not authorized to perform this operation.</Message></Error>";

fn azure_stage(mock_uri: &str, sas: &str) -> StageInfo {
    StageInfo {
        location_type: LocationType::Azure,
        bucket: "test-container".to_string(),
        key_prefix: "prefix/".to_string(),
        region: "eastus2".to_string(),
        creds: CloudCredentials::Azure {
            sas_token: SensitiveString::from(sas.to_string()),
        },
        endpoint: Some(mock_uri.to_string()),
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
        use_s3_regional_url: false,
        storage_account: Some("test".to_string()),
        tls_config: Default::default(),
        crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
    }
}

// =============================================================================
// Scoped log capture (per-test, thread-local `set_default` guard — NOT a global
// subscriber). Mirrors `session/logout.rs::capturing_subscriber`; avoids the
// global-subscriber conflict `#[traced_test]` triggers alongside other tests'
// `setup_logging()` in the shared integration binary (which poisoned a `Once`).
// =============================================================================
fn capturing_subscriber() -> (
    tracing::subscriber::DefaultGuard,
    &'static std::sync::Mutex<Vec<u8>>,
) {
    let buf: &'static std::sync::Mutex<Vec<u8>> =
        Box::leak(Box::new(std::sync::Mutex::new(Vec::new())));
    let mock_writer = tracing_test::internal::MockWriter::new(buf);
    let dispatch = tracing_test::internal::get_subscriber(mock_writer, "trace");
    let guard = tracing::dispatcher::set_default(&dispatch);
    (guard, buf)
}

fn captured(buf: &std::sync::Mutex<Vec<u8>>) -> String {
    String::from_utf8(buf.lock().unwrap().clone()).unwrap_or_default()
}

/// 200 OK response template with the metadata headers required by the
/// download path.
fn azure_200_with_headers(body: &[u8]) -> ResponseTemplate {
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
        .set_body_bytes(body.to_vec())
        .insert_header("x-ms-meta-sfcdigest", "test-digest")
        .insert_header("x-ms-meta-encryptiondata", enc_data.to_string().as_str())
        .insert_header("x-ms-meta-matdesc", mat_desc.to_string().as_str())
}

/// Writes a tiny test file to `dir` and returns its path. `upload_files`
/// goes through `expand_filenames` which requires a real disk path.
fn write_test_file(dir: &TempDir, filename: &str, contents: &[u8]) -> PathBuf {
    let path = dir.path().join(filename);
    std::fs::write(&path, contents).expect("write test file");
    path
}

fn upload_data_for(stage: StageInfo, src: &str) -> UploadData {
    UploadData {
        src_location_pattern: src.to_string(),
        stage_info: stage,
        encryption_material: None,
        auto_compress: false,
        source_compression: SourceCompressionParam::None,
        overwrite: true,
        flavor: PutGetResultsetFlavor::Python,
        legacy_odbc_compression_autodetect: false,
        skip_upload_on_content_match: false,
        multipart: sf_core::file_manager::MultipartParams::default(),
        // Fail-fast: this suite asserts on the specific typed terminal error
        // (e.g. AzureUpload::AzureHttp) from a single-file transfer; collect-all's
        // aggregate `UploadBatchSnafu` would stringify it away.
        put_fastfail: true,
    }
}

fn download_data_for(stage: StageInfo, src_blob: &str, local_dir: &TempDir) -> DownloadData {
    DownloadData {
        src_locations: vec![src_blob.to_string()],
        local_location: local_dir.path().to_string_lossy().into_owned(),
        stage_info: stage,
        encryption_materials: vec![None],
        // Azure uses the SAS embedded in the blob URL, not per-file presigned URLs.
        presigned_urls: vec![None],
        flavor: PutGetResultsetFlavor::Python,
        multipart: sf_core::file_manager::MultipartParams::default(),
        unsafe_file_write: false,
        // See `upload_data_for`'s `put_fastfail` comment: same reasoning.
        get_fastfail: true,
    }
}

// =============================================================================
// Test refresher
//
// The shared `FakeStageInfoRefresher` (in `file_manager::internal`, exposed via
// the `test-utils` feature) is used across the Azure/S3/GCS transfer tests:
// records call count, rotates the production `StageInfoCache` on demand, and can
// be armed to fail. See its docs for what it does NOT prove (notably the
// production coalescing window in `SnowflakeStageInfoRefresher`).
// =============================================================================
use sf_core::file_manager::internal::FakeStageInfoRefresher;

// =============================================================================
// Shared arrange-and-act helpers
// =============================================================================

struct PutOutcome {
    result: Result<Vec<UploadResult>, FileManagerError>,
    refresher: FakeStageInfoRefresher,
    server: MockServer,
    logs: String,
}

async fn run_put_with_refresh<R, A>(
    filename: &str,
    file_contents: &[u8],
    responder: R,
    arm: A,
) -> PutOutcome
where
    R: Fn(&Request) -> ResponseTemplate + Send + Sync + 'static,
    A: FnOnce(&FakeStageInfoRefresher),
{
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .respond_with(responder)
        .mount(&server)
        .await;

    let stage = azure_stage(&server.uri(), ORIGINAL_SAS);
    let refresher = FakeStageInfoRefresher::new(stage.creds.clone());
    arm(&refresher);

    let tmp = TempDir::new().expect("tempdir");
    let src = write_test_file(&tmp, filename, file_contents);
    let data = upload_data_for(stage, src.to_str().unwrap());

    let (_guard, log_buf) = capturing_subscriber();
    let result = upload_files(
        &data,
        &azure_test_retry_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        Some(&refresher as &dyn StageInfoRefresher),
    )
    .await;
    let logs = captured(log_buf);

    PutOutcome {
        result,
        refresher,
        server,
        logs,
    }
}

/// 403 on first PUT until the SAS query parameter changes to the refreshed value.
fn put_403_until_refreshed_responder()
-> impl Fn(&Request) -> ResponseTemplate + Send + Sync + 'static {
    |req: &Request| {
        let url = req.url.as_str();
        if url.contains(REFRESHED_SIG) {
            ResponseTemplate::new(201)
        } else {
            ResponseTemplate::new(403).set_body_string(AZURE_AUTH_FAILED_BODY)
        }
    }
}

fn arm_refresh_to_fresh_sas(refresher: &FakeStageInfoRefresher) {
    refresher.arm_rotation(CloudCredentials::Azure {
        sas_token: SensitiveString::from(REFRESHED_SAS.to_string()),
    });
}

// =============================================================================
// Scenario: should refresh SAS and succeed when Azure PUT returns 403 on the first attempt
// =============================================================================

/// Pins Gherkin: "should refresh SAS and succeed when Azure PUT returns 403
/// on the first attempt".
#[tokio::test]
async fn should_refresh_sas_and_succeed_when_azure_put_returns_403_on_the_first_attempt() {
    // Given Snowflake client is logged in to an Azure-backed deployment
    let outcome = run_put_with_refresh(
        "scenario1.dat",
        b"hello-azure-sas-refresh",
        // And Stage SAS is configured to return HTTP 403 on the first PUT attempt
        put_403_until_refreshed_responder(),
        arm_refresh_to_fresh_sas,
    )
    // When File is uploaded using PUT command
    .await;

    // Then The PUT query is re-issued to obtain a fresh stage credential
    assert_eq!(
        outcome.refresher.refresh_call_count(),
        1,
        "exactly one refresh expected on the single-403 path"
    );
    // And File should be uploaded successfully with the refreshed SAS
    assert!(
        outcome.result.is_ok(),
        "PUT must succeed via SAS refresh on first 403. got: {:?}",
        outcome.result.err()
    );
    let requests = outcome.server.received_requests().await.unwrap_or_default();
    assert!(
        requests
            .iter()
            .any(|r| r.method.as_str() == "PUT" && r.url.as_str().contains(REFRESHED_SIG)),
        "post-refresh PUT must carry the refreshed SAS in its URL"
    );
    // And No warn-level log line is emitted for the recovered 403
    assert!(
        !outcome.logs.contains("WARN"),
        "recovered 403 must log at debug, NOT warn"
    );
    // And The request body is rebuilt for the post-refresh attempt
    let post_refresh_put = requests
        .iter()
        .find(|r| r.method.as_str() == "PUT" && r.url.as_str().contains(REFRESHED_SIG))
        .expect("expected a POST-refresh PUT carrying the refreshed SAS");
    assert_eq!(
        post_refresh_put.body.as_slice(),
        b"hello-azure-sas-refresh",
        "per-attempt source rebuild: the post-refresh PUT body must match the source file byte-for-byte"
    );
}

// =============================================================================
// Scenario: should refresh SAS and re-drive the GET once when Azure returns 403
// =============================================================================

struct GetOutcome {
    result: Result<Vec<DownloadResult>, FileManagerError>,
    refresher: FakeStageInfoRefresher,
    server: MockServer,
    /// Kept alive so the downloaded file path remains valid for content
    /// read-back assertions.
    local_tmp: TempDir,
    logs: String,
}

/// GET analogue of [`run_put_with_refresh`]: mounts a 200 routing-HEAD plus
/// `responder` on GET, arms the fake refresher via `arm`, then drives
/// `download_files` and returns the outcome.
async fn run_get_with_refresh<R, A>(blob_name: &str, responder: R, arm: A) -> GetOutcome
where
    R: Fn(&Request) -> ResponseTemplate + Send + Sync + 'static,
    A: FnOnce(&FakeStageInfoRefresher),
{
    let server = MockServer::start().await;
    // The Azure download issues a routing HEAD (Get Blob Properties) before the
    // GET to learn size + metadata. Answer it 200 with the metadata headers and
    // an empty body (content-length 0 → the single streaming-GET path, below the
    // multipart threshold), so the 403 under test lands on the GET.
    Mock::given(method("HEAD"))
        .respond_with(azure_200_with_headers(b""))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(responder)
        .mount(&server)
        .await;

    let stage = azure_stage(&server.uri(), ORIGINAL_SAS);
    let refresher = FakeStageInfoRefresher::new(stage.creds.clone());
    arm(&refresher);

    let local_tmp = TempDir::new().expect("tempdir");
    let data = download_data_for(stage, blob_name, &local_tmp);

    let (_guard, log_buf) = capturing_subscriber();
    let result = download_files(
        data,
        &azure_test_retry_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        Some(&refresher as &dyn StageInfoRefresher),
    )
    .await;
    let logs = captured(log_buf);

    GetOutcome {
        result,
        refresher,
        server,
        local_tmp,
        logs,
    }
}

/// Happy-path GET responder: 403 on the first attempt, 200 once the SAS rotates.
async fn run_get_with_403(blob_name: &str, body: &'static [u8]) -> GetOutcome {
    let attempts = Arc::new(AtomicU32::new(0));
    run_get_with_refresh(
        blob_name,
        move |req: &Request| {
            let n = attempts.fetch_add(1, Ordering::SeqCst);
            let url = req.url.as_str();
            if n == 0 && !url.contains(REFRESHED_SIG) {
                ResponseTemplate::new(403).set_body_string(AZURE_AUTH_FAILED_BODY)
            } else if url.contains(REFRESHED_SIG) {
                azure_200_with_headers(body)
            } else {
                ResponseTemplate::new(403).set_body_string(AZURE_AUTH_FAILED_BODY)
            }
        },
        arm_refresh_to_fresh_sas,
    )
    .await
}

/// Pins Gherkin: "should refresh SAS and re-drive the GET once when Azure
/// returns 403".
///
/// The Azure download path issues a single streaming GET (not Range-chunked).
/// A 403 is decided at response-status phase (before any body byte). Recovery
/// is: re-issue the GET query for a fresh SAS and re-drive the whole GET once.
/// There is no mid-stream or per-chunk semantics here.
#[tokio::test]
async fn should_refresh_sas_and_re_drive_the_get_once_when_azure_returns_403() {
    // Given Snowflake client is logged in to an Azure-backed deployment
    let blob_filename = "scenario2.dat";
    let blob_name = format!("prefix/{blob_filename}");
    // And File is uploaded to an Azure-backed stage
    let blob_body: &[u8] = b"AAAAAAAA-BBBBBBBB-CCCCCCCC";
    // And Stage SAS is configured to return HTTP 403 on the GET
    let outcome = {
        // When File is downloaded using GET command
        run_get_with_403(&blob_name, blob_body).await
    };

    // Then The GET query is re-issued to obtain a fresh stage credential
    assert_eq!(
        outcome.refresher.refresh_call_count(),
        1,
        "exactly one refresh expected: GET is a single streaming request, one 403 → one refresh"
    );
    // And The GET is re-driven exactly once carrying the refreshed SAS
    let requests = outcome.server.received_requests().await.unwrap_or_default();
    let refreshed_gets = requests
        .iter()
        .filter(|r| r.method.as_str() == "GET" && r.url.as_str().contains(REFRESHED_SIG))
        .count();
    assert_eq!(
        refreshed_gets, 1,
        "exactly one GET must carry the refreshed SAS (single re-drive, not a restart storm). \
         observed {refreshed_gets}"
    );
    // And File should be downloaded with correct content
    assert!(
        outcome.result.is_ok(),
        "GET must succeed after 403 via SAS refresh + whole-GET re-drive. got: {:?}",
        outcome.result.err()
    );
    let downloaded = std::fs::read(outcome.local_tmp.path().join(blob_filename))
        .expect("read downloaded file from local_tmp");
    assert_eq!(
        downloaded, blob_body,
        "downloaded file content must match the served blob bytes byte-for-byte"
    );
    // And No warn-level log line is emitted for the recovered 403
    assert!(
        !outcome.logs.contains("WARN"),
        "recovered GET 403 must log at debug, NOT warn"
    );
}

// =============================================================================
// Scenario: should surface terminal error when GET SAS refresh itself fails
// =============================================================================

/// Pins Gherkin: "should surface terminal error when GET SAS refresh itself
/// fails". GET twin of
/// `should_surface_terminal_error_when_put_sas_refresh_itself_fails`: GET goes
/// through `azure_get_attempt` (which drains an error body) rather than the PUT
/// attempt path, so the refresh-failure surface is exercised independently.
#[tokio::test]
async fn should_surface_terminal_error_when_get_sas_refresh_itself_fails() {
    // Given Snowflake client is logged in to an Azure-backed deployment
    let blob_name = "prefix/scenario_get_refresh_fail.dat";
    // And File is uploaded to an Azure-backed stage
    let outcome = run_get_with_refresh(
        blob_name,
        // And Stage SAS is configured to return HTTP 403 on the GET
        |_: &Request| ResponseTemplate::new(403).set_body_string(AZURE_AUTH_FAILED_BODY),
        // And Snowflake GS is unreachable for the refresh query
        |refresher| {
            // Arm the fake refresher so the refresh query itself fails.
            refresher.arm_failure("GS unreachable: connection refused")
        },
    )
    // When File is downloaded using GET command
    .await;

    // Then The GET query is re-issued to obtain a fresh stage credential
    assert_eq!(
        outcome.refresher.refresh_call_count(),
        1,
        "driver calls refresh() once, it fails, and the error surfaces immediately"
    );
    // And An error is raised indicating SAS refresh failed
    assert!(
        outcome.result.is_err(),
        "refresh failure must surface as a terminal download error"
    );
    let err_str = format!("{:?}", outcome.result.unwrap_err());
    assert!(
        err_str.contains("GS unreachable") || err_str.contains("ServerRejected"),
        "terminal error must carry the refresh-failure detail. got: {err_str}"
    );
    // And An error-level log line is emitted naming the refresh-failure reason
    assert!(
        outcome.logs.contains("ERROR"),
        "terminal refresh-mechanism failure must emit an error-level log"
    );
    assert!(
        outcome.logs.contains("SAS refresh failed"),
        "error log must name the refresh failure via the distinctive terminal-error \
         phrase, not the INFO refresh breadcrumb that fires on every attempt"
    );
}

// =============================================================================
// Scenario: should retry then fail when Azure GET 403 is not caused by SAS expiry
// =============================================================================

/// Pins Gherkin: "should retry then fail when Azure GET 403 is not caused by
/// SAS expiry". GET twin of
/// `should_retry_then_fail_when_azure_put_403_is_not_caused_by_sas_expiry`.
#[tokio::test]
async fn should_retry_then_fail_when_azure_get_403_is_not_caused_by_sas_expiry() {
    // Given Snowflake client is logged in to an Azure-backed deployment
    let blob_name = "prefix/scenario_get_non_token_403.dat";
    // And File is uploaded to an Azure-backed stage
    let outcome = run_get_with_refresh(
        blob_name,
        // And Stage SAS is configured to return HTTP 403 on the GET for a non-token reason
        |_: &Request| {
            // Always 403, even for the refreshed SAS — bucket-policy denial, not token expiry.
            ResponseTemplate::new(403).set_body_string(AZURE_AUTHZ_FAILURE_BODY)
        },
        arm_refresh_to_fresh_sas,
    )
    // When File is downloaded using GET command
    .await;

    // Then The GET query is re-issued to obtain a fresh stage credential
    assert!(
        outcome.refresher.refresh_call_count() >= 1,
        "any-403 predicate must trigger at least one refresh before giving up"
    );
    // And The re-driven GET is also rejected with HTTP 403
    assert!(
        outcome
            .server
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .any(|r| r.method.as_str() == "GET" && r.url.as_str().contains(REFRESHED_SIG)),
        "a GET carrying the refreshed SAS must have been attempted before the terminal 403"
    );
    // And An error is raised indicating Azure storage returned HTTP 403
    let err = outcome
        .result
        .expect_err("non-token 403 that survives refresh must surface as a terminal error");
    assert!(
        matches!(
            &err,
            FileManagerError::AzureDownload {
                source: AzureDownloadError::AzureHttp {
                    status_code: 403,
                    ..
                },
                ..
            }
        ),
        "terminal non-token 403 must surface as AzureDownload/AzureHttp status 403; got: {err:?}"
    );
    // And A warn-level log line is emitted at status 403
    assert!(
        outcome.logs.contains("WARN"),
        "terminal non-token 403 must emit a WARN (contract-drift signal)"
    );
    // And The warn log names the Azure error code
    assert!(
        outcome.logs.contains("AuthorizationFailure"),
        "warn line must name the Azure error code AuthorizationFailure"
    );
    // And The warn log carries a SAS-redacted URL
    assert!(
        outcome.logs.contains("/test-container/prefix/")
            && !outcome.logs.contains(ORIGINAL_SIG)
            && !outcome.logs.contains(REFRESHED_SIG),
        "warn line must carry the host+path-redacted URL (the container/prefix path) \
         and never the raw SAS signature"
    );
}

// =============================================================================
// Scenario: should recover both concurrent PUTs via the shared refreshed SAS
// =============================================================================

/// Pins Gherkin: "should recover both concurrent PUTs via the shared refreshed
/// SAS".
///
/// Synthetic scaffolding, NOT a production topology: production's
/// `SnowflakeStageInfoRefresher` coalesces concurrent refreshes via its
/// single-flight `inflight` coordinator, and `upload_files` is sequential. Here
/// two `upload_files` calls share one `Arc`-wrapped fake under `tokio::join!`;
/// the fake rotates the cache on the first refresh and no-ops the second
/// (first-rotation-wins).
///
/// What this proves: **wrapper-level recovery** — each PUT re-reads the rotated
/// credential from the shared `StageInfoCache` and succeeds after a 403. It
/// intentionally does NOT assert the GS-refresh-call count; that invariant is
/// proven against the real coordinator type in
/// `should_coalesce_n_concurrent_refresh_callers_into_one_fetch`
/// (`within_coalesce_window`) and at block level in the upstack multipart
/// coalescing test.
#[tokio::test]
async fn should_recover_both_concurrent_puts_via_the_shared_refreshed_sas() {
    // Given Snowflake client is logged in to an Azure-backed deployment
    let server = MockServer::start().await;
    // And Stage SAS is configured to return HTTP 403 for both concurrent operations
    Mock::given(method("PUT"))
        .respond_with(put_403_until_refreshed_responder())
        .mount(&server)
        .await;

    let stage = azure_stage(&server.uri(), ORIGINAL_SAS);

    // Two clones share one Arc<RwLock> StageInfoCache — the production-like part.
    // Sharing the refresher across tasks is test scaffolding (prod: one per query).
    // And Two PUT operations are running in parallel against the same Azure stage
    let refresher_a = FakeStageInfoRefresher::new(stage.creds.clone());
    refresher_a.arm_rotation(CloudCredentials::Azure {
        sas_token: SensitiveString::from(REFRESHED_SAS.to_string()),
    });
    let refresher_b = refresher_a.clone();

    let tmp_a = TempDir::new().expect("tempdir-a");
    let src_a = write_test_file(&tmp_a, "concurrent_a.dat", b"payload-a");
    let tmp_b = TempDir::new().expect("tempdir-b");
    let src_b = write_test_file(&tmp_b, "concurrent_b.dat", b"payload-b");

    let stage_a = stage.clone();
    let stage_b = stage.clone();
    let data_a = upload_data_for(stage_a, src_a.to_str().unwrap());
    let data_b = upload_data_for(stage_b, src_b.to_str().unwrap());

    let r_a = refresher_a;
    let r_b = refresher_b;
    let task_a = async move {
        upload_files(
            &data_a,
            &azure_test_retry_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            Some(&r_a as &dyn StageInfoRefresher),
        )
        .await
    };
    let task_b = async move {
        upload_files(
            &data_b,
            &azure_test_retry_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
            Some(&r_b as &dyn StageInfoRefresher),
        )
        .await
    };
    // When Both PUT operations trigger SAS refresh concurrently
    let (result_a, result_b) = tokio::join!(task_a, task_b);

    // Then Both PUTs carry the shared refreshed SAS at the wire
    let reqs = server.received_requests().await.unwrap_or_default();
    let refreshed_puts: Vec<_> = reqs
        .iter()
        .filter(|r| r.method.as_str() == "PUT" && r.url.as_str().contains(REFRESHED_SIG))
        .collect();
    // Exactly one refreshed PUT per file: each upload can only succeed by
    // re-reading the rotated credential and re-PUTting with it (the mock 200s
    // only for REFRESHED_SIG), so two successful files == two refreshed PUTs in
    // every interleaving. This is the stable wire invariant; the count of GS
    // refresh *calls* is interleaving-dependent (1 or 2) and deliberately not
    // asserted — see this test's docstring.
    assert_eq!(
        refreshed_puts.len(),
        2,
        "each concurrent PUT must carry the refreshed SAS exactly once (one per \
         file); saw: {:?}",
        refreshed_puts
            .iter()
            .map(|r| r.url.as_str())
            .collect::<Vec<_>>()
    );
    // And Both operations succeed with the shared refreshed SAS
    assert!(
        result_a.is_ok() && result_b.is_ok(),
        "both concurrent PUTs must succeed via the shared refreshed SAS. \
         a: {:?} b: {:?}",
        result_a.err(),
        result_b.err()
    );
}

// =============================================================================
// Scenario: should surface terminal error when PUT SAS refresh itself fails
// =============================================================================

/// Pins Gherkin: "should surface terminal error when PUT SAS refresh itself
/// fails".
#[tokio::test]
async fn should_surface_terminal_error_when_put_sas_refresh_itself_fails() {
    // Given Snowflake client is logged in to an Azure-backed deployment
    let outcome = run_put_with_refresh(
        "scenario4.dat",
        b"will-not-upload",
        // And Stage SAS is configured to return HTTP 403 on the PUT
        |_: &Request| ResponseTemplate::new(403).set_body_string(AZURE_AUTH_FAILED_BODY),
        // And Snowflake GS is unreachable for the refresh query
        |refresher| {
            // Arm the fake refresher so the refresh query itself fails.
            refresher.arm_failure("GS unreachable: connection refused")
        },
    )
    // When File is uploaded using PUT command
    .await;

    // Then The PUT query is re-issued to obtain a fresh stage credential
    assert_eq!(
        outcome.refresher.refresh_call_count(),
        1,
        "driver calls refresh() once, it fails, and the error surfaces immediately"
    );
    // And An error is raised indicating SAS refresh failed
    assert!(
        outcome.result.is_err(),
        "refresh failure must surface as a terminal upload error"
    );
    let err_str = format!("{:?}", outcome.result.unwrap_err());
    assert!(
        err_str.contains("GS unreachable") || err_str.contains("ServerRejected"),
        "terminal error must carry the refresh-failure detail. got: {err_str}"
    );
    // And An error-level log line is emitted naming the refresh-failure reason
    assert!(
        outcome.logs.contains("ERROR"),
        "terminal refresh-mechanism failure must emit an error-level log"
    );
    assert!(
        outcome.logs.contains("SAS refresh failed"),
        "error log must name the refresh failure via the distinctive terminal-error \
         phrase, not the INFO refresh breadcrumb that fires on every attempt"
    );
}

// =============================================================================
// Scenario: should retry then fail when Azure PUT 403 is not caused by SAS expiry
// =============================================================================

/// Pins Gherkin: "should retry then fail when Azure PUT 403 is not caused by
/// SAS expiry".
///
/// Mock always 403s, even for the refreshed SAS. The driver must refresh once,
/// retry, and when the post-refresh attempt also 403s surface a terminal error.
#[tokio::test]
async fn should_retry_then_fail_when_azure_put_403_is_not_caused_by_sas_expiry() {
    // Given Snowflake client is logged in to an Azure-backed deployment
    let outcome = run_put_with_refresh(
        "scenario5.dat",
        b"will-not-succeed",
        // And Stage SAS is configured to return HTTP 403 for a non-token reason
        |_: &Request| {
            // Always 403, even for the refreshed SAS — bucket-policy denial, not token expiry.
            // AuthorizationFailure exercises the <Code>-parse path through a non-expiry code.
            ResponseTemplate::new(403).set_body_string(AZURE_AUTHZ_FAILURE_BODY)
        },
        arm_refresh_to_fresh_sas,
    )
    // When File is uploaded using PUT command
    .await;

    // Then The PUT query is re-issued to obtain a fresh stage credential
    assert!(
        outcome.refresher.refresh_call_count() >= 1,
        "any-403 predicate must trigger at least one refresh before giving up"
    );
    // And The refreshed SAS is also rejected with HTTP 403
    assert!(
        outcome
            .server
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .any(|r| r.method.as_str() == "PUT" && r.url.as_str().contains(REFRESHED_SIG)),
        "a PUT carrying the refreshed SAS must have been attempted before the terminal 403"
    );
    // And An error is raised indicating Azure storage returned HTTP 403
    let err = outcome
        .result
        .expect_err("non-token 403 that survives refresh must surface as a terminal error");
    assert!(
        matches!(
            &err,
            FileManagerError::AzureUpload {
                source: AzureUploadError::AzureHttp {
                    status_code: 403,
                    ..
                },
                ..
            }
        ),
        "terminal non-token 403 must surface as AzureUpload/AzureHttp status 403; got: {err:?}"
    );
    // And A warn-level log line is emitted at status 403
    assert!(
        outcome.logs.contains("WARN"),
        "terminal non-token 403 must emit a WARN (contract-drift signal)"
    );
    // And The warn log names the Azure error code
    assert!(
        outcome.logs.contains("AuthorizationFailure"),
        "warn line must name the Azure error code AuthorizationFailure"
    );
    // And The warn log carries a SAS-redacted URL
    assert!(
        outcome.logs.contains("/test-container/prefix/")
            && !outcome.logs.contains(ORIGINAL_SIG)
            && !outcome.logs.contains(REFRESHED_SIG),
        "warn line must carry the host+path-redacted URL (the container/prefix path) \
         and never the raw SAS signature"
    );
}
