//! Real-account multipart round-trip: a file larger than the server's 200 MiB
//! PUT/GET threshold, so the upload uses cloud multipart and the download uses
//! parallel ranged GETs — the genuinely end-to-end counterpart to the fast,
//! deterministic wiremock coverage in `integration/http/s3_multipart.rs`,
//! `integration/http/azure_multipart.rs`, and `integration/http/gcs_multipart.rs`.
//!
//! Cloud-agnostic by design, not by branching: `connect_with_default_auth`
//! connects to whichever account each CI `cloud_provider` matrix lane decoded
//! (see `scripts/decode_secrets.sh` — a distinct dedicated account per cloud),
//! so this single test exercises S3 multipart on the `aws` lane, Azure
//! block-blob multipart on the `azure` lane, and GCS resumable multipart on
//! the `gcp` lane without any per-cloud test code. CI scopes it to all three
//! lanes (nightly "Run long-running tests" step).
//!
//! Gated `#[ignore]`: it generates and round-trips ~210 MiB over the network, so
//! it does not run in the normal `e2e` lane. Run it explicitly with:
//!   cargo test -p sf_core --test e2e_tests --features protobuf \
//!     put_get::put_get_multipart_roundtrip::should_upload_and_download_large_file_via_multipart_roundtrip -- --ignored --nocapture
//!
//! The size is forced by the server: the reference account returns a 200 MiB
//! PUT threshold and no GET threshold (the driver falls back to 200 MiB), and
//! the SQL `THRESHOLD=` option is rejected — so a smaller file cannot exercise
//! the multipart paths through the real PUT/GET command path.

use crate::common::put_get_common::{
    MULTIPART_FILE_LEN, file_digest, get_file_from_stage, upload_to_stage_with_options,
    write_payload,
};
use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
#[ignore = "~210 MiB real-cloud multipart round-trip; belongs to the `large_` CI category, run with --ignored. Fast coverage: integration::http::s3_multipart, integration::http::azure_multipart, integration::http::gcs_multipart"]
fn should_upload_and_download_large_file_via_multipart_roundtrip() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = "TEST_STAGE_MULTIPART_LARGE";

    let src_dir = tempfile::tempdir().unwrap();
    let src_path = src_dir.path().join("bigfile.bin");
    write_payload(&src_path, MULTIPART_FILE_LEN);
    let src_digest = file_digest(&src_path);

    // When File exceeding the 200 MiB threshold is uploaded via multipart PUT
    // AUTO_COMPRESS=FALSE keeps the on-cloud size above the threshold; the file
    // is incompressible-enough and we compare the raw bytes either way.
    // OVERWRITE=TRUE makes the test idempotent across reruns on the same stage.
    upload_to_stage_with_options(
        &client,
        stage_name,
        src_path.to_str().unwrap(),
        "AUTO_COMPRESS=FALSE OVERWRITE=TRUE",
    );

    // Then File should be downloaded byte-for-byte identical via ranged GET
    let (_get_result, download_dir) = get_file_from_stage(&client, stage_name, "bigfile.bin");
    let downloaded = download_dir.path().join("bigfile.bin");
    assert!(downloaded.exists(), "downloaded file should exist");

    // Byte-for-byte equality via SHA-256 (streamed; avoids holding 420 MiB).
    assert_eq!(
        file_digest(&downloaded),
        src_digest,
        "downloaded file must match the original byte-for-byte"
    );
}
