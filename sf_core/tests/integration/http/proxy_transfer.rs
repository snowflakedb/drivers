//! Hermetic proof that cloud-storage (Azure/GCS/S3) PUT/GET transfers transit
//! the driver's configured proxy.
//!
//! Each positive test drives a real transfer through a chain of
//! `client → ConnectProxy → TlsProxy → wiremock`, where [`ConnectProxy`] records
//! the authority of the `CONNECT` tunnel the client opens. A test passes only on
//! the **conjunction** of (a) the transfer succeeding and (b) the proxy having
//! observed a `CONNECT` to the exact origin host — never success alone, never a
//! bare "a CONNECT happened". Because the origin host is an RFC-6761 `.invalid`
//! name (guaranteed unresolvable) and each transfer builds a fresh client, a
//! false pass via direct-connection fallback or pooled-connection reuse is
//! structurally impossible: the only route to the backend is the tunnel.
//!
//! Reverting the proxy-wiring hunk in the matching implementation PR leaves the
//! client dialing the `.invalid` host directly, which fails to resolve — so the
//! transfer fails and the test fails. That mutation-sensitivity holds only
//! because of the success + exact-authority conjunction.
//!
//! Each cloud also has two negative controls: a dead proxy port (proves a broken
//! proxy path is detected, not silently bypassed) and a `no_proxy` bypass
//! (proves the exclusion list is honoured — zero CONNECTs recorded and the
//! direct dial to the unresolvable host fails).

use aws_sdk_s3::error::ProvideErrorMetadata;
use sf_core::apis::database_driver_v1::PutGetResultsetFlavor;
use sf_core::config::retry::RetryPolicy;
use sf_core::file_manager::internal::{
    azure_test_retry_policy, gcs_test_retry_policy, test_params,
};
use sf_core::file_manager::types::{
    ByteSource, CloudCredentials, LocationType, SingleUploadData, StageInfo,
};
use sf_core::file_manager::{
    AzureDownloadError, FileManagerError, GcsDownloadError, MultipartParams,
    SourceCompressionParam, UploadFileError, download_from_azure, download_from_gcs,
    upload_single_file,
};
use sf_core::sensitive::SensitiveString;
use sf_core::tls::config::{ProxyConfig, TlsConfig};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::common::connect_proxy::ConnectProxy;
use crate::common::tls_proxy::TlsProxy;

// RFC-6761 `.invalid` origins — no DNS entry can ever exist for these, so a
// request only reaches the backend if the CONNECT proxy maps the authority onto
// the loopback TlsProxy.
const AZURE_HOST: &str = "testaccount.blob.core.snowflake-proxy-test.invalid";
const GCS_HOST: &str = "storage.googleapis.snowflake-proxy-test.invalid";
const S3_HOST: &str = "s3.snowflake-proxy-test.invalid";
const S3_BUCKET: &str = "test-bucket";

fn azure_policy(attempts: u32) -> RetryPolicy {
    azure_test_retry_policy(attempts)
}
fn gcs_policy(attempts: u32) -> RetryPolicy {
    gcs_test_retry_policy(false, attempts)
}
fn s3_policy(attempts: u32) -> RetryPolicy {
    RetryPolicy::put_get(&test_params(attempts))
}

/// Starts `wiremock ← TlsProxy(sans) ← ConnectProxy` and returns all three. The
/// TlsProxy presents a cert valid for `sans` so TLS hostname verification of the
/// (fake) origin genuinely passes over the tunnel.
async fn start_backend_chain(sans: Vec<String>) -> (MockServer, TlsProxy, ConnectProxy) {
    let server = MockServer::start().await;
    let tls_proxy = TlsProxy::start_with_sans(*server.address(), sans).await;
    let connect_proxy = ConnectProxy::start(tls_proxy.addr()).await;
    (server, tls_proxy, connect_proxy)
}

/// A `TlsConfig` that genuinely trusts the TlsProxy's self-signed cert via a
/// custom root store (CRL disabled, hostname verification on). `dir` must be
/// kept alive until the transfer completes — the path is read at client build.
fn trusting_tls_config(tls_proxy: &TlsProxy, dir: &tempfile::TempDir) -> TlsConfig {
    let cert_path = dir.path().join("proxy-ca.pem");
    std::fs::write(&cert_path, tls_proxy.cert_pem()).expect("write proxy cert");
    TlsConfig {
        custom_root_store_path: Some(cert_path),
        ..TlsConfig::default()
    }
}

