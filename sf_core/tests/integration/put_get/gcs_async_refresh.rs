//! Integration tests for the async PUT/GET `sqlText` refresh-context path and the
//! `sql_text == None` fallback driven through `connection_get_query_result`.
//!
//! The key branch under test is in `statement.rs`:
//!
//! ```text
//! let stage_info_refresh_context =
//!     data.sql_text
//!         .as_ref()
//!         .map(|sql| StageInfoRefreshContext { sql, ... });
//! ```
//!
//! - When `sqlText` is absent: `stage_info_refresh_context = None` → no refresh on GCS
//!   400/401 → hard failure.
//! - When `sqlText` is present: a `StageInfoRefreshContext` is built → GCS 400 triggers
//!   re-issue of the original SQL → refreshed `presignedUrls[]` → retry succeeds.

use crate::common::mocks;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_QUERY_ID: &str = "00000000-0000-0000-0000-000000000001";

/// GCS SSE response (no encryption metadata); body is written verbatim.
fn gcs_sse_response(body: &'static [u8]) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_bytes(body.to_vec())
        .insert_header("x-goog-meta-sfc-digest", "test-digest")
}

/// `connection_get_query_result` with `sqlText` absent and GCS returning 400:
/// no `StageInfoRefreshContext` is built, so the 400 is a hard failure — the
/// file transfer fails with no recovery attempt.
#[tokio::test(flavor = "multi_thread")]
async fn connection_get_query_result_sql_text_none_falls_back_to_no_refresh() {
    let sf_server = MockServer::start().await;
    let gcs_server = MockServer::start().await;

    mocks::auth::mount_jwt_login_success(&sf_server).await;

    let tmp = tempfile::tempdir().expect("tempdir");
    let local_location = tmp.path().to_string_lossy().to_string();

    let stale_url = format!("{}/stale-file", gcs_server.uri());

    mocks::put_get::mount_gcs_download_result_no_sql_text(
        &sf_server,
        TEST_QUERY_ID,
        &stale_url,
        &local_location,
    )
    .await;

    // GCS always returns 400 — with no refresher there should be no retry.
    Mock::given(method("GET"))
        .and(path("/stale-file"))
        .respond_with(ResponseTemplate::new(400).set_body_string("ExpiredToken"))
        .mount(&gcs_server)
        .await;

    let sf_uri = sf_server.uri();
    tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&sf_uri));
        let result = client.connection_get_query_result(TEST_QUERY_ID);
        assert!(
            result.is_err(),
            "without sqlText, GCS 400 must be a hard failure (no refresh context)"
        );
    })
    .await
    .unwrap();
}

/// `connection_get_query_result` with `sqlText` present and GCS returning 400 first:
/// the `StageInfoRefreshContext` is built, so the 400 triggers a re-issue of the
/// original SQL → refreshed `presignedUrls[]` → the retry with the fresh URL succeeds.
#[tokio::test(flavor = "multi_thread")]
async fn connection_get_query_result_sql_text_present_builds_refresher_and_recovers() {
    let sf_server = MockServer::start().await;
    let gcs_server = MockServer::start().await;

    mocks::auth::mount_jwt_login_success(&sf_server).await;

    let tmp = tempfile::tempdir().expect("tempdir");
    let local_location = tmp.path().to_string_lossy().to_string();

    let stale_url = format!("{}/stale-file", gcs_server.uri());
    let fresh_url = format!("{}/fresh-file", gcs_server.uri());

    // Initial query result (from /queries/{id}/result) with sqlText present.
    mocks::put_get::mount_gcs_download_result_with_sql_text(
        &sf_server,
        TEST_QUERY_ID,
        &stale_url,
        &local_location,
    )
    .await;

    // Refresh SQL endpoint: returns fresh presigned URL.
    mocks::put_get::mount_gcs_download_refresh_sql_response(
        &sf_server,
        &fresh_url,
        &local_location,
    )
    .await;

    // GCS stale-file → 400 (triggers URL refresh).
    Mock::given(method("GET"))
        .and(path("/stale-file"))
        .respond_with(ResponseTemplate::new(400).set_body_string("ExpiredToken"))
        .mount(&gcs_server)
        .await;

    // GCS fresh-file → 200 (succeeds after refresh).
    Mock::given(method("GET"))
        .and(path("/fresh-file"))
        .respond_with(gcs_sse_response(b"downloaded-content"))
        .mount(&gcs_server)
        .await;

    let sf_uri = sf_server.uri();
    let local_dir = local_location.clone();
    tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&sf_uri));
        let result = client.connection_get_query_result(TEST_QUERY_ID);
        assert!(
            result.is_ok(),
            "with sqlText, GCS 400 should be recovered via stage-info refresh; got: {result:?}"
        );
        let downloaded = std::fs::read(std::path::Path::new(&local_dir).join("file.csv"))
            .expect("downloaded file should exist");
        assert_eq!(
            downloaded, b"downloaded-content",
            "downloaded content must match the fresh GCS response"
        );
    })
    .await
    .unwrap();
}

/// Regression guard: `connection_get_query_result` with `sqlText` present and GCS
/// succeeding immediately (no 400) — the refresh context is built but never invoked,
/// and the download completes normally. Ensures the sqlText path doesn't break the
/// happy-path.
#[tokio::test(flavor = "multi_thread")]
async fn connection_get_query_result_sql_text_present_happy_path() {
    let sf_server = MockServer::start().await;
    let gcs_server = MockServer::start().await;

    mocks::auth::mount_jwt_login_success(&sf_server).await;

    let tmp = tempfile::tempdir().expect("tempdir");
    let local_location = tmp.path().to_string_lossy().to_string();

    let presigned_url = format!("{}/file", gcs_server.uri());

    mocks::put_get::mount_gcs_download_result_with_sql_text(
        &sf_server,
        TEST_QUERY_ID,
        &presigned_url,
        &local_location,
    )
    .await;

    Mock::given(method("GET"))
        .and(path("/file"))
        .respond_with(gcs_sse_response(b"file-content"))
        .mount(&gcs_server)
        .await;

    let sf_uri = sf_server.uri();
    let local_dir = local_location.clone();
    tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&sf_uri));
        let result = client.connection_get_query_result(TEST_QUERY_ID);
        assert!(
            result.is_ok(),
            "sqlText present + GCS 200 → happy path must succeed; got: {result:?}"
        );
        let downloaded = std::fs::read(std::path::Path::new(&local_dir).join("file.csv"))
            .expect("downloaded file should exist");
        assert_eq!(downloaded, b"file-content");
    })
    .await
    .unwrap();
}
