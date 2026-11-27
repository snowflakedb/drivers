//! End-to-end registration and routing with `SNOWFLAKE_RUNNING_INSIDE_XP`.
//!
//! One long test on purpose: C registration attaches to a process-wide slot,
//! so a process has exactly one lifecycle to observe. States that need a
//! different slot get their own binary — `xp_backend_mode_http` and
//! `xp_backend_mode_xp_unregistered`. XP hosts exist only on Linux.

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
use std::sync::Arc;

#[cfg(target_os = "linux")]
use sf_core::apis::database_driver_v1::{
    AsyncArrowBatchFetcher, DatabaseDriverV1, ErrorKind, ExecuteQueryResult, StageTransferOptions,
};
#[cfg(target_os = "linux")]
use sf_core::config::settings::Setting;
#[cfg(target_os = "linux")]
use sf_core::rest::snowflake::{
    AbortOutcome, QueryInput, QueryOptions, RestError, get_query_status, snowflake_abort_query,
    snowflake_cancel_query, snowflake_get_query_result, snowflake_login_with_client,
    snowflake_query_with_client,
};
#[cfg(target_os = "linux")]
use sf_core::sensitive::SensitiveString;
#[cfg(target_os = "linux")]
use sf_core::xp_backend::SnowflakeBackend;
#[cfg(target_os = "linux")]
use sf_core::xp_backend::error_codes;
#[cfg(target_os = "linux")]
use sf_core::xp_backend::ffi::{
    UPLOAD_STREAM_CHUNK_LEN, XP_BACKEND_ABI_VERSION, XpCancelRequest, XpQueryRequest,
    sf_core_register_xp_backend, sf_core_xp_backend_is_available,
};

#[cfg(target_os = "linux")]
use fake_host::{HostState, callbacks_for};
#[cfg(target_os = "linux")]
use fixtures::{
    MONITORING_RUNNING_RESPONSE, SECRET_PASSWORD, SELECT_ONE_RESPONSE, SESSION_INFO,
    login_parameters, query_parameters, result_rows, set_running_inside_xp,
};

