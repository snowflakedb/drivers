//! Windows ODBC Configuration Dialog
//!
//! Implements the Win32 dialog that the ODBC Administrator shows when
//! adding or modifying a DSN for the Snowflake ODBC UD driver.

#![cfg(target_os = "windows")]
#![allow(non_snake_case, non_camel_case_types)]

use std::ptr;
use std::sync::atomic::Ordering;

use crate::c_api::DLL_HINSTANCE;

// ---------------------------------------------------------------------------
// Win32 FFI
// ---------------------------------------------------------------------------

type HWND = *mut core::ffi::c_void;
type HINSTANCE = *mut core::ffi::c_void;
type WPARAM = usize;
type LPARAM = isize;
type INT_PTR = isize;
type LRESULT = isize;
type BOOL = i32;

type DLGPROC = Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> INT_PTR>;

const WM_INITDIALOG: u32 = 0x0110;
const WM_COMMAND: u32 = 0x0111;
const WM_GETTEXTLENGTH: u32 = 0x000E;

const IDOK: u16 = 1;
const IDCANCEL: u16 = 2;

const EN_CHANGE: u16 = 0x0300;

const BN_CLICKED: u16 = 0;

const MB_OK: u32 = 0x0000;
const MB_ICONEXCLAMATION: u32 = 0x0030;

const SQL_HANDLE_ENV: i16 = 1;
const SQL_HANDLE_DBC: i16 = 2;
const SQL_ATTR_ODBC_VERSION: i32 = 200;
const SQL_OV_ODBC3: isize = 3;
const SQL_SUCCESS: i16 = 0;
const SQL_SUCCESS_WITH_INFO: i16 = 1;
const SQL_NTS: i16 = -3;
const SQL_DRIVER_NOPROMPT: u16 = 0;
const SQL_NULL_HANDLE: *mut core::ffi::c_void = ptr::null_mut();

#[link(name = "user32")]
unsafe extern "system" {
    fn DialogBoxParamW(
        hInstance: HINSTANCE,
        lpTemplate: *const u16,
        hWndParent: HWND,
        lpDialogFunc: DLGPROC,
        dwInitParam: LPARAM,
    ) -> INT_PTR;
    fn EndDialog(hDlg: HWND, nResult: INT_PTR) -> BOOL;
    fn SetDlgItemTextW(hDlg: HWND, nIDDlgItem: i32, lpString: *const u16) -> BOOL;
    fn GetDlgItemTextW(hDlg: HWND, nIDDlgItem: i32, lpString: *mut u16, cchMax: i32) -> u32;
    fn EnableWindow(hWnd: HWND, bEnable: BOOL) -> BOOL;
    fn GetDlgItem(hDlg: HWND, nIDDlgItem: i32) -> HWND;
    fn SendDlgItemMessageW(
        hDlg: HWND,
        nIDDlgItem: i32,
        Msg: u32,
        wParam: WPARAM,
        lParam: LPARAM,
    ) -> LRESULT;
    fn MessageBoxW(hWnd: HWND, lpText: *const u16, lpCaption: *const u16, uType: u32) -> i32;
    fn SetWindowTextW(hWnd: HWND, lpString: *const u16) -> BOOL;
    fn GetParent(hWnd: HWND) -> HWND;
    fn GetDesktopWindow() -> HWND;
    fn GetWindowRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    fn MoveWindow(hWnd: HWND, X: i32, Y: i32, nWidth: i32, nHeight: i32, bRepaint: BOOL) -> BOOL;
    fn SetCursor(hCursor: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: *const u16) -> *mut core::ffi::c_void;
}

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
    fn SQLGetPrivateProfileStringW(
        lpszSection: *const u16,
        lpszEntry: *const u16,
        lpszDefault: *const u16,
        retBuffer: *mut u16,
        cbRetBuffer: i32,
        lpszFilename: *const u16,
    ) -> i32;
    fn SQLWriteDSNToIniW(lpszDSN: *const u16, lpszDriver: *const u16) -> BOOL;
    fn SQLWritePrivateProfileStringW(
        lpszSection: *const u16,
        lpszEntry: *const u16,
        lpszString: *const u16,
        lpszFilename: *const u16,
    ) -> BOOL;
    fn SQLValidDSNW(lpszDSN: *const u16) -> BOOL;
}

