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
const DRIVER_MANAGER_ENCODING_KEY: &str = "DriverManagerEncoding";

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum WCharEncoding {
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

/// Latched by [`probe_driver_manager_identity`] when the driver manager
/// actually loaded into the process disagrees with the configured
/// `DriverManagerEncoding`. Once set, all subsequent wide-string writes
/// short-circuit rather than blindly stomping past the caller's buffer — a
/// UTF-32 driver writing 4-byte units into a unixODBC-provided
/// 2-byte-per-slot buffer overruns by 2x and trips `__stack_chk_fail`
/// inside the driver manager (SNOW-3741307).
///
/// Deliberately *not* latched from the wide-input byte-pattern heuristic
/// ([`detect_wchar_encoding_from_bytes`]): that heuristic assumes
/// ASCII-keyword input and mis-reads non-ASCII wide data (a bound UTF-32
/// CJK parameter, whose second byte is non-zero, looks like UTF-16), so
/// using it to drive an irreversible process-global kill switch produced
/// false positives that silently dropped all wide output on
/// correctly-configured iODBC. The heuristic is warn-only.
static MISMATCH_DETECTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn negotiate_from_config() {
    let enc = sf_core::config::get_ini_config()
        .and_then(|ini| ini.get(DRIVER_MANAGER_ENCODING_KEY))
        .and_then(parse_wchar_encoding_value)
        .unwrap_or(WCharEncoding::Utf16);
    // First call wins; subsequent calls are no-ops by design (the
    // negotiated DM-side wide width must remain stable for the life of
    // the process).
    let _ = WCHAR_ENCODING.set(enc);
    // Now that the configured encoding is fixed, probe the driver
    // manager we were actually loaded by. If its identity disagrees
    // with `DriverManagerEncoding`, latch `MISMATCH_DETECTED` before
    // any wide-string write (SNOW-3741307). The probe is a best-effort
    // dlsym+dladdr against the currently-loaded process image; if it
    // can't identify the DM we leave detection to
    // `detect_wchar_encoding_from_bytes`.
    probe_driver_manager_identity(enc);
}

/// Runtime driver-manager identification via `dlsym`/`dladdr`. Returns
/// the encoding the loaded DM appears to expect, or `None` when the DM
/// path can't be resolved or doesn't match a known family.
fn detect_loaded_dm_encoding() -> Option<WCharEncoding> {
    #[cfg(unix)]
    unsafe {
        // Any symbol the DM exports works; `SQLAllocEnv` is defined by
        // every ODBC 3 driver manager and is guaranteed to have been
        // resolved into the driver's process image by the time we run.
        let symbol_name = c"SQLAllocEnv";
        let ptr = libc::dlsym(libc::RTLD_DEFAULT, symbol_name.as_ptr());
        if ptr.is_null() {
            return None;
        }
        let mut info: libc::Dl_info = std::mem::zeroed();
        if libc::dladdr(ptr, &mut info) == 0 || info.dli_fname.is_null() {
            return None;
        }
        let path = std::ffi::CStr::from_ptr(info.dli_fname)
            .to_str()
            .ok()?
            .to_ascii_lowercase();
        // iODBC ships as libiodbc.*.dylib / libiodbc.so. Its SQLWCHAR
        // is `wchar_t` — 4 bytes on macOS and Linux.
        if path.contains("libiodbc") || path.contains("/iodbc") {
            return Some(WCharEncoding::Utf32);
        }
        // unixODBC ships as libodbc.*.dylib / libodbc.so.2. Its
        // SQLWCHAR is `unsigned short` by default (2 bytes) unless
        // built with `-DSQL_WCHART_CONVERT`.
        if path.contains("libodbc.") || path.ends_with("libodbc") {
            return Some(WCharEncoding::Utf16);
        }
        None
    }
    #[cfg(not(unix))]
    {
        None
    }
}

fn probe_driver_manager_identity(configured: WCharEncoding) {
    let Some(detected) = detect_loaded_dm_encoding() else {
        return;
    };
    if detected == configured {
        return;
    }
    MISMATCH_DETECTED.store(true, Ordering::Relaxed);
    if MISMATCH_WARNED
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        tracing::error!(
            configured = %configured,
            detected = %detected,
            "The driver manager loaded into this process expects {detected} \
             SQLWCHAR, but the driver is configured for {configured}. This causes \
             a {}x byte-width mismatch on every wide-string write to the DM and \
             will abort the process inside `extract_diag_error_w` on the first \
             error diagnostic. Wide writes are refused until the config is \
             corrected. Set `{DRIVER_MANAGER_ENCODING_KEY}={detected}` in \
             sf.odbc.ini and restart the driver.",
            configured.byte_size() / detected.byte_size(),
        );
    }
}

