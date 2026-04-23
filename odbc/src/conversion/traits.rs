use odbc_sys as sql;
use serde_json::Value;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::{
    IndicatorRequiredSnafu, JsonBindingError, ReadArrowError, WriteOdbcError,
};
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

/// ARD stride parameters that decide how a base [`Binding`] is laid out
/// across the rows of a block-cursor fetch. `bind_type` is
/// `SQL_ATTR_ROW_BIND_TYPE` (0 ⇒ column-wise; non-zero ⇒ row-wise byte
/// stride); `bind_offset` is the value of `*SQL_ATTR_ROW_BIND_OFFSET_PTR`
/// at fetch entry. `Copy` so it can be passed by value into the per-row
/// hot path with no indirection.
#[derive(Debug, Default, Clone, Copy)]
pub struct BindingStrides {
    pub bind_type: usize,
    pub bind_offset: isize,
}

impl BindingStrides {
    /// Materialise the [`Binding`] that targets row `row_idx` within the
    /// application's bound buffers, given the column's `base` binding.
    /// `#[inline]` so callers in `convert_arrow_range` monomorphise and
    /// LLVM can hoist the stride math.
    #[inline]
    pub fn for_row(self, base: &Binding, row_idx: usize) -> Binding {
        let value_stride = if self.bind_type == 0 {
            base.target_type
                .fixed_size()
                .unwrap_or(base.buffer_length as usize)
        } else {
            self.bind_type
        };
        let indicator_stride = if self.bind_type == 0 {
            size_of::<sql::Len>()
        } else {
            self.bind_type
        };
        Binding {
            target_type: base.target_type,
            target_value_ptr: advance_ptr(
                base.target_value_ptr,
                row_idx,
                value_stride,
                self.bind_offset,
            ),
            buffer_length: base.buffer_length,
            octet_length_ptr: advance_ptr(
                base.octet_length_ptr,
                row_idx,
                indicator_stride,
                self.bind_offset,
            ),
            indicator_ptr: advance_ptr(
                base.indicator_ptr,
                row_idx,
                indicator_stride,
                self.bind_offset,
            ),
            precision: base.precision,
            scale: base.scale,
            datetime_interval_precision: base.datetime_interval_precision,
        }
    }
}

#[inline]
fn advance_ptr<T>(
    ptr: *mut T,
    row_idx: usize,
    element_stride: usize,
    bind_offset: isize,
) -> *mut T {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let stride = row_idx
        .checked_mul(element_stride)
        .expect("row index and element stride multiplication overflowed");
    let byte_ptr = ptr as *mut u8;
    unsafe { byte_ptr.offset(bind_offset).add(stride) as *mut T }
}

