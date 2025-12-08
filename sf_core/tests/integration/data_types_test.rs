use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::test_utils::*;

use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::*;

#[test]
#[ignore] // Requires Snowflake credentials
fn test_numeric_types() {
    let client = SnowflakeTestClient::connect_with_default_auth();

    // Test FIXED (integer)
    let result = client.execute_query("SELECT 42::NUMBER(10,0) as fixed_col");
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper.assert_equals_single_value(String::from("42"));

    // Test DECIMAL
    let result = client.execute_query("SELECT 123.45::NUMBER(10,2) as decimal_col");
    let _arrow_helper = ArrowResultHelper::from_result(result);
    // Test passes if we get a result

    // Test REAL
    let result = client.execute_query("SELECT 3.14::REAL as real_col");
    let _arrow_helper = ArrowResultHelper::from_result(result);

    // Test DOUBLE
    let result = client.execute_query("SELECT 2.71828::DOUBLE as double_col");
    let _arrow_helper = ArrowResultHelper::from_result(result);
}

#[test]
#[ignore]
fn test_temporal_types() {
    let client = SnowflakeTestClient::connect_with_default_auth();

    // Test DATE
    let result = client.execute_query("SELECT CURRENT_DATE() as date_col");
    let _arrow_helper = ArrowResultHelper::from_result(result);

    // Test TIME
    let result = client.execute_query("SELECT CURRENT_TIME() as time_col");
    let _arrow_helper = ArrowResultHelper::from_result(result);

    // Test TIMESTAMP_NTZ
    let result = client.execute_query("SELECT CURRENT_TIMESTAMP()::TIMESTAMP_NTZ as ts_ntz");
    let _arrow_helper = ArrowResultHelper::from_result(result);

    // Test TIMESTAMP_LTZ
    let result = client.execute_query("SELECT CURRENT_TIMESTAMP()::TIMESTAMP_LTZ as ts_ltz");
    let _arrow_helper = ArrowResultHelper::from_result(result);

    // Test TIMESTAMP_TZ
    let result = client.execute_query("SELECT CURRENT_TIMESTAMP()::TIMESTAMP_TZ as ts_tz");
    let _arrow_helper = ArrowResultHelper::from_result(result);
}

#[test]
#[ignore]
fn test_string_and_binary_types() {
    let client = SnowflakeTestClient::connect_with_default_auth();

    // Test VARCHAR
    let result = client.execute_query("SELECT 'Hello, World!'::VARCHAR as varchar_col");
    let mut arrow_helper = ArrowResultHelper::from_result(result);
    arrow_helper.assert_equals_single_value(String::from("Hello, World!"));

    // Test BINARY
    let result = client.execute_query("SELECT TO_BINARY('ABCD', 'HEX') as binary_col");
    let _arrow_helper = ArrowResultHelper::from_result(result);

    // Test BOOLEAN
    let result = client.execute_query("SELECT TRUE as bool_col");
    let _arrow_helper = ArrowResultHelper::from_result(result);
}

#[test]
#[ignore]
fn test_semistructured_types() {
    let client = SnowflakeTestClient::connect_with_default_auth();

    // Test VARIANT
    let result = client.execute_query("SELECT PARSE_JSON('{\"name\":\"test\"}') as variant_col");
    let _arrow_helper = ArrowResultHelper::from_result(result);

    // Test OBJECT
    let result = client.execute_query("SELECT OBJECT_CONSTRUCT('key', 'value') as object_col");
    let _arrow_helper = ArrowResultHelper::from_result(result);

    // Test ARRAY
    let result = client.execute_query("SELECT ARRAY_CONSTRUCT(1, 2, 3) as array_col");
    let _arrow_helper = ArrowResultHelper::from_result(result);
}

#[test]
#[ignore]
fn test_all_types_in_one_query() {
    let client = SnowflakeTestClient::connect_with_default_auth();

    let query = r#"
        SELECT 
            42::NUMBER(10,0) as fixed_col,
            123.45::NUMBER(10,2) as decimal_col,
            3.14::REAL as real_col,
            2.71828::DOUBLE as double_col,
            'Hello'::VARCHAR as varchar_col,
            TRUE as bool_col,
            CURRENT_DATE() as date_col,
            CURRENT_TIME() as time_col,
            CURRENT_TIMESTAMP()::TIMESTAMP_NTZ as ts_ntz,
            PARSE_JSON('{"test": true}') as variant_col,
            ARRAY_CONSTRUCT(1, 2, 3) as array_col
    "#;

    let result = client.execute_query(query);
    let _arrow_helper = ArrowResultHelper::from_result(result);
    // Test passes if we successfully got Arrow data with all types
}
