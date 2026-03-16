use odbc_sys as sql;

use crate::cdata_types::{CDataType, SQL_NO_TOTAL};
use crate::conversion::error::{IndicatorRequiredSnafu, ReadArrowError, WriteOdbcError};
use crate::conversion::warning::{Warning, Warnings};

/// Convert a UTF-8 string to the system's ANSI code page (ACP) bytes.
///
/// Uses `WideCharToMultiByte(CP_ACP, …)` via UTF-8 → UTF-16 → ACP.
/// Characters that cannot be represented in the ACP are replaced with the
/// code page's default substitution character.
#[cfg(windows)]
fn utf8_to_acp_bytes(src: &str) -> Vec<u8> {
    if src.is_empty() {
        return Vec::new();
    }

    unsafe extern "system" {
        fn WideCharToMultiByte(
            code_page: u32,
            dw_flags: u32,
            lp_wide_char_str: *const u16,
            cch_wide_char: i32,
            lp_multi_byte_str: *mut u8,
            cb_multi_byte: i32,
            lp_default_char: *const u8,
            lp_used_default_char: *mut i32,
        ) -> i32;
    }

    const CP_ACP: u32 = 0;

    let wide: Vec<u16> = src.encode_utf16().collect();

    unsafe {
        let byte_len = WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
            std::ptr::null_mut(),
        );
        if byte_len <= 0 {
            return src.as_bytes().to_vec();
        }

        let mut buf = vec![0u8; byte_len as usize];
        let written = WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            buf.as_mut_ptr(),
            byte_len,
            std::ptr::null(),
            std::ptr::null_mut(),
        );
        if written <= 0 {
            return src.as_bytes().to_vec();
        }

        buf.truncate(written as usize);
        buf
    }
}

pub enum LengthOrNull {
    Null,
    Length(sql::Len),
}

#[derive(Debug, Default)]
pub struct Binding {
    pub target_type: CDataType,
    pub target_value_ptr: sql::Pointer,
    pub buffer_length: sql::Len,
    /// Octet-length pointer — receives the byte length of the data after fetch.
    /// Set by `SQLBindCol` (combined StrLen/Ind role) or `SQL_DESC_OCTET_LENGTH_PTR`.
    pub octet_length_ptr: *mut sql::Len,
    /// Indicator (StrLen_or_Ind) pointer.
    /// When `SQLBindCol` is used with a combined StrLen/Ind buffer, this is the same
    /// pointer as `octet_length_ptr`. When separate descriptor fields are used, this
    /// may be distinct from `octet_length_ptr` or null if no indicator is bound.
    pub indicator_ptr: *mut sql::Len,
    /// Numeric precision, set via SQLSetDescField(SQL_DESC_PRECISION) on the ARD.
    /// Used for SQL_C_NUMERIC conversions.
    pub precision: Option<i16>,
    /// Numeric scale, set via SQLSetDescField(SQL_DESC_SCALE) on the ARD.
    /// Used for SQL_C_NUMERIC conversions.
    pub scale: Option<i16>,
    /// Interval leading field precision, set via
    /// SQLSetDescField(SQL_DESC_DATETIME_INTERVAL_PRECISION) on the ARD.
    /// ODBC default is 2 when not explicitly set.
    pub datetime_interval_precision: Option<i16>,
}

impl Binding {
    pub fn write_length_or_null(&self, length_or_null: LengthOrNull) -> Result<(), WriteOdbcError> {
        match length_or_null {
            LengthOrNull::Null => {
                if self.indicator_ptr.is_null() {
                    return IndicatorRequiredSnafu.fail();
                }
                unsafe {
                    std::ptr::write(self.indicator_ptr, crate::cdata_types::SQL_NULL_DATA);
                }
                Ok(())
            }
            LengthOrNull::Length(length) => {
                if !self.octet_length_ptr.is_null() {
                    if !self.indicator_ptr.is_null() {
                        unsafe { std::ptr::write(self.indicator_ptr, 0) };
                    }
                    unsafe { std::ptr::write(self.octet_length_ptr, length) };
                } else if !self.indicator_ptr.is_null() {
                    unsafe { std::ptr::write(self.indicator_ptr, length as sql::Len) };
                }
                Ok(())
            }
        }
    }

    pub fn write_fixed<T>(&self, value: T) {
        unsafe {
            if !self.target_value_ptr.is_null() {
                std::ptr::write(self.target_value_ptr as *mut T, value);
            }
        }
        let _ =
            self.write_length_or_null(LengthOrNull::Length(std::mem::size_of::<T>() as sql::Len));
    }

