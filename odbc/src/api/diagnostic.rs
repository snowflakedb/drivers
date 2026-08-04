//! ODBC diagnostic functions
//!
//! This module provides functions for retrieving diagnostic information
//! from ODBC handles, including error messages, SQL states, and native error codes.

use crate::{
    api::{
        Environment, OdbcError, OdbcResult, SqlState, conn_from_handle, desc_from_handle,
        encoding::{OdbcEncoding, write_string_bytes, write_string_chars},
        env_from_handle,
        error::{
            InvalidDiagnosticIdentifierSnafu, InvalidHandleSnafu, InvalidRecordNumberSnafu,
            NoMoreDataSnafu,
        },
        query_type::QueryType,
        stmt_from_handle,
        types::{DescriptorAccess, DescriptorKind},
    },
    conversion::warning::{Warning, Warnings},
};
use odbc_sys as sql;

/// ODBC Diagnostic Identifiers according to the ODBC standard
///
/// These identifiers are used with SQLGetDiagField to retrieve specific
/// diagnostic information from diagnostic records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum DiagIdentifier {
    /// SQL_DIAG_RETURNCODE - Return code of the function
    ReturnCode = 1,
    /// SQL_DIAG_NUMBER - Number of diagnostic records
    Number = 2,
    /// SQL_DIAG_ROW_COUNT - Number of rows affected by the statement
    RowCount = 3,
    /// SQL_DIAG_SQLSTATE - SQLSTATE value
    SqlState = 4,
    /// SQL_DIAG_NATIVE - Native error code
    Native = 5,
    /// SQL_DIAG_MESSAGE_TEXT - Diagnostic message text
    MessageText = 6,
    /// SQL_DIAG_DYNAMIC_FUNCTION - Name of the SQL statement executed
    DynamicFunction = 7,
    /// SQL_DIAG_CLASS_ORIGIN - Class origin (ISO 9075 or ODBC 3.0)
    ClassOrigin = 8,
    /// SQL_DIAG_SUBCLASS_ORIGIN - Subclass origin
    SubclassOrigin = 9,
    /// SQL_DIAG_CONNECTION_NAME - Connection name
    ConnectionName = 10,
    /// SQL_DIAG_SERVER_NAME - Server name
    ServerName = 11,
    /// SQL_DIAG_DYNAMIC_FUNCTION_CODE - Dynamic function code
    DynamicFunctionCode = 12,
    /// SQL_DIAG_CURSOR_ROW_COUNT - Number of rows in the cursor
    CursorRowCount = 13,
    /// SQL_DIAG_ROW_NUMBER - Row number where the error occurred
    RowNumber = 14,
    /// SQL_DIAG_COLUMN_NUMBER - Column number where the error occurred
    ColumnNumber = 15,
}

impl TryFrom<sql::SmallInt> for DiagIdentifier {
    type Error = OdbcError;

