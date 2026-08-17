//! Cross-platform shim over the ODBC installer API's
//! `SQLGetPrivateProfileString`, used to resolve the driver's installed
//! file name for `SQLGetInfo(SQL_DRIVER_NAME)`. Internally it resolves the
//! full on-disk path of the shared library and then hands callers just the
//! file-name component (see [`resolve_driver_name`]).
//!
//! - **Windows**: the Driver Manager exposes `SQLGetPrivateProfileStringW`
//!   via `odbccp32.dll` (already bound in [`crate::setup_common`]). When the
//!   ini/registry layers come up empty (e.g. User-DSN test setups that can't
//!   write the admin-only `ODBCINST.INI`), the layer-4 fallback below uses
//!   `GetModuleHandleExW` + `GetModuleFileNameW` — the Win32 analogue of the
//!   Unix `dladdr` probe — to recover the loaded driver's own path.
//! - **Unix**: the symbol is exported by `libodbcinst.so(.2)` (unixODBC)
//!   or `libiodbcinst.dylib` (iODBC). Crucially, unixODBC's *DM*
//!   (`libodbc`) does **not** export it and does not load `libodbcinst`
//!   transitively — it keeps its DSN/INI parsing internal and exposes
//!   the installer API as a separate library for applications that
//!   want to configure DSNs. Our driver only links the DM, so a naive
//!   `dlsym(RTLD_DEFAULT, ...)` returns `NULL` for unixODBC-hosted
//!   loads and the resolver below fell through to `""`.
//!
//!   To avoid that we now `dlopen` the installer library lazily on
//!   first use, walking a small candidate list (unixODBC then iODBC,
//!   versioned soname first) and caching the resolved function pointer
//!   in a `OnceLock`. We deliberately leak the dlopen handle: the
//!   installer is meant to live for the lifetime of the process; closing
//!   it would invalidate the cached function pointer.
//!
//!   When even the dlopen fails (genuinely no installer lib on disk,
//!   embedded contexts, sandboxed unit tests) `resolve_driver_path`
//!   has a final layer-4 fallback that uses `dladdr` (see the Windows
//!   bullet above for the equivalent there) to ask the dynamic loader
//!   where *this* shared library lives, which is exactly the
//!   "file name of the driver used to access the data source" the ODBC
//!   spec asks `SQL_DRIVER_NAME` to return.

#[cfg(target_os = "windows")]
mod platform {
    use crate::setup_common::{SQLGetPrivateProfileStringW, from_wide, to_wide};

