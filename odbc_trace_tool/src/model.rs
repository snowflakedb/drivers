#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// ODBC return code as recorded in the trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReturnCode {
    #[serde(rename = "SQL_SUCCESS")]
    Success,
    #[serde(rename = "SQL_SUCCESS_WITH_INFO")]
    SuccessWithInfo,
    #[serde(rename = "SQL_ERROR")]
    Error,
    #[serde(rename = "SQL_INVALID_HANDLE")]
    InvalidHandle,
    #[serde(rename = "SQL_NO_DATA")]
    NoData,
    #[serde(rename = "SQL_NEED_DATA")]
    NeedData,
    #[serde(rename = "SQL_STILL_EXECUTING")]
    StillExecuting,
}

impl ReturnCode {
    pub fn from_code_and_name(code: i32, _name: &str) -> Option<Self> {
        match code {
            0 => Some(Self::Success),
            1 => Some(Self::SuccessWithInfo),
            -1 => Some(Self::Error),
            -2 => Some(Self::InvalidHandle),
            100 => Some(Self::NoData),
            99 => Some(Self::NeedData),
            2 => Some(Self::StillExecuting),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "SQL_SUCCESS" => Some(Self::Success),
            "SQL_SUCCESS_WITH_INFO" => Some(Self::SuccessWithInfo),
            "SQL_ERROR" => Some(Self::Error),
            "SQL_INVALID_HANDLE" => Some(Self::InvalidHandle),
            "SQL_NO_DATA" => Some(Self::NoData),
            "SQL_NEED_DATA" => Some(Self::NeedData),
            "SQL_STILL_EXECUTING" => Some(Self::StillExecuting),
            _ => None,
        }
    }

    /// The C++ OdbcMatchers matcher name for this return code.
    pub fn matcher_name(&self) -> &'static str {
        match self {
            Self::Success => "IsSuccess",
            Self::SuccessWithInfo => "IsSuccessWithInfo",
            Self::Error => "IsError",
            Self::InvalidHandle => "IsInvalidHandle",
            Self::NoData => "IsNoData",
            Self::NeedData => "IsNeedData",
            Self::StillExecuting => "IsStillExecuting",
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success | Self::SuccessWithInfo)
    }
}

