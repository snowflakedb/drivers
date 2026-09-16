//! Hermetic proof that a PUT/GET batch reuses one HTTP client across its
//! files, while single-file APIs that have no batch to join keep building
//! their own (SNOW-3978507).
//!
//! [`CountingProxy`] sits between the client and a `wiremock::MockServer`,
//! splicing every accepted connection through to the mock verbatim. Because
//! HTTP/1.1 keep-alive lets many requests ride one connection, the mock
//! server's own request log can't distinguish "one shared connection" from
//! "one connection per file" — only a connection count below the HTTP layer
//! can. Driving the batch with `parallel = 1` (so files run one at a time)
//! makes the first connection idle, not closed, when the second file's
//! request goes out — the only way the second file can reuse it rather than
//! opening its own.
//!
//! GCS GET, S3 PUT, and happy-path S3 GET require `accept_count == 1`. Azure
//! GET and the S3 ExpiredToken refresh path bound accepts instead: a HEAD
//! response or a 400 may close that socket on some Linux runners, so a
//! second accept is still one shared pool, not a client per file.
use sf_core::apis::database_driver_v1::PutGetResultsetFlavor;
use sf_core::config::param_registry::DEFAULT_PUT_GET_MAX_ATTEMPTS;
use sf_core::config::param_store::ParamStore;
use sf_core::config::retry::RetryPolicy;
use sf_core::file_manager::internal::{
    FakeStageInfoRefresher, azure_test_retry_policy, gcs_test_retry_policy,
};
use sf_core::file_manager::{
    CloudCredentials, DownloadData, FileManagerError, LocationType, MultipartParams,
    SourceCompressionParam, StageInfo, TransferCtx, UploadData, download_files, download_from_gcs,
    upload_files,
};
use sf_core::sensitive::SensitiveString;
use sf_core::tls::error::TlsError;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use crate::common::counting_proxy::CountingProxy;

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
        proxy_config: sf_core::tls::config::ProxyConfig::default(),
        storage_account: None,
    }
}

fn s3_stage_with_creds(endpoint: &str) -> StageInfo {
    StageInfo {
        location_type: LocationType::S3,
        bucket: "test-bucket".to_string(),
        key_prefix: "prefix/".to_string(),
        region: "us-east-1".to_string(),
        creds: CloudCredentials::S3 {
            aws_key_id: SensitiveString::from("AKIAIOSFODNN7EXAMPLE"),
            aws_secret_key: SensitiveString::from("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"),
            aws_token: None,
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

fn azure_stage_with_sas(endpoint: &str) -> StageInfo {
    StageInfo {
        location_type: LocationType::Azure,
        bucket: "test-container".to_string(),
        key_prefix: "prefix/".to_string(),
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
        proxy_config: sf_core::tls::config::ProxyConfig::default(),
    }
}

/// `command_parallel = 1`: file 2 cannot start until file 1's whole GET
/// (headers + body) completes, so the shared client's connection is idle —
/// not mid-request — when file 2's request goes out. That is the only
/// configuration in which "reused the same connection" and "raced a second
/// connection because both files were in flight" are actually distinguishable.
fn sequential_multipart() -> MultipartParams {
    MultipartParams {
        command_parallel: 1,
        ..MultipartParams::from_server(None, Some(1))
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn gcs_download_files_batch_shares_one_connection_across_files() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/test-bucket/prefix/a"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"bytes-for-a".to_vec()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/test-bucket/prefix/b"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"bytes-for-b".to_vec()))
        .mount(&server)
        .await;

    let proxy = CountingProxy::start(*server.address()).await;
    let stage = gcs_stage_with_token(&proxy.uri());

    let tmp = tempfile::tempdir().expect("tempdir");
    let data = DownloadData {
        src_locations: vec!["a".to_string(), "b".to_string()],
        local_location: tmp.path().to_string_lossy().to_string(),
        stage_info: stage,
        encryption_materials: vec![None, None],
        presigned_urls: vec![None, None],
        flavor: PutGetResultsetFlavor::Python,
        multipart: sequential_multipart(),
        unsafe_file_write: false,
        get_fastfail: true,
        cwd: None,
    };

    let results = download_files(
        data,
        &gcs_test_retry_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
        TransferCtx::default(),
    )
    .await
    .expect("both files must download successfully");

    assert_eq!(results.len(), 2, "both files must produce a result row");
    assert_eq!(
        proxy.accept_count(),
        1,
        "a two-file batch must reuse one shared HTTP client, hence one TCP \
         connection, not open a fresh one per file"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn gcs_single_file_downloads_do_not_share_a_connection() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/test-bucket/prefix/a"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"bytes-for-a".to_vec()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/test-bucket/prefix/b"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"bytes-for-b".to_vec()))
        .mount(&server)
        .await;

    let proxy = CountingProxy::start(*server.address()).await;
    let stage = gcs_stage_with_token(&proxy.uri());
    let policy = gcs_test_retry_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS);

    // Two sequential calls through the single-file API — no `TransferCtx`
    // carrying a batch client, exactly the "single-file callers still
    // succeed" acceptance criterion.
    download_from_gcs(&stage, "a", None, &policy, 0, None)
        .await
        .expect("first single-file download must succeed");
    download_from_gcs(&stage, "b", None, &policy, 0, None)
        .await
        .expect("second single-file download must succeed");

    assert_eq!(
        proxy.accept_count(),
        2,
        "single-file callers have no batch to join, so each builds its own \
         client — this is the unshared baseline the batch test above contrasts with"
    );
}

