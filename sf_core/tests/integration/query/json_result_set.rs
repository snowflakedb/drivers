use crate::common::arrow_result_helper::{
    ArrowResultHelper, assert_record_batches_match, assert_schemas_match,
};
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::test_utils::{TableCleanupGuard, unique_table_name};

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

#[test]
fn should_return_timestamp_ltz_as_arrow_even_if_json_result_set_is_returned() {
    run_arrow_and_json_and_match(
        "CREATE OR REPLACE TABLE json_result_set_timestamp_ltz (LTZ0 TIMESTAMP_LTZ(0),\
            LTZ1 TIMESTAMP_LTZ(1), LTZ2 TIMESTAMP_LTZ(2), LTZ3 TIMESTAMP_LTZ(3), LTZ4 TIMESTAMP_LTZ(4), LTZ5 TIMESTAMP_LTZ(5), LTZ6 TIMESTAMP_LTZ(6), LTZ7 TIMESTAMP_LTZ(7), LTZ8 TIMESTAMP_LTZ(8), LTZ9 TIMESTAMP_LTZ(9),\
            LTZ1a TIMESTAMP_LTZ(1), LTZ2a TIMESTAMP_LTZ(2), LTZ3a TIMESTAMP_LTZ(3), LTZ4a TIMESTAMP_LTZ(4), LTZ5a TIMESTAMP_LTZ(5), LTZ6a TIMESTAMP_LTZ(6), LTZ7a TIMESTAMP_LTZ(7), LTZ8a TIMESTAMP_LTZ(8), LTZ9a TIMESTAMP_LTZ(9),\
            LTZ1b TIMESTAMP_LTZ(1), LTZ2b TIMESTAMP_LTZ(2), LTZ3b TIMESTAMP_LTZ(3), LTZ4b TIMESTAMP_LTZ(4), LTZ5b TIMESTAMP_LTZ(5), LTZ6b TIMESTAMP_LTZ(6), LTZ7b TIMESTAMP_LTZ(7), LTZ8b TIMESTAMP_LTZ(8), LTZ9b TIMESTAMP_LTZ(9))",
        "INSERT INTO json_result_set_timestamp_ltz VALUES ('2026-01-01 10:10:10',\
            '2026-01-01 11:11:11.1', '2026-01-01 12:12:12.12', '2026-01-01 13:13:13.123', '2026-01-01 14:14:14.1234', '2026-01-01 15:15:15.12345', '2026-01-01 16:16:16.123456', '2026-01-01 17:17:17.1234567', '2026-01-01 18:18:18.12345678', '2026-01-01 19:19:19.123456789',\
            '2026-01-02 11:11:11', '2026-01-02 12:12:12', '2026-01-02 13:13:13', '2026-01-02 14:14:14', '2026-01-02 15:15:15', '2026-01-02 16:16:16', '2026-01-02 17:17:17', '2026-01-02 18:18:18', '2026-01-02 19:19:19',\
            '2026-01-02 11:11:11.1', '2026-01-02 12:12:12.02', '2026-01-03 13:13:13.003', '2026-01-02 14:14:14.0004', '2026-01-02 15:15:15.00005', '2026-01-02 16:16:16.000006', '2026-01-02 17:17:17.0000007', '2026-01-02 18:18:18.00000008', '2026-01-02 19:19:19.000000009')",
        "SELECT * FROM json_result_set_timestamp_ltz",
    )
}

