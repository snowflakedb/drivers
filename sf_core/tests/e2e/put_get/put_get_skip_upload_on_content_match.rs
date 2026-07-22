//! Live e2e for the Azure skip-match upload optimization.

use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::file_utils::shared_test_data_dir;
use crate::common::put_get_common::PutResult;
use crate::common::snowflake_test_client::{SnowflakeTestClient, unwrap_single_rs_handle};
use crate::require_running_on_azure;
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
    format!(
        "PUT 'file://{}' @{stage} OVERWRITE={overwrite_clause} AUTO_COMPRESS=FALSE",
        file_path.display()
    )
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

/// Four-case matrix exercising the cursor kwarg → Azure HEAD digest
/// pipeline against a live Azure-backed stage:
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
#[test]
fn should_skip_upload_on_content_match_round_trip_matrix() {
    require_running_on_azure!();

    let client = SnowflakeTestClient::connect_with_default_auth();
    // Unique per run so concurrent/repeat runs never collide on the name.
    let stage_name = format!(
        "TEST_AZURE_SKIP_MATCH_E2E_{}",
        uuid::Uuid::new_v4().simple()
    );
    let dir = tempfile::tempdir_in(shared_test_data_dir()).expect("tempdir");
    let file_path = dir.path().join("data.csv");

    // Given a fresh Azure-backed temporary stage and a single local file
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
    // Then the blob lands fresh
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
