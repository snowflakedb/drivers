use std::collections::HashMap;
use std::ffi::{c_void, CStr, CString};
use std::ptr;

use snafu::prelude::*;
use snafu::Location;

use crate::model::{HandleType, OdbcCall, ReturnCode, TraceLog};

#[derive(Snafu, Debug)]
pub enum ReplayerError {
    #[snafu(display("Failed to load ODBC driver manager library: {detail}"))]
    LibraryLoad {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("String contains interior NUL byte: {detail}"))]
    InvalidString {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
}

type Result<T> = std::result::Result<T, ReplayerError>;

type SqlHandle = *mut c_void;
type SqlReturn = i16;
type SqlSmallInt = i16;
type SqlUSmallInt = u16;
type SqlInteger = i32;
type SqlLen = isize;
type SqlULen = usize;
type SqlChar = u8;

const SQL_HANDLE_ENV: SqlSmallInt = 1;
const SQL_HANDLE_DBC: SqlSmallInt = 2;
const SQL_HANDLE_STMT: SqlSmallInt = 3;
const SQL_NULL_HANDLE: SqlHandle = ptr::null_mut();
#[allow(dead_code)]
const SQL_ATTR_ODBC_VERSION: SqlInteger = 200;
#[allow(dead_code)]
const SQL_OV_ODBC3: isize = 3;
#[allow(dead_code)]
const SQL_NTS: SqlInteger = -3;
#[allow(dead_code)]
const SQL_DRIVER_NOPROMPT: SqlUSmallInt = 0;
#[allow(dead_code)]
const SQL_C_CHAR: SqlSmallInt = 1;

const SQL_SUCCESS: SqlReturn = 0;
const SQL_SUCCESS_WITH_INFO: SqlReturn = 1;
const SQL_NO_DATA: SqlReturn = 100;

pub struct ReplayConfig {
    pub connection_string: String,
    pub relaxed_success: bool,
}

#[derive(Debug)]
pub struct CallResult {
    pub function_name: String,
    pub expected_return_code: ReturnCode,
    pub actual_return_code: SqlReturn,
    pub passed: bool,
    pub details: Option<String>,
}

#[derive(Debug)]
pub struct ReplaySummary {
    pub results: Vec<CallResult>,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
}

impl ReplaySummary {
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }
}

#[cfg_attr(target_os = "macos", link(name = "iodbc", kind = "dylib"))]
#[cfg_attr(target_os = "linux", link(name = "odbc", kind = "dylib"))]
#[cfg_attr(target_os = "windows", link(name = "odbc32", kind = "dylib"))]
extern "C" {
    fn SQLAllocHandle(
        handle_type: SqlSmallInt,
        input_handle: SqlHandle,
        output_handle: *mut SqlHandle,
    ) -> SqlReturn;

    fn SQLSetEnvAttr(
        env_handle: SqlHandle,
        attribute: SqlInteger,
        value: *const c_void,
        string_length: SqlInteger,
    ) -> SqlReturn;

    fn SQLDriverConnect(
        dbc: SqlHandle,
        window_handle: SqlHandle,
        in_connection_string: *const SqlChar,
        string_length1: SqlSmallInt,
        out_connection_string: *mut SqlChar,
        buffer_length: SqlSmallInt,
        string_length2: *mut SqlSmallInt,
        driver_completion: SqlUSmallInt,
    ) -> SqlReturn;

    fn SQLPrepare(
        stmt: SqlHandle,
        statement_text: *const SqlChar,
        text_length: SqlInteger,
    ) -> SqlReturn;

    fn SQLExecute(stmt: SqlHandle) -> SqlReturn;

    fn SQLExecDirect(
        stmt: SqlHandle,
        statement_text: *const SqlChar,
        text_length: SqlInteger,
    ) -> SqlReturn;

    fn SQLNumResultCols(stmt: SqlHandle, column_count: *mut SqlSmallInt) -> SqlReturn;

    fn SQLRowCount(stmt: SqlHandle, row_count: *mut SqlLen) -> SqlReturn;

    fn SQLDescribeCol(
        stmt: SqlHandle,
        column_number: SqlUSmallInt,
        column_name: *mut SqlChar,
        buffer_length: SqlSmallInt,
        name_length: *mut SqlSmallInt,
        data_type: *mut SqlSmallInt,
        column_size: *mut SqlULen,
        decimal_digits: *mut SqlSmallInt,
        nullable: *mut SqlSmallInt,
    ) -> SqlReturn;

    fn SQLFetch(stmt: SqlHandle) -> SqlReturn;

    fn SQLFetchScroll(
        stmt: SqlHandle,
        fetch_orientation: SqlSmallInt,
        fetch_offset: SqlLen,
    ) -> SqlReturn;

    fn SQLGetData(
        stmt: SqlHandle,
        col_or_param_num: SqlUSmallInt,
        target_type: SqlSmallInt,
        target_value: *mut c_void,
        buffer_length: SqlLen,
        str_len_or_ind: *mut SqlLen,
    ) -> SqlReturn;

    fn SQLMoreResults(stmt: SqlHandle) -> SqlReturn;

    fn SQLCloseCursor(stmt: SqlHandle) -> SqlReturn;

    fn SQLFreeHandle(handle_type: SqlSmallInt, handle: SqlHandle) -> SqlReturn;

    fn SQLDisconnect(dbc: SqlHandle) -> SqlReturn;

    fn SQLGetInfo(
        dbc: SqlHandle,
        info_type: SqlUSmallInt,
        info_value: *mut c_void,
        buffer_length: SqlSmallInt,
        string_length: *mut SqlSmallInt,
    ) -> SqlReturn;
}

