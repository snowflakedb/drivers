use std::{
    ffi::{CStr, c_char},
    mem, slice, str,
};

use serde_json::{Map, Value};
use snafu::ResultExt;

use crate::api::CDataType;
use crate::api::{ApdDescriptor, IpdDescriptor, ParameterBinding};
use odbc_sys as sql;

use super::binary::SnowflakeBinary;
use super::boolean::SnowflakeBoolean;
use super::date::SnowflakeDate;
use super::error::{
    InvalidParameterIndicesSnafu, InvalidUtf8Snafu, JsonBindingError, NullPointerSnafu,
    SerializationSnafu, UnsupportedParameterTypeSnafu, WCharConversionSnafu,
};
use super::number::{NumericSqlType, SnowflakeNumber};
use super::real::SnowflakeReal;
use super::time::SnowflakeTime;
use super::timestamp::SnowflakeTimestampNtz;
use super::traits::{ReadODBC, SnowflakeLogicalType, WriteJson};
use super::varchar::SnowflakeVarchar;

// =============================================================================
// ParamConverter trait (public interface)
// =============================================================================

/// Trait for converting an ODBC parameter binding into the Snowflake JSON
/// binding format (`sf_type`, `Value`).
pub(crate) trait ParamConverter {
    fn convert(
        &self,
        binding: &ParameterBinding,
    ) -> Result<(SnowflakeLogicalType, Value), JsonBindingError>;
}

/// Generic adapter: any type implementing `ReadODBC + WriteJson` automatically
/// gets a `ParamConverter` implementation via this wrapper.
struct JsonParamConverter<T: ReadODBC + WriteJson> {
    snowflake_type: T,
}

impl<T: ReadODBC + WriteJson> ParamConverter for JsonParamConverter<T> {
    fn convert(
        &self,
        binding: &ParameterBinding,
    ) -> Result<(SnowflakeLogicalType, Value), JsonBindingError> {
        let value = self.snowflake_type.read_odbc(binding)?;
        let json_value = self.snowflake_type.write_json(value)?;
        Ok((self.snowflake_type.sf_type(), json_value))
    }
}

/// Parameter-only converter for SQL_DECIMAL/SQL_NUMERIC: reads the value as a
/// string (like varchar) but reports the Snowflake type as FIXED so the server
/// applies numeric semantics.
struct DecimalParamConverter;

impl ParamConverter for DecimalParamConverter {
    fn convert(
        &self,
        binding: &ParameterBinding,
    ) -> Result<(SnowflakeLogicalType, Value), JsonBindingError> {
        let s = match binding.value_type {
            CDataType::Char => read_char_str(binding)?,
            CDataType::WChar => read_wchar_str(binding)?,
            CDataType::Long | CDataType::SLong => read_unaligned::<i32>(binding).to_string(),
            CDataType::Short | CDataType::SShort => read_unaligned::<i16>(binding).to_string(),
            CDataType::SBigInt => read_unaligned::<i64>(binding).to_string(),
            CDataType::ULong => read_unaligned::<u32>(binding).to_string(),
            CDataType::UShort => read_unaligned::<u16>(binding).to_string(),
            CDataType::UBigInt => read_unaligned::<u64>(binding).to_string(),
            CDataType::TinyInt | CDataType::STinyInt => read_unaligned::<i8>(binding).to_string(),
            CDataType::UTinyInt => read_unaligned::<u8>(binding).to_string(),
            CDataType::Double => read_unaligned::<f64>(binding).to_string(),
            CDataType::Float => read_unaligned::<f32>(binding).to_string(),
            _ => {
                return Err(UnsupportedParameterTypeSnafu {
                    sql_type: sql::SqlDataType::DECIMAL,
                }
                .build());
            }
        };
        Ok((SnowflakeLogicalType::Fixed, Value::String(s)))
    }
}

// =============================================================================
// Factory
// =============================================================================

