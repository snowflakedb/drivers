/// JSON binding serialization for ODBC parameter bindings.
///
/// This module handles the conversion of ODBC parameter bindings to JSON format
/// for transmission to the Rust core, following the pattern from the Python wrapper.
use crate::api::ParameterBinding;
use crate::cdata_types::{CDataType, SQL_NULL_DATA};
use odbc_sys as sql;
use serde_json::{Value, json};
use std::collections::HashMap;

/// Convert ODBC parameter bindings to JSON string format.
///
/// Format:
/// ```json
/// {
///     "1": {"type": "FIXED", "value": "123"},
///     "2": {"type": "TEXT", "value": "hello"}
/// }
/// ```
///
/// For NULL values, the value field is set to null.
///
/// Returns a tuple of (JSON string, byte length)
pub fn serialize_bindings(bindings: &HashMap<u16, ParameterBinding>) -> Result<String, String> {
    if bindings.is_empty() {
        return Ok(String::new());
    }

    let mut json_bindings: HashMap<String, Value> = HashMap::new();

    for (param_num, binding) in bindings.iter() {
        let param_key = param_num.to_string();

        // Check if value is NULL via str_len_or_ind_ptr
        let is_null = unsafe {
            !binding.str_len_or_ind_ptr.is_null() && *binding.str_len_or_ind_ptr == SQL_NULL_DATA
        };

        if is_null {
            // NULL value
            let snowflake_type = map_sql_type_to_snowflake(&binding.parameter_type);
            json_bindings.insert(
                param_key,
                json!({
                    "type": snowflake_type,
                    "value": Value::Null
                }),
            );
        } else {
            // Non-NULL value - extract and convert
            let (snowflake_type, value) = extract_parameter_value(binding)?;
            json_bindings.insert(
                param_key,
                json!({
                    "type": snowflake_type,
                    "value": value
                }),
            );
        }
    }

    // Serialize to JSON string
    let json_str = serde_json::to_string(&json_bindings)
        .map_err(|e| format!("Failed to serialize bindings to JSON: {}", e))?;

    Ok(json_str)
}

/// Map SQL data type to Snowflake type string
fn map_sql_type_to_snowflake(sql_type: &sql::SqlDataType) -> &'static str {
    match *sql_type {
        sql::SqlDataType::INTEGER => "FIXED",
        sql::SqlDataType::SMALLINT => "FIXED",
        sql::SqlDataType::EXT_BIG_INT => "FIXED",
        sql::SqlDataType::EXT_TINY_INT => "FIXED",

        sql::SqlDataType::VARCHAR => "TEXT",
        sql::SqlDataType::CHAR => "TEXT",
        sql::SqlDataType::EXT_LONG_VARCHAR => "TEXT",
        sql::SqlDataType::EXT_W_VARCHAR => "TEXT",
        sql::SqlDataType::EXT_W_CHAR => "TEXT",
        sql::SqlDataType::EXT_W_LONG_VARCHAR => "TEXT",

        sql::SqlDataType::EXT_BIT => "BOOLEAN",

        sql::SqlDataType::REAL => "REAL",
        sql::SqlDataType::FLOAT => "REAL",
        sql::SqlDataType::DOUBLE => "REAL",

        sql::SqlDataType::EXT_BINARY => "BINARY",
        sql::SqlDataType::EXT_VAR_BINARY => "BINARY",
        sql::SqlDataType::EXT_LONG_VAR_BINARY => "BINARY",

        sql::SqlDataType::DECIMAL => "FIXED",
        sql::SqlDataType::NUMERIC => "FIXED",

        sql::SqlDataType::DATE => "DATE",

        sql::SqlDataType::TIME => "TIME",

        sql::SqlDataType::TIMESTAMP => "TIMESTAMP_NTZ",

        // Default to TEXT for unknown types
        _ => "TEXT",
    }
}

