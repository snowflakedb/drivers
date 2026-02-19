use crate::api::{OdbcError, diagnostic::DiagnosticInfo};
use crate::cdata_types::CDataType;
use crate::conversion::Binding;
use crate::conversion::warning::Warnings;
use arrow::{array::RecordBatch, ffi_stream::ArrowArrayStreamReader};
use odbc_sys as sql;
use sf_core::protobuf_gen::database_driver_v1::{
    ConnectionHandle as TConnectionHandle, DatabaseHandle as TDatabaseHandle, StatementHandle,
};
use std::collections::HashMap;

/// Custom Snowflake connection attribute base.
/// Mirrors the old driver's sf_odbc.h: SQL_DRIVER_CONN_ATTR_BASE (0x4000) + 0x53
const SQL_SF_CONN_ATTR_BASE: i32 = 0x4000 + 0x53;

/// ODBC connection attributes — both standard and custom Snowflake attributes.
///
/// Numeric IDs for custom attributes match sf_odbc.h from the old driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionAttribute {
    // Standard ODBC attributes (from sql.h / sqlext.h)
    /// SQL_ATTR_AUTOCOMMIT (102)
    Autocommit,
    /// SQL_ATTR_LOGIN_TIMEOUT (103)
    LoginTimeout,
    /// SQL_ATTR_CONNECTION_TIMEOUT (113)
    ConnectionTimeout,

    // Custom Snowflake attributes (matching sf_odbc.h)
    /// SQL_SF_CONN_ATTR_PRIV_KEY — EVP_PKEY pointer (not supported in new driver)
    PrivKey,
    /// SQL_SF_CONN_ATTR_APPLICATION — Application name
    Application,
    /// SQL_SF_CONN_ATTR_PRIV_KEY_CONTENT — Private key as PEM string
    PrivKeyContent,
    /// SQL_SF_CONN_ATTR_PRIV_KEY_PASSWORD — Private key password/passphrase
    PrivKeyPassword,
    /// SQL_SF_CONN_ATTR_PRIV_KEY_BASE64 — Private key as base64-encoded string
    PrivKeyBase64,
}

impl ConnectionAttribute {
    /// Convert a raw ODBC attribute ID to a `ConnectionAttribute`.
    /// Returns `None` for unrecognized attributes.
    pub fn from_raw(value: i32) -> Option<Self> {
        match value {
            102 => Some(Self::Autocommit),
            103 => Some(Self::LoginTimeout),
            113 => Some(Self::ConnectionTimeout),
            x if x == SQL_SF_CONN_ATTR_BASE + 1 => Some(Self::PrivKey),
            x if x == SQL_SF_CONN_ATTR_BASE + 2 => Some(Self::Application),
            x if x == SQL_SF_CONN_ATTR_BASE + 3 => Some(Self::PrivKeyContent),
            x if x == SQL_SF_CONN_ATTR_BASE + 4 => Some(Self::PrivKeyPassword),
            x if x == SQL_SF_CONN_ATTR_BASE + 5 => Some(Self::PrivKeyBase64),
            _ => None,
        }
    }

    /// Check whether a raw attribute ID falls in the Snowflake custom range.
    pub fn is_snowflake_custom(raw: i32) -> bool {
        raw >= SQL_SF_CONN_ATTR_BASE
    }

    /// Convert back to the raw ODBC attribute ID.
    pub fn as_raw(&self) -> i32 {
        match self {
            Self::Autocommit => 102,
            Self::LoginTimeout => 103,
            Self::ConnectionTimeout => 113,
            Self::PrivKey => SQL_SF_CONN_ATTR_BASE + 1,
            Self::Application => SQL_SF_CONN_ATTR_BASE + 2,
            Self::PrivKeyContent => SQL_SF_CONN_ATTR_BASE + 3,
            Self::PrivKeyPassword => SQL_SF_CONN_ATTR_BASE + 4,
            Self::PrivKeyBase64 => SQL_SF_CONN_ATTR_BASE + 5,
        }
    }
}

/// ODBC statement attribute identifiers (matching `SQL_ATTR_*` constants from `sql.h`).
#[repr(i32)]
#[allow(clippy::enum_variant_names)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StmtAttr {
    /// `SQL_ATTR_APP_ROW_DESC` - handle to the Application Row Descriptor.
    AppRowDesc = 10010,
    /// `SQL_ATTR_APP_PARAM_DESC` - handle to the Application Parameter Descriptor.
    AppParamDesc = 10011,
    /// `SQL_ATTR_IMP_ROW_DESC` - handle to the Implementation Row Descriptor.
    ImpRowDesc = 10012,
    /// `SQL_ATTR_IMP_PARAM_DESC` - handle to the Implementation Parameter Descriptor.
    ImpParamDesc = 10013,
}

