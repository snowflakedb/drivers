use crate::api::OdbcResult;
use crate::api::error::{
    InvalidBufferLengthSnafu, InvalidWideCharSnafu, NullPointerSnafu, TextConversionFromUtf8Snafu,
    TextConversionFromUtf16Snafu, TextConversionUtf8Snafu,
};
use crate::conversion::warning::{Warning, Warnings};
use odbc_sys as sql;
use snafu::ResultExt;
use std::cmp::min;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

// ---------------------------------------------------------------------------
// At the C ABI level we declare every wide-string entry point as taking a
// `*mut u16`.
//
// The choice between the two interpretations is configured by the user via
// the `DriverManagerEncoding` key in `sf.odbc.ini`:
//
//   DriverManagerEncoding=UTF-16   ; default; matches unixODBC
//   DriverManagerEncoding=UTF-32   ; required for iODBC on UNIX
//
// The first wide buffer the driver sees is also inspected for its byte
// pattern as a sanity check; if the bytes don't match the configured
// encoding a warning is logged once with a pointer to the INI key the
// user should update. Auto-detection never changes the configured
// encoding - it only complains.
// ---------------------------------------------------------------------------

pub type WideChar = u16;

/// Compile-time size of one [`WideChar`] in bytes (always 2). Most call
/// sites outside of `encoding.rs` should use [`wchar_byte_size`] instead,
/// which returns the *runtime* size of one DM-side `SQLWCHAR` (2 for
/// UTF-16, 4 for UTF-32). Currently consumed only by test code which builds
/// `[u16; N]` buffers with hard-coded UTF-16 expectations.
#[allow(dead_code)] // used only from `#[cfg(test)]` modules
pub const WIDE_CHAR_SIZE: usize = std::mem::size_of::<WideChar>();

/// `sf.odbc.ini` key that selects the encoding interpretation.
pub const DRIVER_MANAGER_ENCODING_KEY: &str = "DriverManagerEncoding";

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WCharEncoding {
    Utf16,
    Utf32,
}

impl WCharEncoding {
    pub fn as_ini_value(self) -> &'static str {
        match self {
            WCharEncoding::Utf16 => "UTF-16",
            WCharEncoding::Utf32 => "UTF-32",
        }
    }

    #[inline]
    pub const fn byte_size(self) -> usize {
        match self {
            WCharEncoding::Utf16 => 2,
            WCharEncoding::Utf32 => 4,
        }
    }
}

impl std::fmt::Display for WCharEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ini_value())
    }
}

/// Process-wide negotiated encoding. Seeded once by
/// [`negotiate_from_config`] from the ODBC wrapper's startup path
static WCHAR_ENCODING: OnceLock<WCharEncoding> = OnceLock::new();

static MISMATCH_WARNED: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub struct AlreadyInitialisedError;

pub fn init_wchar_encoding(enc: WCharEncoding) -> Result<(), AlreadyInitialisedError> {
    WCHAR_ENCODING.set(enc).map_err(|_| AlreadyInitialisedError)
}

pub fn negotiate_from_config() {
    let enc = sf_core::config::get_ini_config()
        .and_then(|ini| ini.get(DRIVER_MANAGER_ENCODING_KEY))
        .and_then(parse_wchar_encoding_value)
        .unwrap_or(WCharEncoding::Utf16);
    let _ = init_wchar_encoding(enc);
}

pub fn parse_wchar_encoding_value(s: &str) -> Option<WCharEncoding> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("utf-16") || s.eq_ignore_ascii_case("utf16") {
        Some(WCharEncoding::Utf16)
    } else if s.eq_ignore_ascii_case("utf-32") || s.eq_ignore_ascii_case("utf32") {
        Some(WCharEncoding::Utf32)
    } else {
        None
    }
}

#[inline]
pub fn current_wchar_encoding() -> WCharEncoding {
    WCHAR_ENCODING
        .get()
        .copied()
        .unwrap_or(WCharEncoding::Utf16)
}

#[inline]
pub fn wchar_byte_size() -> usize {
    current_wchar_encoding().byte_size()
}

