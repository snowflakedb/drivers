use crate::api::bitmask::Bitmask;
use crate::api::error::InvalidDescriptorKindSnafu;
use crate::api::{OdbcError, diagnostic::DiagnosticInfo};
use crate::conversion::Binding;
use crate::conversion::warning::Warnings;
use crate::conversion::{NumericSettings, SF_DEFAULT_VARCHAR_MAX_LEN};
use arrow::{array::RecordBatch, datatypes::SchemaRef, ffi_stream::ArrowArrayStreamReader};
use odbc_sys as sql;
use sf_core::protobuf::generated::database_driver_v1::{
    ConnectionHandle as TConnectionHandle, DatabaseHandle as TDatabaseHandle, StatementHandle,
};
use std::collections::HashMap;
use std::sync::Weak;
use tokio_util::sync::CancellationToken;

use super::CDataType;

/// SQL_ATTR_ACCESS_MODE values (ODBC spec: SQLUINTEGER).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    /// SQL_MODE_READ_WRITE (0) — default
    ReadWrite = 0,
    /// SQL_MODE_READ_ONLY (1)
    ReadOnly = 1,
}

impl AccessMode {
    pub fn from_raw(val: sql::UInteger) -> Option<Self> {
        match val {
            0 => Some(Self::ReadWrite),
            1 => Some(Self::ReadOnly),
            _ => None,
        }
    }

    pub fn as_raw(self) -> sql::UInteger {
        self as sql::UInteger
    }
}

/// SQL_ATTR_AUTOCOMMIT values (ODBC spec: SQLUINTEGER).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutocommitValue {
    /// SQL_AUTOCOMMIT_OFF (0)
    Off = 0,
    /// SQL_AUTOCOMMIT_ON (1) — default
    On = 1,
}

impl AutocommitValue {
    pub fn from_raw(val: sql::UInteger) -> Option<Self> {
        match val {
            0 => Some(Self::Off),
            1 => Some(Self::On),
            _ => None,
        }
    }

    pub fn as_raw(self) -> sql::UInteger {
        self as sql::UInteger
    }
}

/// Custom Snowflake connection attribute base.
/// Mirrors the old driver's sf_odbc.h: SQL_DRIVER_CONN_ATTR_BASE (0x4000) + 0x53
const SQL_SF_CONN_ATTR_BASE: i32 = 0x4000 + 0x53;

/// ODBC connection attributes — both standard and custom Snowflake attributes.
///
/// Numeric IDs for custom attributes match sf_odbc.h from the old driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionAttribute {
    // Standard ODBC attributes (from sql.h / sqlext.h)
    /// SQL_ATTR_ACCESS_MODE (101)
    AccessMode,
    /// SQL_ATTR_AUTOCOMMIT (102)
    Autocommit,
    /// SQL_ATTR_LOGIN_TIMEOUT (103)
    LoginTimeout,
    /// SQL_ATTR_TXN_ISOLATION (108)
    TxnIsolation,
    /// SQL_ATTR_CURRENT_CATALOG (109)
    CurrentCatalog,
    /// SQL_ATTR_QUIET_MODE (111)
    QuietMode,
    /// SQL_ATTR_PACKET_SIZE (112)
    PacketSize,
    /// SQL_ATTR_CONNECTION_TIMEOUT (113)
    ConnectionTimeout,
    /// SQL_ATTR_CONNECTION_DEAD (1209) — read-only
    ConnectionDead,
    /// SQL_ATTR_AUTO_IPD (10001) — read-only
    AutoIpd,

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
            101 => Some(Self::AccessMode),
            102 => Some(Self::Autocommit),
            103 => Some(Self::LoginTimeout),
            108 => Some(Self::TxnIsolation),
            109 => Some(Self::CurrentCatalog),
            111 => Some(Self::QuietMode),
            112 => Some(Self::PacketSize),
            113 => Some(Self::ConnectionTimeout),
            1209 => Some(Self::ConnectionDead),
            10001 => Some(Self::AutoIpd),
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
            Self::AccessMode => 101,
            Self::Autocommit => 102,
            Self::LoginTimeout => 103,
            Self::TxnIsolation => 108,
            Self::CurrentCatalog => 109,
            Self::QuietMode => 111,
            Self::PacketSize => 112,
            Self::ConnectionTimeout => 113,
            Self::ConnectionDead => 1209,
            Self::AutoIpd => 10001,
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
    /// `SQL_CURSOR_COMMIT_BEHAVIOR` (23) — cursor behavior on commit.
    CursorCommitBehavior = 23,
    /// `SQL_CURSOR_ROLLBACK_BEHAVIOR` (24) — cursor behavior on rollback.
    CursorRollbackBehavior = 24,
    /// `SQL_DRIVER_ODBC_VER` (77) — ODBC version the driver conforms to (string).
    DriverOdbcVer = 77,
    /// `SQL_GETDATA_EXTENSIONS` (81) — bitmask of supported GetData extensions.
    GetDataExtensions = 81,
}

