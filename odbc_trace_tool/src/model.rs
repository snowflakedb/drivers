#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt;

/// ODBC return code as recorded in the trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnCode {
    Success,
    SuccessWithInfo,
    Error,
    InvalidHandle,
    NoData,
    NeedData,
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
    StringValue(String),
}

#[derive(Debug, Clone)]
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
    pub direction: Direction,
    pub function_name: String,
    pub return_code: Option<ReturnCode>,
    pub return_code_raw: Option<i32>,
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Clone)]
pub struct OdbcCall {
    pub function_name: String,
    pub input_params: Vec<Parameter>,
    pub output_params: Vec<Parameter>,
    pub return_code: ReturnCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    pub fn logical_name(&self, address: &str) -> Option<&str> {
        self.handles.get(address).map(|h| h.logical_name.as_str())
    }

    pub fn get(&self, address: &str) -> Option<&HandleInfo> {
        self.handles.get(address)
    }
}

#[derive(Debug, Clone, Default)]
pub struct TraceHeader {
    pub format: TraceFormat,
    pub started: Option<String>,
    pub driver_manager_version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraceFormat {
    #[default]
    IOdbc,
    UnixOdbc,
}

#[derive(Debug)]
pub struct TraceLog {
    pub header: TraceHeader,
    pub calls: Vec<OdbcCall>,
    pub handle_graph: HandleGraph,
}
