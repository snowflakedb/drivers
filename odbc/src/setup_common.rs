//! Shared helpers for the ODBC Setup/Config entrypoints (`c_api::setup`)
//! and the interactive dialog (`setup_dialog`).
//!
//! Everything here talks to `odbccp32.dll` (the ODBC Installer API) and
//! performs DSN registry reads/writes.

#![cfg(target_os = "windows")]

// ---------------------------------------------------------------------------
// odbccp32.dll — ODBC Installer API (raw-dylib, no import lib needed)
// ---------------------------------------------------------------------------
#[cfg_attr(
    target_arch = "x86",
    link(
        name = "odbccp32",
        kind = "raw-dylib",
        import_name_type = "undecorated"
    )
)]
#[cfg_attr(not(target_arch = "x86"), link(name = "odbccp32", kind = "raw-dylib"))]
unsafe extern "system" {
    pub(crate) fn SQLWriteDSNToIniW(lpszDSN: *const u16, lpszDriver: *const u16) -> i32;
    pub(crate) fn SQLRemoveDSNFromIniW(lpszDSN: *const u16) -> i32;
    pub(crate) fn SQLWritePrivateProfileStringW(
        lpszSection: *const u16,
        lpszEntry: *const u16,
        lpszString: *const u16,
        lpszFilename: *const u16,
    ) -> i32;
    pub(crate) fn SQLGetPrivateProfileStringW(
        lpszSection: *const u16,
        lpszEntry: *const u16,
        lpszDefault: *const u16,
        lpszRetBuffer: *mut u16,
        cchRetBuffer: i32,
        lpszFilename: *const u16,
    ) -> i32;
    pub(crate) fn SQLValidDSNW(lpszDSN: *const u16) -> i32;
}

// ---------------------------------------------------------------------------
// String helpers
// ---------------------------------------------------------------------------

pub(crate) fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) fn from_wide(p: &[u16]) -> String {
    let len = p.iter().position(|&c| c == 0).unwrap_or(p.len());
    String::from_utf16_lossy(&p[..len])
}

// ---------------------------------------------------------------------------
// DSN registry read/write
// ---------------------------------------------------------------------------

pub(crate) unsafe fn read_dsn_value(dsn: &str, key: &str) -> String {
    let dsn_w = to_wide(dsn);
    let key_w = to_wide(key);
    let default_w = to_wide("");
    let filename_w = to_wide("odbc.ini");
    let mut buf = vec![0u16; 512];
    loop {
        let copied = unsafe {
            SQLGetPrivateProfileStringW(
                dsn_w.as_ptr(),
                key_w.as_ptr(),
                default_w.as_ptr(),
                buf.as_mut_ptr(),
                buf.len() as i32,
                filename_w.as_ptr(),
            )
        } as usize;
        if copied < buf.len().saturating_sub(1) {
            return from_wide(&buf[..copied]);
        }
        buf.resize(buf.len() * 2, 0);
    }
}

/// Write a DSN and its key/value fields to the ODBC registry.
///
/// Skips `DSN` and `PWD` keys (DSN is implicit in the section name;
/// PWD must never be persisted).  Returns `false` if any write fails.
pub(crate) unsafe fn write_dsn_values(
    dsn: &str,
    driver: &str,
    fields: &[(String, String)],
) -> bool {
    let dsn_w = to_wide(dsn);
    let driver_w = to_wide(driver);
    if unsafe { SQLWriteDSNToIniW(dsn_w.as_ptr(), driver_w.as_ptr()) } == 0 {
        return false;
    }
    let odbc_ini = to_wide("odbc.ini");
    let mut ok = true;
    for (key, value) in fields {
        if key.eq_ignore_ascii_case("DSN") || key.eq_ignore_ascii_case("PWD") {
            continue;
        }
        let key_w = to_wide(key);
        let val_w = to_wide(value);
        if unsafe {
            SQLWritePrivateProfileStringW(
                dsn_w.as_ptr(),
                key_w.as_ptr(),
                val_w.as_ptr(),
                odbc_ini.as_ptr(),
            )
        } == 0
        {
            ok = false;
        }
    }
    ok
}