impl TryFrom<u16> for InfoType {
    type Error = OdbcError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            23 => Ok(InfoType::CursorCommitBehavior),
            24 => Ok(InfoType::CursorRollbackBehavior),
            77 => Ok(InfoType::DriverOdbcVer),
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

/// ODBC cursor type values (matching `SQL_CURSOR_*` constants from `sql.h`).
#[repr(u64)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CursorType {
    /// `SQL_CURSOR_FORWARD_ONLY` (0) — sequential access only.
    ForwardOnly = 0,
    /// `SQL_CURSOR_KEYSET_DRIVEN` (1) — keyset-driven cursor.
    KeysetDriven = 1,
    /// `SQL_CURSOR_DYNAMIC` (2) — dynamic cursor.
    Dynamic = 2,
    /// `SQL_CURSOR_STATIC` (3) — static cursor.
    Static = 3,
}

impl TryFrom<sql::ULen> for CursorType {
    type Error = OdbcError;

    fn try_from(value: sql::ULen) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CursorType::ForwardOnly),
            1 => Ok(CursorType::KeysetDriven),
            2 => Ok(CursorType::Dynamic),
            3 => Ok(CursorType::Static),
            _ => {
                tracing::warn!("Unsupported cursor type: {}", value);
                Err(OdbcError::UnknownAttribute {
                    attribute: value as i32,
                    location: snafu::location!(),
                })
            }
        }
    }
}

/// ODBC statement attribute identifiers (matching `SQL_ATTR_*` constants from `sql.h`).
#[repr(i32)]
#[allow(clippy::enum_variant_names)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StmtAttr {
    /// `SQL_ATTR_MAX_LENGTH` (3) — maximum amount of data returned from character/binary columns.
    MaxLength = 3,
    /// `SQL_ATTR_ROW_BIND_TYPE` (5) — row-wise vs column-wise binding.
    RowBindType = 5,
    /// `SQL_ATTR_CURSOR_TYPE` (6) — type of cursor.
    CursorType = 6,
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
    /// `SQL_SF_STMT_ATTR_LAST_QUERY_ID` (2000100) — last query ID (read-only string).
    SnowflakeLastQueryId = 2000100,
}

