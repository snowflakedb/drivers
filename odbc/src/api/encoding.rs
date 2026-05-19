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
// Wide-character encoding (`SQLWCHAR`)
//
// The size of `SQLWCHAR` differs between ODBC driver managers on UNIX:
//
//   * unixODBC default                     : `unsigned short` (2 bytes, UTF-16)
//   * unixODBC with `-DSQL_WCHART_CONVERT` : `wchar_t`        (4 bytes, UTF-32)
//   * iODBC on UNIX                        : `wchar_t`        (4 bytes, UTF-32)
//   * iODBC on Windows / Windows           : `unsigned short` (2 bytes, UTF-16)
//
// At the C ABI level we declare every wide-string entry point as taking a
// `*mut u16` (matching Windows and stock unixODBC). When loaded under iODBC
// (or unixODBC built with `-DSQL_WCHART_CONVERT`) the driver manager actually
// hands us 4-byte buffers and counts in 4-byte units. Pointers are pointers
// at the ABI level, and the count parameter just forwards an integer; the
// only thing that goes wrong is *interpretation* — we'd read 2-byte chunks
// and treat counts as 2-byte unit counts when both should be 4-byte.
//
// The choice between the two interpretations is configured by the user via
// the `DriverManagerEncoding` key in `sf.odbc.ini`:
//
//   [snowflake]
//   DriverManagerEncoding=UTF-16   ; default; matches unixODBC
//   DriverManagerEncoding=UTF-32   ; required for iODBC on UNIX
//
// The INI file is read exactly once per process by
// [`sf_core::config::sf_odbc_ini::SfOdbcIni`]. The same snapshot drives
// the [`LogManager`] (via `LoggingConfig`), and this module pulls the
// `DriverManagerEncoding` value out of it on first access. The first
// wide buffer the driver sees is also inspected for its byte pattern as
// a sanity check; if the bytes don't match the configured encoding a
// warning is logged once with a pointer to the INI key the user should
// update. Auto-detection never changes the configured encoding — it only
// complains.
//
// [`LogManager`]: sf_core::logging::LogManager
// ---------------------------------------------------------------------------

/// C ABI element type for wide-character ODBC strings. Always 2 bytes. The
/// runtime [`WCharEncoding`] decides whether buffers of this type are read
/// as UTF-16 code units, or whether each consecutive pair of slots actually
/// belongs to one 4-byte UTF-32 code point delivered by iODBC (in which
/// case the pointer is reinterpreted as `*mut u32` internally).
pub type WideChar = u16;

/// Compile-time size of one [`WideChar`] in bytes (always 2). Most call
/// sites outside of `encoding.rs` should use [`wchar_byte_size`] instead,
/// which returns the *runtime* size of one DM-side `SQLWCHAR` (2 for
/// UTF-16, 4 for UTF-32). Currently consumed only by test code which builds
/// `[u16; N]` buffers with hard-coded UTF-16 expectations.
#[allow(dead_code)] // used only from `#[cfg(test)]` modules
pub const WIDE_CHAR_SIZE: usize = std::mem::size_of::<WideChar>();

/// Runtime wide-character encoding negotiated with the driver manager.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WCharEncoding {
    /// `SQLWCHAR` is 2 bytes (Windows; stock unixODBC). Buffers contain
    /// UTF-16 LE code units.
    Utf16,
    /// `SQLWCHAR` is 4 bytes (iODBC on UNIX; unixODBC built with
    /// `-DSQL_WCHART_CONVERT`). Buffers contain UTF-32 LE code points; the
    /// `*mut u16` C ABI pointer is reinterpreted as `*mut u32` internally.
    Utf32,
}

impl WCharEncoding {
    /// Canonical name as used in the `sf.odbc.ini` `DriverManagerEncoding`
    /// key, suitable for surfacing to users in log/error messages.
    pub fn as_ini_value(self) -> &'static str {
        match self {
            WCharEncoding::Utf16 => "UTF-16",
            WCharEncoding::Utf32 => "UTF-32",
        }
    }
}

impl std::fmt::Display for WCharEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ini_value())
    }
}

/// True after the buffer-pattern sanity check has logged a mismatch
/// warning. We only emit it once per process to avoid flooding the log
/// when the same misconfigured pointer keeps coming through.
static MISMATCH_WARNED: AtomicBool = AtomicBool::new(false);

