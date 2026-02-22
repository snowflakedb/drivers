use crate::api::bitmask::Bitmask;
use crate::api::error::InvalidDescriptorKindSnafu;
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

/// ODBC information type identifiers for `SQLGetInfo`
/// (matching `SQL_*` constants from `sql.h` / `sqlext.h`).
#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InfoType {
    /// `SQL_GETDATA_EXTENSIONS` (81) — bitmask of supported GetData extensions.
    GetDataExtensions = 81,
}

impl TryFrom<u16> for InfoType {
    type Error = OdbcError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            81 => Ok(InfoType::GetDataExtensions),
            _ => {
                tracing::warn!("Unsupported info type: {value}");
                Err(OdbcError::UnknownInfoType {
                    info_type: value,
                    location: snafu::location!(),
                })
            }
        }
    }
}

/// SQL_GETDATA_EXTENSIONS bitmask values.
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum GetDataExtensions {
    /// SQL_GD_ANY_COLUMN - SQLGetData can be called for any column
    AnyColumn = 0x0000_0001,
    /// SQL_GD_ANY_ORDER - SQLGetData can be called for columns in any order
    AnyOrder = 0x0000_0002,
    /// SQL_GD_BLOCK - SQLGetData can be called for block data
    Block = 0x0000_0004,
    /// SQL_GD_BOUND - SQLGetData can be called for bound columns
    Bound = 0x0000_0008,
    /// SQL_GD_OUTPUT_PARAMS - SQLGetData can be called for output parameters
    OutputParams = 0x0000_0010,
}

impl Bitmask for GetDataExtensions {
    fn bitmask(&self) -> u32 {
        *self as u32
    }
}

/// ODBC statement attribute identifiers (matching `SQL_ATTR_*` constants from `sql.h`).
#[repr(i32)]
#[allow(clippy::enum_variant_names)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StmtAttr {
    /// `SQL_ATTR_ROW_BIND_TYPE` (5) — row-wise vs column-wise binding.
    RowBindType = 5,
    /// `SQL_ATTR_USE_BOOKMARKS` (12) — whether bookmarks are used.
    UseBookmarks = 12,
    /// `SQL_ATTR_ROW_BIND_OFFSET_PTR` (23) — binding offset pointer.
    RowBindOffsetPtr = 23,
    /// `SQL_ATTR_ROW_STATUS_PTR` (25) — pointer to per-row status array.
    RowStatusPtr = 25,
    /// `SQL_ATTR_ROWS_FETCHED_PTR` (26) — pointer to count of rows fetched.
    RowsFetchedPtr = 26,
    /// `SQL_ATTR_ROW_ARRAY_SIZE` (27) — number of rows per fetch.
    RowArraySize = 27,
    /// `SQL_ATTR_APP_ROW_DESC` — handle to the Application Row Descriptor.
    AppRowDesc = 10010,
    /// `SQL_ATTR_APP_PARAM_DESC` — handle to the Application Parameter Descriptor.
    AppParamDesc = 10011,
    /// `SQL_ATTR_IMP_ROW_DESC` — handle to the Implementation Row Descriptor.
    ImpRowDesc = 10012,
    /// `SQL_ATTR_IMP_PARAM_DESC` — handle to the Implementation Parameter Descriptor.
    ImpParamDesc = 10013,
}

impl TryFrom<i32> for StmtAttr {
    type Error = OdbcError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            5 => Ok(StmtAttr::RowBindType),
            12 => Ok(StmtAttr::UseBookmarks),
            23 => Ok(StmtAttr::RowBindOffsetPtr),
            25 => Ok(StmtAttr::RowStatusPtr),
            26 => Ok(StmtAttr::RowsFetchedPtr),
            27 => Ok(StmtAttr::RowArraySize),
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

/// ODBC descriptor field identifiers (matching `SQL_DESC_*` constants from `sql.h` / `sqlext.h`).
#[repr(i16)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DescField {
    /// `SQL_DESC_CONCISE_TYPE` (2) — concise data type of the column.
    ConciseType = 2,
    /// `SQL_DESC_ARRAY_SIZE` (20) — header: number of rows in the rowset.
    ArraySize = 20,
    /// `SQL_DESC_ARRAY_STATUS_PTR` (21) — header: pointer to row status array.
    ArrayStatusPtr = 21,
    /// `SQL_DESC_BIND_OFFSET_PTR` (24) — header: binding offset pointer.
    BindOffsetPtr = 24,
    /// `SQL_DESC_BIND_TYPE` (25) — header: row-wise vs column-wise binding.
    BindType = 25,
    /// `SQL_DESC_ROWS_PROCESSED_PTR` (34) — header: pointer to rows-processed count.
    RowsProcessedPtr = 34,
    /// `SQL_DESC_COUNT` (1001) — number of bound columns (header field, record 0).
    Count = 1001,
    /// `SQL_DESC_TYPE` (1002) — verbose data type of the column.
    Type = 1002,
    /// `SQL_DESC_OCTET_LENGTH_PTR` (1004) — pointer to the octet-length buffer.
    OctetLengthPtr = 1004,
    /// `SQL_DESC_PRECISION` (1005) — numeric precision.
    Precision = 1005,
    /// `SQL_DESC_SCALE` (1006) — numeric scale.
    Scale = 1006,
    /// `SQL_DESC_INDICATOR_PTR` (1009) — pointer to the indicator buffer.
    IndicatorPtr = 1009,
    /// `SQL_DESC_DATA_PTR` (1010) — pointer to the data buffer.
    DataPtr = 1010,
    /// `SQL_DESC_OCTET_LENGTH` (1013) — length in bytes of the data buffer.
    OctetLength = 1013,
}

