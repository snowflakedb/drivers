//! Real PUT+GET through a live `mitmdump` subprocess — an independent proxy
//! implementation, unlike the hermetic CONNECT harness this repo hand-rolled in
//! `integration/http/proxy_transfer.rs`.
//!
//! Interception proof: the connection trusts *only* mitmdump's CA
//! (`custom_root_store_path` replaces, not appends to, the built-in roots), so
//! a successful HTTPS PUT+GET is only possible if traffic actually transited
//! mitmdump — stronger than the legacy Python test, which just checks `UPLOADED`.
//!
//! Cloud-agnostic by CI lane (`TEST_CLOUD_PROVIDER`): exercises S3/Azure/GCS
//! with no per-cloud branching.
//!
//! `#[ignore]`d; requires `mitmdump` on `PATH` (skips visibly if absent). Run:
//!   cargo test -p sf_core --test e2e_tests --features protobuf \
//!     put_get::proxy_live_mitm -- --ignored --nocapture

use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::dead_proxy::assert_connect_fails_through_dead_proxy;
use crate::common::mitm_proxy::{MitmProxy, cloud_storage_host_suffix, mitmdump_available};
use crate::common::put_get_common::{
    PutResult, file_digest, get_file_from_stage, upload_to_stage_with_options,
};
use crate::common::snowflake_test_client::SnowflakeTestClient;

const PAYLOAD: &[u8] = b"col1,col2\n1,2\n3,4\nproxy-live-mitm-probe\n";
const FILENAME: &str = "proxy_probe.csv";

#[test]
#[ignore = "live-cloud PUT/GET through a real mitmdump proxy; run with --ignored. \
            Requires `mitmdump` on PATH (skips with a message if absent). \
            Hermetic counterpart: integration::http::proxy_transfer"]
fn should_route_live_put_get_through_mitmdump_proxy() {
    if !mitmdump_available() {
        // Visible skip, never a silent pass.
        eprintln!(
            "SKIPPED: mitmdump not found on PATH; install mitmproxy to run \
             put_get::proxy_live_mitm. This is a real dependency, not an optional one."
        );
        return;
    }

    // Fresh proxy per test; no shared client to mask a bypass.
    let proxy = MitmProxy::start();

    // Per-connection options, not global env vars/toggles — other tests in this
    // binary are unaffected.
    let client = SnowflakeTestClient::with_default_jwt_auth_params();
    client.set_connection_option("proxy_host", "127.0.0.1");
    client.set_connection_option_int("proxy_port", i64::from(proxy.port()));
    // Pin explicitly so an ambient HTTP_PROXY/HTTPS_PROXY can't change routing.
    client.set_connection_option_bool("use_proxy_env", false);
    // Trust only mitmdump's CA — the proof-of-transit mechanism (see module doc).
    client.set_connection_option(
        "custom_root_store_path",
        proxy.ca_cert_path().to_str().expect("CA path is utf-8"),
    );
    // Off because mitmdump's leaf certs have no CRL/OCSP responder (already the
    // default; explicit so this stays correct if that default ever changes).
    client.set_connection_option("crl_check_mode", "DISABLED");

    client
        .connect()
        .expect("connect to live account through mitmdump");

    // When a real file is PUT then GET through the proxied connection
    let stage_name = "TEST_STAGE_PROXY_LIVE_MITM";
    let src_dir = tempfile::tempdir().unwrap();
    let src_path = src_dir.path().join(FILENAME);
    std::fs::write(&src_path, PAYLOAD).unwrap();
    let src_digest = file_digest(&src_path);

    // AUTO_COMPRESS=FALSE keeps the stage object name == source name (no `.gz`);
    // OVERWRITE=TRUE makes reruns on the same stage idempotent.
    let put_data = upload_to_stage_with_options(
        &client,
        stage_name,
        src_path.to_str().unwrap(),
        "AUTO_COMPRESS=FALSE OVERWRITE=TRUE",
    );
    let put_result: PutResult = ArrowResultHelper::from_result(put_data)
        .fetch_one()
        .expect("fetch PUT result");
    assert_eq!(put_result.status, "UPLOADED", "PUT must report UPLOADED");

    let (_get, download_dir) = get_file_from_stage(&client, stage_name, FILENAME);
    let downloaded = download_dir.path().join(FILENAME);
    assert!(downloaded.exists(), "downloaded file should exist");

    // Then the transfer round-tripped byte-for-byte — also the transit proof
    // (mitm-CA-only trust store, see module doc).
    assert_eq!(
        file_digest(&downloaded),
        src_digest,
        "downloaded file must match the original byte-for-byte"
    );

    // Direct corroboration that the transfer transited the proxy: mitmdump's
    // request log shows a PUT and a GET for this object on the storage host.
    // This `.any()` match isn't tied to the request whose bytes produced the
    // digest above, so on a retry a stale matching line could satisfy it — that
    // gap is closed by the CA-trust-only store (see module doc), leaving the two
    // proofs jointly, not independently, load-bearing.
    let suffix = cloud_storage_host_suffix(client.current_cloud());
    let recorded = proxy.recorded_requests();
    let matched = |method: &str| {
        recorded
            .iter()
            .any(|r| r.method == method && r.host.ends_with(suffix) && r.path.contains(FILENAME))
    };
    assert!(
        matched("PUT"),
        "mitmdump recorded no PUT of {FILENAME} to a {suffix} host: {recorded:?}"
    );
    assert!(
        matched("GET"),
        "mitmdump recorded no GET of {FILENAME} from a {suffix} host: {recorded:?}"
    );
}

#[test]
#[ignore = "live-account negative control: connect through a dead proxy port must fail; \
            run with --ignored. Mirrors integration::http::proxy_transfer's dead-proxy controls."]
fn should_fail_live_connect_when_mitmdump_proxy_port_dead() {
    // Given otherwise-valid credentials — the only thing wrong is the proxy.
    // Fails at login, before any PUT/GET.
    assert_connect_fails_through_dead_proxy(
        |dead_port| {
            let client = SnowflakeTestClient::with_default_jwt_auth_params();
            client.set_connection_option("proxy_host", "127.0.0.1");
            client.set_connection_option_int("proxy_port", i64::from(dead_port));
            // When the connection is initialized through the dead proxy port
            client.connect()
        },
        |err| {
            // Then it must fail as a transport error — a bypassed proxy would
            // connect and succeed instead. TODO(SNOW-3850381): connect()
            // collapses this to a String; surface a typed transport error
            // instead.
            let lower = err.to_lowercase();
            assert!(
                !lower.contains("incorrect")
                    && !lower.contains("password")
                    && !lower.contains("jwt token is invalid"),
                "dead-proxy failure must be a transport error, not an auth rejection: {err}"
            );
        },
    );
}
