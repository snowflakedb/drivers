use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use arrow::array::Array;

/// Forces the session to return JSON result format and executes the given query.
fn execute_json_query(client: &SnowflakeTestClient, query: &str) -> ArrowResultHelper {
    let stmt = client.new_statement();

    client.set_sql_query(
        &stmt,
        "ALTER SESSION SET PYTHON_CONNECTOR_QUERY_RESULT_FORMAT = JSON",
    );
    let result = client.execute_statement_query(&stmt);
    assert_eq!(result.rows_affected(), 1, "Cannot force JSON result set");

    client.set_sql_query(&stmt, query);
    let result = client.execute_statement_query(&stmt);
    let helper = ArrowResultHelper::from_result(result);
    client.release_statement(&stmt);
    helper
}

#[test]
fn should_handle_null_values_in_json_result_set() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stmt = client.new_statement();

    // Create table with nullable columns of various types
    client.set_sql_query(
        &stmt,
        "CREATE OR REPLACE TABLE json_null_test (
            str_col STRING,
            int_col INTEGER,
            bool_col BOOLEAN,
            real_col DOUBLE,
            date_col DATE,
            ntz_col TIMESTAMP_NTZ(3)
        )",
    );
    client.execute_statement_query(&stmt);

    // Insert rows with null values in different columns
    client.set_sql_query(
        &stmt,
        "INSERT INTO json_null_test VALUES
            ('hello', 42, true, 3.14, '2024-01-15', '2024-01-15 10:30:00.123'),
            (null, null, null, null, null, null),
            ('world', 7, false, 2.71, '2024-06-01', '2024-06-01 12:00:00.456')",
    );
    client.execute_statement_query(&stmt);
    client.release_statement(&stmt);

    // Query with JSON format
    let mut helper = execute_json_query(
        &client,
        "SELECT * FROM json_null_test ORDER BY str_col NULLS FIRST",
    );

    let batch = helper.next_batch().expect("Expected a record batch");
    assert_eq!(batch.num_rows(), 3);
    assert_eq!(batch.num_columns(), 6);

    // First row should be all nulls (NULLS FIRST ordering)
    for col_idx in 0..batch.num_columns() {
        assert!(
            batch.column(col_idx).is_null(0),
            "Column {} row 0 should be null",
            batch.schema().field(col_idx).name()
        );
    }

    // Second and third rows should have valid values
    for col_idx in 0..batch.num_columns() {
        assert!(
            batch.column(col_idx).is_valid(1),
            "Column {} row 1 should be valid",
            batch.schema().field(col_idx).name()
        );
        assert!(
            batch.column(col_idx).is_valid(2),
            "Column {} row 2 should be valid",
            batch.schema().field(col_idx).name()
        );
    }
}

#[test]
fn should_handle_show_schemas_json_result_with_nulls() {
    let client = SnowflakeTestClient::connect_with_default_auth();

    // SHOW SCHEMAS returns JSON format with nullable columns like comment, options
    let mut helper = execute_json_query(&client, "SHOW SCHEMAS");

    let batch = helper.next_batch().expect("Expected a record batch");
    assert!(batch.num_rows() > 0, "Expected at least one schema");
    assert!(
        batch.num_columns() > 0,
        "Expected at least one column in SHOW SCHEMAS result"
    );
}

#[test]
fn should_match_null_positions_between_arrow_and_json_formats() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stmt = client.new_statement();

    // Create table with sparse nulls across different types
    client.set_sql_query(
        &stmt,
        "CREATE OR REPLACE TABLE json_null_parity (
            txt STRING,
            num INTEGER,
            ntz TIMESTAMP_NTZ
        )",
    );
    client.execute_statement_query(&stmt);

    client.set_sql_query(
        &stmt,
        "INSERT INTO json_null_parity VALUES
            ('a', 1, '2024-01-01 00:00:00'),
            (null, 2, '2024-01-02 00:00:00'),
            ('c', null, '2024-01-03 00:00:00'),
            ('d', 4, null)",
    );
    client.execute_statement_query(&stmt);

    // Get Arrow result
    client.set_sql_query(
        &stmt,
        "SELECT * FROM json_null_parity ORDER BY txt NULLS FIRST",
    );
    let arrow_result = client.execute_statement_query(&stmt);
    let mut arrow_helper = ArrowResultHelper::from_result(arrow_result);
    let arrow_batch = arrow_helper.next_batch().expect("Expected arrow batch");

    // Get JSON result
    client.set_sql_query(
        &stmt,
        "ALTER SESSION SET PYTHON_CONNECTOR_QUERY_RESULT_FORMAT = JSON",
    );
    client.execute_statement_query(&stmt);

    client.set_sql_query(
        &stmt,
        "SELECT * FROM json_null_parity ORDER BY txt NULLS FIRST",
    );
    let json_result = client.execute_statement_query(&stmt);
    let mut json_helper = ArrowResultHelper::from_result(json_result);
    let json_batch = json_helper.next_batch().expect("Expected json batch");

    // Verify both have the same number of rows
    assert_eq!(arrow_batch.num_rows(), json_batch.num_rows());
    assert_eq!(arrow_batch.num_columns(), json_batch.num_columns());

    // Verify null positions match between Arrow and JSON-converted-to-Arrow
    for col_idx in 0..arrow_batch.num_columns() {
        let arrow_col = arrow_batch.column(col_idx);
        let json_col = json_batch.column(col_idx);
        let field_name = arrow_batch.schema().field(col_idx).name().clone();

        assert_eq!(
            arrow_col.null_count(),
            json_col.null_count(),
            "Null count mismatch for column '{field_name}'"
        );

        for row_idx in 0..arrow_batch.num_rows() {
            assert_eq!(
                arrow_col.is_null(row_idx),
                json_col.is_null(row_idx),
                "Null mismatch at row {row_idx} for column '{field_name}'"
            );
        }
    }

    client.release_statement(&stmt);
}
