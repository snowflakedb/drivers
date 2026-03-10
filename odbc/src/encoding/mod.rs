#[cfg(target_os = "linux")]
mod platform_linux;
#[cfg(target_os = "macos")]
mod platform_macos;
#[cfg(windows)]
mod platform_windows;

use std::sync::OnceLock;

use encoding_rs::Encoding;
use error_trace::ErrorTrace;
use snafu::{Location, ResultExt, Snafu};

#[cfg(target_os = "linux")]
use platform_linux::detect_locale_encoding;
#[cfg(target_os = "macos")]
use platform_macos::detect_locale_encoding;
#[cfg(windows)]
use platform_windows::detect_locale_encoding;

/// Returns the locale encoding for the current platform.
///
/// - macOS: always UTF-8
/// - Linux: reads `LC_CTYPE` / `LANG`; defaults to UTF-8
/// - Windows: calls `GetACP()` and maps code page to encoding
pub fn locale_encoding() -> &'static Encoding {
    static ENCODING: OnceLock<&'static Encoding> = OnceLock::new();
    ENCODING.get_or_init(detect_locale_encoding)
}

/// Decode a narrow (`SQLCHAR*`) C string into a Rust `String`.
///
/// `length` follows ODBC conventions:
/// - `SQL_NTS` (-3): the string is null-terminated
/// - `> 0`: exact byte count (not necessarily null-terminated)
/// - `0`: empty string
///
/// The decoding uses the platform's locale encoding (UTF-8 on macOS/Linux, ACP on Windows).
///
/// # Safety
///
/// `ptr` must point to a valid, readable byte buffer of at least `length` bytes
/// (or be null-terminated if `length` is `SQL_NTS`).
pub unsafe fn decode_char(ptr: *const u8, length: i32) -> Result<String, EncodingError> {
    if ptr.is_null() {
        return NullPointerSnafu.fail();
    }
    let bytes = unsafe { char_ptr_to_slice(ptr, length) }?;
    let encoding = locale_encoding();
    if encoding == encoding_rs::UTF_8 {
        String::from_utf8(bytes.to_vec()).context(InvalidUtf8Snafu)
    } else {
        let (decoded, _, had_errors) = encoding.decode(bytes);
        if had_errors {
            DecodeFailedSnafu {
                encoding_name: encoding.name().to_string(),
            }
            .fail()
        } else {
            Ok(decoded.into_owned())
        }
    }
}

/// Encode a Rust `&str` into the platform's locale encoding and write it to a buffer.
///
/// Returns the total encoded length in bytes (excluding the null terminator) and whether
/// the output was truncated.
///
/// The output is always null-terminated. If the buffer is too small, the data is truncated
/// at a character boundary (no partial multi-byte sequences).
///
/// # Safety
///
/// `buf` must point to a writable buffer of at least `buf_len` bytes, or be null.
pub unsafe fn encode_char(
    s: &str,
    buf: *mut u8,
    buf_len: usize,
) -> Result<EncodedResult, EncodingError> {
    let encoding = locale_encoding();

    if buf.is_null() || buf_len == 0 {
        let total_len = if encoding == encoding_rs::UTF_8 {
            s.len()
        } else {
            encoding.encode(s).0.len()
        };
        return Ok(EncodedResult {
            total_len,
            written_len: 0,
            truncated: !s.is_empty(),
        });
    }

    let available = buf_len - 1;

    if encoding == encoding_rs::UTF_8 {
        let write_len = find_utf8_boundary(s.as_bytes(), available);
        unsafe {
            std::ptr::copy_nonoverlapping(s.as_bytes().as_ptr(), buf, write_len);
            *buf.add(write_len) = 0;
        }
        return Ok(EncodedResult {
            total_len: s.len(),
            written_len: write_len,
            truncated: s.len() > available,
        });
    }

    let out = unsafe { std::slice::from_raw_parts_mut(buf, buf_len) };
    let mut written = 0usize;
    let mut input_pos = 0usize;

    const CHUNK_SIZE: usize = 4096;

    while input_pos < s.len() {
        let remaining = &s[input_pos..];
        let remaining_space = available - written;

        let chunk_len = floor_char_boundary(remaining, remaining.len().min(CHUNK_SIZE));
        let encoded = encoding.encode(&remaining[..chunk_len]).0;

        if encoded.len() <= remaining_space {
            out[written..written + encoded.len()].copy_from_slice(&encoded);
            written += encoded.len();
            input_pos += chunk_len;
        } else {
            let fit = find_encode_fit(remaining, remaining_space, encoding);
            if fit > 0 {
                let encoded = encoding.encode(&remaining[..fit]).0;
                out[written..written + encoded.len()].copy_from_slice(&encoded);
                written += encoded.len();
            }
            input_pos += fit;
            break;
        }
    }

    out[written] = 0;

    let total_len = if input_pos >= s.len() {
        written
    } else {
        written + encoding.encode(&s[input_pos..]).0.len()
    };

    Ok(EncodedResult {
        total_len,
        written_len: written,
        truncated: total_len > available,
    })
}

