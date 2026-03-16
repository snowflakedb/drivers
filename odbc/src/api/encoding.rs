use crate::api::OdbcResult;
use crate::api::error::{
    InvalidBufferLengthSnafu, NullPointerSnafu, TextConversionFromUtf8Snafu,
    TextConversionFromUtf16Snafu, TextConversionUtf8Snafu,
};
use crate::conversion::warning::{Warning, Warnings};
use odbc_sys as sql;
use snafu::ResultExt;
use std::cmp::min;
use std::sync::OnceLock;

#[cfg(not(windows))]
pub fn is_ascii_locale() -> bool {
    static RESULT: OnceLock<bool> = OnceLock::new();
    *RESULT.get_or_init(|| {
        let locale = unsafe { libc::setlocale(libc::LC_CTYPE, std::ptr::null()) };
        if locale.is_null() {
            return false;
        }
        let locale_str = unsafe { std::ffi::CStr::from_ptr(locale) };
        matches!(locale_str.to_bytes(), b"C" | b"POSIX")
    })
}

#[cfg(windows)]
pub fn is_ascii_locale() -> bool {
    false
}

pub fn mask_non_ascii_characters(src: &str) -> String {
    src.chars()
        .map(|c| if !c.is_ascii() { '\x1a' } else { c })
        .collect()
}

/// Abstracts over ANSI (narrow) and Unicode (wide) ODBC string operations,
/// allowing API-layer functions to be written once as generics.
pub trait OdbcEncoding {
    type Char;

    /// Read a string from an ODBC input buffer.
    fn read_string(text: *const Self::Char, length: sql::Integer) -> OdbcResult<String>;

    /// Core write: copy a Rust string into an ODBC output buffer.
    ///
    /// `buffer_length` is in **encoding units** (bytes for narrow, `u16` code
    /// units for wide), including space for the null terminator.
    ///
    /// Returns `(full_untruncated_length_in_encoding_units, was_truncated)`.
    fn write_string(string: &str, buffer: *mut Self::Char, buffer_length: usize) -> (usize, bool);
}

/// Marker type for ANSI (narrow, `sql::Char` / `u8`) encoding.
pub struct Narrow;

/// Marker type for Unicode (wide, `sql::WChar` / `u16`) encoding.
pub struct Wide;

impl OdbcEncoding for Narrow {
    type Char = sql::Char;

    fn read_string(text: *const Self::Char, length: sql::Integer) -> OdbcResult<String> {
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
            let cstr =
                unsafe { std::ffi::CStr::from_ptr(text as *const std::os::raw::c_char).to_str() };
            cstr.context(TextConversionUtf8Snafu {}).map(String::from)
        } else {
            let slice = unsafe { std::slice::from_raw_parts(text, length as usize) };
            String::from_utf8(slice.to_vec()).context(TextConversionFromUtf8Snafu {})
        }
    }

    fn write_string(string: &str, buffer: *mut Self::Char, buffer_length: usize) -> (usize, bool) {
        let write_inner = |string: &str| {
            let full_len = string.len();
            if buffer.is_null() {
                return (full_len, false);
            }
            if buffer_length == 0 {
                return (full_len, full_len > 0);
            }
            let max_len = buffer_length.saturating_sub(1);
            let copy_len = min(full_len, max_len);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    string.as_ptr() as *const sql::Char,
                    buffer,
                    copy_len,
                );
                *buffer.add(copy_len) = 0;
            }
            (full_len, full_len > max_len)
        };
        if is_ascii_locale() {
            write_inner(&mask_non_ascii_characters(string))
        } else {
            write_inner(string)
        }
    }
}

impl OdbcEncoding for Wide {
    type Char = sql::WChar;

    fn read_string(text: *const Self::Char, length: sql::Integer) -> OdbcResult<String> {
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
            // Find null terminator
            let mut null_terminator_index = 0;
            unsafe {
                while *text.add(null_terminator_index) != 0 {
                    null_terminator_index += 1;
                }
            }
            let slice = unsafe { std::slice::from_raw_parts(text, null_terminator_index) };
            String::from_utf16(slice).context(TextConversionFromUtf16Snafu {})
        } else {
            let slice = unsafe { std::slice::from_raw_parts(text, length as usize) };
            String::from_utf16(slice).context(TextConversionFromUtf16Snafu {})
        }
    }

    fn write_string(string: &str, buffer: *mut Self::Char, buffer_length: usize) -> (usize, bool) {
        let utf16: Vec<u16> = string.encode_utf16().collect();
        let full_len = utf16.len();
        if buffer.is_null() {
            return (full_len, false);
        }
        if buffer_length == 0 {
            return (full_len, full_len > 0);
        }
        let max_chars = buffer_length.saturating_sub(1);
        let copy_len = min(full_len, max_chars);
        unsafe {
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), buffer, copy_len);
            *buffer.add(copy_len) = 0;
        }
        (full_len, full_len > max_chars)
    }
}

// ---------------------------------------------------------------------------
// Input helpers
// ---------------------------------------------------------------------------

