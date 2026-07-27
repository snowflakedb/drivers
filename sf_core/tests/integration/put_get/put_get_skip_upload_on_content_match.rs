//! End-to-end FFI-seam test: drives a real `statement_set_options_blocking`
//! through the proto/FFI bridge and asserts observable HTTP behaviour
//! against a wiremock Azure server. Pins the whole pipe — Python kwarg →
//! `stmt.settings` → `perform_put_get_transfer` → Azure call.

use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::mocks;
use crate::common::put_get_common::PutResult;
use crate::common::snowflake_test_client::{SnowflakeTestClient, unwrap_single_rs_handle};
use sf_core::protobuf::generated::database_driver_v1::{ConfigSetting, ResultSetGetStreamResponse};
use std::io::Write;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mirror of `compute_sha256_digest` (crate-private) using openssl directly.
fn sha256_base64(data: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use openssl::hash::{MessageDigest, hash};
    let digest = hash(MessageDigest::sha256(), data).expect("sha256");
    STANDARD.encode(digest)
}

fn run_put_with_kwarg(
    client: &SnowflakeTestClient,
    sql: &str,
    skip_upload_on_content_match: bool,
) -> ResultSetGetStreamResponse {
    let stmt = client.new_statement();
    client.set_sql_query(&stmt, sql);
    client.set_statement_option(
        &stmt,
        "skip_upload_on_content_match",
        ConfigSetting::from(skip_upload_on_content_match),
    );
    let result = client.execute_statement_query(&stmt);
    let rs_handle = unwrap_single_rs_handle(&result);
    client.result_set_get_stream(&rs_handle)
}

fn write_tempfile(content: &[u8]) -> tempfile::NamedTempFile {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(tmp.path())
        .unwrap()
        .write_all(content)
        .unwrap();
    tmp
}

#[tokio::test(flavor = "multi_thread")]
async fn put_with_skip_match_and_matching_digest_skips_via_azure_head() {
    let gs_server = MockServer::start().await;
    let azure_server = MockServer::start().await;

    mocks::auth::mount_jwt_login_success(&gs_server).await;

    let payload = b"hello-azure-ffi-seam-match";
    let tmp = write_tempfile(payload);
    let src_path = tmp.path().to_str().unwrap().to_string();
    let real_digest = sha256_base64(payload);

    mocks::put_get::mount_azure_put_pointing_at(&gs_server, &azure_server.uri(), &src_path).await;

    Mock::given(method("HEAD"))
        .respond_with(
            ResponseTemplate::new(200).insert_header("x-ms-meta-sfcdigest", real_digest.as_str()),
        )
        .expect(1)
        .mount(&azure_server)
        .await;
    // Load-bearing: a wrong skip-decision sends a real PUT the mock rejects.
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(201))
        .expect(0)
        .mount(&azure_server)
        .await;

    let gs_uri = gs_server.uri();
    tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&gs_uri));
        let sql = format!("PUT 'file://{src_path}' @stage OVERWRITE=TRUE");
        let stream = run_put_with_kwarg(&client, &sql, /* skip_match */ true);

        let mut helper = ArrowResultHelper::from_result(stream);
        let row: PutResult = helper.fetch_one().expect("fetch PutResult");
        assert_eq!(row.status, "SKIPPED");
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn put_with_skip_match_and_mismatching_digest_uploads() {
    // Companion: pins that the FFI delivers the bool to the actual
    // comparison site, not to a constant SKIPPED return.
    let gs_server = MockServer::start().await;
    let azure_server = MockServer::start().await;

    mocks::auth::mount_jwt_login_success(&gs_server).await;

    let payload = b"hello-azure-ffi-seam-mismatch";
    let tmp = write_tempfile(payload);
    let src_path = tmp.path().to_str().unwrap().to_string();
    let other_digest = sha256_base64(b"definitely-not-the-local-content");

    mocks::put_get::mount_azure_put_pointing_at(&gs_server, &azure_server.uri(), &src_path).await;

    Mock::given(method("HEAD"))
        .respond_with(
            ResponseTemplate::new(200).insert_header("x-ms-meta-sfcdigest", other_digest.as_str()),
        )
        .expect(1)
        .mount(&azure_server)
        .await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&azure_server)
        .await;

    let gs_uri = gs_server.uri();
    tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&gs_uri));
        let sql = format!("PUT 'file://{src_path}' @stage OVERWRITE=TRUE");
        let stream = run_put_with_kwarg(&client, &sql, /* skip_match */ true);

        let mut helper = ArrowResultHelper::from_result(stream);
        let row: PutResult = helper.fetch_one().expect("fetch PutResult");
        assert_eq!(row.status, "UPLOADED");
    })
    .await
    .unwrap();
}