impl TryFrom<i32> for StmtAttr {
    type Error = OdbcError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            3 => Ok(StmtAttr::MaxLength),
            5 => Ok(StmtAttr::RowBindType),
            6 => Ok(StmtAttr::CursorType),
            12 => Ok(StmtAttr::UseBookmarks),
            23 => Ok(StmtAttr::RowBindOffsetPtr),
            25 => Ok(StmtAttr::RowStatusPtr),
            26 => Ok(StmtAttr::RowsFetchedPtr),
            27 => Ok(StmtAttr::RowArraySize),
            10010 => Ok(StmtAttr::AppRowDesc),
            10011 => Ok(StmtAttr::AppParamDesc),
            10012 => Ok(StmtAttr::ImpRowDesc),
            10013 => Ok(StmtAttr::ImpParamDesc),
            2000100 => Ok(StmtAttr::SnowflakeLastQueryId),
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
    /// `SQL_DESC_DATETIME_INTERVAL_PRECISION` (26) — leading precision for interval C types.
    DatetimeIntervalPrecision = 26,
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
    /// `SQL_DESC_PARAMETER_TYPE` (33) — parameter direction (IPD only).
    ParameterType = 33,
    /// `SQL_DESC_TYPE_NAME` (14) — data-source-dependent type name.
    TypeName = 14,
    /// `SQL_DESC_NULLABLE` (1008) — whether the parameter is nullable (IPD only).
    Nullable = 1008,
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
            26 => Ok(DescField::DatetimeIntervalPrecision),
            34 => Ok(DescField::RowsProcessedPtr),
            1001 => Ok(DescField::Count),
            1002 => Ok(DescField::Type),
            14 => Ok(DescField::TypeName),
            1004 => Ok(DescField::OctetLengthPtr),
            1005 => Ok(DescField::Precision),
            1006 => Ok(DescField::Scale),
            1008 => Ok(DescField::Nullable),
            1009 => Ok(DescField::IndicatorPtr),
            1010 => Ok(DescField::DataPtr),
            1013 => Ok(DescField::OctetLength),
            33 => Ok(DescField::ParameterType),
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
    Apd = 3,
    Ipd = 4,
}

impl TryFrom<u8> for DescriptorKind {
    type Error = OdbcError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(DescriptorKind::Ard),
            2 => Ok(DescriptorKind::Ird),
            3 => Ok(DescriptorKind::Apd),
            4 => Ok(DescriptorKind::Ipd),
            _ => {
                tracing::error!("Invalid descriptor kind: {}", value);
                InvalidDescriptorKindSnafu { kind: value }.fail()
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FreeStmtOption {
    Close,
    Unbind,
    ResetParams,
}

impl TryFrom<u16> for FreeStmtOption {
    type Error = OdbcError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FreeStmtOption::Close),
            2 => Ok(FreeStmtOption::Unbind),
            3 => Ok(FreeStmtOption::ResetParams),
            _ => {
                tracing::warn!("Invalid FreeStmt option: {value}");
                Err(OdbcError::InvalidFreeStmtOption {
                    option: value,
                    location: snafu::location!(),
                })
            }
        }
    }
}

/// ODBC parameter direction, used in `SQLBindParameter` and the IPD's
/// `SQL_DESC_PARAMETER_TYPE` field.
///
/// Source: `sqlext.h` —
/// <https://github.com/microsoft/ODBC-Specification/blob/master/Windows/inc/sqlext.h>
#[repr(i16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamDirection {
    Input = 1,       // SQL_PARAM_INPUT
    InputOutput = 2, // SQL_PARAM_INPUT_OUTPUT
    ResultCol = 3,   // SQL_RESULT_COL (IPD only, not typical for SQLBindParameter)
    Output = 4,      // SQL_PARAM_OUTPUT
    ReturnValue = 5, // SQL_RETURN_VALUE (stored procedure return values)
}

impl TryFrom<sql::SmallInt> for ParamDirection {
    type Error = OdbcError;

    fn try_from(value: sql::SmallInt) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(ParamDirection::Input),
            2 => Ok(ParamDirection::InputOutput),
            3 => Ok(ParamDirection::ResultCol),
            4 => Ok(ParamDirection::Output),
            5 => Ok(ParamDirection::ReturnValue),
            _ => {
                tracing::error!("Invalid parameter direction: {value}");
                Err(OdbcError::InvalidParameterType {
                    value,
                    location: snafu::location!(),
                })
            }
        }
    }
}