/// A `ProxyConfig` routing through `127.0.0.1:port` (env detection off), with an
/// optional `no_proxy` exclusion list.
fn proxy_via(port: u16, no_proxy: Option<&str>) -> ProxyConfig {
    ProxyConfig {
        host: Some("127.0.0.1".to_string()),
        port: Some(i64::from(port)),
        no_proxy: no_proxy.map(String::from),
        use_proxy_env: false,
        ..Default::default()
    }
}

/// A loopback port with nothing listening on it (bind then drop), for the
/// dead-proxy negative controls.
async fn dead_loopback_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

fn azure_stage(tls_config: TlsConfig, proxy_config: ProxyConfig) -> StageInfo {
    StageInfo {
        location_type: LocationType::Azure,
        bucket: "test-container".to_string(),
        key_prefix: "prefix/".to_string(),
        region: "eastus2".to_string(),
        creds: CloudCredentials::Azure {
            sas_token: SensitiveString::from("sv=2099-01-01&sig=test-sig&se=2099-01-01"),
        },
        endpoint: Some(format!("https://{AZURE_HOST}")),
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
        use_s3_regional_url: false,
        storage_account: Some("testaccount".to_string()),
        tls_config,
        crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
        proxy_config,
    }
}

fn gcs_stage(tls_config: TlsConfig, proxy_config: ProxyConfig) -> StageInfo {
    StageInfo {
        location_type: LocationType::Gcs,
        bucket: "test-bucket".to_string(),
        key_prefix: "prefix/".to_string(),
        region: "us-central1".to_string(),
        creds: CloudCredentials::Gcs {
            gcs_access_token: Some(SensitiveString::from("test-bearer-token")),
        },
        // Non-presigned custom-endpoint path so the target host is deterministic.
        endpoint: Some(format!("https://{GCS_HOST}")),
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
        use_s3_regional_url: false,
        storage_account: None,
        tls_config,
        crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
        proxy_config,
    }
}

fn s3_stage(tls_config: TlsConfig, proxy_config: ProxyConfig) -> StageInfo {
    StageInfo {
        location_type: LocationType::S3,
        bucket: S3_BUCKET.to_string(),
        key_prefix: String::new(),
        region: "us-east-1".to_string(),
        creds: CloudCredentials::S3 {
            aws_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
            aws_secret_key: SensitiveString::from("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"),
            aws_token: SensitiveString::from(""),
        },
        // Pinned fake endpoint: `resolve_s3_endpoint` returns it verbatim and
        // `should_skip_acceleration_probe` is true (endpoint set), so there is
        // no acceleration-probe CONNECT and no region-redirect variability.
        endpoint: Some(format!("https://{S3_HOST}")),
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
        use_s3_regional_url: false,
        storage_account: None,
        tls_config,
        crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
        proxy_config,
    }
}

fn s3_single_upload(stage: StageInfo) -> SingleUploadData {
    SingleUploadData {
        source: ByteSource::Bytes(b"proxy-s3-payload".to_vec().into()),
        filename: "file.bin".to_string(),
        stage_info: stage,
        encryption_material: None,
        auto_compress: false,
        source_compression: SourceCompressionParam::None,
        overwrite: true,
        flavor: PutGetResultsetFlavor::Python,
        legacy_odbc_compression_autodetect: false,
        skip_upload_on_content_match: false,
        // Defaults → below the multipart threshold → a single `PutObject`.
        multipart: MultipartParams::from_server(None, None),
    }
}

/// Asserts `err` is a transport/dispatch-level S3 failure — never a modeled S3
/// service response. A real S3 error (e.g. `NoSuchBucket`, `AccessDenied`)
/// always carries a `code()` from a genuine HTTP response the service sent; a
/// connector/dispatch failure (dead proxy, DNS failure) never got a response at
/// all, so `code()` is `None`. This is a stronger, non-stringly-typed signal
/// than matching the outer `FileManagerError::S3Upload` variant alone, which
/// would also match a legitimate (non-transport) S3 error.
fn assert_s3_connector_failure(err: FileManagerError) {
    let FileManagerError::S3Upload { source, .. } = err else {
        panic!("expected FileManagerError::S3Upload, got: {err:?}");
    };
    let UploadFileError::S3Upload {
        source: aws_err, ..
    } = source
    else {
        panic!("expected UploadFileError::S3Upload, got: {source:?}");
    };
    assert!(
        aws_err.code().is_none(),
        "expected a connector-level failure (no service response), \
         but got a modeled S3 error code: {:?} ({aws_err:?})",
        aws_err.code(),
    );
}