/// `sf.odbc.ini` key that selects the encoding interpretation.
pub const DRIVER_MANAGER_ENCODING_KEY: &str = "DriverManagerEncoding";

#[cfg(test)]
thread_local! {
    /// Per-thread override used by tests so they can exercise both
    /// encoding modes without touching the process-global state (which
    /// would race against parallel tests).
    static THREAD_WCHAR_ENCODING: std::cell::Cell<Option<WCharEncoding>> =
        const { std::cell::Cell::new(None) };
}

/// Returns the current wide-character encoding, defaulting to
/// [`WCharEncoding::Utf16`] when nothing has been configured.
///
/// Reads `DriverManagerEncoding` from
/// [`sf_core::config::sf_odbc_ini::SfOdbcIni`] in production builds; the
/// global INI snapshot is loaded once per process on first access. In
/// `#[cfg(test)]` builds the per-thread override
/// ([`set_thread_wchar_encoding`]) is consulted first so parallel tests
/// can exercise both encodings without touching shared state.
#[inline]
pub fn current_wchar_encoding() -> WCharEncoding {
    #[cfg(test)]
    {
        // Tests deliberately do not consult `SfOdbcIni::global()` so that
        // a stray `SF_ODBC_INI` in the environment can't latch the
        // process-wide encoding for the rest of the test binary's life.
        THREAD_WCHAR_ENCODING
            .with(|c| c.get())
            .unwrap_or(WCharEncoding::Utf16)
    }
    #[cfg(not(test))]
    {
        sf_core::config::sf_odbc_ini::SfOdbcIni::global()
            .raw_value(DRIVER_MANAGER_ENCODING_KEY)
            .and_then(parse_wchar_encoding_value)
            .unwrap_or(WCharEncoding::Utf16)
    }
}

/// Parse a `DriverManagerEncoding` config value. Accepts case-insensitive
/// `utf-16` / `utf16` / `utf-32` / `utf32`.
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

/// Size in bytes of one DM-side `SQLWCHAR` code unit at runtime: 2 for
/// UTF-16 and 4 for UTF-32. Use this instead of [`WIDE_CHAR_SIZE`] anywhere
/// the math is about how the driver manager has packed bytes into a wide
/// buffer.
#[inline]
pub fn wchar_byte_size() -> usize {
    match current_wchar_encoding() {
        WCharEncoding::Utf16 => 2,
        WCharEncoding::Utf32 => 4,
    }
}

/// Inspect the leading bytes of a wide-string buffer the driver manager
/// has just handed us, and **warn once** if its byte pattern disagrees
/// with the encoding the user configured in `sf.odbc.ini`.
///
/// This is purely diagnostic: it never changes the global encoding. A
/// mismatch means the driver and the DM are wired up wrong; the user must
/// either edit [`DRIVER_MANAGER_ENCODING_KEY`] in `sf.odbc.ini` or load the
/// driver under a matching DM and restart.
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
    // Safety: we just verified the caller has at least 4 `WideChar` slots
    // (= 8 bytes) regardless of which DM-side encoding is in effect.
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

#[cfg(test)]
pub struct WCharEncodingGuard(Option<WCharEncoding>);

#[cfg(test)]
impl Drop for WCharEncodingGuard {
    fn drop(&mut self) {
        THREAD_WCHAR_ENCODING.with(|c| c.set(self.0));
    }
}

/// Override the wide-character encoding for the current thread. Restored
/// automatically when the returned guard is dropped. Test-only.
#[cfg(test)]
pub fn set_thread_wchar_encoding(enc: WCharEncoding) -> WCharEncodingGuard {
    let prev = THREAD_WCHAR_ENCODING.with(|c| c.replace(Some(enc)));
    WCharEncodingGuard(prev)
}

// ---------------------------------------------------------------------------
// Format-specific helpers (UTF-16 only)
//
// These do not consult the runtime encoding; they always produce / consume
// UTF-16 data in `u16` slots. They are used by tests (which set up
// `[u16; N]` buffers with hard-coded UTF-16 expectations). Production code
// should use the runtime-aware helpers further down (`write_wide_buffer`,
// `Wide::read_string`, …) so the same call works under either DM.
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
// Runtime-aware helpers
//
// These dispatch on `current_wchar_encoding()` to produce / consume buffers
// in whichever DM-side `SQLWCHAR` format is in effect. All counts and
// offsets they operate on are in **DM-side code units** (one `u16` in
// UTF-16 mode; one `u32` in UTF-32 mode).
// ---------------------------------------------------------------------------