/// Select the appropriate `ParamConverter` for the given SQL data type.
/// The SQL type determines the Snowflake logical type, which in turn knows
/// how to read various C data types from the ODBC buffer.
fn make_converter(
    sql_type: &sql::SqlDataType,
) -> Result<Box<dyn ParamConverter>, JsonBindingError> {
    match *sql_type {
        sql::SqlDataType::INTEGER
        | sql::SqlDataType::SMALLINT
        | sql::SqlDataType::EXT_BIG_INT
        | sql::SqlDataType::EXT_TINY_INT => Ok(Box::new(JsonParamConverter {
            snowflake_type: SnowflakeNumber {
                scale: 0,
                precision: 19,
                sql_type: NumericSqlType::BigInt,
            },
        })),

        sql::SqlDataType::REAL | sql::SqlDataType::FLOAT | sql::SqlDataType::DOUBLE => {
            Ok(Box::new(JsonParamConverter {
                snowflake_type: SnowflakeReal,
            }))
        }

        sql::SqlDataType::VARCHAR
        | sql::SqlDataType::CHAR
        | sql::SqlDataType::EXT_LONG_VARCHAR
        | sql::SqlDataType::EXT_W_CHAR
        | sql::SqlDataType::EXT_W_VARCHAR
        | sql::SqlDataType::EXT_W_LONG_VARCHAR => Ok(Box::new(JsonParamConverter {
            snowflake_type: SnowflakeVarchar { len: 0 },
        })),

        sql::SqlDataType::DECIMAL | sql::SqlDataType::NUMERIC => {
            Ok(Box::new(DecimalParamConverter))
        }

        sql::SqlDataType::EXT_BIT => Ok(Box::new(JsonParamConverter {
            snowflake_type: SnowflakeBoolean,
        })),

        sql::SqlDataType::EXT_BINARY
        | sql::SqlDataType::EXT_VAR_BINARY
        | sql::SqlDataType::EXT_LONG_VAR_BINARY => Ok(Box::new(JsonParamConverter {
            snowflake_type: SnowflakeBinary { len: 0 },
        })),

        sql::SqlDataType::DATE => Ok(Box::new(JsonParamConverter {
            snowflake_type: SnowflakeDate,
        })),

        sql::SqlDataType::TIME => Ok(Box::new(JsonParamConverter {
            snowflake_type: SnowflakeTime { scale: 9 },
        })),

        sql::SqlDataType::TIMESTAMP | sql::SqlDataType::EXT_TIMESTAMP => {
            Ok(Box::new(JsonParamConverter {
                snowflake_type: SnowflakeTimestampNtz,
            }))
        }

        _ => {
            tracing::error!("Unsupported SQL data type for JSON binding: {:?}", sql_type);
            UnsupportedParameterTypeSnafu {
                sql_type: *sql_type,
            }
            .fail()
        }
    }
}

// =============================================================================
// Pipeline
// =============================================================================

/// Convert ODBC parameter bindings (from APD + IPD descriptors) to JSON
/// string format for server-side binding.
///
/// # Safety contract
/// The APD records' `data_ptr` pointers must remain valid for the duration
/// of this call. If `str_len_or_ind_ptr` is non-null, it must also point to
/// valid memory for reads.
///
/// Returns a JSON string in the format:
/// ```json
/// {
///   "1": {"type": "FIXED", "value": "123"},
///   "2": {"type": "TEXT", "value": "hello"}
/// }
/// ```
pub fn odbc_bindings_to_json(
    apd: &ApdDescriptor,
    ipd: &IpdDescriptor,
) -> Result<String, JsonBindingError> {
    let mut json_bindings = Map::new();

    let max_key = apd.desc_count().max(ipd.desc_count());

    for param_num in 1..=max_key {
        let apd_rec = apd.records.get(&param_num).ok_or_else(|| {
            tracing::error!(
                "odbc_bindings_to_json: APD record #{param_num} not found. \
                 Parameter bindings must be contiguous and start at 1.",
            );
            InvalidParameterIndicesSnafu.build()
        })?;
        let ipd_rec = ipd.records.get(&param_num).ok_or_else(|| {
            tracing::error!(
                "odbc_bindings_to_json: IPD record #{param_num} not found. \
                 Parameter bindings must be contiguous and start at 1.",
            );
            InvalidParameterIndicesSnafu.build()
        })?;

        let binding = ParameterBinding::from_apd_ipd(apd_rec, ipd_rec);

        let (snowflake_type, json_value) = if is_null_indicator(&binding) {
            (SnowflakeLogicalType::Any, Value::Null)
        } else {
            if binding.parameter_value_ptr.is_null() {
                return NullPointerSnafu.fail();
            }
            let converter = make_converter(&binding.sql_data_type)?;
            converter.convert(&binding)?
        };

        let mut binding_obj = Map::new();
        binding_obj.insert(
            "type".to_string(),
            Value::String(snowflake_type.as_str().to_string()),
        );
        binding_obj.insert("value".to_string(), json_value);

        json_bindings.insert(param_num.to_string(), Value::Object(binding_obj));
    }

    serde_json::to_string(&Value::Object(json_bindings)).context(SerializationSnafu)
}

