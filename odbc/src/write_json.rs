use std::{
    collections::HashMap,
    ffi::{CStr, c_char},
    slice, str,
};

use error_trace::ErrorTrace;
use serde_json::{Map, Value};
use snafu::{Location, ResultExt, Snafu};

use crate::{api::ParameterBinding, cdata_types::CDataType};
use odbc_sys as sql;

const SF_TYPE_ANY: &str = "ANY";
const SF_TYPE_FIXED: &str = "FIXED";
const SF_TYPE_TEXT: &str = "TEXT";
const SF_TYPE_REAL: &str = "REAL";
const SF_TYPE_BOOLEAN: &str = "BOOLEAN";
const SF_TYPE_BINARY: &str = "BINARY";
const SF_TYPE_DATE: &str = "DATE";
const SF_TYPE_TIME: &str = "TIME";
const SF_TYPE_TIMESTAMP_NTZ: &str = "TIMESTAMP_NTZ";

#[derive(Debug, Snafu, ErrorTrace)]
pub enum JsonBindingError {
    #[snafu(display("Parameter bindings must be contiguous and start at 1"))]
    InvalidParameterIndices {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported SQL parameter type: {sql_type:?}"))]
    UnsupportedParameterType {
        sql_type: sql::SqlDataType,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported C data type for JSON binding: {c_type:?}"))]
    UnsupportedCDataType {
        c_type: CDataType,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Null parameter value pointer encountered"))]
    NullPointer {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Parameter value is not valid UTF-8: {source}"))]
    InvalidUtf8 {
        source: str::Utf8Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to serialize bindings to JSON: {source}"))]
    Serialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Convert ODBC parameter bindings to JSON string format for server-side binding.
///
/// The `bindings` map must contain `ParameterBinding` instances whose
/// `parameter_value_ptr` pointers remain valid for the duration of this call.
///
/// Returns a JSON string in the format:
/// ```json
/// {
///   "1": {"type": "FIXED", "value": "123"},
///   "2": {"type": "TEXT", "value": "hello"}
/// }
/// ```
pub fn odbc_bindings_to_json(
    bindings: &HashMap<u16, ParameterBinding>,
) -> Result<String, JsonBindingError> {
    let mut json_bindings = Map::new();

    let max_key = bindings.keys().copied().max().unwrap_or(0);

    for param_num in 1..=max_key {
        let binding = bindings.get(&param_num).ok_or_else(|| {
            tracing::error!(
                "odbc_bindings_to_json: parameter #{param_num} not found. \
                 Parameter bindings must be contiguous and start at 1.",
            );
            InvalidParameterIndicesSnafu.build()
        })?;

        // Check for NULL value
        let is_null = !binding.str_len_or_ind_ptr.is_null()
            && unsafe { *binding.str_len_or_ind_ptr == sql::NULL_DATA };

        let (snowflake_type, value) = if is_null {
            (SF_TYPE_ANY, Value::Null)
        } else {
            convert_binding_to_json_value(binding)?
        };

        let mut binding_obj = Map::new();
        binding_obj.insert(
            "type".to_string(),
            Value::String(snowflake_type.to_string()),
        );
        binding_obj.insert("value".to_string(), value);

        json_bindings.insert(param_num.to_string(), Value::Object(binding_obj));
    }

    serde_json::to_string(&Value::Object(json_bindings)).context(SerializationSnafu)
}

/// Convert a single parameter binding to a Snowflake type and JSON value.
fn convert_binding_to_json_value(
    binding: &ParameterBinding,
) -> Result<(&'static str, Value), JsonBindingError> {
    let snowflake_type = snowflake_type_from_sql_type(&binding.parameter_type)?;

    check_not_null(binding)?;

    let value = match binding.value_type {
        CDataType::Long => read_numeric::<i32>(binding),
        CDataType::Short => read_numeric::<i16>(binding),
        CDataType::SBigInt => read_numeric::<i64>(binding),
        CDataType::Float => read_numeric::<f32>(binding),
        CDataType::Double => read_numeric::<f64>(binding),
        CDataType::Char => read_char_value(binding),
        CDataType::Bit => read_bit_value(binding),
        CDataType::Binary => read_binary_value(binding),
        _ => {
            tracing::error!(
                "Unsupported C data type for JSON binding: {:?}",
                binding.value_type
            );
            UnsupportedCDataTypeSnafu {
                c_type: binding.value_type,
            }
            .fail()
        }
    }?;

    Ok((snowflake_type, value))
}

/// Map SQL data types to Snowflake binding type names.
fn snowflake_type_from_sql_type(
    sql_type: &sql::SqlDataType,
) -> Result<&'static str, JsonBindingError> {
    match *sql_type {
        sql::SqlDataType::INTEGER
        | sql::SqlDataType::SMALLINT
        | sql::SqlDataType::EXT_BIG_INT
        | sql::SqlDataType::EXT_TINY_INT
        | sql::SqlDataType::DECIMAL
        | sql::SqlDataType::NUMERIC => Ok(SF_TYPE_FIXED),

        sql::SqlDataType::VARCHAR
        | sql::SqlDataType::CHAR
        | sql::SqlDataType::EXT_LONG_VARCHAR
        | sql::SqlDataType::EXT_W_CHAR
        | sql::SqlDataType::EXT_W_VARCHAR
        | sql::SqlDataType::EXT_W_LONG_VARCHAR => Ok(SF_TYPE_TEXT),

        sql::SqlDataType::REAL | sql::SqlDataType::FLOAT | sql::SqlDataType::DOUBLE => {
            Ok(SF_TYPE_REAL)
        }

        sql::SqlDataType::EXT_BIT => Ok(SF_TYPE_BOOLEAN),

        sql::SqlDataType::EXT_BINARY
        | sql::SqlDataType::EXT_VAR_BINARY
        | sql::SqlDataType::EXT_LONG_VAR_BINARY => Ok(SF_TYPE_BINARY),

        sql::SqlDataType::DATE => Ok(SF_TYPE_DATE),
        sql::SqlDataType::TIME => Ok(SF_TYPE_TIME),
        sql::SqlDataType::TIMESTAMP | sql::SqlDataType::EXT_TIMESTAMP => Ok(SF_TYPE_TIMESTAMP_NTZ),

        _ => {
            tracing::error!("Unsupported SQL data type for JSON binding: {:?}", sql_type);
            UnsupportedParameterTypeSnafu {
                sql_type: *sql_type,
            }
            .fail()
        }
    }
}

fn check_not_null(binding: &ParameterBinding) -> Result<(), JsonBindingError> {
    if binding.parameter_value_ptr.is_null() {
        return NullPointerSnafu.fail();
    }
    Ok(())
}

/// Read a numeric value from a binding pointer and convert to a JSON string value.
///
/// Uses `read_unaligned` to safely handle potentially misaligned ODBC pointers.
fn read_numeric<T: std::fmt::Display>(
    binding: &ParameterBinding,
) -> Result<Value, JsonBindingError> {
    let value = unsafe { std::ptr::read_unaligned(binding.parameter_value_ptr as *const T) };
    Ok(Value::String(value.to_string()))
}

/// Determine the actual byte length of buffer data, using the length/indicator
/// pointer if available, falling back to `buffer_length`.
///
/// Negative `buffer_length` values (e.g. `SQL_NTS`) are treated as zero.
/// Indicated length is clamped to `buffer_length` to prevent over-reads.
fn buffer_data_len(binding: &ParameterBinding) -> usize {
    let max_len = if binding.buffer_length < 0 {
        0
    } else {
        binding.buffer_length as usize
    };

    if !binding.str_len_or_ind_ptr.is_null() {
        let indicated_len = unsafe { *binding.str_len_or_ind_ptr };
        if indicated_len >= 0 {
            let indicated = indicated_len as usize;
            return if max_len > 0 {
                indicated.min(max_len)
            } else {
                indicated
            };
        }
    }

    max_len
}

/// Read a SQL_C_CHAR value as a UTF-8 string.
fn read_char_value(binding: &ParameterBinding) -> Result<Value, JsonBindingError> {
    let value_str = if binding.buffer_length == sql::NTS {
        unsafe {
            CStr::from_ptr(binding.parameter_value_ptr as *const c_char)
                .to_string_lossy()
                .to_string()
        }
    } else {
        let len = buffer_data_len(binding);
        let bytes = unsafe { slice::from_raw_parts(binding.parameter_value_ptr as *const u8, len) };
        str::from_utf8(bytes).context(InvalidUtf8Snafu)?.to_string()
    };

    Ok(Value::String(value_str))
}

/// Read a SQL_C_BIT value as a boolean string ("true"/"false").
fn read_bit_value(binding: &ParameterBinding) -> Result<Value, JsonBindingError> {
    let value = unsafe { std::ptr::read_unaligned(binding.parameter_value_ptr as *const u8) };
    Ok(Value::String((value != 0).to_string()))
}

/// Read a SQL_C_BINARY value as a hex-encoded string.
fn read_binary_value(binding: &ParameterBinding) -> Result<Value, JsonBindingError> {
    let len = buffer_data_len(binding);
    let bytes = unsafe { slice::from_raw_parts(binding.parameter_value_ptr as *const u8, len) };

    let hex_string: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    Ok(Value::String(hex_string))
}