/// ODBC SQL data type identifier.
///
/// Source: Microsoft ODBC Specification headers —
/// <https://github.com/microsoft/ODBC-Specification/tree/master/Windows/inc>
#[repr(i16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlType {
    // sql.h — core types
    Char = 1,     // SQL_CHAR
    Numeric = 2,  // SQL_NUMERIC
    Decimal = 3,  // SQL_DECIMAL
    Integer = 4,  // SQL_INTEGER
    SmallInt = 5, // SQL_SMALLINT
    Float = 6,    // SQL_FLOAT
    Real = 7,     // SQL_REAL
    Double = 8,   // SQL_DOUBLE
    DateTime = 9, // SQL_DATETIME (header code for date/time subcodes)
    Varchar = 12, // SQL_VARCHAR

    // sqlext.h — ODBC 2.x backward-compatible types
    Interval = 10,     // SQL_INTERVAL (header code for interval subcodes)
    ExtTimestamp = 11, // ODBC 2.x SQL_TIMESTAMP, superseded by SQL_TYPE_TIMESTAMP (93)

    // sql.h — ODBC 3.x datetime shortcuts
    TypeDate = 91,                  // SQL_TYPE_DATE
    TypeTime = 92,                  // SQL_TYPE_TIME
    TypeTimestamp = 93,             // SQL_TYPE_TIMESTAMP
    TypeTimeWithTimezone = 94,      // SQL_TYPE_TIME_WITH_TIMEZONE (ODBC 4.0)
    TypeTimestampWithTimezone = 95, // SQL_TYPE_TIMESTAMP_WITH_TIMEZONE (ODBC 4.0)

    // sqlext.h — extended types
    LongVarchar = -1,   // SQL_LONGVARCHAR
    Binary = -2,        // SQL_BINARY
    VarBinary = -3,     // SQL_VARBINARY
    LongVarBinary = -4, // SQL_LONGVARBINARY
    BigInt = -5,        // SQL_BIGINT
    TinyInt = -6,       // SQL_TINYINT
    Bit = -7,           // SQL_BIT

    // sqlucode.h — wide-character types
    WChar = -8,         // SQL_WCHAR
    WVarchar = -9,      // SQL_WVARCHAR
    WLongVarchar = -10, // SQL_WLONGVARCHAR

    // sqlext.h
    Guid = -11, // SQL_GUID

    // sqlext.h — ODBC 3.x interval types (100 + subcode)
    IntervalYear = 101,
    IntervalMonth = 102,
    IntervalDay = 103,
    IntervalHour = 104,
    IntervalMinute = 105,
    IntervalSecond = 106,
    IntervalYearToMonth = 107,
    IntervalDayToHour = 108,
    IntervalDayToMinute = 109,
    IntervalDayToSecond = 110,
    IntervalHourToMinute = 111,
    IntervalHourToSecond = 112,
    IntervalMinuteToSecond = 113,
}

impl TryFrom<sql::SmallInt> for SqlType {
    type Error = OdbcError;

    fn try_from(value: sql::SmallInt) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(SqlType::Char),
            2 => Ok(SqlType::Numeric),
            3 => Ok(SqlType::Decimal),
            4 => Ok(SqlType::Integer),
            5 => Ok(SqlType::SmallInt),
            6 => Ok(SqlType::Float),
            7 => Ok(SqlType::Real),
            8 => Ok(SqlType::Double),
            9 => Ok(SqlType::DateTime),
            10 => Ok(SqlType::Interval),
            11 => Ok(SqlType::ExtTimestamp),
            12 => Ok(SqlType::Varchar),
            91 => Ok(SqlType::TypeDate),
            92 => Ok(SqlType::TypeTime),
            93 => Ok(SqlType::TypeTimestamp),
            94 => Ok(SqlType::TypeTimeWithTimezone),
            95 => Ok(SqlType::TypeTimestampWithTimezone),
            -1 => Ok(SqlType::LongVarchar),
            -2 => Ok(SqlType::Binary),
            -3 => Ok(SqlType::VarBinary),
            -4 => Ok(SqlType::LongVarBinary),
            -5 => Ok(SqlType::BigInt),
            -6 => Ok(SqlType::TinyInt),
            -7 => Ok(SqlType::Bit),
            -8 => Ok(SqlType::WChar),
            -9 => Ok(SqlType::WVarchar),
            -10 => Ok(SqlType::WLongVarchar),
            -11 => Ok(SqlType::Guid),
            101 => Ok(SqlType::IntervalYear),
            102 => Ok(SqlType::IntervalMonth),
            103 => Ok(SqlType::IntervalDay),
            104 => Ok(SqlType::IntervalHour),
            105 => Ok(SqlType::IntervalMinute),
            106 => Ok(SqlType::IntervalSecond),
            107 => Ok(SqlType::IntervalYearToMonth),
            108 => Ok(SqlType::IntervalDayToHour),
            109 => Ok(SqlType::IntervalDayToMinute),
            110 => Ok(SqlType::IntervalDayToSecond),
            111 => Ok(SqlType::IntervalHourToMinute),
            112 => Ok(SqlType::IntervalHourToSecond),
            113 => Ok(SqlType::IntervalMinuteToSecond),
            _ => {
                tracing::error!("Invalid SQL data type: {value}");
                Err(OdbcError::InvalidSqlDataType {
                    value,
                    location: snafu::location!(),
                })
            }
        }
    }
}

