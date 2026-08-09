//! Hermetic proof that the connection-level streaming transfers
//! (`connection_upload_stream`, `connection_download_stream`, and the chunked
//! `download_stream_begin` S3 path) transit the driver's configured proxy.
//!
//! Unlike `proxy_transfer.rs`, which drives the `file_manager` transfer
//! functions with a hand-built `StageInfo`, these tests drive the full RPC
//! through a mock GS: the connection is proxy-configured, GS returns a stage
//! whose cloud endpoint is an unresolvable `.invalid` host, and the only route
//! to the backend is `client → ConnectProxy → TlsProxy → wiremock`. Each
//! positive test passes only on the conjunction of (a) the transfer succeeding
//! and (b) the proxy having recorded a `CONNECT` to the exact origin host.
//! Because the host is unresolvable and login is excluded from the proxy via
//! `no_proxy`, only the cloud transfer can reach the tunnel — a false pass via
//! direct dial is structurally impossible.
//!
//! The success half is doubly load-bearing here: the streaming paths must copy
//! BOTH the connection's `proxy_config` (routing) AND its `tls_config` (custom
//! root store) onto the stage, or the transfer fails — reverting either half
//! turns a pass into a failure. `proxy_transfer.rs` can't cover this because it
//! hand-builds the stage; only the RPC drives the copy.
//!
//! The same proof extends to the `SYSTEM$BIND` stage-binding upload path
//! (`stage_binding::upload_blob`): it builds its `StageInfo` outside
//! `perform_put_get_transfer` via a separate `StageTransport`, an independent
//! chance to regress. Tests send CSV bindings directly, bypassing a wrapper's
//! JSON-vs-CSV threshold decision.