/// Inspect the leading bytes of a wide-string buffer the driver manager
/// has just handed us, and **warn once** if its byte pattern disagrees
/// with the encoding the user configured.
///
/// This is purely diagnostic: it never changes the global encoding.
///
/// ODBC W-API string inputs always start with an ASCII keyword (`DRIVER=`,
/// `DSN=`, `UID=`, …). For an ASCII-keyword string of two or more
/// characters the leading 8 bytes look like:
///
///   * UTF-16 LE: `XX 00 YY 00 ZZ 00 …`     (every other byte non-zero)
///   * UTF-32 LE: `XX 00 00 00 YY 00 00 00` (one non-zero per 4-byte stride)
///
/// We require at least 4 `WideChar` slots (= 8 bytes) of explicit-length
/// data before evaluating, because a single-char-plus-null UTF-16 buffer
/// (`XX 00 00 00`) is byte-identical to a single-char UTF-32 buffer in the
/// first 4 bytes. No warning is ever issued for ambiguous inputs.
pub fn detect_wchar_encoding_from_bytes(ptr: *const WideChar, length: sql::Integer) {
    if MISMATCH_WARNED.load(Ordering::Relaxed) {
        return;
    }
    let Some(detected) = inspect_wchar_byte_pattern(ptr, length) else {
        return;
    };
    let configured = current_wchar_encoding();
    if detected == configured {
        return;
    }
    if MISMATCH_WARNED
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        tracing::warn!(
            configured = %configured,
            detected = %detected,
            "Wide-character buffer byte pattern looks like {detected}, but the driver \
             is configured for {configured}. If decoding errors follow, set \
             `{DRIVER_MANAGER_ENCODING_KEY}={detected}` in sf.odbc.ini and restart \
             the driver."
        );
    }
}