impl TryFrom<i32> for StmtAttr {
    type Error = OdbcError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            10010 => Ok(StmtAttr::AppRowDesc),
            10011 => Ok(StmtAttr::AppParamDesc),
            10012 => Ok(StmtAttr::ImpRowDesc),
            10013 => Ok(StmtAttr::ImpParamDesc),
            _ => {
                tracing::warn!("Unknown statement attribute: {}", value);
                Err(OdbcError::UnknownAttribute {
                    attribute: value,
                    location: snafu::location!(),
                })
            }
        }
    }
}

/// ODBC descriptor field identifiers (matching `SQL_DESC_*` constants from `sql.h`).
#[repr(i16)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DescField {
    /// `SQL_DESC_CONCISE_TYPE` - concise data type of the column.
    ConciseType = 2,
    /// `SQL_DESC_COUNT` - number of bound columns (header field, record 0).
    Count = 1001,
    /// `SQL_DESC_TYPE` - verbose data type of the column.
    Type = 1002,
    /// `SQL_DESC_OCTET_LENGTH_PTR` - pointer to the octet-length buffer.
    OctetLengthPtr = 1004,
    /// `SQL_DESC_PRECISION` - numeric precision.
    Precision = 1005,
    /// `SQL_DESC_SCALE` - numeric scale.
    Scale = 1006,
    /// `SQL_DESC_INDICATOR_PTR` - pointer to the indicator buffer.
    IndicatorPtr = 1009,
    /// `SQL_DESC_DATA_PTR` - pointer to the data buffer.
    DataPtr = 1010,
    /// `SQL_DESC_OCTET_LENGTH` - length in bytes of the data buffer.
    OctetLength = 1013,
}

impl TryFrom<i16> for DescField {
    type Error = OdbcError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            2 => Ok(DescField::ConciseType),
            1001 => Ok(DescField::Count),
            1002 => Ok(DescField::Type),
            1004 => Ok(DescField::OctetLengthPtr),
            1005 => Ok(DescField::Precision),
            1006 => Ok(DescField::Scale),
            1009 => Ok(DescField::IndicatorPtr),
            1010 => Ok(DescField::DataPtr),
            1013 => Ok(DescField::OctetLength),
            _ => {
                tracing::warn!("Unknown descriptor field identifier: {}", value);
                Err(OdbcError::UnknownAttribute {
                    attribute: value as i32,
                    location: snafu::location!(),
                })
            }
        }
    }
}

/// Application Row Descriptor (ARD).
///
/// Stores column binding information. This struct is embedded in `Statement`
/// and a pointer to it is returned as the descriptor handle via `SQLGetStmtAttr`.
#[derive(Default)]
pub struct ArdDescriptor {
    pub bindings: HashMap<u16, Binding>,
}

impl ArdDescriptor {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Returns the highest bound column number, or 0 if no columns are bound.
    pub fn desc_count(&self) -> u16 {
        self.bindings.keys().copied().max().unwrap_or(0)
    }

    /// Unbind all columns.
    pub fn unbind_all(&mut self) {
        self.bindings.clear();
    }

    pub fn set_desc_count(&mut self, count: sql::SmallInt) {
        self.bindings.retain(|&col, _| col <= count as u16);
        for col in 1..=count {
            self.bindings.entry(col as u16).or_default();
        }
    }
}

/// Convert a descriptor handle (returned by SQLGetStmtAttr) back to a `&mut ArdDescriptor`.
pub fn desc_from_handle<'a>(handle: sql::Handle) -> OdbcResult<&'a mut ArdDescriptor> {
    let desc_ptr = handle as *mut ArdDescriptor;
    if desc_ptr.is_null() {
        return Err(OdbcError::InvalidHandle {
            location: snafu::location!(),
        });
    }
    // SAFETY: We have checked that `desc_ptr` is not null. The caller is responsible
    // for ensuring that `handle` actually points to a valid `ArdDescriptor`.
    unsafe { Ok(&mut *desc_ptr) }
}

/// Result type for ODBC operations
pub type OdbcResult<T> = Result<T, OdbcError>;

pub trait ToSqlReturn {
    fn to_sql_return(self, warnings: &Warnings) -> sql::SqlReturn;
    fn to_sql_code(self) -> i16;
    fn to_sql_code_with_warnings(self, warnings: &Warnings) -> i16;
}

