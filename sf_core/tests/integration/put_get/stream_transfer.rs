//! Integration tests for the streaming GET handler (`connection_download_stream`),
//! which backs JDBC `downloadStream`.
//!
//! Unlike `connection_get_query_result` (which downloads to a server-supplied
//! `localLocation`), `connection_download_stream` synthesizes its own
//! `GET <stage>/<file> file://<tempdir>` SQL, runs it synchronously through GS
//! (`POST /queries/v1/query-request`), downloads the single file into a private
//! tempdir, reads it back, and returns the bytes — optionally gunzipping.
//!
//! The synchronous GET POST is matched by `mount_gcs_download_refresh_sql_response`
//! (any query body containing "GET"); the handler overrides the response's
//! `localLocation` with its own tempdir, so the file always lands where it reads
//! it back from.

use crate::common::mocks;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::io::Write as _;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// GCS SSE response (no encryption metadata): body returned verbatim.
fn gcs_sse_response(body: Vec<u8>) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_bytes(body)
        .insert_header("x-goog-meta-sfc-digest", "test-digest")
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(bytes).expect("gzip write");
    encoder.finish().expect("gzip finish")
}

/// Plain (non-decompress) GET: the on-stage bytes are returned verbatim.
#[tokio::test(flavor = "multi_thread")]
async fn download_stream_returns_file_bytes() {
    let sf_server = MockServer::start().await;
    let gcs_server = MockServer::start().await;

    mocks::auth::mount_jwt_login_success(&sf_server).await;

    let presigned_url = format!("{}/file", gcs_server.uri());
    // `localLocation` here is irrelevant — the handler overrides it with its
    // own tempdir — but the mock requires a value.
    mocks::put_get::mount_gcs_download_refresh_sql_response(&sf_server, &presigned_url, "/tmp")
        .await;

    Mock::given(method("GET"))
        .and(path("/file"))
        .respond_with(gcs_sse_response(b"streamed-download-content".to_vec()))
        .mount(&gcs_server)
        .await;

    let sf_uri = sf_server.uri();
    tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&sf_uri));
        let bytes = client
            .connection_download_stream("@mock_stage", "file.csv", false)
            .expect("download_stream must succeed on GCS 200");
        assert_eq!(bytes, b"streamed-download-content");
    })
    .await
    .unwrap();
}

/// `decompress = true` gunzips the on-stage bytes before returning them.
#[tokio::test(flavor = "multi_thread")]
async fn download_stream_decompresses_when_requested() {
    let sf_server = MockServer::start().await;
    let gcs_server = MockServer::start().await;

    mocks::auth::mount_jwt_login_success(&sf_server).await;

    let plaintext = b"the quick brown fox jumps over the lazy dog";
    let gzipped = gzip(plaintext);

    let presigned_url = format!("{}/file", gcs_server.uri());
    mocks::put_get::mount_gcs_download_refresh_sql_response(&sf_server, &presigned_url, "/tmp")
        .await;

    Mock::given(method("GET"))
        .and(path("/file"))
        .respond_with(gcs_sse_response(gzipped))
        .mount(&gcs_server)
        .await;

    let sf_uri = sf_server.uri();
    tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&sf_uri));
        let bytes = client
            .connection_download_stream("@mock_stage", "file.csv", true)
            .expect("download_stream with decompress must succeed");
        assert_eq!(
            bytes, b"the quick brown fox jumps over the lazy dog",
            "decompress=true must return the gunzipped plaintext"
        );
    })
    .await
    .unwrap();
}