// ============================ Azure ============================

#[tokio::test]
async fn should_route_azure_download_through_proxy() {
    let (server, tls_proxy, connect_proxy) =
        start_backend_chain(vec![AZURE_HOST.to_string()]).await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"encrypted-data".to_vec())
                .insert_header("x-ms-meta-sfcdigest", "test-digest"),
        )
        .mount(&server)
        .await;

    let dir = tempfile::TempDir::new().unwrap();
    let stage = azure_stage(
        trusting_tls_config(&tls_proxy, &dir),
        proxy_via(connect_proxy.port(), None),
    );

    let result = download_from_azure(&stage, "file.csv", &azure_policy(3), None).await;

    // (a) the transfer genuinely succeeded through the tunnel...
    let response = result.expect("azure download via proxy should succeed");
    assert_eq!(response.data, b"encrypted-data");
    // (b) ...and its first wire action was a CONNECT to the exact origin host.
    assert_eq!(
        connect_proxy.observed_connects(),
        vec![format!("{AZURE_HOST}:443")],
    );
}

#[tokio::test]
async fn should_fail_azure_download_when_proxy_port_dead() {
    let port = dead_loopback_port().await;
    let stage = azure_stage(TlsConfig::default(), proxy_via(port, None));

    let err = download_from_azure(&stage, "file.csv", &azure_policy(1), None)
        .await
        .expect_err("dead proxy must fail the transfer");

    // A dead port yields a transport-layer failure (`Http`/`RetryExhausted`),
    // never an `AzureHttp { status_code }` — no server ever answered. Azure's
    // error boundary collapses reqwest's typed connect error to a string, so the
    // transport-vs-HTTP-status variant is the strictest typed check available.
    assert!(
        matches!(
            err,
            AzureDownloadError::Http { .. } | AzureDownloadError::RetryExhausted { .. }
        ),
        "expected a transport error, got: {err:?}"
    );
}

#[tokio::test]
async fn should_bypass_proxy_for_azure_no_proxy_host() {
    // Live proxy, but `no_proxy` excludes the origin — the client must dial it
    // directly (and fail, since it is unresolvable) without touching the proxy.
    let connect_proxy = ConnectProxy::start("127.0.0.1:1".parse().unwrap()).await;
    let stage = azure_stage(
        TlsConfig::default(),
        proxy_via(connect_proxy.port(), Some(AZURE_HOST)),
    );

    let err = download_from_azure(&stage, "file.csv", &azure_policy(1), None)
        .await
        .expect_err("bypassed proxy + unresolvable host must fail");

    assert!(
        matches!(
            err,
            AzureDownloadError::Http { .. } | AzureDownloadError::RetryExhausted { .. }
        ),
        "expected a transport error, got: {err:?}"
    );
    assert!(
        connect_proxy.observed_connects().is_empty(),
        "no_proxy host must never reach the proxy, saw: {:?}",
        connect_proxy.observed_connects(),
    );
}

// ============================ GCS ============================

#[tokio::test]
async fn should_route_gcs_download_through_proxy() {
    let (server, tls_proxy, connect_proxy) = start_backend_chain(vec![GCS_HOST.to_string()]).await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"encrypted-data".to_vec())
                .insert_header("x-goog-meta-sfc-digest", "test-digest"),
        )
        .mount(&server)
        .await;

    let dir = tempfile::TempDir::new().unwrap();
    let stage = gcs_stage(
        trusting_tls_config(&tls_proxy, &dir),
        proxy_via(connect_proxy.port(), None),
    );

    let result = download_from_gcs(&stage, "file.csv", None, &gcs_policy(3), 0, None).await;

    let response = result.expect("gcs download via proxy should succeed");
    assert_eq!(response.data, b"encrypted-data");
    assert_eq!(
        connect_proxy.observed_connects(),
        vec![format!("{GCS_HOST}:443")],
    );
}