impl From<SqlType> for sql::SqlDataType {
    fn from(value: SqlType) -> Self {
        sql::SqlDataType(value as i16)
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

/// Application Parameter Descriptor (APD).
///
/// Stores parameter binding information from the application's perspective:
/// C data types, data buffer pointers, and indicator pointers.
/// A pointer to this struct is returned as the descriptor handle via
/// `SQLGetStmtAttr(SQL_ATTR_APP_PARAM_DESC)`.
#[repr(C)]
pub struct ApdDescriptor {
    kind: DescriptorKind,
    pub records: HashMap<u16, ApdRecord>,
    /// `SQL_DESC_ARRAY_SIZE` — number of parameter sets (default 1).
    pub array_size: usize,
    /// `SQL_DESC_BIND_TYPE` — 0 = column-wise (default).
    pub bind_type: sql::ULen,
    /// `SQL_DESC_BIND_OFFSET_PTR` — default null.
    pub bind_offset_ptr: *mut sql::Len,
}

impl Default for ApdDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

impl ApdDescriptor {
    pub fn new() -> Self {
        Self {
            kind: DescriptorKind::Apd,
            records: HashMap::new(),
            array_size: 1,
            bind_type: 0,
            bind_offset_ptr: std::ptr::null_mut(),
        }
    }

    pub fn desc_count(&self) -> u16 {
        self.records.keys().copied().max().unwrap_or(0)
    }

    pub fn clear(&mut self) {
        self.records.clear();
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
    /// `SQL_DESC_COUNT` — number of columns in the result set.
    pub desc_count: sql::SmallInt,
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
            desc_count: 0,
            array_status_ptr: std::ptr::null_mut(),
            rows_processed_ptr: std::ptr::null_mut(),
        }
    }
}

/// Implementation Parameter Descriptor (IPD).
///
/// Stores the implementation-side view of bound parameters: SQL data types,
/// precision, scale, and parameter direction.
/// A pointer to this struct is returned as the descriptor handle via
/// `SQLGetStmtAttr(SQL_ATTR_IMP_PARAM_DESC)`.
#[repr(C)]
pub struct IpdDescriptor {
    kind: DescriptorKind,
    pub records: HashMap<u16, IpdRecord>,
    /// `SQL_DESC_ARRAY_STATUS_PTR` — default null.
    pub array_status_ptr: *mut u16,
    /// `SQL_DESC_ROWS_PROCESSED_PTR` — default null.
    pub rows_processed_ptr: *mut sql::ULen,
}

impl Default for IpdDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

impl IpdDescriptor {
    pub fn new() -> Self {
        Self {
            kind: DescriptorKind::Ipd,
            records: HashMap::new(),
            array_status_ptr: std::ptr::null_mut(),
            rows_processed_ptr: std::ptr::null_mut(),
        }
    }

