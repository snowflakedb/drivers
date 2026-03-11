use crate::api::diagnostic::DiagRecData;
use crate::api::error::{EncodingSnafu, InvalidBufferLengthSnafu, LengthOverflowSnafu};
use crate::api::{self, FieldValue};
use crate::conversion::warning::{Warning, Warnings};
use odbc_sys as sql;
use snafu::ResultExt;
use std::mem;

fn write_numeric_length<L: TryFrom<usize>, V>(ptr: *mut L) -> api::OdbcResult<()> {
    if !ptr.is_null() {
        let len = L::try_from(mem::size_of::<V>()).map_err(|_| {
            LengthOverflowSnafu {
                value: mem::size_of::<V>(),
            }
            .build()
        })?;
        unsafe { std::ptr::write(ptr, len) };
    }
    Ok(())
}

/// Write a Rust `&str` to a narrow ODBC output buffer, following ODBC conventions.
///
/// Encodes `s` into `buf` (up to `buf_len` bytes), null-terminates the output,
/// sets `*string_length_ptr` to the full (untruncated) byte length (if non-null),
/// and pushes [`Warning::StringDataTruncated`] to `warnings` if truncated.
///
/// # Safety
///
/// `buf` must point to a writable buffer of at least `buf_len` bytes (or be null).
/// `string_length_ptr` must be valid and writable (or be null).
#[cfg(not(windows))]
pub unsafe fn write_char_to_buffer<L: Into<i32> + TryFrom<usize>>(
    s: &str,
    buf: *mut u8,
    buf_len: L,
    string_length_ptr: *mut L,
    warnings: &mut Warnings,
) -> api::OdbcResult<()> {
    let buf_len: i32 = buf_len.into();
    if buf_len < 0 {
        return InvalidBufferLengthSnafu {
            length: buf_len as i64,
        }
        .fail();
    }
    let buf_len_clamped = buf_len as usize;
    let result =
        unsafe { crate::encoding::encode_char(s, buf, buf_len_clamped) }.context(EncodingSnafu)?;
    if !string_length_ptr.is_null() {
        let len = L::try_from(result.total_len).map_err(|_| {
            LengthOverflowSnafu {
                value: result.total_len,
            }
            .build()
        })?;
        unsafe { std::ptr::write(string_length_ptr, len) };
    }
    if result.truncated && !buf.is_null() {
        warnings.push(Warning::StringDataTruncated);
    }
    Ok(())
}

/// Write a Rust `&str` to a wide ODBC output buffer, following ODBC conventions.
///
/// Encodes `s` into `buf` as UTF-16 (up to `buf_len` bytes worth of `u16` elements),
/// null-terminates the output, sets `*string_length_ptr` to the full (untruncated)
/// length in bytes (not code units), and pushes [`Warning::StringDataTruncated`]
/// to `warnings` if truncated.
///
/// # Safety
///
/// `buf` must point to a writable buffer of at least `buf_len` bytes worth of `u16`
/// elements (or be null). `string_length_ptr` must be valid and writable (or be null).
pub unsafe fn write_wchar_to_buffer<L: Into<isize> + TryFrom<usize>>(
    s: &str,
    buf: *mut u16,
    buf_len: L,
    string_length_ptr: *mut L,
    warnings: &mut Warnings,
) -> api::OdbcResult<()> {
    let buf_len: isize = buf_len.into();
    if buf_len < 0 {
        return InvalidBufferLengthSnafu {
            length: buf_len as i64,
        }
        .fail();
    }
    let buf_units = if buf_len > 0 {
        (buf_len / 2) as usize
    } else {
        0
    };
    let result =
        unsafe { crate::encoding::encode_wchar(s, buf, buf_units) }.context(EncodingSnafu)?;
    if !string_length_ptr.is_null() {
        let byte_len = result.total_len * 2;
        let len =
            L::try_from(byte_len).map_err(|_| LengthOverflowSnafu { value: byte_len }.build())?;
        unsafe { std::ptr::write(string_length_ptr, len) };
    }
    if result.truncated && !buf.is_null() {
        warnings.push(Warning::StringDataTruncated);
    }
    Ok(())
}

