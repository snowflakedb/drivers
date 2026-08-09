//! Real-account multipart round-trip through a live `mitmdump` proxy: extends the
//! transit proof in `proxy_live_mitm.rs` (a 41-byte single request) to the
//! multipart-upload and ranged-GET-download paths that large transfers use.
//!
//! Same proof mechanism as `proxy_live_mitm.rs`: the connection trusts *only*
//! mitmdump's CA (`custom_root_store_path` replaces the built-in roots), so a
//! correct byte-for-byte round-trip is impossible unless every part/range
//! transited mitmdump. All three clouds build one proxy-configured client and
//! share it across every part/range (S3 `create_s3_client`, Azure
//! `create_azure_client`, GCS `create_gcs_client`), so one cloud-agnostic test
//! (`TEST_CLOUD_PROVIDER`) covers S3 on `aws`, Azure block-blob on `azure`, and
//! GCS resumable on `gcp` with no per-cloud branching.
//!
//! The >200 MiB file (mirrors `put_get_multipart_roundtrip.rs`) forces multipart
//! on upload and parallel ranged GETs on download. Scoped to the nightly `large_`
//! lane like `put_get_multipart_roundtrip.rs`, NOT the every-run `mitmdump_proxy`
//! lane: a 210 MiB transfer through mitmdump is far costlier than the 41-byte
//! test. Test names carry `large_` and omit `mitmdump_proxy` so the CI filters
//! route them to the nightly lane only.
//!
//! `#[ignore]`d; the round-trip needs `mitmdump` on PATH (skips visibly if absent):
//!   cargo test -p sf_core --test e2e_tests --features protobuf \
//!     put_get::put_get_multipart_proxy_live_mitm -- --ignored --nocapture

use crate::common::dead_proxy::assert_connect_fails_through_dead_proxy;
use crate::common::mitm_proxy::{MitmProxy, cloud_storage_host_suffix, mitmdump_available};
use crate::common::put_get_common::{
    MULTIPART_FILE_LEN, file_digest, get_file_from_stage, upload_to_stage_with_options,
    write_payload,
};
use crate::common::snowflake_test_client::{CloudProvider, SnowflakeTestClient};
use sf_core::file_manager::internal::MultipartConfig;

const FILENAME: &str = "bigfile_proxy.bin";

#[test]
#[ignore = "~210 MiB real-cloud multipart round-trip through a live mitmdump proxy; \
            nightly `large_` lane, run with --ignored. Requires `mitmdump` on PATH \
            (skips with a message if absent). Single-request counterpart: put_get::proxy_live_mitm"]
fn should_route_large_multipart_put_get_through_live_mitm_proxy() {
    if !mitmdump_available() {
        // Visible skip, never a silent pass.
        eprintln!(
            "SKIPPED: mitmdump not found on PATH; install mitmproxy to run \
             put_get::put_get_multipart_proxy_live_mitm. This is a real dependency, not optional."
        );
        return;
    }

    // Fresh proxy per test; the mitm-CA-only trust store is the transit proof.
    let proxy = MitmProxy::start();

    // Per-connection proxy options (not env vars), mirroring proxy_live_mitm so no
    // other test in this binary is affected.
    let client = SnowflakeTestClient::with_default_jwt_auth_params();
    client.set_connection_option("proxy_host", "127.0.0.1");
    client.set_connection_option_int("proxy_port", i64::from(proxy.port()));
    client.set_connection_option_bool("use_proxy_env", false);
    // Trust only mitmdump's CA — success is impossible unless traffic transits it.
    client.set_connection_option(
        "custom_root_store_path",
        proxy.ca_cert_path().to_str().expect("CA path is utf-8"),
    );
    // mitmdump leaf certs have no CRL/OCSP responder.
    client.set_connection_option("crl_check_mode", "DISABLED");

    client
        .connect()
        .expect("connect to live account through mitmdump");

    // Distinct stage/name from put_get_multipart_roundtrip so the two `large_` tests
    // don't collide when cargo runs them in parallel against the same account.
    let stage_name = "TEST_STAGE_MULTIPART_PROXY_LIVE_MITM";
    let src_dir = tempfile::tempdir().unwrap();
    let src_path = src_dir.path().join(FILENAME);
    write_payload(&src_path, MULTIPART_FILE_LEN);
    let src_digest = file_digest(&src_path);

    // When a >200 MiB file is PUT (multipart) then GET (ranged) through the proxy.
    // AUTO_COMPRESS=FALSE keeps the on-cloud size above the threshold; OVERWRITE=TRUE
    // makes reruns idempotent.
    upload_to_stage_with_options(
        &client,
        stage_name,
        src_path.to_str().unwrap(),
        "AUTO_COMPRESS=FALSE OVERWRITE=TRUE",
    );

    let (_get, download_dir) = get_file_from_stage(&client, stage_name, FILENAME);
    let downloaded = download_dir.path().join(FILENAME);
    assert!(downloaded.exists(), "downloaded file should exist");

    // Then the file round-trips byte-for-byte — also the transit proof: with the
    // mitm-CA-only trust store, every part/range that produced this digest
    // transited mitmdump.
    assert_eq!(
        file_digest(&downloaded),
        src_digest,
        "downloaded file must match the original byte-for-byte"
    );

    // And independently: mitmdump must have recorded at least one storage
    // request per upload part and per download range — proving every chunk, not
    // just some traffic, transited the proxy. The expected count comes from the
    // real per-cloud part-size formula, not a mirrored constant.
    let cloud = client.current_cloud();
    let suffix = cloud_storage_host_suffix(cloud);
    let cfg = match cloud {
        CloudProvider::Aws => MultipartConfig::S3,
        CloudProvider::Azure => MultipartConfig::AZURE,
        CloudProvider::Gcp => MultipartConfig::GCS,
    };
    // Coupled to the *client-side* part-size threshold (`cfg`'s default/grown
    // part size), not the server's actual multipart/ranged-GET threshold: if
    // the server's 200 MiB threshold is ever raised above `MULTIPART_FILE_LEN`,
    // the transfer could legitimately go single-shot and the `>=` assertions
    // below could fail for reasons unrelated to a real regression.
    let expected_parts = cfg
        .expected_part_count(MULTIPART_FILE_LEN)
        .expect("part size within cloud limits");
    assert!(
        expected_parts >= 2,
        "a 210 MiB file must split into >= 2 parts"
    );

    let recorded = proxy.recorded_requests();
    let to_storage = |method: &str| {
        recorded
            .iter()
            .filter(|r| r.method == method && r.host.ends_with(suffix))
            .count() as u64
    };
    let (uploads, downloads) = (to_storage("PUT"), to_storage("GET"));
    assert!(
        uploads >= expected_parts,
        "multipart upload: mitmdump saw {uploads} PUT(s) to a {suffix} host, expected >= {expected_parts}"
    );
    assert!(
        downloads >= expected_parts,
        "ranged download: mitmdump saw {downloads} GET(s) from a {suffix} host, expected >= {expected_parts}"
    );
}

#[test]
#[ignore = "live-account negative control for the multipart lane: connect through a dead \
            proxy port must fail; nightly `large_` lane, run with --ignored. \
            Mirrors put_get::proxy_live_mitm's dead-proxy control."]
fn should_fail_large_transfer_when_live_mitm_proxy_port_dead() {
    // Otherwise-valid creds; only the proxy is wrong. Login itself transits
    // the proxy, so this fails at connect before any transfer — the
    // dead-proxy control cannot be silently bypassed regardless of file size.
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
