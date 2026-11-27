//! Stage downloads with `SNOWFLAKE_RUNNING_INSIDE_XP`: GET through
//! `connection_stage_download` and chunked download streams, served by the
//! host.
//!
//! Own binary, apart from `xp_backend_mode_xp`, so a failure here does not hide
//! the rest of the XP routing coverage. C registration attaches to a
//! process-wide slot, so the binary has one lifecycle and one test. XP hosts
//! exist only on Linux.

#[path = "common/crypto_provider.rs"]
mod crypto_provider;

#[cfg(not(target_os = "linux"))]
#[path = "xp_backend/link_sf_core.rs"]
mod link_sf_core;

#[cfg(target_os = "linux")]
#[path = "xp_backend/fake_host.rs"]
mod fake_host;
#[cfg(target_os = "linux")]
#[path = "xp_backend/fixtures.rs"]
mod fixtures;

#[cfg(target_os = "linux")]
use std::collections::HashMap;

#[cfg(target_os = "linux")]
use sf_core::apis::database_driver_v1::{DatabaseDriverV1, ErrorKind, StageTransferOptions};
#[cfg(target_os = "linux")]
use sf_core::xp_backend::ffi::sf_core_register_xp_backend;

#[cfg(target_os = "linux")]
use fake_host::{HostState, callbacks_for};
#[cfg(target_os = "linux")]
use fixtures::{SELECT_ONE_RESPONSE, SESSION_INFO, result_rows, set_running_inside_xp};

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread")]
async fn xp_mode_downloads_stage_files_through_the_host() {
    set_running_inside_xp();
    let driver = DatabaseDriverV1::new();
    let host = HostState::leaked();
    assert_eq!(
        unsafe { sf_core_register_xp_backend(&callbacks_for(host)) },
        0
    );

    host.set_session_info(SESSION_INFO);
    host.set_query_response(SELECT_ONE_RESPONSE);
    let db = driver.database_new();
    driver.database_init(db).expect("database handle");
    let conn = driver.connection_new();
    driver
        .connection_set_options(conn, HashMap::new(), false, None)
        .await
        .expect("host-backend placeholders should satisfy option validation");
    driver
        .connection_init(None, conn, db)
        .await
        .expect("connection_init should authenticate through the host");

    // A single file: `list_stage_files` resolves the stage reference to one
    // full stage path, and `download_stream` serves its bytes.
    let get_dir = tempfile::tempdir().expect("tempdir");
    host.set_stage_listing(r#"[{"name":"@mystage/get/result.csv"}]"#);
    host.set_stream_body("@mystage/get/result.csv", b"x,y\n1,2\n".to_vec());
    let single_get = driver
        .connection_stage_download(
            None,
            conn,
            "'@mystage/get/result.csv'".to_string(),
            format!("'file://{}'", get_dir.path().display()),
            StageTransferOptions::default(),
        )
        .await
        .expect("GET should route through the backend, never GS");
    let get_rows = result_rows(&driver, single_get.handle).await;
    assert_eq!(get_rows.len(), 1);
    assert_eq!(get_rows[0][0], "result.csv");
    assert_eq!(get_rows[0][1], "8");
    assert_eq!(get_rows[0][2], "DOWNLOADED");
    assert_eq!(
        std::fs::read(get_dir.path().join("result.csv")).expect("file must be written locally"),
        b"x,y\n1,2\n"
    );
    let listing_call = host
        .last_call("list_stage_files")
        .expect("GET must enumerate through list_stage_files");
    assert_eq!(
        listing_call.subject.as_deref(),
        Some("@mystage/get/result.csv")
    );
    assert_eq!(
        listing_call.payload.as_deref(),
        Some("0"),
        "GET's enumeration must not request an exact match"
    );

    // A prefix: every listed file is downloaded into a directory that is
    // created on demand.
    let prefix_root = tempfile::tempdir().expect("tempdir");
    let prefix_dir = prefix_root.path().join("nested");
    host.set_stage_listing(
        r#"[{"name":"@mystage/prefix/a.csv"},{"name":"@mystage/prefix/b.csv"}]"#,
    );
    host.set_stream_body("@mystage/prefix/a.csv", b"aaa".to_vec());
    host.set_stream_body("@mystage/prefix/b.csv", b"bbb".to_vec());
    let prefix_get = driver
        .connection_stage_download(
            None,
            conn,
            "mystage/prefix".to_string(),
            prefix_dir.display().to_string(),
            StageTransferOptions::new([("parallel".to_string(), "4".to_string())]),
        )
        .await
        .expect("a multi-file GET should route through the backend");
    assert_eq!(result_rows(&driver, prefix_get.handle).await.len(), 2);
    assert_eq!(
        host.last_call("list_stage_files").and_then(|c| c.subject),
        Some("@mystage/prefix".to_string()),
        "the stage location gets the @ prefix before listing"
    );
    assert_eq!(
        std::fs::read(prefix_dir.join("a.csv")).expect("a.csv must be written locally"),
        b"aaa"
    );
    assert_eq!(
        std::fs::read(prefix_dir.join("b.csv")).expect("b.csv must be written locally"),
        b"bbb"
    );

    // No listed file is a missing remote file, as for a GET through GS.
    host.set_stage_listing("[]");
    let missing = driver
        .connection_stage_download(
            None,
            conn,
            "@mystage/none".to_string(),
            get_dir.path().display().to_string(),
            StageTransferOptions::default(),
        )
        .await
        .err()
        .expect("an empty listing must fail the GET");
    assert_eq!(missing.kind(), ErrorKind::RemoteFileNotFound);

    // PATTERN has no stored-procedure equivalent and is rejected before listing.
    let listings_before_pattern = host.operation_count("list_stage_files");
    let pattern = driver
        .connection_stage_download(
            None,
            conn,
            "@mystage".to_string(),
            get_dir.path().display().to_string(),
            StageTransferOptions::new([("pattern".to_string(), "'.*[.]csv'".to_string())]),
        )
        .await
        .err()
        .expect("PATTERN must be rejected");
    assert_eq!(pattern.kind(), ErrorKind::NotImplemented);
    assert_eq!(
        host.operation_count("list_stage_files"),
        listings_before_pattern
    );

    // A host error code surfaces as a query failure.
    host.fail_next(2003, Some("stage does not exist"));
    let failed_get = driver
        .connection_stage_download(
            None,
            conn,
            "@nostage".to_string(),
            get_dir.path().display().to_string(),
            StageTransferOptions::default(),
        )
        .await
        .err()
        .expect("a host failure must fail the GET");
    assert_eq!(failed_get.kind(), ErrorKind::QueryFailed);

    // The stage reference is listed as a raw prefix, so a sibling that shares it
    // (`dir2`) is downloaded too.
    let sibling_dir = tempfile::tempdir().expect("tempdir");
    host.set_stage_listing(r#"[{"name":"@mystage/dir/a.csv"},{"name":"@mystage/dir2/b.csv"}]"#);
    host.set_stream_body("@mystage/dir/a.csv", b"in-dir".to_vec());
    host.set_stream_body("@mystage/dir2/b.csv", b"sibling".to_vec());
    driver
        .connection_stage_download(
            None,
            conn,
            "@mystage/dir".to_string(),
            sibling_dir.path().display().to_string(),
            StageTransferOptions::default(),
        )
        .await
        .expect("a prefix GET downloads every listed name");
    assert_eq!(
        std::fs::read(sibling_dir.path().join("a.csv")).expect("a.csv"),
        b"in-dir"
    );
    assert_eq!(
        std::fs::read(sibling_dir.path().join("b.csv")).expect("b.csv"),
        b"sibling"
    );

    // Listed names that share a basename land in one flat file; the later one
    // wins.
    let collide_dir = tempfile::tempdir().expect("tempdir");
    host.set_stage_listing(r#"[{"name":"@mystage/c/x.csv"},{"name":"@mystage/d/x.csv"}]"#);
    host.set_stream_body("@mystage/c/x.csv", b"first".to_vec());
    host.set_stream_body("@mystage/d/x.csv", b"second".to_vec());
    driver
        .connection_stage_download(
            None,
            conn,
            "@mystage".to_string(),
            collide_dir.path().display().to_string(),
            StageTransferOptions::default(),
        )
        .await
        .expect("colliding basenames do not fail the GET");
    assert_eq!(
        std::fs::read(collide_dir.path().join("x.csv")).expect("x.csv"),
        b"second"
    );
    assert_eq!(
        std::fs::read_dir(collide_dir.path())
            .expect("read dir")
            .count(),
        1
    );

    // A listed name that is not a safe file name is never written locally.
    let unsafe_dir = tempfile::tempdir().expect("tempdir");
    host.set_stage_listing(r#"[{"name":"@mystage/unsafe/.."}]"#);
    let unsafe_get = driver
        .connection_stage_download(
            None,
            conn,
            "@mystage/unsafe".to_string(),
            unsafe_dir.path().display().to_string(),
            StageTransferOptions::default(),
        )
        .await;
    assert!(
        unsafe_get.is_err(),
        "an unsafe listed name must fail the GET"
    );
    assert_eq!(
        std::fs::read_dir(unsafe_dir.path())
            .expect("read dir")
            .count(),
        0
    );

    // PARALLEL above 99 is rejected before listing.
    let listings_before_parallel = host.operation_count("list_stage_files");
    let too_parallel = driver
        .connection_stage_download(
            None,
            conn,
            "@mystage".to_string(),
            get_dir.path().display().to_string(),
            StageTransferOptions::new([("PARALLEL".to_string(), "100".to_string())]),
        )
        .await
        .err()
        .expect("PARALLEL=100 must be rejected");
    assert_eq!(too_parallel.kind(), ErrorKind::InvalidArgument);
    assert_eq!(
        host.operation_count("list_stage_files"),
        listings_before_parallel
    );

    // Download stream, chunked: the whole scripted body comes back piecemeal.
    let downloads_before_stream = host.operation_count("download_stream");
    let dl_body = b"chunk-one-chunk-two-chunk-three".to_vec();
    host.set_stream_body("@dlstage/chunked.csv", dl_body.clone());
    let (download_handle, total_len) = driver
        .download_stream_begin(
            None,
            conn,
            "@dlstage".to_string(),
            "chunked.csv".to_string(),
            false,
        )
        .await
        .expect("begin a chunked download");
    assert_eq!(total_len, Some(dl_body.len() as i64));
    let mut collected = Vec::new();
    loop {
        let (bytes, eof) = driver
            .download_stream_chunk(download_handle, 8)
            .await
            .expect("drain a chunk");
        collected.extend_from_slice(&bytes);
        if eof {
            break;
        }
    }
    driver
        .download_stream_close(download_handle)
        .await
        .expect("close the download stream");
    assert_eq!(collected, dl_body);
    assert_eq!(
        host.operation_count("download_stream"),
        downloads_before_stream + 1
    );

    // Closing a download stream before draining it releases the handle.
    host.set_stream_body("@dlstage/early.csv", vec![b'x'; 64 * 1024]);
    let (early_handle, _) = driver
        .download_stream_begin(
            None,
            conn,
            "@dlstage".to_string(),
            "early.csv".to_string(),
            false,
        )
        .await
        .expect("begin a chunked download");
    driver
        .download_stream_close(early_handle)
        .await
        .expect("close before draining");
    assert!(
        driver.download_stream_chunk(early_handle, 8).await.is_err(),
        "a closed stream must not serve chunks"
    );

    // A host error opening the stream keeps its kind.
    host.fail_next(2003, Some("file does not exist"));
    let stream_failure = driver
        .download_stream_begin(
            None,
            conn,
            "@dlstage".to_string(),
            "missing.csv".to_string(),
            false,
        )
        .await
        .expect_err("a host failure must fail the stream open");
    assert_eq!(stream_failure.kind(), ErrorKind::QueryFailed);

    driver.connection_release(conn).expect("release connection");
    driver.database_release(db).expect("release database");
}