/// Write a [`FieldValue`] to ODBC output buffers using locale (narrow) encoding.
///
/// For numeric values: writes directly to `value_ptr`.
/// For string values: encodes to `value_ptr` as locale chars with `buffer_length` limit,
/// and sets `*string_length_ptr` to the full (untruncated) string length.
/// Pushes [`Warning::StringDataTruncated`] to `warnings` if a string was truncated.
///
/// # Safety
///
/// `value_ptr` must be a valid, writable pointer of the correct type (or null).
/// `string_length_ptr` must be valid and writable (or null).
#[cfg(not(windows))]
pub unsafe fn write_field_value<L: Into<i32> + TryFrom<usize>>(
    value: FieldValue,
    value_ptr: sql::Pointer,
    buffer_length: L,
    string_length_ptr: *mut L,
    warnings: &mut Warnings,
) -> api::OdbcResult<()> {
    match value {
        FieldValue::USmallInt(v) => {
            if !value_ptr.is_null() {
                unsafe { std::ptr::write(value_ptr as *mut u16, v) };
            }
            write_numeric_length::<L, u16>(string_length_ptr)?;
        }
        FieldValue::UInteger(v) => {
            if !value_ptr.is_null() {
                unsafe { std::ptr::write(value_ptr as *mut u32, v) };
            }
            write_numeric_length::<L, u32>(string_length_ptr)?;
        }
        FieldValue::Integer(v) => {
            if !value_ptr.is_null() {
                unsafe { std::ptr::write(value_ptr as *mut sql::Integer, v) };
            }
            write_numeric_length::<L, sql::Integer>(string_length_ptr)?;
        }
        FieldValue::Len(v) => {
            if !value_ptr.is_null() {
                unsafe { std::ptr::write(value_ptr as *mut sql::Len, v) };
            }
            write_numeric_length::<L, sql::Len>(string_length_ptr)?;
        }
        FieldValue::ULen(v) => {
            if !value_ptr.is_null() {
                unsafe { std::ptr::write(value_ptr as *mut sql::ULen, v) };
            }
            write_numeric_length::<L, sql::ULen>(string_length_ptr)?;
        }
        FieldValue::RetCode(v) => {
            if !value_ptr.is_null() {
                unsafe { std::ptr::write(value_ptr as *mut sql::RetCode, v) };
            }
            write_numeric_length::<L, sql::RetCode>(string_length_ptr)?;
        }
        FieldValue::String(s) => {
            unsafe {
                write_char_to_buffer(
                    &s,
                    value_ptr as *mut u8,
                    buffer_length,
                    string_length_ptr,
                    warnings,
                )
            }?;
        }
    }
    Ok(())
}

/// Write a [`DiagRecData`] to the output buffers provided by the caller.
#[cfg(not(windows))]
pub fn write_diag_rec_to_buffers(
    diag: &DiagRecData,
    sql_state: *mut sql::Char,
    native_error_ptr: *mut sql::Integer,
    message_text: *mut sql::Char,
    buffer_length: sql::SmallInt,
    text_length_ptr: *mut sql::SmallInt,
    warnings: &mut Warnings,
) -> api::OdbcResult<()> {
    if !sql_state.is_null() {
        let state_str = diag.sql_state.as_str();
        let state_bytes = state_str.as_bytes();
        let len = std::cmp::min(state_bytes.len(), 5);
        unsafe {
            std::ptr::copy_nonoverlapping(state_bytes.as_ptr(), sql_state, len);
            *sql_state.add(len) = 0;
        }
    }
    if !native_error_ptr.is_null() {
        unsafe { std::ptr::write(native_error_ptr, diag.native_error) };
    }

    unsafe {
        write_char_to_buffer(
            &diag.message_text,
            message_text,
            buffer_length,
            text_length_ptr,
            warnings,
        )
    }
}