    pub fn write_char_string(&self, src: &str, get_data_offset: &mut Option<usize>) -> Warnings {
        #[cfg(windows)]
        {
            let acp_bytes = utf8_to_acp_bytes(src);
            self.write_char_bytes(&acp_bytes, get_data_offset)
        }
        #[cfg(not(windows))]
        {
            use crate::api::encoding::{is_ascii_locale, mask_non_ascii_characters};

            if is_ascii_locale() {
                let masked_src = mask_non_ascii_characters(src);
                self.write_char_bytes(masked_src.as_bytes(), get_data_offset)
            } else {
                self.write_char_bytes(src.as_bytes(), get_data_offset)
            }
        }
    }

    fn write_char_bytes(&self, src: &[u8], get_data_offset: &mut Option<usize>) -> Warnings {
        let offset = get_data_offset.unwrap_or(0);
        let remaining = &src[offset..];

        if self.target_value_ptr.is_null() || self.buffer_length <= 0 {
            let _ = self.write_length_or_null(LengthOrNull::Length(remaining.len() as sql::Len));
            return vec![Warning::StringDataTruncated];
        }

        let max_len = self.buffer_length as usize;
        let copy_len = std::cmp::min(remaining.len(), max_len - 1);

        unsafe {
            std::ptr::copy_nonoverlapping(
                remaining.as_ptr(),
                self.target_value_ptr as *mut u8,
                copy_len,
            );
            std::ptr::write((self.target_value_ptr as *mut u8).add(copy_len), 0);
        }

        let _ = self.write_length_or_null(LengthOrNull::Length(remaining.len() as sql::Len));

        if remaining.len() > max_len - 1 {
            *get_data_offset = Some(offset + copy_len);
            vec![Warning::StringDataTruncated]
        } else {
            *get_data_offset = None;
            vec![]
        }
    }

    pub fn write_binary(&self, src: &[u8], get_data_offset: &mut Option<usize>) -> Warnings {
        let offset = get_data_offset.unwrap_or(0);
        let remaining = &src[offset..];
        let buffer_length = self.buffer_length as usize;
        let copy_len = std::cmp::min(remaining.len(), buffer_length);

        unsafe {
            std::ptr::copy_nonoverlapping(
                remaining.as_ptr(),
                self.target_value_ptr as *mut u8,
                copy_len,
            );
        }

        let _ = self.write_length_or_null(LengthOrNull::Length(remaining.len() as sql::Len));

        if remaining.len() > buffer_length {
            *get_data_offset = Some(offset + copy_len);
            vec![Warning::StringDataTruncated]
        } else {
            *get_data_offset = None;
            vec![]
        }
    }

    pub fn write_wchar_string(&self, src: &str, get_data_offset: &mut Option<usize>) -> Warnings {
        if self.target_value_ptr.is_null() || self.buffer_length < 2 {
            let total_bytes = (src.encode_utf16().count() * 2) as sql::Len;
            let _ = self.write_length_or_null(LengthOrNull::Length(total_bytes));
            return vec![Warning::StringDataTruncated];
        }

        let offset = get_data_offset.unwrap_or(0);
        let max_len = (self.buffer_length / 2) as usize;
        let value_ptr = self.target_value_ptr as *mut u16;
        let mut dst_idx = 0;
        for c in src.encode_utf16().skip(offset) {
            if dst_idx == max_len - 1 {
                unsafe {
                    std::ptr::write(value_ptr.add(max_len - 1), 0);
                }
                let _ = self.write_length_or_null(LengthOrNull::Length(SQL_NO_TOTAL));
                *get_data_offset = Some(offset + dst_idx);
                return vec![Warning::StringDataTruncated];
            }
            unsafe {
                std::ptr::write(value_ptr.add(dst_idx), c);
            }
            dst_idx += 1;
        }
        unsafe {
            std::ptr::write(value_ptr.add(dst_idx), 0);
        }
        // COMPATIBILITY: ODBC 3.80 specification says that the string length should be the number of characters, not the number of bytes.
        // However, older versions of Snowflake ODBC driver returns the number of bytes.
        let num_bytes = (dst_idx as sql::Len) * 2;
        let _ = self.write_length_or_null(LengthOrNull::Length(num_bytes));
        *get_data_offset = None;
        vec![]
    }
}

pub trait WriteODBCType: SnowflakeType {
    fn sql_type(&self) -> sql::SqlDataType;

    fn column_size(&self) -> sql::ULen;

    fn decimal_digits(&self) -> sql::SmallInt;

    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
        get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, WriteOdbcError>;
}

pub trait SnowflakeType {
    type Representation<'a>: Sized;
}

pub trait ReadArrowType<ArrowArrayType>: SnowflakeType {
    #[allow(clippy::wrong_self_convention)]
    fn read_arrow_type<'a>(
        &self,
        array: &'a ArrowArrayType,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError>;
}