#[link(name = "odbc32")]
unsafe extern "system" {
    fn SQLAllocHandle(
        HandleType: i16,
        InputHandle: *mut core::ffi::c_void,
        OutputHandle: *mut *mut core::ffi::c_void,
    ) -> i16;
    fn SQLSetEnvAttr(
        EnvironmentHandle: *mut core::ffi::c_void,
        Attribute: i32,
        Value: *mut core::ffi::c_void,
        StringLength: i32,
    ) -> i16;
    fn SQLDriverConnectW(
        ConnectionHandle: *mut core::ffi::c_void,
        WindowHandle: HWND,
        InConnectionString: *const u16,
        StringLength1: i16,
        OutConnectionString: *mut u16,
        BufferLength: i16,
        StringLength2Ptr: *mut i16,
        DriverCompletion: u16,
    ) -> i16;
    fn SQLDisconnect(ConnectionHandle: *mut core::ffi::c_void) -> i16;
    fn SQLFreeHandle(HandleType: i16, Handle: *mut core::ffi::c_void) -> i16;
    fn SQLGetDiagRecW(
        HandleType: i16,
        Handle: *mut core::ffi::c_void,
        RecNumber: i16,
        SqlState: *mut u16,
        NativeError: *mut i32,
        MessageText: *mut u16,
        BufferLength: i16,
        TextLength: *mut i16,
    ) -> i16;
}

