use crate::common::arrow_result_helper::{
    ArrowResultHelper, assert_record_batches_match, assert_schemas_match,
};
use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_return_arrow_even_if_json_result_set_is_returned_for_all_types() {
    run_arrow_and_json_and_match(
        "CREATE OR REPLACE TABLE json_result_set_simple_types (str_col STRING, tinyint_col TINYINT, smallint_col SMALLINT, int_col INT, bigint_col BIGINT, number_scale_0_col NUMBER(38, 0), number_scale_2_col NUMBER(38, 2), ntz TIMESTAMP_NTZ)",
        "INSERT INTO json_result_set_simple_types VALUES ('abc', 123, 12345, 1234567, 12345678901234567890, 12345678901234567890123456789012345678, 123.45, '2026-01-01 13:13:13')",
        "SELECT * FROM json_result_set_simple_types",
    );
}

#[test]
fn should_return_timestamp_ntz_as_arrow_even_if_json_result_set_is_returned() {
    run_arrow_and_json_and_match(
        "CREATE OR REPLACE TABLE json_result_set_timestamp_ntz (ntz0 TIMESTAMP_NTZ(0),\
            ntz1 TIMESTAMP_NTZ(1), ntz2 TIMESTAMP_NTZ(2), ntz3 TIMESTAMP_NTZ(3), ntz4 TIMESTAMP_NTZ(4), ntz5 TIMESTAMP_NTZ(5), ntz6 TIMESTAMP_NTZ(6), ntz7 TIMESTAMP_NTZ(7), ntz8 TIMESTAMP_NTZ(8), ntz9 TIMESTAMP_NTZ(9),\
            ntz1a TIMESTAMP_NTZ(1), ntz2a TIMESTAMP_NTZ(2), ntz3a TIMESTAMP_NTZ(3), ntz4a TIMESTAMP_NTZ(4), ntz5a TIMESTAMP_NTZ(5), ntz6a TIMESTAMP_NTZ(6), ntz7a TIMESTAMP_NTZ(7), ntz8a TIMESTAMP_NTZ(8), ntz9a TIMESTAMP_NTZ(9),\
            ntz1b TIMESTAMP_NTZ(1), ntz2b TIMESTAMP_NTZ(2), ntz3b TIMESTAMP_NTZ(3), ntz4b TIMESTAMP_NTZ(4), ntz5b TIMESTAMP_NTZ(5), ntz6b TIMESTAMP_NTZ(6), ntz7b TIMESTAMP_NTZ(7), ntz8b TIMESTAMP_NTZ(8), ntz9b TIMESTAMP_NTZ(9))",
        "INSERT INTO json_result_set_timestamp_ntz VALUES ('2026-01-01 10:10:10',\
            '2026-01-01 11:11:11.1', '2026-01-01 12:12:12.12', '2026-01-01 13:13:13.123', '2026-01-01 14:14:14.1234', '2026-01-01 15:15:15.12345', '2026-01-01 16:16:16.123456', '2026-01-01 17:17:17.1234567', '2026-01-01 18:18:18.12345678', '2026-01-01 19:19:19.123456789',\
            '2026-01-02 11:11:11', '2026-01-02 12:12:12', '2026-01-02 13:13:13', '2026-01-02 14:14:14', '2026-01-02 15:15:15', '2026-01-02 16:16:16', '2026-01-02 17:17:17', '2026-01-02 18:18:18', '2026-01-02 19:19:19',\
            '2026-01-02 11:11:11.1', '2026-01-02 12:12:12.02', '2026-01-03 13:13:13.003', '2026-01-02 14:14:14.0004', '2026-01-02 15:15:15.00005', '2026-01-02 16:16:16.000006', '2026-01-02 17:17:17.0000007', '2026-01-02 18:18:18.00000008', '2026-01-02 19:19:19.000000009')",
        "SELECT * FROM json_result_set_timestamp_ntz",
    )
}

fn run_arrow_and_json_and_match(create_table_query: &str, insert_query: &str, select_query: &str) {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stmt = client.new_statement();

    client.set_sql_query(&stmt, create_table_query);
    client.execute_statement_query(&stmt);

    client.set_sql_query(&stmt, "ALTER SESSION SET TIMEZONE = 'Pacific/Honolulu'");
    client.execute_statement_query(&stmt);

    client.set_sql_query(&stmt, insert_query);
    client.execute_statement_query(&stmt);

    client.set_sql_query(&stmt, select_query);
    let arrow_result = client.execute_statement_query(&stmt);

    client.set_sql_query(
        &stmt,
        "ALTER SESSION SET PYTHON_CONNECTOR_QUERY_RESULT_FORMAT = JSON",
    );
    let result = client.execute_statement_query(&stmt);
    assert_eq!(result.rows_affected(), 1, "Cannot force JSON result set");

    client.set_sql_query(&stmt, select_query);
    let json_result = client.execute_statement_query(&stmt);

    let mut arrow_result_helper = ArrowResultHelper::from_result(arrow_result);
    let mut json_result_helper = ArrowResultHelper::from_result(json_result);

    let arrow_schema = arrow_result_helper.schema();
    let json_schema = json_result_helper.schema();
    assert_schemas_match(&arrow_schema, &json_schema);

    let arrow_columns = arrow_result_helper.next_batch().unwrap();
    let json_columns = json_result_helper.next_batch().unwrap();

    assert_record_batches_match(&arrow_columns, &json_columns);

    client.release_statement(&stmt);
}
