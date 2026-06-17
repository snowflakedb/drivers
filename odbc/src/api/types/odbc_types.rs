use crate::api::bitmask::Bitmask;
use crate::api::error::OdbcRuntimeSnafu;
use crate::api::handle_registry::{HandleGuard, HandleId};
use crate::api::runtime::global;
use crate::api::{OdbcError, diagnostic::DiagnosticInfo};
use crate::conversion::Binding;
use crate::conversion::warning::Warnings;
use crate::conversion::{NumericSettings, SF_DEFAULT_VARCHAR_MAX_LEN};
use arrow::{array::RecordBatch, datatypes::SchemaRef, ffi_stream::ArrowArrayStreamReader};
use odbc_sys as sql;
use sf_core::protobuf::generated::database_driver_v1::{
    ConnectionHandle as TConnectionHandle, DatabaseHandle as TDatabaseHandle, ExecuteQueryResponse,
    StatementHandle,
};
use snafu::ResultExt;
use std::collections::HashMap;

use parking_lot::Mutex;
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
    /// SQL_ATTR_METADATA_ID (10014) — identifier vs. pattern treatment for catalog functions
    MetadataId,

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
            10014 => Some(Self::MetadataId),
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
            Self::MetadataId => 10014,
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
    /// `SQL_DRIVER_NAME` (6) — name of the driver shared library (string).
    DriverName = 6,
    /// `SQL_DRIVER_VER` (7) — driver release version (string).
    DriverVer = 7,
    /// `SQL_SEARCH_PATTERN_ESCAPE` (14) — escape character for catalog wildcard patterns (string).
    SearchPatternEscape = 14,
    /// `SQL_DBMS_NAME` (17) — name of the DBMS product (string).
    DbmsName = 17,
    /// `SQL_DBMS_VER` (18) — version of the DBMS the connection is talking to (string).
    DbmsVer = 18,
    /// `SQL_CONCAT_NULL_BEHAVIOR` (22) — concat-with-null result (`SQLUSMALLINT`).
    ConcatNullBehavior = 22,
    /// `SQL_CURSOR_COMMIT_BEHAVIOR` (23) — cursor behavior on commit.
    CursorCommitBehavior = 23,
    /// `SQL_CURSOR_ROLLBACK_BEHAVIOR` (24) — cursor behavior on rollback.
    CursorRollbackBehavior = 24,
    /// `SQL_IDENTIFIER_QUOTE_CHAR` (29) — identifier quote character (string).
    IdentifierQuoteChar = 29,
    /// `SQL_SCHEMA_TERM` (39) — DBMS term for schema (string). Aliased as `SQL_OWNER_TERM` in 2.x.
    SchemaTerm = 39,
    /// `SQL_CATALOG_NAME_SEPARATOR` (41) — catalog/schema separator character (string).
    CatalogNameSeparator = 41,
    /// `SQL_CATALOG_TERM` (42) — DBMS term for catalog (string).
    CatalogTerm = 42,
    /// `SQL_CONVERT_FUNCTIONS` (48) — supported `CAST`/`CONVERT` function bitmask.
    ConvertFunctions = 48,
    /// `SQL_NUMERIC_FUNCTIONS` (49) — supported numeric scalar functions bitmask.
    NumericFunctions = 49,
    /// `SQL_STRING_FUNCTIONS` (50) — supported string scalar functions bitmask.
    StringFunctions = 50,
    /// `SQL_SYSTEM_FUNCTIONS` (51) — supported system scalar functions bitmask.
    SystemFunctions = 51,
    /// `SQL_TIMEDATE_FUNCTIONS` (52) — supported timedate scalar functions bitmask.
    TimedateFunctions = 52,
    /// `SQL_CONVERT_BIGINT` (53) — conversion targets from `BIGINT` source bitmask.
    ConvertBigint = 53,
    /// `SQL_CONVERT_BINARY` (54) — conversion targets from `BINARY` source bitmask.
    ConvertBinary = 54,
    /// `SQL_CONVERT_BIT` (55) — conversion targets from `BIT` source bitmask.
    ConvertBit = 55,
    /// `SQL_CONVERT_CHAR` (56) — conversion targets from `CHAR` source bitmask.
    ConvertChar = 56,
    /// `SQL_CONVERT_DATE` (57) — conversion targets from `DATE` source bitmask.
    ConvertDate = 57,
    /// `SQL_CONVERT_DECIMAL` (58) — conversion targets from `DECIMAL` source bitmask.
    ConvertDecimal = 58,
    /// `SQL_CONVERT_DOUBLE` (59) — conversion targets from `DOUBLE` source bitmask.
    ConvertDouble = 59,
    /// `SQL_CONVERT_FLOAT` (60) — conversion targets from `FLOAT` source bitmask.
    ConvertFloat = 60,
    /// `SQL_CONVERT_INTEGER` (61) — conversion targets from `INTEGER` source bitmask.
    ConvertInteger = 61,
    /// `SQL_CONVERT_LONGVARCHAR` (62) — conversion targets from `LONGVARCHAR` source bitmask.
    ConvertLongVarchar = 62,
    /// `SQL_CONVERT_NUMERIC` (63) — conversion targets from `NUMERIC` source bitmask.
    ConvertNumeric = 63,
    /// `SQL_CONVERT_REAL` (64) — conversion targets from `REAL` source bitmask.
    ConvertReal = 64,
    /// `SQL_CONVERT_SMALLINT` (65) — conversion targets from `SMALLINT` source bitmask.
    ConvertSmallint = 65,
    /// `SQL_CONVERT_TIME` (66) — conversion targets from `TIME` source bitmask.
    ConvertTime = 66,
    /// `SQL_CONVERT_TIMESTAMP` (67) — conversion targets from `TIMESTAMP` source bitmask.
    ConvertTimestamp = 67,
    /// `SQL_CONVERT_TINYINT` (68) — conversion targets from `TINYINT` source bitmask.
    ConvertTinyint = 68,
    /// `SQL_CONVERT_VARBINARY` (69) — conversion targets from `VARBINARY` source bitmask.
    ConvertVarbinary = 69,
    /// `SQL_CONVERT_VARCHAR` (70) — conversion targets from `VARCHAR` source bitmask.
    ConvertVarchar = 70,
    /// `SQL_CONVERT_LONGVARBINARY` (71) — conversion targets from `LONGVARBINARY` source bitmask.
    ConvertLongVarbinary = 71,
    /// `SQL_DRIVER_ODBC_VER` (77) — ODBC version the driver conforms to (string).
    DriverOdbcVer = 77,
    /// `SQL_DYNAMIC_CURSOR_ATTRIBUTES1` (144) — dynamic-cursor attribute set 1 bitmask.
    DynamicCursorAttributes1 = 144,
    /// `SQL_GETDATA_EXTENSIONS` (81) — bitmask of supported GetData extensions.
    GetDataExtensions = 81,
    /// `SQL_COLUMN_ALIAS` (87) — whether the driver supports column aliases (string `"Y"`/`"N"`).
    ColumnAlias = 87,
    /// `SQL_GROUP_BY` (88) — `GROUP BY` relationship to selected columns (`SQLUSMALLINT`).
    GroupBy = 88,
    /// `SQL_ORDER_BY_COLUMNS_IN_SELECT` (90) — whether `ORDER BY` columns must appear in the select list (string).
    OrderByColumnsInSelect = 90,
    /// `SQL_SCHEMA_USAGE` (91) — schema usage bitmask. Aliased as `SQL_OWNER_USAGE` in 2.x.
    SchemaUsage = 91,
    /// `SQL_CATALOG_USAGE` (92) — catalog usage bitmask.
    CatalogUsage = 92,
    /// `SQL_SPECIAL_CHARACTERS` (94) — non-alphanumeric characters allowed in identifiers (string).
    SpecialCharacters = 94,
    /// `SQL_MAX_COLUMNS_IN_GROUP_BY` (97) — max columns in a `GROUP BY` (`SQLUSMALLINT`).
    MaxColumnsInGroupBy = 97,
    /// `SQL_MAX_COLUMNS_IN_ORDER_BY` (99) — max columns in an `ORDER BY` (`SQLUSMALLINT`).
    MaxColumnsInOrderBy = 99,
    /// `SQL_MAX_COLUMNS_IN_SELECT` (100) — max columns in a `SELECT` list (`SQLUSMALLINT`).
    MaxColumnsInSelect = 100,
    /// `SQL_TIMEDATE_ADD_INTERVALS` (109) — supported intervals for `TIMESTAMPADD` bitmask.
    TimedateAddIntervals = 109,
    /// `SQL_TIMEDATE_DIFF_INTERVALS` (110) — supported intervals for `TIMESTAMPDIFF` bitmask.
    TimedateDiffIntervals = 110,
    /// `SQL_CATALOG_LOCATION` (114) — whether catalog appears before or after the schema (`SQLUSMALLINT`).
    CatalogLocation = 114,
    /// `SQL_SQL_CONFORMANCE` (118) — SQL-92 conformance level (`SQLUINTEGER`).
    SqlConformance = 118,
    /// `SQL_CONVERT_WCHAR` (122) — conversion targets from `WCHAR` source bitmask.
    ConvertWchar = 122,
    /// `SQL_CONVERT_WLONGVARCHAR` (125) — conversion targets from `WLONGVARCHAR` source bitmask.
    ConvertWlongVarchar = 125,
    /// `SQL_CONVERT_WVARCHAR` (126) — conversion targets from `WVARCHAR` source bitmask.
    ConvertWvarchar = 126,
    /// `SQL_ODBC_INTERFACE_CONFORMANCE` (152) — ODBC interface conformance level (`SQLUINTEGER`).
    OdbcInterfaceConformance = 152,
    /// `SQL_SQL92_PREDICATES` (160) — supported SQL-92 predicates bitmask.
    Sql92Predicates = 160,
    /// `SQL_SQL92_RELATIONAL_JOIN_OPERATORS` (161) — supported SQL-92 join operators bitmask.
    Sql92RelationalJoinOperators = 161,
    /// `SQL_SQL92_VALUE_EXPRESSIONS` (165) — supported SQL-92 value expressions bitmask.
    Sql92ValueExpressions = 165,
    /// `SQL_AGGREGATE_FUNCTIONS` (169) — supported aggregate functions bitmask.
    AggregateFunctions = 169,
    /// `SQL_CONVERT_GUID` (173) — conversion targets from `GUID` source bitmask.
    ConvertGuid = 173,
    /// `SQL_ASYNC_MODE` (10021) — async mode supported by the driver.
    AsyncMode = 10021,
    /// `SQL_MAX_ASYNC_CONCURRENT_STATEMENTS` (10022) — max concurrent async statements.
    MaxAsyncConcurrentStatements = 10022,
    /// `SQL_ASYNC_DBC_FUNCTIONS` (10023) — whether the driver supports async on connections.
    AsyncDbcFunctions = 10023,
    /// `SQL_ASYNC_NOTIFICATION` (10025) — async notification capability.
    AsyncNotification = 10025,
    /// `SQL_CATALOG_NAME` (10003) — whether the driver supports catalog names (string `"Y"`/`"N"`).
    CatalogName = 10003,
    /// `SQL_MAX_IDENTIFIER_LEN` (10005) — max identifier length in characters (`SQLUSMALLINT`).
    MaxIdentifierLen = 10005,
}