/// Same claim as the GCS download test above, on the S3 upload path.
#[tokio::test(flavor = "multi_thread")]
async fn s3_upload_files_batch_shares_one_connection_across_files() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"mock-etag\""))
        .mount(&server)
        .await;

    let proxy = CountingProxy::start(*server.address()).await;
    let stage = s3_stage_with_creds(&proxy.uri());

    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::write(tmp.path().join("a"), b"bytes-for-a").expect("write file a");
    std::fs::write(tmp.path().join("b"), b"bytes-for-b").expect("write file b");

    let data = UploadData {
        src_location_pattern: format!("{}/*", tmp.path().display()),
        stage_info: stage,
        encryption_material: None,
        auto_compress: false,
        source_compression: SourceCompressionParam::None,
        overwrite: true,
        flavor: PutGetResultsetFlavor::Python,
        legacy_odbc_compression_autodetect: false,
        skip_upload_on_content_match: false,
        multipart: sequential_multipart(),
        put_fastfail: true,
        cwd: None,
    };

    let results = upload_files(
        &data,
        &RetryPolicy::put_get(&ParamStore::new()),
        TransferCtx::default(),
    )
    .await
    .expect("both files must upload successfully");

    assert_eq!(results.len(), 2, "both files must produce a result row");
    assert_eq!(
        proxy.accept_count(),
        1,
        "a two-file PUT batch must reuse one shared HTTP client, hence one TCP \
         connection, not open a fresh one per file"
    );
}