/// Best-effort byte-pattern inference. Returns `None` for ambiguous or
/// too-short inputs.
fn inspect_wchar_byte_pattern(ptr: *const WideChar, length: sql::Integer) -> Option<WCharEncoding> {
    if ptr.is_null() {
        return None;
    }
    if length == sql::NTS as i32 || length < 4 {
        return None;
    }
    // Safety: the caller has at least 4 `WideChar` slots (= 8 bytes)
    // regardless of which DM-side encoding is in effect.
    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, 8) };
    if bytes[0] == 0 {
        // Leading zero byte is unusual for an ASCII keyword and gives no
        // signal either way.
        return None;
    }
    let bytes_1_to_3_zero = bytes[1] == 0 && bytes[2] == 0 && bytes[3] == 0;
    if !bytes_1_to_3_zero {
        // Non-zero byte inside the first 4 — typical of a UTF-16 ASCII
        // string with two or more characters (`XX 00 YY 00`).
        return Some(WCharEncoding::Utf16);
    }
    // Bytes [0..4] match `XX 00 00 00`. Could be UTF-32 (one char) or
    // UTF-16 (one char + null + extra padding). The next 4 bytes settle
    // it: a second `YY 00 00 00` stride confirms UTF-32; anything else
    // (including all-zero) is ambiguous, so we don't infer.
    let bytes_5_to_7_zero = bytes[5] == 0 && bytes[6] == 0 && bytes[7] == 0;
    if bytes[4] != 0 && bytes_5_to_7_zero {
        Some(WCharEncoding::Utf32)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Format-specific helpers (UTF-16 only)
//
// These do not consult the runtime encoding; they always produce / consume
// UTF-16 data in `u16` slots. They are used by tests (which set up
// `[u16; N]` buffers with hard-coded UTF-16 expectations). Production code
// should use the runtime-aware helpers further down so the same call works
// under either DM.
// ---------------------------------------------------------------------------

/// Encode `s` as UTF-16 code units. Always UTF-16, regardless of runtime
/// encoding.
#[allow(dead_code)] // used only from `#[cfg(test)]` modules
#[inline]
pub fn encode_wide(s: &str) -> Vec<WideChar> {
    s.encode_utf16().collect()
}

/// Decode a UTF-16 slice into a `String`. Always UTF-16, regardless of
/// runtime encoding.
#[allow(dead_code)] // used only from `#[cfg(test)]` modules
pub fn decode_wide(units: &[WideChar]) -> OdbcResult<String> {
    String::from_utf16(units).context(TextConversionFromUtf16Snafu {})
}

// ---------------------------------------------------------------------------
// Encoding-aware helpers — explicit-encoding form.
//
// The `*_in` variants take a `WCharEncoding` directly and are pure with
// respect to global state. Tests that need to exercise both encodings in
// the same process call these directly. The convenience wrappers below
// route through `current_wchar_encoding()` for production callers.
//
// All counts and offsets they operate on are in **DM-side code units**
// (one `u16` in UTF-16 mode; one `u32` in UTF-32 mode).
// ---------------------------------------------------------------------------

/// Number of DM-side `SQLWCHAR` code units required to encode `s` under
/// `enc`.
#[inline]
pub fn wide_unit_len_in(s: &str, enc: WCharEncoding) -> usize {
    match enc {
        WCharEncoding::Utf16 => s.encode_utf16().count(),
        WCharEncoding::Utf32 => s.chars().count(),
    }
}

/// Number of DM-side `SQLWCHAR` code units required to encode `s` under
/// the negotiated runtime encoding.
#[inline]
pub fn wide_unit_len(s: &str) -> usize {
    wide_unit_len_in(s, current_wchar_encoding())
}

/// Encode `s` and write up to `max_units` DM-side code units of `enc`
/// into `buf`, starting from `offset_units` in `s`. Returns the number of
/// DM-side units actually written (may be less than `max_units` if the
/// source runs out first).
///
/// Does **not** write a null terminator; pair with [`write_wide_null_in`]
/// when one is needed.
///
/// # Safety
/// `buf` must point to a writable buffer of at least
/// `max_units * enc.byte_size()` bytes that remains valid for the
/// duration of the call.
pub unsafe fn write_wide_buffer_in(
    s: &str,
    buf: *mut WideChar,
    max_units: usize,
    offset_units: usize,
    enc: WCharEncoding,
) -> usize {
    if max_units == 0 {
        return 0;
    }
    match enc {
        WCharEncoding::Utf16 => {
            let mut written = 0;
            for u in s.encode_utf16().skip(offset_units).take(max_units) {
                unsafe { std::ptr::write(buf.add(written), u) };
                written += 1;
            }
            written
        }
        WCharEncoding::Utf32 => {
            let buf32 = buf as *mut u32;
            let mut written = 0;
            for c in s.chars().skip(offset_units).take(max_units) {
                unsafe { std::ptr::write(buf32.add(written), c as u32) };
                written += 1;
            }
            written
        }
    }
}

/// Default-encoding form of [`write_wide_buffer_in`].
///
/// # Safety
/// See [`write_wide_buffer_in`].
#[inline]
pub unsafe fn write_wide_buffer(
    s: &str,
    buf: *mut WideChar,
    max_units: usize,
    offset_units: usize,
) -> usize {
    unsafe { write_wide_buffer_in(s, buf, max_units, offset_units, current_wchar_encoding()) }
}

/// Write a single DM-side null terminator at offset `pos` from `buf`
/// under `enc`.
///
/// # Safety
/// `buf.add(pos)` (UTF-16) or `(buf as *mut u32).add(pos)` (UTF-32) must
/// be a valid writable address.
pub unsafe fn write_wide_null_in(buf: *mut WideChar, pos: usize, enc: WCharEncoding) {
    match enc {
        WCharEncoding::Utf16 => unsafe { std::ptr::write(buf.add(pos), 0) },
        WCharEncoding::Utf32 => unsafe {
            std::ptr::write((buf as *mut u32).add(pos), 0);
        },
    }
}

/// Default-encoding form of [`write_wide_null_in`].
///
/// # Safety
/// See [`write_wide_null_in`].
#[inline]
pub unsafe fn write_wide_null(buf: *mut WideChar, pos: usize) {
    unsafe { write_wide_null_in(buf, pos, current_wchar_encoding()) }
}

/// Length, in DM-side code units, of a null-terminated wide-string buffer
/// under `enc`. Bounded by `max_units` to prevent runaway scans.
///
/// # Safety
/// `ptr` must be valid for reads of at least `max_units * enc.byte_size()`
/// bytes (or unbounded if `max_units == usize::MAX` and the caller has
/// guaranteed a null terminator exists).
pub unsafe fn wide_strlen_bounded_in(
    ptr: *const WideChar,
    max_units: usize,
    enc: WCharEncoding,
) -> usize {
    match enc {
        WCharEncoding::Utf16 => {
            let mut i = 0;
            unsafe {
                while i < max_units && *ptr.add(i) != 0 {
                    i += 1;
                }
            }
            i
        }
        WCharEncoding::Utf32 => {
            let p32 = ptr as *const u32;
            let mut i = 0;
            unsafe {
                while i < max_units && *p32.add(i) != 0 {
                    i += 1;
                }
            }
            i
        }
    }
}

/// Default-encoding form of [`wide_strlen_bounded_in`].
///
/// # Safety
/// See [`wide_strlen_bounded_in`].
#[inline]
pub unsafe fn wide_strlen_bounded(ptr: *const WideChar, max_units: usize) -> usize {
    unsafe { wide_strlen_bounded_in(ptr, max_units, current_wchar_encoding()) }
}

/// Decode a DM-side wide buffer of `length` code units (or until the
/// first null when `length == SQL_NTS`) into a Rust `String` under `enc`.
///
/// # Safety
/// `ptr` must be valid for reads of either `length * enc.byte_size()`
/// bytes (explicit length) or up to the first null terminator (SQL_NTS).
pub unsafe fn read_wide_string_in(
    ptr: *const WideChar,
    length: sql::Integer,
    enc: WCharEncoding,
) -> OdbcResult<String> {
    if ptr.is_null() {
        return NullPointerSnafu.fail();
    }
    // Matches the legacy Narrow/Wide read_string contract: explicit
    // non-positive lengths are rejected; only [`sql::NTS`] is accepted as
    // a special sentinel for "null-terminated".
    if length != sql::NTS as i32 && length <= 0 {
        return InvalidBufferLengthSnafu {
            length: length as i64,
        }
        .fail();
    }
    match enc {
        WCharEncoding::Utf16 => {
            let slice = if length == sql::NTS as i32 {
                let mut len = 0;
                unsafe {
                    while *ptr.add(len) != 0 {
                        len += 1;
                    }
                    std::slice::from_raw_parts(ptr, len)
                }
            } else {
                unsafe { std::slice::from_raw_parts(ptr, length as usize) }
            };
            String::from_utf16(slice).context(TextConversionFromUtf16Snafu {})
        }
        WCharEncoding::Utf32 => {
            let p32 = ptr as *const u32;
            let units = if length == sql::NTS as i32 {
                let mut len = 0;
                unsafe {
                    while *p32.add(len) != 0 {
                        len += 1;
                    }
                    std::slice::from_raw_parts(p32, len)
                }
            } else {
                unsafe { std::slice::from_raw_parts(p32, length as usize) }
            };
            let mut out = String::with_capacity(units.len());
            for &cp in units {
                match char::from_u32(cp) {
                    Some(c) => out.push(c),
                    None => return InvalidWideCharSnafu { code_point: cp }.fail(),
                }
            }
            Ok(out)
        }
    }
}

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

/// Replace each non-ASCII character with `\x1a` (SUB control character).
///
/// Returns a borrow for pure-ASCII input to avoid a per-call heap
/// allocation on the `write_char_string` hot path. ODBC ANSI output is
/// ASCII-only in C/POSIX locales, and in practice the vast majority of
/// values that flow through here (numbers, dates, times, English text)
/// are already ASCII.
pub fn mask_non_ascii_characters(src: &str) -> std::borrow::Cow<'_, str> {
    if src.is_ascii() {
        return std::borrow::Cow::Borrowed(src);
    }
    std::borrow::Cow::Owned(
        src.chars()
            .map(|c| if !c.is_ascii() { '\x1a' } else { c })
            .collect(),
    )
}

