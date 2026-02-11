use sfodbc::ParameterBinding;
/// Integration tests for ODBC JSON parameter binding
///
/// These tests verify that parameter bindings are correctly serialized to JSON
/// format and work end-to-end within the ODBC wrapper.
use sfodbc::json_binding::serialize_bindings;
use std::collections::HashMap;
use std::ffi::CString;

#[test]
#[allow(clippy::approx_constant)]
fn test_mixed_types_integration() {
    use odbc_sys::{self as sql, SqlDataType};
    use sfodbc::CDataType;

    let mut bindings = HashMap::new();

    // Parameter 1: INTEGER
    let int_val: i32 = 42;
    let int_ptr = &int_val as *const i32 as sql::Pointer;
    let mut int_len: sql::Len = 0;
    bindings.insert(
        1,
        ParameterBinding {
            parameter_type: SqlDataType::INTEGER,
            value_type: CDataType::SLong,
            parameter_value_ptr: int_ptr,
            buffer_length: 4,
            str_len_or_ind_ptr: &mut int_len as *mut sql::Len,
        },
    );

    // Parameter 2: VARCHAR
    let str_val = CString::new("test").unwrap();
    let str_ptr = str_val.as_ptr() as sql::Pointer;
    let mut str_len: sql::Len = 4;
    bindings.insert(
        2,
        ParameterBinding {
            parameter_type: SqlDataType::VARCHAR,
            value_type: CDataType::Char,
            parameter_value_ptr: str_ptr,
            buffer_length: 100,
            str_len_or_ind_ptr: &mut str_len as *mut sql::Len,
        },
    );

    // Parameter 3: NULL
    let null_val: i32 = 0;
    let null_ptr = &null_val as *const i32 as sql::Pointer;
    let mut null_len: sql::Len = sql::NULL_DATA;
    bindings.insert(
        3,
        ParameterBinding {
            parameter_type: SqlDataType::INTEGER,
            value_type: CDataType::SLong,
            parameter_value_ptr: null_ptr,
            buffer_length: 4,
            str_len_or_ind_ptr: &mut null_len as *mut sql::Len,
        },
    );

    // Parameter 4: DOUBLE
    let double_val: f64 = 3.14159; // Test arbitrary float value
    let double_ptr = &double_val as *const f64 as sql::Pointer;
    let mut double_len: sql::Len = 0;
    bindings.insert(
        4,
        ParameterBinding {
            parameter_type: SqlDataType::DOUBLE,
            value_type: CDataType::Double,
            parameter_value_ptr: double_ptr,
            buffer_length: 8,
            str_len_or_ind_ptr: &mut double_len as *mut sql::Len,
        },
    );

    // Parameter 5: BOOLEAN
    let bool_val: u8 = 1;
    let bool_ptr = &bool_val as *const u8 as sql::Pointer;
    let mut bool_len: sql::Len = 0;
    bindings.insert(
        5,
        ParameterBinding {
            parameter_type: SqlDataType::EXT_BIT,
            value_type: CDataType::Bit,
            parameter_value_ptr: bool_ptr,
            buffer_length: 1,
            str_len_or_ind_ptr: &mut bool_len as *mut sql::Len,
        },
    );

    // Serialize bindings
    let json_str = serialize_bindings(&bindings).expect("Serialization should succeed");

    // Parse and verify
    let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("Should be valid JSON");

    // Verify structure
    assert_eq!(parsed.as_object().unwrap().len(), 5);

    // Verify each parameter
    assert_eq!(parsed["1"]["type"], "FIXED");
    assert_eq!(parsed["1"]["value"], "42");

    assert_eq!(parsed["2"]["type"], "TEXT");
    assert_eq!(parsed["2"]["value"], "test");

    assert_eq!(parsed["3"]["type"], "FIXED");
    assert!(parsed["3"]["value"].is_null());

    assert_eq!(parsed["4"]["type"], "REAL");
    assert_eq!(parsed["4"]["value"], "3.14159");

    assert_eq!(parsed["5"]["type"], "BOOLEAN");
    assert_eq!(parsed["5"]["value"], "true");
}