    pub fn get_private_profile_string(
        section: &str,
        entry: &str,
        filename: &str,
    ) -> Option<String> {
        let section_w = to_wide(section);
        let entry_w = to_wide(entry);
        let default_w = to_wide("");
        let filename_w = to_wide(filename);
        let mut buf = vec![0u16; 512];
        loop {
            let copied = unsafe {
                SQLGetPrivateProfileStringW(
                    section_w.as_ptr(),
                    entry_w.as_ptr(),
                    default_w.as_ptr(),
                    buf.as_mut_ptr(),
                    buf.len() as i32,
                    filename_w.as_ptr(),
                )
            };
            if copied < 0 {
                return None;
            }
            let copied = copied as usize;
            // Microsoft docs: when the returned value equals
            // `cchRetBuffer - 1` the string was truncated. Grow and retry.
            if copied + 1 >= buf.len() {
                buf.resize(buf.len() * 2, 0);
                continue;
            }
            return Some(from_wide(&buf[..copied]));
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use libc::{RTLD_DEFAULT, RTLD_GLOBAL, RTLD_LAZY, c_char, c_int, c_void, dlopen, dlsym};
    use std::ffi::CString;
    use std::sync::OnceLock;

    type SqlGetPrivateProfileStringFn = unsafe extern "C" fn(
        section: *const c_char,
        entry: *const c_char,
        default_value: *const c_char,
        ret_buffer: *mut c_char,
        buffer_size: c_int,
        filename: *const c_char,
    ) -> c_int;

    static FUNC: OnceLock<Option<SqlGetPrivateProfileStringFn>> = OnceLock::new();

    /// Candidate installer libraries to `dlopen`, ordered by priority.
    /// Versioned sonames first (what's actually shipped in distros) and
    /// unversioned as a fallback for dev installs. unixODBC's
    /// `libodbcinst` precedes iODBC's `libiodbcinst` because unixODBC
    /// is what every CI box and the Linux reference container use.
    #[cfg(target_os = "macos")]
    const INSTALLER_LIB_CANDIDATES: &[&str] = &[
        "libodbcinst.2.dylib",
        "libodbcinst.dylib",
        "libiodbcinst.2.dylib",
        "libiodbcinst.dylib",
    ];

    #[cfg(not(target_os = "macos"))]
    const INSTALLER_LIB_CANDIDATES: &[&str] = &[
        "libodbcinst.so.2",
        "libodbcinst.so",
        "libiodbcinst.so.2",
        "libiodbcinst.so",
    ];

    fn resolve_symbol() -> Option<SqlGetPrivateProfileStringFn> {
        *FUNC.get_or_init(|| unsafe {
            let name = CString::new("SQLGetPrivateProfileString").ok()?;

            // Fast path: the symbol is already in our address space
            // (Windows-style applications that linked `libodbcinst`
            // directly, or callers running under a DM that did so on
            // their behalf).
            let sym = dlsym(RTLD_DEFAULT, name.as_ptr());
            if !sym.is_null() {
                return Some(std::mem::transmute::<
                    *mut c_void,
                    SqlGetPrivateProfileStringFn,
                >(sym));
            }

            // Slow path: pull the installer in ourselves. unixODBC's DM
            // doesn't load `libodbcinst.so` and Linux/macOS application
            // binaries that only `-lodbc` therefore can't reach the
            // symbol via `RTLD_DEFAULT`. `RTLD_GLOBAL` lets a subsequent
            // `RTLD_DEFAULT` probe (from any other component) succeed
            // too; `RTLD_LAZY` is fine because we resolve exactly one
            // symbol immediately after.
            //
            // The handle is intentionally leaked — the installer is
            // expected to live for the process lifetime and the cached
            // function pointer below would become a dangling dlsym
            // result if we ever `dlclose`'d it.
            for candidate in INSTALLER_LIB_CANDIDATES {
                let Ok(lib_name) = CString::new(*candidate) else {
                    continue;
                };
                let handle = dlopen(lib_name.as_ptr(), RTLD_LAZY | RTLD_GLOBAL);
                if handle.is_null() {
                    continue;
                }
                let sym = dlsym(handle, name.as_ptr());
                if !sym.is_null() {
                    return Some(std::mem::transmute::<
                        *mut c_void,
                        SqlGetPrivateProfileStringFn,
                    >(sym));
                }
                // The lib loaded but doesn't export the symbol — keep
                // searching. We don't `dlclose` because some other code
                // may already be holding pointers into this handle's
                // address space (e.g. the DM's own internal lookups).
            }
            None
        })
    }

    pub fn get_private_profile_string(
        section: &str,
        entry: &str,
        filename: &str,
    ) -> Option<String> {
        let f = resolve_symbol()?;
        let section_c = CString::new(section).ok()?;
        let entry_c = CString::new(entry).ok()?;
        let default_c = CString::new("").ok()?;
        let filename_c = CString::new(filename).ok()?;
        let mut buf = vec![0u8; 512];
        loop {
            let copied = unsafe {
                f(
                    section_c.as_ptr(),
                    entry_c.as_ptr(),
                    default_c.as_ptr(),
                    buf.as_mut_ptr() as *mut c_char,
                    buf.len() as c_int,
                    filename_c.as_ptr(),
                )
            };
            if copied < 0 {
                return None;
            }
            let copied = copied as usize;
            if copied + 1 >= buf.len() {
                buf.resize(buf.len() * 2, 0);
                continue;
            }
            return std::str::from_utf8(&buf[..copied]).ok().map(str::to_owned);
        }
    }
}

pub use platform::get_private_profile_string;

/// Driver-section name baked into the install template at
/// `odbc/installer/shared/templates/odbcinst.ini.template`. Used as the
/// final fallback by [`resolve_driver_path`] when neither the connection
/// string's `DRIVER={...}` keyword nor a DSN-mediated lookup produced a
/// usable section.
const DEFAULT_DRIVER_SECTION: &str = "Snowflake ODBC";

/// Ask the dynamic loader where the shared library containing this code
/// lives on disk. On Unix uses `dladdr`; on Windows uses
/// `GetModuleHandleExW(FROM_ADDRESS)` + `GetModuleFileNameW`. Returns
/// `None` when the loader is unavailable, the call fails, or it reports
/// no path (anonymous mapping, statically linked binary, etc.).
#[cfg(not(target_os = "windows"))]
fn current_driver_path() -> Option<String> {
    use libc::{Dl_info, c_void, dladdr};

    let mut info: Dl_info = unsafe { std::mem::zeroed() };
    // SAFETY: probe with an address that is guaranteed to live inside
    // this shared library — this function itself. `dladdr` only reads
    // process metadata and the `&mut info` is exclusively borrowed.
    let ok = unsafe { dladdr(current_driver_path as *const c_void, &mut info) };
    if ok == 0 || info.dli_fname.is_null() {
        return None;
    }
    // SAFETY: `dli_fname` is a NUL-terminated string owned by the
    // dynamic loader; we copy it out immediately.
    let cstr = unsafe { std::ffi::CStr::from_ptr(info.dli_fname) };
    cstr.to_str()
        .ok()
        .map(str::to_owned)
        .filter(|s| !s.is_empty())
}

#[cfg(target_os = "windows")]
fn current_driver_path() -> Option<String> {
    use crate::setup_common::from_wide;
    use core::ffi::c_void;

    // `GetModuleHandleExW` flags. `FROM_ADDRESS` reinterprets `lpModuleName`
    // as an address and returns the handle of the module that contains it —
    // the Win32 analogue of `dladdr`. `UNCHANGED_REFCOUNT` retrieves the
    // handle *without* incrementing the module's load count, so there is no
    // handle to `FreeLibrary` afterwards. That's exactly right here: we are
    // executing inside the driver DLL, so it is already pinned for our
    // lifetime, and we must not leave a dangling reference behind.
    const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 0x0000_0004;
    const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: u32 = 0x0000_0002;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleExW(
            dw_flags: u32,
            lp_module_name: *const u16,
            ph_module: *mut *mut c_void,
        ) -> i32;
        fn GetModuleFileNameW(h_module: *mut c_void, lp_filename: *mut u16, n_size: u32) -> u32;
    }

    // Resolve the module (DLL) that contains this function's code. Using an
    // address known to live in *this* shared library — the function pointer
    // of `current_driver_path` itself — mirrors the `dladdr(current_driver_path, …)`
    // probe on Unix.
    let mut module: *mut c_void = core::ptr::null_mut();
    // SAFETY: `module` is a valid out-pointer; the address argument points
    // into this module's code section. The call only reads process module
    // metadata and writes the resolved handle into `module`.
    let ok = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            (current_driver_path as *const c_void).cast::<u16>(),
            &mut module,
        )
    };
    if ok == 0 || module.is_null() {
        return None;
    }

    let mut buf = vec![0u16; 512];
    loop {
        // SAFETY: `module` is a valid module handle from the call above and
        // `buf` is a writable buffer of `buf.len()` UTF-16 code units.
        let len = unsafe { GetModuleFileNameW(module, buf.as_mut_ptr(), buf.len() as u32) };
        if len == 0 {
            return None;
        }
        // The return value excludes the terminating NUL, so a fully-fitting
        // name is always strictly shorter than the buffer. When it equals
        // the buffer size the path was truncated (Win32 sets
        // ERROR_INSUFFICIENT_BUFFER); grow and retry rather than hand back a
        // truncated path.
        if len as usize >= buf.len() {
            buf.resize(buf.len() * 2, 0);
            continue;
        }
        return Some(from_wide(&buf[..len as usize])).filter(|s| !s.is_empty());
    }
}