/// Same claim as the GCS download test above, on Azure.
#[tokio::test(flavor = "multi_thread")]
async fn azure_download_files_batch_shares_one_connection_across_files() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/test-container/prefix/a"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-length", "11"))
        .mount(&server)
        .await;
    Mock::given(method("HEAD"))
        .and(path("/test-container/prefix/b"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-length", "11"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/test-container/prefix/a"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"bytes-for-a".to_vec()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/test-container/prefix/b"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"bytes-for-b".to_vec()))
        .mount(&server)
        .await;

    let proxy = CountingProxy::start(*server.address()).await;
    let stage = azure_stage_with_sas(&proxy.uri());

    let tmp = tempfile::tempdir().expect("tempdir");
    let data = DownloadData {
        src_locations: vec!["a".to_string(), "b".to_string()],
        local_location: tmp.path().to_string_lossy().to_string(),
        stage_info: stage,
        encryption_materials: vec![None, None],
        presigned_urls: vec![None, None],
        flavor: PutGetResultsetFlavor::Python,
        multipart: sequential_multipart(),
        unsafe_file_write: false,
        get_fastfail: true,
        cwd: None,
    };

    let results = download_files(
        data,
        &azure_test_retry_policy(DEFAULT_PUT_GET_MAX_ATTEMPTS),
        TransferCtx::default(),
    )
    .await
    .expect("both files must download successfully");

    assert_eq!(results.len(), 2, "both files must produce a result row");
    let accepts = proxy.accept_count();
    assert!(
        accepts <= 2,
        "a two-file Azure GET batch must share one HTTP client (4 requests). \
         A second TCP accept after HEAD is observed on some Linux runners; \
         a fresh client per request would be 4. got {accepts}"
    );
}

fn two_file_download(stage: StageInfo, local_location: String) -> DownloadData {
    DownloadData {
        src_locations: vec!["a".to_string(), "b".to_string()],
        local_location,
        stage_info: stage,
        encryption_materials: vec![None, None],
        presigned_urls: vec![None, None],
        flavor: PutGetResultsetFlavor::Python,
        multipart: sequential_multipart(),
        unsafe_file_write: false,
        get_fastfail: true,
        cwd: None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn s3_download_files_batch_shares_one_connection_across_files() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-length", "11"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"bytes-for-x".to_vec()))
        .mount(&server)
        .await;

    let proxy = CountingProxy::start(*server.address()).await;
    let tmp = tempfile::tempdir().expect("tempdir");
    let results = download_files(
        two_file_download(
            s3_stage_with_creds(&proxy.uri()),
            tmp.path().to_string_lossy().to_string(),
        ),
        &RetryPolicy::put_get(&ParamStore::new()),
        TransferCtx::default(),
    )
    .await
    .expect("both files must download successfully");

    assert_eq!(results.len(), 2, "both files must produce a result row");
    assert_eq!(
        proxy.accept_count(),
        1,
        "a two-file S3 GET batch must reuse one shared HTTP client across \
         files, including the S3Client rebuild on each attempt"
    );
}

/// First GET is AWS `ExpiredToken`; the refresher rotates creds and
/// `create_s3_client` rebuilds `S3Client` on the same `reqwest` pool.
#[tokio::test(flavor = "multi_thread")]
async fn s3_download_files_batch_reuses_the_pool_across_credential_refresh() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-length", "11"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(S3GetThenRefreshed::default())
        .mount(&server)
        .await;

    let proxy = CountingProxy::start(*server.address()).await;
    let stage = s3_stage_with_creds(&proxy.uri());
    let fake = FakeStageInfoRefresher::new(stage.creds.clone());
    fake.arm_rotation(CloudCredentials::S3 {
        aws_key_id: SensitiveString::from("AKIA-REFRESHED"),
        aws_secret_key: SensitiveString::from("refreshed-secret"),
        aws_token: Some(SensitiveString::from("refreshed-token")),
    });

    let tmp = tempfile::tempdir().expect("tempdir");
    let results = download_files(
        two_file_download(stage, tmp.path().to_string_lossy().to_string()),
        &RetryPolicy::put_get(&ParamStore::new()),
        TransferCtx::with_refresher(&fake),
    )
    .await
    .expect("the batch must succeed after rotating expired S3 credentials");

    assert_eq!(results.len(), 2, "both files must produce a result row");
    assert!(
        fake.refresh_call_count() == 1,
        "the first ExpiredToken GET must trigger exactly one credential refresh"
    );
    let accepts = proxy.accept_count();
    assert!(
        accepts <= 2,
        "rebuilding S3Client after credential refresh must keep the shared \
         reqwest pool. The ExpiredToken 400 may close that socket, so a second \
         accept is the retry GET; a new client per attempt would be 3. got {accepts}"
    );
}

#[derive(Default)]
struct S3GetThenRefreshed {
    calls: AtomicUsize,
}

impl Respond for S3GetThenRefreshed {
    fn respond(&self, _req: &Request) -> ResponseTemplate {
        if self.calls.fetch_add(1, Ordering::Relaxed) == 0 {
            ResponseTemplate::new(400)
                .set_body_string(
                    r#"<?xml version="1.0" encoding="UTF-8"?><Error><Code>ExpiredToken</Code><Message>The provided token has expired.</Message><RequestId>test-request-id</RequestId><HostId>test-host-id</HostId></Error>"#,
                )
                .insert_header("Content-Type", "application/xml")
        } else {
            ResponseTemplate::new(200).set_body_bytes(b"bytes-for-x".to_vec())
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn download_files_fails_the_batch_when_the_http_client_cannot_build() {
    let mut stage = gcs_stage_with_token("http://127.0.0.1:1");
    stage.tls_config.custom_root_store_path = Some(std::path::PathBuf::from("/no/such/roots.pem"));

    let tmp = tempfile::tempdir().expect("tempdir");
    let err = download_files(
        DownloadData {
            src_locations: vec!["a".to_string()],
            local_location: tmp.path().to_string_lossy().to_string(),
            stage_info: stage,
            encryption_materials: vec![None],
            presigned_urls: vec![None],
            flavor: PutGetResultsetFlavor::Python,
            multipart: sequential_multipart(),
            unsafe_file_write: false,
            get_fastfail: true,
            cwd: None,
        },
        &gcs_test_retry_policy(false, DEFAULT_PUT_GET_MAX_ATTEMPTS),
        TransferCtx::default(),
    )
    .await
    .expect_err("a missing custom root store must fail the batch before any file runs");

    let FileManagerError::BatchHttpClient { source, .. } = err else {
        panic!("expected BatchHttpClient, got: {err:?}");
    };
    assert!(
        matches!(source.as_ref(), TlsError::PemParse { .. }),
        "missing custom root store must surface as PemParse, got: {source:?}"
    );
}
