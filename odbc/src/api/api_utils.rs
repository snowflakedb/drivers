use std::cmp::min;

use crate::api::OdbcResult;
use crate::api::error::{
    InvalidBufferLengthSnafu, NullPointerSnafu, TextConversionFromUtf8Snafu,
    TextConversionFromUtf16Snafu,
};
use odbc_sys as sql;
use snafu::ResultExt;

#[allow(dead_code)]
pub fn cstr_to_string(text: *const sql::Char, length: sql::Integer) -> OdbcResult<String> {
    if text.is_null() {
        return NullPointerSnafu.fail();
    }
    if length != sql::NTS as i32 && length <= 0 {
        return InvalidBufferLengthSnafu {
            length: length as i64,
        }
        .fail();
    }
    if length == sql::NTS as i32 {
        let mut len = 0;
        unsafe {
            while *text.add(len) != 0 {
                len += 1;
            }
        }
        let text_slice = unsafe { std::slice::from_raw_parts(text, len) };
        String::from_utf8(text_slice.to_vec()).context(TextConversionFromUtf8Snafu {})
    } else {
        let text_slice = unsafe { std::slice::from_raw_parts(text, length as usize) };
        String::from_utf8(text_slice.to_vec()).context(TextConversionFromUtf8Snafu {})
    }
}

pub fn utf16_to_string(text: *const sql::WChar, length: sql::Integer) -> OdbcResult<String> {
    if text.is_null() {
        return NullPointerSnafu.fail();
    }
    if length != sql::NTS as i32 && length <= 0 {
        return InvalidBufferLengthSnafu {
            length: length as i64,
        }
        .fail();
    }
    if length == sql::NTS as i32 {
        let mut len = 0;
        unsafe {
            while *text.add(len) != 0 {
                len += 1;
            }
        }
        let text_slice = unsafe { std::slice::from_raw_parts(text, len) };
        String::from_utf16(text_slice).context(TextConversionFromUtf16Snafu {})
    } else {
        let text_slice = unsafe { std::slice::from_raw_parts(text, length as usize) };
        String::from_utf16(text_slice).context(TextConversionFromUtf16Snafu {})
    }
}

pub fn string_to_cstr(
    string: &str,
    buffer: *mut sql::Char,
    buffer_length: sql::Len,
) -> OdbcResult<()> {
    if buffer.is_null() || buffer_length <= 0 {
        return Ok(());
    }
    unsafe {
        let max_len = (buffer_length - 1) as usize; // reserve space for NUL terminator
        let length = min(string.len(), max_len);
        std::ptr::copy_nonoverlapping(string.as_ptr() as *const sql::Char, buffer, length);
        // NUL terminate
        *buffer.add(length) = 0;
    }
    Ok(())
}

/// Write a Rust string to a wide-character (UTF-16) ODBC output buffer.
/// `buffer_length` is the number of wide characters (not bytes) the buffer can hold,
/// including space for the null terminator.
/// Returns `true` if the string was truncated.
pub fn string_to_wstr(
    string: &str,
    buffer: *mut sql::WChar,
    buffer_length_chars: sql::SmallInt,
) -> bool {
    let utf16: Vec<u16> = string.encode_utf16().collect();
    if buffer.is_null() || buffer_length_chars <= 0 {
        return utf16.len() >= buffer_length_chars.max(0) as usize;
    }
    let max_chars = (buffer_length_chars as usize).saturating_sub(1);
    let copy_len = min(utf16.len(), max_chars);
    unsafe {
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), buffer, copy_len);
        *buffer.add(copy_len) = 0; // null terminate
    }
    utf16.len() > max_chars
}

/// Read a string value from an ODBC input pointer (e.g. `SQLSetConnectAttr` value).
///
/// Returns an empty string if the pointer is null.
pub fn read_string_from_ptr(
    value_ptr: sql::Pointer,
    string_length: sql::Integer,
) -> OdbcResult<String> {
    if value_ptr.is_null() {
        return Ok(String::new());
    }
    cstr_to_string(value_ptr as *const sql::Char, string_length)
}

/// Write a string value to an ODBC output buffer, reporting full length and truncation.
///
/// Used for `SQLGetConnectAttr` and similar output-string functions.
/// Per the ODBC spec, `string_length_ptr` always receives the full (untruncated) length,
/// even when the buffer is too small.
///
/// Returns `true` if the value was truncated (caller should report `SQL_SUCCESS_WITH_INFO` / 01004).
pub fn write_string_to_buffer(
    value: &str,
    buffer: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> bool {
    // Always report the full length, even if truncated (per ODBC spec)
    if !string_length_ptr.is_null() {
        unsafe {
            *string_length_ptr = value.len() as sql::Integer;
        }
    }
    if !buffer.is_null() && buffer_length > 0 {
        let buf = buffer as *mut sql::Char;
        let max_len = min(value.len(), (buffer_length - 1) as usize);
        unsafe {
            std::ptr::copy_nonoverlapping(value.as_ptr() as *const sql::Char, buf, max_len);
            *buf.add(max_len) = 0; // NUL terminate
        }
        // Truncation occurred if the value is longer than the available buffer
        value.len() > (buffer_length - 1) as usize
    } else {
        false
    }
}
