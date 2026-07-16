//! E2E tests verifying that `log_query_text` / `log_query_parameters` gate
//! whether SQL and bind values appear in logs during real query execution.
//!
//! A thread-local capturing subscriber is used so these tests don't race with
//! the global subscriber installed by `setup_logging`.

use std::sync::Mutex;

use crate::common::snowflake_test_client::SnowflakeTestClient;

/// Install a thread-local capturing subscriber and return its guard and buffer.
///
/// The guard must be kept alive for the duration of the test; dropping it
/// restores the previous dispatcher.
fn capturing_subscriber() -> (tracing::subscriber::DefaultGuard, &'static Mutex<Vec<u8>>) {
    let buf: &'static Mutex<Vec<u8>> = Box::leak(Box::new(Mutex::new(Vec::new())));
    let mock_writer = tracing_test::internal::MockWriter::new(buf);
    let dispatch = tracing_test::internal::get_subscriber(mock_writer, "info");
    let guard = tracing::dispatcher::set_default(&dispatch);
    (guard, buf)
}

fn captured_logs(buf: &Mutex<Vec<u8>>) -> String {
    String::from_utf8(buf.lock().unwrap().clone()).unwrap_or_default()
}

/// Sentinel SQL text and binding value chosen to be distinctive enough that
/// a false positive from an unrelated log line is effectively impossible.
const SENTINEL_SQL: &str = "SELECT ? AS log_gate_e2e_sentinel";
const SENTINEL_BINDING_VALUE: &str = "log_gate_e2e_binding_42";

fn sentinel_binding_json() -> String {
    // {"1":{"type":"TEXT","value":"<sentinel>"}}
    format!(r#"{{"1":{{"type":"TEXT","value":"{SENTINEL_BINDING_VALUE}"}}}}"#)
}

#[test]
fn should_omit_sql_and_bindings_from_logs_when_log_query_text_is_disabled_by_default() {
    // Given a capturing subscriber is active for this thread
    let (_guard, log_buf) = capturing_subscriber();

    // And a Snowflake client connected with default settings (log_query_text not set)
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When a parameterized query is executed
    let stmt = client.new_statement();
    client.set_sql_query(&stmt, SENTINEL_SQL);
    client.execute_statement_query_with_bindings(&stmt, Some(&sentinel_binding_json()));
    client.release_statement(&stmt);

    // Then the SQL text must NOT appear in any log line
    let logs = captured_logs(log_buf);
    assert!(
        !logs.contains("log_gate_e2e_sentinel"),
        "SQL text must not appear in logs when log_query_text is not enabled, got:\n{logs}"
    );

    // And the binding value must NOT appear in any log line
    assert!(
        !logs.contains(SENTINEL_BINDING_VALUE),
        "Binding value must not appear in logs when log_query_text is not enabled, got:\n{logs}"
    );
}

#[test]
fn should_include_sql_and_bindings_in_logs_when_log_query_text_and_log_query_parameters_are_enabled()
 {
    // Given a capturing subscriber is active for this thread
    let (_guard, log_buf) = capturing_subscriber();

    // And a Snowflake client with log_query_text and log_query_parameters enabled
    // Options must be set before connect() — after connection_init the session is live.
    let client = SnowflakeTestClient::with_default_jwt_auth_params();
    client.set_connection_option("log_query_text", "true");
    client.set_connection_option("log_query_parameters", "true");
    client.connect().unwrap();

    // When a parameterized query is executed
    let stmt = client.new_statement();
    client.set_sql_query(&stmt, SENTINEL_SQL);
    client.execute_statement_query_with_bindings(&stmt, Some(&sentinel_binding_json()));
    client.release_statement(&stmt);

    // Then the SQL text must appear in at least one log line
    let logs = captured_logs(log_buf);
    assert!(
        logs.contains("log_gate_e2e_sentinel"),
        "SQL text must appear in logs when log_query_text=true, got:\n{logs}"
    );

    // And the binding value must appear in at least one log line
    assert!(
        logs.contains(SENTINEL_BINDING_VALUE),
        "Binding value must appear in logs when log_query_parameters=true, got:\n{logs}"
    );
}
