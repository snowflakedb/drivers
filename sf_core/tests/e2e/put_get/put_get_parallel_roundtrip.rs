use crate::common::file_utils::{create_test_file, path_to_sql_uri};
use crate::common::put_get_common::{get_from_stage_with_parallel, upload_to_stage_with_options};
use crate::common::snowflake_test_client::SnowflakeTestClient;
use uuid::Uuid;

const SMALL_FILE_COUNT: usize = 20;
const PARALLEL: u32 = 4;

fn random_stage_name(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4().simple())
}

/// File-level counterpart to `put_get_multipart_roundtrip`, which covers
/// part-level concurrency within a single large file.
#[test]
fn should_roundtrip_many_small_files_with_parallel_files() {
    // Given Twenty small files whose contents identify which file they came from
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = random_stage_name("TEST_PARALLEL_MANY");
    scopeguard::defer! {
        client.execute_sql(&format!("DROP STAGE IF EXISTS {stage_name}"));
    }

    let src_dir = tempfile::TempDir::new().unwrap();
    let expected: Vec<(String, String)> = (0..SMALL_FILE_COUNT)
        .map(|i| {
            let name = format!("small_{i:02}.csv");
            let body = format!("id,value\n{i},payload-for-file-{i}\n");
            create_test_file(src_dir.path(), &name, &body);
            (name, body)
        })
        .collect();

    // When They are uploaded in one PUT with PARALLEL set, so files transfer
    // concurrently. A single wildcard PUT is the batch the fan-out applies to;
    // twenty separate PUTs would each be a batch of one.
    let wildcard = format!("{}/small_*.csv", path_to_sql_uri(src_dir.path()));
    upload_to_stage_with_options(
        &client,
        &stage_name,
        &wildcard,
        &format!("AUTO_COMPRESS=FALSE OVERWRITE=TRUE PARALLEL={PARALLEL}"),
    );

    // Then Every file downloads with its own contents intact
    let (_get_result, download_dir) = get_from_stage_with_parallel(&client, &stage_name, PARALLEL);

    for (name, body) in &expected {
        let path = download_dir.path().join(name);
        assert!(path.exists(), "{name} should have been downloaded");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read downloaded file"),
            *body,
            "{name} must come back with its own contents, not another file's"
        );
    }

    let downloaded_count = std::fs::read_dir(download_dir.path())
        .expect("read download dir")
        .count();
    assert_eq!(
        downloaded_count, SMALL_FILE_COUNT,
        "every uploaded file must be downloaded exactly once"
    );
}