pub fn replay(trace: &TraceLog, config: &ReplayConfig) -> Result<ReplaySummary> {
    let mut ctx = ReplayContext::new(config);
    ctx.replay(trace)
}

struct ReplayContext<'a> {
    config: &'a ReplayConfig,
    handle_map: HashMap<String, SqlHandle>,
    results: Vec<CallResult>,
    skipped: usize,
}

impl<'a> ReplayContext<'a> {
    fn new(config: &'a ReplayConfig) -> Self {
        Self {
            config,
            handle_map: HashMap::new(),
            results: Vec::new(),
            skipped: 0,
        }
    }

    fn replay(&mut self, trace: &TraceLog) -> Result<ReplaySummary> {
        let env = unsafe {
            let mut h: SqlHandle = ptr::null_mut();
            let ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &mut h);
            if ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO {
                return Err(ReplayerError::LibraryLoad {
                    detail: format!("SQLAllocHandle(ENV) failed with code {ret}"),
                    location: Location::default(),
                });
            }
            let ret = SQLSetEnvAttr(h, SQL_ATTR_ODBC_VERSION, SQL_OV_ODBC3 as *const c_void, 0);
            if ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO {
                SQLFreeHandle(SQL_HANDLE_ENV, h);
                return Err(ReplayerError::LibraryLoad {
                    detail: format!("SQLSetEnvAttr(SQL_ATTR_ODBC_VERSION) failed with code {ret}"),
                    location: Location::default(),
                });
            }
            h
        };