#[cfg(target_os = "linux")]
// `multi_thread`, not `current_thread`: the chunked-upload-stream coverage
// below goes through `connection_upload_stream_chunk`, which calls
// `tokio::task::block_in_place` (see `stream_transfer.rs`) and panics outside
// a multi-threaded runtime.
#[tokio::test(flavor = "multi_thread")]
async fn xp_mode_registers_once_and_routes_query_and_login_to_the_host() {
    set_running_inside_xp();
    let driver = DatabaseDriverV1::new();

    let host = HostState::leaked();

    // Each must leave the slot untouched, so a host that gets the ABI wrong
    // fails at startup instead of half-installing.

    assert_eq!(
        unsafe { sf_core_register_xp_backend(std::ptr::null()) },
        error_codes::INVALID_ARGUMENT,
        "a null callbacks pointer must be rejected, not dereferenced"
    );

    let mut wrong_version = callbacks_for(host);
    wrong_version.abi_version = XP_BACKEND_ABI_VERSION + 1;
    assert_eq!(
        unsafe { sf_core_register_xp_backend(&wrong_version) },
        error_codes::ABI_MISMATCH,
        "a host built against a different ABI must be rejected"
    );

    let mut wrong_size = callbacks_for(host);
    wrong_size.struct_size -= 8;
    assert_eq!(
        unsafe { sf_core_register_xp_backend(&wrong_size) },
        error_codes::ABI_MISMATCH,
        "a struct-size mismatch must be rejected before any field is read"
    );

    unsafe extern "C" fn unknown_callback() {}
    let mut reserved_slot_set = callbacks_for(host);
    reserved_slot_set.reserved[0] = Some(unknown_callback);
    assert_eq!(
        unsafe { sf_core_register_xp_backend(&reserved_slot_set) },
        error_codes::ABI_MISMATCH,
        "a callback this driver does not define must be rejected, not ignored"
    );

    let mut missing_required = callbacks_for(host);
    missing_required.authenticate = None;
    assert_eq!(
        unsafe { sf_core_register_xp_backend(&missing_required) },
        error_codes::INVALID_ARGUMENT,
        "a null required callback must be caught at registration, not on first use"
    );

    assert_eq!(
        host.call_count(),
        0,
        "registration must not invoke any callback"
    );

    let callbacks = callbacks_for(host);
    assert_eq!(unsafe { sf_core_register_xp_backend(&callbacks) }, 0);

    assert_eq!(
        unsafe { sf_core_register_xp_backend(&callbacks) },
        error_codes::ALREADY_REGISTERED,
        "backends cannot be replaced once installed"
    );

    let backend: Arc<dyn SnowflakeBackend> = Arc::clone(
        driver
            .xp_backend()
            .expect("xp mode with a registration")
            .expect("a backend is installed"),
    );
    let xp_backend = Some(backend.as_ref());

    host.set_query_response(SELECT_ONE_RESPONSE);
    let client = reqwest::Client::new();

    let bindings = serde_json::value::RawValue::from_string(
        r#"{"1":{"type":"TEXT","value":"abc"}}"#.to_string(),
    )
    .unwrap();
    let input = QueryInput {
        sql: "select 1 where c = ?".to_string(),
        bindings: Some(&bindings),
        bind_stage: Some("@stage/binds".to_string()),
        describe_only: Some(true),
        query_parameters: Some(HashMap::from([(
            "TIMEZONE".to_string(),
            serde_json::json!("UTC"),
        )])),
        query_context: Default::default(),
    };

    let response = snowflake_query_with_client(
        &client,
        query_parameters(),
        "unused-session-token",
        input,
        QueryOptions::default(),
        xp_backend,
    )
    .await
    .expect("query should be served by the host");

    assert!(response.success);
    assert_eq!(
        response.data.rowset.as_ref().expect("rowset present"),
        &vec![vec![Some("1".to_string())]]
    );

    let call = host
        .last_call("execute_query")
        .expect("the host should have been asked to execute the query");
    assert_eq!(call.subject.as_deref(), Some("select 1 where c = ?"));

    let request: XpQueryRequest =
        serde_json::from_str(&call.payload.expect("execute_query carries a payload"))
            .expect("the driver should send a parseable XpQueryRequest");
    assert!(request.is_describe_only, "describe_only must be forwarded");
    assert!(
        !request.is_async,
        "the default QueryOptions is blocking, so is_async must be false"
    );
    assert_eq!(request.bind_stage.as_deref(), Some("@stage/binds"));
    assert_eq!(
        request.bindings,
        Some(serde_json::json!({"1": {"type": "TEXT", "value": "abc"}})),
        "bindings must arrive as JSON, not as an escaped string"
    );
    assert_eq!(
        request.statement_params,
        Some(HashMap::from([(
            "TIMEZONE".to_string(),
            serde_json::json!("UTC")
        )]))
    );
    assert!(
        request
            .request_id
            .as_deref()
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok()),
        "a parseable requestId must be forwarded so the host can correlate a cancel"
    );

    host.set_session_info(SESSION_INFO);
    let login = snowflake_login_with_client(
        &sf_core::rest::snowflake::LoginClients::new(&client),
        &login_parameters(),
        Some(&HashMap::from([(
            "QUERY_TAG".to_string(),
            "sproc".to_string(),
        )])),
        None,
        None,
        &Default::default(),
        xp_backend,
        None,
    )
    .await
    .expect("login should be served by the host");

    assert_eq!(login.tokens.session_token.reveal(), "host-session-token");
    assert_eq!(login.tokens.session_id, Some(987_654_321));
    assert_eq!(login.database_name.as_deref(), Some("TESTDB"));
    assert_eq!(login.server_version.as_deref(), Some("9.1.0"));
    assert_eq!(
        login
            .session_parameters
            .as_ref()
            .and_then(|p| p.get("AUTOCOMMIT")),
        Some(&Setting::String("true".to_string()))
    );
    assert!(
        !login.tokens.is_session_expired() && !login.tokens.is_master_expired(),
        "a host session with no stated validity must not present as expired"
    );

    let auth_payload = host
        .last_call("authenticate")
        .and_then(|c| c.payload)
        .expect("authenticate carries a payload");
    assert!(
        !auth_payload.contains(SECRET_PASSWORD),
        "the login password must never cross the ABI; payload was: {auth_payload}"
    );
    assert!(
        auth_payload.contains("testaccount") && auth_payload.contains("QUERY_TAG"),
        "the host needs the account and the requested session parameters: {auth_payload}"
    );

    // Result fetch (the async-query and async PUT/GET retrieval path).
    let fetched = snowflake_get_query_result(
        &client,
        &query_parameters(),
        "unused-session-token",
        "01b2c3d4-0000-0000-0000-000000000001",
        &Default::default(),
        xp_backend,
    )
    .await
    .expect("query result should be served by the host");
    assert!(fetched.success);
    assert_eq!(
        host.last_call("get_query_result").and_then(|c| c.subject),
        Some("01b2c3d4-0000-0000-0000-000000000001".to_string())
    );

    // A monitoring body returned verbatim, run through the driver's usual
    // extraction.
    host.set_json_payload(MONITORING_RUNNING_RESPONSE);
    let status = get_query_status(
        &client,
        &query_parameters(),
        &SensitiveString::from("unused-session-token".to_string()),
        "01b2c3d4-0000-0000-0000-000000000001",
        &Default::default(),
        xp_backend,
    )
    .await
    .expect("status should be served by the host");
    assert_eq!(status.status_name, "RUNNING");
    assert_eq!(status.state, "RUNNING");
    assert_eq!(status.session_id, 987_654_321);
    assert_eq!(
        status.error_code, None,
        "errorCode 0 must read as no error, not as error 0"
    );

    // Cancel by query id, via abort.
    host.set_cancel_outcome(true);
    let outcome = snowflake_abort_query(
        &client,
        &query_parameters(),
        "unused-session-token",
        "01b2c3d4-0000-0000-0000-000000000001",
        xp_backend,
    )
    .await
    .expect("abort should be served by the host");
    assert_eq!(outcome, AbortOutcome::Aborted);
    let cancel_request: XpCancelRequest = serde_json::from_str(
        &host
            .last_call("cancel_query")
            .and_then(|c| c.payload)
            .expect("cancel carries a payload"),
    )
    .unwrap();
    assert_eq!(
        cancel_request.query_id.as_deref(),
        Some("01b2c3d4-0000-0000-0000-000000000001")
    );
    assert_eq!(
        cancel_request.request_id, None,
        "the abort path knows the query id, so it must not also send a requestId"
    );

    // Cancel by requestId, the path taken before a query id exists.
    host.set_cancel_outcome(false);
    let outcome = snowflake_cancel_query(
        &client,
        &query_parameters(),
        "unused-session-token",
        "3fa85f64-5717-4562-b3fc-2c963f66afa6",
        "select 1",
        xp_backend,
    )
    .await
    .expect("a cancel that finds nothing running is not an error");
    assert_eq!(outcome, AbortOutcome::NotRunning);
    let cancel_request: XpCancelRequest = serde_json::from_str(
        &host
            .last_call("cancel_query")
            .and_then(|c| c.payload)
            .expect("cancel carries a payload"),
    )
    .unwrap();
    assert_eq!(
        cancel_request.request_id.as_deref(),
        Some("3fa85f64-5717-4562-b3fc-2c963f66afa6")
    );
    assert_eq!(cancel_request.sql_text.as_deref(), Some("select 1"));
    assert_eq!(cancel_request.query_id, None);

    // Session parameters.
    host.set_json_payload(r#"{"AUTOCOMMIT":"true","TIMEZONE":"UTC"}"#);
    let params = backend
        .get_session_parameters()
        .await
        .expect("session parameters should be served by the host");
    assert_eq!(params.get("AUTOCOMMIT").map(String::as_str), Some("true"));
    assert_eq!(params.get("TIMEZONE").map(String::as_str), Some("UTC"));

    // A Snowflake server code must survive rather than being flattened into a
    // generic transport failure.
    host.fail_next(1003, Some("SQL compilation error: syntax error"));
    let err = snowflake_query_with_client(
        &client,
        query_parameters(),
        "unused-session-token",
        QueryInput::new("select nonsense"),
        QueryOptions::default(),
        xp_backend,
    )
    .await
    .map(|_| ())
    .expect_err("a scripted host failure should surface as an error");

    match err {
        RestError::Backend { source, .. } => {
            assert_eq!(source.code, 1003);
            assert_eq!(source.message, "SQL compilation error: syntax error");
        }
        other => panic!("expected RestError::Backend, got: {other:?}"),
    }

    host.fail_next(0, None);
    let err = snowflake_query_with_client(
        &client,
        query_parameters(),
        "unused-session-token",
        QueryInput::new("select 1"),
        QueryOptions::default(),
        xp_backend,
    )
    .await
    .map(|_| ())
    .expect_err("a messageless failure is still a failure");
    match err {
        RestError::Backend { source, .. } => {
            assert_eq!(source.code, error_codes::PROTOCOL);
            assert!(
                source.message.contains("execute_query")
                    && source.message.contains("without a message"),
                "got: {}",
                source.message
            );
        }
        other => panic!("expected RestError::Backend, got: {other:?}"),
    }

    let err = snowflake_query_with_client(
        &client,
        query_parameters(),
        "unused-session-token",
        QueryInput::new("select 1\0trailing"),
        QueryOptions::default(),
        xp_backend,
    )
    .await
    .map(|_| ())
    .expect_err("an interior NUL in SQL must not panic across the ABI");
    match err {
        RestError::Backend { source, .. } => {
            assert_eq!(source.code, error_codes::INVALID_ARGUMENT);
            assert!(
                source.message.contains("SQL text"),
                "got: {}",
                source.message
            );
        }
        other => panic!("expected RestError::Backend, got: {other:?}"),
    }

    host.set_query_response(
        r#"{"data":{"queryId":"01-failed","sqlState":"42S02"},"code":"002003","message":"SQL compilation error: Object 'MISSING' does not exist or not authorized.","success":false}"#,
    );
    let err = snowflake_query_with_client(
        &client,
        query_parameters(),
        "unused-session-token",
        QueryInput::new("select * from missing"),
        QueryOptions::default(),
        xp_backend,
    )
    .await
    .map(|_| ())
    .expect_err("a failed query answered by the host must not read as an empty result");
    match err {
        RestError::QueryFailed {
            code,
            sql_state,
            message,
            ids,
            ..
        } => {
            assert_eq!(code, Some(2003));
            assert_eq!(sql_state.as_deref(), Some("42S02"));
            assert!(message.contains("does not exist"), "got: {message}");
            assert_eq!(ids.query_id.as_deref(), Some("01-failed"));
            assert!(ids.request_id.is_some());
        }
        other => panic!("expected RestError::QueryFailed, got: {other:?}"),
    }
    let err = snowflake_get_query_result(
        &client,
        &query_parameters(),
        "unused-session-token",
        "01-failed",
        &Default::default(),
        xp_backend,
    )
    .await
    .map(|_| ())
    .expect_err("a failed result fetched from the host must surface as an error");
    assert!(
        matches!(
            err,
            RestError::QueryFailed {
                code: Some(2003),
                ..
            }
        ),
        "got: {err:?}"
    );

    host.set_session_info(SESSION_INFO);
    host.set_query_response(SELECT_ONE_RESPONSE);
    let authenticate_before_init = host.operation_count("authenticate");
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
    assert_eq!(
        host.operation_count("authenticate"),
        authenticate_before_init + 1,
        "connection_init must call authenticate rather than HTTP login"
    );

    let stmt = driver.statement_new(conn).expect("statement handle");
    driver
        .statement_set_sql_query(stmt, "select 1".to_string())
        .await
        .expect("set sql");
    let result = driver
        .statement_execute_query(None, stmt, None, None, None, None)
        .await
        .expect("statement_execute_query should go through the host");
    assert!(matches!(result, ExecuteQueryResult::Single { .. }));
    assert_eq!(
        host.last_call("execute_query").and_then(|c| c.subject),
        Some("select 1".to_string())
    );
    driver.statement_release(stmt).expect("release statement");
    // --- Stage uploads through the structured API ---
    //
    // `query_parameters()` points at `UNREACHABLE_SERVER_URL`, so a regression
    // that fell back to GS fails instead of reaching a real endpoint.

    host.set_stage_listing("[]");
    let put_dir = tempfile::tempdir().expect("tempdir");
    let put_file = put_dir.path().join("data.csv");
    std::fs::write(&put_file, b"a,b,c\n1,2,3\n").expect("write temp file");
    let default_put = driver
        .connection_stage_upload(
            None,
            conn,
            format!("'file://{}'", put_file.display()),
            "mystage/sub".to_string(),
            StageTransferOptions::default(),
        )
        .await
        .expect("PUT should route through the backend, never GS");
    let rows = result_rows(&driver, default_put.handle).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "data.csv");
    assert_eq!(rows[0][1], "data.csv.gz");
    assert_eq!(rows[0][6], "UPLOADED");

    let (uploaded_bytes, metadata_json) = host
        .uploaded_blob("@mystage/sub/data.csv.gz")
        .expect("auto-compress must rename the target with a .gz suffix");
    assert_eq!(
        &uploaded_bytes[..2],
        &[0x1f, 0x8b],
        "the backend must receive gzip bytes, not the plaintext source"
    );
    assert_eq!(rows[0][3], uploaded_bytes.len().to_string());
    let metadata: serde_json::Value =
        serde_json::from_str(&metadata_json.expect("upload_stream carries metadata_json"))
            .expect("metadata_json must parse");
    assert_eq!(metadata["overwrite"], false);
    assert_eq!(metadata["target_compression"], "GZIP");

    // The host reports the stored size when it pads the object.
    host.set_upload_stream_response(Some(r#"{"padded_file_size": 4096}"#));
    let padded_file = put_dir.path().join("padded.csv");
    std::fs::write(&padded_file, b"x").expect("write temp file");
    let padded_put = driver
        .connection_stage_upload(
            None,
            conn,
            padded_file.display().to_string(),
            "@mystage".to_string(),
            StageTransferOptions::default(),
        )
        .await
        .expect("PUT should succeed");
    assert_eq!(result_rows(&driver, padded_put.handle).await[0][3], "4096");

    // The stored size is part of the host contract: a result without it fails.
    host.set_upload_stream_response(Some(r#"{"status":"UPLOADED"}"#));
    let unsized_put = driver
        .connection_stage_upload(
            None,
            conn,
            padded_file.display().to_string(),
            "@mystage".to_string(),
            StageTransferOptions::new([("OVERWRITE".to_string(), "TRUE".to_string())]),
        )
        .await;
    assert!(
        unsized_put.is_err(),
        "a finish result without padded_file_size must fail the PUT"
    );
    host.set_upload_stream_response(None);

    // A file larger than one chunk reaches the host as several writes.
    let large_file = put_dir.path().join("large.bin");
    let large_bytes: Vec<u8> = (0..UPLOAD_STREAM_CHUNK_LEN * 2 + 123)
        .map(|i| (i % 251) as u8)
        .collect();
    std::fs::write(&large_file, &large_bytes).expect("write temp file");
    let large_put = driver
        .connection_stage_upload(
            None,
            conn,
            large_file.display().to_string(),
            "@mystage/large".to_string(),
            StageTransferOptions::new([
                ("AUTO_COMPRESS".to_string(), "FALSE".to_string()),
                ("OVERWRITE".to_string(), "TRUE".to_string()),
            ]),
        )
        .await
        .expect("a multi-chunk PUT should succeed");
    let (received, _) = host
        .uploaded_blob("@mystage/large/large.bin")
        .expect("the large file must reach the host");
    assert!(
        received == large_bytes,
        "the chunks must reassemble to the source bytes"
    );
    assert_eq!(host.upload_write_count("@mystage/large/large.bin"), Some(3));
    assert_eq!(
        result_rows(&driver, large_put.handle).await[0][3],
        large_bytes.len().to_string()
    );

    // A failed write aborts the host's upload and fails the PUT.
    let aborts_before = host.aborted_uploads();
    host.fail_next_upload_write(2003);
    let write_failure = driver
        .connection_stage_upload(
            None,
            conn,
            large_file.display().to_string(),
            "@mystage/broken".to_string(),
            StageTransferOptions::new([("OVERWRITE".to_string(), "TRUE".to_string())]),
        )
        .await
        .err()
        .expect("a failed write must fail the PUT");
    assert_eq!(write_failure.kind(), ErrorKind::QueryFailed);
    assert_eq!(host.aborted_uploads(), aborts_before + 1);
    assert!(host.uploaded_blob("@mystage/broken/large.bin.gz").is_none());

    // OVERWRITE=FALSE against an existing destination skips without uploading.
    let uploads_before_skip = host.operation_count("upload_stream");
    host.set_stage_listing(r#"[{"name":"@mystage/sub/skip.csv"}]"#);
    let skip_file = put_dir.path().join("skip.csv");
    std::fs::write(&skip_file, b"ignored").expect("write temp file");
    let skip_put = driver
        .connection_stage_upload(
            None,
            conn,
            skip_file.display().to_string(),
            "mystage/sub".to_string(),
            StageTransferOptions::new([("auto_compress".to_string(), "False".to_string())]),
        )
        .await
        .expect("an existing target must SKIP, not error");
    let skip_rows = result_rows(&driver, skip_put.handle).await;
    assert_eq!(skip_rows[0][6], "SKIPPED");
    assert_eq!(
        skip_rows[0][3], "0",
        "a skipped file reports a target size of 0"
    );
    assert_eq!(
        host.operation_count("upload_stream"),
        uploads_before_skip,
        "a SKIPPED upload must never call upload_stream"
    );
    let listing_call = host
        .last_call("list_stage_files")
        .expect("the skip check must consult list_stage_files");
    assert_eq!(
        listing_call.subject.as_deref(),
        Some("@mystage/sub/skip.csv")
    );
    assert_eq!(
        listing_call.payload.as_deref(),
        Some("1"),
        "the skip check must ask for an exact match"
    );

    // A listing that is not a JSON array is a protocol error, never a skip.
    host.set_stage_listing("null");
    let malformed = driver
        .connection_stage_upload(
            None,
            conn,
            skip_file.display().to_string(),
            "mystage/sub".to_string(),
            StageTransferOptions::default(),
        )
        .await;
    assert!(malformed.is_err(), "a malformed listing must fail the PUT");
    assert_eq!(host.operation_count("upload_stream"), uploads_before_skip);

    // OVERWRITE=FALSE cannot be honored on snow:// stages.
    host.set_stage_listing("[]");
    let snow_put = driver
        .connection_stage_upload(
            None,
            conn,
            skip_file.display().to_string(),
            "snow://stage/dir".to_string(),
            StageTransferOptions::default(),
        )
        .await
        .err()
        .expect("overwrite=false on snow:// must be rejected");
    assert_eq!(snow_put.kind(), ErrorKind::NotImplemented);
    assert_eq!(host.operation_count("upload_stream"), uploads_before_skip);

    // A host error code surfaces as a query failure, and the PUT stops there.
    host.fail_next(2003, Some("stage does not exist"));
    let failed = driver
        .connection_stage_upload(
            None,
            conn,
            skip_file.display().to_string(),
            "mystage/sub".to_string(),
            StageTransferOptions::new([("OVERWRITE".to_string(), "TRUE".to_string())]),
        )
        .await
        .err()
        .expect("a host failure must fail the PUT");
    assert_eq!(failed.kind(), ErrorKind::QueryFailed);

    // A glob matching several files uploads them one at a time.
    let glob_dir = tempfile::tempdir().expect("tempdir");
    for name in ["a.csv", "b.csv", "c.csv", "d.csv"] {
        std::fs::write(glob_dir.path().join(name), name).expect("write temp file");
    }
    let glob_put = driver
        .connection_stage_upload(
            None,
            conn,
            format!("file://{}/*.csv", glob_dir.path().display()),
            "mystage/glob".to_string(),
            StageTransferOptions::new([
                ("PARALLEL".to_string(), "8".to_string()),
                ("OVERWRITE".to_string(), "TRUE".to_string()),
            ]),
        )
        .await
        .expect("glob PUT should succeed");
    assert_eq!(result_rows(&driver, glob_put.handle).await.len(), 4);
    assert_eq!(
        host.max_concurrent_uploads(),
        1,
        "XP uploads run one file at a time whatever PARALLEL says"
    );

    // Upload stream, chunked: bytes arrive piecemeal but the host sees one body.
    host.set_stage_listing("[]");
    let upload_handle = driver
        .connection_stage_upload_stream_begin(
            conn,
            "'@mystage/chunked/chunked.csv'".to_string(),
            StageTransferOptions::new([("AUTO_COMPRESS".to_string(), "FALSE".to_string())]),
        )
        .await
        .expect("begin a chunked upload");
    driver
        .connection_upload_stream_chunk(upload_handle, b"first,".to_vec())
        .await
        .expect("append first chunk");
    driver
        .connection_upload_stream_chunk(upload_handle, b"second".to_vec())
        .await
        .expect("append second chunk");
    let stream_put = driver
        .connection_upload_stream_finish(None, upload_handle)
        .await
        .expect("finish should route the assembled body through the backend");
    let (chunked_bytes, _) = host
        .uploaded_blob("@mystage/chunked/chunked.csv")
        .expect("the assembled chunks must reach upload_stream as one body");
    assert_eq!(chunked_bytes, b"first,second");
    let stream_rows = result_rows(&driver, stream_put.handle).await;
    assert_eq!(stream_rows[0][1], "chunked.csv");
    assert_eq!(stream_rows[0][6], "UPLOADED");

    // `cursor.upload_stream`'s options write the bytes as is: gzip bytes under a
    // .zip name are neither detected, recompressed nor renamed, and no listing
    // runs.
    let listings_before_raw = host.operation_count("list_stage_files");
    let raw_bytes = vec![0x1f, 0x8b, 0x08, 0x00, 0x01, 0x02];
    let raw_handle = driver
        .connection_stage_upload_stream_begin(
            conn,
            "@db.sch.\"my stage\"/udfs/lib.zip".to_string(),
            StageTransferOptions::new([
                ("auto_compress".to_string(), "False".to_string()),
                ("source_compression".to_string(), "NONE".to_string()),
                ("overwrite".to_string(), "True".to_string()),
            ]),
        )
        .await
        .expect("begin a raw upload");
    driver
        .connection_upload_stream_chunk(raw_handle, raw_bytes.clone())
        .await
        .expect("append raw bytes");
    driver
        .connection_upload_stream_finish(None, raw_handle)
        .await
        .expect("finish the raw upload");
    let (stored_raw, _) = host
        .uploaded_blob("@db.sch.\"my stage\"/udfs/lib.zip")
        .expect("the raw upload keeps its destination name");
    assert_eq!(stored_raw, raw_bytes);
    assert_eq!(
        host.operation_count("list_stage_files"),
        listings_before_raw
    );

    // A destination that names no file is rejected before any bytes move.
    assert!(
        driver
            .connection_stage_upload_stream_begin(
                conn,
                "@mystage/dir/".to_string(),
                StageTransferOptions::default(),
            )
            .await
            .is_err()
    );

    // Raw PUT SQL gets no special handling: it is an ordinary query for the host.
    let calls_before_sql = host.operation_count("execute_query");
    let put_sql_stmt = driver.statement_new(conn).expect("statement handle");
    driver
        .statement_set_sql_query(put_sql_stmt, "PUT file:///tmp/x @st".to_string())
        .await
        .expect("set sql");
    let _ = driver
        .statement_execute_query(None, put_sql_stmt, None, None, None, None)
        .await;
    driver
        .statement_release(put_sql_stmt)
        .expect("release statement");
    assert_eq!(
        host.operation_count("execute_query"),
        calls_before_sql + 1,
        "PUT SQL must reach the host's execute_query like any other statement"
    );

    // Multi-chunk result set: the initial inline row comes back with the
    // query response, and the remaining row is fetched from a URL the host
    // never actually serves as a reachable location — only
    // `download_query_chunk` resolves it.
    host.set_query_response(
        r#"{
        "data": {
            "rowtype": [
                {"name": "N", "type": "fixed", "length": null, "scale": 0,
                 "precision": 1, "nullable": false}
            ],
            "rowset": [["1"]],
            "total": 2,
            "queryId": "01b2c3d4-0000-0000-0000-000000000099",
            "queryResultFormat": "json",
            "chunks": [
                {"url": "https://backend-chunk/0", "rowCount": 1,
                 "uncompressedSize": 8, "compressedSize": 8}
            ],
            "chunkHeaders": {"X-Chunk-Auth": "token-abc"}
        },
        "code": null,
        "message": null,
        "success": true
    }"#,
    );
    // A JSON chunk body is a sequence of row arrays with NO outer wrapping
    // array, e.g. `["2"]` for the single row `[ "2" ]` — not `[["2"]]"`.
    host.set_chunk_body("https://backend-chunk/0", br#"["2"]"#.to_vec());
    let chunk_stmt = driver.statement_new(conn).expect("statement handle");
    driver
        .statement_set_sql_query(chunk_stmt, "select n from two_rows".to_string())
        .await
        .expect("set sql");
    let chunk_result = driver
        .statement_execute_query(None, chunk_stmt, None, None, None, None)
        .await
        .expect("statement_execute_query should go through the host");
    let ExecuteQueryResult::Single { info, .. } = chunk_result else {
        panic!("expected a single result set");
    };
    let mut fetcher: AsyncArrowBatchFetcher = driver
        .result_set_get_async_stream(info.handle)
        .await
        .expect("building the async fetcher must not itself download anything yet");
    let mut batches = Vec::new();
    while let Some(batch) = fetcher
        .next_batch()
        .await
        .expect("the remote chunk must be fetched through the backend, not GS")
    {
        batches.push(batch);
    }
    driver
        .statement_release(chunk_stmt)
        .expect("release statement");

    let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    assert_eq!(
        total_rows, 2,
        "both the inline row and the backend-fetched chunk row must be present"
    );
    let rendered = batches
        .iter()
        .map(|b| format!("{:?}", b.column(0)))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        rendered.contains('1') && rendered.contains('2'),
        "expected both rows' values in the rendered batches, got: {rendered}"
    );
    let chunk_call = host
        .last_call("download_query_chunk")
        .expect("the remote chunk must be fetched via download_query_chunk");
    assert_eq!(
        chunk_call.subject.as_deref(),
        Some("https://backend-chunk/0")
    );
    let headers = host
        .chunk_headers("https://backend-chunk/0")
        .expect("headers_json must be forwarded");
    assert!(
        headers.contains("X-Chunk-Auth") && headers.contains("token-abc"),
        "got: {headers}"
    );

    driver.connection_release(conn).expect("release connection");
    driver.database_release(db).expect("release database");

    assert_eq!(sf_core_xp_backend_is_available(), 1);
}