    pub fn desc_count(&self) -> u16 {
        self.records.keys().copied().max().unwrap_or(0)
    }
}

/// A resolved descriptor reference returned by `desc_ref_from_handle`.
pub enum DescriptorRef<'a> {
    Ard(&'a mut ArdDescriptor),
    Ird(&'a mut IrdDescriptor),
    Apd(&'a mut ApdDescriptor),
    Ipd(&'a mut IpdDescriptor),
}

/// Convert a descriptor handle (returned by `SQLGetStmtAttr`) back to a
/// typed `DescriptorRef` by reading the `DescriptorKind` tag at offset 0.
pub fn desc_ref_from_handle<'a>(handle: sql::Handle) -> OdbcResult<DescriptorRef<'a>> {
    if handle.is_null() {
        return Err(OdbcError::InvalidHandle {
            location: snafu::location!(),
        });
    }
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
        DescriptorKind::Apd => {
            let desc = unsafe { &mut *(handle as *mut ApdDescriptor) };
            Ok(DescriptorRef::Apd(desc))
        }
        DescriptorKind::Ipd => {
            let desc = unsafe { &mut *(handle as *mut IpdDescriptor) };
            Ok(DescriptorRef::Ipd(desc))
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
    pub connection_pooling: sql::AttrConnectionPooling,
    pub connection_pool_match: sql::AttrCpMatch,
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
    pub numeric_settings: NumericSettings,
    /// SQL_ATTR_ACCESS_MODE — advisory only (default SQL_MODE_READ_WRITE)
    pub access_mode: AccessMode,
    /// SQL_ATTR_QUIET_MODE — window handle pointer (default null)
    pub quiet_mode: sql::Pointer,
    /// SQL_ATTR_PACKET_SIZE — pre-connect only (default 0 = driver-defined)
    pub packet_size: sql::UInteger,
    /// Weak references to all child statements allocated on this connection, paired with
    /// the raw pointer obtained from `Arc::into_raw` at allocation time.
    /// Storing this pointer ensures `Arc::from_raw` (used in `free_connection`) receives a
    /// pointer that satisfies its documented contract (obtained via `Arc::into_raw`, not
    /// `Weak::as_ptr`).
    pub(crate) child_statements: Vec<(Weak<Statement>, *const Statement)>,
    /// Cached local autocommit state. Defaults to `AutocommitValue::On`.
    /// Updated when SQL_ATTR_AUTOCOMMIT is set; used as fallback for get_connect_attr
    /// when the server session parameter is unavailable.
    pub cached_autocommit: AutocommitValue,
    /// Cached SQL_ATTR_CURRENT_CATALOG value. Populated after connect and updated
    /// after each successful USE DATABASE (SET). SQLGetConnectAttr always refreshes
    /// this from the server per spec; the field is used to track the catalog for
    /// internal purposes (e.g. logging, future optimizations).
    pub current_catalog: Option<String>,
}

// Safety: Send is required so that the async runtime can transfer ownership of the
// Connection allocation between threads (e.g. when a Tokio task completes on a
// different thread than it started). All ODBC API access remains serialised on the
// single caller thread — the raw-pointer DBC handle is never shared across threads.
// `*const Statement` inside `child_statements` is `!Send`, but Connection is never
// accessed concurrently, so this is safe.
unsafe impl Send for Connection {}

/// Application Parameter Descriptor (APD) record.
///
/// Stores the application-side view of a bound parameter: the C data type,
/// the pointer to the application's data buffer, its length, and the
/// indicator/length pointer. Populated by `SQLBindParameter` or
/// `SQLSetDescField` on the APD handle.
#[derive(Debug)]
pub struct ApdRecord {
    pub value_type: CDataType,
    pub data_ptr: sql::Pointer,
    pub buffer_length: sql::Len,
    pub str_len_or_ind_ptr: *mut sql::Len,
}

impl Default for ApdRecord {
    fn default() -> Self {
        Self {
            value_type: CDataType::Default,
            data_ptr: std::ptr::null_mut(),
            buffer_length: 0,
            str_len_or_ind_ptr: std::ptr::null_mut(),
        }
    }
}

/// Implementation Parameter Descriptor (IPD) record.
///
/// Stores the implementation-side view of a bound parameter: the SQL data type,
/// column size, decimal digits, and parameter direction. Populated by
/// `SQLBindParameter` or `SQLSetDescField` on the IPD handle.
#[derive(Debug)]
pub struct IpdRecord {
    pub sql_data_type: sql::SqlDataType,
    pub column_size: sql::ULen,
    pub decimal_digits: sql::SmallInt,
    pub direction: sql::SmallInt,
    pub nullable: sql::SmallInt,
}

impl IpdRecord {
    /// Create a default IPD record for an untyped `?` marker, using the
    /// server-provided max VARCHAR size as `column_size`.
    pub fn with_varchar_size(max_varchar_size: u64) -> Self {
        Self {
            sql_data_type: sql::SqlDataType::VARCHAR,
            column_size: max_varchar_size.min(sql::ULen::MAX as u64) as sql::ULen,
            decimal_digits: 0,
            direction: sql::ParamType::Input as sql::SmallInt,
            nullable: 1, // SQL_NULLABLE — per ODBC spec
        }
    }
}

impl Default for IpdRecord {
    fn default() -> Self {
        Self::with_varchar_size(SF_DEFAULT_VARCHAR_MAX_LEN)
    }
}

/// Combined view of APD + IPD records, reconstructed at execution time
/// for consumption by the parameter conversion pipeline.
#[derive(Debug, Clone)]
pub struct ParameterBinding {
    pub sql_data_type: sql::SqlDataType,
    pub value_type: CDataType,
    pub parameter_value_ptr: sql::Pointer,
    pub buffer_length: sql::Len,
    pub str_len_or_ind_ptr: *mut sql::Len,
}

impl ParameterBinding {
    pub fn from_apd_ipd(apd: &ApdRecord, ipd: &IpdRecord) -> Self {
        Self {
            sql_data_type: ipd.sql_data_type,
            value_type: apd.value_type,
            parameter_value_ptr: apd.data_ptr,
            buffer_length: apd.buffer_length,
            str_len_or_ind_ptr: apd.str_len_or_ind_ptr,
        }
    }
}

pub enum StatementState {
    Created,
    Prepared {
        schema: SchemaRef,
    },
    /// ODBC state S5: SELECT/catalog function executed, cursor is open.
    QueryExecuted {
        reader: ArrowArrayStreamReader,
        rows_affected: Option<i64>,
        /// `true` when reached via `SQLExecute` (prepared path). On
        /// `SQLFreeStmt(SQL_CLOSE)` the state returns to `Prepared`;
        /// when `false` (exec-direct path) it returns to `Created`.
        prepared: bool,
    },
    /// ODBC state S4 for DDL. No cursor opened; SQLRowCount returns -1.
    DdlExecuted {
        schema: SchemaRef,
        prepared: bool,
    },
    /// ODBC state S4 for DML (INSERT/UPDATE/DELETE/MERGE).
    /// No cursor opened; SQLRowCount returns rows_affected.
    DmlExecuted {
        rows_affected: i64,
        schema: SchemaRef,
        prepared: bool,
    },
    Fetching {
        reader: ArrowArrayStreamReader,
        record_batch: RecordBatch,
        batch_idx: usize,
        rows_affected: Option<i64>,
        prepared: bool,
    },
    Done {
        schema: SchemaRef,
        prepared: bool,
    },
    Error,
}

impl StatementState {
    /// A cursor is open in `QueryExecuted` (S5), `Fetching` (S6), and `Done` (S7).
    pub fn has_open_cursor(&self) -> bool {
        matches!(
            self,
            StatementState::QueryExecuted { .. }
                | StatementState::Fetching { .. }
                | StatementState::Done { .. }
        )
    }
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

    /// Invariant: `current_state` is always `Some` between public API calls.
    /// Every caller must call `set` before returning to restore the invariant.
    fn take(&mut self) -> T {
        self.current_state.take().expect(
            "State::take called on an empty state — set() was not called after a previous take()",
        )
    }

    pub fn set(&mut self, state: T) {
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

pub struct Statement {
    /// Raw pointer to the owning connection. Valid for the entire lifetime of this Statement
    /// (the connection always outlives its statements). Access via `conn()` / `conn_ptr()`.
    conn: *mut Connection,
    pub stmt_handle: StatementHandle,
    pub state: State<StatementState>,
    pub ard: ArdDescriptor,
    pub ird: IrdDescriptor,
    pub apd: ApdDescriptor,
    pub ipd: IpdDescriptor,
    pub diagnostic_info: DiagnosticInfo,
    pub get_data_state: Option<GetDataState>,
    /// `SQL_ATTR_CURSOR_TYPE` — default `ForwardOnly`.
    pub cursor_type: CursorType,
    /// `SQL_ATTR_MAX_LENGTH` — default 0 (no limit). Stored but not enforced.
    pub max_length: sql::ULen,
    /// Set when `SQLExtendedFetch` has been used on this statement.
    /// Per ODBC spec, `SQLFetch` cannot be mixed with `SQLExtendedFetch`
    /// without first closing the cursor via `SQLFreeStmt(SQL_CLOSE)`.
    pub used_extended_fetch: bool,
    /// Query ID of the last executed query (`SQL_SF_STMT_ATTR_LAST_QUERY_ID`).
    pub last_query_id: Option<String>,
    /// Cancelled by `SQLCancel` (possibly from another thread) and observed
    /// by execution functions via `tokio::select!` when cross-thread cancel
    /// is wired up. Replaced with a fresh token at the start of each
    /// execution/prepare call so that stale cancels do not affect new ops.
    pub cancel_token: CancellationToken,
}

/// Safety: Statement is always accessed on the single ODBC thread that holds the handle.
// The conn raw pointer is valid for the Statement's lifetime (Connection outlives Statement).
// `Send` allows moving the allocation across threads (e.g. when the runtime hands the raw
// Arc pointer back on an arbitrary thread). `Sync` is NOT implemented: sharing `&Statement`
// across threads is unsound because `conn` is a `*mut Connection` with no synchronisation.
// Connection itself uses `unsafe impl Send` to suppress auto-trait checks on the
// `Vec<(Weak<Statement>, *const Statement)>` field, so `Statement: Sync` is not required
// for `Connection: Send`.
unsafe impl Send for Statement {}

impl Statement {
    /// Construct a new Statement for the given connection.
    pub fn new(conn: *mut Connection, stmt_handle: StatementHandle) -> Self {
        Self {
            conn,
            stmt_handle,
            state: StatementState::Created.into(),
            ard: ArdDescriptor::new(),
            ird: IrdDescriptor::new(),
            apd: ApdDescriptor::new(),
            ipd: IpdDescriptor::new(),
            diagnostic_info: DiagnosticInfo::default(),
            get_data_state: None,
            cursor_type: CursorType::ForwardOnly,
            max_length: 0,
            used_extended_fetch: false,
            last_query_id: None,
            cancel_token: CancellationToken::new(),
        }
    }

    /// Borrow the owning connection.
    ///
    /// # Safety
    /// The caller must ensure the Connection outlives this borrow and no other
    /// mutable reference to the Connection exists simultaneously.
    pub unsafe fn conn(&self) -> &Connection {
        debug_assert!(
            !self.conn.is_null(),
            "Statement::conn: connection pointer is null"
        );
        unsafe { &*self.conn }
    }

    /// Return the raw connection pointer without creating a Rust borrow on `self`.
    ///
    /// Use this when you need both a `&mut Connection` and access to other
    /// `Statement` fields in the same scope — the raw pointer carries no borrow
    /// on `self`, so the borrow checker treats the resulting `&mut Connection`
    /// as independent.
    ///
    /// # Safety
    /// The caller must ensure that no live `conn()` borrow (or any other `&Connection`
    /// derived from this statement) exists while the returned pointer is dereferenced
    /// mutably. Having both an active `&Connection` and a `&mut Connection` pointing
    /// to the same allocation is undefined behaviour.
    pub(crate) unsafe fn conn_ptr(&self) -> *mut Connection {
        self.conn
    }
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

pub fn stmt_from_handle<'a>(handle: sql::Handle) -> &'a mut Statement {
    let stmt_ptr = handle as *mut Statement;
    unsafe { stmt_ptr.as_mut().unwrap() }
}