#[test]
fn test_json_format_matches_python_wrapper() {
    use odbc_sys::{self as sql, SqlDataType};
    use sfodbc::CDataType;

    let mut bindings = HashMap::new();

    let int_val: i32 = 123;
    let int_ptr = &int_val as *const i32 as sql::Pointer;
    let mut int_len: sql::Len = 0;
    bindings.insert(
        1,
        ParameterBinding {
            parameter_type: SqlDataType::INTEGER,
            value_type: CDataType::SLong,
            parameter_value_ptr: int_ptr,
            buffer_length: 4,
            str_len_or_ind_ptr: &mut int_len as *mut sql::Len,
        },
    );

    let json_str = serialize_bindings(&bindings).unwrap();

    // Verify format matches Python wrapper expectations:
    // {"1": {"type": "FIXED", "value": "123"}}
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Parameter key should be string "1"
    assert!(parsed.get("1").is_some());

    // Should have "type" and "value" fields
    let param = &parsed["1"];
    assert!(param.get("type").is_some());
    assert!(param.get("value").is_some());

    // Types should be uppercase strings
    assert_eq!(param["type"].as_str().unwrap(), "FIXED");

    // Values should be strings (even for numbers)
    assert_eq!(param["value"].as_str().unwrap(), "123");
}

#[test]
fn test_large_number_of_parameters() {
    use odbc_sys::{self as sql, SqlDataType};
    use sfodbc::CDataType;

    let mut bindings = HashMap::new();
    let mut values: Vec<i32> = Vec::new();
    let mut lengths: Vec<sql::Len> = Vec::new();

    // Create 50 parameters
    for i in 1..=50 {
        let val = i * 10;
        values.push(val);
        lengths.push(0);
    }

    for i in 1..=50 {
        let val_ptr = &values[(i - 1) as usize] as *const i32 as sql::Pointer;
        let len_ptr = &mut lengths[(i - 1) as usize] as *mut sql::Len;
        bindings.insert(
            i,
            ParameterBinding {
                parameter_type: SqlDataType::INTEGER,
                value_type: CDataType::SLong,
                parameter_value_ptr: val_ptr,
                buffer_length: 4,
                str_len_or_ind_ptr: len_ptr,
            },
        );
    }

    let json_str = serialize_bindings(&bindings).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Verify all 50 parameters are present
    assert_eq!(parsed.as_object().unwrap().len(), 50);

    // Spot check a few values
    assert_eq!(parsed["1"]["value"], "10");
    assert_eq!(parsed["25"]["value"], "250");
    assert_eq!(parsed["50"]["value"], "500");
}

#[test]
fn test_utf8_string_handling() {
    use odbc_sys::{self as sql, SqlDataType};
    use sfodbc::CDataType;

    let mut bindings = HashMap::new();

    // Test UTF-8 strings with various characters
    let utf8_str = CString::new("Hello 世界 🌍").unwrap();
    let str_ptr = utf8_str.as_ptr() as sql::Pointer;
    let mut str_len: sql::Len = utf8_str.as_bytes().len() as sql::Len;

    bindings.insert(
        1,
        ParameterBinding {
            parameter_type: SqlDataType::VARCHAR,
            value_type: CDataType::Char,
            parameter_value_ptr: str_ptr,
            buffer_length: 100,
            str_len_or_ind_ptr: &mut str_len as *mut sql::Len,
        },
    );

    let json_str = serialize_bindings(&bindings).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(parsed["1"]["type"], "TEXT");
    assert_eq!(parsed["1"]["value"], "Hello 世界 🌍");
}

#[test]
fn test_binary_data_various_lengths() {
    use odbc_sys::{self as sql, SqlDataType};
    use sfodbc::CDataType;

    let mut bindings = HashMap::new();

    // Test binary data of different lengths
    let binary1: [u8; 1] = [0xFF];
    let binary4: [u8; 4] = [0x01, 0x02, 0x03, 0x04];
    let binary8: [u8; 8] = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11];

    let mut len1: sql::Len = 1;
    let mut len4: sql::Len = 4;
    let mut len8: sql::Len = 8;

    bindings.insert(
        1,
        ParameterBinding {
            parameter_type: SqlDataType::EXT_BINARY,
            value_type: CDataType::Binary,
            parameter_value_ptr: binary1.as_ptr() as sql::Pointer,
            buffer_length: 1,
            str_len_or_ind_ptr: &mut len1 as *mut sql::Len,
        },
    );

    bindings.insert(
        2,
        ParameterBinding {
            parameter_type: SqlDataType::EXT_BINARY,
            value_type: CDataType::Binary,
            parameter_value_ptr: binary4.as_ptr() as sql::Pointer,
            buffer_length: 4,
            str_len_or_ind_ptr: &mut len4 as *mut sql::Len,
        },
    );

    bindings.insert(
        3,
        ParameterBinding {
            parameter_type: SqlDataType::EXT_BINARY,
            value_type: CDataType::Binary,
            parameter_value_ptr: binary8.as_ptr() as sql::Pointer,
            buffer_length: 8,
            str_len_or_ind_ptr: &mut len8 as *mut sql::Len,
        },
    );

    let json_str = serialize_bindings(&bindings).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(parsed["1"]["value"], "ff");
    assert_eq!(parsed["2"]["value"], "01020304");
    assert_eq!(parsed["3"]["value"], "aabbccddeeff0011");
}

