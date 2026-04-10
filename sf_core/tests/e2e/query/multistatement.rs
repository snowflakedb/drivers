use sf_core::protobuf::generated::database_driver_v1::*;

use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::test_utils::unique_table_name;

#[test]
fn should_execute_multiple_select_statements() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When Multistatement query with 3 SELECTs is executed
    let sql = "SELECT 1 AS a; SELECT 2 AS b; SELECT 3 AS c";
    let result = client.execute_multistatement(sql, 3);

    // Then 3 result sets are returned
    let multi = unwrap_multi_result(result);
    assert_eq!(multi.query_ids.len(), 3, "Expected 3 child query IDs");

    // And each result set contains correct data
    for (i, query_id) in multi.query_ids.iter().enumerate() {
        let rs = client.connection_get_result_set(query_id);
        let mut helper = ArrowResultHelper::from_result(rs);
        let rows = helper.transform_into_array::<i64>().unwrap();
        assert_eq!(rows.len(), 1, "Result set {i} should have 1 row");
        assert_eq!(
            rows[0][0],
            (i + 1) as i64,
            "Result set {i} should contain value {}",
            i + 1
        );
    }
}

#[test]
fn should_execute_multiple_dml_statements() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When Multistatement query with CREATE TABLE, INSERT, and DROP is executed
    let table = unique_table_name("ms_dml_test");
    let sql = format!(
        "\
        CREATE OR REPLACE TEMPORARY TABLE {table}(id INT); \
        INSERT INTO {table} VALUES (1),(2),(3); \
        DROP TABLE {table}"
    );
    let result = client.execute_multistatement(&sql, 3);

    // Then 3 result sets are returned
    let multi = unwrap_multi_result(result);
    assert_eq!(multi.query_ids.len(), 3, "Expected 3 child query IDs");
}

#[test]
fn should_execute_mixed_statement_types() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When Multistatement query with ALTER SESSION, CREATE TABLE, INSERT, SELECT and DROP is executed
    let table = unique_table_name("ms_mix_test");
    let sql = format!(
        "\
        ALTER SESSION SET TIMEZONE='UTC'; \
        CREATE OR REPLACE TEMPORARY TABLE {table}(val TEXT); \
        INSERT INTO {table} VALUES ('hello'); \
        SELECT val FROM {table};\
        DROP TABLE {table}"
    );
    let result = client.execute_multistatement(&sql, 5);

    // Then 5 result sets are returned
    let multi = unwrap_multi_result(result);
    assert_eq!(multi.query_ids.len(), 5, "Expected 5 child query IDs");

    // And the SELECT result contains expected data
    let select_rs = client.connection_get_result_set(&multi.query_ids[3]);
    let mut helper = ArrowResultHelper::from_result(select_rs);
    let rows = helper.transform_into_array::<String>().unwrap();
    assert_eq!(rows.len(), 1, "SELECT should return 1 row");
    assert_eq!(rows[0][0], "hello");
}

#[test]
fn should_fail_when_multistatement_sql_is_sent_without_multi_statement_count() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When Multistatement SQL is executed without configuring multi_statement_count
    let sql = "SELECT 1; SELECT 2; SELECT 3";
    let result = client.execute_query_no_unwrap(sql);

    // Then an error is returned indicating multi-statement is not enabled
    assert!(
        result.is_err(),
        "Expected error when executing multi-statement without multi_statement_count"
    );
    let err = result.unwrap_err();
    assert!(err.contains("Actual statement count 3 did not match the desired statement count 1"));
}

#[test]
fn should_fail_when_multi_statement_count_does_not_match_actual_statement_count() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When Single SELECT is executed with multi_statement_count set to 3
    let sql = "SELECT 1";
    let result = client.execute_multistatement_no_unwrap(sql, 3);

    // Then an error is returned indicating statement count mismatch
    assert!(
        result.is_err(),
        "Expected error when multi_statement_count doesn't match actual count"
    );
}

fn unwrap_multi_result(result: execute_query_response::Result) -> MultiStatementResult {
    match result {
        execute_query_response::Result::Multi(m) => m,
        other => panic!("Expected multi-statement result, got: {other:?}"),
    }
}