impl fmt::Display for ReturnCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "SQL_SUCCESS"),
            Self::SuccessWithInfo => write!(f, "SQL_SUCCESS_WITH_INFO"),
            Self::Error => write!(f, "SQL_ERROR"),
            Self::InvalidHandle => write!(f, "SQL_INVALID_HANDLE"),
            Self::NoData => write!(f, "SQL_NO_DATA"),
            Self::NeedData => write!(f, "SQL_NEED_DATA"),
            Self::StillExecuting => write!(f, "SQL_STILL_EXECUTING"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamValue {
    Integer(i64),
    NamedConstant {
        value: i64,
        name: String,
    },
    Address(String),
    NullPointer,
    OutputInteger {
        address: String,
        value: i64,
    },
    OutputNamedConstant {
        address: String,
        name: String,
    },
    OutputAddress {
        address: String,
        output_address: String,
    },
    StringValue {
        value: String,
        truncated: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub type_name: String,
    pub value: ParamValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Enter,
    Exit,
}

#[derive(Debug, Clone)]
pub struct TraceEntry {
    pub timestamp: String,
    pub thread_id: Option<String>,
    pub direction: Direction,
    pub function_name: String,
    pub return_code: Option<ReturnCode>,
    pub return_code_raw: Option<i32>,
    pub parameters: Vec<Parameter>,
    pub line_number: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracedCall {
    pub call: OdbcCall,
    pub entry_line: Option<usize>,
    pub exit_line: Option<usize>,
}

// ---------------------------------------------------------------------------
// Typed ODBC call structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocHandle {
    pub return_code: ReturnCode,
    pub handle_type: Option<HandleType>,
    pub parent_handle: Option<String>,
    pub child_handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeHandle {
    pub return_code: ReturnCode,
    pub handle_type: Option<HandleType>,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetEnvAttr {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
    pub attribute: Option<String>,
    pub value: Option<i64>,
    pub str_len: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetConnectAttr {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
    pub attribute: Option<String>,
    pub value: Option<i64>,
    pub str_len: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverConnect {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disconnect {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prepare {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
    pub sql: Option<String>,
    pub sql_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execute {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecDirect {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
    pub sql: Option<String>,
    pub sql_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumResultCols {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
    pub count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeCol {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
    pub column_number: Option<i64>,
    pub column_name: Option<String>,
    pub buffer_length: Option<i64>,
    pub data_type: Option<String>,
    pub column_size: Option<i64>,
    pub decimal_digits: Option<i64>,
    pub nullable: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fetch {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchScroll {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
    pub orientation: Option<i64>,
    pub orientation_name: Option<String>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetData {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
    pub column_number: Option<i64>,
    pub target_type: Option<i64>,
    pub target_type_name: Option<String>,
    pub buffer_length: Option<i64>,
    pub value: Option<String>,
    pub indicator: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowCount {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
    pub count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoreResults {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseCursor {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInfo {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
    pub info_type: Option<String>,
    pub info_type_value: Option<i64>,
    pub info_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDiagRec {
    pub return_code: ReturnCode,
    pub handle_type: Option<HandleType>,
    pub handle: Option<String>,
    pub rec_number: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetFunctions {
    pub return_code: ReturnCode,
    pub handle: Option<String>,
    pub function_id: Option<i64>,
}

// ---------------------------------------------------------------------------
// OdbcCall enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "function")]
pub enum OdbcCall {
    #[serde(rename = "SQLAllocHandle")]
    AllocHandle(AllocHandle),
    #[serde(rename = "SQLFreeHandle")]
    FreeHandle(FreeHandle),
    #[serde(rename = "SQLSetEnvAttr")]
    SetEnvAttr(SetEnvAttr),
    #[serde(rename = "SQLSetConnectAttr")]
    SetConnectAttr(SetConnectAttr),
    #[serde(rename = "SQLDriverConnect")]
    DriverConnect(DriverConnect),
    #[serde(rename = "SQLDisconnect")]
    Disconnect(Disconnect),
    #[serde(rename = "SQLPrepare")]
    Prepare(Prepare),
    #[serde(rename = "SQLExecute")]
    Execute(Execute),
    #[serde(rename = "SQLExecDirect")]
    ExecDirect(ExecDirect),
    #[serde(rename = "SQLNumResultCols")]
    NumResultCols(NumResultCols),
    #[serde(rename = "SQLDescribeCol")]
    DescribeCol(DescribeCol),
    #[serde(rename = "SQLFetch")]
    Fetch(Fetch),
    #[serde(rename = "SQLFetchScroll")]
    FetchScroll(FetchScroll),
    #[serde(rename = "SQLGetData")]
    GetData(GetData),
    #[serde(rename = "SQLRowCount")]
    RowCount(RowCount),
    #[serde(rename = "SQLMoreResults")]
    MoreResults(MoreResults),
    #[serde(rename = "SQLCloseCursor")]
    CloseCursor(CloseCursor),
    #[serde(rename = "SQLGetInfo")]
    GetInfo(GetInfo),
    #[serde(rename = "SQLGetDiagRec")]
    GetDiagRec(GetDiagRec),
    #[serde(rename = "SQLGetFunctions")]
    GetFunctions(GetFunctions),
}

impl OdbcCall {
    pub fn return_code(&self) -> ReturnCode {
        match self {
            Self::AllocHandle(c) => c.return_code,
            Self::FreeHandle(c) => c.return_code,
            Self::SetEnvAttr(c) => c.return_code,
            Self::SetConnectAttr(c) => c.return_code,
            Self::DriverConnect(c) => c.return_code,
            Self::Disconnect(c) => c.return_code,
            Self::Prepare(c) => c.return_code,
            Self::Execute(c) => c.return_code,
            Self::ExecDirect(c) => c.return_code,
            Self::NumResultCols(c) => c.return_code,
            Self::DescribeCol(c) => c.return_code,
            Self::Fetch(c) => c.return_code,
            Self::FetchScroll(c) => c.return_code,
            Self::GetData(c) => c.return_code,
            Self::RowCount(c) => c.return_code,
            Self::MoreResults(c) => c.return_code,
            Self::CloseCursor(c) => c.return_code,
            Self::GetInfo(c) => c.return_code,
            Self::GetDiagRec(c) => c.return_code,
            Self::GetFunctions(c) => c.return_code,
        }
    }

    pub fn function_name(&self) -> &str {
        match self {
            Self::AllocHandle(_) => "SQLAllocHandle",
            Self::FreeHandle(_) => "SQLFreeHandle",
            Self::SetEnvAttr(_) => "SQLSetEnvAttr",
            Self::SetConnectAttr(_) => "SQLSetConnectAttr",
            Self::DriverConnect(_) => "SQLDriverConnect",
            Self::Disconnect(_) => "SQLDisconnect",
            Self::Prepare(_) => "SQLPrepare",
            Self::Execute(_) => "SQLExecute",
            Self::ExecDirect(_) => "SQLExecDirect",
            Self::NumResultCols(_) => "SQLNumResultCols",
            Self::DescribeCol(_) => "SQLDescribeCol",
            Self::Fetch(_) => "SQLFetch",
            Self::FetchScroll(_) => "SQLFetchScroll",
            Self::GetData(_) => "SQLGetData",
            Self::RowCount(_) => "SQLRowCount",
            Self::MoreResults(_) => "SQLMoreResults",
            Self::CloseCursor(_) => "SQLCloseCursor",
            Self::GetInfo(_) => "SQLGetInfo",
            Self::GetDiagRec(_) => "SQLGetDiagRec",
            Self::GetFunctions(_) => "SQLGetFunctions",
        }
    }

    pub fn has_truncated_sql(&self) -> bool {
        match self {
            Self::Prepare(c) => c.sql_truncated,
            Self::ExecDirect(c) => c.sql_truncated,
            _ => false,
        }
    }

    /// The primary handle address this call operates on.
    pub fn handle_addr(&self) -> Option<&str> {
        match self {
            Self::AllocHandle(c) => c.parent_handle.as_deref(),
            Self::FreeHandle(c) => c.handle.as_deref(),
            Self::SetEnvAttr(c) => c.handle.as_deref(),
            Self::SetConnectAttr(c) => c.handle.as_deref(),
            Self::DriverConnect(c) => c.handle.as_deref(),
            Self::Disconnect(c) => c.handle.as_deref(),
            Self::Prepare(c) => c.handle.as_deref(),
            Self::Execute(c) => c.handle.as_deref(),
            Self::ExecDirect(c) => c.handle.as_deref(),
            Self::NumResultCols(c) => c.handle.as_deref(),
            Self::DescribeCol(c) => c.handle.as_deref(),
            Self::Fetch(c) => c.handle.as_deref(),
            Self::FetchScroll(c) => c.handle.as_deref(),
            Self::GetData(c) => c.handle.as_deref(),
            Self::RowCount(c) => c.handle.as_deref(),
            Self::MoreResults(c) => c.handle.as_deref(),
            Self::CloseCursor(c) => c.handle.as_deref(),
            Self::GetInfo(c) => c.handle.as_deref(),
            Self::GetDiagRec(c) => c.handle.as_deref(),
            Self::GetFunctions(c) => c.handle.as_deref(),
        }
    }

    /// Replace raw handle addresses with logical names where a mapping exists.
    pub fn resolve_handles(&mut self, map: &HashMap<String, String>) {
        fn resolve(field: &mut Option<String>, map: &HashMap<String, String>) {
            if let Some(addr) = field.as_ref() {
                if let Some(name) = map.get(addr) {
                    *field = Some(name.clone());
                }
            }
        }
        match self {
            Self::AllocHandle(c) => {
                resolve(&mut c.parent_handle, map);
                resolve(&mut c.child_handle, map);
            }
            Self::FreeHandle(c) => resolve(&mut c.handle, map),
            Self::SetEnvAttr(c) => resolve(&mut c.handle, map),
            Self::SetConnectAttr(c) => resolve(&mut c.handle, map),
            Self::DriverConnect(c) => resolve(&mut c.handle, map),
            Self::Disconnect(c) => resolve(&mut c.handle, map),
            Self::Prepare(c) => resolve(&mut c.handle, map),
            Self::Execute(c) => resolve(&mut c.handle, map),
            Self::ExecDirect(c) => resolve(&mut c.handle, map),
            Self::NumResultCols(c) => resolve(&mut c.handle, map),
            Self::DescribeCol(c) => resolve(&mut c.handle, map),
            Self::Fetch(c) => resolve(&mut c.handle, map),
            Self::FetchScroll(c) => resolve(&mut c.handle, map),
            Self::GetData(c) => resolve(&mut c.handle, map),
            Self::RowCount(c) => resolve(&mut c.handle, map),
            Self::MoreResults(c) => resolve(&mut c.handle, map),
            Self::CloseCursor(c) => resolve(&mut c.handle, map),
            Self::GetInfo(c) => resolve(&mut c.handle, map),
            Self::GetDiagRec(c) => resolve(&mut c.handle, map),
            Self::GetFunctions(c) => resolve(&mut c.handle, map),
        }
    }

    pub fn from_raw(
        function_name: &str,
        input_params: Vec<Parameter>,
        output_params: Vec<Parameter>,
        return_code: ReturnCode,
    ) -> Self {
        match function_name {
            "SQLAllocHandle" => raw::build_alloc_handle(input_params, output_params, return_code),
            "SQLFreeHandle" => raw::build_free_handle(input_params, output_params, return_code),
            "SQLSetEnvAttr" => raw::build_set_env_attr(input_params, output_params, return_code),
            "SQLSetConnectAttr" => {
                raw::build_set_connect_attr(input_params, output_params, return_code)
            }
            "SQLDriverConnect" => {
                raw::build_simple_handle_call(input_params, output_params, return_code, |h, rc| {
                    Self::DriverConnect(DriverConnect {
                        return_code: rc,
                        handle: h,
                    })
                })
            }
            "SQLDisconnect" => {
                raw::build_simple_handle_call(input_params, output_params, return_code, |h, rc| {
                    Self::Disconnect(Disconnect {
                        return_code: rc,
                        handle: h,
                    })
                })
            }
            "SQLPrepare" => raw::build_sql_call(
                input_params,
                output_params,
                return_code,
                |h, sql, trunc, rc| {
                    Self::Prepare(Prepare {
                        return_code: rc,
                        handle: h,
                        sql,
                        sql_truncated: trunc,
                    })
                },
            ),
            "SQLExecute" => {
                raw::build_simple_handle_call(input_params, output_params, return_code, |h, rc| {
                    Self::Execute(Execute {
                        return_code: rc,
                        handle: h,
                    })
                })
            }
            "SQLExecDirect" => raw::build_sql_call(
                input_params,
                output_params,
                return_code,
                |h, sql, trunc, rc| {
                    Self::ExecDirect(ExecDirect {
                        return_code: rc,
                        handle: h,
                        sql,
                        sql_truncated: trunc,
                    })
                },
            ),
            "SQLNumResultCols" => {
                raw::build_num_result_cols(input_params, output_params, return_code)
            }
            "SQLDescribeCol" => raw::build_describe_col(input_params, output_params, return_code),
            "SQLFetch" => {
                raw::build_simple_handle_call(input_params, output_params, return_code, |h, rc| {
                    Self::Fetch(Fetch {
                        return_code: rc,
                        handle: h,
                    })
                })
            }
            "SQLFetchScroll" => raw::build_fetch_scroll(input_params, output_params, return_code),
            "SQLGetData" => raw::build_get_data(input_params, output_params, return_code),
            "SQLRowCount" => raw::build_row_count(input_params, output_params, return_code),
            "SQLMoreResults" => {
                raw::build_simple_handle_call(input_params, output_params, return_code, |h, rc| {
                    Self::MoreResults(MoreResults {
                        return_code: rc,
                        handle: h,
                    })
                })
            }
            "SQLCloseCursor" => {
                raw::build_simple_handle_call(input_params, output_params, return_code, |h, rc| {
                    Self::CloseCursor(CloseCursor {
                        return_code: rc,
                        handle: h,
                    })
                })
            }
            "SQLGetInfo" => raw::build_get_info(input_params, output_params, return_code),
            "SQLGetDiagRec" => raw::build_get_diag_rec(input_params, output_params, return_code),
            "SQLGetFunctions" => raw::build_get_functions(input_params, output_params, return_code),
            _ => panic!("unsupported ODBC function: {function_name}"),
        }
    }
}

impl fmt::Display for OdbcCall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.function_name(), self.return_code())
    }
}

/// Raw-parameter extraction used by `OdbcCall::from_raw`.
mod raw {
    use super::*;

    pub fn build_alloc_handle(
        input: Vec<Parameter>,
        output: Vec<Parameter>,
        rc: ReturnCode,
    ) -> OdbcCall {
        let mut handle_type = int_or_named(&output, 0)
            .or_else(|| int_by_name(&input, "Handle Type"))
            .and_then(HandleType::from_value);
        let mut parent = addr_at(&output, 1).or_else(|| addr_by_name(&input, "Input Handle"));
        let mut child =
            output_addr_at(&output, 2).or_else(|| addr_by_name(&output, "Output Handle"));

        if handle_type.is_none() && input.is_empty() {
            if let Some(env_addr) = addr_by_name(&output, "Environment") {
                handle_type = Some(HandleType::Env);
                parent = None;
                child = Some(env_addr);
            }
        }

        OdbcCall::AllocHandle(AllocHandle {
            return_code: rc,
            handle_type,
            parent_handle: parent,
            child_handle: child,
        })
    }

    pub fn build_free_handle(
        input: Vec<Parameter>,
        output: Vec<Parameter>,
        rc: ReturnCode,
    ) -> OdbcCall {
        let handle_type = int_or_named(&output, 0)
            .or_else(|| int_by_name(&input, "Handle Type"))
            .and_then(HandleType::from_value);
        let handle = addr_at(&output, 1)
            .or_else(|| addr_by_name(&input, "Input Handle"))
            .or_else(|| first_handle_addr(&input))
            .or_else(|| first_handle_addr(&output));
        OdbcCall::FreeHandle(FreeHandle {
            return_code: rc,
            handle_type,
            handle,
        })
    }

    pub fn build_set_env_attr(
        input: Vec<Parameter>,
        output: Vec<Parameter>,
        rc: ReturnCode,
    ) -> OdbcCall {
        let handle = addr_by_name(&input, "Environment")
            .or_else(|| first_addr(&input))
            .or_else(|| first_addr(&output));
        let attribute = attr_name(&input).or_else(|| attr_name(&output));
        let value = pointer_as_int(&input).or_else(|| pointer_as_int(&output));
        let str_len = int_by_name(&input, "StrLen").or_else(|| int_by_name(&output, "StrLen"));
        OdbcCall::SetEnvAttr(SetEnvAttr {
            return_code: rc,
            handle,
            attribute,
            value,
            str_len,
        })
    }

    pub fn build_set_connect_attr(
        input: Vec<Parameter>,
        output: Vec<Parameter>,
        rc: ReturnCode,
    ) -> OdbcCall {
        let handle = addr_by_name(&input, "Connection")
            .or_else(|| first_addr(&input))
            .or_else(|| first_addr(&output));
        let attribute = attr_name(&input).or_else(|| attr_name(&output));
        let value = pointer_as_int(&input).or_else(|| pointer_as_int(&output));
        let str_len = int_by_name(&input, "StrLen").or_else(|| int_by_name(&output, "StrLen"));
        OdbcCall::SetConnectAttr(SetConnectAttr {
            return_code: rc,
            handle,
            attribute,
            value,
            str_len,
        })
    }

    pub fn build_simple_handle_call(
        input: Vec<Parameter>,
        output: Vec<Parameter>,
        rc: ReturnCode,
        make: impl FnOnce(Option<String>, ReturnCode) -> OdbcCall,
    ) -> OdbcCall {
        let handle = first_addr(&input).or_else(|| first_addr(&output));
        make(handle, rc)
    }

    pub fn build_sql_call(
        input: Vec<Parameter>,
        output: Vec<Parameter>,
        rc: ReturnCode,
        make: impl FnOnce(Option<String>, Option<String>, bool, ReturnCode) -> OdbcCall,
    ) -> OdbcCall {
        let handle = addr_by_name(&input, "Statement")
            .or_else(|| first_addr(&input))
            .or_else(|| first_addr(&output));
        let (sql, truncated) = first_string_truncated(&input)
            .or_else(|| first_string_truncated(&output))
            .unwrap_or((None, false));
        make(handle, sql, truncated, rc)
    }

    pub fn build_num_result_cols(
        input: Vec<Parameter>,
        output: Vec<Parameter>,
        rc: ReturnCode,
    ) -> OdbcCall {
        let handle = addr_by_name(&input, "Statement")
            .or_else(|| first_addr(&output))
            .or_else(|| first_addr(&input));
        let count = output_int_at(&output, 1).or_else(|| output_int_by_name(&output, "Count"));
        OdbcCall::NumResultCols(NumResultCols {
            return_code: rc,
            handle,
            count,
        })
    }

    pub fn build_row_count(
        input: Vec<Parameter>,
        output: Vec<Parameter>,
        rc: ReturnCode,
    ) -> OdbcCall {
        let handle = addr_by_name(&input, "Statement")
            .or_else(|| first_addr(&output))
            .or_else(|| first_addr(&input));
        let count = output_int_at(&output, 1).or_else(|| output_int_by_name(&output, "Row Count"));
        OdbcCall::RowCount(RowCount {
            return_code: rc,
            handle,
            count,
        })
    }

    pub fn build_describe_col(
        input: Vec<Parameter>,
        output: Vec<Parameter>,
        rc: ReturnCode,
    ) -> OdbcCall {
        let handle = addr_by_name(&input, "Statement")
            .or_else(|| first_addr(&output))
            .or_else(|| first_addr(&input));
        OdbcCall::DescribeCol(DescribeCol {
            return_code: rc,
            handle,
            column_number: int_or_named(&output, 1)
                .or_else(|| int_by_name(&input, "Column Number")),
            column_name: first_string(&output).or_else(|| string_by_name(&output, "Column Name")),
            buffer_length: int_or_named(&output, 3)
                .or_else(|| int_by_name(&input, "Buffer Length")),
            data_type: output_named_at(&output, 5)
                .or_else(|| output_int_by_name(&output, "Data Type").map(|v| v.to_string())),
            column_size: output_int_at(&output, 6)
                .or_else(|| output_int_by_name(&output, "Column Size")),
            decimal_digits: output_int_at(&output, 7)
                .or_else(|| output_int_by_name(&output, "Decimal Digits")),
            nullable: output_named_at(&output, 8)
                .or_else(|| output_int_by_name(&output, "Nullable").map(|v| v.to_string())),
        })
    }

    pub fn build_fetch_scroll(
        input: Vec<Parameter>,
        output: Vec<Parameter>,
        rc: ReturnCode,
    ) -> OdbcCall {
        let handle = addr_by_name(&input, "Statement")
            .or_else(|| first_addr(&output))
            .or_else(|| first_addr(&input));
        OdbcCall::FetchScroll(FetchScroll {
            return_code: rc,
            handle,
            orientation: int_or_named(&output, 1)
                .or_else(|| int_by_name(&input, "Fetch Orientation")),
            orientation_name: named_const_at(&output, 1)
                .or_else(|| named_const_by_name(&input, "Fetch Orientation")),
            offset: int_or_named(&output, 2).or_else(|| int_by_name(&input, "Fetch Offset")),
        })
    }

    pub fn build_get_data(
        input: Vec<Parameter>,
        output: Vec<Parameter>,
        rc: ReturnCode,
    ) -> OdbcCall {
        let handle = addr_by_name(&input, "Statement")
            .or_else(|| first_addr(&output))
            .or_else(|| first_addr(&input));
        OdbcCall::GetData(GetData {
            return_code: rc,
            handle,
            column_number: int_or_named(&output, 1)
                .or_else(|| int_by_name(&input, "Column Number")),
            target_type: int_or_named(&output, 2).or_else(|| int_by_name(&input, "Target Type")),
            target_type_name: named_const_at(&output, 2)
                .or_else(|| named_const_by_name(&input, "Target Type")),
            buffer_length: int_or_named(&output, 4)
                .or_else(|| int_by_name(&input, "Buffer Length")),
            value: first_string(&output).or_else(|| string_by_name(&output, "Buffer")),
            indicator: output_int_at(&output, 5)
                .or_else(|| output_int_by_name(&output, "Strlen Or Ind")),
        })
    }

    pub fn build_get_info(
        input: Vec<Parameter>,
        output: Vec<Parameter>,
        rc: ReturnCode,
    ) -> OdbcCall {
        let handle = addr_by_name(&input, "Connection")
            .or_else(|| first_addr(&output))
            .or_else(|| first_addr(&input));
        OdbcCall::GetInfo(GetInfo {
            return_code: rc,
            handle,
            info_type: named_const_at(&output, 1)
                .or_else(|| named_const_by_name(&input, "Info Type")),
            info_type_value: int_or_named(&output, 1).or_else(|| int_by_name(&input, "Info Type")),
            info_value: first_string(&output),
        })
    }

    pub fn build_get_diag_rec(
        input: Vec<Parameter>,
        output: Vec<Parameter>,
        rc: ReturnCode,
    ) -> OdbcCall {
        let handle_type = int_or_named(&output, 0)
            .or_else(|| int_or_named(&input, 0))
            .and_then(HandleType::from_value);
        let handle = addr_at(&output, 1)
            .or_else(|| addr_at(&input, 1))
            .or_else(|| first_addr(&input))
            .or_else(|| first_addr(&output));
        let rec_number = int_or_named(&output, 2).or_else(|| int_or_named(&input, 2));
        OdbcCall::GetDiagRec(GetDiagRec {
            return_code: rc,
            handle_type,
            handle,
            rec_number,
        })
    }

    pub fn build_get_functions(
        input: Vec<Parameter>,
        output: Vec<Parameter>,
        rc: ReturnCode,
    ) -> OdbcCall {
        let handle = first_addr(&input).or_else(|| first_addr(&output));
        let function_id = int_or_named(&output, 1).or_else(|| int_or_named(&input, 1));
        OdbcCall::GetFunctions(GetFunctions {
            return_code: rc,
            handle,
            function_id,
        })
    }

    // -- extraction helpers --

    fn int_or_named(params: &[Parameter], idx: usize) -> Option<i64> {
        params.get(idx).and_then(|p| match &p.value {
            ParamValue::Integer(v) => Some(*v),
            ParamValue::NamedConstant { value, .. } => Some(*value),
            _ => None,
        })
    }

    fn int_by_name(params: &[Parameter], name: &str) -> Option<i64> {
        params
            .iter()
            .find(|p| p.type_name == name)
            .and_then(|p| match &p.value {
                ParamValue::Integer(v) => Some(*v),
                ParamValue::NamedConstant { value, .. } => Some(*value),
                _ => None,
            })
    }

    fn addr_at(params: &[Parameter], idx: usize) -> Option<String> {
        params.get(idx).and_then(|p| match &p.value {
            ParamValue::Address(a) => Some(a.clone()),
            _ => None,
        })
    }

    fn output_addr_at(params: &[Parameter], idx: usize) -> Option<String> {
        params.get(idx).and_then(|p| match &p.value {
            ParamValue::OutputAddress { output_address, .. } => Some(output_address.clone()),
            _ => None,
        })
    }

    fn addr_by_name(params: &[Parameter], name: &str) -> Option<String> {
        params
            .iter()
            .find(|p| p.type_name == name)
            .and_then(|p| match &p.value {
                ParamValue::Address(a) => Some(a.clone()),
                ParamValue::OutputAddress { output_address, .. } => Some(output_address.clone()),
                _ => None,
            })
    }

    fn first_addr(params: &[Parameter]) -> Option<String> {
        params.iter().find_map(|p| match &p.value {
            ParamValue::Address(a) => Some(a.clone()),
            _ => None,
        })
    }

    fn first_handle_addr(params: &[Parameter]) -> Option<String> {
        params.iter().find_map(|p| {
            if let ParamValue::Address(a) = &p.value {
                let ht = matches!(
                    p.type_name.as_str(),
                    "SQLHENV"
                        | "SQLHDBC"
                        | "SQLHSTMT"
                        | "SQLHDESC"
                        | "Environment"
                        | "Connection"
                        | "Statement"
                );
                if ht {
                    Some(a.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    fn first_string(params: &[Parameter]) -> Option<String> {
        params.iter().find_map(|p| match &p.value {
            ParamValue::StringValue { value, .. } => Some(value.clone()),
            _ => None,
        })
    }

    fn first_string_truncated(params: &[Parameter]) -> Option<(Option<String>, bool)> {
        params.iter().find_map(|p| match &p.value {
            ParamValue::StringValue { value, truncated } => Some((Some(value.clone()), *truncated)),
            _ => None,
        })
    }

    fn named_const_at(params: &[Parameter], idx: usize) -> Option<String> {
        params.get(idx).and_then(|p| match &p.value {
            ParamValue::NamedConstant { name, .. } => Some(name.clone()),
            _ => None,
        })
    }

    fn output_int_by_name(params: &[Parameter], name: &str) -> Option<i64> {
        params
            .iter()
            .find(|p| p.type_name == name)
            .and_then(|p| match &p.value {
                ParamValue::OutputInteger { value, .. } => Some(*value),
                ParamValue::Integer(v) => Some(*v),
                _ => None,
            })
    }

    fn named_const_by_name(params: &[Parameter], name: &str) -> Option<String> {
        params
            .iter()
            .find(|p| p.type_name == name)
            .and_then(|p| match &p.value {
                ParamValue::NamedConstant { name, .. } => Some(name.clone()),
                _ => None,
            })
    }

    fn string_by_name(params: &[Parameter], name: &str) -> Option<String> {
        params
            .iter()
            .find(|p| p.type_name == name)
            .and_then(|p| match &p.value {
                ParamValue::StringValue { value, .. } => Some(value.clone()),
                _ => None,
            })
    }

    fn output_int_at(params: &[Parameter], idx: usize) -> Option<i64> {
        params.get(idx).and_then(|p| match &p.value {
            ParamValue::OutputInteger { value, .. } => Some(*value),
            _ => None,
        })
    }

    fn output_named_at(params: &[Parameter], idx: usize) -> Option<String> {
        params.get(idx).and_then(|p| match &p.value {
            ParamValue::OutputNamedConstant { name, .. } => Some(name.clone()),
            _ => None,
        })
    }

    fn attr_name(params: &[Parameter]) -> Option<String> {
        params
            .iter()
            .find(|p| p.type_name == "Attribute")
            .and_then(|p| match &p.value {
                ParamValue::NamedConstant { name, .. } => Some(name.clone()),
                _ => None,
            })
    }

    fn pointer_as_int(params: &[Parameter]) -> Option<i64> {
        params
            .iter()
            .find(|p| p.type_name == "Value")
            .map(|p| match &p.value {
                ParamValue::NullPointer => 0,
                ParamValue::Integer(v) => *v,
                ParamValue::Address(addr) => addr
                    .strip_prefix("0x")
                    .or_else(|| addr.strip_prefix("0X"))
                    .and_then(|h| i64::from_str_radix(h, 16).ok())
                    .unwrap_or(0),
                _ => 0,
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HandleType {
    Env,
    Dbc,
    Stmt,
    Desc,
}

impl HandleType {
    pub fn from_value(v: i64) -> Option<Self> {
        match v {
            1 => Some(Self::Env),
            2 => Some(Self::Dbc),
            3 => Some(Self::Stmt),
            4 => Some(Self::Desc),
            _ => None,
        }
    }

    pub fn sql_handle_type_constant(&self) -> &'static str {
        match self {
            Self::Env => "SQL_HANDLE_ENV",
            Self::Dbc => "SQL_HANDLE_DBC",
            Self::Stmt => "SQL_HANDLE_STMT",
            Self::Desc => "SQL_HANDLE_DESC",
        }
    }

    pub fn c_type_name(&self) -> &'static str {
        match self {
            Self::Env => "SQLHENV",
            Self::Dbc => "SQLHDBC",
            Self::Stmt => "SQLHSTMT",
            Self::Desc => "SQLHDESC",
        }
    }

    pub fn sql_null_constant(&self) -> &'static str {
        match self {
            Self::Env => "SQL_NULL_HENV",
            Self::Dbc => "SQL_NULL_HDBC",
            Self::Stmt => "SQL_NULL_HSTMT",
            Self::Desc => "SQL_NULL_HDESC",
        }
    }
}

#[derive(Debug, Clone)]
pub struct HandleInfo {
    pub handle_type: HandleType,
    pub address: String,
    pub parent_address: Option<String>,
    pub logical_name: String,
}

#[derive(Debug, Default)]
pub struct HandleGraph {
    pub handles: HashMap<String, HandleInfo>,
    counters: HashMap<HandleType, usize>,
}

impl HandleGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_alloc(
        &mut self,
        handle_type: HandleType,
        parent_address: &str,
        child_address: &str,
    ) {
        let counter = self.counters.entry(handle_type).or_insert(0);
        let prefix = match handle_type {
            HandleType::Env => "env",
            HandleType::Dbc => "dbc",
            HandleType::Stmt => "stmt",
            HandleType::Desc => "desc",
        };
        let logical_name = format!("{prefix}{counter}");
        *counter += 1;

        self.handles.insert(
            child_address.to_string(),
            HandleInfo {
                handle_type,
                address: child_address.to_string(),
                parent_address: Some(parent_address.to_string()),
                logical_name,
            },
        );
    }

    pub fn register_free(&mut self, address: &str) {
        self.handles.remove(address);
    }

    pub fn logical_name(&self, address: &str) -> Option<&str> {
        self.handles.get(address).map(|h| h.logical_name.as_str())
    }

    pub fn get(&self, address: &str) -> Option<&HandleInfo> {
        self.handles.get(address)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraceHeader {
    pub format: TraceFormat,
    pub started: Option<String>,
    pub driver_manager_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TraceFormat {
    #[default]
    #[serde(rename = "iodbc")]
    IOdbc,
    #[serde(rename = "unixodbc")]
    UnixOdbc,
}

#[derive(Debug)]
pub struct TraceLog {
    pub header: TraceHeader,
    pub calls: Vec<TracedCall>,
    pub handle_graph: HandleGraph,
}