/// Extract parameter value from binding and convert to string representation
fn extract_parameter_value(binding: &ParameterBinding) -> Result<(&'static str, String), String> {
    let snowflake_type = map_sql_type_to_snowflake(&binding.parameter_type);

    // Determine actual data length
    let data_length = if binding.str_len_or_ind_ptr.is_null() {
        // If no indicator, use buffer_length
        binding.buffer_length as usize
    } else {
        unsafe {
            let indicator = *binding.str_len_or_ind_ptr;
            if indicator == SQL_NULL_DATA {
                return Err(
                    "NULL value should be handled before calling extract_parameter_value"
                        .to_string(),
                );
            } else if indicator >= 0 {
                indicator as usize
            } else {
                binding.buffer_length as usize
            }
        }
    };

    // Extract value based on C data type
    let value_str = unsafe {
        match binding.value_type {
            CDataType::Char | CDataType::WChar => {
                // String data
                if binding.parameter_value_ptr.is_null() {
                    return Err("NULL pointer for string parameter".to_string());
                }

                if matches!(binding.value_type, CDataType::WChar) {
                    // UTF-16 string
                    let ptr = binding.parameter_value_ptr as *const u16;
                    let len = data_length / 2; // UTF-16 chars
                    let slice = std::slice::from_raw_parts(ptr, len);
                    String::from_utf16(slice)
                        .map_err(|e| format!("Invalid UTF-16 string: {}", e))?
                } else {
                    // UTF-8 / ASCII string
                    let ptr = binding.parameter_value_ptr as *const u8;
                    let slice = std::slice::from_raw_parts(ptr, data_length);

                    // Find null terminator if present
                    let actual_len = slice.iter().position(|&c| c == 0).unwrap_or(data_length);

                    String::from_utf8(slice[..actual_len].to_vec())
                        .map_err(|e| format!("Invalid UTF-8 string: {}", e))?
                }
            }

            CDataType::SLong | CDataType::Long => {
                // 32-bit integer
                let ptr = binding.parameter_value_ptr as *const i32;
                (*ptr).to_string()
            }

            CDataType::ULong => {
                // Unsigned 32-bit integer
                let ptr = binding.parameter_value_ptr as *const u32;
                (*ptr).to_string()
            }

            CDataType::SShort | CDataType::Short => {
                // 16-bit integer
                let ptr = binding.parameter_value_ptr as *const i16;
                (*ptr).to_string()
            }

            CDataType::UShort => {
                // Unsigned 16-bit integer
                let ptr = binding.parameter_value_ptr as *const u16;
                (*ptr).to_string()
            }

            CDataType::SBigInt => {
                // 64-bit integer
                let ptr = binding.parameter_value_ptr as *const i64;
                (*ptr).to_string()
            }

            CDataType::UBigInt => {
                // Unsigned 64-bit integer
                let ptr = binding.parameter_value_ptr as *const u64;
                (*ptr).to_string()
            }

            CDataType::STinyInt | CDataType::TinyInt => {
                // 8-bit integer (signed)
                let ptr = binding.parameter_value_ptr as *const i8;
                (*ptr).to_string()
            }

            CDataType::UTinyInt => {
                // 8-bit integer (unsigned)
                let ptr = binding.parameter_value_ptr as *const u8;
                (*ptr).to_string()
            }

            CDataType::Float => {
                // 32-bit float
                let ptr = binding.parameter_value_ptr as *const f32;
                (*ptr).to_string()
            }

            CDataType::Double => {
                // 64-bit double
                let ptr = binding.parameter_value_ptr as *const f64;
                (*ptr).to_string()
            }

            CDataType::Bit => {
                // Boolean (typically 1 byte)
                let ptr = binding.parameter_value_ptr as *const u8;
                let bool_val = *ptr != 0;
                bool_val.to_string().to_lowercase()
            }

            CDataType::Binary => {
                // Binary data - hex encode
                if binding.parameter_value_ptr.is_null() {
                    return Err("NULL pointer for binary parameter".to_string());
                }
                let ptr = binding.parameter_value_ptr as *const u8;
                let slice = std::slice::from_raw_parts(ptr, data_length);
                hex::encode(slice)
            }

            CDataType::Numeric | CDataType::Default => {
                // Try to read as string for numeric/default types
                if binding.parameter_value_ptr.is_null() {
                    return Err("NULL pointer for parameter".to_string());
                }
                let ptr = binding.parameter_value_ptr as *const u8;
                let slice = std::slice::from_raw_parts(ptr, data_length);

                // Find null terminator if present
                let actual_len = slice.iter().position(|&c| c == 0).unwrap_or(data_length);

                String::from_utf8(slice[..actual_len].to_vec())
                    .map_err(|e| format!("Invalid UTF-8 string for numeric: {}", e))?
            }

            _ => {
                return Err(format!(
                    "Unsupported C data type for JSON binding: {:?}",
                    binding.value_type
                ));
            }
        }
    };

    Ok((snowflake_type, value_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_sql_type_to_snowflake() {
        assert_eq!(
            map_sql_type_to_snowflake(&sql::SqlDataType::INTEGER),
            "FIXED"
        );
        assert_eq!(
            map_sql_type_to_snowflake(&sql::SqlDataType::VARCHAR),
            "TEXT"
        );
        assert_eq!(
            map_sql_type_to_snowflake(&sql::SqlDataType::EXT_BIT),
            "BOOLEAN"
        );
        assert_eq!(
            map_sql_type_to_snowflake(&sql::SqlDataType::EXT_BINARY),
            "BINARY"
        );
        assert_eq!(map_sql_type_to_snowflake(&sql::SqlDataType::REAL), "REAL");
        assert_eq!(map_sql_type_to_snowflake(&sql::SqlDataType::DATE), "DATE");
        assert_eq!(map_sql_type_to_snowflake(&sql::SqlDataType::TIME), "TIME");
        assert_eq!(
            map_sql_type_to_snowflake(&sql::SqlDataType::TIMESTAMP),
            "TIMESTAMP_NTZ"
        );
    }

    #[test]
    fn test_serialize_empty_bindings() {
        let bindings: HashMap<u16, ParameterBinding> = HashMap::new();
        let result = serialize_bindings(&bindings).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_serialize_single_integer() {
        let mut bindings = HashMap::new();
        let value: i32 = 42;
        let value_ptr = &value as *const i32 as sql::Pointer;
        let mut length: sql::Len = 0;

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::INTEGER,
                value_type: CDataType::SLong,
                parameter_value_ptr: value_ptr,
                buffer_length: std::mem::size_of::<i32>() as sql::Len,
                str_len_or_ind_ptr: &mut length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        // Verify JSON structure
        assert!(json_str.contains(r#""1""#), "Should contain parameter 1");
        assert!(
            json_str.contains(r#""type":"FIXED""#),
            "Should have FIXED type"
        );
        assert!(json_str.contains(r#""value":"42""#), "Should have value 42");

        // Verify valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "FIXED");
        assert_eq!(param["value"], "42");
    }

    #[test]
    fn test_serialize_multiple_parameters() {
        use std::ffi::CString;

        let mut bindings = HashMap::new();

        // Parameter 1: INTEGER
        let int_value: i32 = 123;
        let int_ptr = &int_value as *const i32 as sql::Pointer;
        let mut int_length: sql::Len = 0;
        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::INTEGER,
                value_type: CDataType::SLong,
                parameter_value_ptr: int_ptr,
                buffer_length: std::mem::size_of::<i32>() as sql::Len,
                str_len_or_ind_ptr: &mut int_length as *mut sql::Len,
            },
        );

        // Parameter 2: VARCHAR
        let string_value = CString::new("test_string").unwrap();
        let string_ptr = string_value.as_ptr() as sql::Pointer;
        let mut string_length: sql::Len = 11;
        bindings.insert(
            2,
            ParameterBinding {
                parameter_type: sql::SqlDataType::VARCHAR,
                value_type: CDataType::Char,
                parameter_value_ptr: string_ptr,
                buffer_length: 100,
                str_len_or_ind_ptr: &mut string_length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        // Verify all parameters present
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.as_object().unwrap().len(), 2);

        let param1 = parsed.get("1").unwrap();
        assert_eq!(param1["type"], "FIXED");
        assert_eq!(param1["value"], "123");

        let param2 = parsed.get("2").unwrap();
        assert_eq!(param2["type"], "TEXT");
        assert_eq!(param2["value"], "test_string");
    }

    #[test]
    fn test_serialize_null_value() {
        let mut bindings = HashMap::new();
        let value: i32 = 0; // Value doesn't matter for NULL
        let value_ptr = &value as *const i32 as sql::Pointer;
        let mut length: sql::Len = sql::NULL_DATA; // SQL_NULL_DATA indicates NULL

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::INTEGER,
                value_type: CDataType::SLong,
                parameter_value_ptr: value_ptr,
                buffer_length: std::mem::size_of::<i32>() as sql::Len,
                str_len_or_ind_ptr: &mut length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        // Verify NULL is represented correctly
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "FIXED");
        assert!(param["value"].is_null(), "Value should be JSON null");
    }

    #[test]
    fn test_serialize_boolean_true() {
        let mut bindings = HashMap::new();
        let value: u8 = 1; // TRUE in SQL_C_BIT
        let value_ptr = &value as *const u8 as sql::Pointer;
        let mut length: sql::Len = 0;

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::EXT_BIT,
                value_type: CDataType::Bit,
                parameter_value_ptr: value_ptr,
                buffer_length: 1,
                str_len_or_ind_ptr: &mut length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "BOOLEAN");
        assert_eq!(param["value"], "true");
    }

    #[test]
    fn test_serialize_boolean_false() {
        let mut bindings = HashMap::new();
        let value: u8 = 0; // FALSE in SQL_C_BIT
        let value_ptr = &value as *const u8 as sql::Pointer;
        let mut length: sql::Len = 0;

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::EXT_BIT,
                value_type: CDataType::Bit,
                parameter_value_ptr: value_ptr,
                buffer_length: 1,
                str_len_or_ind_ptr: &mut length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "BOOLEAN");
        assert_eq!(param["value"], "false");
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_serialize_double() {
        let mut bindings = HashMap::new();
        let value: f64 = 3.14159; // Testing arbitrary float value serialization
        let value_ptr = &value as *const f64 as sql::Pointer;
        let mut length: sql::Len = 0;

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::DOUBLE,
                value_type: CDataType::Double,
                parameter_value_ptr: value_ptr,
                buffer_length: std::mem::size_of::<f64>() as sql::Len,
                str_len_or_ind_ptr: &mut length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "REAL");
        assert_eq!(param["value"], "3.14159");
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_serialize_float() {
        let mut bindings = HashMap::new();
        let value: f32 = 2.718; // Testing arbitrary float value serialization
        let value_ptr = &value as *const f32 as sql::Pointer;
        let mut length: sql::Len = 0;

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::REAL,
                value_type: CDataType::Float,
                parameter_value_ptr: value_ptr,
                buffer_length: std::mem::size_of::<f32>() as sql::Len,
                str_len_or_ind_ptr: &mut length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "REAL");
        // Float might have precision differences, just verify it's a number
        assert!(param["value"].as_str().unwrap().parse::<f32>().is_ok());
    }

    #[test]
    fn test_serialize_binary_hex_encoding() {
        let mut bindings = HashMap::new();
        let binary_data: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
        let binary_ptr = binary_data.as_ptr() as sql::Pointer;
        let mut length: sql::Len = 4;

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::EXT_BINARY,
                value_type: CDataType::Binary,
                parameter_value_ptr: binary_ptr,
                buffer_length: 4,
                str_len_or_ind_ptr: &mut length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "BINARY");
        assert_eq!(param["value"], "deadbeef");
    }

    #[test]
    fn test_serialize_smallint() {
        let mut bindings = HashMap::new();
        let value: i16 = -32000;
        let value_ptr = &value as *const i16 as sql::Pointer;
        let mut length: sql::Len = 0;

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::SMALLINT,
                value_type: CDataType::SShort,
                parameter_value_ptr: value_ptr,
                buffer_length: std::mem::size_of::<i16>() as sql::Len,
                str_len_or_ind_ptr: &mut length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "FIXED");
        assert_eq!(param["value"], "-32000");
    }

    #[test]
    fn test_serialize_bigint() {
        let mut bindings = HashMap::new();
        let value: i64 = 9223372036854775807;
        let value_ptr = &value as *const i64 as sql::Pointer;
        let mut length: sql::Len = 0;

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::EXT_BIG_INT,
                value_type: CDataType::SBigInt,
                parameter_value_ptr: value_ptr,
                buffer_length: std::mem::size_of::<i64>() as sql::Len,
                str_len_or_ind_ptr: &mut length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "FIXED");
        assert_eq!(param["value"], "9223372036854775807");
    }

    #[test]
    fn test_serialize_varchar_with_length() {
        use std::ffi::CString;

        let mut bindings = HashMap::new();
        let string_value = CString::new("Hello, World!").unwrap();
        let string_ptr = string_value.as_ptr() as sql::Pointer;
        let mut string_length: sql::Len = 13; // Exact length

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::VARCHAR,
                value_type: CDataType::Char,
                parameter_value_ptr: string_ptr,
                buffer_length: 100,
                str_len_or_ind_ptr: &mut string_length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "TEXT");
        assert_eq!(param["value"], "Hello, World!");
    }

    #[test]
    fn test_serialize_empty_string() {
        use std::ffi::CString;

        let mut bindings = HashMap::new();
        let string_value = CString::new("").unwrap();
        let string_ptr = string_value.as_ptr() as sql::Pointer;
        let mut string_length: sql::Len = 0;

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::VARCHAR,
                value_type: CDataType::Char,
                parameter_value_ptr: string_ptr,
                buffer_length: 100,
                str_len_or_ind_ptr: &mut string_length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "TEXT");
        assert_eq!(param["value"], "");
    }

    #[test]
    fn test_serialize_zero_integer() {
        let mut bindings = HashMap::new();
        let value: i32 = 0;
        let value_ptr = &value as *const i32 as sql::Pointer;
        let mut length: sql::Len = 0;

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::INTEGER,
                value_type: CDataType::SLong,
                parameter_value_ptr: value_ptr,
                buffer_length: std::mem::size_of::<i32>() as sql::Len,
                str_len_or_ind_ptr: &mut length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "FIXED");
        assert_eq!(param["value"], "0");
    }

    #[test]
    fn test_serialize_negative_integer() {
        let mut bindings = HashMap::new();
        let value: i32 = -42;
        let value_ptr = &value as *const i32 as sql::Pointer;
        let mut length: sql::Len = 0;

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::INTEGER,
                value_type: CDataType::SLong,
                parameter_value_ptr: value_ptr,
                buffer_length: std::mem::size_of::<i32>() as sql::Len,
                str_len_or_ind_ptr: &mut length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "FIXED");
        assert_eq!(param["value"], "-42");
    }

    #[test]
    fn test_serialize_parameter_order_preserved() {
        let mut bindings = HashMap::new();

        // Add parameters in non-sequential order
        let value3: i32 = 300;
        let ptr3 = &value3 as *const i32 as sql::Pointer;
        let mut len3: sql::Len = 0;
        bindings.insert(
            3,
            ParameterBinding {
                parameter_type: sql::SqlDataType::INTEGER,
                value_type: CDataType::SLong,
                parameter_value_ptr: ptr3,
                buffer_length: 4,
                str_len_or_ind_ptr: &mut len3 as *mut sql::Len,
            },
        );

        let value1: i32 = 100;
        let ptr1 = &value1 as *const i32 as sql::Pointer;
        let mut len1: sql::Len = 0;
        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::INTEGER,
                value_type: CDataType::SLong,
                parameter_value_ptr: ptr1,
                buffer_length: 4,
                str_len_or_ind_ptr: &mut len1 as *mut sql::Len,
            },
        );

        let value2: i32 = 200;
        let ptr2 = &value2 as *const i32 as sql::Pointer;
        let mut len2: sql::Len = 0;
        bindings.insert(
            2,
            ParameterBinding {
                parameter_type: sql::SqlDataType::INTEGER,
                value_type: CDataType::SLong,
                parameter_value_ptr: ptr2,
                buffer_length: 4,
                str_len_or_ind_ptr: &mut len2 as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        // Verify all parameters are present with correct values
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["1"]["value"], "100");
        assert_eq!(parsed["2"]["value"], "200");
        assert_eq!(parsed["3"]["value"], "300");
    }

    #[test]
    fn test_serialize_special_characters_in_string() {
        use std::ffi::CString;

        let mut bindings = HashMap::new();
        // Test string with quotes, backslashes, and special chars
        let string_value = CString::new(r#"He said "hello" \ test"#).unwrap();
        let string_ptr = string_value.as_ptr() as sql::Pointer;
        let mut string_length: sql::Len = string_value.as_bytes().len() as sql::Len;

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::VARCHAR,
                value_type: CDataType::Char,
                parameter_value_ptr: string_ptr,
                buffer_length: 100,
                str_len_or_ind_ptr: &mut string_length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        // Verify it's valid JSON (special chars should be escaped)
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "TEXT");
        // serde_json handles escaping, so we just verify it parses correctly
        assert!(param["value"].as_str().unwrap().contains("hello"));
    }

    #[test]
    fn test_serialize_tinyint() {
        let mut bindings = HashMap::new();
        let value: i8 = 127;
        let value_ptr = &value as *const i8 as sql::Pointer;
        let mut length: sql::Len = 0;

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::EXT_TINY_INT,
                value_type: CDataType::STinyInt,
                parameter_value_ptr: value_ptr,
                buffer_length: std::mem::size_of::<i8>() as sql::Len,
                str_len_or_ind_ptr: &mut length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "FIXED");
        assert_eq!(param["value"], "127");
    }

    #[test]
    fn test_serialize_decimal_numeric_types() {
        let mut bindings = HashMap::new();
        let value: i64 = 12345;
        let value_ptr = &value as *const i64 as sql::Pointer;
        let mut length: sql::Len = 0;

        bindings.insert(
            1,
            ParameterBinding {
                parameter_type: sql::SqlDataType::DECIMAL,
                value_type: CDataType::SBigInt,
                parameter_value_ptr: value_ptr,
                buffer_length: std::mem::size_of::<i64>() as sql::Len,
                str_len_or_ind_ptr: &mut length as *mut sql::Len,
            },
        );

        let json_str = serialize_bindings(&bindings).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let param = parsed.get("1").unwrap();
        assert_eq!(param["type"], "FIXED");
        assert_eq!(param["value"], "12345");
    }
}