impl TryFrom<i16> for DescField {
    type Error = OdbcError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            2 => Ok(DescField::ConciseType),
            20 => Ok(DescField::ArraySize),
            21 => Ok(DescField::ArrayStatusPtr),
            24 => Ok(DescField::BindOffsetPtr),
            25 => Ok(DescField::BindType),
            34 => Ok(DescField::RowsProcessedPtr),
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

/// Discriminant tag placed at offset 0 in both `ArdDescriptor` and `IrdDescriptor`
/// so that `desc_ref_from_handle` can determine the descriptor type from a raw handle.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorKind {
    Ard = 1,
    Ird = 2,
}

impl TryFrom<u8> for DescriptorKind {
    type Error = OdbcError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(DescriptorKind::Ard),
            2 => Ok(DescriptorKind::Ird),
            _ => {
                tracing::error!("Invalid descriptor kind: {}", value);
                InvalidDescriptorKindSnafu { kind: value }.fail()
            }
        }
    }
}
/// Application Row Descriptor (ARD).
///
/// Stores column binding information and block-cursor header fields.
/// A pointer to this struct is returned as the descriptor handle via
/// `SQLGetStmtAttr(SQL_ATTR_APP_ROW_DESC)`.
#[repr(C)]
pub struct ArdDescriptor {
    kind: DescriptorKind,
    pub bindings: HashMap<u16, Binding>,
    /// `SQL_DESC_ARRAY_SIZE` / `SQL_ATTR_ROW_ARRAY_SIZE` — default 1.
    pub array_size: usize,
    /// `SQL_DESC_BIND_TYPE` / `SQL_ATTR_ROW_BIND_TYPE` — 0 = column-wise (default).
    pub bind_type: sql::ULen,
    /// `SQL_DESC_BIND_OFFSET_PTR` / `SQL_ATTR_ROW_BIND_OFFSET_PTR` — default null.
    pub bind_offset_ptr: *mut sql::Len,
}

impl Default for ArdDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

impl ArdDescriptor {
    pub fn new() -> Self {
        Self {
            kind: DescriptorKind::Ard,
            bindings: HashMap::new(),
            array_size: 1,
            bind_type: 0,
            bind_offset_ptr: std::ptr::null_mut(),
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

/// Implementation Row Descriptor (IRD).
///
/// Stores per-fetch status information written by the driver.
/// A pointer to this struct is returned as the descriptor handle via
/// `SQLGetStmtAttr(SQL_ATTR_IMP_ROW_DESC)`.
#[repr(C)]
pub struct IrdDescriptor {
    kind: DescriptorKind,
    /// `SQL_DESC_ARRAY_STATUS_PTR` / `SQL_ATTR_ROW_STATUS_PTR` — default null.
    pub array_status_ptr: *mut u16,
    /// `SQL_DESC_ROWS_PROCESSED_PTR` / `SQL_ATTR_ROWS_FETCHED_PTR` — default null.
    pub rows_processed_ptr: *mut sql::ULen,
}

impl Default for IrdDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

impl IrdDescriptor {
    pub fn new() -> Self {
        Self {
            kind: DescriptorKind::Ird,
            array_status_ptr: std::ptr::null_mut(),
            rows_processed_ptr: std::ptr::null_mut(),
        }
    }
}

/// A resolved descriptor reference returned by `desc_ref_from_handle`.
pub enum DescriptorRef<'a> {
    Ard(&'a mut ArdDescriptor),
    Ird(&'a mut IrdDescriptor),
}

/// Convert a descriptor handle (returned by `SQLGetStmtAttr`) back to a
/// typed `DescriptorRef` by reading the `DescriptorKind` tag at offset 0.
pub fn desc_ref_from_handle<'a>(handle: sql::Handle) -> OdbcResult<DescriptorRef<'a>> {
    if handle.is_null() {
        return Err(OdbcError::InvalidHandle {
            location: snafu::location!(),
        });
    }
    // Read the raw discriminant byte at offset 0 and validate before
    // interpreting it as a DescriptorKind enum to avoid UB on corrupt handles.
    let raw_kind = unsafe { *(handle as *const u8) };
    let kind = DescriptorKind::try_from(raw_kind)?;
    match kind {
        DescriptorKind::Ard => {
            let desc = unsafe { &mut *(handle as *mut ArdDescriptor) };
            Ok(DescriptorRef::Ard(desc))
        }
        DescriptorKind::Ird => {
            let desc = unsafe { &mut *(handle as *mut IrdDescriptor) };
            Ok(DescriptorRef::Ird(desc))
        }
    }
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
        rows_affected: Option<i64>,
    },
    NoResultSet,
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

/// Tracks the state of a partial SQLGetData retrieval for a column.
pub enum GetDataState {
    /// All data has been returned; next call for same column returns SQL_NO_DATA.
    Completed { col: u16 },
    /// Partial retrieval in progress; offset tracks how many bytes have been
    /// returned so far.
    Partial { col: u16, offset: usize },
}

impl GetDataState {
    pub fn col(&self) -> u16 {
        match self {
            GetDataState::Completed { col } | GetDataState::Partial { col, .. } => *col,
        }
    }
}

pub struct Statement<'a> {
    pub conn: &'a mut Connection,
    pub stmt_handle: StatementHandle,
    pub state: State<StatementState>,
    pub parameter_bindings: HashMap<u16, ParameterBinding>,
    pub ard: ArdDescriptor,
    pub ird: IrdDescriptor,
    pub diagnostic_info: DiagnosticInfo,
    pub get_data_state: Option<GetDataState>,
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