impl TryFrom<u16> for InfoType {
    type Error = OdbcError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            6 => Ok(InfoType::DriverName),
            7 => Ok(InfoType::DriverVer),
            14 => Ok(InfoType::SearchPatternEscape),
            17 => Ok(InfoType::DbmsName),
            18 => Ok(InfoType::DbmsVer),
            22 => Ok(InfoType::ConcatNullBehavior),
            23 => Ok(InfoType::CursorCommitBehavior),
            24 => Ok(InfoType::CursorRollbackBehavior),
            29 => Ok(InfoType::IdentifierQuoteChar),
            39 => Ok(InfoType::SchemaTerm),
            41 => Ok(InfoType::CatalogNameSeparator),
            42 => Ok(InfoType::CatalogTerm),
            48 => Ok(InfoType::ConvertFunctions),
            49 => Ok(InfoType::NumericFunctions),
            50 => Ok(InfoType::StringFunctions),
            51 => Ok(InfoType::SystemFunctions),
            52 => Ok(InfoType::TimedateFunctions),
            53 => Ok(InfoType::ConvertBigint),
            54 => Ok(InfoType::ConvertBinary),
            55 => Ok(InfoType::ConvertBit),
            56 => Ok(InfoType::ConvertChar),
            57 => Ok(InfoType::ConvertDate),
            58 => Ok(InfoType::ConvertDecimal),
            59 => Ok(InfoType::ConvertDouble),
            60 => Ok(InfoType::ConvertFloat),
            61 => Ok(InfoType::ConvertInteger),
            62 => Ok(InfoType::ConvertLongVarchar),
            63 => Ok(InfoType::ConvertNumeric),
            64 => Ok(InfoType::ConvertReal),
            65 => Ok(InfoType::ConvertSmallint),
            66 => Ok(InfoType::ConvertTime),
            67 => Ok(InfoType::ConvertTimestamp),
            68 => Ok(InfoType::ConvertTinyint),
            69 => Ok(InfoType::ConvertVarbinary),
            70 => Ok(InfoType::ConvertVarchar),
            71 => Ok(InfoType::ConvertLongVarbinary),
            77 => Ok(InfoType::DriverOdbcVer),
            144 => Ok(InfoType::DynamicCursorAttributes1),
            81 => Ok(InfoType::GetDataExtensions),
            87 => Ok(InfoType::ColumnAlias),
            88 => Ok(InfoType::GroupBy),
            90 => Ok(InfoType::OrderByColumnsInSelect),
            91 => Ok(InfoType::SchemaUsage),
            92 => Ok(InfoType::CatalogUsage),
            94 => Ok(InfoType::SpecialCharacters),
            97 => Ok(InfoType::MaxColumnsInGroupBy),
            99 => Ok(InfoType::MaxColumnsInOrderBy),
            100 => Ok(InfoType::MaxColumnsInSelect),
            109 => Ok(InfoType::TimedateAddIntervals),
            110 => Ok(InfoType::TimedateDiffIntervals),
            114 => Ok(InfoType::CatalogLocation),
            118 => Ok(InfoType::SqlConformance),
            122 => Ok(InfoType::ConvertWchar),
            125 => Ok(InfoType::ConvertWlongVarchar),
            126 => Ok(InfoType::ConvertWvarchar),
            152 => Ok(InfoType::OdbcInterfaceConformance),
            160 => Ok(InfoType::Sql92Predicates),
            161 => Ok(InfoType::Sql92RelationalJoinOperators),
            165 => Ok(InfoType::Sql92ValueExpressions),
            169 => Ok(InfoType::AggregateFunctions),
            173 => Ok(InfoType::ConvertGuid),
            10003 => Ok(InfoType::CatalogName),
            10005 => Ok(InfoType::MaxIdentifierLen),
            10021 => Ok(InfoType::AsyncMode),
            10022 => Ok(InfoType::MaxAsyncConcurrentStatements),
            10023 => Ok(InfoType::AsyncDbcFunctions),
            10025 => Ok(InfoType::AsyncNotification),
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

/// ODBC statement attribute value constants.
/// `SQL_CONCUR_READ_ONLY` (1) — read-only cursor concurrency (default).
pub const SQL_CONCUR_READ_ONLY: sql::ULen = 1;
/// `SQL_CONCUR_LOCK` (2) — cursor concurrency with locking.
pub const SQL_CONCUR_LOCK: sql::ULen = 2;
/// `SQL_CONCUR_ROWVER` (3) — cursor concurrency with row versioning.
#[allow(dead_code)] // Covered by SQL_CONCUR_LOCK..=SQL_CONCUR_VALUES range pattern
pub const SQL_CONCUR_ROWVER: sql::ULen = 3;
/// `SQL_CONCUR_VALUES` (4) — cursor concurrency with optimistic values.
pub const SQL_CONCUR_VALUES: sql::ULen = 4;
/// `SQL_NONSCROLLABLE` (0) — non-scrollable cursor (default).
pub const SQL_NONSCROLLABLE: sql::ULen = 0;
/// `SQL_SCROLLABLE` (1) — scrollable cursor.
pub const SQL_SCROLLABLE: sql::ULen = 1;
/// `SQL_UNSPECIFIED` (0) — unspecified cursor sensitivity (default).
pub const SQL_UNSPECIFIED: sql::ULen = 0;
/// `SQL_INSENSITIVE` (1) — insensitive cursor.
pub const SQL_INSENSITIVE: sql::ULen = 1;
/// `SQL_SENSITIVE` (2) — sensitive cursor.
pub const SQL_SENSITIVE: sql::ULen = 2;
/// `SQL_NOSCAN_OFF` (0) — scan for escape sequences (default).
pub const SQL_NOSCAN_OFF: sql::ULen = 0;
/// `SQL_NOSCAN_ON` (1) — do not scan for escape sequences.
pub const SQL_NOSCAN_ON: sql::ULen = 1;
/// `SQL_SC_NON_UNIQUE` (0) — simulate non-unique cursors (default).
pub const SQL_SC_NON_UNIQUE: sql::ULen = 0;
/// `SQL_RD_OFF` (0) — do not retrieve data after positioned update.
pub const SQL_RD_OFF: sql::ULen = 0;
/// `SQL_RD_ON` (1) — retrieve data after positioned update (default).
pub const SQL_RD_ON: sql::ULen = 1;

/// ODBC statement attribute identifiers (matching `SQL_ATTR_*` constants from `sql.h`).
#[repr(i32)]
#[allow(clippy::enum_variant_names)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StmtAttr {
    /// `SQL_ATTR_CURSOR_SCROLLABLE` (-1) — whether the cursor is scrollable.
    CursorScrollable = -1,
    /// `SQL_ATTR_CURSOR_SENSITIVITY` (-2) — cursor sensitivity to changes.
    CursorSensitivity = -2,
    /// `SQL_ATTR_QUERY_TIMEOUT` (0) — query timeout in seconds (0 = no timeout).
    QueryTimeout = 0,
    /// `SQL_ATTR_MAX_ROWS` (1) — maximum rows returned (0 = no limit).
    MaxRows = 1,
    /// `SQL_ATTR_NOSCAN` (2) — whether to scan for ODBC escape sequences.
    Noscan = 2,
    /// `SQL_ATTR_MAX_LENGTH` (3) — maximum amount of data returned from character/binary columns.
    MaxLength = 3,
    /// `SQL_ATTR_ASYNC_ENABLE` (4) — enable/disable asynchronous execution.
    AsyncEnable = 4,
    /// `SQL_ATTR_ROW_BIND_TYPE` (5) — row-wise vs column-wise binding.
    RowBindType = 5,
    /// `SQL_ATTR_CURSOR_TYPE` (6) — type of cursor.
    CursorType = 6,
    /// `SQL_ATTR_CONCURRENCY` (7) — cursor concurrency.
    Concurrency = 7,
    /// `SQL_ATTR_KEYSET_SIZE` (8) — keyset size for keyset-driven cursors.
    KeysetSize = 8,
    /// `SQL_ATTR_SIMULATE_CURSOR` (10) — how to simulate positioned update/delete statements.
    SimulateCursor = 10,
    /// `SQL_ATTR_RETRIEVE_DATA` (11) — whether to retrieve data after a positioned update.
    RetrieveData = 11,
    /// `SQL_ATTR_USE_BOOKMARKS` (12) — whether bookmarks are used.
    UseBookmarks = 12,
    /// `SQL_ATTR_ENABLE_AUTO_IPD` (15) — automatic population of the IPD.
    EnableAutoIpd = 15,
    /// `SQL_ATTR_PARAM_BIND_OFFSET_PTR` (17) — pointer to offset added to APD data/indicator ptrs.
    ParamBindOffsetPtr = 17,
    /// `SQL_ATTR_PARAM_BIND_TYPE` (18) — column-wise (0) vs row-wise parameter binding.
    ParamBindType = 18,
    /// `SQL_ATTR_PARAM_STATUS_PTR` (20) — pointer to per-parameter-set status array (written by driver).
    ParamStatusPtr = 20,
    /// `SQL_ATTR_PARAMS_PROCESSED_PTR` (21) — pointer where driver writes count of processed param sets.
    ParamsProcessedPtr = 21,
    /// `SQL_ATTR_PARAMSET_SIZE` (22) — number of parameter sets in each `SQLExecute`/`SQLExecDirect`.
    ParamsetSize = 22,
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
    /// `SQL_ATTR_METADATA_ID` (10014) — identifier vs. pattern treatment for catalog functions.
    MetadataId = 10014,