/// Decode a wide (`SQLWCHAR*`) UTF-16 string into a Rust `String`.
///
/// `length` follows ODBC conventions:
/// - `SQL_NTS` (-3): scan for a null `u16` terminator
/// - `> 0`: count of `u16` code units
/// - `0`: empty string
///
/// # Safety
///
/// `ptr` must point to a valid, readable buffer of at least `length` `u16` elements
/// (or be null-terminated if `length` is `SQL_NTS`).
pub unsafe fn decode_wchar(ptr: *const u16, length: i32) -> Result<String, EncodingError> {
    if ptr.is_null() {
        return NullPointerSnafu.fail();
    }
    let slice = unsafe { wchar_ptr_to_slice(ptr, length) }?;
    String::from_utf16(slice).context(InvalidUtf16Snafu)
}

/// Encode a Rust `&str` as UTF-16 and write it to a wide-character buffer.
///
/// `buf_len` is the number of `u16` elements available in the buffer.
///
/// Returns the total encoded length in `u16` code units (excluding the null terminator)
/// and whether the output was truncated.
///
/// The output is always null-terminated with a `0u16`.
///
/// # Safety
///
/// `buf` must point to a writable buffer of at least `buf_len` `u16` elements, or be null.
pub unsafe fn encode_wchar(
    s: &str,
    buf: *mut u16,
    buf_len: usize,
) -> Result<EncodedResult, EncodingError> {
    let utf16: Vec<u16> = s.encode_utf16().collect();
    let total_len = utf16.len();

    if buf.is_null() || buf_len == 0 {
        return Ok(EncodedResult {
            total_len,
            written_len: 0,
            truncated: total_len > 0,
        });
    }

    let available = buf_len - 1; // reserve space for null terminator
    let mut write_len = std::cmp::min(total_len, available);

    // Don't split a surrogate pair
    if write_len > 0 && write_len < total_len {
        let last = utf16[write_len - 1];
        if (0xD800..=0xDBFF).contains(&last) {
            write_len -= 1;
        }
    }

    unsafe {
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), buf, write_len);
        *buf.add(write_len) = 0;
    }

    Ok(EncodedResult {
        total_len,
        written_len: write_len,
        truncated: total_len > available,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedResult {
    /// Total encoded length (bytes for char, code units for wchar), excluding null terminator.
    pub total_len: usize,
    /// Number of units actually written (excluding null terminator).
    pub written_len: usize,
    /// Whether the output was truncated due to insufficient buffer space.
    pub truncated: bool,
}

#[derive(Debug, Snafu, ErrorTrace)]
pub enum EncodingError {
    #[snafu(display("Null pointer"))]
    NullPointer {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid length: {length}"))]
    InvalidLength {
        length: i32,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid UTF-8: {source}"))]
    InvalidUtf8 {
        source: std::string::FromUtf8Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid UTF-16: {source}"))]
    InvalidUtf16 {
        source: std::string::FromUtf16Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Decode failed for encoding: {encoding_name}"))]
    DecodeFailed {
        encoding_name: String,
        #[snafu(implicit)]
        location: Location,
    },
}

const SQL_NTS: i32 = -3;

/// Extract a byte slice from a narrow C string pointer with ODBC length semantics.
///
/// # Safety
///
/// `ptr` must point to a valid, readable byte buffer of at least `length` bytes
/// (or be null-terminated if `length` is `SQL_NTS`). The buffer must remain valid
/// for the lifetime `'a`.
unsafe fn char_ptr_to_slice<'a>(ptr: *const u8, length: i32) -> Result<&'a [u8], EncodingError> {
    if length == SQL_NTS {
        let mut len = 0usize;
        unsafe {
            while *ptr.add(len) != 0 {
                len += 1;
            }
            Ok(std::slice::from_raw_parts(ptr, len))
        }
    } else if length >= 0 {
        Ok(unsafe { std::slice::from_raw_parts(ptr, length as usize) })
    } else {
        InvalidLengthSnafu { length }.fail()
    }
}

/// Extract a `u16` slice from a wide C string pointer with ODBC length semantics.
///
/// # Safety
///
/// `ptr` must point to a valid, readable buffer of at least `length` `u16` elements
/// (or be null-terminated if `length` is `SQL_NTS`). The buffer must remain valid
/// for the lifetime `'a`.
unsafe fn wchar_ptr_to_slice<'a>(ptr: *const u16, length: i32) -> Result<&'a [u16], EncodingError> {
    if length == SQL_NTS {
        let mut len = 0usize;
        unsafe {
            while *ptr.add(len) != 0 {
                len += 1;
            }
            Ok(std::slice::from_raw_parts(ptr, len))
        }
    } else if length >= 0 {
        Ok(unsafe { std::slice::from_raw_parts(ptr, length as usize) })
    } else {
        InvalidLengthSnafu { length }.fail()
    }
}

/// Find the last safe boundary in a UTF-8 byte slice that doesn't exceed `max`.
fn find_utf8_boundary(bytes: &[u8], max: usize) -> usize {
    if max >= bytes.len() {
        return bytes.len();
    }
    let mut pos = max;
    while pos > 0 && (bytes[pos] & 0xC0) == 0x80 {
        pos -= 1;
    }
    pos
}

fn floor_char_boundary(s: &str, i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    let mut pos = i;
    while !s.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

/// Binary search for the largest prefix of `input` (at a char boundary) whose
/// encoded form fits within `max_encoded_bytes`.
fn find_encode_fit(input: &str, max_encoded_bytes: usize, encoding: &'static Encoding) -> usize {
    if input.is_empty() || max_encoded_bytes == 0 {
        return 0;
    }

    let mut lo = 1usize;
    let mut hi = input.len();
    let mut best = 0usize;
    let mut last_tested = 0usize;

    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        let pos = floor_char_boundary(input, mid);

        if pos <= last_tested {
            lo = mid + 1;
            continue;
        }
        last_tested = pos;

        let encoded_len = encoding.encode(&input[..pos]).0.len();
        if encoded_len <= max_encoded_bytes {
            best = pos;
            if pos >= input.len() {
                break;
            }
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- locale_encoding ----------

    #[test]
    fn locale_encoding_returns_valid_encoding() {
        let enc = locale_encoding();
        assert!(!enc.name().is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn locale_encoding_is_utf8_on_macos() {
        assert_eq!(locale_encoding(), encoding_rs::UTF_8);
    }

    // ---------- decode_char ----------

    #[test]
    fn decode_char_null_terminated() {
        let input = b"hello\0";
        let result = unsafe { decode_char(input.as_ptr(), SQL_NTS) }.unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn decode_char_with_explicit_length() {
        let input = b"hello world";
        let result = unsafe { decode_char(input.as_ptr(), 5) }.unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn decode_char_empty_string_nts() {
        let input = b"\0";
        let result = unsafe { decode_char(input.as_ptr(), SQL_NTS) }.unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn decode_char_empty_string_zero_length() {
        let input = b"anything";
        let result = unsafe { decode_char(input.as_ptr(), 0) }.unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn decode_char_null_pointer() {
        let result = unsafe { decode_char(std::ptr::null(), 5) };
        assert!(matches!(result, Err(EncodingError::NullPointer { .. })));
    }

    #[test]
    fn decode_char_negative_length() {
        let input = b"hello\0";
        let result = unsafe { decode_char(input.as_ptr(), -5) };
        assert!(matches!(
            result,
            Err(EncodingError::InvalidLength { length: -5, .. })
        ));
    }

    #[test]
    fn decode_char_multibyte_utf8() {
        let input = "café\0";
        let result = unsafe { decode_char(input.as_ptr(), SQL_NTS) }.unwrap();
        assert_eq!(result, "café");
    }

    #[test]
    fn decode_char_multibyte_utf8_explicit_len() {
        let input = "日本語";
        let bytes = input.as_bytes();
        let result = unsafe { decode_char(bytes.as_ptr(), bytes.len() as i32) }.unwrap();
        assert_eq!(result, "日本語");
    }

    // ---------- encode_char ----------

    #[test]
    fn encode_char_simple_ascii() {
        let mut buf = vec![0xFFu8; 16];
        let result = unsafe { encode_char("hello", buf.as_mut_ptr(), buf.len()) }.unwrap();
        assert_eq!(result.total_len, 5);
        assert_eq!(result.written_len, 5);
        assert!(!result.truncated);
        assert_eq!(&buf[..6], b"hello\0");
    }

    #[test]
    fn encode_char_truncation() {
        let mut buf = vec![0xFFu8; 4]; // room for 3 chars + null
        let result = unsafe { encode_char("hello", buf.as_mut_ptr(), buf.len()) }.unwrap();
        assert_eq!(result.total_len, 5);
        assert_eq!(result.written_len, 3);
        assert!(result.truncated);
        assert_eq!(buf[3], 0);
    }

    #[test]
    fn encode_char_empty_string() {
        let mut buf = vec![0xFFu8; 4];
        let result = unsafe { encode_char("", buf.as_mut_ptr(), buf.len()) }.unwrap();
        assert_eq!(result.total_len, 0);
        assert_eq!(result.written_len, 0);
        assert!(!result.truncated);
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn encode_char_null_buffer() {
        let result = unsafe { encode_char("hello", std::ptr::null_mut(), 0) }.unwrap();
        assert_eq!(result.total_len, 5);
        assert_eq!(result.written_len, 0);
        assert!(result.truncated);
    }

    #[test]
    fn encode_char_multibyte_utf8() {
        let mut buf = vec![0xFFu8; 32];
        let result = unsafe { encode_char("café", buf.as_mut_ptr(), buf.len()) }.unwrap();
        // "café" in UTF-8 is 5 bytes (é = 2 bytes)
        assert_eq!(result.total_len, 5);
        assert_eq!(result.written_len, 5);
        assert!(!result.truncated);
    }

    #[test]
    fn encode_char_truncation_at_utf8_boundary() {
        let mut buf = vec![0xFFu8; 5]; // room for 4 bytes + null
        // "café" = [63, 61, 66, c3, a9] — 5 bytes
        let result = unsafe { encode_char("café", buf.as_mut_ptr(), buf.len()) }.unwrap();
        assert!(result.truncated);
        // Should not split the 2-byte é sequence
        assert_eq!(result.written_len, 3); // "caf" only
        assert_eq!(buf[result.written_len], 0);
    }

    // ---------- decode_wchar ----------

    #[test]
    fn decode_wchar_null_terminated() {
        let input: Vec<u16> = "hello".encode_utf16().chain(std::iter::once(0)).collect();
        let result = unsafe { decode_wchar(input.as_ptr(), SQL_NTS) }.unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn decode_wchar_with_explicit_length() {
        let input: Vec<u16> = "hello world".encode_utf16().collect();
        let result = unsafe { decode_wchar(input.as_ptr(), 5) }.unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn decode_wchar_empty_nts() {
        let input: Vec<u16> = vec![0];
        let result = unsafe { decode_wchar(input.as_ptr(), SQL_NTS) }.unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn decode_wchar_empty_zero_length() {
        let input: Vec<u16> = vec![0x1234];
        let result = unsafe { decode_wchar(input.as_ptr(), 0) }.unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn decode_wchar_null_pointer() {
        let result = unsafe { decode_wchar(std::ptr::null(), 5) };
        assert!(matches!(result, Err(EncodingError::NullPointer { .. })));
    }

    #[test]
    fn decode_wchar_negative_length() {
        let input: Vec<u16> = vec![0x0041, 0];
        let result = unsafe { decode_wchar(input.as_ptr(), -5) };
        assert!(matches!(
            result,
            Err(EncodingError::InvalidLength { length: -5, .. })
        ));
    }

    #[test]
    fn decode_wchar_cjk_characters() {
        let input_str = "日本語";
        let input: Vec<u16> = input_str.encode_utf16().chain(std::iter::once(0)).collect();
        let result = unsafe { decode_wchar(input.as_ptr(), SQL_NTS) }.unwrap();
        assert_eq!(result, "日本語");
    }

    #[test]
    fn decode_wchar_emoji_surrogate_pairs() {
        let input_str = "👋🌍";
        let input: Vec<u16> = input_str.encode_utf16().chain(std::iter::once(0)).collect();
        let result = unsafe { decode_wchar(input.as_ptr(), SQL_NTS) }.unwrap();
        assert_eq!(result, "👋🌍");
    }

    // ---------- encode_wchar ----------

    #[test]
    fn encode_wchar_simple_ascii() {
        let mut buf = vec![0xFFFFu16; 16];
        let result = unsafe { encode_wchar("hello", buf.as_mut_ptr(), buf.len()) }.unwrap();
        assert_eq!(result.total_len, 5);
        assert_eq!(result.written_len, 5);
        assert!(!result.truncated);
        let expected: Vec<u16> = "hello".encode_utf16().chain(std::iter::once(0)).collect();
        assert_eq!(&buf[..6], &expected[..]);
    }

    #[test]
    fn encode_wchar_truncation() {
        let mut buf = vec![0xFFFFu16; 4]; // room for 3 code units + null
        let result = unsafe { encode_wchar("hello", buf.as_mut_ptr(), buf.len()) }.unwrap();
        assert_eq!(result.total_len, 5);
        assert_eq!(result.written_len, 3);
        assert!(result.truncated);
        assert_eq!(buf[3], 0);
    }

    #[test]
    fn encode_wchar_empty_string() {
        let mut buf = vec![0xFFFFu16; 4];
        let result = unsafe { encode_wchar("", buf.as_mut_ptr(), buf.len()) }.unwrap();
        assert_eq!(result.total_len, 0);
        assert_eq!(result.written_len, 0);
        assert!(!result.truncated);
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn encode_wchar_null_buffer() {
        let result = unsafe { encode_wchar("hello", std::ptr::null_mut(), 0) }.unwrap();
        assert_eq!(result.total_len, 5);
        assert_eq!(result.written_len, 0);
        assert!(result.truncated);
    }

    #[test]
    fn encode_wchar_surrogate_pair_not_split() {
        // "👋" = U+1F44B, encoded as two u16 surrogate pair elements
        let emoji = "👋";
        let utf16_len = emoji.encode_utf16().count(); // 2
        assert_eq!(utf16_len, 2);

        // Buffer has room for 2 code units + null = 3 total
        let mut buf = vec![0u16; 3];
        let result = unsafe { encode_wchar(emoji, buf.as_mut_ptr(), buf.len()) }.unwrap();
        assert_eq!(result.total_len, 2);
        assert_eq!(result.written_len, 2);
        assert!(!result.truncated);

        // Buffer has room for only 1 code unit + null = 2 total
        // Should skip the high surrogate to avoid splitting the pair
        let mut buf = vec![0u16; 2];
        let result = unsafe { encode_wchar(emoji, buf.as_mut_ptr(), buf.len()) }.unwrap();
        assert_eq!(result.total_len, 2);
        assert_eq!(result.written_len, 0); // can't fit the pair, so write nothing
        assert!(result.truncated);
        assert_eq!(buf[0], 0); // null terminator only
    }

    // ---------- round-trip tests ----------

    #[test]
    fn char_round_trip_ascii() {
        let original = "SELECT * FROM table1";
        let mut buf = vec![0u8; 64];
        let enc = unsafe { encode_char(original, buf.as_mut_ptr(), buf.len()) }.unwrap();
        let decoded = unsafe { decode_char(buf.as_ptr(), enc.written_len as i32) }.unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn char_round_trip_utf8_multibyte() {
        let original = "Ñoño Ünïcödé";
        let mut buf = vec![0u8; 64];
        let enc = unsafe { encode_char(original, buf.as_mut_ptr(), buf.len()) }.unwrap();
        let decoded = unsafe { decode_char(buf.as_ptr(), enc.written_len as i32) }.unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn wchar_round_trip_ascii() {
        let original = "SELECT * FROM table1";
        let mut buf = vec![0u16; 64];
        let enc = unsafe { encode_wchar(original, buf.as_mut_ptr(), buf.len()) }.unwrap();
        let decoded = unsafe { decode_wchar(buf.as_ptr(), enc.written_len as i32) }.unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn wchar_round_trip_multibyte() {
        let original = "日本語テスト 🌍";
        let mut buf = vec![0u16; 64];
        let enc = unsafe { encode_wchar(original, buf.as_mut_ptr(), buf.len()) }.unwrap();
        let decoded = unsafe { decode_wchar(buf.as_ptr(), enc.written_len as i32) }.unwrap();
        assert_eq!(decoded, original);
    }

    // ---------- edge cases ----------

    #[test]
    fn decode_char_sql_nts_with_interior_data() {
        // NTS should stop at first null byte
        let input = b"abc\0xyz\0";
        let result = unsafe { decode_char(input.as_ptr(), SQL_NTS) }.unwrap();
        assert_eq!(result, "abc");
    }

    #[test]
    fn decode_wchar_sql_nts_with_interior_data() {
        let input: Vec<u16> = vec![0x0041, 0x0042, 0x0000, 0x0043, 0x0000];
        let result = unsafe { decode_wchar(input.as_ptr(), SQL_NTS) }.unwrap();
        assert_eq!(result, "AB");
    }

    #[test]
    fn encode_char_exact_fit() {
        // "abc" = 3 bytes, buffer of 4 = 3 + null — exact fit
        let mut buf = vec![0xFFu8; 4];
        let result = unsafe { encode_char("abc", buf.as_mut_ptr(), 4) }.unwrap();
        assert_eq!(result.total_len, 3);
        assert_eq!(result.written_len, 3);
        assert!(!result.truncated);
        assert_eq!(&buf[..4], b"abc\0");
    }

    #[test]
    fn encode_wchar_exact_fit() {
        let mut buf = vec![0xFFFFu16; 4]; // room for 3 code units + null
        let result = unsafe { encode_wchar("abc", buf.as_mut_ptr(), 4) }.unwrap();
        assert_eq!(result.total_len, 3);
        assert_eq!(result.written_len, 3);
        assert!(!result.truncated);
        assert_eq!(buf[3], 0);
    }

    #[test]
    fn encode_char_buffer_size_one() {
        // Buffer of 1 byte can only hold the null terminator
        let mut buf = vec![0xFFu8; 1];
        let result = unsafe { encode_char("a", buf.as_mut_ptr(), 1) }.unwrap();
        assert_eq!(result.total_len, 1);
        assert_eq!(result.written_len, 0);
        assert!(result.truncated);
        assert_eq!(buf[0], 0);
    }
}