/// Resolve the on-disk file path of the driver shared library — the
/// value `SQLGetInfo(SQL_DRIVER_NAME)` must return per the ODBC spec
/// ("A character string with the file name of the driver used to access
/// the data source").
///
/// Four layers are tried in order, returning the first non-empty
/// result:
///
/// 1. **Direct (`DRIVER={X}`)**: if the application passed `DRIVER={X}`
///    to `SQLDriverConnect`, read `Driver=` from `[X]` in `odbcinst.ini`.
/// 2. **DSN-mediated**: if a DSN was supplied (via `SQLConnect("Y", ...)`
///    or `SQLDriverConnect("DSN=Y;...")`), read `[Y]/Driver` from
///    `odbc.ini` to get the driver short name, then resolve that short
///    name in `odbcinst.ini` exactly as in (1).
/// 3. **Hardcoded default**: read `[Snowflake ODBC]/Driver` from
///    `odbcinst.ini` — covers connections that supply neither keyword
///    (e.g. `DRIVER={/abs/path/lib.so}` connections where there is no
///    section at all).
/// 4. **Self-path via the dynamic loader**: ask the loader for the path
///    of the shared library this code lives in — `dladdr` on Unix,
///    `GetModuleHandleExW(FROM_ADDRESS) + GetModuleFileNameW` on Windows.
///    The ODBC spec defines `SQL_DRIVER_NAME` as "the file name of the
///    driver used to access the data source" — and we _are_ the driver —
///    so the loader's answer is the most authoritative result we can
///    produce. Covers DM-less contexts (Rust unit tests, embedded use),
///    environments where the installer library can't be `dlopen`'d (no
///    `libodbcinst` on disk, sandbox restrictions, …), and Windows test
///    setups that register the driver as a User DSN without writing
///    `ODBCINST.INI` (which requires admin).
///
/// Returns an empty string only when all four layers fail. We
/// deliberately avoid surfacing an error to the caller: the ODBC spec
/// lists `SQLGetInfo(SQL_DRIVER_NAME)` as always-succeeds-when-connected,
/// so missing/malformed inis must produce an empty string rather than
/// `08003` / `HY000`.
pub fn resolve_driver_path(driver_section: Option<&str>, dsn_name: Option<&str>) -> String {
    resolve_driver_path_with(
        driver_section,
        dsn_name,
        &get_private_profile_string,
        current_driver_path,
    )
}