#[repr(C)]
struct RECT {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

// ---------------------------------------------------------------------------
// Dialog resource IDs (must match resource.h)
// ---------------------------------------------------------------------------

const IDD_CONFIG_DSN: u16 = 105;
const IDD_CONFIG_DSN_EDIT: u16 = 205;
const IDD_TESTRESULT_DIALOG: u16 = 305;

const IDC_DSNEDIT: i32 = 1001;
const IDC_UIDEDIT: i32 = 1002;
const IDC_PWDEDIT: i32 = 1003;
const IDC_HOSTEDIT: i32 = 1004;
const IDC_DBEDIT: i32 = 1007;
const IDC_SCHEMAEDIT: i32 = 1008;
const IDC_WAREHOUSEEDIT: i32 = 1009;
const IDC_ROLEEDIT: i32 = 1014;
const IDC_TRACINGEDIT: i32 = 1015;
const IDC_AUTHENTICATOREDIT: i32 = 1016;
const IDC_PROXYEDIT: i32 = 1017;
const IDC_NO_PROXYEDIT: i32 = 1018;
const IDC_TEST_BUTTON: i32 = 1019;
const IDC_TESTRESULT_EDIT: i32 = 1021;
const IDC_PRIV_KEY_FILE_EDIT: i32 = 1022;
const IDC_PRIV_KEY_FILE_PWD_EDIT: i32 = 1023;
const IDC_OAUTH_AUTHORIZATION_URL_EDIT: i32 = 1024;
const IDC_OAUTH_TOKEN_REQUEST_URL_EDIT: i32 = 1025;
const IDC_OAUTH_REDIRECT_URI_EDIT: i32 = 1026;
const IDC_OAUTH_CLIENT_ID_EDIT: i32 = 1027;
const IDC_OAUTH_CLIENT_SECRET_EDIT: i32 = 1028;
const IDC_OAUTH_SCOPE_EDIT: i32 = 1029;

/// Maps dialog control IDs to DSN registry key names.
const FIELD_MAP: &[(i32, &str)] = &[
    (IDC_UIDEDIT, "UID"),
    (IDC_HOSTEDIT, "SERVER"),
    (IDC_DBEDIT, "DATABASE"),
    (IDC_SCHEMAEDIT, "SCHEMA"),
    (IDC_WAREHOUSEEDIT, "WAREHOUSE"),
    (IDC_ROLEEDIT, "ROLE"),
    (IDC_TRACINGEDIT, "TRACING"),
    (IDC_AUTHENTICATOREDIT, "AUTHENTICATOR"),
    (IDC_PROXYEDIT, "PROXY"),
    (IDC_NO_PROXYEDIT, "NO_PROXY"),
    (IDC_PRIV_KEY_FILE_EDIT, "PRIV_KEY_FILE"),
    (IDC_PRIV_KEY_FILE_PWD_EDIT, "PRIV_KEY_FILE_PWD"),
    (IDC_OAUTH_CLIENT_ID_EDIT, "OAUTH_CLIENT_ID"),
    (IDC_OAUTH_CLIENT_SECRET_EDIT, "OAUTH_CLIENT_SECRET"),
    (IDC_OAUTH_SCOPE_EDIT, "OAUTH_SCOPE"),
    (IDC_OAUTH_AUTHORIZATION_URL_EDIT, "OAUTH_AUTHORIZATION_URL"),
    (IDC_OAUTH_TOKEN_REQUEST_URL_EDIT, "OAUTH_TOKEN_REQUEST_URL"),
    (IDC_OAUTH_REDIRECT_URI_EDIT, "OAUTH_REDIRECT_URI"),
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn from_wide(p: &[u16]) -> String {
    let len = p.iter().position(|&c| c == 0).unwrap_or(p.len());
    String::from_utf16_lossy(&p[..len])
}

#[inline]
fn loword(v: usize) -> u16 {
    v as u16
}

#[inline]
fn hiword(v: usize) -> u16 {
    (v >> 16) as u16
}

fn make_int_resource(id: u16) -> *const u16 {
    id as usize as *const u16
}

unsafe fn get_dlg_text(dlg: HWND, id: i32) -> String {
    let len = unsafe { SendDlgItemMessageW(dlg, id, WM_GETTEXTLENGTH, 0, 0) } as usize;
    if len == 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len + 1];
    unsafe { GetDlgItemTextW(dlg, id, buf.as_mut_ptr(), buf.len() as i32) };
    from_wide(&buf)
}

unsafe fn set_dlg_text(dlg: HWND, id: i32, text: &str) {
    let w = to_wide(text);
    unsafe { SetDlgItemTextW(dlg, id, w.as_ptr()) };
}

unsafe fn center_dialog(dlg: HWND) {
    let mut parent = unsafe { GetParent(dlg) };
    if parent.is_null() {
        parent = unsafe { GetDesktopWindow() };
    }
    let mut parent_rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut dlg_rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe {
        GetWindowRect(parent, &mut parent_rect);
        GetWindowRect(dlg, &mut dlg_rect);
    }
    let w = dlg_rect.right - dlg_rect.left;
    let h = dlg_rect.bottom - dlg_rect.top;
    let x = parent_rect.left + (parent_rect.right - parent_rect.left - w) / 2;
    let y = parent_rect.top + (parent_rect.bottom - parent_rect.top - h) / 2;
    unsafe { MoveWindow(dlg, x.max(0), y.max(0), w, h, 1) };
}

fn sql_succeeded(rc: i16) -> bool {
    rc == SQL_SUCCESS || rc == SQL_SUCCESS_WITH_INFO
}

// ---------------------------------------------------------------------------
// DSN registry read/write via ODBC installer API
// ---------------------------------------------------------------------------

unsafe fn read_dsn_value(dsn: &str, key: &str) -> String {
    let dsn_w = to_wide(dsn);
    let key_w = to_wide(key);
    let default_w = to_wide("");
    let filename_w = to_wide("odbc.ini");
    let mut buf = [0u16; 512];
    unsafe {
        SQLGetPrivateProfileStringW(
            dsn_w.as_ptr(),
            key_w.as_ptr(),
            default_w.as_ptr(),
            buf.as_mut_ptr(),
            buf.len() as i32,
            filename_w.as_ptr(),
        );
    }
    from_wide(&buf)
}

unsafe fn write_dsn_values(dsn: &str, driver: &str, fields: &[(String, String)]) -> bool {
    let dsn_w = to_wide(dsn);
    let driver_w = to_wide(driver);
    if unsafe { SQLWriteDSNToIniW(dsn_w.as_ptr(), driver_w.as_ptr()) } == 0 {
        return false;
    }
    let odbc_ini = to_wide("odbc.ini");
    for (key, value) in fields {
        if key.eq_ignore_ascii_case("DSN") || key.eq_ignore_ascii_case("PWD") {
            continue;
        }
        let key_w = to_wide(key);
        let val_w = to_wide(value);
        unsafe {
            SQLWritePrivateProfileStringW(
                dsn_w.as_ptr(),
                key_w.as_ptr(),
                val_w.as_ptr(),
                odbc_ini.as_ptr(),
            );
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Dialog context passed via LPARAM
// ---------------------------------------------------------------------------

struct DialogContext {
    driver: String,
    dsn: String,
    is_new: bool,
    attrs: Vec<(String, String)>,
    ok_pressed: bool,
}

// ---------------------------------------------------------------------------
// Main config dialog proc
// ---------------------------------------------------------------------------

unsafe extern "system" fn config_dialog_proc(
    dlg: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> INT_PTR {
    match msg {
        WM_INITDIALOG => {
            let ctx = unsafe { &mut *(lparam as *mut DialogContext) };
            #[cfg(target_pointer_width = "64")]
            unsafe {
                SetWindowLongPtrW(dlg, GWLP_USERDATA, lparam);
            }
            #[cfg(target_pointer_width = "32")]
            unsafe {
                SetWindowLongW(dlg, GWLP_USERDATA, lparam as i32);
            }

            unsafe { center_dialog(dlg) };

            let title = to_wide("Snowflake ODBC UD Configuration");
            unsafe { SetWindowTextW(dlg, title.as_ptr()) };

            unsafe { set_dlg_text(dlg, IDC_DSNEDIT, &ctx.dsn) };

            if !ctx.is_new && !ctx.dsn.is_empty() {
                for &(ctl_id, key) in FIELD_MAP {
                    let val = unsafe { read_dsn_value(&ctx.dsn, key) };
                    if !val.is_empty() {
                        unsafe { set_dlg_text(dlg, ctl_id, &val) };
                    }
                }
            }
            for (key, value) in &ctx.attrs {
                if key.eq_ignore_ascii_case("DSN") {
                    continue;
                }
                for &(ctl_id, field_key) in FIELD_MAP {
                    if key.eq_ignore_ascii_case(field_key) {
                        unsafe { set_dlg_text(dlg, ctl_id, value) };
                        break;
                    }
                }
            }

            unsafe { check_enable_ok(dlg) };
            1
        }
        WM_COMMAND => {
            let ctl = loword(wparam);
            let notif = hiword(wparam);

            #[cfg(target_pointer_width = "64")]
            let ctx_ptr = unsafe { GetWindowLongPtrW(dlg, GWLP_USERDATA) } as *mut DialogContext;
            #[cfg(target_pointer_width = "32")]
            let ctx_ptr = unsafe { GetWindowLongW(dlg, GWLP_USERDATA) } as *mut DialogContext;

            match ctl {
                IDOK => {
                    if !ctx_ptr.is_null() {
                        let ctx = unsafe { &mut *ctx_ptr };
                        let dsn = unsafe { get_dlg_text(dlg, IDC_DSNEDIT) };
                        let dsn_w = to_wide(&dsn);
                        if dsn.is_empty() || unsafe { SQLValidDSNW(dsn_w.as_ptr()) } == 0 {
                            let msg = to_wide("Invalid Data Source Name.");
                            let cap = to_wide("Error");
                            unsafe {
                                MessageBoxW(
                                    dlg,
                                    msg.as_ptr(),
                                    cap.as_ptr(),
                                    MB_OK | MB_ICONEXCLAMATION,
                                );
                            }
                            return 1;
                        }

                        let mut fields = Vec::new();
                        for &(ctl_id, key) in FIELD_MAP {
                            let val = unsafe { get_dlg_text(dlg, ctl_id) };
                            if !val.is_empty() {
                                fields.push((key.to_string(), val));
                            }
                        }

                        if unsafe { write_dsn_values(&dsn, &ctx.driver, &fields) } {
                            ctx.dsn = dsn;
                            ctx.ok_pressed = true;
                            unsafe { EndDialog(dlg, 1) };
                        } else {
                            let msg = to_wide("Failed to save DSN configuration.");
                            let cap = to_wide("Error");
                            unsafe {
                                MessageBoxW(
                                    dlg,
                                    msg.as_ptr(),
                                    cap.as_ptr(),
                                    MB_OK | MB_ICONEXCLAMATION,
                                );
                            }
                        }
                    }
                    1
                }
                IDCANCEL => {
                    unsafe { EndDialog(dlg, 0) };
                    1
                }
                _ if ctl == IDC_TEST_BUTTON as u16 && notif == BN_CLICKED => {
                    unsafe { do_test_connection(dlg) };
                    1
                }
                _ if (ctl == IDC_DSNEDIT as u16 || ctl == IDC_HOSTEDIT as u16)
                    && notif == EN_CHANGE =>
                {
                    unsafe { check_enable_ok(dlg) };
                    1
                }
                _ => 0,
            }
        }
        _ => 0,
    }
}

unsafe fn check_enable_ok(dlg: HWND) {
    let server = unsafe { get_dlg_text(dlg, IDC_HOSTEDIT) };
    let enable: BOOL = if server.trim().is_empty() { 0 } else { 1 };
    unsafe {
        EnableWindow(GetDlgItem(dlg, IDOK as i32), enable);
        EnableWindow(GetDlgItem(dlg, IDC_TEST_BUTTON), enable);
    }
}

// ---------------------------------------------------------------------------
// SetWindowLongPtr FFI (pointer-width dependent)
// ---------------------------------------------------------------------------

const GWLP_USERDATA: i32 = -21;

#[cfg(target_pointer_width = "64")]
#[link(name = "user32")]
unsafe extern "system" {
    fn SetWindowLongPtrW(hWnd: HWND, nIndex: i32, dwNewLong: isize) -> isize;
    fn GetWindowLongPtrW(hWnd: HWND, nIndex: i32) -> isize;
}

#[cfg(target_pointer_width = "32")]
#[link(name = "user32")]
unsafe extern "system" {
    fn SetWindowLongW(hWnd: HWND, nIndex: i32, dwNewLong: i32) -> i32;
    fn GetWindowLongW(hWnd: HWND, nIndex: i32) -> i32;
}

// ---------------------------------------------------------------------------
// Test Connection
// ---------------------------------------------------------------------------

unsafe fn do_test_connection(dlg: HWND) {
    let idc_wait: *const u16 = 32514 as *const u16; // IDC_WAIT
    let idc_arrow: *const u16 = 32512 as *const u16; // IDC_ARROW

    let cursor = unsafe { LoadCursorW(ptr::null_mut(), idc_wait) };
    if !cursor.is_null() {
        unsafe { SetCursor(cursor) };
    }

    let driver_path = std::env::var("DRIVER_PATH").unwrap_or_default();
    let server = unsafe { get_dlg_text(dlg, IDC_HOSTEDIT) };
    let uid = unsafe { get_dlg_text(dlg, IDC_UIDEDIT) };
    let pwd = unsafe { get_dlg_text(dlg, IDC_PWDEDIT) };
    let db = unsafe { get_dlg_text(dlg, IDC_DBEDIT) };
    let schema = unsafe { get_dlg_text(dlg, IDC_SCHEMAEDIT) };
    let warehouse = unsafe { get_dlg_text(dlg, IDC_WAREHOUSEEDIT) };
    let role = unsafe { get_dlg_text(dlg, IDC_ROLEEDIT) };
    let authenticator = unsafe { get_dlg_text(dlg, IDC_AUTHENTICATOREDIT) };

    let mut conn_str = format!("DRIVER={{{driver_path}}};SERVER={server}");
    if !uid.is_empty() {
        conn_str.push_str(&format!(";UID={uid}"));
    }
    if !pwd.is_empty() {
        conn_str.push_str(&format!(";PWD={pwd}"));
    }
    if !db.is_empty() {
        conn_str.push_str(&format!(";DATABASE={db}"));
    }
    if !schema.is_empty() {
        conn_str.push_str(&format!(";SCHEMA={schema}"));
    }
    if !warehouse.is_empty() {
        conn_str.push_str(&format!(";WAREHOUSE={warehouse}"));
    }
    if !role.is_empty() {
        conn_str.push_str(&format!(";ROLE={role}"));
    }
    if !authenticator.is_empty() {
        conn_str.push_str(&format!(";AUTHENTICATOR={authenticator}"));
    }

    let result = unsafe { attempt_odbc_connection(&conn_str) };

    let cursor = unsafe { LoadCursorW(ptr::null_mut(), idc_arrow) };
    if !cursor.is_null() {
        unsafe { SetCursor(cursor) };
    }

    unsafe { show_test_result(dlg, &result) };
}

unsafe fn attempt_odbc_connection(conn_str: &str) -> String {
    let mut henv: *mut core::ffi::c_void = ptr::null_mut();
    let mut hdbc: *mut core::ffi::c_void = ptr::null_mut();

    let rc = unsafe { SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &mut henv) };
    if !sql_succeeded(rc) {
        return "FAILED!\r\n\r\nCould not allocate ODBC environment handle.".to_string();
    }

    unsafe {
        SQLSetEnvAttr(henv, SQL_ATTR_ODBC_VERSION, SQL_OV_ODBC3 as *mut _, 0);
    }

    let rc = unsafe { SQLAllocHandle(SQL_HANDLE_DBC, henv, &mut hdbc) };
    if !sql_succeeded(rc) {
        unsafe { SQLFreeHandle(SQL_HANDLE_ENV, henv) };
        return "FAILED!\r\n\r\nCould not allocate ODBC connection handle.".to_string();
    }

    let conn_w = to_wide(conn_str);
    let rc = unsafe {
        SQLDriverConnectW(
            hdbc,
            ptr::null_mut(),
            conn_w.as_ptr(),
            SQL_NTS as i16,
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            SQL_DRIVER_NOPROMPT,
        )
    };

    let result = if sql_succeeded(rc) {
        unsafe { SQLDisconnect(hdbc) };
        "SUCCESS!\r\n\r\nSuccessfully connected to data source.".to_string()
    } else {
        let mut state = [0u16; 6];
        let mut native = 0i32;
        let mut msg_buf = [0u16; 1024];
        let mut msg_len = 0i16;
        unsafe {
            SQLGetDiagRecW(
                SQL_HANDLE_DBC,
                hdbc,
                1,
                state.as_mut_ptr(),
                &mut native,
                msg_buf.as_mut_ptr(),
                msg_buf.len() as i16,
                &mut msg_len,
            );
        }
        let state_str = from_wide(&state);
        let msg_str = from_wide(&msg_buf);
        format!("FAILED!\r\n\r\n[{state_str}] {msg_str}")
    };

    unsafe {
        SQLFreeHandle(SQL_HANDLE_DBC, hdbc);
        SQLFreeHandle(SQL_HANDLE_ENV, henv);
    }

    result
}

// ---------------------------------------------------------------------------
// Test result dialog
// ---------------------------------------------------------------------------

unsafe extern "system" fn test_result_proc(
    dlg: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> INT_PTR {
    match msg {
        WM_INITDIALOG => {
            unsafe { center_dialog(dlg) };
            let text = unsafe { &*(lparam as *const String) };
            unsafe { set_dlg_text(dlg, IDC_TESTRESULT_EDIT, text) };
            1
        }
        WM_COMMAND if loword(wparam) == IDOK || loword(wparam) == IDCANCEL => {
            unsafe { EndDialog(dlg, 1) };
            1
        }
        _ => 0,
    }
}

unsafe fn show_test_result(parent: HWND, result: &str) {
    let hinstance = DLL_HINSTANCE.load(Ordering::Relaxed);
    let text = result.to_string();
    unsafe {
        DialogBoxParamW(
            hinstance,
            make_int_resource(IDD_TESTRESULT_DIALOG),
            parent,
            Some(test_result_proc),
            &text as *const String as LPARAM,
        );
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Show the configuration dialog for Add or Modify DSN.
///
/// Returns `true` if the user pressed OK (and DSN was saved), `false` if cancelled.
pub(crate) unsafe fn show_config_dialog(
    hwnd_parent: HWND,
    is_add: bool,
    driver: &str,
    dsn: &str,
    attrs: &[(String, String)],
) -> bool {
    let hinstance = DLL_HINSTANCE.load(Ordering::Relaxed);
    if hinstance.is_null() {
        return false;
    }

    let dialog_id = if is_add {
        IDD_CONFIG_DSN
    } else {
        IDD_CONFIG_DSN_EDIT
    };

    let mut ctx = DialogContext {
        driver: driver.to_string(),
        dsn: dsn.to_string(),
        is_new: is_add,
        attrs: attrs.to_vec(),
        ok_pressed: false,
    };

    unsafe {
        DialogBoxParamW(
            hinstance,
            make_int_resource(dialog_id),
            hwnd_parent,
            Some(config_dialog_proc),
            &mut ctx as *mut DialogContext as LPARAM,
        );
    }

    ctx.ok_pressed
}