use flate2::read::GzDecoder;
use serde_json::json;
use std::io::Read;
use wiremock::matchers::{body_string_contains, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::common::connect_proxy::ConnectProxy;
use crate::common::mocks;
use crate::common::private_key_helper::{self, PrivateKeyFile};
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::TlsProxy;

const S3_HOST: &str = "s3.snowflake-proxy-test.invalid";
const S3_BUCKET: &str = "test-bucket";
const GCS_HOST: &str = "storage.googleapis.snowflake-proxy-test.invalid";
const UPLOAD_PAYLOAD: &[u8] = b"proxy-stream-upload-payload";
// Injected by the tunnel's TLS-terminating hop only. Its presence at the
// backend is an independent proof-of-transit signal (separate from
// ConnectProxy's CONNECT log) and a structural tripwire: a future edit that
// accidentally drops the CONNECT/TLS-intercept hop would lose the marker and
// fail the positive tests.
const PROXY_MARKER: (&str, &str) = ("x-sf-proxy-tunnel-marker", "via-tls-proxy");

/// The `PROXY_MARKER` as owned strings for `TlsProxy::start_with_sans_and_marker`.
fn tunnel_marker() -> Option<(String, String)> {
    Some((PROXY_MARKER.0.to_string(), PROXY_MARKER.1.to_string()))
}

/// Asserts `req` (a wiremock-recorded request) carries the tunnel marker header.
fn assert_has_marker(req: &wiremock::Request) {
    assert_eq!(
        req.headers
            .get(PROXY_MARKER.0)
            .and_then(|v| v.to_str().ok()),
        Some(PROXY_MARKER.1),
        "request must carry the tunnel marker header (proof it transited the TLS-intercept hop)"
    );
}

/// Asserts `req` did NOT transit the marker-injecting tunnel hop.
fn assert_no_marker(req: &wiremock::Request) {
    assert!(
        req.headers.get(PROXY_MARKER.0).is_none(),
        "request must NOT carry the tunnel marker (it should have bypassed the proxy), got {:?}",
        req.headers.get(PROXY_MARKER.0)
    );
}

/// Mount a GS PUT response whose S3 `stageInfo` pins `endPoint` at `s3_endpoint`
/// (an `https://…invalid` origin), with no client-side encryption and
/// `OVERWRITE=TRUE` so the upload is a single, un-probed `PutObject`.
async fn mount_s3_put_pointing_at(server: &MockServer, s3_endpoint: &str) {
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .and(body_string_contains("PUT"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true,
                    "data": {
                        "command": "UPLOAD",
                        "stageInfo": {
                            "locationType": "S3",
                            "location": format!("{S3_BUCKET}/"),
                            "path": "",
                            "region": "us-east-1",
                            "endPoint": s3_endpoint,
                            "isClientSideEncrypted": false,
                            "creds": {
                                "AWS_KEY_ID": "AKIAIOSFODNN7EXAMPLE",
                                "AWS_SECRET_KEY": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
                                "AWS_TOKEN": ""
                            }
                        },
                        "src_locations": ["data.bin"],
                        "autoCompress": false,
                        "overwrite": true,
                        "sourceCompression": "NONE"
                    }
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(server)
        .await;
}

/// Mount a GS GET response whose GCS `stageInfo` carries a single presigned URL
/// at `presigned_url` (an `https://…invalid` origin). SSE (no CSE) so the body
/// is returned verbatim.
async fn mount_gcs_get_pointing_at(server: &MockServer, presigned_url: &str) {
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .and(body_string_contains("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true,
                    "data": {
                        "command": "DOWNLOAD",
                        "src_locations": ["file.csv"],
                        "stageInfo": {
                            "locationType": "GCS",
                            "location": "test-bucket/prefix/",
                            "path": "prefix/",
                            "region": "us-central1",
                            "creds": { "GCS_ACCESS_TOKEN": "test-bearer-token" },
                            "presignedUrl": null,
                            "endPoint": null
                        },
                        "localLocation": "/tmp",
                        "presignedUrls": [presigned_url]
                    }
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(server)
        .await;
}

/// Mount a GS GET response whose S3 `stageInfo` pins `endPoint` at
/// `s3_endpoint`, no client-side encryption. Drives the chunked S3-only
/// `download_stream_begin` path, which fetches via the S3 client (creds +
/// endpoint), not a presigned URL.
async fn mount_s3_get_pointing_at(server: &MockServer, s3_endpoint: &str) {
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .and(body_string_contains("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true,
                    "data": {
                        "command": "DOWNLOAD",
                        "src_locations": ["file.csv"],
                        "stageInfo": {
                            "locationType": "S3",
                            "location": format!("{S3_BUCKET}/"),
                            "path": "",
                            "region": "us-east-1",
                            "endPoint": s3_endpoint,
                            "isClientSideEncrypted": false,
                            "creds": {
                                "AWS_KEY_ID": "AKIAIOSFODNN7EXAMPLE",
                                "AWS_SECRET_KEY": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
                                "AWS_TOKEN": ""
                            }
                        },
                        "localLocation": "/tmp"
                    }
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(server)
        .await;
}

/// Mounts a trivial GS success response for query-requests whose `sqlText`
/// starts with `needle` — the plain (non-PUT/GET) queries in the bind-stage
/// flow: `CREATE TEMPORARY STAGE … SYSTEM$BIND` and the final bound statement.
///
/// Matches on the `sqlText` prefix with explicit `.with_priority(1)`, not a
/// bare body substring: the bound statement's injected
/// `TIMESTAMP_INPUT_FORMAT` parameter itself contains the substring `"PUT"`,
/// which would otherwise tie-break against `mount_s3_put_pointing_at`'s
/// `body_string_contains("PUT")` and route it to the wrong mock.
async fn mount_plain_query_success(server: &MockServer, needle: &'static str) {
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .and(body_string_contains(format!("\"sqlText\":\"{needle}")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true,
                    "data": {
                        "queryId": "bind-stage-plain-query",
                        "queryResultFormat": "json",
                        "rowset": [],
                        "rowtype": [],
                    }
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .with_priority(1)
        .mount(server)
        .await;
}

/// The default `CLIENT_STAGE_ARRAY_BINDING_THRESHOLD` cell count (255 * 256;
/// see `odbc/src/api/statement.rs::stage_binding_threshold`), so the test
/// payload is representative of a real stage-bind, not a single-row toy.
const BIND_STAGE_CSV_ROWS: usize = 65_280;

/// One quoted value per row, matching `odbc_bindings_to_csv`'s shape.
/// Uploaded verbatim and never parsed, so content just needs to look
/// realistic, not be semantically bindable.
fn build_bind_stage_csv() -> Vec<u8> {
    let mut csv = String::with_capacity(BIND_STAGE_CSV_ROWS * 8);
    for i in 0..BIND_STAGE_CSV_ROWS {
        csv.push_str(&format!("\"{i}\"\n"));
    }
    csv.into_bytes()
}

/// Gzip-decompresses `body` fully — the bind-stage upload always
/// auto-compresses, so the recorded body must be un-gzipped before comparing
/// it against the original CSV bytes.
fn gunzip(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    GzDecoder::new(body)
        .read_to_end(&mut out)
        .expect("recorded PUT body must be valid gzip");
    out
}

/// Strips the `aws-chunked` framing the S3 SDK wraps around a streamed
/// (file-backed) upload body sent with a trailing checksum — the bind-stage
/// upload's gzip output is spooled to a tempfile rather than kept in memory,
/// which triggers this framing. Only the single-chunk case is handled; a
/// multi-chunk body would fail loudly (bounds panic or a failed equality
/// assertion) rather than pass silently, so that's safe to leave unhandled.
fn strip_aws_chunked_framing(body: &[u8]) -> Vec<u8> {
    let header_end = body
        .windows(2)
        .position(|w| w == b"\r\n")
        .expect("aws-chunked body must start with a `{hex-length}\\r\\n` chunk header");
    let hex_len = std::str::from_utf8(&body[..header_end]).expect("chunk length must be ASCII");
    let chunk_len = usize::from_str_radix(hex_len, 16).expect("chunk length must be a hex number");
    let data_start = header_end + 2;
    body[data_start..data_start + chunk_len].to_vec()
}

/// Writes the TlsProxy cert to a temp file and returns (dir, path-string). The
/// dir must be kept alive until the transfer completes.
fn write_ca(tls_proxy: &TlsProxy) -> (tempfile::TempDir, String) {
    let dir = tempfile::TempDir::new().unwrap();
    let cert_path = dir.path().join("proxy-ca.pem");
    std::fs::write(&cert_path, tls_proxy.cert_pem()).expect("write proxy cert");
    let s = cert_path.to_str().unwrap().to_string();
    (dir, s)
}

/// Builds an int-test JWT client pointed at `gs_uri` and routed through the
/// proxy on `proxy_port`, excluding loopback (GS) from the proxy via `no_proxy`.
/// `ca_path` (the TlsProxy cert) is the sole trust anchor when present; omit it
/// for dead-proxy controls that never reach a TLS handshake. Returns the client
/// plus the key file, which the caller must keep alive for the transfer.
fn proxied_jwt_client(
    gs_uri: &str,
    proxy_port: u16,
    ca_path: Option<&str>,
) -> (SnowflakeTestClient, PrivateKeyFile) {
    let key_file = private_key_helper::get_test_private_key_file().expect("test key");
    let client = SnowflakeTestClient::with_int_tests_params(Some(gs_uri));
    client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
    client.set_connection_option("private_key_file", key_file.path().to_str().unwrap());
    client.set_connection_option("proxy_host", "127.0.0.1");
    client.set_connection_option_int("proxy_port", i64::from(proxy_port));
    // Pin routing so an ambient HTTP(S)_PROXY can't change it, and keep the mock
    // GS (loopback, plain HTTP) off the proxy so only the cloud transfer tunnels.
    client.set_connection_option_bool("use_proxy_env", false);
    client.set_connection_option("no_proxy", "127.0.0.1,localhost");
    if let Some(ca) = ca_path {
        client.set_connection_option("custom_root_store_path", ca);
    }
    // The TlsProxy leaf cert has no CRL/OCSP responder.
    client.set_connection_option("crl_check_mode", "DISABLED");
    (client, key_file)
}

/// A loopback port with nothing listening (bind then drop) for the dead-proxy
/// controls.
async fn dead_loopback_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

#[tokio::test(flavor = "multi_thread")]
async fn should_route_s3_upload_stream_through_proxy() {
    let gs = MockServer::start().await;
    let cloud = MockServer::start().await;
    let tls_proxy = TlsProxy::start_with_sans_and_marker(
        *cloud.address(),
        vec![S3_HOST.to_string(), format!("{S3_BUCKET}.{S3_HOST}")],
        tunnel_marker(),
    )
    .await;
    let connect_proxy = ConnectProxy::start(tls_proxy.addr()).await;

    mocks::auth::mount_jwt_login_success(&gs).await;
    mount_s3_put_pointing_at(&gs, &format!("https://{S3_HOST}")).await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"mock-etag\""))
        .mount(&cloud)
        .await;

    let (_ca_dir, ca_path) = write_ca(&tls_proxy);
    let gs_uri = gs.uri();
    let connect_port = connect_proxy.port();

    let result = tokio::task::spawn_blocking(move || {
        let (client, _key) = proxied_jwt_client(&gs_uri, connect_port, Some(&ca_path));
        client.connect().expect("connect via mock GS through proxy");
        client.connection_upload_stream(
            "PUT file:///tmp/data.bin @mock_stage AUTO_COMPRESS=FALSE OVERWRITE=TRUE",
            UPLOAD_PAYLOAD.to_vec(),
        )
    })
    .await
    .unwrap();

    result.expect("upload_stream via proxy should succeed");
    assert_eq!(
        connect_proxy.observed_connects(),
        vec![format!("{S3_BUCKET}.{S3_HOST}:443")],
    );
    // Backend-side correctness: the mock accepts any PUT, so assert on the
    // captured request that the proxied upload hit the exact S3 key and carried
    // the uploaded bytes — not merely that "some PUT succeeded".
    let recorded = cloud.received_requests().await.unwrap_or_default();
    let puts: Vec<_> = recorded
        .iter()
        .filter(|r| r.method.as_str() == "PUT")
        .collect();
    assert_eq!(
        puts.len(),
        1,
        "expected exactly one backend PUT, saw: {recorded:?}"
    );
    assert_eq!(
        puts[0].url.path(),
        "/data.bin",
        "PUT must target the S3 key"
    );
    // Unencrypted, uncompressed upload — the backend body must equal the
    // payload exactly (exact-eq also catches leading/trailing corruption a
    // substring check would miss).
    assert_eq!(
        puts[0].body.as_slice(),
        UPLOAD_PAYLOAD,
        "recorded PUT body must equal the uploaded payload exactly"
    );
    assert_has_marker(puts[0]);
}

#[tokio::test(flavor = "multi_thread")]
async fn should_route_download_stream_through_proxy() {
    let gs = MockServer::start().await;
    let cloud = MockServer::start().await;
    let tls_proxy = TlsProxy::start_with_sans_and_marker(
        *cloud.address(),
        vec![GCS_HOST.to_string()],
        tunnel_marker(),
    )
    .await;
    let connect_proxy = ConnectProxy::start(tls_proxy.addr()).await;

    mocks::auth::mount_jwt_login_success(&gs).await;
    mount_gcs_get_pointing_at(&gs, &format!("https://{GCS_HOST}/file")).await;
    Mock::given(method("GET"))
        .and(path("/file"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"streamed-download-via-proxy".to_vec())
                .insert_header("x-goog-meta-sfc-digest", "test-digest"),
        )
        .mount(&cloud)
        .await;

    let (_ca_dir, ca_path) = write_ca(&tls_proxy);
    let gs_uri = gs.uri();
    let connect_port = connect_proxy.port();

    let bytes = tokio::task::spawn_blocking(move || {
        let (client, _key) = proxied_jwt_client(&gs_uri, connect_port, Some(&ca_path));
        client.connect().expect("connect via mock GS through proxy");
        client.download_stream("@mock_stage", "file.csv", false)
    })
    .await
    .unwrap();

    assert_eq!(
        bytes.expect("download_stream via proxy should succeed"),
        b"streamed-download-via-proxy",
    );
    assert_eq!(
        connect_proxy.observed_connects(),
        vec![format!("{GCS_HOST}:443")],
    );
    let recorded = cloud.received_requests().await.unwrap_or_default();
    let gets: Vec<_> = recorded
        .iter()
        .filter(|r| r.method.as_str() == "GET")
        .collect();
    assert_eq!(
        gets.len(),
        1,
        "expected exactly one backend GET, saw: {recorded:?}"
    );
    assert_has_marker(gets[0]);
}

#[tokio::test(flavor = "multi_thread")]
async fn should_route_download_stream_begin_through_proxy() {
    let gs = MockServer::start().await;
    let cloud = MockServer::start().await;
    let tls_proxy = TlsProxy::start_with_sans_and_marker(
        *cloud.address(),
        vec![S3_HOST.to_string(), format!("{S3_BUCKET}.{S3_HOST}")],
        tunnel_marker(),
    )
    .await;
    let connect_proxy = ConnectProxy::start(tls_proxy.addr()).await;

    mocks::auth::mount_jwt_login_success(&gs).await;
    mount_s3_get_pointing_at(&gs, &format!("https://{S3_HOST}")).await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(b"chunked-s3-download-via-proxy".to_vec()),
        )
        .mount(&cloud)
        .await;

    let (_ca_dir, ca_path) = write_ca(&tls_proxy);
    let gs_uri = gs.uri();
    let connect_port = connect_proxy.port();

    let bytes = tokio::task::spawn_blocking(move || {
        let (client, _key) = proxied_jwt_client(&gs_uri, connect_port, Some(&ca_path));
        client.connect().expect("connect via mock GS through proxy");
        client.connection_download_stream_chunked("@mock_stage", "file.csv", false)
    })
    .await
    .unwrap();

    assert_eq!(
        bytes.expect("download_stream_begin via proxy should succeed"),
        b"chunked-s3-download-via-proxy",
    );
    assert_eq!(
        connect_proxy.observed_connects(),
        vec![format!("{S3_BUCKET}.{S3_HOST}:443")],
    );
    // The mock accepts any GET, so assert on the captured request that the
    // chunked download fetched the exact S3 key (the returned bytes alone would
    // pass even against a wrong key served by the loose mock).
    let recorded = cloud.received_requests().await.unwrap_or_default();
    let gets: Vec<_> = recorded
        .iter()
        .filter(|r| r.method.as_str() == "GET")
        .collect();
    assert_eq!(
        gets.len(),
        1,
        "expected exactly one backend GET, saw: {recorded:?}"
    );
    assert_eq!(
        gets[0].url.path(),
        "/file.csv",
        "GET must target the S3 key"
    );
    assert_has_marker(gets[0]);
}

#[tokio::test(flavor = "multi_thread")]
async fn should_fail_download_stream_begin_when_proxy_port_dead() {
    let gs = MockServer::start().await;
    mocks::auth::mount_jwt_login_success(&gs).await;
    mount_s3_get_pointing_at(&gs, &format!("https://{S3_HOST}")).await;

    let gs_uri = gs.uri();
    let dead_port = dead_loopback_port().await;

    let result = tokio::task::spawn_blocking(move || {
        let (client, _key) = proxied_jwt_client(&gs_uri, dead_port, None);
        client
            .connect()
            .expect("login bypasses the dead proxy via no_proxy");
        client.connection_download_stream_chunked("@mock_stage", "file.csv", false)
    })
    .await
    .unwrap();

    // The chunked path fails at stream-open (the GetObject) through the dead
    // proxy, before any chunk is served.
    result.expect_err("download_stream_begin through a dead proxy port must fail");
}

#[tokio::test(flavor = "multi_thread")]
async fn should_fail_upload_stream_when_proxy_port_dead() {
    let gs = MockServer::start().await;
    mocks::auth::mount_jwt_login_success(&gs).await;
    mount_s3_put_pointing_at(&gs, &format!("https://{S3_HOST}")).await;

    let gs_uri = gs.uri();
    let dead_port = dead_loopback_port().await;

    let result = tokio::task::spawn_blocking(move || {
        // No trust anchor needed: a dead proxy fails at connect, before TLS.
        let (client, _key) = proxied_jwt_client(&gs_uri, dead_port, None);
        client
            .connect()
            .expect("login bypasses the dead proxy via no_proxy");
        client.connection_upload_stream(
            "PUT file:///tmp/data.bin @mock_stage AUTO_COMPRESS=FALSE OVERWRITE=TRUE",
            b"proxy-stream-upload-payload".to_vec(),
        )
    })
    .await
    .unwrap();

    // Login succeeds (loopback bypasses the proxy); only the cloud upload, which
    // must transit the dead proxy, fails. A bypassed proxy would instead dial the
    // unresolvable host and also fail — the positive test is what distinguishes
    // the two.
    result.expect_err("upload_stream through a dead proxy port must fail");
}

#[tokio::test(flavor = "multi_thread")]
async fn should_bypass_proxy_for_download_stream_no_proxy_host() {
    // A reachable host on `no_proxy` must be dialed directly, never tunneled.
    // Two independent TLS-terminating hops front the same backend: the proxied
    // route (via ConnectProxy) injects the marker; the direct route does not. A
    // correct bypass hits the direct hop — success, zero CONNECTs, no marker. A
    // broken bypass would instead tunnel to the marker hop, tripping BOTH the
    // CONNECT log and the marker (the two proofs are independent).
    let gs = MockServer::start().await;
    let cloud = MockServer::start().await;
    let sans = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    // Would-be tunnel hop (marker-injecting), reachable only via ConnectProxy.
    let marker_tls =
        TlsProxy::start_with_sans_and_marker(*cloud.address(), sans.clone(), tunnel_marker()).await;
    let connect_proxy = ConnectProxy::start(marker_tls.addr()).await;
    // Direct hop (no marker), dialed straight when no_proxy applies.
    let direct_tls = TlsProxy::start_with_sans(*cloud.address(), sans).await;

    mocks::auth::mount_jwt_login_success(&gs).await;
    // Presigned URL on a directly-dialable loopback host (a hostname, not an IP,
    // so the self-signed DNS-SAN cert validates), which `no_proxy` excludes.
    let presigned = format!("https://localhost:{}/file", direct_tls.addr().port());
    mount_gcs_get_pointing_at(&gs, &presigned).await;
    Mock::given(method("GET"))
        .and(path("/file"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"bypassed-download".to_vec())
                .insert_header("x-goog-meta-sfc-digest", "test-digest"),
        )
        .mount(&cloud)
        .await;

    // Trust both hops so a hypothetical broken-bypass regression fails on the
    // marker/CONNECT assertions, not on a TLS error.
    let ca_dir = tempfile::TempDir::new().unwrap();
    let ca_path = ca_dir.path().join("proxy-ca.pem");
    std::fs::write(
        &ca_path,
        format!("{}\n{}", direct_tls.cert_pem(), marker_tls.cert_pem()),
    )
    .unwrap();
    let ca_str = ca_path.to_str().unwrap().to_string();
    let gs_uri = gs.uri();
    let connect_port = connect_proxy.port();

    let bytes = tokio::task::spawn_blocking(move || {
        // proxied_jwt_client sets no_proxy=127.0.0.1,localhost, so `localhost`
        // (the cloud host here) is excluded from the proxy.
        let (client, _key) = proxied_jwt_client(&gs_uri, connect_port, Some(&ca_str));
        client.connect().expect("connect via mock GS");
        client.download_stream("@mock_stage", "file.csv", false)
    })
    .await
    .unwrap();

    // (a) the transfer succeeded...
    assert_eq!(
        bytes.expect("download via a bypassed proxy should succeed"),
        b"bypassed-download",
    );
    // (b) ...the proxy recorded no CONNECT for the no_proxy host...
    assert!(
        connect_proxy.observed_connects().is_empty(),
        "no_proxy host must never reach the proxy, saw: {:?}",
        connect_proxy.observed_connects(),
    );
    // (c) ...and independently, the backend request never crossed the marker hop.
    let recorded = cloud.received_requests().await.unwrap_or_default();
    let gets: Vec<_> = recorded
        .iter()
        .filter(|r| r.method.as_str() == "GET")
        .collect();
    assert_eq!(
        gets.len(),
        1,
        "expected exactly one backend GET, saw: {recorded:?}"
    );
    assert_no_marker(gets[0]);
}

#[tokio::test(flavor = "multi_thread")]
async fn should_fail_download_stream_when_proxy_port_dead() {
    let gs = MockServer::start().await;
    mocks::auth::mount_jwt_login_success(&gs).await;
    mount_gcs_get_pointing_at(&gs, &format!("https://{GCS_HOST}/file")).await;

    let gs_uri = gs.uri();
    let dead_port = dead_loopback_port().await;

    let result = tokio::task::spawn_blocking(move || {
        let (client, _key) = proxied_jwt_client(&gs_uri, dead_port, None);
        client
            .connect()
            .expect("login bypasses the dead proxy via no_proxy");
        client.download_stream("@mock_stage", "file.csv", false)
    })
    .await
    .unwrap();

    result.expect_err("download_stream through a dead proxy port must fail");
}

#[tokio::test(flavor = "multi_thread")]
async fn should_route_bind_stage_csv_upload_through_proxy() {
    let gs = MockServer::start().await;
    let cloud = MockServer::start().await;
    let tls_proxy = TlsProxy::start_with_sans_and_marker(
        *cloud.address(),
        vec![S3_HOST.to_string(), format!("{S3_BUCKET}.{S3_HOST}")],
        tunnel_marker(),
    )
    .await;
    let connect_proxy = ConnectProxy::start(tls_proxy.addr()).await;

    mocks::auth::mount_jwt_login_success(&gs).await;
    // Three GS query-requests in sequence: CREATE TEMPORARY STAGE, the PUT,
    // then the bound statement. `mount_plain_query_success` disambiguates the
    // first and third from the PUT mock (see its doc comment for why).
    mount_plain_query_success(&gs, "CREATE TEMPORARY STAGE").await;
    mount_s3_put_pointing_at(&gs, &format!("https://{S3_HOST}")).await;
    mount_plain_query_success(&gs, "INSERT INTO").await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"mock-etag\""))
        .mount(&cloud)
        .await;

    let (_ca_dir, ca_path) = write_ca(&tls_proxy);
    let gs_uri = gs.uri();
    let connect_port = connect_proxy.port();
    let csv_bindings = build_bind_stage_csv();

    let result = tokio::task::spawn_blocking(move || {
        let (client, _key) = proxied_jwt_client(&gs_uri, connect_port, Some(&ca_path));
        client.connect().expect("connect via mock GS through proxy");
        let stmt = client.new_statement();
        client.set_sql_query(&stmt, "INSERT INTO bind_stage_test_table (a) VALUES (?)");
        client.execute_statement_query_with_csv_bindings_no_unwrap(&stmt, &csv_bindings)
    })
    .await
    .unwrap();

    result.expect("bind-stage CSV upload via proxy should succeed");
    assert_eq!(
        connect_proxy.observed_connects(),
        vec![format!("{S3_BUCKET}.{S3_HOST}:443")],
    );
    // Backend-side correctness: the mock accepts any PUT, so assert on the
    // captured request that the proxied bind-stage upload carried the exact
    // CSV bytes (after undoing the upload's own gzip auto-compression) — not
    // merely that "some PUT succeeded".
    let recorded = cloud.received_requests().await.unwrap_or_default();
    let puts: Vec<_> = recorded
        .iter()
        .filter(|r| r.method.as_str() == "PUT")
        .collect();
    assert_eq!(
        puts.len(),
        1,
        "expected exactly one backend PUT, saw: {recorded:?}"
    );
    // Bind-stage upload always names its source "0" and auto-compresses it,
    // so the S3 key is "0.gz" (unlike the `AUTO_COMPRESS=FALSE` test above).
    assert_eq!(puts[0].url.path(), "/0.gz", "PUT must target the S3 key");
    assert_eq!(
        gunzip(&strip_aws_chunked_framing(&puts[0].body)),
        build_bind_stage_csv(),
        "recorded PUT body must de-chunk and gunzip back to the uploaded CSV bindings exactly"
    );
    assert_has_marker(puts[0]);
}

#[tokio::test(flavor = "multi_thread")]
async fn should_fail_bind_stage_csv_upload_when_proxy_port_dead() {
    let gs = MockServer::start().await;
    mocks::auth::mount_jwt_login_success(&gs).await;
    mount_plain_query_success(&gs, "CREATE TEMPORARY STAGE").await;
    mount_s3_put_pointing_at(&gs, &format!("https://{S3_HOST}")).await;

    let gs_uri = gs.uri();
    let dead_port = dead_loopback_port().await;
    let csv_bindings = build_bind_stage_csv();

    let result = tokio::task::spawn_blocking(move || {
        // No trust anchor needed: a dead proxy fails at connect, before TLS.
        let (client, _key) = proxied_jwt_client(&gs_uri, dead_port, None);
        client
            .connect()
            .expect("login bypasses the dead proxy via no_proxy");
        let stmt = client.new_statement();
        client.set_sql_query(&stmt, "INSERT INTO bind_stage_test_table (a) VALUES (?)");
        client.execute_statement_query_with_csv_bindings_no_unwrap(&stmt, &csv_bindings)
    })
    .await
    .unwrap();

    // Login and CREATE STAGE/PUT-query bypass the dead proxy (loopback,
    // excluded via no_proxy); only the cloud upload transits it and fails.
    result.expect_err("bind-stage CSV upload through a dead proxy port must fail");
}
