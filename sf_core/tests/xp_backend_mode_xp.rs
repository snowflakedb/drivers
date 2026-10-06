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
use sf_core::apis::database_driver_v1::{DatabaseDriverV1, ExecuteQueryResult};
#[cfg(target_os = "linux")]
use sf_core::config::rest_parameters::test_fixtures::test_client_info;
#[cfg(target_os = "linux")]
use sf_core::config::settings::Setting;
#[cfg(target_os = "linux")]
use sf_core::crl::CrlManager;
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
    XP_BACKEND_ABI_VERSION, XpCancelRequest, XpQueryRequest, sf_core_register_xp_backend,
    sf_core_xp_backend_is_available,
};

#[cfg(target_os = "linux")]
use fake_host::{HostState, callbacks_for};
#[cfg(target_os = "linux")]
use fixtures::{
    MONITORING_RUNNING_RESPONSE, SECRET_PASSWORD, SELECT_ONE_RESPONSE, SESSION_INFO,
    UNREACHABLE_SERVER_URL, login_parameters, query_parameters, set_running_inside_xp,
};

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "current_thread")]
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
        &client,
        &login_parameters(),
        Some(&HashMap::from([(
            "QUERY_TAG".to_string(),
            "sproc".to_string(),
        )])),
        None,
        None,
        &Default::default(),
        None,
        xp_backend,
        CrlManager::new(),
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
        UNREACHABLE_SERVER_URL,
        &test_client_info(),
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
    driver.connection_release(conn).expect("release connection");
    driver.database_release(db).expect("release database");

    assert_eq!(sf_core_xp_backend_is_available(), 1);
}
