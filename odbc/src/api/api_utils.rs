use std::cmp::min;

use crate::api::OdbcResult;
use crate::api::error::{TextConversionFromUtf8Snafu, TextConversionUtf8Snafu};
use odbc_sys as sql;
use snafu::ResultExt;

#[allow(dead_code)]
pub fn cstr_to_string(text: *const sql::Char, length: sql::Integer) -> OdbcResult<String> {
    if length == sql::NTS as i32 {
        let result = unsafe { std::ffi::CStr::from_ptr(text as *const i8).to_str() };
        result.context(TextConversionUtf8Snafu {}).map(String::from)
    } else {
        let text_slice = unsafe { std::slice::from_raw_parts(text, length as usize) };
        String::from_utf8(text_slice.to_vec()).context(TextConversionFromUtf8Snafu {})
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