#[test]
fn should_return_timestamp_tz_as_arrow_even_if_json_result_set_is_returned() {
    run_arrow_and_json_and_match(
        "CREATE OR REPLACE TABLE json_result_set_timestamp_tz (TZ0 TIMESTAMP_TZ(0),\
            TZ1 TIMESTAMP_TZ(1), TZ2 TIMESTAMP_TZ(2), TZ3 TIMESTAMP_TZ(3), TZ4 TIMESTAMP_TZ(4), TZ5 TIMESTAMP_TZ(5), TZ6 TIMESTAMP_TZ(6), TZ7 TIMESTAMP_TZ(7), TZ8 TIMESTAMP_TZ(8), TZ9 TIMESTAMP_TZ(9),\
            TZ1a TIMESTAMP_TZ(1), TZ2a TIMESTAMP_TZ(2), TZ3a TIMESTAMP_TZ(3), TZ4a TIMESTAMP_TZ(4), TZ5a TIMESTAMP_TZ(5), TZ6a TIMESTAMP_TZ(6), TZ7a TIMESTAMP_TZ(7), TZ8a TIMESTAMP_TZ(8), TZ9a TIMESTAMP_TZ(9),\
            TZ1b TIMESTAMP_TZ(1), TZ2b TIMESTAMP_TZ(2), TZ3b TIMESTAMP_TZ(3), TZ4b TIMESTAMP_TZ(4), TZ5b TIMESTAMP_TZ(5), TZ6b TIMESTAMP_TZ(6), TZ7b TIMESTAMP_TZ(7), TZ8b TIMESTAMP_TZ(8), TZ9b TIMESTAMP_TZ(9))",
        "INSERT INTO json_result_set_timestamp_tz VALUES ('2026-01-01 10:10:10',\
            '2026-01-01 11:11:11.1 +01:00', '2026-01-01 12:12:12.12+02:00', '2026-01-01 13:13:13.123 +03:00', '2026-01-01 14:14:14.1234 +04:00', '2026-01-01 15:15:15.12345 +05:00', '2026-01-01 16:16:16.123456 +06:00', '2026-01-01 17:17:17.1234567 +07:00', '2026-01-01 18:18:18.12345678 +08:00', '2026-01-01 19:19:19.123456789 +09:00',\
            '2026-01-02 11:11:11 +01:00', '2026-01-02 12:12:12 +02:00', '2026-01-02 13:13:13 +03:00', '2026-01-02 14:14:14 +04:00', '2026-01-02 15:15:15 +05:00', '2026-01-02 16:16:16 +06:00', '2026-01-02 17:17:17 +07:00', '2026-01-02 18:18:18 +08:00', '2026-01-02 19:19:19 +09:00',\
            '2026-01-02 11:11:11.1 +01:00', '2026-01-02 12:12:12.02 +02:00', '2026-01-03 13:13:13.003 +03:00', '2026-01-02 14:14:14.0004 +04:00', '2026-01-02 15:15:15.00005 +05:00', '2026-01-02 16:16:16.000006 +06:00', '2026-01-02 17:17:17.0000007 +07:00', '2026-01-02 18:18:18.00000008 +08:00', '2026-01-02 19:19:19.000000009 +09:00')",
        "SELECT * FROM json_result_set_timestamp_tz",
    )
}

#[test]
fn should_return_time_as_arrow_even_if_json_result_set_is_returned() {
    run_arrow_and_json_and_match(
        "CREATE OR REPLACE TABLE json_result_set_time (t0 TIME(0),\
            t1 TIME(1), t2 TIME(2), t3 TIME(3), t4 TIME(4), t5 TIME(5), t6 TIME(6), t7 TIME(7), t8 TIME(8), t9 TIME(9),\
            t1a TIME(1), t2a TIME(2), t3a TIME(3), t4a TIME(4), t5a TIME(5), t6a TIME(6), t7a TIME(7), t8a TIME(8), t9a TIME(9))",
        "INSERT INTO json_result_set_time VALUES ('10:10:10',\
            '11:11:11.1', '12:12:12.12', '13:13:13.123', '14:14:14.1234', '15:15:15.12345', '16:16:16.123456', '17:17:17.1234567', '18:18:18.12345678', '19:19:19.123456789',\
            '11:11:11', '12:12:12', '13:13:13', '14:14:14', '15:15:15', '16:16:16', '17:17:17', '18:18:18', '19:19:19')",
        "SELECT * FROM json_result_set_time",
    )
}

#[test]
fn should_return_binary_as_arrow_even_if_json_result_set_is_returned() {
    run_arrow_and_json_and_match(
        "CREATE OR REPLACE TABLE json_result_set_binary (b BINARY)",
        "INSERT INTO json_result_set_binary VALUES (TO_BINARY('hello', 'UTF-8'))",
        "SELECT * FROM json_result_set_binary",
    )
}

