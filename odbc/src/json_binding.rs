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
}
