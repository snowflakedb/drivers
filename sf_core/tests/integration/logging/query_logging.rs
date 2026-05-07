//! Integration tests for query log gating (sf_core/SNOW-XXXX).
//!
//! Verifies that `log_query_text` / `log_query_parameters` toggle whether the
//! INFO-level query log line includes the (truncated) SQL and JSON bindings.
//! Logs are captured via [`tracing_test::internal::MockWriter`] using a
//! thread-local subscriber so this test does not race with `setup_logging`'s
//! global subscriber.

use std::sync::Mutex;

use serde_json::json;
use sf_core::config::rest_parameters::test_fixtures::test_client_info;
use sf_core::config::rest_parameters::{DEFAULT_LOG_MAX_QUERY_LENGTH, QueryParameters};
use sf_core::config::retry::RetryPolicy;
use sf_core::rest::snowflake::{QueryExecutionMode, QueryInput, snowflake_query_with_client};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Build a thread-local capturing subscriber so log assertions don't race
/// with any global subscriber installed by other tests in the same binary.
fn capturing_subscriber() -> (tracing::subscriber::DefaultGuard, &'static Mutex<Vec<u8>>) {
    let buf: &'static Mutex<Vec<u8>> = Box::leak(Box::new(Mutex::new(Vec::new())));
    let mock_writer = tracing_test::internal::MockWriter::new(buf);
    let dispatch = tracing_test::internal::get_subscriber(mock_writer, "trace");
    let guard = tracing::dispatcher::set_default(&dispatch);
    (guard, buf)
}

fn captured_logs(buf: &Mutex<Vec<u8>>) -> String {
    String::from_utf8(buf.lock().unwrap().clone()).unwrap_or_default()
}

fn query_params(
    server_url: &str,
    log_query_text: bool,
    log_query_parameters: bool,
) -> QueryParameters {
    QueryParameters {
        server_url: server_url.to_string(),
        client_info: test_client_info(),
        log_max_query_length: DEFAULT_LOG_MAX_QUERY_LENGTH,
        log_query_text,
        log_query_parameters,
    }
}

async fn mount_query_success(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/queries/v1/query-request"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "queryId": "01abcdef-0000-0000-0000-000000000000",
                "rowtype": [],
                "rowset": [],
            }
        })))
        .mount(server)
        .await;
}

const SQL_LONG: &str =
    "SELECT a_very_distinctive_marker_token FROM information_schema.tables WHERE 1=2";

#[tokio::test]
async fn sync_query_emits_info_log_without_sql_when_flag_off() {
    let (_guard, log_buf) = capturing_subscriber();

    let server = MockServer::start().await;
    mount_query_success(&server).await;

    let client = reqwest::Client::new();
    let result = snowflake_query_with_client(
        &client,
        query_params(&server.uri(), false, false),
        "test-token",
        QueryInput::new(SQL_LONG),
        &RetryPolicy::default(),
        QueryExecutionMode::Blocking,
    )
    .await;

    assert!(result.is_ok(), "Query should succeed");

    let logs = captured_logs(log_buf);
    assert!(
        logs.contains("Executing sync query"),
        "Expected the sync entry log to be emitted, got:\n{logs}"
    );
    assert!(
        logs.contains("INFO"),
        "Sync entry log should be at INFO level, got:\n{logs}"
    );
    assert!(
        !logs.contains("a_very_distinctive_marker_token"),
        "SQL text must NOT appear when log_query_text=false, got:\n{logs}"
    );
    assert!(
        !logs.contains("bindings"),
        "Bindings field must NOT appear when log_query_text=false, got:\n{logs}"
    );
}

#[tokio::test]
async fn sync_query_emits_info_log_with_sql_when_text_flag_on() {
    let (_guard, log_buf) = capturing_subscriber();

    let server = MockServer::start().await;
    mount_query_success(&server).await;

    let client = reqwest::Client::new();
    // Build a SQL longer than DEFAULT_LOG_MAX_QUERY_LENGTH so we can confirm
    // truncation happens at the configured boundary.
    let result = snowflake_query_with_client(
        &client,
        query_params(&server.uri(), true, false),
        "test-token",
        QueryInput::new(SQL_LONG),
        &RetryPolicy::default(),
        QueryExecutionMode::Blocking,
    )
    .await;

    assert!(result.is_ok(), "Query should succeed");

    let logs = captured_logs(log_buf);
    assert!(
        logs.contains("Executing sync query"),
        "Expected the sync entry log to be emitted, got:\n{logs}"
    );
    let truncated: String = SQL_LONG
        .chars()
        .take(DEFAULT_LOG_MAX_QUERY_LENGTH)
        .collect();
    assert!(
        logs.contains(&truncated),
        "Truncated SQL prefix should appear when log_query_text=true.\nExpected prefix: {truncated}\nGot logs:\n{logs}"
    );
    assert!(
        !logs.contains("bindings="),
        "Bindings field must NOT appear when log_query_parameters=false, got:\n{logs}"
    );
}

#[tokio::test]
async fn sync_query_emits_info_log_with_sql_and_bindings_when_both_flags_on() {
    let (_guard, log_buf) = capturing_subscriber();

    let server = MockServer::start().await;
    mount_query_success(&server).await;

    let bindings_json = r#"{"1":{"type":"TEXT","value":"sentinel_binding_value"}}"#;
    let bindings: Box<serde_json::value::RawValue> =
        serde_json::value::RawValue::from_string(bindings_json.to_string()).unwrap();
    let mut input = QueryInput::new("SELECT ?");
    input.bindings = Some(&bindings);

    let client = reqwest::Client::new();
    let result = snowflake_query_with_client(
        &client,
        query_params(&server.uri(), true, true),
        "test-token",
        input,
        &RetryPolicy::default(),
        QueryExecutionMode::Blocking,
    )
    .await;

    assert!(result.is_ok(), "Query should succeed");

    let logs = captured_logs(log_buf);
    assert!(
        logs.contains("Executing sync query"),
        "Expected the sync entry log to be emitted, got:\n{logs}"
    );
    assert!(
        logs.contains("SELECT ?"),
        "SQL prefix should appear when log_query_text=true, got:\n{logs}"
    );
    assert!(
        logs.contains("sentinel_binding_value"),
        "Bindings JSON should appear when log_query_parameters=true, got:\n{logs}"
    );
}

#[tokio::test]
async fn async_submit_emits_info_log_with_sql_when_text_flag_on() {
    let (_guard, log_buf) = capturing_subscriber();

    let server = MockServer::start().await;
    mount_query_success(&server).await;

    let client = reqwest::Client::new();
    let result = snowflake_query_with_client(
        &client,
        query_params(&server.uri(), true, false),
        "test-token",
        QueryInput::new(SQL_LONG),
        &RetryPolicy::default(),
        QueryExecutionMode::Async,
    )
    .await;

    assert!(result.is_ok(), "Async query should succeed");

    let logs = captured_logs(log_buf);
    assert!(
        logs.contains("Executing async query"),
        "Expected the async entry log to be emitted, got:\n{logs}"
    );
    let truncated: String = SQL_LONG
        .chars()
        .take(DEFAULT_LOG_MAX_QUERY_LENGTH)
        .collect();
    assert!(
        logs.contains(&truncated),
        "Truncated SQL prefix should appear in async entry log, got:\n{logs}"
    );
}
