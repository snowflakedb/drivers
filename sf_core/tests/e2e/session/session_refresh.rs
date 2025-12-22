//! E2E tests for session token management and refresh.
//!
//! Note: Snowflake's session token lifetime is controlled by the server and typically
//! lasts for hours. Testing actual session refresh requires either:
//! - Waiting for the session to naturally expire (not practical for CI)
//! - Using a test account with configured short session timeout
//! - Manually invalidating the session token (not supported via API)
//!
//! These tests verify basic session management works correctly. The integration
//! tests in `tests/integration/http/session_refresh.rs` cover the refresh logic
//! using mock servers.

use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_maintain_session_across_multiple_queries() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When we execute multiple queries
    for i in 1..=3 {
        let stmt = client.new_statement();
        let sql = format!("SELECT {} AS query_num", i);
        client.set_sql_query(&stmt, &sql);
        let result = client.execute_statement_query(&stmt);

        // Then each query should succeed with the correct result
        let mut helper = ArrowResultHelper::from_result(result);
        let rows = helper.transform_into_array::<i64>().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], i as i64);

        client.release_statement(&stmt);
    }
}

#[test]
fn should_execute_queries_with_delay_between_them() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When we execute queries with delays between them
    for i in 1..=2 {
        let stmt = client.new_statement();
        let sql = format!("SELECT {} AS seq", i);
        client.set_sql_query(&stmt, &sql);
        let result = client.execute_statement_query(&stmt);

        // Then each query should succeed
        let mut helper = ArrowResultHelper::from_result(result);
        let rows = helper.transform_into_array::<i64>().unwrap();
        assert_eq!(rows[0][0], i as i64);

        client.release_statement(&stmt);

        // Short delay between queries - session should remain valid
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