#[test]
fn test_numeric_edge_cases() {
    use odbc_sys::{self as sql, SqlDataType};
    use sfodbc::CDataType;

    let mut bindings = HashMap::new();

    // Max/min values for different types
    let max_i32 = i32::MAX;
    let min_i32 = i32::MIN;
    let max_i64 = i64::MAX;
    let min_i64 = i64::MIN;

    let mut len1: sql::Len = 0;
    let mut len2: sql::Len = 0;
    let mut len3: sql::Len = 0;
    let mut len4: sql::Len = 0;

    bindings.insert(
        1,
        ParameterBinding {
            parameter_type: SqlDataType::INTEGER,
            value_type: CDataType::SLong,
            parameter_value_ptr: &max_i32 as *const i32 as sql::Pointer,
            buffer_length: 4,
            str_len_or_ind_ptr: &mut len1 as *mut sql::Len,
        },
    );

    bindings.insert(
        2,
        ParameterBinding {
            parameter_type: SqlDataType::INTEGER,
            value_type: CDataType::SLong,
            parameter_value_ptr: &min_i32 as *const i32 as sql::Pointer,
            buffer_length: 4,
            str_len_or_ind_ptr: &mut len2 as *mut sql::Len,
        },
    );

    bindings.insert(
        3,
        ParameterBinding {
            parameter_type: SqlDataType::EXT_BIG_INT,
            value_type: CDataType::SBigInt,
            parameter_value_ptr: &max_i64 as *const i64 as sql::Pointer,
            buffer_length: 8,
            str_len_or_ind_ptr: &mut len3 as *mut sql::Len,
        },
    );

    bindings.insert(
        4,
        ParameterBinding {
            parameter_type: SqlDataType::EXT_BIG_INT,
            value_type: CDataType::SBigInt,
            parameter_value_ptr: &min_i64 as *const i64 as sql::Pointer,
            buffer_length: 8,
            str_len_or_ind_ptr: &mut len4 as *mut sql::Len,
        },
    );

    let json_str = serialize_bindings(&bindings).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(parsed["1"]["value"], "2147483647");
    assert_eq!(parsed["2"]["value"], "-2147483648");
    assert_eq!(parsed["3"]["value"], "9223372036854775807");
    assert_eq!(parsed["4"]["value"], "-9223372036854775808");
}

#[test]
fn test_all_null_parameters() {
    use odbc_sys::{self as sql, SqlDataType};
    use sfodbc::CDataType;

    let mut bindings = HashMap::new();

    let dummy1: i32 = 0;
    let dummy2: i32 = 0;
    let dummy3: i32 = 0;

    let mut len1: sql::Len = sql::NULL_DATA;
    let mut len2: sql::Len = sql::NULL_DATA;
    let mut len3: sql::Len = sql::NULL_DATA;

    bindings.insert(
        1,
        ParameterBinding {
            parameter_type: SqlDataType::INTEGER,
            value_type: CDataType::SLong,
            parameter_value_ptr: &dummy1 as *const i32 as sql::Pointer,
            buffer_length: 4,
            str_len_or_ind_ptr: &mut len1 as *mut sql::Len,
        },
    );

    bindings.insert(
        2,
        ParameterBinding {
            parameter_type: SqlDataType::VARCHAR,
            value_type: CDataType::Char,
            parameter_value_ptr: &dummy2 as *const i32 as sql::Pointer,
            buffer_length: 100,
            str_len_or_ind_ptr: &mut len2 as *mut sql::Len,
        },
    );

    bindings.insert(
        3,
        ParameterBinding {
            parameter_type: SqlDataType::DOUBLE,
            value_type: CDataType::Double,
            parameter_value_ptr: &dummy3 as *const i32 as sql::Pointer,
            buffer_length: 8,
            str_len_or_ind_ptr: &mut len3 as *mut sql::Len,
        },
    );

    let json_str = serialize_bindings(&bindings).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // All parameters should have null values
    assert!(parsed["1"]["value"].is_null());
    assert!(parsed["2"]["value"].is_null());
    assert!(parsed["3"]["value"].is_null());
}