        let dbc = unsafe {
            let mut h: SqlHandle = ptr::null_mut();
            let ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &mut h);
            if ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO {
                SQLFreeHandle(SQL_HANDLE_ENV, env);
                return Err(ReplayerError::LibraryLoad {
                    detail: format!("SQLAllocHandle(DBC) failed with code {ret}"),
                    location: Location::default(),
                });
            }
            h
        };

        for tc in &trace.calls {
            if let OdbcCall::DriverConnect(c) = &tc.call {
                if let Some(addr) = &c.handle {
                    self.handle_map.insert(addr.clone(), dbc);
                }
                break;
            }
        }

        for tc in &trace.calls {
            if matches!(
                &tc.call,
                OdbcCall::GetDiagRec(_) | OdbcCall::GetFunctions(_)
            ) {
                self.skipped += 1;
                continue;
            }
            self.replay_call(&tc.call, env, dbc)?;
        }

        unsafe {
            SQLDisconnect(dbc);
            SQLFreeHandle(SQL_HANDLE_DBC, dbc);
            SQLFreeHandle(SQL_HANDLE_ENV, env);
        }

        let passed = self.results.iter().filter(|r| r.passed).count();
        let failed = self.results.iter().filter(|r| !r.passed).count();
        let total = self.results.len();

        Ok(ReplaySummary {
            results: std::mem::take(&mut self.results),
            total,
            passed,
            failed,
            skipped: self.skipped,
        })
    }

    fn replay_call(&mut self, call: &OdbcCall, _env: SqlHandle, dbc: SqlHandle) -> Result<()> {
        match call {
            OdbcCall::DriverConnect(c) => self.replay_driver_connect(c, dbc)?,
            OdbcCall::AllocHandle(c) => self.replay_alloc_handle(c, dbc),
            OdbcCall::Prepare(c) => self.replay_prepare(c)?,
            OdbcCall::Execute(c) => self.replay_execute(c),
            OdbcCall::ExecDirect(c) => self.replay_exec_direct(c)?,
            OdbcCall::NumResultCols(c) => self.replay_num_result_cols(c),
            OdbcCall::DescribeCol(c) => self.replay_describe_col(c),
            OdbcCall::FetchScroll(c) => self.replay_fetch_scroll(c),
            OdbcCall::Fetch(c) => self.replay_fetch(c),
            OdbcCall::GetData(c) => self.replay_get_data(c),
            OdbcCall::RowCount(c) => self.replay_row_count(c),
            OdbcCall::MoreResults(c) => self.replay_more_results(c),
            OdbcCall::CloseCursor(c) => self.replay_close_cursor(c),
            OdbcCall::GetInfo(c) => self.replay_get_info(c, dbc),
            OdbcCall::FreeHandle(c) => self.replay_free_handle(c),
            OdbcCall::Disconnect(_) => {}
            _ => {
                self.skipped += 1;
            }
        }
        Ok(())
    }

    fn replay_driver_connect(
        &mut self,
        call: &crate::model::DriverConnect,
        dbc: SqlHandle,
    ) -> Result<()> {
        let conn_str = CString::new(self.config.connection_string.as_str()).map_err(|e| {
            ReplayerError::InvalidString {
                detail: format!("connection string: {e}"),
                location: Location::default(),
            }
        })?;
        let ret = unsafe {
            SQLDriverConnect(
                dbc,
                ptr::null_mut(),
                conn_str.as_ptr() as *const SqlChar,
                SQL_NTS as SqlSmallInt,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                SQL_DRIVER_NOPROMPT,
            )
        };
        self.record(call.return_code, "SQLDriverConnect", ret, true);
        Ok(())
    }

    fn replay_alloc_handle(&mut self, call: &crate::model::AllocHandle, dbc: SqlHandle) {
        let Some(ht) = call.handle_type else {
            self.skipped += 1;
            return;
        };
        if !matches!(ht, HandleType::Stmt) {
            self.skipped += 1;
            return;
        }
        let mut new_handle: SqlHandle = ptr::null_mut();
        let ret = unsafe { SQLAllocHandle(SQL_HANDLE_STMT, dbc, &mut new_handle) };
        if let Some(addr) = &call.child_handle {
            self.handle_map.insert(addr.clone(), new_handle);
        }
        self.record(call.return_code, "SQLAllocHandle", ret, false);
    }

    fn replay_prepare(&mut self, call: &crate::model::Prepare) -> Result<()> {
        let stmt = self.resolve_handle(&call.handle);
        let sql = call.sql.as_deref().unwrap_or_default();
        let sql_c = CString::new(sql).map_err(|e| ReplayerError::InvalidString {
            detail: format!("SQL statement: {e}"),
            location: Location::default(),
        })?;
        let ret = unsafe { SQLPrepare(stmt, sql_c.as_ptr() as *const SqlChar, SQL_NTS) };
        self.record(call.return_code, "SQLPrepare", ret, false);
        Ok(())
    }

    fn replay_execute(&mut self, call: &crate::model::Execute) {
        let stmt = self.resolve_handle(&call.handle);
        let ret = unsafe { SQLExecute(stmt) };
        self.record(call.return_code, "SQLExecute", ret, false);
    }

    fn replay_exec_direct(&mut self, call: &crate::model::ExecDirect) -> Result<()> {
        let stmt = self.resolve_handle(&call.handle);
        let sql = call.sql.as_deref().unwrap_or_default();
        let sql_c = CString::new(sql).map_err(|e| ReplayerError::InvalidString {
            detail: format!("SQL statement: {e}"),
            location: Location::default(),
        })?;
        let ret = unsafe { SQLExecDirect(stmt, sql_c.as_ptr() as *const SqlChar, SQL_NTS) };
        self.record(call.return_code, "SQLExecDirect", ret, false);
        Ok(())
    }

    fn replay_num_result_cols(&mut self, call: &crate::model::NumResultCols) {
        let stmt = self.resolve_handle(&call.handle);
        let mut count: SqlSmallInt = 0;
        let ret = unsafe { SQLNumResultCols(stmt, &mut count) };
        let details = call.count.and_then(|expected| {
            if i64::from(count) != expected {
                Some(format!("Column count: expected {expected}, got {count}"))
            } else {
                None
            }
        });
        self.record_with_details(call.return_code, "SQLNumResultCols", ret, false, details);
    }

    fn replay_row_count(&mut self, call: &crate::model::RowCount) {
        let stmt = self.resolve_handle(&call.handle);
        let mut count: SqlLen = 0;
        let ret = unsafe { SQLRowCount(stmt, &mut count) };
        let details = call.count.and_then(|expected| {
            if count as i64 != expected {
                Some(format!("Row count: expected {expected}, got {count}"))
            } else {
                None
            }
        });
        self.record_with_details(call.return_code, "SQLRowCount", ret, false, details);
    }

    fn replay_describe_col(&mut self, call: &crate::model::DescribeCol) {
        let stmt = self.resolve_handle(&call.handle);
        let col_num = call.column_number.unwrap_or(1) as SqlUSmallInt;
        let mut col_name = [0u8; 256];
        let mut name_len: SqlSmallInt = 0;
        let mut data_type: SqlSmallInt = 0;
        let mut col_size: SqlULen = 0;
        let mut decimal_digits: SqlSmallInt = 0;
        let mut nullable: SqlSmallInt = 0;
        let ret = unsafe {
            SQLDescribeCol(
                stmt,
                col_num,
                col_name.as_mut_ptr(),
                256,
                &mut name_len,
                &mut data_type,
                &mut col_size,
                &mut decimal_digits,
                &mut nullable,
            )
        };
        let mut mismatches = Vec::new();
        if let Some(expected_name) = &call.column_name {
            let actual = unsafe { CStr::from_ptr(col_name.as_ptr() as *const _) }
                .to_string_lossy()
                .to_string();
            if actual != *expected_name {
                mismatches.push(format!(
                    "Column name: expected '{expected_name}', got '{actual}'"
                ));
            }
        }
        let details = if mismatches.is_empty() {
            None
        } else {
            Some(mismatches.join("; "))
        };
        self.record_with_details(call.return_code, "SQLDescribeCol", ret, false, details);
    }

    fn replay_fetch_scroll(&mut self, call: &crate::model::FetchScroll) {
        let stmt = self.resolve_handle(&call.handle);
        let orientation = call.orientation.unwrap_or(1) as SqlSmallInt;
        let offset = call.offset.unwrap_or(1) as SqlLen;
        let ret = unsafe { SQLFetchScroll(stmt, orientation, offset) };
        self.record(call.return_code, "SQLFetchScroll", ret, false);
    }

    fn replay_fetch(&mut self, call: &crate::model::Fetch) {
        let stmt = self.resolve_handle(&call.handle);
        let ret = unsafe { SQLFetch(stmt) };
        self.record(call.return_code, "SQLFetch", ret, false);
    }

    fn replay_get_data(&mut self, call: &crate::model::GetData) {
        let stmt = self.resolve_handle(&call.handle);
        let col_num = call.column_number.unwrap_or(1) as SqlUSmallInt;
        let target_type = call.target_type.unwrap_or(i64::from(SQL_C_CHAR)) as SqlSmallInt;
        let buf_len = call.buffer_length.unwrap_or(1024) as SqlLen;
        let mut buffer = vec![0u8; buf_len as usize + 1];
        let mut indicator: SqlLen = 0;
        let ret = unsafe {
            SQLGetData(
                stmt,
                col_num,
                target_type,
                buffer.as_mut_ptr() as *mut c_void,
                buf_len,
                &mut indicator,
            )
        };
        let mut details = None;
        if ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO {
            if let Some(expected_str) = &call.value {
                let actual = unsafe { CStr::from_ptr(buffer.as_ptr() as *const _) }
                    .to_string_lossy()
                    .to_string();
                if actual != *expected_str {
                    details = Some(format!(
                        "Data value: expected '{expected_str}', got '{actual}'"
                    ));
                }
            }
        }
        self.record_with_details(call.return_code, "SQLGetData", ret, false, details);
    }

    fn replay_more_results(&mut self, call: &crate::model::MoreResults) {
        let stmt = self.resolve_handle(&call.handle);
        let ret = unsafe { SQLMoreResults(stmt) };
        self.record(call.return_code, "SQLMoreResults", ret, false);
    }

    fn replay_close_cursor(&mut self, call: &crate::model::CloseCursor) {
        let stmt = self.resolve_handle(&call.handle);
        let ret = unsafe { SQLCloseCursor(stmt) };
        self.record(call.return_code, "SQLCloseCursor", ret, false);
    }

    fn replay_get_info(&mut self, call: &crate::model::GetInfo, dbc: SqlHandle) {
        let info_type = call.info_type_value.unwrap_or(0) as SqlUSmallInt;
        let mut buffer = [0u8; 256];
        let mut len: SqlSmallInt = 0;
        let ret = unsafe {
            SQLGetInfo(
                dbc,
                info_type,
                buffer.as_mut_ptr() as *mut c_void,
                255,
                &mut len,
            )
        };
        self.record(call.return_code, "SQLGetInfo", ret, true);
    }

    fn replay_free_handle(&mut self, call: &crate::model::FreeHandle) {
        let Some(ht) = call.handle_type else {
            return;
        };
        if matches!(ht, HandleType::Env | HandleType::Dbc) {
            return;
        }
        if let Some(handle) = call.handle.as_ref().and_then(|a| self.handle_map.remove(a)) {
            unsafe {
                SQLFreeHandle(SQL_HANDLE_STMT, handle);
            }
        }
    }

    fn resolve_handle(&self, addr: &Option<String>) -> SqlHandle {
        addr.as_ref()
            .and_then(|a| self.handle_map.get(a))
            .copied()
            .unwrap_or(ptr::null_mut())
    }

    fn record(
        &mut self,
        expected: ReturnCode,
        function_name: &str,
        actual: SqlReturn,
        relaxed_success: bool,
    ) {
        self.record_with_details(expected, function_name, actual, relaxed_success, None);
    }

    fn record_with_details(
        &mut self,
        expected: ReturnCode,
        function_name: &str,
        actual: SqlReturn,
        relaxed_success: bool,
        details: Option<String>,
    ) {
        let passed =
            self.return_codes_match(expected, actual, relaxed_success) && details.is_none();
        self.results.push(CallResult {
            function_name: function_name.to_string(),
            expected_return_code: expected,
            actual_return_code: actual,
            passed,
            details,
        });
    }

    fn return_codes_match(&self, expected: ReturnCode, actual: SqlReturn, relaxed: bool) -> bool {
        let relaxed = relaxed || self.config.relaxed_success;
        if relaxed {
            match expected {
                ReturnCode::Success | ReturnCode::SuccessWithInfo => {
                    actual == SQL_SUCCESS || actual == SQL_SUCCESS_WITH_INFO
                }
                ReturnCode::NoData => actual == SQL_NO_DATA,
                ReturnCode::Error => actual == -1,
                ReturnCode::InvalidHandle => actual == -2,
                _ => actual == expected_code(expected),
            }
        } else {
            actual == expected_code(expected)
        }
    }
}