#[derive(Debug, Default, Clone, Copy)]
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
                    std::ptr::write(self.indicator_ptr, crate::api::SQL_NULL_DATA);
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

    /// Helper for writing data generated by a function to a buffer.
    unsafe fn write_from_fn_impl<T: Copy>(
        target: *mut T,
        offset: usize,
        max_write_len: usize,
        converter: impl Fn(usize) -> Option<T>,
    ) -> usize {
        let mut written = 0;
        for pos in offset..offset + max_write_len {
            if let Some(element) = converter(pos) {
                unsafe {
                    std::ptr::write(target.add(written), element);
                }
                written += 1;
            } else {
                break;
            }
        }
        written
    }

    /// Write char data to a char buffer, with each byte generated by a function.
    /// `total_char_count` is the total number of output characters (= bytes for SQL_C_CHAR).
    pub fn write_char_from_fn<F>(
        &self,
        converter: F,
        total_char_count: sql::Len,
        get_data_offset: &mut Option<usize>,
    ) -> Warnings
    where
        F: Fn(usize) -> Option<u8>,
    {
        let offset = get_data_offset.unwrap_or(0);
        let remaining = total_char_count.saturating_sub(offset as sql::Len);

        if self.target_value_ptr.is_null() || self.buffer_length <= 0 {
            if remaining == 0 {
                *get_data_offset = None;
                let _ = self.write_length_or_null(LengthOrNull::Length(0));
                return vec![];
            }
            let _ = self.write_length_or_null(LengthOrNull::Length(remaining));
            return vec![Warning::StringDataTruncated];
        }

        let max_write_len = (self.buffer_length - 1) as usize;
        let written = unsafe {
            let target = self.target_value_ptr as *mut u8;
            let written = Self::write_from_fn_impl(target, offset, max_write_len, converter);
            std::ptr::write(target.add(written), 0);
            written
        };

        let _ = self.write_length_or_null(LengthOrNull::Length(remaining));

        let new_offset = offset + written;
        if new_offset < total_char_count as usize {
            *get_data_offset = Some(new_offset);
            vec![Warning::StringDataTruncated]
        } else {
            *get_data_offset = None;
            vec![]
        }
    }

    /// Write wide-char data to a wide-char buffer, with each code unit generated by a function.
    /// `total_char_count` is the total number of output wide characters.
    /// The indicator is set in bytes (chars * 2) per ODBC wide-char convention.
    pub fn write_wchar_from_fn<F>(
        &self,
        converter: F,
        total_char_count: sql::Len,
        get_data_offset: &mut Option<usize>,
    ) -> Warnings
    where
        F: Fn(usize) -> Option<u16>,
    {
        let offset = get_data_offset.unwrap_or(0);
        let remaining_chars = total_char_count.saturating_sub(offset as sql::Len);
        let remaining_bytes = remaining_chars.saturating_mul(2);

        if self.target_value_ptr.is_null() || self.buffer_length < 2 {
            if remaining_chars == 0 {
                *get_data_offset = None;
                let _ = self.write_length_or_null(LengthOrNull::Length(0));
                return vec![];
            }
            let _ = self.write_length_or_null(LengthOrNull::Length(remaining_bytes));
            return vec![Warning::StringDataTruncated];
        }

        let max_write_len = ((self.buffer_length / 2) - 1) as usize;
        let written = unsafe {
            let target = self.target_value_ptr as *mut u16;
            let written = Self::write_from_fn_impl(target, offset, max_write_len, converter);
            std::ptr::write(target.add(written), 0);
            written
        };

        let _ = self.write_length_or_null(LengthOrNull::Length(remaining_bytes));

        let new_offset = offset + written;
        if new_offset < total_char_count as usize {
            *get_data_offset = Some(new_offset);
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
                // Return remaining byte count instead of SQL_NO_TOTAL (BD#23).
                // The ODBC spec says the indicator should contain the data length
                // when determinable, and ours always is.
                let remaining_bytes = (src.encode_utf16().count() - offset) as sql::Len * 2;
                let _ = self.write_length_or_null(LengthOrNull::Length(remaining_bytes));
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

/// Snowflake logical type names used in the binding protocol and Arrow metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnowflakeLogicalType {
    Any,
    Fixed,
    Text,
    Real,
    Boolean,
    Binary,
    Date,
    Time,
    TimestampNtz,
    TimestampLtz,
    TimestampTz,
}

impl SnowflakeLogicalType {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Any => "ANY",
            Self::Fixed => "FIXED",
            Self::Text => "TEXT",
            Self::Real => "REAL",
            Self::Boolean => "BOOLEAN",
            Self::Binary => "BINARY",
            Self::Date => "DATE",
            Self::Time => "TIME",
            Self::TimestampNtz => "TIMESTAMP_NTZ",
            Self::TimestampLtz => "TIMESTAMP_LTZ",
            Self::TimestampTz => "TIMESTAMP_TZ",
        }
    }
}

impl std::fmt::Display for SnowflakeLogicalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Reads a typed value from a raw ODBC `ParameterBinding` buffer.
pub(crate) trait ReadODBC: SnowflakeType {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, JsonBindingError>;
}

/// Converts a typed representation into a JSON value for the Snowflake binding protocol.
pub(crate) trait WriteJson: SnowflakeType {
    fn write_json(&self, value: Self::Representation<'_>) -> Result<Value, JsonBindingError>;
    fn sf_type(&self) -> SnowflakeLogicalType;
}

#[cfg(test)]
mod binding_strides_tests {
    use super::*;

