use arrow::array::{Array, GenericByteArray};
use arrow::datatypes::Utf8Type;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use odbc_sys as sql;

use crate::cdata_types::CDataType;
use crate::conversion::error::{
    InvalidValueSnafu, NumericParsingSnafu, NumericValueOutOfRangeSnafu, ReadArrowError,
    UnsupportedOdbcTypeSnafu, WriteOdbcError,
};
use crate::conversion::traits::Binding;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

pub(crate) struct SnowflakeVarchar {
    #[allow(dead_code)]
    pub len: u32,
}

impl SnowflakeType for SnowflakeVarchar {
    type Representation<'a> = &'a str;
}

impl ReadArrowType<GenericByteArray<Utf8Type>> for SnowflakeVarchar {
    fn read_arrow_type<'a>(
        &self,
        array: &'a GenericByteArray<Utf8Type>,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        if array.is_null(row_idx) {
            return Err(ReadArrowError::NullValue {
                location: snafu::location!(),
            });
        }
        let v = array.value(row_idx);
        Ok(v)
    }
}

macro_rules! parse_i_number {
    ($value:expr, $type:ty) => {
        $value.trim().parse::<$type>().map_err(|e| match e.kind() {
            std::num::IntErrorKind::NegOverflow | std::num::IntErrorKind::PosOverflow => {
                NumericValueOutOfRangeSnafu {
                    reason: format!("Value out of range for type {}", stringify!($type)),
                }
                .build()
            }
            _ => NumericParsingSnafu {
                reason: e.to_string(),
            }
            .build(),
        })
    };
}

macro_rules! parse_u_number {
    ($value:expr, $type:ty) => {{
        let value = $value.trim();
        if value.starts_with('-') {
            NumericValueOutOfRangeSnafu {
                reason: "Value is negative".to_string(),
            }
            .fail()
        } else {
            match value.parse::<$type>() {
                Ok(value) => Ok(value),
                Err(e) => match e.kind() {
                    std::num::IntErrorKind::PosOverflow => NumericValueOutOfRangeSnafu {
                        reason: format!("Value out of range for type {}", stringify!($type)),
                    }
                    .fail(),
                    _ => NumericParsingSnafu {
                        reason: e.to_string(),
                    }
                    .fail(),
                },
            }
        }
    }};
}

macro_rules! write_i_number {
    ($value:expr, $type:ty, $binding:expr) => {{
        let value = parse_i_number!($value, $type)?;
        unsafe {
            std::ptr::write($binding.target_value_ptr as *mut $type, value);
        }
        if !$binding.str_len_or_ind_ptr.is_null() {
            unsafe {
                std::ptr::write(
                    $binding.str_len_or_ind_ptr,
                    std::mem::size_of::<$type>() as sql::Len,
                )
            };
        }
        Ok(())
    }};
}

macro_rules! write_u_number {
    ($value:expr, $type:ty, $binding:expr) => {{
        let value = parse_u_number!($value, $type)?;
        unsafe {
            std::ptr::write($binding.target_value_ptr as *mut $type, value);
        }
        if !$binding.str_len_or_ind_ptr.is_null() {
            unsafe {
                std::ptr::write(
                    $binding.str_len_or_ind_ptr,
                    std::mem::size_of::<$type>() as sql::Len,
                )
            };
        }
        Ok(())
    }};
}

macro_rules! write_float {
    ($value:expr, $type:ty, $binding:expr) => {{
        let value = $value.trim().parse::<$type>().map_err(|e| {
            NumericParsingSnafu {
                reason: e.to_string(),
            }
            .build()
        })?;
        unsafe {
            std::ptr::write($binding.target_value_ptr as *mut $type, value);
        };
        if !$binding.str_len_or_ind_ptr.is_null() {
            unsafe {
                std::ptr::write(
                    $binding.str_len_or_ind_ptr,
                    std::mem::size_of::<$type>() as sql::Len,
                )
            };
        }
        Ok(())
    }};
}