/// Read a string from an `sql::Pointer` where `string_length` is in **bytes**.
/// Returns an empty string if the pointer is null.
///
/// Used by: `SQLSetConnectAttr`.
pub fn read_string_from_pointer<E: OdbcEncoding>(
    value_ptr: sql::Pointer,
    string_length: sql::Integer,
) -> OdbcResult<String> {
    if value_ptr.is_null() {
        return Ok(String::new());
    }
    let char_size = std::mem::size_of::<E::Char>() as sql::Integer;
    let length_in_chars = string_length / char_size;
    E::read_string(value_ptr as *const E::Char, length_in_chars)
}

// ---------------------------------------------------------------------------
// Output helpers
//
// Each helper wraps `E::write_string` with the length-unit and integer-type
// conventions of a particular group of ODBC functions.
// ---------------------------------------------------------------------------

/// Write a string where `buffer_length` and `*string_length_ptr` count
/// **encoding units** (characters) as `sql::SmallInt`.
///
/// Used by: `SQLGetDiagRec`, `SQLDescribeCol`.
pub fn write_string_chars<E: OdbcEncoding>(
    string: &str,
    buffer: *mut E::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    warnings: Option<&mut Warnings>,
) {
    let buf_units = if buffer_length < 0 {
        0
    } else {
        buffer_length as usize
    };
    let (char_len, truncated) = E::write_string(string, buffer, buf_units);
    if !string_length_ptr.is_null() {
        unsafe { std::ptr::write(string_length_ptr, char_len as sql::SmallInt) };
    }
    if truncated && let Some(w) = warnings {
        w.push(Warning::StringDataTruncated);
    }
}

/// Write a string where `buffer_length` and `*string_length_ptr` count
/// **bytes** as `sql::SmallInt`.
///
/// For Narrow this is identical to `write_string_chars` (1 byte = 1 encoding
/// unit). For Wide the byte buffer length is divided by `size_of::<WChar>()`
/// to obtain encoding units, and the reported length is multiplied back.
///
/// Used by: `SQLGetDiagField`, `SQLGetInfo`.
pub fn write_string_bytes<E: OdbcEncoding>(
    string: &str,
    buffer: *mut E::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    warnings: Option<&mut Warnings>,
) {
    let char_size = std::mem::size_of::<E::Char>();
    let buf_bytes = if buffer_length < 0 {
        0
    } else {
        buffer_length as usize
    };
    let buf_units = buf_bytes / char_size;
    let (char_len, truncated) = E::write_string(string, buffer, buf_units);

    if !string_length_ptr.is_null() {
        let byte_len = char_len * char_size;
        unsafe { std::ptr::write(string_length_ptr, byte_len as sql::SmallInt) };
    }
    if truncated && let Some(w) = warnings {
        w.push(Warning::StringDataTruncated);
    }
}

/// Write a string where `buffer_length` and `*string_length_ptr` count
/// **bytes** as `sql::Integer`.
///
/// Used by: `SQLGetConnectAttr`.
pub fn write_string_bytes_i32<E: OdbcEncoding>(
    string: &str,
    buffer: *mut E::Char,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
    warnings: Option<&mut Warnings>,
) {
    let char_size = std::mem::size_of::<E::Char>();
    let buf_bytes = if buffer_length < 0 {
        0
    } else {
        buffer_length as usize
    };
    let buf_units = buf_bytes / char_size;
    let (char_len, truncated) = E::write_string(string, buffer, buf_units);

    if !string_length_ptr.is_null() {
        let byte_len = char_len * char_size;
        unsafe { std::ptr::write(string_length_ptr, byte_len as sql::Integer) };
    }
    if truncated && let Some(w) = warnings {
        w.push(Warning::StringDataTruncated);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_non_ascii_preserves_pure_ascii() {
        assert_eq!(mask_non_ascii_characters("Hello"), "Hello");
    }

    #[test]
    fn mask_non_ascii_preserves_empty_string() {
        assert_eq!(mask_non_ascii_characters(""), "");
    }

    #[test]
    fn mask_non_ascii_replaces_japanese_characters() {
        assert_eq!(mask_non_ascii_characters("日本語"), "\x1a\x1a\x1a");
    }

    #[test]
    fn mask_non_ascii_replaces_mixed_string() {
        assert_eq!(mask_non_ascii_characters("Hello日World"), "Hello\x1aWorld");
    }

    #[test]
    fn mask_non_ascii_replaces_emojis() {
        assert_eq!(mask_non_ascii_characters("⛄🚀🎉"), "\x1a\x1a\x1a");
    }

    #[test]
    fn mask_non_ascii_replaces_greek_letters() {
        assert_eq!(mask_non_ascii_characters("αβγδ"), "\x1a\x1a\x1a\x1a");
    }

    #[test]
    fn mask_non_ascii_replaces_combined_characters() {
        assert_eq!(mask_non_ascii_characters("y\u{0306}es"), "y\x1aes");
    }

    #[test]
    fn mask_non_ascii_replaces_surrogate_pair_character() {
        assert_eq!(mask_non_ascii_characters("𝄞"), "\x1a");
    }
}