/// Abstracts over ANSI (narrow) and Unicode (wide) ODBC string operations,
/// allowing API-layer functions to be written once as generics.
pub trait OdbcEncoding {
    type Char;

    /// Effective byte size of one DM-side code unit (1 for narrow, 2 or 4
    /// for wide depending on the runtime [`WCharEncoding`]). Use this
    /// rather than `size_of::<Char>()` when converting between byte counts
    /// and unit counts on the wire.
    fn effective_char_size() -> usize;

    /// Read a string from an ODBC input buffer. `length` is in DM-side
    /// code units, or [`sql::NTS`] for null-terminated.
    fn read_string(text: *const Self::Char, length: sql::Integer) -> OdbcResult<String>;

    /// Core write: copy a Rust string into an ODBC output buffer.
    ///
    /// `buffer_length` is in **DM-side code units** (bytes for narrow, one
    /// `SQLWCHAR` for wide — 2 bytes in UTF-16 mode, 4 bytes in UTF-32
    /// mode), including space for the null terminator.
    ///
    /// Returns `(full_untruncated_length_in_dm_units, was_truncated)`.
    fn write_string(string: &str, buffer: *mut Self::Char, buffer_length: usize) -> (usize, bool);
}

/// Marker type for ANSI (narrow, `sql::Char` / `u8`) encoding.
pub struct Narrow;