/// Validates that a date string is in strict YYYY-MM-DD format.
/// Returns false for formats like "24-01-15" (2-digit year) or "2024-1-5" (single-digit month/day).
fn is_valid_date_format(s: &str) -> bool {
    // Must be exactly 10 characters: YYYY-MM-DD
    if s.len() != 10 {
        return false;
    }
    let bytes = s.as_bytes();
    // Check format: DDDD-DD-DD where D is digit
    bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4] == b'-'
        && bytes[5].is_ascii_digit()
        && bytes[6].is_ascii_digit()
        && bytes[7] == b'-'
        && bytes[8].is_ascii_digit()
        && bytes[9].is_ascii_digit()
}

/// Validates that a time string is in strict HH:MM:SS format.
/// Returns false for formats like "9:5:3" (single-digit components).
fn is_valid_time_format(s: &str) -> bool {
    // Must be exactly 8 characters: HH:MM:SS
    if s.len() != 8 {
        return false;
    }
    let bytes = s.as_bytes();
    // Check format: DD:DD:DD where D is digit
    bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b':'
        && bytes[3].is_ascii_digit()
        && bytes[4].is_ascii_digit()
        && bytes[5] == b':'
        && bytes[6].is_ascii_digit()
        && bytes[7].is_ascii_digit()
}

/// Writes ASCII characters from `src` to `dst`, replacing non-ASCII characters with a single 0x1a byte.
/// Multi-byte UTF-8 characters are replaced with a single 0x1a byte.
/// Returns the number of bytes written.
fn write_ascii_replacing_non_ascii(src: &str, dst: *mut u8, len: usize) -> usize {
    let mut dst_idx = 0;
    for c in src.chars() {
        if dst_idx >= len {
            break;
        }
        if c.is_ascii() {
            unsafe {
                std::ptr::write(dst.add(dst_idx), c as u8);
            }
        } else {
            unsafe {
                std::ptr::write(dst.add(dst_idx), 0x1a);
            }
        }
        dst_idx += 1;
    }
    dst_idx
}