    /// Pointer fields are non-null sentinels but never dereferenced — only
    /// the address arithmetic produced by `for_row` is checked.
    fn base_binding(target_type: CDataType, buffer_length: sql::Len) -> Binding {
        Binding {
            target_type,
            target_value_ptr: 1024usize as sql::Pointer,
            buffer_length,
            octet_length_ptr: 2048usize as *mut sql::Len,
            indicator_ptr: 4096usize as *mut sql::Len,
            ..Default::default()
        }
    }

    #[test]
    fn column_wise_uses_fixed_size_for_value_stride() {
        let base = base_binding(CDataType::SBigInt, 0);
        let strides = BindingStrides {
            bind_type: 0,
            bind_offset: 0,
        };
        let row3 = strides.for_row(&base, 3);
        assert_eq!(row3.target_value_ptr as usize, 1024 + 3 * 8);
        let len_size = size_of::<sql::Len>();
        assert_eq!(row3.octet_length_ptr as usize, 2048 + 3 * len_size);
        assert_eq!(row3.indicator_ptr as usize, 4096 + 3 * len_size);
    }

    #[test]
    fn column_wise_falls_back_to_buffer_length_for_variable_size_type() {
        let base = base_binding(CDataType::Char, 64);
        let strides = BindingStrides {
            bind_type: 0,
            bind_offset: 0,
        };
        let row5 = strides.for_row(&base, 5);
        assert_eq!(row5.target_value_ptr as usize, 1024 + 5 * 64);
    }

    #[test]
    fn row_wise_uses_bind_type_for_every_pointer_stride() {
        let base = base_binding(CDataType::SBigInt, 0);
        let strides = BindingStrides {
            bind_type: 64,
            bind_offset: 0,
        };
        let row2 = strides.for_row(&base, 2);
        assert_eq!(row2.target_value_ptr as usize, 1024 + 2 * 64);
        assert_eq!(row2.octet_length_ptr as usize, 2048 + 2 * 64);
        assert_eq!(row2.indicator_ptr as usize, 4096 + 2 * 64);
    }

    #[test]
    fn bind_offset_is_applied_before_striding() {
        let base = base_binding(CDataType::SBigInt, 0);
        let strides = BindingStrides {
            bind_type: 0,
            bind_offset: 32,
        };
        let row1 = strides.for_row(&base, 1);
        assert_eq!(row1.target_value_ptr as usize, 1024 + 32 + 8);
        let len_size = size_of::<sql::Len>();
        assert_eq!(row1.octet_length_ptr as usize, 2048 + 32 + len_size);
        assert_eq!(row1.indicator_ptr as usize, 4096 + 32 + len_size);
    }

    #[test]
    fn negative_bind_offset_walks_pointer_back() {
        let base = base_binding(CDataType::SBigInt, 0);
        let strides = BindingStrides {
            bind_type: 0,
            bind_offset: -16,
        };
        let row0 = strides.for_row(&base, 0);
        assert_eq!(row0.target_value_ptr as usize, 1024 - 16);
    }

    #[test]
    fn null_pointers_remain_null_after_adjustment() {
        let mut base = base_binding(CDataType::SBigInt, 0);
        base.octet_length_ptr = std::ptr::null_mut();
        base.indicator_ptr = std::ptr::null_mut();
        let strides = BindingStrides {
            bind_type: 0,
            bind_offset: 64,
        };
        let row7 = strides.for_row(&base, 7);
        assert!(row7.octet_length_ptr.is_null());
        assert!(row7.indicator_ptr.is_null());
        assert_eq!(row7.target_value_ptr as usize, 1024 + 64 + 7 * 8);
    }

    #[test]
    fn metadata_fields_are_propagated_unchanged() {
        let mut base = base_binding(CDataType::Numeric, size_of::<sql::Numeric>() as sql::Len);
        base.precision = Some(10);
        base.scale = Some(2);
        base.datetime_interval_precision = Some(6);
        let strides = BindingStrides {
            bind_type: 0,
            bind_offset: 0,
        };
        let row4 = strides.for_row(&base, 4);
        assert_eq!(row4.precision, Some(10));
        assert_eq!(row4.scale, Some(2));
        assert_eq!(row4.datetime_interval_precision, Some(6));
        assert_eq!(row4.target_type, CDataType::Numeric);
        assert_eq!(row4.buffer_length, base.buffer_length);
    }
}