fn parse_wchar_encoding_value(s: &str) -> Option<WCharEncoding> {
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
pub(crate) fn current_wchar_encoding() -> WCharEncoding {
    WCHAR_ENCODING
        .get()
        .copied()
        .unwrap_or(WCharEncoding::Utf16)
}

#[inline]
pub(crate) fn wchar_byte_size() -> usize {
    current_wchar_encoding().byte_size()
}

/// True once [`probe_driver_manager_identity`] has confirmed the loaded
/// driver manager disagrees with the configured encoding. Wide-string
/// writes short-circuit while this is set.
#[inline]
pub(crate) fn wchar_mismatch_detected() -> bool {
    MISMATCH_DETECTED.load(Ordering::Relaxed)
}

#[cfg(test)]
pub(crate) fn set_mismatch_detected_for_test(v: bool) {
    MISMATCH_DETECTED.store(v, Ordering::Relaxed);
}

/// Inspect the leading bytes of a wide-string buffer the driver manager
/// has just handed us and **warn once** if the byte pattern disagrees with
/// the configured encoding.
///
/// Purely diagnostic: it never latches [`MISMATCH_DETECTED`] and never
/// changes the negotiated encoding. The inference assumes ASCII-keyword
/// input (see below) and mis-classifies non-ASCII wide data, so it is
/// unfit to gate the fatal kill switch — that job belongs to
/// [`probe_driver_manager_identity`], which identifies the loaded DM
/// directly rather than guessing from bytes.
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
fn detect_wchar_encoding_from_bytes(ptr: *const WideChar, length: sql::Integer) {
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
    // Diagnostic only — do NOT latch `MISMATCH_DETECTED` here. This
    // byte-pattern inference assumes ASCII-keyword input and is unreliable
    // on arbitrary wide data (a bound UTF-32 CJK parameter reads back as
    // UTF-16), so driving the irreversible kill switch from it silently
    // dropped all wide output on correctly-configured iODBC. The
    // authoritative mismatch latch lives in `probe_driver_manager_identity`,
    // which identifies the loaded DM directly. Warn once and carry on.
    if MISMATCH_WARNED
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        tracing::warn!(
            configured = %configured,
            detected = %detected,
            "Wide-character buffer byte pattern looks like {detected}, but the driver \
             is configured for {configured}. If decoding errors follow, verify \
             `{DRIVER_MANAGER_ENCODING_KEY}` matches the driver manager and restart \
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
/// encoding. Not part of the driver's public API — production code must
/// use [`write_wide_buffer`] so the runtime [`WCharEncoding`] is honoured.
#[allow(dead_code)] // used only from `#[cfg(test)]` modules
#[doc(hidden)]
#[inline]
pub(crate) fn encode_wide(s: &str) -> Vec<WideChar> {
    s.encode_utf16().collect()
}

/// Decode a UTF-16 slice into a `String`. Always UTF-16, regardless of
/// runtime encoding. Not part of the driver's public API — production
/// code must use the [`OdbcEncoding`] trait so the runtime
/// [`WCharEncoding`] is honoured.
#[allow(dead_code)] // used only from `#[cfg(test)]` modules
#[doc(hidden)]
pub(crate) fn decode_wide(units: &[WideChar]) -> OdbcResult<String> {
    String::from_utf16(units).context(TextConversionFromUtf16Snafu {})
}

// ---------------------------------------------------------------------------
// Encoding-aware helpers — explicit-encoding form.
//
// The `*_in` variants take a `WCharEncoding` directly and are pure with
// respect to global state.
//
// All counts and offsets they operate on are in **DM-side code units**
// (one `u16` in UTF-16 mode; one `u32` in UTF-32 mode).
// ---------------------------------------------------------------------------

/// Number of DM-side `SQLWCHAR` code units required to encode `s` under
/// `enc`.
#[doc(hidden)]
#[inline]
pub(crate) fn wide_unit_len_in(s: &str, enc: WCharEncoding) -> usize {
    match enc {
        WCharEncoding::Utf16 => s.encode_utf16().count(),
        WCharEncoding::Utf32 => s.chars().count(),
    }
}

/// Number of DM-side `SQLWCHAR` code units required to encode `s` under
/// the negotiated runtime encoding.
#[inline]
pub(crate) fn wide_unit_len(s: &str) -> usize {
    wide_unit_len_in(s, current_wchar_encoding())
}

/// Encode `s` and write up to `max_units` DM-side code units of `enc`
/// into `buf`, starting from `offset_units` in `s`. Returns the number of
/// DM-side units actually written.
///
/// The returned count may be less than `max_units` for any of:
/// - the source ran out (normal end-of-data);
/// - **UTF-16 only**: the next code point would emit a surrogate pair
///   and only one slot is left in `buf`. Splitting a surrogate pair
///   across two `SQLGetData` chunks would leave the application unable
///   to decode either chunk independently, so the lead surrogate is
///   deliberately held back for the next call. UTF-32 never splits
///   because one DM-side unit always equals one Unicode code point.
///
/// Does **not** write a null terminator; pair with [`write_wide_null_in`]
/// when one is needed.
///
/// # Safety
/// `buf` must point to a writable buffer of at least
/// `max_units * enc.byte_size()` bytes that remains valid for the
/// duration of the call.
#[doc(hidden)]
pub(crate) unsafe fn write_wide_buffer_in(
    s: &str,
    buf: *mut WideChar,
    max_units: usize,
    offset_units: usize,
    enc: WCharEncoding,
) -> usize {
    if max_units == 0 {
        return 0;
    }
    // Refuse to write past this point once the driver manager's byte
    // pattern has been shown to disagree with our configured encoding
    // (SNOW-3741307). Callers see zero units written, which surfaces to
    // applications as an obviously-empty result.
    if wchar_mismatch_detected() {
        return 0;
    }
    match enc {
        WCharEncoding::Utf16 => {
            let mut iter = s.encode_utf16().skip(offset_units).peekable();
            let mut written = 0;
            while written < max_units {
                let Some(u) = iter.next() else { break };
                // High surrogate (lead) + remaining capacity is exactly
                // one + more data follows ⇒ stop without writing, so the
                // pair lands together on the next call.
                if (0xD800..=0xDBFF).contains(&u)
                    && written + 1 == max_units
                    && iter.peek().is_some()
                {
                    break;
                }
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
pub(crate) unsafe fn write_wide_buffer(
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
#[doc(hidden)]
pub(crate) unsafe fn write_wide_null_in(buf: *mut WideChar, pos: usize, enc: WCharEncoding) {
    // Match the short-circuit in [`write_wide_buffer_in`] when the DM
    // byte width disagrees with the configured encoding.
    if wchar_mismatch_detected() {
        return;
    }
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
pub(crate) unsafe fn write_wide_null(buf: *mut WideChar, pos: usize) {
    unsafe { write_wide_null_in(buf, pos, current_wchar_encoding()) }
}

/// Length, in DM-side code units, of a null-terminated wide-string buffer
/// under `enc`. Bounded by `max_units` to prevent runaway scans.
///
/// # Safety
/// `ptr` must be valid for reads of at least `max_units * enc.byte_size()`
/// bytes (or unbounded if `max_units == usize::MAX` and the caller has
/// guaranteed a null terminator exists).
#[doc(hidden)]
pub(crate) unsafe fn wide_strlen_bounded_in(
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
pub(crate) unsafe fn wide_strlen_bounded(ptr: *const WideChar, max_units: usize) -> usize {
    unsafe { wide_strlen_bounded_in(ptr, max_units, current_wchar_encoding()) }
}

/// Decode a DM-side wide buffer of `length` code units (or until the
/// first null when `length == SQL_NTS`) into a Rust `String` under `enc`.
///
/// # Safety
/// `ptr` must be valid for reads of either `length * enc.byte_size()`
/// bytes (explicit length) or up to the first null terminator (SQL_NTS).
#[doc(hidden)]
pub(crate) unsafe fn read_wide_string_in(
    ptr: *const WideChar,
    length: sql::Integer,
    enc: WCharEncoding,
) -> OdbcResult<String> {
    if ptr.is_null() {
        return NullPointerSnafu.fail();
    }
    if length != sql::NTS as i32 && length < 0 {
        return InvalidBufferLengthSnafu {
            length: length as i64,
        }
        .fail();
    }
    if length == 0 {
        return Ok(String::new());
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
pub(crate) fn is_ascii_locale() -> bool {
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
pub(crate) fn is_ascii_locale() -> bool {
    false
}

/// Replace each non-ASCII character with `\x1a` (SUB control character).
///
/// Returns a borrow for pure-ASCII input to avoid a per-call heap
/// allocation on the `write_char_string` hot path. ODBC ANSI output is
/// ASCII-only in C/POSIX locales, and in practice the vast majority of
/// values that flow through here (numbers, dates, times, English text)
/// are already ASCII.
pub(crate) fn mask_non_ascii_characters(src: &str) -> std::borrow::Cow<'_, str> {
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
pub(crate) trait OdbcEncoding {
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
pub(crate) struct Narrow;

/// Marker type for Unicode (wide) encoding. The C ABI element type is
/// always `u16` ([`WideChar`]); whether the bytes inside a buffer are
/// interpreted as UTF-16 or UTF-32 is decided at runtime via
/// [`WCharEncoding`].
pub(crate) struct Wide;

impl OdbcEncoding for Narrow {
    type Char = sql::Char;

    fn effective_char_size() -> usize {
        1
    }

    fn read_string(text: *const Self::Char, length: sql::Integer) -> OdbcResult<String> {
        if text.is_null() {
            return NullPointerSnafu.fail();
        }
        if length != sql::NTS as i32 && length < 0 {
            return InvalidBufferLengthSnafu {
                length: length as i64,
            }
            .fail();
        }
        if length == 0 {
            return Ok(String::new());
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

/// Read a string from an `sql::Pointer`. `string_length` is in **bytes**
/// for the narrow path, in **DM-side code units** for the wide path, or
/// `SQL_NTS` to request null-terminated decoding.
///
/// Returns an empty string if the pointer is null.
///
/// Used by: `SQLSetConnectAttr` / `SQLSetConnectAttrW`.
pub(crate) fn read_string_from_pointer<E: OdbcEncoding>(
    value_ptr: sql::Pointer,
    string_length: sql::Integer,
) -> OdbcResult<String> {
    if value_ptr.is_null() {
        return Ok(String::new());
    }
    // SQL_NTS must propagate through to `E::read_string` so it can run its
    // null-terminator scan. The byte→unit conversion below would otherwise
    // map `SQL_NTS = -3` to `0` (UTF-32) or `-1` (UTF-16) and produce an
    // empty string / an `InvalidBufferLength` error.
    if string_length == sql::NTS as sql::Integer {
        return E::read_string(value_ptr as *const E::Char, string_length);
    }
    let char_size = E::effective_char_size() as sql::Integer;
    let length_in_chars = string_length / char_size;
    E::read_string(value_ptr as *const E::Char, length_in_chars)
}

/// Read a Snowflake-custom string-valued connect attribute (e.g.
/// `SQL_SF_CONN_ATTR_PRIV_KEY_BASE64`), tolerating iODBC's quirk of
/// forwarding narrow buffers to `SQLSetConnectAttrW` for unknown
/// attribute IDs.
///
/// iODBC only transcodes narrow→wide on `SQLSetConnectAttr` for the
/// attribute IDs it recognises (`SQL_ATTR_CURRENT_CATALOG`,
/// `SQL_ATTR_TRACEFILE`, …). For anything else it forwards the narrow
/// pointer and the narrow byte count to the driver's W variant as-is.
/// A driver that blindly decodes the buffer as UTF-32 then sees garbage
/// (or `InvalidWideChar`) and the user's payload is lost.
///
/// We sniff the leading bytes to tell the two layouts apart:
///
///   * non-empty narrow ASCII : `XX YY ZZ …`     (byte 1 non-zero)
///   * UTF-16 wide ASCII      : `XX 00 YY 00 …`  (byte 1 zero, byte 2 non-zero)
///   * UTF-32 wide ASCII      : `XX 00 00 00 …`  (bytes 1-3 zero, byte 4 non-zero)
///
/// The Snowflake-custom string attributes only ever carry printable
/// ASCII (base64, PEM, app names, passphrases), so the byte pattern is
/// unambiguous in practice.
pub(crate) fn read_pre_connection_string_attr<E: OdbcEncoding>(
    value_ptr: sql::Pointer,
    string_length: sql::Integer,
) -> OdbcResult<String> {
    if value_ptr.is_null() {
        return Ok(String::new());
    }
    if looks_like_narrow_buffer(value_ptr, string_length) {
        return Narrow::read_string(value_ptr as *const sql::Char, string_length);
    }
    read_string_from_pointer::<E>(value_ptr, string_length)
}

/// Return `true` when the leading bytes of `value_ptr` match a non-empty
/// narrow ASCII string rather than a UTF-16 / UTF-32 wide buffer. Used
/// only by [`read_pre_connection_string_attr`].
fn looks_like_narrow_buffer(value_ptr: sql::Pointer, string_length: sql::Integer) -> bool {
    if value_ptr.is_null() {
        return false;
    }
    // We need at least 2 bytes of data to distinguish narrow from wide.
    // SQL_NTS is `-3`; treat it like "unknown length" and assume the
    // caller has at least 2 bytes of payload before the terminator.
    if string_length != sql::NTS as sql::Integer && string_length < 2 {
        return false;
    }
    // Safety: caller-owned buffer at least 2 bytes long under the
    // conditions above.
    let bytes = unsafe { std::slice::from_raw_parts(value_ptr as *const u8, 2) };
    // A leading zero byte is unusual for any of our payloads (base64,
    // PEM, ASCII passwords / app names) — bail out and let the declared
    // encoding handle it.
    if bytes[0] == 0 {
        return false;
    }
    // Narrow ASCII keeps every byte non-zero; UTF-16/UTF-32 ASCII has a
    // zero high byte right after the leading ASCII char.
    bytes[1] != 0
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
///
/// Returns whether the value was truncated. Callers that report truncation as a
/// `01004` warning pass `Some(warnings)`; callers that need to act on truncation
/// differently (e.g. `SQLBrowseConnect`, which returns `SQL_NEED_DATA`) pass
/// `None` and use the returned flag.
pub(crate) fn write_string_chars<E: OdbcEncoding>(
    string: &str,
    buffer: *mut E::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    warnings: Option<&mut Warnings>,
) -> bool {
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
    truncated
}

/// Write a string where `buffer_length` and `*string_length_ptr` count
/// **characters** as `sql::Integer`.
///
/// Used by: `SQLNativeSql`, whose `BufferLength` / `TextLength2Ptr` are
/// character counts per the ODBC spec, independent of the DM-side `SQLWCHAR`
/// width. (Contrast `write_string_bytes_i32`, whose lengths are byte counts.)
pub(crate) fn write_string_chars_i32<E: OdbcEncoding>(
    string: &str,
    buffer: *mut E::Char,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
    warnings: Option<&mut Warnings>,
) {
    let buf_units = if buffer_length < 0 {
        0
    } else {
        buffer_length as usize
    };
    let (char_len, truncated) = E::write_string(string, buffer, buf_units);
    if !string_length_ptr.is_null() {
        unsafe { std::ptr::write(string_length_ptr, char_len as sql::Integer) };
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
pub(crate) fn write_string_bytes<E: OdbcEncoding>(
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
pub(crate) fn write_string_bytes_i32<E: OdbcEncoding>(
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
        // 4-aligned u32 backing buffer: write_wide_buffer_in's UTF-32
        // branch reinterprets the pointer as *mut u32, which requires
        // 4-byte alignment. Casting at the call site preserves the
        // public *mut WideChar signature.
        let mut buf = [0u32; 4];
        let n = unsafe {
            write_wide_buffer_in(
                "Hi!",
                buf.as_mut_ptr() as *mut WideChar,
                4,
                0,
                WCharEncoding::Utf32,
            )
        };
        assert_eq!(n, 3);
        assert_eq!(&buf[..3], &[b'H' as u32, b'i' as u32, b'!' as u32]);
        unsafe { write_wide_null_in(buf.as_mut_ptr() as *mut WideChar, n, WCharEncoding::Utf32) };
        let decoded = unsafe {
            read_wide_string_in(
                buf.as_ptr() as *const WideChar,
                sql::NTS as i32,
                WCharEncoding::Utf32,
            )
        }
        .unwrap();
        assert_eq!(decoded, "Hi!");
    }

    #[test]
    fn write_wide_buffer_in_utf32_handles_supplementary_plane() {
        // U+1F680 ROCKET — supplementary plane.
        let s = "\u{1F680}";
        // 4-aligned u32 backing buffer; see _round_trip test for rationale.
        let mut buf = [0u32; 2];
        let n = unsafe {
            write_wide_buffer_in(
                s,
                buf.as_mut_ptr() as *mut WideChar,
                2,
                0,
                WCharEncoding::Utf32,
            )
        };
        assert_eq!(n, 1);
        assert_eq!(buf[0], 0x0001_F680);
        unsafe { write_wide_null_in(buf.as_mut_ptr() as *mut WideChar, n, WCharEncoding::Utf32) };
        let decoded = unsafe {
            read_wide_string_in(
                buf.as_ptr() as *const WideChar,
                sql::NTS as i32,
                WCharEncoding::Utf32,
            )
        }
        .unwrap();
        assert_eq!(decoded, s);
    }

    /// `write_wide_buffer_in` must not split a UTF-16 surrogate pair
    /// across two chunked SQLGetData calls. With `max_units == 1` and a
    /// non-BMP code point pending, the call must return 0 (lead
    /// surrogate held back); the next call with capacity ≥ 2 emits both
    /// surrogates together.
    #[test]
    fn write_wide_buffer_in_utf16_does_not_split_surrogate_pair() {
        let s = "\u{1F680}";
        let mut buf = [0u16; 2];
        let first =
            unsafe { write_wide_buffer_in(s, buf.as_mut_ptr(), 1, 0, WCharEncoding::Utf16) };
        assert_eq!(first, 0, "must not emit a lead surrogate as the final unit");
        let second =
            unsafe { write_wide_buffer_in(s, buf.as_mut_ptr(), 2, 0, WCharEncoding::Utf16) };
        assert_eq!(second, 2);
        assert_eq!(buf, [0xD83D, 0xDE80]);
    }

    /// When a lead surrogate genuinely *is* the last unit of the source
    /// (e.g. malformed input or natural end-of-string), the no-split
    /// guard must not hold it back: there is nothing to pair it with on
    /// the next call. Emit it like any other unit.
    #[test]
    fn write_wide_buffer_in_utf16_emits_trailing_unit_when_no_more_data() {
        // "A\u{1F680}" encodes to [0x0041, 0xD83D, 0xDE80].
        let s = "A\u{1F680}";
        // Ask for offset 2 (past the lead surrogate) so the iterator
        // produces only the trail surrogate (0xDE80). This isn't a lead
        // surrogate so the guard is irrelevant, but the test also
        // exercises the `iter.peek().is_some()` arm: with max_units=1
        // and only one unit left, no hold-back occurs.
        let mut buf = [0u16; 1];
        let n = unsafe { write_wide_buffer_in(s, buf.as_mut_ptr(), 1, 2, WCharEncoding::Utf16) };
        assert_eq!(n, 1);
        assert_eq!(buf, [0xDE80]);
    }

    #[test]
    fn read_wide_string_in_utf32_rejects_invalid_code_points() {
        // 0x0011_0000 is one past the highest valid Unicode code point.
        // 4-aligned u32 backing buffer; see _round_trip test for rationale.
        let buf: [u32; 1] = [0x0011_0000];
        let res = unsafe {
            read_wide_string_in(buf.as_ptr() as *const WideChar, 1, WCharEncoding::Utf32)
        };
        assert!(matches!(
            res,
            Err(crate::api::OdbcError::InvalidWideChar { .. })
        ));
    }

    #[test]
    fn read_wide_string_in_rejects_negative_explicit_lengths() {
        for enc in [WCharEncoding::Utf16, WCharEncoding::Utf32] {
            // 4-byte alignment so the UTF-32 reinterpretation can pass
            // any internal slice precondition; we only care about the
            // pre-slice length check here.
            let buf: [u32; 1] = [0];
            let res = unsafe { read_wide_string_in(buf.as_ptr() as *const u16, -1, enc) };
            assert!(matches!(
                res,
                Err(crate::api::OdbcError::InvalidBufferLength { .. })
            ));
        }
    }

    /// Explicit length 0 must produce an empty string, not an error. This
    /// is what `read_wchar_str` does for an empty null-terminated
    /// `SQL_C_WCHAR` parameter (`SQLWCHAR val[] = {0}; SQL_NTS`):
    /// `wide_strlen_bounded` returns 0 and the reader is called with
    /// length 0, which must round-trip to "".
    #[test]
    fn read_wide_string_in_accepts_zero_length_as_empty() {
        for enc in [WCharEncoding::Utf16, WCharEncoding::Utf32] {
            let buf: [u32; 1] = [0];
            let res = unsafe { read_wide_string_in(buf.as_ptr() as *const u16, 0, enc) }.unwrap();
            assert_eq!(res, "");
        }
    }

    #[test]
    fn narrow_read_string_rejects_negative_explicit_lengths() {
        let buf = [0u8; 1];
        let res = Narrow::read_string(buf.as_ptr(), -1);
        assert!(matches!(
            res,
            Err(crate::api::OdbcError::InvalidBufferLength { .. })
        ));
    }

    #[test]
    fn narrow_read_string_accepts_zero_length_as_empty() {
        let buf = [0u8; 1];
        let res = Narrow::read_string(buf.as_ptr(), 0).unwrap();
        assert_eq!(res, "");
    }

    // ---------- read_string_from_pointer (SQL_NTS handling) ----------

    /// When iODBC forwards a narrow
    /// `SQLSetConnectAttr(SQL_ATTR_CURRENT_CATALOG, "MYDB", SQL_NTS)` call
    /// into `SQLSetConnectAttrW`, the wide entry point receives a wide LE
    /// buffer with `string_length == SQL_NTS = -3`. The reader must
    /// propagate `SQL_NTS` through to the encoded null-terminator scan
    /// instead of converting `-3` to a unit count via `-3 / char_size`
    /// (which previously produced `0` under UTF-32 and `-1` under UTF-16,
    /// silently dropping the value or returning `InvalidBufferLength`).
    #[test]
    fn read_string_from_pointer_wide_sql_nts_round_trips() {
        let s = "MYDB";
        let mut buf: Vec<u16> = s.encode_utf16().collect();
        buf.push(0);
        let recovered = read_string_from_pointer::<Wide>(
            buf.as_ptr() as sql::Pointer,
            sql::NTS as sql::Integer,
        )
        .unwrap();
        assert_eq!(recovered, s);
    }

    /// `SQL_NTS` must also work for the narrow path
    #[test]
    fn read_string_from_pointer_narrow_sql_nts_round_trips() {
        let s = "MYDB\0";
        let recovered = read_string_from_pointer::<Narrow>(
            s.as_ptr() as sql::Pointer,
            sql::NTS as sql::Integer,
        )
        .unwrap();
        assert_eq!(recovered, "MYDB");
    }

    // ---------- read_pre_connection_string_attr (iODBC-tolerant) ----------

    /// Simulates the iODBC quirk: the application calls narrow
    /// `SQLSetConnectAttr` with a base64 ASCII string and iODBC forwards
    /// the narrow pointer + narrow byte count to the driver's W variant
    /// (no transcoding for unknown attribute IDs). The reader must
    /// recover the original ASCII string instead of mis-decoding it as
    /// UTF-32.
    #[test]
    fn read_pre_connection_string_attr_recovers_narrow_buffer_in_wide_path() {
        let s = "LS0tLS1CRUdJTi0tLS0t"; // base64-style ASCII payload
        let buf = s.as_bytes();
        let recovered = read_pre_connection_string_attr::<Wide>(
            buf.as_ptr() as sql::Pointer,
            buf.len() as sql::Integer,
        )
        .unwrap();
        assert_eq!(recovered, s);
    }

    // Note: end-to-end UTF-16 / UTF-32 decoding through the W path is
    // covered by the `read_wide_string_in_*` tests (which exercise the
    // full `Wide` decoder with an explicit `WCharEncoding`) together
    // with the `looks_like_narrow_buffer_rejects_utf{16,32}_pair`
    // tests below (which prove the heuristic in
    // `read_pre_connection_string_attr` does not false-positive on
    // legitimate wide buffers).

    /// Narrow path (Narrow::read_string) must keep working unchanged.
    #[test]
    fn read_pre_connection_string_attr_narrow_path_unchanged() {
        let s = "plain-ascii";
        let recovered = read_pre_connection_string_attr::<Narrow>(
            s.as_ptr() as sql::Pointer,
            s.len() as sql::Integer,
        )
        .unwrap();
        assert_eq!(recovered, s);
    }

    /// Null pointer is treated as the empty string (same contract as
    /// [`read_string_from_pointer`]).
    #[test]
    fn read_pre_connection_string_attr_null_pointer_is_empty() {
        let recovered = read_pre_connection_string_attr::<Wide>(std::ptr::null_mut(), 0).unwrap();
        assert_eq!(recovered, "");
    }

    #[test]
    fn looks_like_narrow_buffer_recognises_ascii_pair() {
        let buf = [b'A', b'B'];
        assert!(looks_like_narrow_buffer(
            buf.as_ptr() as sql::Pointer,
            buf.len() as sql::Integer,
        ));
    }

    #[test]
    fn looks_like_narrow_buffer_rejects_utf16_pair() {
        let buf = [b'A', 0u8];
        assert!(!looks_like_narrow_buffer(
            buf.as_ptr() as sql::Pointer,
            buf.len() as sql::Integer,
        ));
    }

    #[test]
    fn looks_like_narrow_buffer_rejects_utf32_pair() {
        let buf = [b'A', 0u8, 0u8, 0u8];
        assert!(!looks_like_narrow_buffer(
            buf.as_ptr() as sql::Pointer,
            buf.len() as sql::Integer,
        ));
    }

    #[test]
    fn looks_like_narrow_buffer_rejects_leading_zero_byte() {
        let buf = [0u8, b'A'];
        assert!(!looks_like_narrow_buffer(
            buf.as_ptr() as sql::Pointer,
            buf.len() as sql::Integer,
        ));
    }

    #[test]
    fn looks_like_narrow_buffer_handles_short_input() {
        let buf = [b'A'];
        assert!(!looks_like_narrow_buffer(
            buf.as_ptr() as sql::Pointer,
            buf.len() as sql::Integer,
        ));
    }

    // ---------- wchar_mismatch_detected fail-fast -------------------------

    /// Once [`MISMATCH_DETECTED`] is set, `write_wide_buffer_in` must
    /// refuse to write anything and return zero units — regardless of
    /// requested encoding — for a UTF-32 driver against a unixODBC
    /// (2-byte SQLWCHAR) manager (SNOW-3741307).
    #[test]
    fn write_wide_buffer_in_refuses_writes_after_mismatch_utf32() {
        set_mismatch_detected_for_test(true);
        let mut buf = [0u16; 32];
        let n = unsafe {
            write_wide_buffer_in(
                "DRIVER=Snowflake",
                buf.as_mut_ptr(),
                16,
                0,
                WCharEncoding::Utf32,
            )
        };
        set_mismatch_detected_for_test(false);
        assert_eq!(n, 0);
        assert!(
            buf.iter().all(|&u| u == 0),
            "buffer must not have been touched"
        );
    }

    /// Same guarantee for UTF-16 — the fail-fast is unconditional once
    /// the flag is latched, not just for the UTF-32 code path.
    #[test]
    fn write_wide_buffer_in_refuses_writes_after_mismatch_utf16() {
        set_mismatch_detected_for_test(true);
        let mut buf = [0u16; 32];
        let n =
            unsafe { write_wide_buffer_in("Hi!", buf.as_mut_ptr(), 32, 0, WCharEncoding::Utf16) };
        set_mismatch_detected_for_test(false);
        assert_eq!(n, 0);
        assert!(buf.iter().all(|&u| u == 0));
    }

    /// `write_wide_null_in` must also short-circuit when the DM byte
    /// width disagrees with the configured encoding.
    #[test]
    fn write_wide_null_in_refuses_writes_after_mismatch() {
        set_mismatch_detected_for_test(true);
        let mut buf = [0xAAu16; 4];
        unsafe { write_wide_null_in(buf.as_mut_ptr(), 0, WCharEncoding::Utf32) };
        set_mismatch_detected_for_test(false);
        assert_eq!(buf, [0xAA, 0xAA, 0xAA, 0xAA]);
    }

    /// Sanity check: the accessor round-trips through the same atomic
    /// the write functions consult.
    #[test]
    fn wchar_mismatch_detected_round_trip() {
        set_mismatch_detected_for_test(false);
        assert!(!wchar_mismatch_detected());
        set_mismatch_detected_for_test(true);
        assert!(wchar_mismatch_detected());
        set_mismatch_detected_for_test(false);
    }

    /// Root-cause regression for the iODBC false-positive (SNOW-3741307
    /// follow-up): the byte-pattern heuristic mis-reads non-ASCII UTF-32
    /// input as UTF-16. "日本" (U+65E5 U+672C) in UTF-32 LE is
    /// `E5 65 00 00 2C 67 00 00`; the non-zero second byte (0x65) matches
    /// the UTF-16 `XX 00 YY 00` shape, so the inference returns UTF-16 for
    /// what is really UTF-32. Because bound wide parameters are routinely
    /// non-ASCII, this heuristic is unsound as a mismatch signal and must
    /// stay diagnostic-only — it must never gate the irreversible
    /// `MISMATCH_DETECTED` kill switch (which silently dropped all wide
    /// output when it did).
    #[test]
    fn byte_pattern_misclassifies_non_ascii_utf32_as_utf16() {
        // [u16; 4] on a little-endian host lays out as the UTF-32 LE bytes above.
        let utf32_cjk: [WideChar; 4] = [0x65E5, 0x0000, 0x672C, 0x0000];
        assert_eq!(
            inspect_wchar_byte_pattern(utf32_cjk.as_ptr(), utf32_cjk.len() as sql::Integer),
            Some(WCharEncoding::Utf16),
            "non-ASCII UTF-32 is mis-inferred as UTF-16, so the heuristic is unfit \
             to gate the fatal kill switch"
        );
    }
}