fn expected_code(rc: ReturnCode) -> SqlReturn {
    match rc {
        ReturnCode::Success => 0,
        ReturnCode::SuccessWithInfo => 1,
        ReturnCode::Error => -1,
        ReturnCode::InvalidHandle => -2,
        ReturnCode::NoData => 100,
        ReturnCode::NeedData => 99,
        ReturnCode::StillExecuting => 2,
    }
}

pub fn print_report(summary: &ReplaySummary) {
    println!("\n=== ODBC Trace Replay Report ===\n");
    for result in &summary.results {
        let status = if result.passed { "PASS" } else { "FAIL" };
        let actual_name = return_code_name(result.actual_return_code);
        print!(
            "[{status}] {}: expected {}, got {actual_name}",
            result.function_name, result.expected_return_code
        );
        if let Some(detail) = &result.details {
            print!(" -- {detail}");
        }
        println!();
    }
    println!("\n--- Summary ---");
    println!("Total:   {}", summary.total);
    println!("Passed:  {}", summary.passed);
    println!("Failed:  {}", summary.failed);
    println!("Skipped: {}", summary.skipped);
    if summary.all_passed() {
        println!("\nResult: ALL PASSED");
    } else {
        println!("\nResult: FAILURES DETECTED");
    }
}

fn return_code_name(code: SqlReturn) -> &'static str {
    match code {
        0 => "SQL_SUCCESS",
        1 => "SQL_SUCCESS_WITH_INFO",
        -1 => "SQL_ERROR",
        -2 => "SQL_INVALID_HANDLE",
        100 => "SQL_NO_DATA",
        99 => "SQL_NEED_DATA",
        2 => "SQL_STILL_EXECUTING",
        _ => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context(relaxed: bool) -> ReplayContext<'static> {
        static CONFIG_STRICT: ReplayConfig = ReplayConfig {
            connection_string: String::new(),
            relaxed_success: false,
        };
        static CONFIG_RELAXED: ReplayConfig = ReplayConfig {
            connection_string: String::new(),
            relaxed_success: true,
        };
        ReplayContext::new(if relaxed {
            &CONFIG_RELAXED
        } else {
            &CONFIG_STRICT
        })
    }

    #[test]
    fn test_return_codes_match_exact() {
        let ctx = make_context(false);
        assert!(ctx.return_codes_match(ReturnCode::Success, 0, false));
        assert!(ctx.return_codes_match(ReturnCode::SuccessWithInfo, 1, false));
        assert!(ctx.return_codes_match(ReturnCode::Error, -1, false));
        assert!(ctx.return_codes_match(ReturnCode::InvalidHandle, -2, false));
        assert!(ctx.return_codes_match(ReturnCode::NoData, 100, false));
        assert!(ctx.return_codes_match(ReturnCode::NeedData, 99, false));
        assert!(ctx.return_codes_match(ReturnCode::StillExecuting, 2, false));
    }

    #[test]
    fn test_return_codes_match_exact_mismatch() {
        let ctx = make_context(false);
        assert!(!ctx.return_codes_match(ReturnCode::Success, 1, false));
        assert!(!ctx.return_codes_match(ReturnCode::SuccessWithInfo, 0, false));
        assert!(!ctx.return_codes_match(ReturnCode::NoData, 0, false));
    }

    #[test]
    fn test_return_codes_match_relaxed_success_interchangeable() {
        let ctx = make_context(false);
        assert!(ctx.return_codes_match(ReturnCode::Success, 0, true));
        assert!(ctx.return_codes_match(ReturnCode::Success, 1, true));
        assert!(ctx.return_codes_match(ReturnCode::SuccessWithInfo, 0, true));
        assert!(ctx.return_codes_match(ReturnCode::SuccessWithInfo, 1, true));
    }

    #[test]
    fn test_return_codes_match_relaxed_no_data_strict() {
        let ctx = make_context(false);
        assert!(ctx.return_codes_match(ReturnCode::NoData, 100, true));
        assert!(!ctx.return_codes_match(ReturnCode::NoData, 0, true));
    }

    #[test]
    fn test_return_codes_match_config_relaxed() {
        let ctx = make_context(true);
        assert!(ctx.return_codes_match(ReturnCode::Success, 1, false));
        assert!(ctx.return_codes_match(ReturnCode::SuccessWithInfo, 0, false));
    }

    #[test]
    fn test_return_codes_match_need_data_exact() {
        let ctx = make_context(false);
        assert!(ctx.return_codes_match(ReturnCode::NeedData, 99, true));
        assert!(!ctx.return_codes_match(ReturnCode::NeedData, 0, true));
    }

    #[test]
    fn test_return_codes_match_still_executing_exact() {
        let ctx = make_context(false);
        assert!(ctx.return_codes_match(ReturnCode::StillExecuting, 2, true));
        assert!(!ctx.return_codes_match(ReturnCode::StillExecuting, 0, true));
    }
}