    fn try_from(value: sql::SmallInt) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(DiagIdentifier::ReturnCode),
            2 => Ok(DiagIdentifier::Number),
            3 => Ok(DiagIdentifier::RowCount),
            4 => Ok(DiagIdentifier::SqlState),
            5 => Ok(DiagIdentifier::Native),
            6 => Ok(DiagIdentifier::MessageText),
            7 => Ok(DiagIdentifier::DynamicFunction),
            8 => Ok(DiagIdentifier::ClassOrigin),
            9 => Ok(DiagIdentifier::SubclassOrigin),
            10 => Ok(DiagIdentifier::ConnectionName),
            11 => Ok(DiagIdentifier::ServerName),
            12 => Ok(DiagIdentifier::DynamicFunctionCode),
            13 => Ok(DiagIdentifier::CursorRowCount),
            14 => Ok(DiagIdentifier::RowNumber),
            15 => Ok(DiagIdentifier::ColumnNumber),
            _ => InvalidDiagnosticIdentifierSnafu { identifier: value }.fail(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DiagnosticHeader {
    cursor_row_count: Option<sql::Len>,
    /// SQL_DIAG_DYNAMIC_FUNCTION: string name of the executed SQL statement type
    /// (e.g. "INSERT", "SELECT CURSOR"). Set after SQLExecute/SQLExecDirect.
    pub dynamic_function: Option<String>,
    /// SQL_DIAG_DYNAMIC_FUNCTION_CODE: integer code for the statement type.
    pub dynamic_function_code: Option<sql::Integer>,
    number_of_records: Option<sql::Integer>,
    pub return_code: sql::RetCode,
    pub row_count: Option<sql::Len>,
}

/// ODBC 3.0 spec: class_origin / subclass_origin is "ISO 9075" for SQL-standard
/// SQLSTATE classes, "ODBC 3.0" for ODBC-defined ones.
///
/// Every SQLSTATE reaching these helpers is a well-formed 5-char code by
/// construction: the fixed `SqlState` variants (locked by the sql_state tests)
/// and `Unknown`, which `error.rs` only builds after `is_well_formed_sql_state`.
/// So we classify directly by the class/subclass without length guards.
///
/// ODBC-defined class prefixes: HY (generic ODBC errors), IM (Driver Manager),
/// 0Z (deprecated). Everything else is ISO-defined.
pub fn class_origin_for_sqlstate(sqlstate: &str) -> ClassOrigin {
    match sqlstate.get(..2) {
        Some("HY") | Some("IM") | Some("0Z") => ClassOrigin::Odbc3_0,
        _ => ClassOrigin::Iso9075,
    }
}

/// A subclass is ODBC-defined when its first character is '5'-'9' or 'A'-'Z'
/// (implementation-defined space); '0'-'4' (which includes the "000" no-subclass
/// case) is SQL-standard. An ODBC-defined class always has an ODBC-defined subclass.
pub fn subclass_origin_for_sqlstate(sqlstate: &str) -> ClassOrigin {
    if matches!(class_origin_for_sqlstate(sqlstate), ClassOrigin::Odbc3_0) {
        return ClassOrigin::Odbc3_0;
    }
    match sqlstate.as_bytes().get(2) {
        Some(&b) if b.is_ascii_digit() && b <= b'4' => ClassOrigin::Iso9075,
        _ => ClassOrigin::Odbc3_0,
    }
}

#[derive(Debug, Clone, Default)]
pub enum ClassOrigin {
    #[default]
    Odbc3_0,
    Iso9075,
}

#[derive(Debug, Clone, Default)]
pub struct DiagnosticRecord {
    pub class_origin: ClassOrigin,
    pub subclass_origin: ClassOrigin,
    pub column_number: Option<sql::Integer>,
    /// Row number where the error occurred. ODBC spec requires SQLLEN (pointer-sized).
    pub row_number: Option<sql::Len>,
    pub server_name: String,
    pub connection_name: String,
    pub message_text: String,
    pub sql_state: SqlState,
    pub native_error: sql::Integer,
}

#[derive(Debug, Clone, Default)]
pub struct DiagnosticInfo {
    header: DiagnosticHeader,
    records: Vec<DiagnosticRecord>,
}

impl DiagnosticInfo {
    pub fn add_record(&mut self, record: DiagnosticRecord) {
        self.records.push(record);
    }

    pub fn clear(&mut self) {
        self.header = DiagnosticHeader::default();
        self.records.clear();
    }

    /// Set SQL_DIAG_RETURNCODE in the header. Called after the final return
    /// code for a function is known (after both result and warnings are merged).
    pub fn set_return_code(&mut self, code: sql::RetCode) {
        self.header.return_code = code;
    }

    /// Set execution-derived header fields (SQL_DIAG_ROW_COUNT,
    /// SQL_DIAG_DYNAMIC_FUNCTION, SQL_DIAG_DYNAMIC_FUNCTION_CODE) directly
    /// from the execution response. Called from within statement.rs while
    /// the statement lock is already held.
    pub fn set_execution_info(
        &mut self,
        statement_type_id: Option<i64>,
        rows_affected: Option<i64>,
    ) {
        let qt = QueryType::from_raw(statement_type_id);
        let (fn_name, fn_code) = query_type_to_dynamic_function(qt);
        self.header.row_count = rows_affected.map(|v| v as sql::Len);
        self.header.dynamic_function = Some(fn_name.to_owned());
        self.header.dynamic_function_code = Some(fn_code);
    }
}

pub trait WithDiagnosticInfo {
    fn get_diag_info(&self) -> &DiagnosticInfo;
    fn get_diag_info_mut(&mut self) -> &mut DiagnosticInfo;
}

// TODO: With changes to the environment locking mechanism we need to
// rework this solution so that we do not acquire the environment 3 times during one call
impl WithDiagnosticInfo for Environment {
    fn get_diag_info(&self) -> &DiagnosticInfo {
        &self.diagnostic_info
    }
    fn get_diag_info_mut(&mut self) -> &mut DiagnosticInfo {
        &mut self.diagnostic_info
    }
}

impl WithDiagnosticInfo for crate::api::Connection {
    fn get_diag_info(&self) -> &DiagnosticInfo {
        &self.diagnostic_info
    }
    fn get_diag_info_mut(&mut self) -> &mut DiagnosticInfo {
        &mut self.diagnostic_info
    }
}

impl WithDiagnosticInfo for crate::api::StatementInner {
    fn get_diag_info(&self) -> &DiagnosticInfo {
        &self.diagnostic_info
    }
    fn get_diag_info_mut(&mut self) -> &mut DiagnosticInfo {
        &mut self.diagnostic_info
    }
}

impl WithDiagnosticInfo for crate::api::types::ArdDescriptor {
    fn get_diag_info(&self) -> &DiagnosticInfo {
        &self.diagnostic_info
    }
    fn get_diag_info_mut(&mut self) -> &mut DiagnosticInfo {
        &mut self.diagnostic_info
    }
}

impl WithDiagnosticInfo for crate::api::types::ApdDescriptor {
    fn get_diag_info(&self) -> &DiagnosticInfo {
        &self.diagnostic_info
    }
    fn get_diag_info_mut(&mut self) -> &mut DiagnosticInfo {
        &mut self.diagnostic_info
    }
}

impl WithDiagnosticInfo for crate::api::types::IrdDescriptor {
    fn get_diag_info(&self) -> &DiagnosticInfo {
        &self.diagnostic_info
    }
    fn get_diag_info_mut(&mut self) -> &mut DiagnosticInfo {
        &mut self.diagnostic_info
    }
}

impl WithDiagnosticInfo for crate::api::types::IpdDescriptor {
    fn get_diag_info(&self) -> &DiagnosticInfo {
        &self.diagnostic_info
    }
    fn get_diag_info_mut(&mut self) -> &mut DiagnosticInfo {
        &mut self.diagnostic_info
    }
}

pub fn clear_diag_info(handle_type: sql::HandleType, handle: sql::Handle) {
    if handle.is_null() {
        return;
    }
    if handle_type == sql::HandleType::Env {
        let Ok(env) = env_from_handle(handle) else {
            return;
        };
        let mut guard = env.environment.lock();
        guard.get_diag_info_mut().clear();
        return;
    }
    if handle_type == sql::HandleType::Dbc {
        let Ok(dbc) = conn_from_handle(handle) else {
            return;
        };
        dbc.connection.lock().get_diag_info_mut().clear();
        return;
    }
    if handle_type == sql::HandleType::Stmt {
        let Ok(guard) = stmt_from_handle(handle) else {
            return;
        };
        guard.inner.lock().get_diag_info_mut().clear();
        return;
    }
    if handle_type == sql::HandleType::Desc {
        let Ok(access) = desc_from_handle(handle) else {
            return;
        };
        match access {
            DescriptorAccess::Implicit { guard, kind } => {
                let mut inner = guard.inner.lock();
                match kind {
                    DescriptorKind::Ard => inner.ard.get_diag_info_mut().clear(),
                    DescriptorKind::Ird => inner.ird.get_diag_info_mut().clear(),
                    DescriptorKind::Apd => inner.apd.get_diag_info_mut().clear(),
                    DescriptorKind::Ipd => inner.ipd.get_diag_info_mut().clear(),
                }
            }
            DescriptorAccess::Explicit { desc } => {
                desc.lock().get_diag_info_mut().clear();
            }
        }
    }
}

pub fn from_warning(warning: &Warning) -> DiagnosticRecord {
    let message_text = match warning {
        Warning::StringDataTruncated => "String data truncated",
        Warning::NumericValueTruncated => "Numeric value truncated",
        Warning::RowError => "Error in row",
        Warning::OptionValueChanged => "Option value changed",
    };
    let sql_state = match warning {
        Warning::StringDataTruncated => SqlState::StringDataRightTruncated,
        Warning::NumericValueTruncated => SqlState::FractionalTruncation,
        Warning::RowError => SqlState::ErrorInRow,
        Warning::OptionValueChanged => SqlState::OptionValueChanged,
    };
    let state_str = sql_state.as_str();
    DiagnosticRecord {
        native_error: 0,
        class_origin: class_origin_for_sqlstate(state_str),
        subclass_origin: subclass_origin_for_sqlstate(state_str),
        sql_state,
        message_text: message_text.to_string(),
        ..Default::default()
    }
}

pub fn set_diag_info_from_warnings(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    warnings: &Warnings,
) {
    if handle.is_null() || warnings.is_empty() {
        return;
    }
    let add_warnings = |diagnostic_info: &mut DiagnosticInfo| {
        for warning in warnings {
            diagnostic_info.add_record(from_warning(warning));
        }
        // Upgrade SQL_SUCCESS → SQL_SUCCESS_WITH_INFO now that we have warnings.
        if diagnostic_info.header.return_code == sql::SqlReturn::SUCCESS.0 {
            diagnostic_info.header.return_code = sql::SqlReturn::SUCCESS_WITH_INFO.0;
        }
    };
    if handle_type == sql::HandleType::Env {
        let Ok(env) = env_from_handle(handle) else {
            return;
        };
        add_warnings(env.environment.lock().get_diag_info_mut());
        return;
    }
    if handle_type == sql::HandleType::Dbc {
        let Ok(dbc) = conn_from_handle(handle) else {
            return;
        };
        add_warnings(dbc.connection.lock().get_diag_info_mut());
        return;
    }
    if handle_type == sql::HandleType::Stmt {
        let Ok(guard) = stmt_from_handle(handle) else {
            return;
        };
        add_warnings(guard.inner.lock().get_diag_info_mut());
        return;
    }
    if handle_type == sql::HandleType::Desc {
        let Ok(access) = desc_from_handle(handle) else {
            return;
        };
        match access {
            DescriptorAccess::Implicit { guard, kind } => {
                let mut inner = guard.inner.lock();
                let diagnostic_info = match kind {
                    DescriptorKind::Ard => inner.ard.get_diag_info_mut(),
                    DescriptorKind::Ird => inner.ird.get_diag_info_mut(),
                    DescriptorKind::Apd => inner.apd.get_diag_info_mut(),
                    DescriptorKind::Ipd => inner.ipd.get_diag_info_mut(),
                };
                add_warnings(diagnostic_info);
            }
            DescriptorAccess::Explicit { desc } => add_warnings(desc.lock().get_diag_info_mut()),
        }
    }
}

pub fn set_diag_info_from_result<T>(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    result: &OdbcResult<T>,
) {
    if handle.is_null() {
        return;
    }
    // Provisional return code from result alone (may be upgraded to
    // SQL_SUCCESS_WITH_INFO later by set_diag_info_from_warnings).
    let provisional_return_code: sql::RetCode = match result {
        Ok(_) => sql::SqlReturn::SUCCESS.0,
        Err(OdbcError::NoMoreData { .. }) => sql::SqlReturn::NO_DATA.0,
        Err(OdbcError::InvalidHandle { .. }) => sql::SqlReturn::INVALID_HANDLE.0,
        Err(OdbcError::DaeRequired { .. }) => sql::SqlReturn::NEED_DATA.0,
        Err(OdbcError::StillExecuting { .. }) => sql::SqlReturn::STILL_EXECUTING.0,
        Err(_) => sql::SqlReturn::ERROR.0,
    };
    let add_from_result = |diagnostic_info: &mut DiagnosticInfo, server_name: Option<&str>| {
        diagnostic_info.set_return_code(provisional_return_code);
        match result {
            Ok(_) => {}
            Err(OdbcError::DaeRequired { .. }) => {}
            Err(OdbcError::StillExecuting { .. }) => {}
            Err(error) => {
                let mut record = error.to_diagnostic_record();
                if let Some(name) = server_name {
                    record.server_name = name.to_owned();
                }
                diagnostic_info.add_record(record);
            }
        }
    };
    if handle_type == sql::HandleType::Env {
        let Ok(env) = env_from_handle(handle) else {
            return;
        };
        let mut guard = env.environment.lock();
        add_from_result(guard.get_diag_info_mut(), None);
        return;
    }
    if handle_type == sql::HandleType::Dbc {
        let Ok(dbc) = conn_from_handle(handle) else {
            return;
        };
        let mut conn = dbc.connection.lock();
        let dsn = conn.dsn_name.clone();
        add_from_result(conn.get_diag_info_mut(), dsn.as_deref());
        return;
    }
    if handle_type == sql::HandleType::Stmt {
        let Ok(guard) = stmt_from_handle(handle) else {
            return;
        };
        add_from_result(guard.inner.lock().get_diag_info_mut(), None);
        return;
    }
    if handle_type == sql::HandleType::Desc {
        let Ok(access) = desc_from_handle(handle) else {
            return;
        };
        match access {
            DescriptorAccess::Implicit { guard, kind } => {
                let mut inner = guard.inner.lock();
                let diagnostic_info = match kind {
                    DescriptorKind::Ard => inner.ard.get_diag_info_mut(),
                    DescriptorKind::Ird => inner.ird.get_diag_info_mut(),
                    DescriptorKind::Apd => inner.apd.get_diag_info_mut(),
                    DescriptorKind::Ipd => inner.ipd.get_diag_info_mut(),
                };
                add_from_result(diagnostic_info, None);
            }
            DescriptorAccess::Explicit { desc } => {
                add_from_result(desc.lock().get_diag_info_mut(), None);
            }
        }
    }
}

pub fn get_diag_info(
    handle_type: sql::HandleType,
    handle: sql::Handle,
) -> OdbcResult<DiagnosticInfo> {
    if handle_type == sql::HandleType::Env {
        let env = env_from_handle(handle)?;
        let guard = env.environment.lock();
        return Ok(guard.get_diag_info().clone());
    }
    if handle_type == sql::HandleType::Dbc {
        let dbc = conn_from_handle(handle)?;
        return Ok(dbc.connection.lock().get_diag_info().clone());
    }
    if handle_type == sql::HandleType::Stmt {
        let guard = stmt_from_handle(handle)?;
        return Ok(guard.inner.lock().get_diag_info().clone());
    }
    if handle_type == sql::HandleType::Desc {
        let access = desc_from_handle(handle)?;
        return match access {
            DescriptorAccess::Implicit { guard, kind } => {
                let inner = guard.inner.lock();
                let diag = match kind {
                    DescriptorKind::Ard => inner.ard.get_diag_info(),
                    DescriptorKind::Ird => inner.ird.get_diag_info(),
                    DescriptorKind::Apd => inner.apd.get_diag_info(),
                    DescriptorKind::Ipd => inner.ipd.get_diag_info(),
                };
                Ok(diag.clone())
            }
            DescriptorAccess::Explicit { desc } => Ok(desc.lock().get_diag_info().clone()),
        };
    }
    InvalidHandleSnafu.fail()
}

/// Get diagnostic record from handle (SQLGetDiagRec / SQLGetDiagRecW).
///
/// Retrieves diagnostic information associated with a specific handle.
///
/// Per the ODBC spec, `text_length_ptr` always receives the full (untruncated)
/// message length so the caller can allocate a sufficiently large buffer.
/// If the message is truncated, a `StringDataTruncated` warning is pushed.
#[allow(clippy::too_many_arguments)]
pub unsafe fn get_diag_rec<E: OdbcEncoding>(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    rec_number: sql::SmallInt,
    sql_state: *mut E::Char,
    native_error_ptr: *mut sql::Integer,
    message_text: *mut E::Char,
    buffer_length: sql::SmallInt,
    text_length_ptr: *mut sql::SmallInt,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    use crate::api::error::InvalidBufferLengthSnafu;
    if buffer_length < 0 {
        return InvalidBufferLengthSnafu {
            length: buffer_length as i64,
        }
        .fail();
    }
    let diagnostic_info = get_diag_info(handle_type, handle)?;
    if rec_number <= 0 {
        return InvalidRecordNumberSnafu { number: rec_number }.fail();
    }

    if rec_number > diagnostic_info.records.len() as i16 {
        return NoMoreDataSnafu.fail();
    }

    let Some(record) = diagnostic_info.records.get((rec_number - 1) as usize) else {
        return NoMoreDataSnafu.fail();
    };

    let state = &record.sql_state.as_str()[..5.min(record.sql_state.as_str().len())];
    write_string_chars::<E>(state, sql_state, 6, std::ptr::null_mut(), None);
    write_string_chars::<E>(
        &record.message_text,
        message_text,
        buffer_length,
        text_length_ptr,
        Some(warnings),
    );

    unsafe {
        if !native_error_ptr.is_null() {
            std::ptr::write(native_error_ptr, record.native_error);
        }
    }

    Ok(())
}

/// Get diagnostic field from handle (SQLGetDiagField / SQLGetDiagFieldW).
///
/// Retrieves a specific diagnostic field from a diagnostic record.
/// `warnings` is populated when a string field is truncated (SQL_SUCCESS_WITH_INFO / 01004).
#[allow(clippy::too_many_arguments)]
pub fn get_diag_field<E: OdbcEncoding>(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    rec_number: sql::SmallInt,
    diag_identifier: sql::SmallInt,
    diag_info_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    let diagnostic_info = get_diag_info(handle_type, handle)?;
    tracing::debug!(
        "get_diag_field: handle_type={:?}, rec_number={rec_number}, diag_identifier={diag_identifier:?}",
        handle_type,
    );
    if rec_number < 0 {
        return InvalidRecordNumberSnafu { number: rec_number }.fail();
    }

    let diag_id = DiagIdentifier::try_from(diag_identifier)?;

    if rec_number == 0 {
        match diag_id {
            DiagIdentifier::Number => {
                let count = diagnostic_info
                    .header
                    .number_of_records
                    .unwrap_or(diagnostic_info.records.len() as sql::Integer);
                unsafe {
                    std::ptr::write(diag_info_ptr as *mut sql::Integer, count);
                }
                Ok(())
            }
            DiagIdentifier::ReturnCode => {
                unsafe {
                    std::ptr::write(
                        diag_info_ptr as *mut sql::RetCode,
                        diagnostic_info.header.return_code,
                    );
                }
                Ok(())
            }
            DiagIdentifier::RowCount => {
                unsafe {
                    std::ptr::write(
                        diag_info_ptr as *mut sql::Len,
                        diagnostic_info.header.row_count.unwrap_or(0),
                    );
                }
                Ok(())
            }
            DiagIdentifier::DynamicFunction => {
                let name = diagnostic_info
                    .header
                    .dynamic_function
                    .as_deref()
                    .unwrap_or("");
                write_string_bytes::<E>(
                    name,
                    diag_info_ptr as *mut E::Char,
                    buffer_length,
                    string_length_ptr,
                    Some(warnings),
                );
                Ok(())
            }
            DiagIdentifier::DynamicFunctionCode => {
                unsafe {
                    std::ptr::write(
                        diag_info_ptr as *mut sql::Integer,
                        diagnostic_info.header.dynamic_function_code.unwrap_or(0),
                    );
                }
                Ok(())
            }
            DiagIdentifier::CursorRowCount => {
                unsafe {
                    std::ptr::write(
                        diag_info_ptr as *mut sql::Len,
                        diagnostic_info.header.cursor_row_count.unwrap_or(0),
                    );
                }
                Ok(())
            }
            _ => NoMoreDataSnafu.fail(),
        }
    } else {
        if rec_number > diagnostic_info.records.len() as i16 {
            return NoMoreDataSnafu.fail();
        }

        let record = &diagnostic_info.records[(rec_number - 1) as usize];

        match diag_id {
            DiagIdentifier::SqlState => {
                write_string_bytes::<E>(
                    record.sql_state.as_str(),
                    diag_info_ptr as *mut E::Char,
                    buffer_length,
                    string_length_ptr,
                    Some(warnings),
                );
                Ok(())
            }
            DiagIdentifier::Native => {
                unsafe {
                    std::ptr::write(diag_info_ptr as *mut sql::Integer, record.native_error);
                }
                Ok(())
            }
            DiagIdentifier::MessageText => {
                write_string_bytes::<E>(
                    &record.message_text,
                    diag_info_ptr as *mut E::Char,
                    buffer_length,
                    string_length_ptr,
                    Some(warnings),
                );
                Ok(())
            }
            DiagIdentifier::ClassOrigin => {
                let origin_str = match record.class_origin {
                    ClassOrigin::Odbc3_0 => "ODBC 3.0",
                    ClassOrigin::Iso9075 => "ISO 9075",
                };
                write_string_bytes::<E>(
                    origin_str,
                    diag_info_ptr as *mut E::Char,
                    buffer_length,
                    string_length_ptr,
                    Some(warnings),
                );
                Ok(())
            }
            DiagIdentifier::SubclassOrigin => {
                let origin_str = match record.subclass_origin {
                    ClassOrigin::Odbc3_0 => "ODBC 3.0",
                    ClassOrigin::Iso9075 => "ISO 9075",
                };
                write_string_bytes::<E>(
                    origin_str,
                    diag_info_ptr as *mut E::Char,
                    buffer_length,
                    string_length_ptr,
                    Some(warnings),
                );
                Ok(())
            }
            DiagIdentifier::ConnectionName => {
                write_string_bytes::<E>(
                    &record.connection_name,
                    diag_info_ptr as *mut E::Char,
                    buffer_length,
                    string_length_ptr,
                    Some(warnings),
                );
                Ok(())
            }
            DiagIdentifier::ServerName => {
                write_string_bytes::<E>(
                    &record.server_name,
                    diag_info_ptr as *mut E::Char,
                    buffer_length,
                    string_length_ptr,
                    Some(warnings),
                );
                Ok(())
            }
            DiagIdentifier::ColumnNumber => {
                unsafe {
                    std::ptr::write(
                        diag_info_ptr as *mut sql::Integer,
                        record.column_number.unwrap_or(0),
                    );
                }
                Ok(())
            }
            DiagIdentifier::RowNumber => {
                unsafe {
                    // ODBC spec requires SQLLEN (pointer-sized integer) for SQL_DIAG_ROW_NUMBER.
                    std::ptr::write(
                        diag_info_ptr as *mut sql::Len,
                        record.row_number.unwrap_or(0),
                    );
                }
                Ok(())
            }
            _ => NoMoreDataSnafu.fail(),
        }
    }
}

/// Map a Snowflake QueryType to the ODBC SQL_DIAG_DYNAMIC_FUNCTION string and
/// SQL_DIAG_DYNAMIC_FUNCTION_CODE integer per ODBC 3.x Appendix B.
fn query_type_to_dynamic_function(qt: QueryType) -> (&'static str, sql::Integer) {
    // ODBC 3.x function-code constants (from sqlext.h)
    const SQL_DIAG_SELECT_CURSOR: sql::Integer = 85;
    const SQL_DIAG_INSERT: sql::Integer = 20;
    const SQL_DIAG_UPDATE_WHERE: sql::Integer = 82;
    const SQL_DIAG_DELETE_WHERE: sql::Integer = 19;
    const SQL_DIAG_CALL: sql::Integer = 7;
    const SQL_DIAG_UNKNOWN_STATEMENT: sql::Integer = 0;

    if qt.belongs_to(QueryType::SELECT)
        || qt.belongs_to(QueryType::SHOW)
        || qt.belongs_to(QueryType::DESCRIBE)
        || qt.belongs_to(QueryType::EXPLAIN)
        || qt.belongs_to(QueryType::LIST_FILES)
    {
        ("SELECT CURSOR", SQL_DIAG_SELECT_CURSOR)
    } else if qt.belongs_to(QueryType::INSERT) {
        ("INSERT", SQL_DIAG_INSERT)
    } else if qt.belongs_to(QueryType::UPDATE) {
        ("UPDATE WHERE", SQL_DIAG_UPDATE_WHERE)
    } else if qt.belongs_to(QueryType::DELETE) {
        ("DELETE WHERE", SQL_DIAG_DELETE_WHERE)
    } else if qt.belongs_to(QueryType::CALL) {
        ("CALL", SQL_DIAG_CALL)
    } else {
        ("", SQL_DIAG_UNKNOWN_STATEMENT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::ToSqlReturn;

    /// The class-origin helpers take `&str`, but the only values that reach them
    /// are `SqlState::as_str()` results — a closed enum that
    /// `sql_state::tests::every_sql_state_is_a_well_formed_5_char_code` proves is
    /// always a 5-char code. Exercise the helpers over every real SQLSTATE to
    /// confirm they classify the whole known set without misbehaving, and
    /// spot-check the ODBC-defined vs ISO-defined split.
    #[test]
    fn class_origin_helpers_cover_every_sql_state() {
        use strum::IntoEnumIterator;
        for state in SqlState::iter() {
            let s = state.as_str();
            let _ = class_origin_for_sqlstate(s);
            let _ = subclass_origin_for_sqlstate(s);
        }
        assert!(matches!(
            class_origin_for_sqlstate("HY000"),
            ClassOrigin::Odbc3_0
        ));
        assert!(matches!(
            class_origin_for_sqlstate("IM001"),
            ClassOrigin::Odbc3_0
        ));
        assert!(matches!(
            class_origin_for_sqlstate("0Z002"),
            ClassOrigin::Odbc3_0
        ));
        assert!(matches!(
            class_origin_for_sqlstate("42S02"),
            ClassOrigin::Iso9075
        ));
        assert!(matches!(
            class_origin_for_sqlstate("01004"),
            ClassOrigin::Iso9075
        ));
    }

    #[test]
    fn invalid_handle_type_returns_sql_error_not_invalid_handle() {
        let result: OdbcResult<()> = crate::api::error::InvalidHandleTypeSnafu {
            handle_type: sql::HandleType::Desc as i16,
        }
        .fail();
        assert_eq!(result.to_sql_code(), sql::SqlReturn::ERROR.0);
    }

    #[test]
    fn class_origin_iso_for_standard_classes() {
        assert!(matches!(
            class_origin_for_sqlstate("01000"),
            ClassOrigin::Iso9075
        ));
        assert!(matches!(
            class_origin_for_sqlstate("22001"),
            ClassOrigin::Iso9075
        ));
        assert!(matches!(
            class_origin_for_sqlstate("42000"),
            ClassOrigin::Iso9075
        ));
    }

    #[test]
    fn class_origin_odbc_for_hy_im_0z() {
        assert!(matches!(
            class_origin_for_sqlstate("HY000"),
            ClassOrigin::Odbc3_0
        ));
        assert!(matches!(
            class_origin_for_sqlstate("IM002"),
            ClassOrigin::Odbc3_0
        ));
        assert!(matches!(
            class_origin_for_sqlstate("0Z000"),
            ClassOrigin::Odbc3_0
        ));
    }

    #[test]
    fn subclass_origin_iso_for_standard_subclass() {
        // 01000 — no subclass (000) → ISO
        assert!(matches!(
            subclass_origin_for_sqlstate("01000"),
            ClassOrigin::Iso9075
        ));
        // 22001 — subclass "001" (first char '0' ≤ '4') → ISO
        assert!(matches!(
            subclass_origin_for_sqlstate("22001"),
            ClassOrigin::Iso9075
        ));
    }

    #[test]
    fn subclass_origin_odbc_for_odbc_subclass() {
        // HY000 → ODBC class → subclass also ODBC
        assert!(matches!(
            subclass_origin_for_sqlstate("HY000"),
            ClassOrigin::Odbc3_0
        ));
        // 01S00 — subclass "S00" (first char 'S' > '4') → ODBC
        assert!(matches!(
            subclass_origin_for_sqlstate("01S00"),
            ClassOrigin::Odbc3_0
        ));
    }
}
