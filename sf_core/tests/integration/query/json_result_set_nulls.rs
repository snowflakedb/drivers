use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::test_utils::{TableCleanupGuard, unique_table_name};
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
    let table = unique_table_name("json_null_test");
    let client = SnowflakeTestClient::connect_with_default_auth();
    let _guard = TableCleanupGuard::new(table.clone(), |name| {
        let stmt = client.new_statement();
        client.set_sql_query(&stmt, &format!("DROP TABLE IF EXISTS {name}"));
        client.execute_statement_query(&stmt);
        client.release_statement(&stmt);
    });
    let stmt = client.new_statement();

    // Create table with nullable columns of various types
    client.set_sql_query(
        &stmt,
        &format!(
            "CREATE OR REPLACE TEMPORARY TABLE {table} (
            str_col STRING,
            tinyint_col TINYINT,
            smallint_col SMALLINT,
            int_col INT,
            bigint_col BIGINT,
            num_col NUMBER(38, 0),
            num_scale_col NUMBER(38, 2),
            bool_col BOOLEAN,
            real_col DOUBLE,
            date_col DATE,
            ntz_col TIMESTAMP_NTZ(3),
            ntz_hi_col TIMESTAMP_NTZ(9),
            ltz_col TIMESTAMP_LTZ(3),
            ltz_hi_col TIMESTAMP_LTZ(9),
            tz_col TIMESTAMP_TZ(3),
            tz_hi_col TIMESTAMP_TZ(9),
            time_col TIME(3),
            bin_col BINARY,
            variant_col VARIANT,
            object_col OBJECT,
            array_col ARRAY,
            decfloat_col DECFLOAT
        )"
        ),
    );
    client.execute_statement_query(&stmt);

    client.set_sql_query(
        &stmt,
        &format!(
            "INSERT INTO {table}
            SELECT 'hello', 42, 1234, 123456, 1234567890123, 12345678901234567890123456789012345678, 123.45,
                true, 3.14, '2024-01-15', '2024-01-15 10:30:00.123',
                '2024-01-15 10:30:00.123456789',
                '2024-01-15 10:30:00.123', '2024-01-15 10:30:00.123456789',
                '2024-01-15 10:30:00.123 +01:00', '2024-01-15 10:30:00.123456789 +01:00',
                '10:30:00.123',
                TO_BINARY('hello', 'UTF-8'),
                TO_VARIANT('test'), PARSE_JSON('{{\"k\": 1}}'), PARSE_JSON('[1, 2]'),
                123.456::DECFLOAT
            UNION ALL
            SELECT null, null, null, null, null, null, null,
                null, null, null, null,
                null,
                null, null,
                null, null,
                null,
                null,
                null, null, null,
                null
            UNION ALL
            SELECT 'world', 7, 567, 78901, 9876543210987, 98765432109876543210987654321098765432, 789.01,
                false, 2.71, '2024-06-01', '2024-06-01 12:00:00.456',
                '2024-06-01 12:00:00.456789012',
                '2024-06-01 12:00:00.456', '2024-06-01 12:00:00.456789012',
                '2024-06-01 12:00:00.456 +02:00', '2024-06-01 12:00:00.456789012 +02:00',
                '12:00:00.456',
                TO_BINARY('world', 'UTF-8'),
                TO_VARIANT(123), PARSE_JSON('{{\"k\": 2}}'), PARSE_JSON('[3, 4]'),
                789.012::DECFLOAT"
        ),
    );
    client.execute_statement_query(&stmt);
    client.release_statement(&stmt);

    let mut helper = execute_json_query(
        &client,
        &format!("SELECT * FROM {table} ORDER BY str_col NULLS FIRST"),
    );

    let batch = helper.next_batch().expect("Expected a record batch");
    assert_eq!(batch.num_rows(), 3);
    assert_eq!(batch.num_columns(), 22);

    for col_idx in 0..batch.num_columns() {
        assert!(
            batch.column(col_idx).is_null(0),
            "Column {} row 0 should be null",
            batch.schema().field(col_idx).name()
        );
    }

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
    let table = unique_table_name("json_null_parity");
    let client = SnowflakeTestClient::connect_with_default_auth();
    let _guard = TableCleanupGuard::new(table.clone(), |name| {
        let stmt = client.new_statement();
        client.set_sql_query(&stmt, &format!("DROP TABLE IF EXISTS {name}"));
        client.execute_statement_query(&stmt);
        client.release_statement(&stmt);
    });
    let stmt = client.new_statement();

    client.set_sql_query(
        &stmt,
        &format!(
            "CREATE OR REPLACE TABLE {table} (
            txt STRING,
            num INTEGER,
            ntz TIMESTAMP_NTZ
        )"
        ),
    );
    client.execute_statement_query(&stmt);

    client.set_sql_query(
        &stmt,
        &format!(
            "INSERT INTO {table} VALUES
            ('a', 1, '2024-01-01 00:00:00'),
            (null, 2, '2024-01-02 00:00:00'),
            ('c', null, '2024-01-03 00:00:00'),
            ('d', 4, null)"
        ),
    );
    client.execute_statement_query(&stmt);

    client.set_sql_query(
        &stmt,
        &format!("SELECT * FROM {table} ORDER BY txt NULLS FIRST"),
    );
    let arrow_result = client.execute_statement_query(&stmt);
    let mut arrow_helper = ArrowResultHelper::from_result(arrow_result);
    let arrow_batch = arrow_helper.next_batch().expect("Expected arrow batch");

    client.set_sql_query(
        &stmt,
        "ALTER SESSION SET PYTHON_CONNECTOR_QUERY_RESULT_FORMAT = JSON",
    );
    client.execute_statement_query(&stmt);

    client.set_sql_query(
        &stmt,
        &format!("SELECT * FROM {table} ORDER BY txt NULLS FIRST"),
    );
    let json_result = client.execute_statement_query(&stmt);
    let mut json_helper = ArrowResultHelper::from_result(json_result);
    let json_batch = json_helper.next_batch().expect("Expected json batch");

    assert_eq!(arrow_batch.num_rows(), json_batch.num_rows());
    assert_eq!(arrow_batch.num_columns(), json_batch.num_columns());

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
