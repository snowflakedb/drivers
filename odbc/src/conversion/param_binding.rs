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
    BindingNumericOutOfRangeSnafu, InvalidParameterIndicesSnafu, InvalidUtf8Snafu,
    JsonBindingError, NullPointerSnafu, NumericMagnitudeOverflowSnafu, SerializationSnafu,
    UnsupportedParameterTypeSnafu, WCharConversionSnafu,
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
            CDataType::Bit => read_unaligned::<u8>(binding).to_string(),
            CDataType::Numeric => {
                let (value, scale) = read_numeric_struct(binding)?;
                format_numeric_value(value, scale)
            }
            CDataType::Binary => {
                let len = buffer_data_len(binding);
                if len == std::mem::size_of::<sql::Numeric>() {
                    let (value, scale) = read_numeric_struct(binding)?;
                    format_numeric_value(value, scale)
                } else {
                    return Err(BindingNumericOutOfRangeSnafu {
                        reason: format!(
                            "SQL_C_BINARY buffer length {len} does not match SQL_NUMERIC_STRUCT size ({})",
                            std::mem::size_of::<sql::Numeric>()
                        ),
                    }
                    .build());
                }
            }
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
            snowflake_type: SnowflakeVarchar {
                len: 0,
                is_semi_structured: false,
            },
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
                snowflake_type: SnowflakeTimestampNtz { scale: 9 },
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
    max_params: u16,
) -> Result<String, JsonBindingError> {
    let mut json_bindings = Map::new();

    for param_num in 1..=max_params {
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

/// Read and decode an `SQL_NUMERIC_STRUCT` from the parameter buffer.
///
/// Returns `(signed_value, scale)` where `signed_value` is the integer
/// mantissa with sign applied, and `scale` is the number of decimal digits
/// after the point. The caller divides by `10^scale` to recover the true
/// numeric value.
///
/// Returns an error if the magnitude exceeds the representable `i128` range.
pub(crate) fn read_numeric_struct(
    binding: &ParameterBinding,
) -> Result<(i128, i8), JsonBindingError> {
    let ns = read_unaligned::<sql::Numeric>(binding);
    let magnitude = u128::from_le_bytes(ns.val);
    let negative_min_magnitude = (i128::MAX as u128) + 1;
    let signed = if ns.sign == 0 {
        if magnitude == negative_min_magnitude {
            i128::MIN
        } else if magnitude <= i128::MAX as u128 {
            -(magnitude as i128)
        } else {
            return NumericMagnitudeOverflowSnafu {
                reason: format!(
                    "SQL_NUMERIC_STRUCT magnitude {magnitude} exceeds i128 negative range"
                ),
            }
            .fail();
        }
    } else if ns.sign == 1 {
        if magnitude <= i128::MAX as u128 {
            magnitude as i128
        } else {
            return NumericMagnitudeOverflowSnafu {
                reason: format!(
                    "SQL_NUMERIC_STRUCT magnitude {magnitude} exceeds i128 positive range"
                ),
            }
            .fail();
        }
    } else {
        return NumericMagnitudeOverflowSnafu {
            reason: format!(
                "SQL_NUMERIC_STRUCT sign {} is invalid; expected 0 or 1",
                ns.sign
            ),
        }
        .fail();
    };
    Ok((signed, ns.scale))
}

/// Format a scaled integer value into its decimal string representation.
/// For example, `(12345, 2)` becomes `"123.45"`.
///
/// Uses string manipulation rather than arithmetic scaling to avoid
/// overflow when `value` is large or `scale` is very negative.
pub(crate) fn format_numeric_value(value: i128, scale: i8) -> String {
    if scale == 0 {
        return value.to_string();
    }

    let is_negative = value < 0;
    let abs = value.unsigned_abs();
    let mut s = abs.to_string();

    if scale < 0 {
        let trailing_zeros = if scale == i8::MIN {
            (i8::MAX as usize) + 1
        } else {
            (-scale) as usize
        };
        s.extend(std::iter::repeat_n('0', trailing_zeros));
        if is_negative {
            s.insert(0, '-');
        }
        return s;
    }

    let scale = scale as usize;
    while s.len() <= scale {
        s.insert(0, '0');
    }
    let decimal_pos = s.len() - scale;
    s.insert(decimal_pos, '.');
    if is_negative {
        s.insert(0, '-');
    }
    s
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

/// Read a fixed-size POD struct `T` from an `SQL_C_BINARY` parameter buffer,
/// rejecting buffers whose length does not exactly match `size_of::<T>()`.
///
/// `struct_name` is used only to produce a descriptive error message
/// (e.g. `"SQL_DATE_STRUCT"`) when the length check fails.
pub(crate) fn read_binary_struct<T: Copy>(
    binding: &ParameterBinding,
    struct_name: &str,
) -> Result<T, JsonBindingError> {
    let len = buffer_data_len(binding);
    let expected = std::mem::size_of::<T>();
    if len != expected {
        return BindingNumericOutOfRangeSnafu {
            reason: format!(
                "SQL_C_BINARY buffer length {len} does not match {struct_name} size ({expected})"
            ),
        }
        .fail();
    }
    Ok(read_unaligned::<T>(binding))
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
///
/// When `StrLen_or_IndPtr` is NULL or points to `SQL_NTS`, the buffer is
/// treated as null-terminated (scans for the first `0x0000` code unit).
/// Otherwise the indicated byte length is used (clamped to `buffer_length`).
pub(crate) fn read_wchar_str(binding: &ParameterBinding) -> Result<String, JsonBindingError> {
    let null_terminated =
        binding.str_len_or_ind_ptr.is_null() || unsafe { *binding.str_len_or_ind_ptr } == sql::NTS;

    let units = if null_terminated {
        let ptr = binding.parameter_value_ptr as *const u16;
        let max_units = if binding.buffer_length > 0 {
            binding.buffer_length as usize / mem::size_of::<u16>()
        } else {
            usize::MAX
        };
        let mut len = 0;
        unsafe {
            while len < max_units && *ptr.add(len) != 0 {
                len += 1;
            }
            slice::from_raw_parts(ptr, len)
        }
    } else {
        let byte_len = buffer_data_len(binding);
        let unit_len = byte_len / mem::size_of::<u16>();
        unsafe { slice::from_raw_parts(binding.parameter_value_ptr as *const u16, unit_len) }
    };
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

    // -- read_wchar_str tests -------------------------------------------------

    #[test]
    fn read_wchar_str_with_explicit_length() -> TestResult {
        let data: [u16; 4] = ['h' as u16, 'i' as u16, '!' as u16, 0];
        let mut ind: sql::Len = 3 * mem::size_of::<u16>() as sql::Len;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::VARCHAR,
            data.as_ptr() as sql::Pointer,
            (4 * mem::size_of::<u16>()) as sql::Len,
            &mut ind,
        );
        assert_eq!(read_wchar_str(&binding)?, "hi!");
        Ok(())
    }

    #[test]
    fn read_wchar_str_with_sql_nts() -> TestResult {
        let data: [u16; 4] = ['h' as u16, 'i' as u16, '!' as u16, 0];
        let mut ind: sql::Len = sql::NTS;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::VARCHAR,
            data.as_ptr() as sql::Pointer,
            (4 * mem::size_of::<u16>()) as sql::Len,
            &mut ind,
        );
        assert_eq!(read_wchar_str(&binding)?, "hi!");
        Ok(())
    }

    #[test]
    fn read_wchar_str_with_null_indicator() -> TestResult {
        let data: [u16; 4] = ['h' as u16, 'i' as u16, '!' as u16, 0];
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::VARCHAR,
            data.as_ptr() as sql::Pointer,
            (4 * mem::size_of::<u16>()) as sql::Len,
            std::ptr::null_mut(),
        );
        assert_eq!(read_wchar_str(&binding)?, "hi!");
        Ok(())
    }

    #[test]
    fn read_wchar_str_sql_nts_zero_buffer_length() -> TestResult {
        let data: [u16; 4] = ['h' as u16, 'i' as u16, '!' as u16, 0];
        let mut ind: sql::Len = sql::NTS;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::VARCHAR,
            data.as_ptr() as sql::Pointer,
            0,
            &mut ind,
        );
        assert_eq!(read_wchar_str(&binding)?, "hi!");
        Ok(())
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

    // -- C types → BOOLEAN (SQL_BIT) ------------------------------------------

    #[test]
    fn convert_char_to_boolean_true() -> TestResult {
        let val = b"1\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_char_to_boolean_false() -> TestResult {
        let val = b"0\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_char_true_string_to_boolean() -> TestResult {
        let val = b"true\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_char_false_string_to_boolean() -> TestResult {
        let val = b"false\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_wchar_to_boolean_true() -> TestResult {
        let val: [u16; 1] = [b'1' as u16];
        let mut ind: sql::Len = 2;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            2,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_wchar_to_boolean_false() -> TestResult {
        let val: [u16; 1] = [b'0' as u16];
        let mut ind: sql::Len = 2;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            2,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_slong_to_boolean_true() -> TestResult {
        let val: i32 = 42;
        let binding = make_binding(
            CDataType::SLong,
            sql::SqlDataType::EXT_BIT,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_slong_to_boolean_false() -> TestResult {
        let val: i32 = 0;
        let binding = make_binding(
            CDataType::SLong,
            sql::SqlDataType::EXT_BIT,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_sbigint_to_boolean_true() -> TestResult {
        let val: i64 = -1;
        let binding = make_binding(
            CDataType::SBigInt,
            sql::SqlDataType::EXT_BIT,
            &val as *const i64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_double_to_boolean_true() -> TestResult {
        let val: f64 = 1.5;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::EXT_BIT,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_double_to_boolean_false() -> TestResult {
        let val: f64 = 0.0;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::EXT_BIT,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_float_to_boolean_true() -> TestResult {
        let val: f32 = 0.5;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_numeric_to_boolean_true() -> TestResult {
        let n = sql::Numeric {
            precision: 10,
            scale: 0,
            sign: 1,
            val: 1u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::EXT_BIT,
            &n as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_numeric_to_boolean_false() -> TestResult {
        let n = sql::Numeric {
            precision: 10,
            scale: 0,
            sign: 1,
            val: 0u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::EXT_BIT,
            &n as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_to_boolean_true() -> TestResult {
        let val: [u8; 1] = [0x01];
        let mut ind: sql::Len = 1;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            1,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_to_boolean_false() -> TestResult {
        let val: [u8; 1] = [0x00];
        let mut ind: sql::Len = 1;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            1,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_stinyint_to_boolean_true() -> TestResult {
        let val: i8 = -1;
        let binding = make_binding(
            CDataType::STinyInt,
            sql::SqlDataType::EXT_BIT,
            &val as *const i8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_stinyint_to_boolean_false() -> TestResult {
        let val: i8 = 0;
        let binding = make_binding(
            CDataType::STinyInt,
            sql::SqlDataType::EXT_BIT,
            &val as *const i8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_utinyint_to_boolean_true() -> TestResult {
        let val: u8 = 255;
        let binding = make_binding(
            CDataType::UTinyInt,
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
    fn convert_ulong_to_boolean_true() -> TestResult {
        let val: u32 = 1;
        let binding = make_binding(
            CDataType::ULong,
            sql::SqlDataType::EXT_BIT,
            &val as *const u32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_ulong_to_boolean_false() -> TestResult {
        let val: u32 = 0;
        let binding = make_binding(
            CDataType::ULong,
            sql::SqlDataType::EXT_BIT,
            &val as *const u32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_ushort_to_boolean_true() -> TestResult {
        let val: u16 = 1;
        let binding = make_binding(
            CDataType::UShort,
            sql::SqlDataType::EXT_BIT,
            &val as *const u16 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_ushort_to_boolean_false() -> TestResult {
        let val: u16 = 0;
        let binding = make_binding(
            CDataType::UShort,
            sql::SqlDataType::EXT_BIT,
            &val as *const u16 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_ubigint_to_boolean_true() -> TestResult {
        let val: u64 = 1;
        let binding = make_binding(
            CDataType::UBigInt,
            sql::SqlDataType::EXT_BIT,
            &val as *const u64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_ubigint_to_boolean_false() -> TestResult {
        let val: u64 = 0;
        let binding = make_binding(
            CDataType::UBigInt,
            sql::SqlDataType::EXT_BIT,
            &val as *const u64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_float_to_boolean_false() -> TestResult {
        let val: f32 = 0.0;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_float_nan_to_boolean_fails() {
        let val: f32 = f32::NAN;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Float NaN should not convert to boolean"
        );
    }

    #[test]
    fn convert_float_inf_to_boolean_fails() {
        let val: f32 = f32::INFINITY;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Float infinity should not convert to boolean"
        );
    }

    #[test]
    fn convert_double_nan_to_boolean_fails() {
        let val: f64 = f64::NAN;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::EXT_BIT,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Double NaN should not convert to boolean"
        );
    }

    #[test]
    fn convert_double_inf_to_boolean_fails() {
        let val: f64 = f64::INFINITY;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::EXT_BIT,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Double infinity should not convert to boolean"
        );
    }

    #[test]
    fn convert_double_neg_inf_to_boolean_fails() {
        let val: f64 = f64::NEG_INFINITY;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::EXT_BIT,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Double -infinity should not convert to boolean"
        );
    }

    #[test]
    fn convert_slong_negative_to_boolean_true() -> TestResult {
        let val: i32 = -99;
        let binding = make_binding(
            CDataType::SLong,
            sql::SqlDataType::EXT_BIT,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_char_nan_to_boolean_fails() {
        let val = b"NaN\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "NaN should not be accepted as a boolean value"
        );
    }

    #[test]
    fn convert_char_inf_to_boolean_fails() {
        let val = b"inf\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "inf should not be accepted as a boolean value"
        );
    }

    #[test]
    fn convert_char_garbage_to_boolean_fails() {
        let val = b"hello\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Non-numeric non-boolean string should fail"
        );
    }

    #[test]
    fn convert_sshort_to_boolean_false() -> TestResult {
        let val: i16 = 0;
        let binding = make_binding(
            CDataType::SShort,
            sql::SqlDataType::EXT_BIT,
            &val as *const i16 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_sbigint_to_boolean_false() -> TestResult {
        let val: i64 = 0;
        let binding = make_binding(
            CDataType::SBigInt,
            sql::SqlDataType::EXT_BIT,
            &val as *const i64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_char_numeric_nonzero_to_boolean() -> TestResult {
        let val = b"42\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_char_negative_to_boolean() -> TestResult {
        let val = b"-1\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_char_float_string_to_boolean() -> TestResult {
        let val = b"0.5\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_char_float_zero_string_to_boolean() -> TestResult {
        let val = b"0.0\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_char_neg_zero_string_to_boolean() -> TestResult {
        let val = b"-0.0\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_float_neg_zero_to_boolean_false() -> TestResult {
        let val: f32 = -0.0;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_double_neg_zero_to_boolean_false() -> TestResult {
        let val: f64 = -0.0;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::EXT_BIT,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_float_neg_inf_to_boolean_fails() {
        let val: f32 = f32::NEG_INFINITY;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Float -infinity should not convert to boolean"
        );
    }

    #[test]
    fn convert_binary_multibyte_to_boolean_fails() {
        let val: [u8; 3] = [0x00, 0x01, 0x00];
        let mut ind: sql::Len = 3;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            3,
            &mut ind,
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Multi-byte binary should be rejected for SQL_BIT (ODBC spec: len must equal 1)"
        );
    }

    #[test]
    fn convert_binary_empty_to_boolean_fails() {
        let val: [u8; 0] = [];
        let mut ind: sql::Len = 0;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            0,
            &mut ind,
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Empty binary should be rejected for SQL_BIT (ODBC spec: len must equal 1)"
        );
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
        let json = odbc_bindings_to_json(&apd, &ipd, apd.desc_count().max(ipd.desc_count()))?;
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
        assert!(odbc_bindings_to_json(&apd, &ipd, apd.desc_count().max(ipd.desc_count())).is_err());
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
        let json = odbc_bindings_to_json(&apd, &ipd, apd.desc_count().max(ipd.desc_count()))?;
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
        let json = odbc_bindings_to_json(&apd, &ipd, apd.desc_count().max(ipd.desc_count()))?;
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
        assert!(odbc_bindings_to_json(&apd, &ipd, apd.desc_count().max(ipd.desc_count())).is_err());
    }

    #[test]
    fn max_params_zero_skips_phantom_dae_binding() -> TestResult {
        // Simulates: SQLPrepare("SELECT 1") → 0 markers, then
        // SQLBindParameter(1, ..., (SQLPOINTER)1, ..., SQL_DATA_AT_EXEC).
        // The DM may or may not strip phantom bindings, so we test the
        // serializer directly. With max_params=0 the dummy pointer at
        // address 0x1 must never be dereferenced.
        let mut dae_ind: sql::Len = sql::DATA_AT_EXEC;
        let (apd, ipd) = make_descriptors(vec![(
            1,
            CDataType::Char,
            sql::SqlDataType::VARCHAR,
            1usize as sql::Pointer, // dummy DAE token, not a real address
            0,
            &mut dae_ind,
        )]);
        let json = odbc_bindings_to_json(&apd, &ipd, 0)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed, serde_json::json!({}));
        Ok(())
    }

    #[test]
    fn max_params_caps_serialization_to_valid_range() -> TestResult {
        // Simulates: SQLPrepare("SELECT ?") → 1 marker, then two bindings:
        //   param 1 = valid integer
        //   param 2 = phantom DAE bind with dummy pointer
        // With max_params=1, only param 1 is serialized; the dummy pointer
        // for param 2 is never touched.
        let val: i32 = 42;
        let mut dae_ind: sql::Len = sql::DATA_AT_EXEC;
        let (apd, ipd) = make_descriptors(vec![
            (
                1,
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                &val as *const i32 as sql::Pointer,
                0,
                std::ptr::null_mut(),
            ),
            (
                2,
                CDataType::Char,
                sql::SqlDataType::VARCHAR,
                1usize as sql::Pointer, // dummy DAE token
                0,
                &mut dae_ind,
            ),
        ]);
        let json = odbc_bindings_to_json(&apd, &ipd, 1)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["1"]["type"], "FIXED");
        assert_eq!(parsed["1"]["value"], "42");
        assert!(parsed.get("2").is_none());
        Ok(())
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

    // Non-finite numeric-literal rejection (SQLSTATE 22018)
    //
    // Rust's f64::from_str accepts "Infinity", "-Infinity" and "NaN", but the
    // ODBC "numeric-literal" grammar (MS ODBC spec, Appendix C) does not
    // permit these tokens. The driver rejects them client-side so the caller
    // sees InvalidCharacterValueForCast instead of a value that only works
    // for SQL_REAL/SQL_DOUBLE targets.

    #[test]
    fn convert_char_infinity_as_real_rejected() {
        let val = b"Infinity\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_char_neg_infinity_as_real_rejected() {
        let val = b"-Infinity\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_char_nan_as_real_rejected() {
        let val = b"NaN\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_wchar_infinity_as_real_rejected() {
        let val: [u16; 9] = [
            b'I' as u16,
            b'n' as u16,
            b'f' as u16,
            b'i' as u16,
            b'n' as u16,
            b'i' as u16,
            b't' as u16,
            b'y' as u16,
            0,
        ];
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_wchar_neg_infinity_as_real_rejected() {
        let val: [u16; 10] = [
            b'-' as u16,
            b'I' as u16,
            b'n' as u16,
            b'f' as u16,
            b'i' as u16,
            b'n' as u16,
            b'i' as u16,
            b't' as u16,
            b'y' as u16,
            0,
        ];
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_wchar_nan_as_real_rejected() {
        let val: [u16; 4] = [b'N' as u16, b'a' as u16, b'N' as u16, 0];
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::InvalidNumericLiteral { .. })
        ));
    }

    // Overflow vs. explicit-non-finite-token discrimination
    //
    // Rust's `f64::from_str` overflows well-formed numeric literals whose
    // magnitude exceeds `f64::MAX` (e.g. "1e309") silently to +/-inf. Those
    // literals are valid ODBC numeric-literals; only the magnitude is out of
    // range, so the spec-aligned SQLSTATE is 22003 (NumericMagnitudeOverflow),
    // not 22018 (InvalidNumericLiteral, reserved for tokens that aren't in
    // the ODBC numeric-literal grammar at all). The next four tests pin both
    // halves of that contract.

    #[test]
    fn convert_char_overflow_as_real_overflows() {
        let val = b"1e309\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::NumericMagnitudeOverflow { .. })
        ));
    }

    #[test]
    fn convert_char_neg_overflow_as_real_overflows() {
        let val = b"-1e309\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::NumericMagnitudeOverflow { .. })
        ));
    }

    #[test]
    fn convert_wchar_overflow_as_real_overflows() {
        // UTF-16 of "1e309"
        let val: [u16; 6] = [
            b'1' as u16,
            b'e' as u16,
            b'3' as u16,
            b'0' as u16,
            b'9' as u16,
            0,
        ];
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::NumericMagnitudeOverflow { .. })
        ));
    }

    #[test]
    fn convert_char_lowercase_inf_as_real_rejected() {
        // Lock in case-insensitive token detection: a future "let's only
        // match the canonical \"Infinity\" spelling" regression must fail
        // this test, since "inf" is also accepted by Rust's parser.
        let val = b"inf\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::InvalidNumericLiteral { .. })
        ));
    }

    // -- Structured C types → VARCHAR -----------------------------------------
    //
    // These tests live here (not in varchar.rs) because they validate the full
    // C-to-SQL pipeline: make_converter → ParamConverter::convert → ReadODBC +
    // WriteJson. This mirrors all other conversion tests above (integer, float,
    // char, bit, binary) which also exercise the end-to-end binding path.

    #[test]
    fn convert_timestamp_as_varchar() -> TestResult {
        let ts = sql::Timestamp {
            year: 2024,
            month: 1,
            day: 15,
            hour: 10,
            minute: 30,
            second: 45,
            fraction: 0,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::VARCHAR,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, Value::String("2024-01-15 10:30:45".to_string()));
        Ok(())
    }

    #[test]
    fn convert_timestamp_with_fraction_as_varchar() -> TestResult {
        let ts = sql::Timestamp {
            year: 1,
            month: 1,
            day: 1,
            hour: 1,
            minute: 1,
            second: 1,
            fraction: 1,
        };
        let binding = make_binding(
            CDataType::TimeStamp,
            sql::SqlDataType::VARCHAR,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(
            v,
            Value::String("0001-01-01 01:01:01.000000001".to_string())
        );
        Ok(())
    }

    #[test]
    fn convert_date_as_varchar() -> TestResult {
        let d = sql::Date {
            year: 2024,
            month: 12,
            day: 25,
        };
        let binding = make_binding(
            CDataType::TypeDate,
            sql::SqlDataType::VARCHAR,
            &d as *const sql::Date as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, Value::String("2024-12-25".to_string()));
        Ok(())
    }

    #[test]
    fn convert_time_as_varchar() -> TestResult {
        let t = sql::Time {
            hour: 14,
            minute: 30,
            second: 59,
        };
        let binding = make_binding(
            CDataType::TypeTime,
            sql::SqlDataType::VARCHAR,
            &t as *const sql::Time as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, Value::String("14:30:59".to_string()));
        Ok(())
    }

    #[test]
    fn convert_numeric_as_varchar() -> TestResult {
        let n = sql::Numeric {
            precision: 10,
            scale: 0,
            sign: 1,
            val: 42u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::VARCHAR,
            &n as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, Value::String("42".to_string()));
        Ok(())
    }

    #[test]
    fn convert_negative_numeric_as_varchar() -> TestResult {
        let n = sql::Numeric {
            precision: 10,
            scale: 2,
            sign: 0,
            val: 12345u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::VARCHAR,
            &n as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, Value::String("-123.45".to_string()));
        Ok(())
    }

    #[test]
    fn convert_numeric_small_scale_as_varchar() -> TestResult {
        let n = sql::Numeric {
            precision: 5,
            scale: 3,
            sign: 1,
            val: 5u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::VARCHAR,
            &n as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, Value::String("0.005".to_string()));
        Ok(())
    }

    #[test]
    fn convert_numeric_negative_scale_as_varchar() -> TestResult {
        let n = sql::Numeric {
            precision: 10,
            scale: -2,
            sign: 1,
            val: 123u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::VARCHAR,
            &n as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, Value::String("12300".to_string()));
        Ok(())
    }

    #[test]
    fn convert_numeric_negative_scale_negative_value_as_varchar() -> TestResult {
        let n = sql::Numeric {
            precision: 10,
            scale: -3,
            sign: 0,
            val: 5u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::VARCHAR,
            &n as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, Value::String("-5000".to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_as_varchar() -> TestResult {
        let val: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
        let mut ind: sql::Len = 4;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::VARCHAR,
            val.as_ptr() as sql::Pointer,
            4,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, Value::String("deadbeef".to_string()));
        Ok(())
    }

    // -- format_numeric_value tests -------------------------------------------

    #[test]
    fn format_numeric_value_no_scale() {
        assert_eq!(format_numeric_value(42, 0), "42");
        assert_eq!(format_numeric_value(-42, 0), "-42");
        assert_eq!(format_numeric_value(0, 0), "0");
    }

    #[test]
    fn format_numeric_value_positive_scale() {
        assert_eq!(format_numeric_value(12345, 2), "123.45");
        assert_eq!(format_numeric_value(-12345, 2), "-123.45");
        assert_eq!(format_numeric_value(5, 3), "0.005");
    }

    #[test]
    fn format_numeric_value_negative_scale() {
        assert_eq!(format_numeric_value(42, -2), "4200");
        assert_eq!(format_numeric_value(-5, -3), "-5000");
    }

    #[test]
    fn format_numeric_value_negative_scale_large_value() {
        let large = i128::MAX / 2;
        let result = format_numeric_value(large, -1);
        assert_eq!(result, format!("{}0", large));
    }

    #[test]
    fn format_numeric_value_negative_scale_i8_min() {
        let result = format_numeric_value(1, i8::MIN);
        assert!(result.starts_with('1'));
        assert_eq!(result.len(), 129); // "1" + 128 zeros
    }

    // -- read_numeric_struct tests --------------------------------------------

    #[test]
    fn read_numeric_struct_positive() {
        let ns = sql::Numeric {
            precision: 10,
            scale: 0,
            sign: 1,
            val: 42u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (value, scale) = read_numeric_struct(&binding).unwrap();
        assert_eq!(value, 42);
        assert_eq!(scale, 0);
    }

    #[test]
    fn read_numeric_struct_negative() {
        let ns = sql::Numeric {
            precision: 10,
            scale: 0,
            sign: 0,
            val: 99u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (value, scale) = read_numeric_struct(&binding).unwrap();
        assert_eq!(value, -99);
        assert_eq!(scale, 0);
    }

    #[test]
    fn read_numeric_struct_with_scale() {
        let ns = sql::Numeric {
            precision: 10,
            scale: 3,
            val: 12345u128.to_le_bytes(),
            sign: 1,
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::DECIMAL,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (value, scale) = read_numeric_struct(&binding).unwrap();
        assert_eq!(value, 12345);
        assert_eq!(scale, 3);
    }

    #[test]
    fn read_numeric_struct_overflow_positive() {
        let ns = sql::Numeric {
            precision: 38,
            scale: 0,
            sign: 1,
            val: u128::MAX.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(read_numeric_struct(&binding).is_err());
    }

    #[test]
    fn read_numeric_struct_negative_min() {
        let magnitude = (i128::MAX as u128) + 1;
        let ns = sql::Numeric {
            precision: 38,
            scale: 0,
            sign: 0,
            val: magnitude.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (value, _) = read_numeric_struct(&binding).unwrap();
        assert_eq!(value, i128::MIN);
    }

    // -- cross-type numeric conversion tests ----------------------------------

    #[test]
    fn convert_float_as_integer() -> TestResult {
        let val: f32 = 42.0;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::INTEGER,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("42".to_string()));
        Ok(())
    }

    #[test]
    fn convert_float_nan_as_integer_fails() {
        let val: f32 = f32::NAN;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::INTEGER,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::NumericMagnitudeOverflow { .. })
        ));
    }

    #[test]
    fn convert_double_infinity_as_integer_fails() {
        let val: f64 = f64::INFINITY;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::INTEGER,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::NumericMagnitudeOverflow { .. })
        ));
    }

    #[test]
    fn convert_double_neg_infinity_as_integer_fails() {
        let val: f64 = f64::NEG_INFINITY;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::INTEGER,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::NumericMagnitudeOverflow { .. })
        ));
    }

    #[test]
    fn convert_double_as_integer() -> TestResult {
        let val: f64 = -123.0;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::INTEGER,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("-123".to_string()));
        Ok(())
    }

    #[test]
    fn convert_double_truncates_fraction_for_integer() -> TestResult {
        let val: f64 = 42.99;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::INTEGER,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("42".to_string()));
        Ok(())
    }

    #[test]
    fn convert_bit_as_integer() -> TestResult {
        let val: u8 = 1;
        let binding = make_binding(
            CDataType::Bit,
            sql::SqlDataType::INTEGER,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("1".to_string()));
        Ok(())
    }

    #[test]
    fn convert_numeric_as_integer() -> TestResult {
        let ns = sql::Numeric {
            precision: 10,
            scale: 0,
            sign: 1,
            val: 999u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("999".to_string()));
        Ok(())
    }

    #[test]
    fn convert_numeric_with_scale_as_integer() -> TestResult {
        let ns = sql::Numeric {
            precision: 10,
            scale: 2,
            sign: 1,
            val: 4299u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("42".to_string()));
        Ok(())
    }

    #[test]
    fn convert_numeric_extreme_negative_scale_as_integer_fails() {
        let ns = sql::Numeric {
            precision: 38,
            scale: i8::MIN,
            sign: 1,
            val: 1u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(convert_binding(&binding).is_err());
    }

    #[test]
    fn convert_numeric_large_positive_scale_as_integer_fails() {
        let ns = sql::Numeric {
            precision: 38,
            scale: 100,
            sign: 1,
            val: 42u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(convert_binding(&binding).is_err());
    }

    #[test]
    fn convert_default_as_real() -> TestResult {
        let val: f64 = 4.25;
        let binding = make_binding(
            CDataType::Default,
            sql::SqlDataType::DOUBLE,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, Value::String("4.25".to_string()));
        Ok(())
    }

    #[test]
    fn convert_slong_as_real() -> TestResult {
        let val: i32 = 42;
        let binding = make_binding(
            CDataType::SLong,
            sql::SqlDataType::DOUBLE,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, Value::String("42".to_string()));
        Ok(())
    }

    #[test]
    fn convert_sbigint_as_real() -> TestResult {
        let val: i64 = 1_000_000;
        let binding = make_binding(
            CDataType::SBigInt,
            sql::SqlDataType::DOUBLE,
            &val as *const i64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, Value::String("1000000".to_string()));
        Ok(())
    }

    #[test]
    fn convert_bit_as_real() -> TestResult {
        let val: u8 = 1;
        let binding = make_binding(
            CDataType::Bit,
            sql::SqlDataType::DOUBLE,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, Value::String("1".to_string()));
        Ok(())
    }

    #[test]
    fn convert_numeric_as_real() -> TestResult {
        let ns = sql::Numeric {
            precision: 10,
            scale: 2,
            sign: 1,
            val: 314u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::DOUBLE,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, Value::String("3.14".to_string()));
        Ok(())
    }

    #[test]
    fn convert_default_as_boolean_true() -> TestResult {
        let val: u8 = 1;
        let binding = make_binding(
            CDataType::Default,
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
    fn convert_default_as_boolean_false() -> TestResult {
        let val: u8 = 0;
        let binding = make_binding(
            CDataType::Default,
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
    fn convert_slong_as_boolean() -> TestResult {
        let val: i32 = 1;
        let binding = make_binding(
            CDataType::SLong,
            sql::SqlDataType::EXT_BIT,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_slong_zero_as_boolean() -> TestResult {
        let val: i32 = 0;
        let binding = make_binding(
            CDataType::SLong,
            sql::SqlDataType::EXT_BIT,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_float_as_boolean() -> TestResult {
        let val: f32 = 1.0;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_float_nan_as_boolean_fails() {
        let val: f32 = f32::NAN;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::InvalidBooleanValue { .. })
        ));
    }

    #[test]
    fn convert_double_infinity_as_boolean_fails() {
        let val: f64 = f64::INFINITY;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::EXT_BIT,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::InvalidBooleanValue { .. })
        ));
    }

    #[test]
    fn convert_numeric_as_boolean() -> TestResult {
        let ns = sql::Numeric {
            precision: 10,
            scale: 0,
            sign: 1,
            val: 1u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::EXT_BIT,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_bit_as_decimal() -> TestResult {
        let val: u8 = 1;
        let binding = make_binding(
            CDataType::Bit,
            sql::SqlDataType::DECIMAL,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("1".to_string()));
        Ok(())
    }

    #[test]
    fn convert_numeric_as_decimal() -> TestResult {
        let ns = sql::Numeric {
            precision: 10,
            scale: 3,
            sign: 1,
            val: 12345678u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::DECIMAL,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("12345.678".to_string()));
        Ok(())
    }

    #[test]
    fn convert_bit_zero_as_varchar() -> TestResult {
        let val: u8 = 0;
        let binding = make_binding(
            CDataType::Bit,
            sql::SqlDataType::VARCHAR,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, Value::String("0".to_string()));
        Ok(())
    }

    #[test]
    fn convert_char_as_boolean_true() -> TestResult {
        let val = b"1\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("true".to_string()));
        Ok(())
    }

    #[test]
    fn convert_char_as_boolean_false() -> TestResult {
        let val = b"0\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, Value::String("false".to_string()));
        Ok(())
    }

    #[test]
    fn convert_char_nan_as_boolean_fails() {
        let val = b"NaN\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::InvalidBooleanValue { .. })
        ));
    }

    #[test]
    fn convert_char_infinity_as_boolean_fails() {
        let val = b"inf\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::InvalidBooleanValue { .. })
        ));
    }

    // =========================================================================
    // SQL_C_BINARY → SQL_INTEGER / SQL_BIGINT / SQL_SMALLINT / SQL_TINYINT
    // =========================================================================

    #[test]
    fn convert_binary_4bytes_to_integer() -> TestResult {
        let val: i32 = 42;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::INTEGER,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("42".to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_8bytes_to_bigint() -> TestResult {
        let val: i64 = 9_999_999_999;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_BIG_INT,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("9999999999".to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_2bytes_to_smallint() -> TestResult {
        let val: i16 = -7;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::SMALLINT,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("-7".to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_1byte_to_tinyint() -> TestResult {
        let val: i8 = 127;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_TINY_INT,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("127".to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_wrong_size_to_integer_fails() {
        let bytes: [u8; 3] = [1, 2, 3];
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::INTEGER,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::BindingNumericOutOfRange { .. })
        ));
    }

    #[test]
    fn convert_binary_4bytes_to_bigint_fails() {
        let val: i32 = 42;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_BIG_INT,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::BindingNumericOutOfRange { .. })
        ));
    }

    #[test]
    fn convert_binary_8bytes_to_real_fails() {
        let val: f64 = 3.125;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::REAL,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::BindingNumericOutOfRange { .. })
        ));
    }

    // =========================================================================
    // SQL_C_BINARY → SQL_FLOAT / SQL_DOUBLE / SQL_REAL
    // =========================================================================

    #[test]
    fn convert_binary_8bytes_to_double() -> TestResult {
        let val: f64 = 3.125;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::DOUBLE,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, Value::String("3.125".to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_4bytes_to_real() -> TestResult {
        let val: f32 = 2.5;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::REAL,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, Value::String("2.5".to_string()));
        Ok(())
    }

    // Per MS ODBC "C to SQL: Binary" spec, SQL_C_BINARY -> SQL_REAL/DOUBLE is
    // specified to do a length-equals check only and then pass the bytes
    // through. NaN and +/-Infinity are valid IEEE-754 values and Snowflake
    // FLOAT columns accept them, so the driver forwards them to the server
    // rather than rejecting client-side. These tests pin that behavior.

    #[test]
    fn convert_binary_nan_to_double_forwards_to_server() -> TestResult {
        let val: f64 = f64::NAN;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::DOUBLE,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        // Rust's Display for f64::NAN is the literal "NaN" — the server-side
        // JSON binding parser accepts the same literal for FLOAT targets.
        assert_eq!(v, Value::String("NaN".to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_infinity_to_real_forwards_to_server() -> TestResult {
        let val: f32 = f32::INFINITY;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::REAL,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        // The driver's `SnowflakeReal::write_json` maps non-finite floats to
        // the literals Snowflake's JSON bind parser accepts: "Infinity" /
        // "-Infinity" / "NaN". Rust's `Display` for f32::INFINITY is the
        // short form "inf", which the server rejects.
        assert_eq!(v, Value::String("Infinity".to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_negative_infinity_to_real_forwards_to_server() -> TestResult {
        let val: f32 = f32::NEG_INFINITY;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::REAL,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        // Same rationale as the +Infinity case: `SnowflakeReal::write_json`
        // emits the full "-Infinity" literal Snowflake's JSON bind parser
        // accepts, not Rust's short "-inf" form.
        assert_eq!(v, Value::String("-Infinity".to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_wrong_size_to_double_fails() {
        let bytes: [u8; 3] = [1, 2, 3];
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::DOUBLE,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::BindingNumericOutOfRange { .. })
        ));
    }

    // =========================================================================
    // SQL_C_BINARY → SQL_DECIMAL / SQL_NUMERIC (via DecimalParamConverter)
    // =========================================================================

    #[test]
    fn convert_binary_numeric_struct_to_decimal() -> TestResult {
        let numeric = sql::Numeric {
            precision: 10,
            scale: 2,
            sign: 1,
            val: {
                let mut v = [0u8; 16];
                let bytes = 12345u128.to_le_bytes();
                v.copy_from_slice(&bytes);
                v
            },
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &numeric as *const _ as *const u8,
                mem::size_of::<sql::Numeric>(),
            )
        };
        let mut ind: sql::Len = mem::size_of::<sql::Numeric>() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::DECIMAL,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("123.45".to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_wrong_size_to_decimal_fails() {
        let bytes: [u8; 10] = [0; 10];
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::DECIMAL,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::BindingNumericOutOfRange { .. })
        ));
    }

    // =========================================================================
    // SQL_C_BINARY → SQL_DATE
    // =========================================================================

    #[test]
    fn convert_binary_to_date() -> TestResult {
        let date = sql::Date {
            year: 2025,
            month: 3,
            day: 26,
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(&date as *const _ as *const u8, mem::size_of::<sql::Date>())
        };
        let mut ind: sql::Len = mem::size_of::<sql::Date>() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::DATE,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Date);
        let expected_millis = (chrono::NaiveDate::from_ymd_opt(2025, 3, 26).unwrap()
            - chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
        .num_days()
            * 86_400_000;
        assert_eq!(v, Value::String(expected_millis.to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_wrong_size_to_date_fails() {
        let bytes: [u8; 4] = [0; 4];
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::DATE,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::BindingNumericOutOfRange { .. })
        ));
    }

    // =========================================================================
    // SQL_C_BINARY → SQL_TIME
    // =========================================================================

    #[test]
    fn convert_binary_to_time() -> TestResult {
        let time = sql::Time {
            hour: 14,
            minute: 30,
            second: 45,
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(&time as *const _ as *const u8, mem::size_of::<sql::Time>())
        };
        let mut ind: sql::Len = mem::size_of::<sql::Time>() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::TIME,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Time);
        let nanos = 14 * 3600 * 1_000_000_000i64 + 30 * 60 * 1_000_000_000 + 45 * 1_000_000_000;
        assert_eq!(v, Value::String(nanos.to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_wrong_size_to_time_fails() {
        let bytes: [u8; 4] = [0; 4];
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::TIME,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::BindingNumericOutOfRange { .. })
        ));
    }

    // =========================================================================
    // SQL_C_BINARY → SQL_TIMESTAMP
    // =========================================================================

    #[test]
    fn convert_binary_to_timestamp() -> TestResult {
        let ts = sql::Timestamp {
            year: 2025,
            month: 3,
            day: 26,
            hour: 14,
            minute: 30,
            second: 45,
            fraction: 0,
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &ts as *const _ as *const u8,
                mem::size_of::<sql::Timestamp>(),
            )
        };
        let mut ind: sql::Len = mem::size_of::<sql::Timestamp>() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::TIMESTAMP,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::TimestampNtz);
        let expected_nanos = chrono::NaiveDate::from_ymd_opt(2025, 3, 26)
            .unwrap()
            .and_hms_opt(14, 30, 45)
            .unwrap()
            .and_utc()
            .timestamp_nanos_opt()
            .unwrap();
        assert_eq!(v, Value::String(expected_nanos.to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_wrong_size_to_timestamp_fails() {
        let bytes: [u8; 8] = [0; 8];
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::TIMESTAMP,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(JsonBindingError::BindingNumericOutOfRange { .. })
        ));
    }

    #[test]
    fn convert_binary_negative_i32_to_integer() -> TestResult {
        let val: i32 = -100;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::INTEGER,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, Value::String("-100".to_string()));
        Ok(())
    }

    #[test]
    fn convert_binary_timestamp_with_fraction() -> TestResult {
        let ts = sql::Timestamp {
            year: 2025,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            fraction: 500_000_000,
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &ts as *const _ as *const u8,
                mem::size_of::<sql::Timestamp>(),
            )
        };
        let mut ind: sql::Len = mem::size_of::<sql::Timestamp>() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::TIMESTAMP,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::TimestampNtz);
        let expected_nanos = chrono::NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_nano_opt(0, 0, 0, 500_000_000)
            .unwrap()
            .and_utc()
            .timestamp_nanos_opt()
            .unwrap();
        assert_eq!(v, Value::String(expected_nanos.to_string()));
        Ok(())
    }
}