impl WriteODBCType for SnowflakeVarchar {
    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
    ) -> Result<(), WriteOdbcError> {
        match binding.target_type {
            CDataType::Char => {
                let written = write_ascii_replacing_non_ascii(
                    snowflake_value,
                    binding.target_value_ptr as *mut u8,
                    binding.buffer_length as usize,
                );
                let len = std::cmp::min(written, binding.buffer_length as usize);
                if !binding.str_len_or_ind_ptr.is_null() {
                    unsafe { std::ptr::write(binding.str_len_or_ind_ptr, len as sql::Len) };
                }
                Ok(())
            }
            CDataType::WChar => {
                let utf16_value = snowflake_value.encode_utf16().collect::<Vec<u16>>();
                let len = std::cmp::min(binding.buffer_length as usize, utf16_value.len() * 2);
                if !binding.str_len_or_ind_ptr.is_null() {
                    unsafe {
                        std::ptr::write(binding.str_len_or_ind_ptr, len as sql::Len);
                    };
                }
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        utf16_value.as_ptr() as *const u8,
                        binding.target_value_ptr as *mut u8,
                        len,
                    );
                }
                Ok(())
            }
            CDataType::SBigInt => write_i_number!(snowflake_value, i64, binding),
            CDataType::UBigInt => write_u_number!(snowflake_value, u64, binding),
            CDataType::Long | CDataType::SLong => write_i_number!(snowflake_value, i32, binding),
            CDataType::ULong => write_u_number!(snowflake_value, u32, binding),
            CDataType::Short | CDataType::SShort => write_i_number!(snowflake_value, i16, binding),
            CDataType::UShort => write_u_number!(snowflake_value, u16, binding),
            CDataType::TinyInt | CDataType::STinyInt => {
                write_i_number!(snowflake_value, i8, binding)
            }
            CDataType::UTinyInt => write_u_number!(snowflake_value, u8, binding),
            CDataType::Double => write_float!(snowflake_value, f64, binding),
            CDataType::Float => write_float!(snowflake_value, f32, binding),
            CDataType::Bit => {
                let value = parse_u_number!(snowflake_value, u8)?;
                match value {
                    0 | 1 => {
                        unsafe { std::ptr::write(binding.target_value_ptr as *mut u8, value) };
                        if !binding.str_len_or_ind_ptr.is_null() {
                            unsafe {
                                std::ptr::write(
                                    binding.str_len_or_ind_ptr,
                                    std::mem::size_of::<u8>() as sql::Len,
                                )
                            };
                        }
                        Ok(())
                    }
                    _ => NumericValueOutOfRangeSnafu {
                        reason: "Trying to convert non-binary string to BIT".to_string(),
                    }
                    .fail(),
                }
            }
            CDataType::Date | CDataType::TypeDate => {
                // Strict format validation: YYYY-MM-DD (exactly 10 chars)
                let value = snowflake_value.trim();
                if !is_valid_date_format(value) {
                    return InvalidValueSnafu {
                        reason: "Date must be in YYYY-MM-DD format".to_string(),
                    }
                    .fail();
                }
                let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|e| {
                    InvalidValueSnafu {
                        reason: e.to_string(),
                    }
                    .build()
                })?;
                let date = sql::Date {
                    year: Datelike::year(&date) as i16,
                    month: Datelike::month(&date) as u16,
                    day: Datelike::day(&date) as u16,
                };
                unsafe { std::ptr::write(binding.target_value_ptr as *mut sql::Date, date) };
                Ok(())
            }
            CDataType::Time | CDataType::TypeTime => {
                // Strict format validation: HH:MM:SS (exactly 8 chars)
                let value = snowflake_value.trim();
                if !is_valid_time_format(value) {
                    return InvalidValueSnafu {
                        reason: "Time must be in HH:MM:SS format".to_string(),
                    }
                    .fail();
                }
                let time = NaiveTime::parse_from_str(value, "%H:%M:%S").map_err(|e| {
                    InvalidValueSnafu {
                        reason: e.to_string(),
                    }
                    .build()
                })?;
                let time = sql::Time {
                    hour: Timelike::hour(&time) as u16,
                    minute: Timelike::minute(&time) as u16,
                    second: Timelike::second(&time) as u16,
                };
                unsafe { std::ptr::write(binding.target_value_ptr as *mut sql::Time, time) };
                Ok(())
            }
            CDataType::TimeStamp | CDataType::TypeTimestamp => {
                // Try parsing as full timestamp first, then as date-only with midnight default,
                // then as time-only with today's date
                let value = snowflake_value.trim();
                let timestamp = if let Ok(ts) =
                    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                {
                    ts
                } else if is_valid_date_format(value) {
                    // Date-only string: default time to midnight
                    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|e| {
                        InvalidValueSnafu {
                            reason: e.to_string(),
                        }
                        .build()
                    })?;
                    date.and_hms_opt(0, 0, 0).ok_or_else(|| {
                        InvalidValueSnafu {
                            reason: "Failed to create midnight timestamp".to_string(),
                        }
                        .build()
                    })?
                } else if is_valid_time_format(value) {
                    // Time-only string: default date to today
                    let time = NaiveTime::parse_from_str(value, "%H:%M:%S").map_err(|e| {
                        InvalidValueSnafu {
                            reason: e.to_string(),
                        }
                        .build()
                    })?;
                    let today = chrono::Local::now().date_naive();
                    today.and_time(time)
                } else {
                    return InvalidValueSnafu {
                        reason: "Timestamp must be in YYYY-MM-DD HH:MM:SS, YYYY-MM-DD, or HH:MM:SS format".to_string(),
                    }.fail();
                };
                let timestamp = sql::Timestamp {
                    year: Datelike::year(&timestamp) as i16,
                    month: Datelike::month(&timestamp) as u16,
                    day: Datelike::day(&timestamp) as u16,
                    hour: Timelike::hour(&timestamp) as u16,
                    minute: Timelike::minute(&timestamp) as u16,
                    second: Timelike::second(&timestamp) as u16,
                    fraction: 0,
                };
                unsafe {
                    std::ptr::write(binding.target_value_ptr as *mut sql::Timestamp, timestamp)
                };
                Ok(())
            }
            CDataType::Numeric => {
                // Parse string as a numeric value and convert to SQL_NUMERIC_STRUCT
                let value = snowflake_value.trim();
                let (is_negative, digits_str) = if let Some(stripped) = value.strip_prefix('-') {
                    (true, stripped)
                } else if let Some(stripped) = value.strip_prefix('+') {
                    (false, stripped)
                } else {
                    (false, value)
                };

                // Split into integer and fractional parts
                let (int_part, frac_part) = if let Some(pos) = digits_str.find('.') {
                    (&digits_str[..pos], &digits_str[pos + 1..])
                } else {
                    (digits_str, "")
                };

                // Validate that parts contain only digits
                if !int_part.chars().all(|c| c.is_ascii_digit())
                    || !frac_part.chars().all(|c| c.is_ascii_digit())
                {
                    return InvalidValueSnafu {
                        reason: "Invalid numeric format".to_string(),
                    }
                    .fail();
                }

                // Combine digits (remove decimal point)
                let combined = format!("{}{}", int_part, frac_part);
                let scale = frac_part.len() as i8;

                // Parse as u128 for the val field
                let val: u128 = combined.parse().map_err(|e: std::num::ParseIntError| {
                    InvalidValueSnafu {
                        reason: e.to_string(),
                    }
                    .build()
                })?;

                // Convert to little-endian byte array
                let val_bytes = val.to_le_bytes();

                let numeric = sql::Numeric {
                    precision: (int_part.len() + frac_part.len()) as u8,
                    scale,
                    sign: if is_negative { 0 } else { 1 },
                    val: val_bytes,
                };
                unsafe { std::ptr::write(binding.target_value_ptr as *mut sql::Numeric, numeric) };
                if !binding.str_len_or_ind_ptr.is_null() {
                    unsafe {
                        std::ptr::write(
                            binding.str_len_or_ind_ptr,
                            std::mem::size_of::<sql::Numeric>() as sql::Len,
                        )
                    };
                }
                Ok(())
            }
            CDataType::Binary => {
                // Copy the string bytes directly to the buffer
                let bytes = snowflake_value.as_bytes();
                let copy_len = std::cmp::min(bytes.len(), binding.buffer_length as usize);
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        binding.target_value_ptr as *mut u8,
                        copy_len,
                    );
                }
                if !binding.str_len_or_ind_ptr.is_null() {
                    unsafe { std::ptr::write(binding.str_len_or_ind_ptr, bytes.len() as sql::Len) };
                }
                Ok(())
            }
            _ => UnsupportedOdbcTypeSnafu {
                target_type: binding.target_type,
            }
            .fail(),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_write_odbc_type_surrogate_pair() {
        // Test with musical symbol treble clef (𝄞) which requires surrogate pairs in UTF-16
        let varchar = SnowflakeVarchar { len: 100 };
        let snowflake_value = "𝄞";

        // Test SQL_C_CHAR conversion - should replace with 0x1a
        let mut char_buffer = [0u8; 10];
        let mut char_indicator: sql::Len = 0;
        let char_binding = Binding {
            target_type: CDataType::Char,
            target_value_ptr: char_buffer.as_mut_ptr() as *mut std::ffi::c_void,
            buffer_length: char_buffer.len() as isize,
            str_len_or_ind_ptr: &mut char_indicator as *mut sql::Len,
        };

        varchar
            .write_odbc_type(snowflake_value, &char_binding)
            .unwrap();
        assert_eq!(char_indicator, 1); // 1 character written
        assert_eq!(char_buffer[0], 0x1a); // SUB character

        // Test SQL_C_WCHAR conversion - should preserve the character as UTF-16 surrogate pair
        let mut wchar_buffer = [0u16; 10];
        let mut wchar_indicator: sql::Len = 0;
        let wchar_binding = Binding {
            target_type: CDataType::WChar,
            target_value_ptr: wchar_buffer.as_mut_ptr() as *mut std::ffi::c_void,
            buffer_length: (wchar_buffer.len() * 2) as isize,
            str_len_or_ind_ptr: &mut wchar_indicator as *mut sql::Len,
        };

        varchar
            .write_odbc_type(snowflake_value, &wchar_binding)
            .unwrap();
        assert_eq!(wchar_indicator, 4); // 4 bytes (2 UTF-16 code units)

        // Verify the surrogate pair is correctly written
        let utf16_chars_string = String::from_utf16(&wchar_buffer[..2]).unwrap();
        assert_eq!(utf16_chars_string, snowflake_value);
    }

    use super::*;

    #[test]
    fn test_ascii_only() {
        let src = "hello";
        let mut dst = [0u8; 10];
        let len = write_ascii_replacing_non_ascii(src, dst.as_mut_ptr(), dst.len());
        assert_eq!(len, 5);
        assert_eq!(&dst[..len], b"hello");
    }

    #[test]
    fn test_empty_string() {
        let src = "";
        let mut dst = [0u8; 10];
        let len = write_ascii_replacing_non_ascii(src, dst.as_mut_ptr(), dst.len());
        assert_eq!(len, 0);
    }

    #[test]
    fn test_multibyte_utf8_replaced_with_single_byte() {
        // "café" - 'é' is 2 bytes in UTF-8 (0xC3 0xA9)
        let src = "café";
        let mut dst = [0u8; 10];
        let len = write_ascii_replacing_non_ascii(src, dst.as_mut_ptr(), dst.len());
        assert_eq!(len, 4); // 4 characters, not 5 bytes
        assert_eq!(&dst[..len], &[b'c', b'a', b'f', 0x1a]);
    }

    #[test]
    fn test_three_byte_utf8_character() {
        // "a中b" - '中' is 3 bytes in UTF-8
        let src = "a中b";
        let mut dst = [0u8; 10];
        let len = write_ascii_replacing_non_ascii(src, dst.as_mut_ptr(), dst.len());
        assert_eq!(len, 3); // 3 characters
        assert_eq!(&dst[..len], &[b'a', 0x1a, b'b']);
    }

    #[test]
    fn test_four_byte_utf8_character() {
        // "a😀b" - '😀' is 4 bytes in UTF-8
        let src = "a😀b";
        let mut dst = [0u8; 10];
        let len = write_ascii_replacing_non_ascii(src, dst.as_mut_ptr(), dst.len());
        assert_eq!(len, 3); // 3 characters
        assert_eq!(&dst[..len], &[b'a', 0x1a, b'b']);
    }

    #[test]
    fn test_all_non_ascii() {
        // "日本語" - all 3-byte characters
        let src = "日本語";
        let mut dst = [0u8; 10];
        let len = write_ascii_replacing_non_ascii(src, dst.as_mut_ptr(), dst.len());
        assert_eq!(len, 3);
        assert_eq!(&dst[..len], &[0x1a, 0x1a, 0x1a]);
    }

    #[test]
    fn test_buffer_smaller_than_input() {
        let src = "hello world";
        let mut dst = [0u8; 5];
        let len = write_ascii_replacing_non_ascii(src, dst.as_mut_ptr(), dst.len());
        assert_eq!(len, 5);
        assert_eq!(&dst[..len], b"hello");
    }

    #[test]
    fn test_buffer_smaller_than_input_with_multibyte() {
        // "hëllo" - 'ë' is 2 bytes, but buffer can only fit 3 characters
        let src = "hëllo";
        let mut dst = [0u8; 3];
        let len = write_ascii_replacing_non_ascii(src, dst.as_mut_ptr(), dst.len());
        assert_eq!(len, 3);
        assert_eq!(&dst[..len], &[b'h', 0x1a, b'l']);
    }

    #[test]
    fn test_empty_buffer() {
        let src = "hello";
        let mut dst = [0u8; 0];
        let len = write_ascii_replacing_non_ascii(src, dst.as_mut_ptr(), dst.len());
        assert_eq!(len, 0);
    }

    #[test]
    fn test_mixed_ascii_and_non_ascii() {
        // Mix of ASCII, 2-byte, 3-byte, and 4-byte UTF-8
        let src = "a é 中 😀 z";
        let mut dst = [0u8; 20];
        let len = write_ascii_replacing_non_ascii(src, dst.as_mut_ptr(), dst.len());
        // Characters: 'a', ' ', 'é', ' ', '中', ' ', '😀', ' ', 'z' = 9 chars
        assert_eq!(len, 9);
        assert_eq!(
            &dst[..len],
            &[b'a', b' ', 0x1a, b' ', 0x1a, b' ', 0x1a, b' ', b'z']
        );
    }

    #[test]
    fn test_combining_characters() {
        // "y̆es" - 'y' + combining breve '\u{0306}' + 'e' + 's'
        // This tests handling of combining characters where the base character
        // is ASCII but the combining mark is non-ASCII
        let src = "y̆es";
        let mut dst = [0u8; 10];
        let len = write_ascii_replacing_non_ascii(src, dst.as_mut_ptr(), dst.len());
        // Characters: 'y', '\u{0306}' (combining breve), 'e', 's' = 4 chars
        assert_eq!(len, 4);
        assert_eq!(&dst[..len], &[b'y', 0x1a, b'e', b's']);
    }
}
