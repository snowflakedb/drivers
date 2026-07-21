//! E2E tests for QUERY_TAG.
//!
//! These tests implement scenarios from shared/query/query_tag.feature.
//! QUERY_TAG can be set at the connection level (session parameter, tagging
//! every query) or per-statement (tagging only that query without mutating
//! session state).

use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_tag_queries_when_query_tag_is_set_at_connection_level() {
    // Given Snowflake client is logged in with connection option QUERY_TAG set to "conn_tag_e2e"
    let client = SnowflakeTestClient::with_default_jwt_auth_params();
    client.set_connection_option("QUERY_TAG", "conn_tag_e2e");
    client.connect().unwrap();

    // When Query "SELECT CURRENT_QUERY_TAG()" is executed
    let result = client.execute_query("SELECT CURRENT_QUERY_TAG()");

    // Then the result should contain value "conn_tag_e2e"
    let mut helper = ArrowResultHelper::from_result(result);
    let rows = helper.transform_into_array::<String>().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "conn_tag_e2e");
}

#[test]
fn should_tag_a_single_query_via_statement_level_query_tag() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::with_default_jwt_auth_params();
    client.connect().unwrap();

    // When Query "SELECT CURRENT_QUERY_TAG()" is executed with statement-level QUERY_TAG "stmt_tag_e2e"
    let result = client.execute_query_with_statement_params(
        "SELECT CURRENT_QUERY_TAG()",
        &[("QUERY_TAG", "stmt_tag_e2e")],
    );

    // Then the result should contain value "stmt_tag_e2e"
    let mut helper = ArrowResultHelper::from_result(result);
    let rows = helper.transform_into_array::<String>().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "stmt_tag_e2e");
}

#[test]
fn should_not_leak_statement_level_query_tag_into_session_state() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::with_default_jwt_auth_params();
    client.connect().unwrap();

    // When Query "SELECT CURRENT_QUERY_TAG()" is executed with statement-level QUERY_TAG "stmt_tag_e2e"
    client.execute_query_with_statement_params(
        "SELECT CURRENT_QUERY_TAG()",
        &[("QUERY_TAG", "stmt_tag_e2e")],
    );

    // And Query "SELECT CURRENT_QUERY_TAG()" is executed without a statement-level tag
    let result = client.execute_query("SELECT CURRENT_QUERY_TAG()");

    // Then the last result should contain empty value
    let mut helper = ArrowResultHelper::from_result(result);
    let rows = helper.transform_into_array::<String>().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "");
}
