use std::{
    collections::HashMap,
    ffi::{CStr, c_char},
    slice, str,
};

use serde_json::{Map, Value};
use snafu::Snafu;

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

#[derive(Debug, Snafu)]
pub enum JsonBindingError {
    #[snafu(display("Parameter bindings must be contiguous and start at 1"))]
    InvalidParameterIndices,

    #[snafu(display("Unsupported SQL parameter type: {sql_type:?}"))]
    UnsupportedParameterType { sql_type: sql::SqlDataType },

    #[snafu(display("Unsupported C data type for JSON binding: {c_type:?}"))]
    UnsupportedCDataType { c_type: CDataType },

    #[snafu(display("Null parameter value pointer encountered"))]
    NullPointer,

    #[snafu(display("Parameter value is not valid UTF-8"))]
    InvalidUtf8,

    #[snafu(display("Failed to serialize bindings to JSON: {message}"))]
    Serialization { message: String },
}

// =============================================================================
// Stage 1: Intermediate representation
// =============================================================================

/// Type-safe intermediate representation of a bound parameter value.
///
/// Sits between the raw ODBC C buffer (read stage) and the Snowflake JSON
/// output (write stage), decoupling pointer handling from serialization.
#[derive(Debug, Clone, PartialEq)]
pub enum BindingValue {
    Null,
    Integer(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
    Binary(Vec<u8>),
}

// =============================================================================
// Stage 2: Read — CDataType + raw pointer → BindingValue
// =============================================================================

/// Read a typed value from the ODBC parameter binding buffer.
///
/// # Safety contract
/// The caller must guarantee that `binding.parameter_value_ptr` points to
/// valid memory of the type implied by `binding.value_type`, and that the
/// pointer remains valid for the duration of this call.
pub fn read_odbc_value(binding: &ParameterBinding) -> Result<BindingValue, JsonBindingError> {
    if is_null_indicator(binding) {
        return Ok(BindingValue::Null);
    }

    if binding.parameter_value_ptr.is_null() {
        return Err(JsonBindingError::NullPointer);
    }

    match binding.value_type {
        CDataType::Long | CDataType::SLong => {
            Ok(BindingValue::Integer(read_unaligned::<i32>(binding) as i64))
        }
        CDataType::Short | CDataType::SShort => {
            Ok(BindingValue::Integer(read_unaligned::<i16>(binding) as i64))
        }
        CDataType::SBigInt => Ok(BindingValue::Integer(read_unaligned::<i64>(binding))),
        CDataType::ULong => Ok(BindingValue::Integer(read_unaligned::<u32>(binding) as i64)),
        CDataType::UShort => Ok(BindingValue::Integer(read_unaligned::<u16>(binding) as i64)),
        CDataType::UBigInt => Ok(BindingValue::Integer(read_unaligned::<u64>(binding) as i64)),
        CDataType::TinyInt | CDataType::STinyInt => {
            Ok(BindingValue::Integer(read_unaligned::<i8>(binding) as i64))
        }
        CDataType::UTinyInt => Ok(BindingValue::Integer(read_unaligned::<u8>(binding) as i64)),
        CDataType::Float => Ok(BindingValue::Float(read_unaligned::<f32>(binding) as f64)),
        CDataType::Double => Ok(BindingValue::Float(read_unaligned::<f64>(binding))),
        CDataType::Char => Ok(BindingValue::Text(read_char_str(binding)?)),
        CDataType::WChar => Ok(BindingValue::Text(read_char_str(binding)?)),
        CDataType::Bit => Ok(BindingValue::Boolean(read_unaligned::<u8>(binding) != 0)),
        CDataType::Binary => Ok(BindingValue::Binary(read_binary_bytes(binding))),
        _ => {
            tracing::error!(
                "Unsupported C data type for JSON binding: {:?}",
                binding.value_type
            );
            Err(JsonBindingError::UnsupportedCDataType {
                c_type: binding.value_type,
            })
        }
    }
}

// =============================================================================
// Stage 3: Write — BindingValue + SqlDataType → (Snowflake type, JSON value)
// =============================================================================

/// Convert an intermediate `BindingValue` and the declared SQL type into the
/// Snowflake JSON binding format: a type name string and a `serde_json::Value`.
pub fn write_json_value(
    value: &BindingValue,
    sql_type: &sql::SqlDataType,
) -> Result<(&'static str, Value), JsonBindingError> {
    match value {
        BindingValue::Null => Ok((SF_TYPE_ANY, Value::Null)),
        BindingValue::Integer(v) => {
            let sf_type = snowflake_type_from_sql_type(sql_type)?;
            Ok((sf_type, Value::String(v.to_string())))
        }
        BindingValue::Float(v) => {
            let sf_type = snowflake_type_from_sql_type(sql_type)?;
            Ok((sf_type, Value::String(v.to_string())))
        }
        BindingValue::Text(s) => {
            let sf_type = snowflake_type_from_sql_type(sql_type)?;
            Ok((sf_type, Value::String(s.clone())))
        }
        BindingValue::Boolean(b) => Ok((SF_TYPE_BOOLEAN, Value::String(b.to_string()))),
        BindingValue::Binary(bytes) => {
            let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
            Ok((SF_TYPE_BINARY, Value::String(hex)))
        }
    }
}

// =============================================================================
// Pipeline: combines read + write into the top-level conversion
// =============================================================================

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
            JsonBindingError::InvalidParameterIndices
        })?;

        let value = read_odbc_value(binding)?;
        let (snowflake_type, json_value) = write_json_value(&value, &binding.parameter_type)?;

        let mut binding_obj = Map::new();
        binding_obj.insert(
            "type".to_string(),
            Value::String(snowflake_type.to_string()),
        );
        binding_obj.insert("value".to_string(), json_value);

        json_bindings.insert(param_num.to_string(), Value::Object(binding_obj));
    }

    serde_json::to_string(&Value::Object(json_bindings)).map_err(|e| {
        tracing::error!("Failed to serialize bindings to JSON: {}", e);
        JsonBindingError::Serialization {
            message: e.to_string(),
        }
    })
}

