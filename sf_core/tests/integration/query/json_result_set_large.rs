use crate::common::arrow_deserialize::RecordBatch;
use crate::common::arrow_result_helper::{
    ArrowResultHelper, assert_record_batches_match, assert_schemas_match,
};
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::test_utils::{TableCleanupGuard, unique_table_name};
use arrow::array::Array;
use arrow::compute::concat_batches;

#[test]
fn should_return_arrow_matching_json_for_large_multi_chunk_result() {
    let table_name = unique_table_name("json_result_set_large");
    let client = SnowflakeTestClient::connect_with_default_auth();
    let _guard = TableCleanupGuard::new(table_name.clone(), |name| {
        let stmt = client.new_statement();
        client.set_sql_query(&stmt, &format!("DROP TABLE IF EXISTS {name}"));
        client.execute_statement_query(&stmt);
        client.release_statement(&stmt);
    });
    let stmt = client.new_statement();

    client.set_sql_query(
        &stmt,
        &format!(
            "CREATE OR REPLACE TABLE {table_name} (\
                str_col STRING, \
                tinyint_col TINYINT, \
                smallint_col SMALLINT, \
                int_col INT, \
                bigint_col BIGINT, \
                num_col NUMBER(38,0), \
                num_scale_col NUMBER(38,2), \
                bool_col BOOLEAN, \
                real_col DOUBLE, \
                date_col DATE, \
                ntz_col TIMESTAMP_NTZ(3), \
                ntz_hi_col TIMESTAMP_NTZ(9), \
                ltz_col TIMESTAMP_LTZ(3), \
                ltz_hi_col TIMESTAMP_LTZ(9), \
                tz_col TIMESTAMP_TZ(3), \
                tz_hi_col TIMESTAMP_TZ(9), \
                time_col TIME(3), \
                bin_col BINARY, \
                variant_col VARIANT, \
                object_col OBJECT, \
                array_col ARRAY
            )"
        ),
    );
    client.execute_statement_query(&stmt);

    client.set_sql_query(&stmt, "ALTER SESSION SET TIMEZONE = 'Pacific/Honolulu'");
    client.execute_statement_query(&stmt);

    client.set_sql_query(
        &stmt,
        &format!(
            "INSERT INTO {table_name} \
            SELECT \
                CASE WHEN seq4() % 2 = 1 THEN 'abc' END, \
                CASE WHEN seq4() % 2 = 1 THEN 42 END, \
                CASE WHEN seq4() % 2 = 1 THEN 12345 END, \
                CASE WHEN seq4() % 2 = 1 THEN 1234567 END, \
                CASE WHEN seq4() % 2 = 1 THEN 1234567890123 END, \
                CASE WHEN seq4() % 2 = 1 THEN 12345678901234567890123456789012345678 END, \
                CASE WHEN seq4() % 2 = 1 THEN 123.45 END, \
                CASE WHEN seq4() % 2 = 1 THEN TRUE END, \
                CASE WHEN seq4() % 2 = 1 THEN 3.14 END, \
                CASE WHEN seq4() % 2 = 1 THEN '2024-01-15'::DATE END, \
                CASE WHEN seq4() % 2 = 1 THEN '2024-01-15 10:30:00.123'::TIMESTAMP_NTZ(3) END, \
                CASE WHEN seq4() % 2 = 1 THEN '2024-01-15 10:30:00.123456789'::TIMESTAMP_NTZ(9) END, \
                CASE WHEN seq4() % 2 = 1 THEN '2024-01-15 10:30:00.123'::TIMESTAMP_LTZ(3) END, \
                CASE WHEN seq4() % 2 = 1 THEN '2024-01-15 10:30:00.123456789'::TIMESTAMP_LTZ(9) END, \
                CASE WHEN seq4() % 2 = 1 THEN '2024-01-15 10:30:00.123 +01:00'::TIMESTAMP_TZ(3) END, \
                CASE WHEN seq4() % 2 = 1 THEN '2024-01-15 10:30:00.123456789 +01:00'::TIMESTAMP_TZ(9) END, \
                CASE WHEN seq4() % 2 = 1 THEN '10:30:00.123'::TIME(3) END, \
                CASE WHEN seq4() % 2 = 1 THEN TO_BINARY('hello', 'UTF-8') END, \
                CASE WHEN seq4() % 2 = 1 THEN TO_VARIANT('test') END, \
                CASE WHEN seq4() % 2 = 1 THEN PARSE_JSON('{{\"k\": 1}}') END, \
                CASE WHEN seq4() % 2 = 1 THEN PARSE_JSON('[1, 2]') END \
            FROM TABLE(GENERATOR(ROWCOUNT => 3000)) v"
        ),
    );
    client.execute_statement_query(&stmt);

    client.set_sql_query(&stmt, "ALTER SESSION SET TIMEZONE = 'Europe/Warsaw'");
    client.execute_statement_query(&stmt);

    let select_query = format!("SELECT * FROM {table_name}");

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

    let mut arrow_helper = ArrowResultHelper::from_result(arrow_result);
    let mut json_helper = ArrowResultHelper::from_result(json_result);

    let arrow_schema = arrow_helper.schema();
    let json_schema = json_helper.schema();
    assert_schemas_match(&arrow_schema, &json_schema);

    let arrow_merged = collect_all_batches(&mut arrow_helper);
    let json_merged = collect_all_batches(&mut json_helper);

    assert_eq!(arrow_merged.num_rows(), 3000);
    assert_eq!(json_merged.num_rows(), 3000);

    assert_record_batches_match(&arrow_merged, &json_merged);

    // Each column should have exactly 1500 nulls (from the all-null rows)
    // and 1500 non-null values (from the non-null rows)
    for col_idx in 0..json_merged.num_columns() {
        let field_name = json_merged.schema().field(col_idx).name().clone();
        let col = json_merged.column(col_idx);
        assert_eq!(
            col.null_count(),
            1500,
            "Column '{field_name}' should have exactly 1500 nulls"
        );
    }

    client.release_statement(&stmt);
}

fn collect_all_batches(helper: &mut ArrowResultHelper) -> RecordBatch {
    let schema = helper.schema();
    let mut batches = Vec::new();
    while let Some(batch) = helper.next_batch() {
        batches.push(batch);
    }
    assert!(!batches.is_empty(), "Expected at least one batch");
    concat_batches(&schema, &batches).expect("Failed to concatenate batches")
}