/// Number of DM-side `SQLWCHAR` code units required to encode `s`.
#[inline]
pub fn wide_unit_len(s: &str) -> usize {
    match current_wchar_encoding() {
        WCharEncoding::Utf16 => s.encode_utf16().count(),
        WCharEncoding::Utf32 => s.chars().count(),
    }
}

/// Encode `s` and write up to `max_units` DM-side code units into `buf`,
/// starting from `offset_units` in `s`. Returns the number of DM-side
/// units actually written (may be less than `max_units` if the source
/// runs out first).
///
/// Does **not** write a null terminator; pair with [`write_wide_null`]
/// when one is needed.
///
/// # Safety
/// `buf` must point to a writable buffer of at least `max_units * wchar_byte_size()`
/// bytes. The buffer must remain valid for the duration of the call.
pub unsafe fn write_wide_buffer(
    s: &str,
    buf: *mut WideChar,
    max_units: usize,
    offset_units: usize,
) -> usize {
    if max_units == 0 {
        return 0;
    }
    match current_wchar_encoding() {
        WCharEncoding::Utf16 => {
            let mut written = 0;
            for u in s.encode_utf16().skip(offset_units).take(max_units) {
                unsafe {
                    std::ptr::write(buf.add(written), u);
                }
                written += 1;
            }
            written
        }
        WCharEncoding::Utf32 => {
            let buf32 = buf as *mut u32;
            let mut written = 0;
            for c in s.chars().skip(offset_units).take(max_units) {
                unsafe {
                    std::ptr::write(buf32.add(written), c as u32);
                }
                written += 1;
            }
            written
        }
    }
}

/// Write a single DM-side null terminator at offset `pos` from `buf`.
///
/// # Safety
/// `buf.add(pos * wchar_byte_size() / sizeof(WideChar))` must be a valid
/// writable address.
pub unsafe fn write_wide_null(buf: *mut WideChar, pos: usize) {
    match current_wchar_encoding() {
        WCharEncoding::Utf16 => unsafe {
            std::ptr::write(buf.add(pos), 0);
        },
        WCharEncoding::Utf32 => unsafe {
            let buf32 = buf as *mut u32;
            std::ptr::write(buf32.add(pos), 0);
        },
    }
}