// =============================================================================
// Helpers — type mapping
// =============================================================================

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
            Err(JsonBindingError::UnsupportedParameterType {
                sql_type: *sql_type,
            })
        }
    }
}

// =============================================================================
// Helpers — raw pointer reads
// =============================================================================

fn is_null_indicator(binding: &ParameterBinding) -> bool {
    !binding.str_len_or_ind_ptr.is_null()
        && unsafe { *binding.str_len_or_ind_ptr == sql::NULL_DATA }
}

/// Read a fixed-size value using `read_unaligned` for ODBC pointer safety.
fn read_unaligned<T: Copy>(binding: &ParameterBinding) -> T {
    unsafe { std::ptr::read_unaligned(binding.parameter_value_ptr as *const T) }
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
fn read_char_str(binding: &ParameterBinding) -> Result<String, JsonBindingError> {
    if binding.buffer_length == sql::NTS {
        let s = unsafe {
            CStr::from_ptr(binding.parameter_value_ptr as *const c_char)
                .to_string_lossy()
                .to_string()
        };
        Ok(s)
    } else {
        let len = buffer_data_len(binding);
        let bytes = unsafe { slice::from_raw_parts(binding.parameter_value_ptr as *const u8, len) };
        str::from_utf8(bytes)
            .map_err(|_| JsonBindingError::InvalidUtf8)
            .map(|s| s.to_string())
    }
}

/// Read raw bytes from a SQL_C_BINARY binding.
fn read_binary_bytes(binding: &ParameterBinding) -> Vec<u8> {
    let len = buffer_data_len(binding);
    let bytes = unsafe { slice::from_raw_parts(binding.parameter_value_ptr as *const u8, len) };
    bytes.to_vec()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_binding(
        value_type: CDataType,
        parameter_type: sql::SqlDataType,
        ptr: sql::Pointer,
        buffer_length: sql::Len,
        ind_ptr: *mut sql::Len,
    ) -> ParameterBinding {
        ParameterBinding {
            parameter_type,
            value_type,
            parameter_value_ptr: ptr,
            buffer_length,
            str_len_or_ind_ptr: ind_ptr,
        }
    }

    // -- read_odbc_value tests ------------------------------------------------

    #[test]
    fn read_integer_i32() {
        let val: i32 = 42;
        let binding = make_binding(
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert_eq!(
            read_odbc_value(&binding).unwrap(),
            BindingValue::Integer(42)
        );
    }

    #[test]
    fn read_integer_i16() {
        let val: i16 = -7;
        let binding = make_binding(
            CDataType::Short,
            sql::SqlDataType::SMALLINT,
            &val as *const i16 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert_eq!(
            read_odbc_value(&binding).unwrap(),
            BindingValue::Integer(-7)
        );
    }

    #[test]
    fn read_integer_i64() {
        let val: i64 = 9_999_999_999;
        let binding = make_binding(
            CDataType::SBigInt,
            sql::SqlDataType::EXT_BIG_INT,
            &val as *const i64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert_eq!(
            read_odbc_value(&binding).unwrap(),
            BindingValue::Integer(9_999_999_999)
        );
    }

    #[test]
    fn read_float_f64() {
        let val: f64 = 3.14;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::DOUBLE,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert_eq!(
            read_odbc_value(&binding).unwrap(),
            BindingValue::Float(3.14)
        );
    }

    #[test]
    fn read_float_f32() {
        let val: f32 = 1.5;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::REAL,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        if let BindingValue::Float(f) = read_odbc_value(&binding).unwrap() {
            assert!((f - 1.5).abs() < 0.001);
        } else {
            panic!("expected Float");
        }
    }

    #[test]
    fn read_char_nts() {
        let val = b"hello\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::VARCHAR,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert_eq!(
            read_odbc_value(&binding).unwrap(),
            BindingValue::Text("hello".to_string())
        );
    }

    #[test]
    fn read_char_with_length() {
        let val = b"hello world";
        let mut ind: sql::Len = 5;
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::VARCHAR,
            val.as_ptr() as sql::Pointer,
            11,
            &mut ind,
        );
        assert_eq!(
            read_odbc_value(&binding).unwrap(),
            BindingValue::Text("hello".to_string())
        );
    }

    #[test]
    fn read_bit_true() {
        let val: u8 = 1;
        let binding = make_binding(
            CDataType::Bit,
            sql::SqlDataType::EXT_BIT,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert_eq!(
            read_odbc_value(&binding).unwrap(),
            BindingValue::Boolean(true)
        );
    }

    #[test]
    fn read_bit_false() {
        let val: u8 = 0;
        let binding = make_binding(
            CDataType::Bit,
            sql::SqlDataType::EXT_BIT,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert_eq!(
            read_odbc_value(&binding).unwrap(),
            BindingValue::Boolean(false)
        );
    }

    #[test]
    fn read_binary() {
        let val: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
        let mut ind: sql::Len = 4;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_BINARY,
            val.as_ptr() as sql::Pointer,
            4,
            &mut ind,
        );
        assert_eq!(
            read_odbc_value(&binding).unwrap(),
            BindingValue::Binary(vec![0xDE, 0xAD, 0xBE, 0xEF])
        );
    }

    #[test]
    fn read_null_data() {
        let mut ind: sql::Len = sql::NULL_DATA;
        let binding = make_binding(
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            std::ptr::null_mut(),
            0,
            &mut ind,
        );
        assert_eq!(read_odbc_value(&binding).unwrap(), BindingValue::Null);
    }

    #[test]
    fn read_null_pointer_without_indicator_fails() {
        let binding = make_binding(
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        );
        assert!(read_odbc_value(&binding).is_err());
    }

    #[test]
    fn read_unsupported_c_type() {
        let val: i32 = 0;
        let binding = make_binding(
            CDataType::Guid,
            sql::SqlDataType::VARCHAR,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(read_odbc_value(&binding).is_err());
    }

    // -- write_json_value tests -----------------------------------------------

    #[test]
    fn write_null() {
        let (ty, val) = write_json_value(&BindingValue::Null, &sql::SqlDataType::INTEGER).unwrap();
        assert_eq!(ty, "ANY");
        assert_eq!(val, Value::Null);
    }

    #[test]
    fn write_integer() {
        let (ty, val) =
            write_json_value(&BindingValue::Integer(42), &sql::SqlDataType::INTEGER).unwrap();
        assert_eq!(ty, "FIXED");
        assert_eq!(val, Value::String("42".to_string()));
    }

    #[test]
    fn write_float() {
        let (ty, val) =
            write_json_value(&BindingValue::Float(3.14), &sql::SqlDataType::DOUBLE).unwrap();
        assert_eq!(ty, "REAL");
        assert_eq!(val, Value::String("3.14".to_string()));
    }

    #[test]
    fn write_text() {
        let (ty, val) = write_json_value(
            &BindingValue::Text("hello".to_string()),
            &sql::SqlDataType::VARCHAR,
        )
        .unwrap();
        assert_eq!(ty, "TEXT");
        assert_eq!(val, Value::String("hello".to_string()));
    }

    #[test]
    fn write_boolean() {
        let (ty, val) =
            write_json_value(&BindingValue::Boolean(true), &sql::SqlDataType::EXT_BIT).unwrap();
        assert_eq!(ty, "BOOLEAN");
        assert_eq!(val, Value::String("true".to_string()));
    }

    #[test]
    fn write_binary() {
        let (ty, val) = write_json_value(
            &BindingValue::Binary(vec![0xCA, 0xFE]),
            &sql::SqlDataType::EXT_BINARY,
        )
        .unwrap();
        assert_eq!(ty, "BINARY");
        assert_eq!(val, Value::String("cafe".to_string()));
    }

    #[test]
    fn write_unsupported_sql_type() {
        let bogus_type: sql::SqlDataType = unsafe { std::mem::transmute(9999i16) };
        let result = write_json_value(&BindingValue::Integer(1), &bogus_type);
        assert!(result.is_err());
    }

    // -- end-to-end pipeline tests -------------------------------------------

    #[test]
    fn pipeline_integer_binding() {
        let val: i32 = 99;
        let binding = make_binding(
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );

        let repr = read_odbc_value(&binding).unwrap();
        assert_eq!(repr, BindingValue::Integer(99));

        let (ty, json_val) = write_json_value(&repr, &binding.parameter_type).unwrap();
        assert_eq!(ty, "FIXED");
        assert_eq!(json_val, Value::String("99".to_string()));
    }

    #[test]
    fn pipeline_full_json_output() {
        let val: i32 = 7;
        let mut bindings = HashMap::new();
        bindings.insert(
            1u16,
            make_binding(
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                &val as *const i32 as sql::Pointer,
                0,
                std::ptr::null_mut(),
            ),
        );

        let json = odbc_bindings_to_json(&bindings).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["1"]["type"], "FIXED");
        assert_eq!(parsed["1"]["value"], "7");
    }

    #[test]
    fn pipeline_null_json_output() {
        let mut ind: sql::Len = sql::NULL_DATA;
        let mut bindings = HashMap::new();
        bindings.insert(
            1u16,
            make_binding(
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                std::ptr::null_mut(),
                0,
                &mut ind,
            ),
        );

        let json = odbc_bindings_to_json(&bindings).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["1"]["type"], "ANY");
        assert!(parsed["1"]["value"].is_null());
    }

    #[test]
    fn pipeline_non_contiguous_params_error() {
        let val: i32 = 1;
        let mut bindings = HashMap::new();
        bindings.insert(
            1u16,
            make_binding(
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                &val as *const i32 as sql::Pointer,
                0,
                std::ptr::null_mut(),
            ),
        );
        // Skip parameter 2, add parameter 3
        bindings.insert(
            3u16,
            make_binding(
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                &val as *const i32 as sql::Pointer,
                0,
                std::ptr::null_mut(),
            ),
        );

        let result = odbc_bindings_to_json(&bindings);
        assert!(result.is_err());
    }
}