#[test]
fn should_return_decfloat_as_arrow_even_if_json_result_set_is_returned() {
    run_arrow_and_json_and_match(
        "CREATE OR REPLACE TABLE json_result_set_decfloat (\
            d_zero DECFLOAT,\
            d_pos DECFLOAT,\
            d_neg DECFLOAT,\
            d_pos_frac DECFLOAT,\
            d_neg_frac DECFLOAT,\
            d_large_pos_exp DECFLOAT,\
            d_large_neg_exp DECFLOAT,\
            d_tiny_pos DECFLOAT,\
            d_tiny_neg DECFLOAT,\
            d_max_precision DECFLOAT, d_neg_max_precision DECFLOAT,\
            d_max_exp DECFLOAT,\
            d_min_exp DECFLOAT,\
            d_neg_large_exp DECFLOAT,\
            d_pos_large_exp DECFLOAT,\
            d_one DECFLOAT,\
            d_neg_one DECFLOAT,\
            d_integer DECFLOAT)",
        "INSERT INTO json_result_set_decfloat SELECT \
            0::DECFLOAT, \
            123.456::DECFLOAT, \
            '-789.012'::DECFLOAT, \
            1.5::DECFLOAT, \
            '-1.5'::DECFLOAT, \
            1.23E+20::DECFLOAT, \
            '-9.87E-15'::DECFLOAT, \
            1E-16383::DECFLOAT, \
            '-1E-16383'::DECFLOAT,
            12345678901234567890123456789012345678::DECFLOAT, \
            '-12345678901234567890123456789012345678'::DECFLOAT, \
            '1E+16384'::DECFLOAT, \
            '1E-16383'::DECFLOAT, \
            '-1.234E+8000'::DECFLOAT, \
            '9.876E-8000'::DECFLOAT, \
            1::DECFLOAT, \
            (-1)::DECFLOAT, \
            42::DECFLOAT",
        "SELECT * FROM json_result_set_decfloat",
    )
}

#[test]
fn should_return_variant_as_arrow_even_if_json_result_set_is_returned() {
    run_arrow_and_json_and_match(
        "CREATE OR REPLACE TABLE json_result_set_variant (\
            v_string VARIANT, v_number VARIANT, v_bool VARIANT, \
            v_array VARIANT, v_object VARIANT)",
        "INSERT INTO json_result_set_variant SELECT \
            TO_VARIANT('hello'), \
            TO_VARIANT(123.456), \
            TO_VARIANT(TRUE), \
            TO_VARIANT(PARSE_JSON('[1, 2, 3]')), \
            TO_VARIANT(PARSE_JSON('{\"key\": \"value\"}'))",
        "SELECT * FROM json_result_set_variant",
    )
}

#[test]
fn should_return_object_as_arrow_even_if_json_result_set_is_returned() {
    run_arrow_and_json_and_match(
        "CREATE OR REPLACE TABLE json_result_set_object (o OBJECT)",
        "INSERT INTO json_result_set_object SELECT PARSE_JSON('{\"a\": 1, \"b\": \"two\"}')",
        "SELECT * FROM json_result_set_object",
    )
}

#[test]
fn should_return_array_as_arrow_even_if_json_result_set_is_returned() {
    run_arrow_and_json_and_match(
        "CREATE OR REPLACE TABLE json_result_set_array (a ARRAY)",
        "INSERT INTO json_result_set_array SELECT PARSE_JSON('[1, \"two\", 3.0, null]')",
        "SELECT * FROM json_result_set_array",
    )
}

fn run_arrow_and_json_and_match(create_table_query: &str, insert_query: &str, select_query: &str) {
    let after_table = &create_table_query[create_table_query.find("TABLE ").unwrap() + 6..];
    let base_name = after_table
        .split(|c: char| c.is_whitespace() || c == '(')
        .next()
        .unwrap();
    let table_name = unique_table_name(base_name);
    let create_table_query = create_table_query.replace(base_name, &table_name);
    let insert_query = insert_query.replace(base_name, &table_name);
    let select_query = select_query.replace(base_name, &table_name);

    let client = SnowflakeTestClient::connect_with_default_auth();
    let _guard = TableCleanupGuard::new(table_name.clone(), |name| {
        let stmt = client.new_statement();
        client.set_sql_query(&stmt, &format!("DROP TABLE IF EXISTS {name}"));
        client.execute_statement_query(&stmt);
        client.release_statement(&stmt);
    });
    let stmt = client.new_statement();

    client.set_sql_query(&stmt, &create_table_query);
    client.execute_statement_query(&stmt);

    client.set_sql_query(&stmt, "ALTER SESSION SET TIMEZONE = 'Pacific/Honolulu'");
    client.execute_statement_query(&stmt);

    client.set_sql_query(&stmt, &insert_query);
    client.execute_statement_query(&stmt);

    client.set_sql_query(&stmt, "ALTER SESSION SET TIMEZONE = 'Europe/Warsaw'");
    client.execute_statement_query(&stmt);

    client.set_sql_query(&stmt, &select_query);
    let arrow_result = client.execute_statement_query(&stmt);

    client.set_sql_query(
        &stmt,
        "ALTER SESSION SET PYTHON_CONNECTOR_QUERY_RESULT_FORMAT = JSON",
    );
    let result = client.execute_statement_query(&stmt);
    assert_eq!(result.rows_affected(), 1, "Cannot force JSON result set");

    client.set_sql_query(&stmt, &select_query);
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