// =============================================================================
// Helpers — raw pointer reads
// =============================================================================

fn is_null_indicator(binding: &ParameterBinding) -> bool {
    !binding.str_len_or_ind_ptr.is_null()
        && unsafe { *binding.str_len_or_ind_ptr == sql::NULL_DATA }
}

/// Read a fixed-size value using `read_unaligned` for ODBC pointer safety.
pub(crate) fn read_unaligned<T: Copy>(binding: &ParameterBinding) -> T {
    unsafe { std::ptr::read_unaligned(binding.parameter_value_ptr as *const T) }
}

/// Determine the actual byte length of buffer data, using the length/indicator
/// pointer if available, falling back to `buffer_length`.
///
/// Negative `buffer_length` values (e.g. `SQL_NTS`) are treated as zero.
/// Indicated length is clamped to `buffer_length` to prevent over-reads.
pub(crate) fn buffer_data_len(binding: &ParameterBinding) -> usize {
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

/// Convert bytes from the system's ANSI code page to a Rust UTF-8 `String`.
///
/// On Windows, SQL_C_CHAR data uses the active ANSI code page (ACP), which may
/// not be UTF-8. We call `MultiByteToWideChar(CP_ACP, …)` to widen to UTF-16,
/// then convert the UTF-16 to a Rust `String`.
#[cfg(windows)]
fn acp_bytes_to_string(bytes: &[u8]) -> Result<String, JsonBindingError> {
    if bytes.is_empty() {
        return Ok(String::new());
    }

    use std::ptr;

    unsafe extern "system" {
        fn MultiByteToWideChar(
            code_page: u32,
            dw_flags: u32,
            lp_multi_byte_str: *const u8,
            cb_multi_byte: i32,
            lp_wide_char_str: *mut u16,
            cch_wide_char: i32,
        ) -> i32;
    }

    const CP_ACP: u32 = 0;

    let result = unsafe {
        let wide_len = MultiByteToWideChar(
            CP_ACP,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            ptr::null_mut(),
            0,
        );
        if wide_len <= 0 {
            return AcpConversionSnafu.fail();
        }

        let mut wide_buf = vec![0u16; wide_len as usize];
        let written = MultiByteToWideChar(
            CP_ACP,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            wide_buf.as_mut_ptr(),
            wide_len,
        );
        if written <= 0 {
            return AcpConversionSnafu.fail();
        }

        String::from_utf16(&wide_buf[..written as usize]).map_err(|_| AcpConversionSnafu.build())
    };
    result
}

#[cfg(not(windows))]
fn acp_bytes_to_string(bytes: &[u8]) -> Result<String, JsonBindingError> {
    str::from_utf8(bytes)
        .context(InvalidUtf8Snafu)
        .map(|s| s.to_string())
}

#[cfg(windows)]
use super::error::AcpConversionSnafu;

/// Read a SQL_C_CHAR value, converting from the system ANSI code page to UTF-8.
///
/// Per ODBC spec: when the indicator is SQL_NTS or the indicator pointer is
/// NULL, character data is null-terminated. Otherwise we use the indicated
/// length (clamped to buffer_length).
pub(crate) fn read_char_str(binding: &ParameterBinding) -> Result<String, JsonBindingError> {
    let null_terminated =
        binding.str_len_or_ind_ptr.is_null() || unsafe { *binding.str_len_or_ind_ptr } == sql::NTS;

    let bytes = if null_terminated {
        unsafe { CStr::from_ptr(binding.parameter_value_ptr as *const c_char).to_bytes() }
    } else {
        let len = buffer_data_len(binding);
        unsafe { slice::from_raw_parts(binding.parameter_value_ptr as *const u8, len) }
    };

    acp_bytes_to_string(bytes)
}

/// Read a SQL_C_WCHAR (UTF-16) value and convert to a UTF-8 string.
pub(crate) fn read_wchar_str(binding: &ParameterBinding) -> Result<String, JsonBindingError> {
    let byte_len = buffer_data_len(binding);
    let unit_len = byte_len / mem::size_of::<u16>();
    let units =
        unsafe { slice::from_raw_parts(binding.parameter_value_ptr as *const u16, unit_len) };
    String::from_utf16(units).map_err(|_| WCharConversionSnafu.build())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::CDataType;
    use crate::api::{ApdRecord, IpdRecord};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn make_binding(
        value_type: CDataType,
        parameter_type: sql::SqlDataType,
        ptr: sql::Pointer,
        buffer_length: sql::Len,
        ind_ptr: *mut sql::Len,
    ) -> ParameterBinding {
        ParameterBinding {
            sql_data_type: parameter_type,
            value_type,
            parameter_value_ptr: ptr,
            buffer_length,
            str_len_or_ind_ptr: ind_ptr,
        }
    }

    fn make_descriptors(
        params: Vec<(
            u16,
            CDataType,
            sql::SqlDataType,
            sql::Pointer,
            sql::Len,
            *mut sql::Len,
        )>,
    ) -> (ApdDescriptor, IpdDescriptor) {
        let mut apd = ApdDescriptor::new();
        let mut ipd = IpdDescriptor::new();
        for (num, value_type, parameter_type, ptr, buf_len, ind_ptr) in params {
            apd.records.insert(
                num,
                ApdRecord {
                    value_type,
                    data_ptr: ptr,
                    buffer_length: buf_len,
                    str_len_or_ind_ptr: ind_ptr,
                },
            );
            ipd.records.insert(
                num,
                IpdRecord {
                    sql_data_type: parameter_type,
                    ..IpdRecord::default()
                },
            );
        }
        (apd, ipd)
    }

    fn convert_binding(
        binding: &ParameterBinding,
    ) -> Result<(SnowflakeLogicalType, Value), JsonBindingError> {
        let converter = make_converter(&binding.sql_data_type)?;
        converter.convert(binding)
    }

    // -- ParamConverter tests per type ----------------------------------------

    #[test]
    fn convert_integer_i32() -> TestResult {
        let val: i32 = 42;
        let binding = make_binding(
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("42".to_string()));
        Ok(())
    }

    #[test]
    fn convert_integer_i16() -> TestResult {
        let val: i16 = -7;
        let binding = make_binding(
            CDataType::Short,
            sql::SqlDataType::SMALLINT,
            &val as *const i16 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("-7".to_string()));
        Ok(())
    }

    #[test]
    fn convert_integer_i64() -> TestResult {
        let val: i64 = 9_999_999_999;
        let binding = make_binding(
            CDataType::SBigInt,
            sql::SqlDataType::EXT_BIG_INT,
            &val as *const i64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("9999999999".to_string()));
        Ok(())
    }

    #[test]
    fn convert_unsigned_u32() -> TestResult {
        let val: u32 = 4_000_000_000;
        let binding = make_binding(
            CDataType::ULong,
            sql::SqlDataType::INTEGER,
            &val as *const u32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("4000000000".to_string()));
        Ok(())
    }

    #[test]
    fn convert_unsigned_u16() -> TestResult {
        let val: u16 = 65535;
        let binding = make_binding(
            CDataType::UShort,
            sql::SqlDataType::SMALLINT,
            &val as *const u16 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("65535".to_string()));
        Ok(())
    }

    #[test]
    fn convert_unsigned_u64() -> TestResult {
        let val: u64 = 1_000_000_000_000;
        let binding = make_binding(
            CDataType::UBigInt,
            sql::SqlDataType::EXT_BIG_INT,
            &val as *const u64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("1000000000000".to_string()));
        Ok(())
    }

    #[test]
    fn convert_unsigned_u8() -> TestResult {
        let val: u8 = 255;
        let binding = make_binding(
            CDataType::UTinyInt,
            sql::SqlDataType::EXT_TINY_INT,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("255".to_string()));
        Ok(())
    }

    #[test]
    fn convert_signed_i8() -> TestResult {
        let val: i8 = -128;
        let binding = make_binding(
            CDataType::STinyInt,
            sql::SqlDataType::EXT_TINY_INT,
            &val as *const i8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("-128".to_string()));
        Ok(())
    }

    #[test]
    fn convert_float_f64() -> TestResult {
        let val: f64 = 1.234;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::DOUBLE,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, Value::String("1.234".to_string()));
        Ok(())
    }

    #[test]
    fn convert_float_f32() -> TestResult {
        let val: f32 = 1.5;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::REAL,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert!(v.as_str().unwrap().starts_with("1.5"));
        Ok(())
    }

    #[test]
    fn convert_char_nts() -> TestResult {
        let val = b"hello\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::VARCHAR,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, Value::String("hello".to_string()));
        Ok(())
    }

    #[test]
    fn convert_char_with_length() -> TestResult {
        let val = b"hello world";
        let mut ind: sql::Len = 5;
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::VARCHAR,
            val.as_ptr() as sql::Pointer,
            11,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, Value::String("hello".to_string()));
        Ok(())
    }

    #[test]
    fn convert_bit_true() -> TestResult {
        let val: u8 = 1;
        let binding = make_binding(
            CDataType::Bit,
            sql::SqlDataType::EXT_BIT,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_bit_false() -> TestResult {
        let val: u8 = 0;
        let binding = make_binding(
            CDataType::Bit,
            sql::SqlDataType::EXT_BIT,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary() -> TestResult {
        let val: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
        let mut ind: sql::Len = 4;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_BINARY,
            val.as_ptr() as sql::Pointer,
            4,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Binary);
        assert_eq!(v, Value::String("deadbeef".to_string()));
        Ok(())
    }

    #[test]
    fn convert_null_data() -> TestResult {
        let mut ind: sql::Len = sql::NULL_DATA;
        let (apd, ipd) = make_descriptors(vec![(
            1,
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            std::ptr::null_mut(),
            0,
            &mut ind,
        )]);
        let json = odbc_bindings_to_json(&apd, &ipd)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["1"]["type"], "ANY");
        assert!(parsed["1"]["value"].is_null());
        Ok(())
    }

    #[test]
    fn convert_null_pointer_without_indicator_fails() {
        let (apd, ipd) = make_descriptors(vec![(
            1,
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        )]);
        assert!(odbc_bindings_to_json(&apd, &ipd).is_err());
    }

    #[test]
    fn convert_unsupported_sql_type() {
        let val: i32 = 1;
        let binding = make_binding(
            CDataType::Long,
            sql::SqlDataType(9999),
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(make_converter(&binding.sql_data_type).is_err());
    }

    // -- end-to-end pipeline tests -------------------------------------------

    #[test]
    fn pipeline_integer_binding() -> TestResult {
        let val: i32 = 99;
        let binding = make_binding(
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, json_val) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(json_val, Value::String("99".to_string()));
        Ok(())
    }

    #[test]
    fn pipeline_full_json_output() -> TestResult {
        let val: i32 = 7;
        let (apd, ipd) = make_descriptors(vec![(
            1,
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        )]);
        let json = odbc_bindings_to_json(&apd, &ipd)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["1"]["type"], "FIXED");
        assert_eq!(parsed["1"]["value"], "7");
        Ok(())
    }

    #[test]
    fn pipeline_null_json_output() -> TestResult {
        let mut ind: sql::Len = sql::NULL_DATA;
        let (apd, ipd) = make_descriptors(vec![(
            1,
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            std::ptr::null_mut(),
            0,
            &mut ind,
        )]);
        let json = odbc_bindings_to_json(&apd, &ipd)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["1"]["type"], "ANY");
        assert!(parsed["1"]["value"].is_null());
        Ok(())
    }

    #[test]
    fn pipeline_non_contiguous_params_error() {
        let val: i32 = 1;
        let (mut apd, mut ipd) = make_descriptors(vec![(
            1,
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        )]);
        apd.records.insert(
            3,
            ApdRecord {
                value_type: CDataType::Long,
                data_ptr: &val as *const i32 as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
            },
        );
        ipd.records.insert(
            3,
            IpdRecord {
                sql_data_type: sql::SqlDataType::INTEGER,
                ..IpdRecord::default()
            },
        );
        assert!(odbc_bindings_to_json(&apd, &ipd).is_err());
    }

    #[test]
    fn convert_char_as_integer() -> TestResult {
        let val = b"12345\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::INTEGER,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("12345".to_string()));
        Ok(())
    }

    #[test]
    fn convert_char_as_real() -> TestResult {
        let val = b"3.14\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, Value::String("3.14".to_string()));
        Ok(())
    }
}