/// Marker type for Unicode (wide) encoding. The C ABI element type is
/// always `u16` ([`WideChar`]); whether the bytes inside a buffer are
/// interpreted as UTF-16 or UTF-32 is decided at runtime via
/// [`WCharEncoding`].
pub struct Wide;

impl OdbcEncoding for Narrow {
    type Char = sql::Char;

    fn effective_char_size() -> usize {
        1
    }

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
    type Char = WideChar;

    #[inline]
    fn effective_char_size() -> usize {
        wchar_byte_size()
    }

    fn read_string(text: *const Self::Char, length: sql::Integer) -> OdbcResult<String> {
        // Auto-detect the DM-side encoding before we slice. Once any wide
        // input has been seen the warning state is pinned for the rest
        // of the process; everything below dispatches on the configured
        // encoding.
        detect_wchar_encoding_from_bytes(text, length);
        unsafe { read_wide_string_in(text, length, current_wchar_encoding()) }
    }

    fn write_string(string: &str, buffer: *mut Self::Char, buffer_length: usize) -> (usize, bool) {
        let enc = current_wchar_encoding();
        let full_len = wide_unit_len_in(string, enc);
        if buffer.is_null() {
            return (full_len, false);
        }
        if buffer_length == 0 {
            return (full_len, full_len > 0);
        }
        let max_chars = buffer_length.saturating_sub(1);
        let written = unsafe { write_wide_buffer_in(string, buffer, max_chars, 0, enc) };
        unsafe { write_wide_null_in(buffer, written, enc) };
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
    let char_size = E::effective_char_size() as sql::Integer;
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
/// **DM-side code units** (characters) as `sql::SmallInt`.
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
/// For Narrow this is identical to `write_string_chars` (1 byte = 1
/// encoding unit). For Wide the byte buffer length is divided by the
/// runtime DM-side `SQLWCHAR` size to obtain code units, and the reported
/// length is multiplied back.
///
/// Used by: `SQLGetDiagField`, `SQLGetInfo`.
pub fn write_string_bytes<E: OdbcEncoding>(
    string: &str,
    buffer: *mut E::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    warnings: Option<&mut Warnings>,
) {
    let char_size = E::effective_char_size();
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
    let char_size = E::effective_char_size();
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

    // ---------- mask_non_ascii_characters ---------------------------------

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

    // ---------- parse_wchar_encoding_value --------------------------------

    #[test]
    fn parse_wchar_encoding_value_accepts_canonical_forms() {
        for s in ["UTF-16", "utf-16", "UTF16", "utf16", " UTF-16 "] {
            assert_eq!(parse_wchar_encoding_value(s), Some(WCharEncoding::Utf16));
        }
        for s in ["UTF-32", "utf-32", "UTF32", "utf32", " UTF-32 "] {
            assert_eq!(parse_wchar_encoding_value(s), Some(WCharEncoding::Utf32));
        }
    }

    #[test]
    fn parse_wchar_encoding_value_rejects_garbage() {
        for s in ["", "ascii", "utf-8", "wchar"] {
            assert_eq!(parse_wchar_encoding_value(s), None);
        }
    }

    #[test]
    fn wchar_encoding_display_uses_canonical_ini_form() {
        assert_eq!(format!("{}", WCharEncoding::Utf16), "UTF-16");
        assert_eq!(format!("{}", WCharEncoding::Utf32), "UTF-32");
        assert_eq!(WCharEncoding::Utf16.as_ini_value(), "UTF-16");
        assert_eq!(WCharEncoding::Utf32.as_ini_value(), "UTF-32");
    }

    #[test]
    fn wchar_byte_size_matches_encoding() {
        assert_eq!(WCharEncoding::Utf16.byte_size(), 2);
        assert_eq!(WCharEncoding::Utf32.byte_size(), 4);
    }

    // ---------- inspect_wchar_byte_pattern --------------------------------

    /// Single-char-plus-padding buffers (`XX 00 00 00 00 00 00 00`) are
    /// genuinely ambiguous; the inspector must report `None` rather than
    /// guessing one way or the other.
    #[test]
    fn inspect_returns_none_for_ambiguous_one_char_pattern() {
        #[repr(align(4))]
        struct Aligned([u8; 8]);
        let src = Aligned([0x44, 0, 0, 0, 0, 0, 0, 0]);
        let detected = inspect_wchar_byte_pattern(src.0.as_ptr() as *const u16, 4);
        assert_eq!(detected, None);
    }

    /// `"DR" → 44 00 00 00 52 00 00 00` — UTF-32 with two ASCII chars.
    #[test]
    fn inspect_recognises_utf32_two_char_ascii_pattern() {
        #[repr(align(4))]
        struct Aligned([u8; 8]);
        let src = Aligned([0x44, 0, 0, 0, 0x52, 0, 0, 0]);
        let detected = inspect_wchar_byte_pattern(src.0.as_ptr() as *const u16, 4);
        assert_eq!(detected, Some(WCharEncoding::Utf32));
    }

    /// `"DR" → 44 00 52 00 ...` — UTF-16 with two ASCII chars.
    #[test]
    fn inspect_recognises_utf16_two_char_ascii_pattern() {
        #[repr(align(2))]
        struct Aligned([u8; 8]);
        let src = Aligned([0x44, 0, 0x52, 0, 0x49, 0, 0x56, 0]);
        let detected = inspect_wchar_byte_pattern(src.0.as_ptr() as *const u16, 4);
        assert_eq!(detected, Some(WCharEncoding::Utf16));
    }

    #[test]
    fn inspect_returns_none_for_nts() {
        let src = [0x44u8, 0, 0, 0, 0, 0, 0, 0];
        let detected = inspect_wchar_byte_pattern(src.as_ptr() as *const u16, sql::NTS as i32);
        assert_eq!(detected, None);
    }

    #[test]
    fn inspect_returns_none_for_null_pointer() {
        let detected = inspect_wchar_byte_pattern(std::ptr::null(), 4);
        assert_eq!(detected, None);
    }

    // ---------- *_in helpers ----------------------------------------------

    #[test]
    fn wide_unit_len_in_counts_units_per_encoding() {
        assert_eq!(wide_unit_len_in("Hi!", WCharEncoding::Utf16), 3);
        assert_eq!(wide_unit_len_in("Hi!", WCharEncoding::Utf32), 3);
        // Supplementary-plane code point: 2 UTF-16 units, 1 UTF-32 unit.
        let s = "\u{1F680}";
        assert_eq!(wide_unit_len_in(s, WCharEncoding::Utf16), 2);
        assert_eq!(wide_unit_len_in(s, WCharEncoding::Utf32), 1);
    }

    #[test]
    fn write_wide_buffer_in_utf16_round_trip() {
        let mut buf = [0u16; 8];
        let n =
            unsafe { write_wide_buffer_in("Hi!", buf.as_mut_ptr(), 8, 0, WCharEncoding::Utf16) };
        assert_eq!(n, 3);
        assert_eq!(&buf[..3], &[b'H' as u16, b'i' as u16, b'!' as u16]);
        unsafe { write_wide_null_in(buf.as_mut_ptr(), n, WCharEncoding::Utf16) };
        let decoded =
            unsafe { read_wide_string_in(buf.as_ptr(), sql::NTS as i32, WCharEncoding::Utf16) }
                .unwrap();
        assert_eq!(decoded, "Hi!");
    }

    #[test]
    fn write_wide_buffer_in_utf32_round_trip() {
        // 8 u16 slots = 4 u32 slots.
        let mut buf = [0u16; 8];
        let n =
            unsafe { write_wide_buffer_in("Hi!", buf.as_mut_ptr(), 4, 0, WCharEncoding::Utf32) };
        assert_eq!(n, 3);
        // Each ASCII char in UTF-32 LE: XX 00 00 00 -> two u16 slots:
        // [XX 00] then [00 00].
        assert_eq!(&buf[..6], &[b'H' as u16, 0, b'i' as u16, 0, b'!' as u16, 0]);
        unsafe { write_wide_null_in(buf.as_mut_ptr(), n, WCharEncoding::Utf32) };
        let decoded =
            unsafe { read_wide_string_in(buf.as_ptr(), sql::NTS as i32, WCharEncoding::Utf32) }
                .unwrap();
        assert_eq!(decoded, "Hi!");
    }

    #[test]
    fn write_wide_buffer_in_utf32_handles_supplementary_plane() {
        // U+1F680 ROCKET — supplementary plane.
        let s = "\u{1F680}";
        let mut buf = [0u16; 4];
        let n = unsafe { write_wide_buffer_in(s, buf.as_mut_ptr(), 2, 0, WCharEncoding::Utf32) };
        assert_eq!(n, 1);
        // 0x1F680 = 0x0001_F680: low half 0xF680, high half 0x0001.
        assert_eq!(buf[0], 0xF680);
        assert_eq!(buf[1], 0x0001);
        unsafe { write_wide_null_in(buf.as_mut_ptr(), n, WCharEncoding::Utf32) };
        let decoded =
            unsafe { read_wide_string_in(buf.as_ptr(), sql::NTS as i32, WCharEncoding::Utf32) }
                .unwrap();
        assert_eq!(decoded, s);
    }

    #[test]
    fn read_wide_string_in_utf32_rejects_invalid_code_points() {
        // 0x0011_0000 is one past the highest valid Unicode code point.
        let buf: [u16; 2] = [0x0000, 0x0011];
        let res = unsafe { read_wide_string_in(buf.as_ptr(), 1, WCharEncoding::Utf32) };
        assert!(matches!(
            res,
            Err(crate::api::OdbcError::InvalidWideChar { .. })
        ));
    }

    #[test]
    fn read_wide_string_in_rejects_non_positive_explicit_lengths() {
        for enc in [WCharEncoding::Utf16, WCharEncoding::Utf32] {
            // 4-byte alignment so the UTF-32 reinterpretation can pass
            // any internal slice precondition; we only care about the
            // pre-slice length check here.
            let buf: [u32; 1] = [0];
            for bad in [-1, 0] {
                let res = unsafe { read_wide_string_in(buf.as_ptr() as *const u16, bad, enc) };
                assert!(matches!(
                    res,
                    Err(crate::api::OdbcError::InvalidBufferLength { .. })
                ));
            }
        }
    }

    #[test]
    fn narrow_read_string_rejects_non_positive_explicit_lengths() {
        let buf = [0u8; 1];
        for bad in [-1, 0] {
            let res = Narrow::read_string(buf.as_ptr(), bad);
            assert!(matches!(
                res,
                Err(crate::api::OdbcError::InvalidBufferLength { .. })
            ));
        }
    }
}
