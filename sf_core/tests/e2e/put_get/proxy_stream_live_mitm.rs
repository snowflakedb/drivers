//! Real streaming PUT+GET (`connection_upload_stream` / `connection_download_stream`,
//! JDBC `uploadStream`/`downloadStream`, Python `file_stream`) through a live
//! `mitmdump` proxy — the streaming-RPC counterpart to `proxy_live_mitm.rs`'s
//! file-path PUT/GET.
//!
//! Same interception proof: the connection trusts *only* mitmdump's CA
//! (`custom_root_store_path` replaces the built-in roots), so a byte-for-byte
//! round-trip is impossible unless the streaming transfer actually transited
//! mitmdump — which additionally proves the streaming path now threads the
//! connection's `tls_config` (custom root store) AND `proxy_config` onto the
//! stage, not just the file-path path.
//!
//! Cloud-agnostic by CI lane (`TEST_CLOUD_PROVIDER`): S3/Azure/GCS, no
//! per-cloud branching.
//!
//! `#[ignore]`d; requires `mitmdump` on `PATH` (skips visibly if absent). Run:
//!   cargo test -p sf_core --test e2e_tests --features protobuf \
//!     put_get::proxy_stream_live_mitm -- --ignored --nocapture

use crate::common::dead_proxy::assert_connect_fails_through_dead_proxy;
use crate::common::mitm_proxy::{MitmProxy, cloud_storage_host_suffix, mitmdump_available};
use crate::common::snowflake_test_client::SnowflakeTestClient;

const PAYLOAD: &[u8] = b"col1,col2\n1,2\n3,4\nproxy-stream-live-mitm-probe\n";
const FILENAME: &str = "proxy_stream_probe.csv";

#[test]
#[ignore = "live-cloud streaming PUT/GET through a real mitmdump proxy; run with --ignored. \
            Requires `mitmdump` on PATH (skips with a message if absent). \
            Hermetic counterpart: integration::http::proxy_stream_transfer"]
fn should_route_live_stream_put_get_through_mitmdump_proxy() {
    if !mitmdump_available() {
        // Visible skip, never a silent pass.
        eprintln!(
            "SKIPPED: mitmdump not found on PATH; install mitmproxy to run \
             put_get::proxy_stream_live_mitm. This is a real dependency, not an optional one."
        );
        return;
    }

    // Fresh proxy per test; no shared client to mask a bypass.
    let proxy = MitmProxy::start();

    // Per-connection options, not global env vars — other tests in this binary
    // are unaffected. Mirrors proxy_live_mitm.rs exactly.
    let client = SnowflakeTestClient::with_default_jwt_auth_params();
    client.set_connection_option("proxy_host", "127.0.0.1");
    client.set_connection_option_int("proxy_port", i64::from(proxy.port()));
    client.set_connection_option_bool("use_proxy_env", false);
    // Trust only mitmdump's CA — the proof-of-transit mechanism (see module doc).
    client.set_connection_option(
        "custom_root_store_path",
        proxy.ca_cert_path().to_str().expect("CA path is utf-8"),
    );
    // mitmdump leaf certs have no CRL/OCSP responder.
    client.set_connection_option("crl_check_mode", "DISABLED");

    client
        .connect()
        .expect("connect to live account through mitmdump");

    // When a payload is uploaded then downloaded via the streaming RPCs
    let stage_name = "TEST_STAGE_PROXY_STREAM_LIVE_MITM";
    client.create_temporary_stage(stage_name);
    client
        .connection_upload_stream(
            &format!("PUT file:///{FILENAME} @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE"),
            PAYLOAD.to_vec(),
        )
        .expect("streaming PUT through mitmdump must succeed");

    let downloaded = client
        .download_stream(&format!("@{stage_name}"), FILENAME, false)
        .expect("streaming GET through mitmdump must succeed");

    // Then the transfer round-tripped byte-for-byte — also the transit proof
    // (mitm-CA-only trust store, see module doc).
    assert_eq!(
        downloaded, PAYLOAD,
        "streamed download must match the uploaded payload byte-for-byte"
    );

    // Direct corroboration: mitmdump's request log shows a PUT and a GET for
    // this object on the storage host. Jointly load-bearing with the
    // CA-trust-only store, not independently (a stale retry line could match on
    // its own) — same reasoning as proxy_live_mitm.rs.
    let suffix = cloud_storage_host_suffix(client.current_cloud());
    let recorded = proxy.recorded_requests();
    let matched = |method: &str| {
        recorded
            .iter()
            .any(|r| r.method == method && r.host.ends_with(suffix) && r.path.contains(FILENAME))
    };
    assert!(
        matched("PUT"),
        "mitmdump recorded no streaming PUT of {FILENAME} to a {suffix} host: {recorded:?}"
    );
    assert!(
        matched("GET"),
        "mitmdump recorded no streaming GET of {FILENAME} from a {suffix} host: {recorded:?}"
    );
}

#[test]
#[ignore = "live-account negative control: streaming transfer through a dead proxy port must fail; \
            run with --ignored. Mirrors proxy_live_mitm's dead-proxy control."]
fn should_fail_live_stream_when_mitmdump_proxy_port_dead() {
    // Given otherwise-valid credentials — the only thing wrong is the proxy.
    // Login itself transits the proxy, so this fails at connect, before any
    // streaming transfer.
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
            // connect and reach auth instead. TODO(SNOW-3850381): connect()
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