/// Length, in DM-side code units, of a null-terminated wide-string buffer.
/// Bounded by `max_units` to prevent runaway scans.
///
/// # Safety
/// `ptr` must be valid for reads of at least `max_units * wchar_byte_size()`
/// bytes (or unbounded if `max_units == usize::MAX` and the caller has
/// guaranteed a null terminator exists).
pub unsafe fn wide_strlen_bounded(ptr: *const WideChar, max_units: usize) -> usize {
    match current_wchar_encoding() {
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
        // `length == 0` is a valid representation of an empty input
        // (e.g. binding `""` with explicit byte length 0). Only strictly
        // negative explicit lengths are invalid.
        if length != sql::NTS as i32 && length < 0 {
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
        if text.is_null() {
            return NullPointerSnafu.fail();
        }
        // `length == 0` is a valid representation of an empty wide input
        // (e.g. an `SQL_C_WCHAR` empty string bound with `SQL_NTS` whose
        // buffer holds just the wide null terminator — `read_wchar_str`
        // resolves it to 0 code units before delegating here). Only
        // strictly negative explicit lengths are invalid.
        if length != sql::NTS as i32 && length < 0 {
            return InvalidBufferLengthSnafu {
                length: length as i64,
            }
            .fail();
        }
        // Auto-detect the DM-side encoding before we slice. Once any wide
        // input has been seen the global is pinned for the rest of the
        // process; everything below dispatches on it.
        detect_wchar_encoding_from_bytes(text, length);

        match current_wchar_encoding() {
            WCharEncoding::Utf16 => {
                let slice = if length == sql::NTS as i32 {
                    let mut len = 0;
                    unsafe {
                        while *text.add(len) != 0 {
                            len += 1;
                        }
                        std::slice::from_raw_parts(text, len)
                    }
                } else {
                    unsafe { std::slice::from_raw_parts(text, length as usize) }
                };
                String::from_utf16(slice).context(TextConversionFromUtf16Snafu {})
            }
            WCharEncoding::Utf32 => {
                let p32 = text as *const u32;
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
                let mut s = String::with_capacity(units.len());
                for &cp in units {
                    match char::from_u32(cp) {
                        Some(c) => s.push(c),
                        None => return InvalidWideCharSnafu { code_point: cp }.fail(),
                    }
                }
                Ok(s)
            }
        }
    }

    fn write_string(string: &str, buffer: *mut Self::Char, buffer_length: usize) -> (usize, bool) {
        let full_len = wide_unit_len(string);
        if buffer.is_null() {
            return (full_len, false);
        }
        if buffer_length == 0 {
            return (full_len, full_len > 0);
        }
        let max_chars = buffer_length.saturating_sub(1);
        let written = unsafe { write_wide_buffer(string, buffer, max_chars, 0) };
        unsafe { write_wide_null(buffer, written) };
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

    #[test]
    fn detect_never_changes_encoding() {
        // The detector is a warning-only sanity check — the configured
        // encoding (here forced to UTF-16 on this thread) must always win.
        let _g = set_thread_wchar_encoding(WCharEncoding::Utf16);
        // Deliberately UTF-32-shaped bytes for "DR" (`44 00 00 00 52 00 00 00`).
        #[repr(align(4))]
        struct Aligned([u8; 8]);
        let src = Aligned([0x44, 0, 0, 0, 0x52, 0, 0, 0]);
        detect_wchar_encoding_from_bytes(src.0.as_ptr() as *const u16, 4);
        assert_eq!(current_wchar_encoding(), WCharEncoding::Utf16);
    }

    #[test]
    fn wchar_byte_size_tracks_encoding() {
        {
            let _g = set_thread_wchar_encoding(WCharEncoding::Utf16);
            assert_eq!(wchar_byte_size(), 2);
            assert_eq!(wide_unit_len("Hi"), 2);
        }
        {
            let _g = set_thread_wchar_encoding(WCharEncoding::Utf32);
            assert_eq!(wchar_byte_size(), 4);
            assert_eq!(wide_unit_len("Hi"), 2);
        }
    }

    #[test]
    fn write_wide_buffer_utf16_round_trip() {
        let _g = set_thread_wchar_encoding(WCharEncoding::Utf16);
        let mut buf = [0u16; 8];
        let n = unsafe { write_wide_buffer("Hi!", buf.as_mut_ptr(), 8, 0) };
        assert_eq!(n, 3);
        assert_eq!(&buf[..3], &[b'H' as u16, b'i' as u16, b'!' as u16]);
        // Round-trip via Wide::read_string.
        unsafe { write_wide_null(buf.as_mut_ptr(), n) };
        let s = Wide::read_string(buf.as_ptr(), sql::NTS as i32).unwrap();
        assert_eq!(s, "Hi!");
    }

    #[test]
    fn write_wide_buffer_utf32_round_trip() {
        let _g = set_thread_wchar_encoding(WCharEncoding::Utf32);
        // 8 u16 slots = 4 u32 slots.
        let mut buf = [0u16; 8];
        let n = unsafe { write_wide_buffer("Hi!", buf.as_mut_ptr(), 4, 0) };
        assert_eq!(n, 3);
        // Each ASCII char in UTF-32 LE: XX 00 00 00 -> two u16 slots:
        // [XX 00] then [00 00].
        assert_eq!(&buf[..6], &[b'H' as u16, 0, b'i' as u16, 0, b'!' as u16, 0]);
        unsafe { write_wide_null(buf.as_mut_ptr(), n) };
        let s = Wide::read_string(buf.as_ptr(), sql::NTS as i32).unwrap();
        assert_eq!(s, "Hi!");
    }

    #[test]
    fn write_wide_buffer_utf32_handles_supplementary_plane() {
        let _g = set_thread_wchar_encoding(WCharEncoding::Utf32);
        // A supplementary-plane code point: U+1F680 ROCKET.
        let s = "\u{1F680}";
        let mut buf = [0u16; 4];
        let n = unsafe { write_wide_buffer(s, buf.as_mut_ptr(), 2, 0) };
        assert_eq!(n, 1);
        // 0x1F680 = 0x0001_F680: low half 0xF680, high half 0x0001.
        assert_eq!(buf[0], 0xF680);
        assert_eq!(buf[1], 0x0001);
        unsafe { write_wide_null(buf.as_mut_ptr(), n) };
        let decoded = Wide::read_string(buf.as_ptr(), sql::NTS as i32).unwrap();
        assert_eq!(decoded, s);
    }

    #[test]
    fn write_wide_buffer_utf32_decode_rejects_invalid_code_points() {
        let _g = set_thread_wchar_encoding(WCharEncoding::Utf32);
        // 0x0011_0000 is one past the highest valid Unicode code point.
        let buf: [u16; 2] = [0x0000, 0x0011];
        let res = Wide::read_string(buf.as_ptr(), 1);
        assert!(res.is_err());
    }

    /// The byte-pattern inspector must recognise the canonical iODBC
    /// pattern: `"DR" → 44 00 00 00 52 00 00 00`.
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
    fn wide_read_string_round_trips_explicit_length_in_both_modes() {
        for enc in [WCharEncoding::Utf16, WCharEncoding::Utf32] {
            let _g = set_thread_wchar_encoding(enc);
            let mut buf = [0u16; 16];
            let s = "Hi!";
            let (n, _) = Wide::write_string(s, buf.as_mut_ptr(), 16);
            assert_eq!(n, s.chars().count());
            let decoded = Wide::read_string(buf.as_ptr(), n as i32).unwrap();
            assert_eq!(decoded, s);
        }
    }

    /// Regression test: binding an `SQL_C_WCHAR` empty string with
    /// `SQL_NTS` resolves to `unit_len == 0`, which is then forwarded to
    /// `Wide::read_string` as an explicit length. Length 0 must succeed
    /// with an empty result rather than be rejected as an invalid buffer
    /// length.
    ///
    /// The backing buffer is `u32`-aligned so the UTF-32 branch's
    /// internal `*const u32` reinterpretation satisfies
    /// `slice::from_raw_parts`'s alignment precondition even for a
    /// zero-length slice; production iODBC pointers are always 4-byte
    /// aligned for the same reason.
    #[test]
    fn wide_read_string_accepts_explicit_length_zero_as_empty() {
        for enc in [WCharEncoding::Utf16, WCharEncoding::Utf32] {
            let _g = set_thread_wchar_encoding(enc);
            let buf: [u32; 1] = [0];
            let s = Wide::read_string(buf.as_ptr() as *const u16, 0)
                .expect("length 0 is a valid empty string");
            assert_eq!(s, "");
        }
    }

    /// Mirror of [`wide_read_string_accepts_explicit_length_zero_as_empty`]
    /// for the narrow encoding: an explicit length of 0 means "empty
    /// string", not "invalid".
    #[test]
    fn narrow_read_string_accepts_explicit_length_zero_as_empty() {
        let buf = [0u8; 1];
        let s = Narrow::read_string(buf.as_ptr(), 0).expect("length 0 is a valid empty string");
        assert_eq!(s, "");
    }

    /// Strictly negative explicit lengths (anything other than `SQL_NTS`)
    /// must still be rejected by both encodings. The negative-length
    /// validator must reject before any pointer reinterpretation, so the
    /// buffer doesn't need to be 4-byte aligned for the wide check.
    #[test]
    fn read_string_still_rejects_negative_non_nts_lengths() {
        let wbuf: [u32; 1] = [0];
        assert!(Wide::read_string(wbuf.as_ptr() as *const u16, -1).is_err());
        let nbuf = [0u8; 1];
        assert!(Narrow::read_string(nbuf.as_ptr(), -1).is_err());
    }

    #[test]
    fn wchar_encoding_display_uses_canonical_ini_form() {
        assert_eq!(format!("{}", WCharEncoding::Utf16), "UTF-16");
        assert_eq!(format!("{}", WCharEncoding::Utf32), "UTF-32");
        assert_eq!(WCharEncoding::Utf16.as_ini_value(), "UTF-16");
        assert_eq!(WCharEncoding::Utf32.as_ini_value(), "UTF-32");
    }
}
