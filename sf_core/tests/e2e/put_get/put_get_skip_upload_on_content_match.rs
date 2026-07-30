//! Live e2e for the skip-match upload optimization.
//!
//! Provider-agnostic: runs against the cloud-backed stage of whatever
//! provider the test account targets (AWS / Azure / GCP lanes, plus the
//! dev wildcard). The skip-on-content-match round-trip is the observable
//! contract regardless of the backing object store.

use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::file_utils::shared_test_data_dir;
use crate::common::put_get_common::PutResult;
use crate::common::snowflake_test_client::{SnowflakeTestClient, unwrap_single_rs_handle};
use sf_core::protobuf::generated::database_driver_v1::{ConfigSetting, ResultSetGetStreamResponse};
use std::path::Path;

fn run_put_with_kwarg(
    client: &SnowflakeTestClient,
    sql: &str,
    skip_match: bool,
) -> ResultSetGetStreamResponse {
    let stmt = client.new_statement();
    client.set_sql_query(&stmt, sql);
    if skip_match {
        client.set_statement_option(
            &stmt,
            "skip_upload_on_content_match",
            ConfigSetting::from(true),
        );
    }
    let result = client.execute_statement_query(&stmt);
    let rs_handle = unwrap_single_rs_handle(&result);
    client.result_set_get_stream(&rs_handle)
}

fn build_put_sql(stage: &str, file_path: &Path, overwrite: bool) -> String {
    // AUTO_COMPRESS=FALSE: gzip output isn't byte-deterministic, so a
    // re-PUT with compression on would mismatch its own remote digest.
    let overwrite_clause = if overwrite { "TRUE" } else { "FALSE" };
    // Snowflake's SQL string-literal parser treats backslash as an escape
    // character, so a Windows path (which `Path::display()` renders with `\`
    // separators) would be corrupted inside the quoted `file://` URI. Normalize
    // to forward slashes, which Snowflake accepts on every host OS.
    let file_uri = file_path.to_string_lossy().replace('\\', "/");
    format!("PUT 'file://{file_uri}' @{stage} OVERWRITE={overwrite_clause} AUTO_COMPRESS=FALSE")
}

fn assert_status(stream: ResultSetGetStreamResponse, expected: &str) {
    let mut helper = ArrowResultHelper::from_result(stream);
    let row: PutResult = helper.fetch_one().expect("fetch PutResult");
    assert_eq!(
        row.status, expected,
        "PUT status mismatch (file={}, expected={expected}, got={})",
        row.source, row.status,
    );
}

/// Four-case matrix exercising the cursor kwarg → HEAD digest pipeline
/// against the cloud-backed stage of whatever provider the test account
/// targets:
///
/// | Round | content | overwrite | skip_match | expected status     |
/// |------:|---------|-----------|------------|---------------------|
/// |   1   | "A"     | TRUE      | false      | UPLOADED            |
/// |   2   | "A"     | TRUE      | true       | SKIPPED (digest)    |
/// |   3   | "B"     | TRUE      | true       | UPLOADED (mismatch) |
/// |   4   | "C"     | FALSE     | false      | SKIPPED (existence) |
///
/// Round 4 pins that existence wins under !overwrite even when the
/// local content differs from what's on stage.
///
/// Scope: this is a cross-provider round-trip smoke test — it proves the
/// kwarg → HEAD → skip pipeline works on the backing store, but it does NOT
/// prove the flag is *required*. The one round that would distinguish an
/// opt-in cloud (S3/Azure) from an unconditional one (GCS) —
/// `overwrite=TRUE + matching digest + skip_match=FALSE` → UPLOADED on opt-in,
/// SKIPPED on unconditional — is deliberately absent (it would fail the GCS
/// lane at this SHA; #728 adds it). A green run here is therefore NOT proof
/// that the opt-in gate holds; that gate is pinned at the unit layer by
/// `content_match_skip_does_not_fire_for_s3_without_opt_in`.
#[test]
fn should_skip_upload_on_content_match_round_trip_matrix() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    // Unique per run so concurrent/repeat runs never collide on the name.
    let stage_name = format!("TEST_SKIP_MATCH_E2E_{}", uuid::Uuid::new_v4().simple());
    let dir = tempfile::tempdir_in(shared_test_data_dir()).expect("tempdir");
    let file_path = dir.path().join("data.csv");

    // Given a fresh cloud-backed temporary stage and a single local file
    client.execute_sql(&format!("CREATE OR REPLACE TEMPORARY STAGE {stage_name}"));
    // Drop the stage even if an assertion below panics.
    scopeguard::defer! {
        client.execute_sql(&format!("DROP STAGE IF EXISTS {stage_name}"));
    }

    // When Round 1 PUTs content "A" with OVERWRITE=TRUE and skip flag off
    std::fs::write(&file_path, b"content A").expect("write A");
    let r1 = run_put_with_kwarg(
        &client,
        &build_put_sql(&stage_name, &file_path, /*overwrite*/ true),
        /*skip_match*/ false,
    );
    // Then the object lands fresh
    assert_status(r1, "UPLOADED");

    // And Round 2 PUTs the same content with skip_match=true
    let r2 = run_put_with_kwarg(&client, &build_put_sql(&stage_name, &file_path, true), true);
    // Then the upload is skipped via digest equality
    assert_status(r2, "SKIPPED");

    // And Round 3 modifies content to "B" with skip_match still on
    std::fs::write(&file_path, b"content B").expect("write B");
    let r3 = run_put_with_kwarg(&client, &build_put_sql(&stage_name, &file_path, true), true);
    // Then the upload runs because the digest mismatch defeats the skip
    assert_status(r3, "UPLOADED");

    // And Round 4 modifies content to "C" with OVERWRITE=FALSE and skip flag off
    std::fs::write(&file_path, b"content C").expect("write C");
    let r4 = run_put_with_kwarg(
        &client,
        &build_put_sql(&stage_name, &file_path, /*overwrite*/ false),
        /*skip_match*/ false,
    );
    // Then existence wins and the upload is skipped without comparing digests
    assert_status(r4, "SKIPPED");
}

/// Pure regression pin for the `\` → `/` PUT-path normalization in
/// `build_put_sql`. The backslash branch is only hit transitively by the
/// Windows `aws` live lanes, so this credential-free test keeps it green on
/// Linux CI too (backslashes are ordinary characters in a Unix path string,
/// so `Path::new` preserves them for the `replace` to rewrite).
#[test]
fn should_normalize_windows_backslashes_in_build_put_sql() {
    // Given a Windows-style local path with backslash separators
    let win_path = Path::new(r"C:\Users\me\My Files\data.csv");
    // When build_put_sql renders the quoted file:// URI for the PUT statement
    let sql = build_put_sql("MY_STAGE", win_path, /*overwrite*/ true);
    // Then every backslash is rewritten to a forward slash, so Snowflake's SQL
    // string-literal parser never treats a separator as an escape sequence
    assert_eq!(
        sql,
        "PUT 'file://C:/Users/me/My Files/data.csv' @MY_STAGE OVERWRITE=TRUE AUTO_COMPRESS=FALSE",
    );
    assert!(
        !sql.contains('\\'),
        "PUT path must not contain backslashes (SQL string-literal escape): {sql}",
    );
}