#[tokio::test]
async fn should_fail_gcs_download_when_proxy_port_dead() {
    let port = dead_loopback_port().await;
    let stage = gcs_stage(TlsConfig::default(), proxy_via(port, None));

    let err = download_from_gcs(&stage, "file.csv", None, &gcs_policy(1), 0, None)
        .await
        .expect_err("dead proxy must fail the transfer");

    // GCS preserves the typed `reqwest::Error`, so we can assert the failure is
    // specifically a connect error (not a timeout or an HTTP-status error).
    match err {
        GcsDownloadError::Http { source, .. } => assert!(
            source.is_connect(),
            "expected a connect error, got: {source:?}"
        ),
        GcsDownloadError::RetryExhausted { .. } => {}
        other => panic!("expected a transport error, got: {other:?}"),
    }
}

#[tokio::test]
async fn should_bypass_proxy_for_gcs_no_proxy_host() {
    let connect_proxy = ConnectProxy::start("127.0.0.1:1".parse().unwrap()).await;
    let stage = gcs_stage(
        TlsConfig::default(),
        proxy_via(connect_proxy.port(), Some(GCS_HOST)),
    );

    let err = download_from_gcs(&stage, "file.csv", None, &gcs_policy(1), 0, None)
        .await
        .expect_err("bypassed proxy + unresolvable host must fail");

    assert!(
        matches!(
            err,
            GcsDownloadError::Http { .. } | GcsDownloadError::RetryExhausted { .. }
        ),
        "expected a transport error, got: {err:?}"
    );
    assert!(
        connect_proxy.observed_connects().is_empty(),
        "no_proxy host must never reach the proxy, saw: {:?}",
        connect_proxy.observed_connects(),
    );
}

// ============================ S3 ============================

#[tokio::test(flavor = "multi_thread")]
async fn should_route_s3_upload_through_proxy() {
    // The AWS SDK addresses a pinned custom endpoint virtual-host style
    // (`<bucket>.<host>`), confirmed empirically; the cert also covers the bare
    // host as a safety margin in case that addressing choice ever changes.
    let (server, tls_proxy, connect_proxy) =
        start_backend_chain(vec![S3_HOST.to_string(), format!("{S3_BUCKET}.{S3_HOST}")]).await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"mock-etag\""))
        .mount(&server)
        .await;

    let dir = tempfile::TempDir::new().unwrap();
    let stage = s3_stage(
        trusting_tls_config(&tls_proxy, &dir),
        proxy_via(connect_proxy.port(), None),
    );

    let result = upload_single_file(s3_single_upload(stage), &s3_policy(3), None).await;

    let upload = result.expect("s3 upload via proxy should succeed");
    assert_eq!(upload.status, "UPLOADED");
    // Exactly one CONNECT (single PutObject; the acceleration probe is skipped
    // because the endpoint is pinned), to the exact origin authority. The AWS
    // SDK addresses the pinned endpoint virtual-host style, prefixing the
    // bucket — a fixed, deterministic host for this fixed bucket + endpoint.
    assert_eq!(
        connect_proxy.observed_connects(),
        vec![format!("{S3_BUCKET}.{S3_HOST}:443")],
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_fail_s3_upload_when_proxy_port_dead() {
    let port = dead_loopback_port().await;
    let stage = s3_stage(TlsConfig::default(), proxy_via(port, None));

    let err = upload_single_file(s3_single_upload(stage), &s3_policy(1), None)
        .await
        .expect_err("dead proxy must fail the upload");

    assert_s3_connector_failure(err);
}

#[tokio::test(flavor = "multi_thread")]
async fn should_bypass_proxy_for_s3_no_proxy_host() {
    let connect_proxy = ConnectProxy::start("127.0.0.1:1".parse().unwrap()).await;
    let stage = s3_stage(
        TlsConfig::default(),
        proxy_via(connect_proxy.port(), Some(S3_HOST)),
    );

    let err = upload_single_file(s3_single_upload(stage), &s3_policy(1), None)
        .await
        .expect_err("bypassed proxy + unresolvable host must fail");

    assert_s3_connector_failure(err);
    assert!(
        connect_proxy.observed_connects().is_empty(),
        "no_proxy host must never reach the proxy, saw: {:?}",
        connect_proxy.observed_connects(),
    );
}