/// Write a [`FieldValue`] to ODBC output buffers using wide (UTF-16) encoding.
///
/// # Safety
///
/// `value_ptr` must be a valid, writable pointer of the correct type (or null).
/// `string_length_ptr` must be valid and writable (or null).
pub unsafe fn write_field_value_w<L: Into<isize> + TryFrom<usize>>(
    value: FieldValue,
    value_ptr: sql::Pointer,
    buffer_length: L,
    string_length_ptr: *mut L,
    warnings: &mut Warnings,
) -> api::OdbcResult<()> {
    match value {
        FieldValue::USmallInt(v) => {
            if !value_ptr.is_null() {
                unsafe { std::ptr::write(value_ptr as *mut u16, v) };
            }
            write_numeric_length::<L, u16>(string_length_ptr)?;
        }
        FieldValue::UInteger(v) => {
            if !value_ptr.is_null() {
                unsafe { std::ptr::write(value_ptr as *mut u32, v) };
            }
            write_numeric_length::<L, u32>(string_length_ptr)?;
        }
        FieldValue::Integer(v) => {
            if !value_ptr.is_null() {
                unsafe { std::ptr::write(value_ptr as *mut sql::Integer, v) };
            }
            write_numeric_length::<L, sql::Integer>(string_length_ptr)?;
        }
        FieldValue::Len(v) => {
            if !value_ptr.is_null() {
                unsafe { std::ptr::write(value_ptr as *mut sql::Len, v) };
            }
            write_numeric_length::<L, sql::Len>(string_length_ptr)?;
        }
        FieldValue::ULen(v) => {
            if !value_ptr.is_null() {
                unsafe { std::ptr::write(value_ptr as *mut sql::ULen, v) };
            }
            write_numeric_length::<L, sql::ULen>(string_length_ptr)?;
        }
        FieldValue::RetCode(v) => {
            if !value_ptr.is_null() {
                unsafe { std::ptr::write(value_ptr as *mut sql::RetCode, v) };
            }
            write_numeric_length::<L, sql::RetCode>(string_length_ptr)?;
        }
        FieldValue::String(s) => {
            unsafe {
                write_wchar_to_buffer(
                    &s,
                    value_ptr as *mut u16,
                    buffer_length,
                    string_length_ptr,
                    warnings,
                )
            }?;
        }
    }
    Ok(())
}

/// Write a [`DiagRecData`] to wide (UTF-16) output buffers provided by the caller.
pub fn write_diag_rec_to_buffers_w(
    diag: &DiagRecData,
    sql_state: *mut sql::WChar,
    native_error_ptr: *mut sql::Integer,
    message_text: *mut sql::WChar,
    buffer_length: sql::SmallInt,
    text_length_ptr: *mut sql::SmallInt,
    warnings: &mut Warnings,
) -> api::OdbcResult<()> {
    if !sql_state.is_null() {
        let state_str = diag.sql_state.as_str();
        let units: Vec<u16> = state_str.encode_utf16().collect();
        let len = std::cmp::min(units.len(), 5);
        unsafe {
            std::ptr::copy_nonoverlapping(units.as_ptr(), sql_state, len);
            *sql_state.add(len) = 0;
        }
    }
    if !native_error_ptr.is_null() {
        unsafe { std::ptr::write(native_error_ptr, diag.native_error) };
    }

    let buf_len_bytes = buffer_length as isize;
    let mut dummy_len: isize = 0;
    let dummy_ptr: *mut isize = &mut dummy_len;

    unsafe {
        write_wchar_to_buffer(
            &diag.message_text,
            message_text,
            buf_len_bytes,
            dummy_ptr,
            warnings,
        )
    }?;

    if !text_length_ptr.is_null() {
        let len = sql::SmallInt::try_from(dummy_len as usize).map_err(|_| {
            LengthOverflowSnafu {
                value: dummy_len as usize,
            }
            .build()
        })?;
        unsafe { std::ptr::write(text_length_ptr, len) };
    }

    Ok(())
}