impl ToSqlReturn for OdbcResult<()> {
    fn to_sql_return(self, warnings: &Warnings) -> sql::SqlReturn {
        match self {
            Ok(_) => {
                if warnings.is_empty() {
                    sql::SqlReturn::SUCCESS
                } else {
                    sql::SqlReturn::SUCCESS_WITH_INFO
                }
            }
            Err(OdbcError::NoMoreData { .. }) => sql::SqlReturn::NO_DATA,
            Err(OdbcError::InvalidHandle { .. }) => sql::SqlReturn::INVALID_HANDLE,
            Err(_) => sql::SqlReturn::ERROR,
        }
    }
    fn to_sql_code(self) -> sql::RetCode {
        self.to_sql_return(&vec![]).0
    }

    fn to_sql_code_with_warnings(self, warnings: &Warnings) -> sql::RetCode {
        self.to_sql_return(warnings).0
    }
}

pub struct Environment {
    pub odbc_version: sql::Integer,
    pub diagnostic_info: DiagnosticInfo,
}

pub enum ConnectionState {
    Disconnected,
    Connected {
        #[allow(dead_code)]
        db_handle: TDatabaseHandle,
        conn_handle: TConnectionHandle,
    },
}

/// Pre-connection attributes set via SQLSetConnectAttr before connecting.
/// These are applied to the sf_core connection during driver_connect/connect.
pub type PreConnectionAttributes = HashMap<ConnectionAttribute, String>;

pub struct Connection {
    pub state: ConnectionState,
    pub diagnostic_info: DiagnosticInfo,
    /// Attributes set via SQLSetConnectAttr before the connection is established
    pub pre_connection_attrs: PreConnectionAttributes,
}

#[derive(Debug, Clone)]
pub struct ParameterBinding {
    pub parameter_type: sql::SqlDataType,
    pub value_type: CDataType,
    pub parameter_value_ptr: sql::Pointer,
    pub buffer_length: sql::Len,
    pub str_len_or_ind_ptr: *mut sql::Len,
}

pub enum StatementState {
    Created,
    Executed {
        reader: ArrowArrayStreamReader,
        rows_affected: i64,
    },
    Fetching {
        reader: ArrowArrayStreamReader,
        record_batch: RecordBatch,
        batch_idx: usize,
    },
    Done,
    Error,
}

pub struct State<T> {
    current_state: Option<T>,
}

/// # Safety
/// All public functions assume that the state is not None and leave object with current state set.
impl<T> State<T> {
    pub fn new(initial_state: T) -> Self {
        Self {
            current_state: Some(initial_state),
        }
    }

    /// # Safety
    /// This function assumes that the state is not None, make sure to call set after taking.
    fn take(&mut self) -> T {
        self.current_state.take().unwrap()
    }

    fn set(&mut self, state: T) {
        self.current_state = Some(state);
    }

    pub fn transition_or_err<R, E>(
        &mut self,
        f: impl Fn(T) -> Result<(T, R), (T, E)>,
    ) -> Result<R, E> {
        let state: T = self.take();
        match f(state) {
            Ok((next_state, result)) => {
                self.set(next_state);
                Ok(result)
            }
            Err((next_state, error)) => {
                self.set(next_state);
                Err(error)
            }
        }
    }

    pub fn as_ref(&self) -> &T {
        self.current_state.as_ref().unwrap()
    }
}

impl<T> From<T> for State<T> {
    fn from(state: T) -> Self {
        Self::new(state)
    }
}

pub trait WithState<T, R> {
    fn with_state(self, state: T) -> R;
}

impl<T, R, E> WithState<T, Result<R, (T, E)>> for Result<R, E> {
    fn with_state(self, state: T) -> Result<R, (T, E)> {
        self.map_err(|e| (state, e))
    }
}

pub struct Statement<'a> {
    pub conn: &'a mut Connection,
    pub stmt_handle: StatementHandle,
    pub state: State<StatementState>,
    pub parameter_bindings: HashMap<u16, ParameterBinding>,
    pub ard: ArdDescriptor,
    pub diagnostic_info: DiagnosticInfo,
}

// Helper functions for handle conversion
pub fn env_from_handle<'a>(handle: sql::Handle) -> &'a mut Environment {
    let env_ptr = handle as *mut Environment;
    unsafe { env_ptr.as_mut().unwrap() }
}

pub fn conn_from_handle<'a>(handle: sql::Handle) -> &'a mut Connection {
    let conn_ptr = handle as *mut Connection;
    unsafe { conn_ptr.as_mut().unwrap() }
}

pub fn stmt_from_handle<'a>(handle: sql::Handle) -> &'a mut Statement<'a> {
    let stmt_ptr = handle as *mut Statement;
    unsafe { stmt_ptr.as_mut().unwrap() }
}