/// Resolve the driver's on-disk file *name* — the value
/// `SQLGetInfo(SQL_DRIVER_NAME)` must return per the ODBC spec ("A
/// character string with the file name of the driver used to access the
/// data source"). Delegates to [`resolve_driver_path`] for the full-path
/// layering, then strips the directory component so callers receive just
/// the library file name (e.g. `libsfodbc.so`) rather than its absolute
/// path (`/opt/snowflake/lib/libsfodbc.so`). Returns an empty string when
/// [`resolve_driver_path`] found nothing to report.
pub fn resolve_driver_name(driver_section: Option<&str>, dsn_name: Option<&str>) -> String {
    file_name_of(&resolve_driver_path(driver_section, dsn_name))
}

/// Return the final path component (file name) of `path`, tolerating
/// either platform's directory separator. `Path::file_name` only
/// recognises the *native* separator, but a path read from an ini file
/// can carry the other platform's separator (e.g. a Windows
/// `C:\...\sfodbc.dll` value surfacing on a Unix box), so split on both.
/// Returns `path` unchanged when it has no directory component and an
/// empty string when `path` is empty.
fn file_name_of(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

fn resolve_driver_path_with<F, G>(
    driver_section: Option<&str>,
    dsn_name: Option<&str>,
    lookup: &F,
    self_path: G,
) -> String
where
    F: Fn(&str, &str, &str) -> Option<String>,
    G: FnOnce() -> Option<String>,
{
    if let Some(section) = driver_section
        && let Some(path) = lookup(section, "Driver", "odbcinst.ini").filter(|s| !s.is_empty())
    {
        return path;
    }
    if let Some(dsn) = dsn_name
        && let Some(driver_value) = lookup(dsn, "Driver", "odbc.ini").filter(|s| !s.is_empty())
    {
        // Standard layout: value is a short driver section name; look up the
        // real DLL path in odbcinst.ini.
        if let Some(path) =
            lookup(&driver_value, "Driver", "odbcinst.ini").filter(|s| !s.is_empty())
        {
            return path;
        }
        // Non-standard layout (e.g. Windows test setups that lack admin
        // rights to write ODBCINST.INI): the DSN's Driver value is the
        // absolute DLL path itself rather than a section name.
        if std::path::Path::new(&driver_value).is_absolute() {
            return driver_value;
        }
    }
    if let Some(path) =
        lookup(DEFAULT_DRIVER_SECTION, "Driver", "odbcinst.ini").filter(|s| !s.is_empty())
    {
        return path;
    }
    self_path().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Captures (section, entry, filename) tuples and returns canned
    /// responses keyed on (section, filename). Records call order so
    /// tests can assert which layer fired.
    struct MockLookup {
        responses: std::collections::HashMap<(String, String), String>,
        calls: RefCell<Vec<(String, String, String)>>,
    }

    impl MockLookup {
        fn new() -> Self {
            Self {
                responses: Default::default(),
                calls: RefCell::new(Vec::new()),
            }
        }
        fn with(mut self, section: &str, filename: &str, value: &str) -> Self {
            self.responses.insert(
                (section.to_string(), filename.to_string()),
                value.to_string(),
            );
            self
        }
        fn calls(&self) -> Vec<(String, String, String)> {
            self.calls.borrow().clone()
        }
        /// Pass-through lookup: returns whatever was registered via
        /// [`Self::with`], or `None` for unregistered (section, filename)
        /// pairs. Crucially, empty-string responses are *not* filtered
        /// here so that
        /// [`super::tests::empty_string_responses_are_treated_as_missing`]
        /// can exercise the resolver's own empty-string check.
        fn lookup(&self) -> impl Fn(&str, &str, &str) -> Option<String> + '_ {
            |section, entry, filename| {
                self.calls.borrow_mut().push((
                    section.to_string(),
                    entry.to_string(),
                    filename.to_string(),
                ));
                self.responses
                    .get(&(section.to_string(), filename.to_string()))
                    .cloned()
            }
        }
    }

    /// Convenience wrapper that supplies a no-op self-path resolver,
    /// matching the legacy 3-layer behaviour the older tests assert.
    fn resolve_three_layers<F>(
        driver_section: Option<&str>,
        dsn_name: Option<&str>,
        lookup: &F,
    ) -> String
    where
        F: Fn(&str, &str, &str) -> Option<String>,
    {
        resolve_driver_path_with(driver_section, dsn_name, lookup, || None)
    }

    #[test]
    fn direct_driver_section_returns_first_layer_result() {
        let mock = MockLookup::new().with(
            "Snowflake ODBC",
            "odbcinst.ini",
            "/opt/snowflake/lib/libsfodbc.so",
        );
        let lookup = mock.lookup();
        let path = resolve_three_layers(Some("Snowflake ODBC"), None, &lookup);
        assert_eq!(path, "/opt/snowflake/lib/libsfodbc.so");
        assert_eq!(
            mock.calls(),
            vec![(
                "Snowflake ODBC".to_string(),
                "Driver".to_string(),
                "odbcinst.ini".to_string(),
            )]
        );
    }

    #[test]
    fn dsn_mediated_lookup_resolves_through_two_inis() {
        let mock = MockLookup::new()
            .with("MySnowflake", "odbc.ini", "Snowflake ODBC")
            .with(
                "Snowflake ODBC",
                "odbcinst.ini",
                "/opt/snowflake/lib/libsfodbc.so",
            );
        let lookup = mock.lookup();
        let path = resolve_three_layers(None, Some("MySnowflake"), &lookup);
        assert_eq!(path, "/opt/snowflake/lib/libsfodbc.so");
        let calls = mock.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].2, "odbc.ini");
        assert_eq!(calls[1].2, "odbcinst.ini");
    }

    #[test]
    fn direct_section_takes_precedence_over_dsn() {
        let mock = MockLookup::new()
            .with("CustomSection", "odbcinst.ini", "/from/direct.so")
            .with("MySnowflake", "odbc.ini", "Snowflake ODBC")
            .with("Snowflake ODBC", "odbcinst.ini", "/from/dsn.so");
        let lookup = mock.lookup();
        let path = resolve_three_layers(Some("CustomSection"), Some("MySnowflake"), &lookup);
        assert_eq!(path, "/from/direct.so");
        assert_eq!(mock.calls().len(), 1);
    }

    #[test]
    fn falls_back_to_default_section_when_no_hints() {
        let mock = MockLookup::new().with(
            "Snowflake ODBC",
            "odbcinst.ini",
            "/opt/snowflake/lib/libsfodbc.so",
        );
        let lookup = mock.lookup();
        let path = resolve_three_layers(None, None, &lookup);
        assert_eq!(path, "/opt/snowflake/lib/libsfodbc.so");
    }

    #[test]
    fn empty_string_responses_are_treated_as_missing() {
        let mock = MockLookup::new()
            .with("CustomSection", "odbcinst.ini", "")
            .with(
                "Snowflake ODBC",
                "odbcinst.ini",
                "/opt/snowflake/lib/libsfodbc.so",
            );
        let lookup = mock.lookup();
        let path = resolve_three_layers(Some("CustomSection"), None, &lookup);
        assert_eq!(path, "/opt/snowflake/lib/libsfodbc.so");
    }

    #[test]
    fn returns_empty_string_when_all_layers_including_self_path_fail() {
        let mock = MockLookup::new();
        let lookup = mock.lookup();
        let path = resolve_driver_path_with(Some("Unknown"), Some("AlsoUnknown"), &lookup, || None);
        assert_eq!(path, "");
    }

    #[test]
    fn dsn_short_name_lookup_failure_still_falls_back_to_default() {
        let mock = MockLookup::new().with(
            "Snowflake ODBC",
            "odbcinst.ini",
            "/opt/snowflake/lib/libsfodbc.so",
        );
        let lookup = mock.lookup();
        let path = resolve_three_layers(None, Some("MySnowflake"), &lookup);
        assert_eq!(path, "/opt/snowflake/lib/libsfodbc.so");
    }

    #[test]
    fn self_path_layer_answers_when_inis_are_silent() {
        // Mirrors a no-installer-lib / DM-less context: every ini
        // lookup returns nothing, but `dladdr` knows where the driver
        // dylib lives. Layer 4 must surface that path verbatim.
        let mock = MockLookup::new();
        let lookup = mock.lookup();
        let path = resolve_driver_path_with(Some("Unknown"), Some("AlsoUnknown"), &lookup, || {
            Some("/opt/snowflake/lib/libsfodbc.dylib".to_string())
        });
        assert_eq!(path, "/opt/snowflake/lib/libsfodbc.dylib");
    }

    #[test]
    fn ini_results_win_over_self_path() {
        // Self-path is a last resort: when any ini layer answers, the
        // user's configured path takes precedence so that overrides
        // (e.g. `DRIVER={...}` in the connection string) actually win.
        let mock = MockLookup::new().with(
            "Snowflake ODBC",
            "odbcinst.ini",
            "/opt/snowflake/lib/libsfodbc.so",
        );
        let lookup = mock.lookup();
        let path = resolve_driver_path_with(None, None, &lookup, || {
            Some("/should/not/be/used.dylib".to_string())
        });
        assert_eq!(path, "/opt/snowflake/lib/libsfodbc.so");
    }

    #[test]
    fn dsn_with_direct_path_is_returned_without_odbcinst_lookup() {
        // Windows test environments write the DLL path directly into the DSN's
        // Driver value instead of going through ODBCINST.INI (requires admin).
        // When odbcinst.ini has no matching section, the absolute path itself
        // must be returned.
        #[cfg(windows)]
        let direct_path = r"C:\path\to\sfodbc.dll";
        #[cfg(not(windows))]
        let direct_path = "/path/to/sfodbc.so";

        let mock = MockLookup::new().with("MySnowflake", "odbc.ini", direct_path);
        let lookup = mock.lookup();
        let path = resolve_driver_path_with(None, Some("MySnowflake"), &lookup, || None);
        assert_eq!(path, direct_path);
        // The odbcinst.ini lookup should have been attempted but found nothing.
        let calls = mock.calls();
        assert_eq!(calls.len(), 2); // odbc.ini + failed odbcinst.ini attempt
        assert_eq!(calls[1].2, "odbcinst.ini");
    }

    #[test]
    fn empty_self_path_is_not_treated_as_a_result() {
        // Guard against `dladdr` ever handing us an empty string —
        // the public `current_driver_path` already filters that, but
        // an internal change to its callers shouldn't be able to
        // resurrect the empty-string return value.
        let mock = MockLookup::new();
        let lookup = mock.lookup();
        let path = resolve_driver_path_with(None, None, &lookup, || Some(String::new()));
        // `unwrap_or_default()` on `Some("")` yields `""`, which is
        // exactly the documented "all layers failed" outcome.
        assert_eq!(path, "");
    }

    #[test]
    fn file_name_of_strips_unix_directory() {
        assert_eq!(
            file_name_of("/opt/snowflake/lib/libsfodbc.so"),
            "libsfodbc.so"
        );
    }

    #[test]
    fn file_name_of_strips_windows_directory() {
        assert_eq!(
            file_name_of(r"C:\Program Files\Snowflake ODBC\sfodbc.dll"),
            "sfodbc.dll"
        );
    }

    #[test]
    fn file_name_of_returns_bare_name_unchanged() {
        assert_eq!(file_name_of("libsfodbc.dylib"), "libsfodbc.dylib");
    }

    #[test]
    fn file_name_of_maps_empty_path_to_empty_name() {
        assert_eq!(file_name_of(""), "");
    }

    /// `current_driver_path` should always answer when run inside a
    /// `cargo test` binary — the test binary itself is a linked
    /// ELF/Mach-O/PE image and the loader (`dladdr` on Unix,
    /// `GetModuleHandleExW`/`GetModuleFileNameW` on Windows) knows where
    /// it lives.
    #[test]
    fn current_driver_path_returns_a_real_filesystem_path() {
        let path = current_driver_path().expect("loader should resolve the test binary");
        assert!(!path.is_empty(), "self path must be non-empty");
        // The reported path must be an absolute, real file. We don't
        // pin a particular extension (test binaries are executables,
        // not dylibs) — only that the loader's reported path exists.
        let p = std::path::Path::new(&path);
        assert!(
            p.is_absolute(),
            "expected absolute path from dladdr, got {path:?}"
        );
        assert!(
            p.exists(),
            "dladdr returned {path:?} but no such file on disk"
        );
    }
}