    // Custom Snowflake statement attributes (SQL_SF_STMT_ATTR_BASE = 0x4000 + 0x106)
    /// `SQL_SF_STMT_ATTR_LAST_QUERY_ID` (0x4107 = 16647) — query ID of last execution (read-only).
    SnowflakeLastQueryId = 16647,
    /// `SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT` (0x4108 = 16648) — multi-statement count.
    SnowflakeMultiStatementCount = 16648,
}

impl TryFrom<i32> for StmtAttr {
    type Error = OdbcError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            -2 => Ok(StmtAttr::CursorSensitivity),
            -1 => Ok(StmtAttr::CursorScrollable),
            0 => Ok(StmtAttr::QueryTimeout),
            1 => Ok(StmtAttr::MaxRows),
            2 => Ok(StmtAttr::Noscan),
            3 => Ok(StmtAttr::MaxLength),
            4 => Ok(StmtAttr::AsyncEnable),
            5 => Ok(StmtAttr::RowBindType),
            6 => Ok(StmtAttr::CursorType),
            7 => Ok(StmtAttr::Concurrency),
            8 => Ok(StmtAttr::KeysetSize),
            10 => Ok(StmtAttr::SimulateCursor),
            11 => Ok(StmtAttr::RetrieveData),
            12 => Ok(StmtAttr::UseBookmarks),
            15 => Ok(StmtAttr::EnableAutoIpd),
            17 => Ok(StmtAttr::ParamBindOffsetPtr),
            18 => Ok(StmtAttr::ParamBindType),
            20 => Ok(StmtAttr::ParamStatusPtr),
            21 => Ok(StmtAttr::ParamsProcessedPtr),
            22 => Ok(StmtAttr::ParamsetSize),
            23 => Ok(StmtAttr::RowBindOffsetPtr),
            25 => Ok(StmtAttr::RowStatusPtr),
            26 => Ok(StmtAttr::RowsFetchedPtr),
            27 => Ok(StmtAttr::RowArraySize),
            10010 => Ok(StmtAttr::AppRowDesc),
            10011 => Ok(StmtAttr::AppParamDesc),
            10012 => Ok(StmtAttr::ImpRowDesc),
            10013 => Ok(StmtAttr::ImpParamDesc),
            10014 => Ok(StmtAttr::MetadataId),
            // Windows/Microsoft ODBC (SQL_DRIVER_STMT_ATTR_BASE = 1000)
            1263 => Ok(StmtAttr::SnowflakeLastQueryId),
            1264 => Ok(StmtAttr::SnowflakeMultiStatementCount),
            // Mac/iODBC (SQL_DRIVER_STMT_ATTR_BASE = 16384)
            16647 => Ok(StmtAttr::SnowflakeLastQueryId),
            16648 => Ok(StmtAttr::SnowflakeMultiStatementCount),
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
    /// `SQL_DESC_DISPLAY_SIZE` (6) — maximum display width.
    DisplaySize = 6,
    /// `SQL_DESC_UNSIGNED` (8) — whether the column is unsigned.
    Unsigned = 8,
    /// `SQL_DESC_FIXED_PREC_SCALE` (9) — whether the column has fixed precision/scale.
    FixedPrecScale = 9,
    /// `SQL_DESC_UPDATABLE` (10) — whether the column is updatable.
    Updatable = 10,
    /// `SQL_DESC_AUTO_UNIQUE_VALUE` (11) — whether the column auto-increments.
    AutoUniqueValue = 11,
    /// `SQL_DESC_CASE_SENSITIVE` (12) — whether the column is case-sensitive.
    CaseSensitive = 12,
    /// `SQL_DESC_SEARCHABLE` (13) — searchability of the column.
    Searchable = 13,
    /// `SQL_DESC_TYPE_NAME` (14) — data-source-dependent type name.
    TypeName = 14,
    /// `SQL_DESC_TABLE_NAME` (15) — table name.
    TableName = 15,
    /// `SQL_DESC_SCHEMA_NAME` (16) — schema name.
    SchemaName = 16,
    /// `SQL_DESC_CATALOG_NAME` (17) — catalog name.
    CatalogName = 17,
    /// `SQL_DESC_LABEL` (18) — column label or title.
    Label = 18,
    /// `SQL_DESC_ARRAY_SIZE` (20) — header: number of rows in the rowset.
    ArraySize = 20,
    /// `SQL_DESC_ARRAY_STATUS_PTR` (21) — header: pointer to row status array.
    ArrayStatusPtr = 21,
    /// `SQL_DESC_BASE_COLUMN_NAME` (22) — base column name.
    BaseColumnName = 22,
    /// `SQL_DESC_BASE_TABLE_NAME` (23) — base table name.
    BaseTableName = 23,
    /// `SQL_DESC_BIND_OFFSET_PTR` (24) — header: binding offset pointer.
    BindOffsetPtr = 24,
    /// `SQL_DESC_BIND_TYPE` (25) — header: row-wise vs column-wise binding.
    BindType = 25,
    /// `SQL_DESC_DATETIME_INTERVAL_PRECISION` (26) — leading precision for interval C types.
    DatetimeIntervalPrecision = 26,
    /// `SQL_DESC_LITERAL_PREFIX` (27) — literal prefix for the type.
    LiteralPrefix = 27,
    /// `SQL_DESC_LITERAL_SUFFIX` (28) — literal suffix for the type.
    LiteralSuffix = 28,
    /// `SQL_DESC_LOCAL_TYPE_NAME` (29) — localized type name.
    LocalTypeName = 29,
    /// `SQL_DESC_NUM_PREC_RADIX` (32) — numeric precision radix (2 or 10).
    NumPrecRadix = 32,
    /// `SQL_DESC_PARAMETER_TYPE` (33) — parameter direction (IPD only).
    ParameterType = 33,
    /// `SQL_DESC_ROWS_PROCESSED_PTR` (34) — header: pointer to rows-processed count.
    RowsProcessedPtr = 34,
    /// `SQL_DESC_COUNT` (1001) — number of bound columns (header field, record 0).
    Count = 1001,
    /// `SQL_DESC_TYPE` (1002) — verbose data type of the column.
    Type = 1002,
    /// `SQL_DESC_LENGTH` (1003) — column length in characters.
    Length = 1003,
    /// `SQL_DESC_OCTET_LENGTH_PTR` (1004) — pointer to the octet-length buffer.
    OctetLengthPtr = 1004,
    /// `SQL_DESC_PRECISION` (1005) — numeric precision.
    Precision = 1005,
    /// `SQL_DESC_SCALE` (1006) — numeric scale.
    Scale = 1006,
    /// `SQL_DESC_NULLABLE` (1008) — whether the column is nullable.
    Nullable = 1008,
    /// `SQL_DESC_INDICATOR_PTR` (1009) — pointer to the indicator buffer.
    IndicatorPtr = 1009,
    /// `SQL_DESC_DATA_PTR` (1010) — pointer to the data buffer.
    DataPtr = 1010,
    /// `SQL_DESC_NAME` (1011) — column name (string, IRD only).
    Name = 1011,
    /// `SQL_DESC_UNNAMED` (1012) — whether the column is named (SQL_NAMED / SQL_UNNAMED).
    Unnamed = 1012,
    /// `SQL_DESC_OCTET_LENGTH` (1013) — length in bytes of the data buffer.
    OctetLength = 1013,

    // ODBC 2.x SQL_COLUMN_* identifiers (used by SQLColAttributes)
    /// `SQL_COLUMN_COUNT` (0) — number of columns.
    ColumnCount = 0,
    /// `SQL_COLUMN_NAME` (1) — column name (ODBC 2.x alias).
    ColumnName = 1,
    /// `SQL_COLUMN_LENGTH` (3) — transfer octet length (ODBC 2.x).
    ColumnLength = 3,
    /// `SQL_COLUMN_PRECISION` (4) — column size (ODBC 2.x).
    ColumnPrecision = 4,
    /// `SQL_COLUMN_SCALE` (5) — decimal digits (ODBC 2.x).
    ColumnScale = 5,
    /// `SQL_COLUMN_NULLABLE` (7) — nullable (ODBC 2.x).
    ColumnNullable = 7,
}

impl TryFrom<i16> for DescField {
    type Error = OdbcError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(DescField::ColumnCount),
            1 => Ok(DescField::ColumnName),
            2 => Ok(DescField::ConciseType),
            3 => Ok(DescField::ColumnLength),
            4 => Ok(DescField::ColumnPrecision),
            5 => Ok(DescField::ColumnScale),
            6 => Ok(DescField::DisplaySize),
            7 => Ok(DescField::ColumnNullable),
            8 => Ok(DescField::Unsigned),
            9 => Ok(DescField::FixedPrecScale),
            10 => Ok(DescField::Updatable),
            11 => Ok(DescField::AutoUniqueValue),
            12 => Ok(DescField::CaseSensitive),
            13 => Ok(DescField::Searchable),
            14 => Ok(DescField::TypeName),
            15 => Ok(DescField::TableName),
            16 => Ok(DescField::SchemaName),
            17 => Ok(DescField::CatalogName),
            18 => Ok(DescField::Label),
            20 => Ok(DescField::ArraySize),
            21 => Ok(DescField::ArrayStatusPtr),
            22 => Ok(DescField::BaseColumnName),
            23 => Ok(DescField::BaseTableName),
            24 => Ok(DescField::BindOffsetPtr),
            25 => Ok(DescField::BindType),
            26 => Ok(DescField::DatetimeIntervalPrecision),
            27 => Ok(DescField::LiteralPrefix),
            28 => Ok(DescField::LiteralSuffix),
            29 => Ok(DescField::LocalTypeName),
            32 => Ok(DescField::NumPrecRadix),
            33 => Ok(DescField::ParameterType),
            34 => Ok(DescField::RowsProcessedPtr),
            1001 => Ok(DescField::Count),
            1002 => Ok(DescField::Type),
            1003 => Ok(DescField::Length),
            1004 => Ok(DescField::OctetLengthPtr),
            1005 => Ok(DescField::Precision),
            1006 => Ok(DescField::Scale),
            1008 => Ok(DescField::Nullable),
            1009 => Ok(DescField::IndicatorPtr),
            1010 => Ok(DescField::DataPtr),
            1011 => Ok(DescField::Name),
            1012 => Ok(DescField::Unnamed),
            1013 => Ok(DescField::OctetLength),
            _ => {
                tracing::warn!("Unknown descriptor field identifier: {}", value);
                Err(OdbcError::InvalidDescriptorFieldId {
                    field_id: value,
                    location: snafu::location!(),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorKind {
    Ard,
    Ird,
    Apd,
    Ipd,
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

    // Snowflake-specific vendor SQL type codes for TIMESTAMP variants.
    //
    // Defined here so applications can pass them as the `ParameterType`
    // argument to `SQLBindParameter` and explicitly request that a bound
    // value round-trip as `TIMESTAMP_LTZ` / `_TZ` / `_NTZ` instead of being
    // routed by the standard `SQL_TYPE_TIMESTAMP` (93) which has no way to
    // distinguish the three. The values match the legacy 3.16.0 Snowflake
    // ODBC driver (`Source/sf_odbc.h`) for application compatibility.
    //
    // These are *not* returned from `SQLDescribeCol` or
    // `SQLColAttribute(SQL_DESC_CONCISE_TYPE)`: per the MS ODBC spec, those
    // descriptors must report the standard `SQL_TYPE_TIMESTAMP` (93) for
    // ODBC 3.x output. Applications distinguish the three subtypes via
    // `SQLColAttribute(SQL_DESC_TYPE_NAME)`.
    SqlSfTimestampLtz = 2000, // SQL_SF_TIMESTAMP_LTZ
    SqlSfTimestampTz = 2001,  // SQL_SF_TIMESTAMP_TZ
    SqlSfTimestampNtz = 2002, // SQL_SF_TIMESTAMP_NTZ

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
            2000 => Ok(SqlType::SqlSfTimestampLtz),
            2001 => Ok(SqlType::SqlSfTimestampTz),
            2002 => Ok(SqlType::SqlSfTimestampNtz),
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

/// Snowflake vendor SQL type codes as `odbc_sys::SqlDataType` constants. Match
/// the legacy 3.16.0 driver's macros from `Source/sf_odbc.h` and the
/// corresponding `SqlType::SqlSfTimestamp{Ltz,Tz,Ntz}` enum variants. Use
/// these forms in `match` patterns against `sql::SqlDataType` (e.g. when
/// dispatching by the `ParameterType` argument of `SQLBindParameter`).
pub const SQL_SF_TIMESTAMP_LTZ: sql::SqlDataType = sql::SqlDataType(2000);
pub const SQL_SF_TIMESTAMP_TZ: sql::SqlDataType = sql::SqlDataType(2001);
pub const SQL_SF_TIMESTAMP_NTZ: sql::SqlDataType = sql::SqlDataType(2002);

/// Snowflake-specific timestamp subtype carried alongside the standard ODBC
/// `SQL_TYPE_TIMESTAMP` (93) on the IPD. Set by `SQLBindParameter` when the
/// application uses one of the vendor codes `SQL_SF_TIMESTAMP_{LTZ,TZ,NTZ}`,
/// so the binding pipeline knows which Snowflake logical type to emit on the
/// wire while `SQLDescribeParam` and `SQLGetDescField(IPD, SQL_DESC_TYPE)`
/// keep returning the spec-mandated 93. `None` means "no vendor opt-in";
/// the converter dispatch falls back to the default for the SQL type
/// (which for `SQL_TYPE_TIMESTAMP` is NTZ, mirroring the legacy driver).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampSubtype {
    /// Vendor code 2002: explicit opt-in to TIMESTAMP_NTZ on the wire.
    Ntz,
    /// Vendor code 2000: TIMESTAMP_LTZ — naive datetime interpreted in the
    /// session timezone (matches legacy 3.16.0 wall-clock-string semantics).
    Ltz,
    /// Vendor code 2001: TIMESTAMP_TZ — preserves the offset on the wire.
    Tz,
}

impl TimestampSubtype {
    /// Map a `SQLBindParameter` `ParameterType` argument to its Snowflake
    /// timestamp subtype, if it is one of the vendor codes 2000/2001/2002.
    /// Returns `None` for every other SQL type — including the standard
    /// `SQL_TYPE_TIMESTAMP` (93), which has no vendor opt-in associated.
    pub fn from_parameter_type(parameter_type: sql::SqlDataType) -> Option<Self> {
        match parameter_type {
            SQL_SF_TIMESTAMP_NTZ => Some(Self::Ntz),
            SQL_SF_TIMESTAMP_LTZ => Some(Self::Ltz),
            SQL_SF_TIMESTAMP_TZ => Some(Self::Tz),
            _ => None,
        }
    }
}

/// Application Row Descriptor (ARD).
///
/// Stores column binding information and block-cursor header fields.
pub struct ArdDescriptor {
    pub diagnostic_info: DiagnosticInfo,
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
            diagnostic_info: DiagnosticInfo::default(),
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
pub struct ApdDescriptor {
    pub diagnostic_info: DiagnosticInfo,
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
            diagnostic_info: DiagnosticInfo::default(),
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
pub struct IrdDescriptor {
    pub diagnostic_info: DiagnosticInfo,
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
            diagnostic_info: DiagnosticInfo::default(),
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
pub struct IpdDescriptor {
    pub diagnostic_info: DiagnosticInfo,
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
            diagnostic_info: DiagnosticInfo::default(),
            records: HashMap::new(),
            array_status_ptr: std::ptr::null_mut(),
            rows_processed_ptr: std::ptr::null_mut(),
        }
    }

    pub fn desc_count(&self) -> u16 {
        self.records.keys().copied().max().unwrap_or(0)
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
            Err(OdbcError::DaeRequired { .. }) => sql::SqlReturn::NEED_DATA,
            Err(OdbcError::StillExecuting { .. }) => sql::SqlReturn::STILL_EXECUTING,
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
pub struct Env {
    pub environment: Mutex<Environment>,
}

// TODO: this is a hack to allow the Env to be used in a multi-threaded environment
// Will be removed after this PR stack is completed
unsafe impl Send for Env {}
unsafe impl Sync for Env {}

pub struct Environment {
    pub odbc_version: sql::Integer,
    pub connection_pooling: sql::AttrConnectionPooling,
    pub connection_pool_match: sql::AttrCpMatch,
    pub diagnostic_info: DiagnosticInfo,
    pub connections: Vec<HandleId>,
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

pub struct Dbc {
    pub connection: Mutex<Connection>,
    pub env_id: HandleId,
}

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
    /// HandleIds of all child statements allocated on this connection.
    /// Used by `free_connection` to release orphaned statements.
    pub(crate) child_statements: Vec<HandleId>,
    /// Cached local autocommit state. Defaults to `AutocommitValue::On`.
    /// Updated when SQL_ATTR_AUTOCOMMIT is set; used as fallback for get_connect_attr
    /// when the server session parameter is unavailable.
    pub cached_autocommit: AutocommitValue,
    /// Cached SQL_ATTR_CURRENT_CATALOG value. Populated after connect and updated
    /// after each successful USE DATABASE (SET). SQLGetConnectAttr always refreshes
    /// this from the server per spec; the field is used to track the catalog for
    /// internal purposes (e.g. logging, future optimizations).
    pub current_catalog: Option<String>,
    /// SQL_ATTR_METADATA_ID — identifier vs. pattern treatment for catalog functions (default false)
    pub metadata_id: bool,
    /// Value of the `DRIVER` keyword captured from the `SQLDriverConnect`
    /// connection string, if present. Used as the primary lookup section
    /// in `odbcinst.ini` when resolving `SQLGetInfo(SQL_DRIVER_NAME)`.
    pub driver_section: Option<String>,
    /// Value of the `DSN` keyword captured at connect time (either from
    /// `SQLConnect`'s server-name argument or `SQLDriverConnect`'s
    /// `DSN=...`). Used to find the driver short name via `odbc.ini`
    /// when `driver_section` is absent.
    pub dsn_name: Option<String>,
}

// Safety: Connection contains raw pointers (quiet_mode: sql::Pointer) that are !Send + !Sync.
// Access to Connection is always serialised through the Mutex<Connection> in Dbc, and ODBC
// guarantees that a single connection handle is only used from one thread at a time.
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
///
/// `sql_data_type` always holds a *standard* ODBC SQL type code (1..=12,
/// 91..=95, 101..=113, etc.) so that `SQLDescribeParam` and
/// `SQLGetDescField(IPD, SQL_DESC_TYPE)` echo spec-conformant values back
/// to the application. When `SQLBindParameter` is called with one of the
/// Snowflake vendor codes (`SQL_SF_TIMESTAMP_LTZ` / `_TZ` / `_NTZ`), the
/// vendor opt-in is normalised: `sql_data_type` becomes
/// `SQL_TYPE_TIMESTAMP` (93) and the chosen subtype is stashed on
/// `sf_subtype` for the bind-time converter dispatch.
#[derive(Debug)]
pub struct IpdRecord {
    pub sql_data_type: sql::SqlDataType,
    pub column_size: sql::ULen,
    pub decimal_digits: sql::SmallInt,
    pub direction: sql::SmallInt,
    pub nullable: sql::SmallInt,
    /// Snowflake-specific timestamp subtype, set when the application binds
    /// with `SQL_SF_TIMESTAMP_{LTZ,TZ,NTZ}` to opt in to a non-default
    /// Snowflake logical type for the wire. `None` for every other binding.
    pub sf_subtype: Option<TimestampSubtype>,
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
            sf_subtype: None,
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
    /// Mirrors `IpdRecord::sf_subtype`. Lets the converter dispatch route a
    /// `SQL_TYPE_TIMESTAMP` bind to the right Snowflake logical type
    /// (NTZ / LTZ / TZ) when the application opted in via a vendor code.
    pub sf_subtype: Option<TimestampSubtype>,
}

impl ParameterBinding {
    /// Build a `ParameterBinding` directly from a single APD/IPD record pair
    /// without applying any row offset. Production code routes through
    /// `binding_for_row`, which handles column-wise / row-wise array binding;
    /// this constructor is kept for unit tests that build a single binding.
    #[cfg(test)]
    pub fn from_apd_ipd(apd: &ApdRecord, ipd: &IpdRecord) -> Self {
        Self {
            sql_data_type: ipd.sql_data_type,
            value_type: apd.value_type,
            parameter_value_ptr: apd.data_ptr,
            buffer_length: apd.buffer_length,
            str_len_or_ind_ptr: apd.str_len_or_ind_ptr,
            sf_subtype: ipd.sf_subtype,
        }
    }
}

/// Tracks whether the current execution originated from `SQLPrepare`+`SQLExecute`
/// or from `SQLExecDirect`. Maps to the ODBC spec's `[p]`/`[np]` transition
/// annotations (e.g. `SQLFreeStmt(SQL_CLOSE)` in S5 → S1 [np] / S3 [p]).
#[derive(Clone, Debug)]
pub enum ExecutionOrigin {
    Prepared { schema: SchemaRef },
    Direct,
}

impl ExecutionOrigin {
    pub fn restore_state(&self) -> StatementState {
        match self {
            ExecutionOrigin::Prepared { schema } => StatementState::Prepared {
                schema: schema.clone(),
            },
            ExecutionOrigin::Direct => StatementState::Created,
        }
    }

    pub fn is_prepared(&self) -> bool {
        matches!(self, ExecutionOrigin::Prepared { .. })
    }
}

/// State of an individual DAE parameter's data during the `SQLPutData` loop.
pub enum ParamValue {
    Pending,
    Null,
    Data(Vec<Vec<u8>>),
}

/// Holds the context for a data-at-execution operation in progress.
pub struct DaeContext {
    pub dae_params: Vec<u16>,
    pub current_index: usize,
    pub pushed_data: HashMap<u16, ParamValue>,
    pub deferred_query: Option<String>,
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
        origin: ExecutionOrigin,
    },
    /// ODBC state S4 for DDL. No cursor opened; SQLRowCount returns -1.
    DdlExecuted {
        schema: SchemaRef,
        origin: ExecutionOrigin,
    },
    /// ODBC state S4 for DML (INSERT/UPDATE/DELETE/MERGE).
    /// No cursor opened; SQLRowCount returns rows_affected.
    DmlExecuted {
        rows_affected: i64,
        schema: SchemaRef,
        origin: ExecutionOrigin,
    },
    Fetching {
        reader: ArrowArrayStreamReader,
        record_batch: RecordBatch,
        batch_idx: usize,
        rows_affected: Option<i64>,
        origin: ExecutionOrigin,
    },
    Done {
        #[allow(dead_code)]
        schema: SchemaRef,
        origin: ExecutionOrigin,
    },
    /// ODBC state S8: Need data, waiting for `SQLParamData`.
    AwaitingParamData {
        dae_context: Box<DaeContext>,
        origin: ExecutionOrigin,
    },
    /// ODBC state S9: Need data, waiting for `SQLPutData`.
    AwaitingPutData {
        dae_context: Box<DaeContext>,
        origin: ExecutionOrigin,
    },
    /// ODBC state S10: Need data, `SQLPutData` called at least once.
    PutDataCalled {
        dae_context: Box<DaeContext>,
        origin: ExecutionOrigin,
    },
    /// Async `SQLExecDirect` spawned; polling for completion.
    AsyncExecDirect {
        join_handle: tokio::task::JoinHandle<Result<ExecDirectOutcome, OdbcError>>,
    },
    /// Async `SQLPrepare` spawned; polling for completion.
    AsyncPrepare {
        join_handle: tokio::task::JoinHandle<Result<PrepareOutcome, OdbcError>>,
    },
    /// Async `SQLExecute` spawned; polling for completion.
    AsyncExecute {
        join_handle: tokio::task::JoinHandle<Result<ExecuteOutcome, OdbcError>>,
        origin: ExecutionOrigin,
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

    /// Returns `true` when the statement is in any of the NeedData states (S8/S9/S10).
    pub fn is_need_data(&self) -> bool {
        matches!(
            self,
            StatementState::AwaitingParamData { .. }
                | StatementState::AwaitingPutData { .. }
                | StatementState::PutDataCalled { .. }
        )
    }

    /// Returns `true` when an async operation has been spawned and is awaiting poll completion.
    pub fn is_async_executing(&self) -> bool {
        matches!(
            self,
            Self::AsyncExecDirect { .. } | Self::AsyncPrepare { .. } | Self::AsyncExecute { .. }
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
    pub(crate) fn take(&mut self) -> T {
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

/// Outer Statement handle.
///
/// Most mutable state lives inside `inner: Mutex<StatementInner>`.
/// `cancel_token` is also mutable (interior mutability via its own Mutex)
/// to allow zero-contention cross-thread cancellation without locking `inner`.
/// The `HandleManager` stores `Statement` inside `Arc<RwLock<Option<Statement>>>`,
/// so the outer fields are accessible through `HandleGuard::deref()` without
/// any additional locking.
pub struct Statement {
    /// ID of the parent connection handle. Looked up via the global dbc_registry.
    pub conn_id: HandleId,
    pub stmt_handle: StatementHandle,
    pub inner: parking_lot::Mutex<StatementInner>,
    /// Cancellation token for the currently in-flight operation, if any.
    /// `Some(token)` while a cancellable operation is running (sync or async); `None` otherwise.
    /// SQLCancel checks this without locking `inner` — zero-contention cross-thread cancel.
    pub cancel_token: parking_lot::Mutex<Option<CancellationToken>>,
}

pub struct ExecDirectOutcome {
    pub response: ExecuteQueryResponse,
    pub conn_handle: TConnectionHandle,
}

pub struct PrepareOutcome {
    pub number_of_binds: u16,
    pub schema: SchemaRef,
    pub array_bind_supported: bool,
}

pub struct ExecuteOutcome {
    pub response: ExecuteQueryResponse,
    pub conn_handle: TConnectionHandle,
}

/// All mutable statement state, protected by `Statement::inner`.
///
/// # Lock ordering
///
/// When both `Connection` (`dbc.connection.lock()`) and `inner` must be
/// held, `Connection` is locked first. `exec_direct_impl`,
/// `prepare_impl`, `param_data` (and the `execute_dae` it delegates to),
/// `fetch`, and `extended_fetch` follow this rule.
///
/// Functions that only mutate `inner` (`SQLBindCol`, `SQLBindParameter`,
/// `SQLPutData`, `SQLSetStmtAttr`, `SQLFreeStmt`, `SQLNumParams`,
/// `SQLDescribeParam`, the diagnostic helpers) do not lock `Connection`
/// at all. `SQLCancel` operates only on `Statement::cancel_token` and
/// never touches either mutex.
pub struct StatementInner {
    pub state: State<StatementState>,
    pub ard: ArdDescriptor,
    pub ird: IrdDescriptor,
    pub apd: ApdDescriptor,
    pub ipd: IpdDescriptor,
    pub ard_handle: HandleId,
    pub ird_handle: HandleId,
    pub apd_handle: HandleId,
    pub ipd_handle: HandleId,
    pub diagnostic_info: DiagnosticInfo,
    pub get_data_state: Option<GetDataState>,
    /// `SQL_ATTR_CURSOR_TYPE` — default `ForwardOnly`.
    pub cursor_type: CursorType,
    /// `SQL_ATTR_MAX_LENGTH` — default 0 (no limit). Stored but not enforced.
    pub max_length: sql::ULen,
    /// `SQL_ATTR_METADATA_ID` — inherited from connection at allocation time (default false).
    pub metadata_id: bool,
    /// Set when `SQLExtendedFetch` has been used on this statement.
    /// Per ODBC spec, `SQLFetch` cannot be mixed with `SQLExtendedFetch`
    /// without first closing the cursor via `SQLFreeStmt(SQL_CLOSE)`.
    pub used_extended_fetch: bool,
    /// Number of `?` parameter markers reported by the server after
    /// `SQLPrepare`. Used to ignore spurious APD bindings on non-existent
    /// parameters (e.g. DAE detection for "SELECT 1" with a bound param).
    /// `None` before the first prepare or after exec-direct.
    pub prepared_param_count: Option<u16>,
    /// Server hint from the most recent `SQLPrepare` describe
    /// (`arrayBindSupported`). `None` for exec-direct paths (where the
    /// describe phase is skipped and the wrapper has no per-statement
    /// hint) - those should default to the conservative `false` behaviour
    /// when consulting the flag. Cleared back to `None` whenever the
    /// statement leaves the prepared-state (SQLFreeStmt, new prepare, etc.).
    pub prepared_array_bind_supported: Option<bool>,
    /// `SQL_ATTR_QUERY_TIMEOUT` — query timeout in seconds (default 0 = no timeout).
    pub query_timeout: sql::ULen,
    /// `SQL_ATTR_NOSCAN` — whether to scan for ODBC escape sequences (default SQL_NOSCAN_OFF = 0).
    pub noscan: sql::ULen,
    /// `SQL_ATTR_MAX_ROWS` — maximum rows returned (default 0 = no limit).
    pub max_rows: sql::ULen,
    /// Rows returned to the application so far in the current result set.
    /// Reset to 0 on each execution. Used to enforce `max_rows`.
    pub rows_returned: sql::ULen,
    /// `SQL_ATTR_CONCURRENCY` — cursor concurrency (default SQL_CONCUR_READ_ONLY = 1).
    pub concurrency: sql::ULen,
    /// `SQL_ATTR_CURSOR_SCROLLABLE` — cursor scrollability (default SQL_NONSCROLLABLE = 0).
    pub cursor_scrollable: sql::ULen,
    /// `SQL_ATTR_CURSOR_SENSITIVITY` — cursor sensitivity (default SQL_UNSPECIFIED = 0).
    pub cursor_sensitivity: sql::ULen,
    /// `SQL_ATTR_KEYSET_SIZE` — keyset size for keyset-driven cursors (default 0).
    pub keyset_size: sql::ULen,
    /// `SQL_ATTR_SIMULATE_CURSOR` — simulate positioned update/delete (default SQL_SC_NON_UNIQUE = 0).
    pub simulate_cursor: sql::ULen,
    /// `SQL_ATTR_RETRIEVE_DATA` — whether to retrieve data after positioned update (default SQL_RD_ON = 1).
    pub retrieve_data: sql::ULen,
    /// `SQL_SF_STMT_ATTR_LAST_QUERY_ID` — query ID from the last successful execution (read-only).
    /// `None` before any execution; `Some("")` if sf_core returned an empty string.
    pub last_query_id: Option<String>,
    /// Child query IDs for multi-statement execution (consumed by SQLMoreResults).
    pub multi_query_ids: Vec<String>,
    /// Index of the next child result to fetch in `multi_query_ids`.
    pub multi_current_idx: usize,
    /// `SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT` — multi-statement execution count.
    /// -1 = auto-detect (default), 0 = single statement, N > 0 = expect exactly N statements.
    pub multi_statement_count: i16,
    /// `SQL_ATTR_ASYNC_ENABLE` — whether async polling is enabled (default false).
    pub async_enabled: bool,
}

// Safety: StatementInner contains raw pointers (descriptor fields like bind_offset_ptr,
// array_status_ptr) that make it !Send + !Sync. These are application-owned pointers
// that are only dereferenced on the calling thread. This temporary unsafe impl allows
// Mutex<StatementInner> to work; PR 5 removes it by adding proper interior mutability.
unsafe impl Send for StatementInner {}
unsafe impl Sync for StatementInner {}

impl Statement {
    /// Construct a new Statement for the given connection.
    pub fn new(conn_id: HandleId, stmt_handle: StatementHandle, metadata_id: bool) -> Self {
        Self {
            conn_id,
            stmt_handle,
            inner: parking_lot::Mutex::new(StatementInner {
                state: StatementState::Created.into(),
                ard: ArdDescriptor::new(),
                ird: IrdDescriptor::new(),
                apd: ApdDescriptor::new(),
                ipd: IpdDescriptor::new(),
                ard_handle: HandleId::default(),
                ird_handle: HandleId::default(),
                apd_handle: HandleId::default(),
                ipd_handle: HandleId::default(),
                diagnostic_info: DiagnosticInfo::default(),
                get_data_state: None,
                cursor_type: CursorType::ForwardOnly,
                max_length: 0,
                used_extended_fetch: false,
                prepared_param_count: None,
                prepared_array_bind_supported: None,
                metadata_id,
                query_timeout: 0,
                noscan: SQL_NOSCAN_OFF,
                max_rows: 0,
                rows_returned: 0,
                concurrency: SQL_CONCUR_READ_ONLY,
                cursor_scrollable: SQL_NONSCROLLABLE,
                cursor_sensitivity: SQL_UNSPECIFIED,
                keyset_size: 0,
                simulate_cursor: SQL_SC_NON_UNIQUE,
                retrieve_data: SQL_RD_ON,
                last_query_id: None,
                multi_query_ids: Vec::new(),
                multi_current_idx: 0,
                multi_statement_count: -1,
                async_enabled: false,
            }),
            cancel_token: parking_lot::Mutex::new(None),
        }
    }

    /// Look up the parent connection via the global dbc_registry.
    ///
    /// Callers should take this guard only when they will (a) lock
    /// `dbc.connection`, (b) read the session `conn_handle` from
    /// `ConnectionState::Connected`, or (c) otherwise dereference connection
    /// state. Functions that only mutate `Statement::inner` or
    /// `Statement::cancel_token` (e.g. `SQLBindCol`, `SQLBindParameter`,
    /// `SQLPutData`, `SQLSetStmtAttr`, `SQLCancel`) do not need this.
    ///
    /// Returns an error if the parent connection has already been freed.
    pub fn conn(&self) -> OdbcResult<HandleGuard<Dbc>> {
        global()
            .context(OdbcRuntimeSnafu)?
            .dbc_registry
            .get(self.conn_id)
    }
}

// Helper functions for handle conversion
pub fn env_from_handle(handle: sql::Handle) -> OdbcResult<HandleGuard<Env>> {
    let handle_id = HandleId::from(handle);
    global()
        .context(OdbcRuntimeSnafu)?
        .env_registry
        .get(handle_id)
}

pub fn conn_from_handle(handle: sql::Handle) -> OdbcResult<HandleGuard<Dbc>> {
    let handle_id = HandleId::from(handle);
    global()
        .context(OdbcRuntimeSnafu)?
        .dbc_registry
        .get(handle_id)
}

pub fn stmt_from_handle(handle: sql::Handle) -> OdbcResult<HandleGuard<Statement>> {
    let handle_id = HandleId::from(handle);
    global()
        .context(OdbcRuntimeSnafu)?
        .stmt_registry
        .get(handle_id)
}

pub fn desc_from_handle(
    desc_handle: sql::Handle,
) -> OdbcResult<(HandleGuard<Statement>, DescriptorKind)> {
    if desc_handle.is_null() {
        return Err(OdbcError::InvalidHandle {
            location: snafu::location!(),
        });
    }
    let desc_id = HandleId::from(desc_handle);
    let g = global().context(OdbcRuntimeSnafu)?;
    let desc_guard = g.desc_manager.get(desc_id)?;
    let stmt_id = desc_guard.stmt_id;
    let kind = desc_guard.kind;
    drop(desc_guard);
    let stmt_guard = g.stmt_registry.get(stmt_id)?;
    Ok((stmt_guard, kind))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the Snowflake-vendor-code → standard-ODBC-type normalisation that
    /// `bind_parameter` relies on to keep `SQLDescribeParam` and
    /// `SQLGetDescField(IPD, SQL_DESC_TYPE)` returning spec-mandated codes.
    #[test]
    fn from_parameter_type_recognises_vendor_codes() {
        assert_eq!(
            TimestampSubtype::from_parameter_type(SQL_SF_TIMESTAMP_NTZ),
            Some(TimestampSubtype::Ntz)
        );
        assert_eq!(
            TimestampSubtype::from_parameter_type(SQL_SF_TIMESTAMP_LTZ),
            Some(TimestampSubtype::Ltz)
        );
        assert_eq!(
            TimestampSubtype::from_parameter_type(SQL_SF_TIMESTAMP_TZ),
            Some(TimestampSubtype::Tz)
        );
    }

    /// Standard ODBC SQL type codes -- and TIMESTAMP in particular -- must
    /// not be classified as vendor opt-ins. `None` here is what keeps the
    /// dispatch in `make_converter` defaulting to NTZ for backward-compat
    /// callers that bind via the standard `SQL_TYPE_TIMESTAMP`.
    #[test]
    fn from_parameter_type_returns_none_for_standard_codes() {
        assert_eq!(
            TimestampSubtype::from_parameter_type(sql::SqlDataType::TIMESTAMP),
            None
        );
        assert_eq!(
            TimestampSubtype::from_parameter_type(sql::SqlDataType::INTEGER),
            None
        );
        assert_eq!(
            TimestampSubtype::from_parameter_type(sql::SqlDataType::VARCHAR),
            None
        );
        // SQL_TYPE_TIMESTAMP_WITH_TIMEZONE (95) is a standard ODBC type, not
        // the Snowflake vendor TZ code (2001), and must not be treated as one.
        assert_eq!(
            TimestampSubtype::from_parameter_type(sql::SqlDataType(95)),
            None
        );
    }

    /// Pin the on-the-wire mapping for `SQLGetInfo` codes the new ODBC driver
    /// claims to support. Excel/PowerQuery probes 6/7/17/18/77/81 during
    /// `SQLDriverConnect`; bumping these values breaks the AS-bound trace
    /// replay tests and breaks application discovery.
    #[test]
    fn info_type_try_from_round_trip() {
        let cases: &[(u16, InfoType)] = &[
            (6, InfoType::DriverName),
            (7, InfoType::DriverVer),
            (14, InfoType::SearchPatternEscape),
            (17, InfoType::DbmsName),
            (18, InfoType::DbmsVer),
            (22, InfoType::ConcatNullBehavior),
            (23, InfoType::CursorCommitBehavior),
            (24, InfoType::CursorRollbackBehavior),
            (29, InfoType::IdentifierQuoteChar),
            (39, InfoType::SchemaTerm),
            (41, InfoType::CatalogNameSeparator),
            (42, InfoType::CatalogTerm),
            (48, InfoType::ConvertFunctions),
            (49, InfoType::NumericFunctions),
            (50, InfoType::StringFunctions),
            (51, InfoType::SystemFunctions),
            (52, InfoType::TimedateFunctions),
            (53, InfoType::ConvertBigint),
            (54, InfoType::ConvertBinary),
            (55, InfoType::ConvertBit),
            (56, InfoType::ConvertChar),
            (57, InfoType::ConvertDate),
            (58, InfoType::ConvertDecimal),
            (59, InfoType::ConvertDouble),
            (60, InfoType::ConvertFloat),
            (61, InfoType::ConvertInteger),
            (62, InfoType::ConvertLongVarchar),
            (63, InfoType::ConvertNumeric),
            (64, InfoType::ConvertReal),
            (65, InfoType::ConvertSmallint),
            (66, InfoType::ConvertTime),
            (67, InfoType::ConvertTimestamp),
            (68, InfoType::ConvertTinyint),
            (69, InfoType::ConvertVarbinary),
            (70, InfoType::ConvertVarchar),
            (71, InfoType::ConvertLongVarbinary),
            (77, InfoType::DriverOdbcVer),
            (144, InfoType::DynamicCursorAttributes1),
            (81, InfoType::GetDataExtensions),
            (87, InfoType::ColumnAlias),
            (88, InfoType::GroupBy),
            (90, InfoType::OrderByColumnsInSelect),
            (91, InfoType::SchemaUsage),
            (92, InfoType::CatalogUsage),
            (94, InfoType::SpecialCharacters),
            (97, InfoType::MaxColumnsInGroupBy),
            (99, InfoType::MaxColumnsInOrderBy),
            (100, InfoType::MaxColumnsInSelect),
            (109, InfoType::TimedateAddIntervals),
            (110, InfoType::TimedateDiffIntervals),
            (114, InfoType::CatalogLocation),
            (118, InfoType::SqlConformance),
            (122, InfoType::ConvertWchar),
            (125, InfoType::ConvertWlongVarchar),
            (126, InfoType::ConvertWvarchar),
            (152, InfoType::OdbcInterfaceConformance),
            (160, InfoType::Sql92Predicates),
            (161, InfoType::Sql92RelationalJoinOperators),
            (165, InfoType::Sql92ValueExpressions),
            (169, InfoType::AggregateFunctions),
            (173, InfoType::ConvertGuid),
            (10003, InfoType::CatalogName),
            (10005, InfoType::MaxIdentifierLen),
            (10021, InfoType::AsyncMode),
            (10022, InfoType::MaxAsyncConcurrentStatements),
            (10023, InfoType::AsyncDbcFunctions),
            (10025, InfoType::AsyncNotification),
        ];
        for (raw, expected) in cases {
            assert_eq!(
                InfoType::try_from(*raw).unwrap(),
                *expected,
                "raw={raw} expected={expected:?}",
            );
        }

        match InfoType::try_from(9999_u16) {
            Err(OdbcError::UnknownInfoType { info_type, .. }) => assert_eq!(info_type, 9999),
            other => panic!("expected UnknownInfoType, got {other:?}"),
        }
    }

    /// Excel uses `SQLColAttribute(SQL_DESC_NAME)` and `SQL_DESC_LENGTH` while
    /// rendering result-set previews. Pin the discriminants so descriptor
    /// dispatch keeps routing to the column-name / column-size code paths.
    #[test]
    fn desc_field_try_from_round_trip() {
        assert_eq!(DescField::try_from(1003_i16).unwrap(), DescField::Length);
        assert_eq!(DescField::try_from(1011_i16).unwrap(), DescField::Name);
        assert_eq!(DescField::try_from(2_i16).unwrap(), DescField::ConciseType);
        assert_eq!(DescField::try_from(1002_i16).unwrap(), DescField::Type);

        match DescField::try_from(-1_i16) {
            Err(OdbcError::InvalidDescriptorFieldId { field_id, .. }) => assert_eq!(field_id, -1),
            other => panic!("expected InvalidDescriptorFieldId, got {other:?}"),
        }
    }
}
