//! ODBC diagnostic functions
//!
//! This module provides functions for retrieving diagnostic information
//! from ODBC handles, including error messages, SQL states, and native error codes.

use crate::{
    api::{
        Connection, Environment, OdbcError, OdbcResult, SqlState, Statement, conn_from_handle,
        encoding::{OdbcEncoding, write_string_bytes, write_string_chars},
        env_from_handle,
        error::{
            InvalidDiagnosticIdentifierSnafu, InvalidHandleSnafu, InvalidRecordNumberSnafu,
            NoMoreDataSnafu,
        },
        stmt_from_handle,
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

impl DiagIdentifier {
    /// Convert DiagIdentifier to sql::SmallInt
    #[allow(dead_code)]
    pub fn to_small_int(self) -> sql::SmallInt {
        self as sql::SmallInt
    }

    /// Get all diagnostic identifiers applicable to header fields
    #[allow(dead_code)]
    pub fn header_fields() -> Vec<DiagIdentifier> {
        vec![
            DiagIdentifier::ReturnCode,
            DiagIdentifier::Number,
            DiagIdentifier::RowCount,
            DiagIdentifier::DynamicFunction,
            DiagIdentifier::DynamicFunctionCode,
            DiagIdentifier::CursorRowCount,
        ]
    }

    /// Get all diagnostic identifiers applicable to record fields
    #[allow(dead_code)]
    pub fn record_fields() -> Vec<DiagIdentifier> {
        vec![
            DiagIdentifier::SqlState,
            DiagIdentifier::Native,
            DiagIdentifier::MessageText,
            DiagIdentifier::ClassOrigin,
            DiagIdentifier::SubclassOrigin,
            DiagIdentifier::ConnectionName,
            DiagIdentifier::ServerName,
            DiagIdentifier::RowNumber,
            DiagIdentifier::ColumnNumber,
        ]
    }
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
    dynamic_function_code: Option<String>,
    number_of_records: Option<sql::Integer>,
    return_code: sql::RetCode,
    row_count: Option<sql::Len>,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub enum ClassOrigin {
    #[default]
    Odbc3_0,
    Iso9075,
}

#[derive(Debug, Clone, Default)]
pub struct DiagnosticRecord {
    pub class_origin: ClassOrigin,
    pub column_number: Option<sql::Integer>,
    pub row_number: Option<sql::Integer>,
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
}

pub trait WithDiagnosticInfo {
    fn get_diag_info(&self) -> &DiagnosticInfo;
    fn get_diag_info_mut(&mut self) -> &mut DiagnosticInfo;
}

impl WithDiagnosticInfo for Environment {
    fn get_diag_info(&self) -> &DiagnosticInfo {
        &self.diagnostic_info
    }
    fn get_diag_info_mut(&mut self) -> &mut DiagnosticInfo {
        &mut self.diagnostic_info
    }
}

impl WithDiagnosticInfo for Connection {
    fn get_diag_info(&self) -> &DiagnosticInfo {
        &self.diagnostic_info
    }
    fn get_diag_info_mut(&mut self) -> &mut DiagnosticInfo {
        &mut self.diagnostic_info
    }
}

impl WithDiagnosticInfo for Statement {
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
    let t: &mut dyn WithDiagnosticInfo = match handle_type {
        sql::HandleType::Env => env_from_handle(handle),
        sql::HandleType::Dbc => conn_from_handle(handle),
        sql::HandleType::Stmt => stmt_from_handle(handle),
        _ => return,
    };
    t.get_diag_info_mut().clear();
}

pub fn from_handle_type<'a>(
    handle_type: sql::HandleType,
    handle: sql::Handle,
) -> Option<&'a mut dyn WithDiagnosticInfo> {
    match handle_type {
        sql::HandleType::Env => Some(env_from_handle(handle)),
        sql::HandleType::Dbc => Some(conn_from_handle(handle)),
        sql::HandleType::Stmt => Some(stmt_from_handle(handle)),
        _ => {
            tracing::info!("Invalid handle type: {:?}", handle_type);
            None
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
    DiagnosticRecord {
        native_error: 0,
        sql_state,
        class_origin: ClassOrigin::Odbc3_0,
        column_number: None,
        row_number: None,
        connection_name: "".to_string(),
        message_text: message_text.to_string(),
    }
}

pub fn set_diag_info_from_warnings(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    warnings: &Warnings,
) {
    if handle.is_null() {
        return;
    }
    if let Some(t) = from_handle_type(handle_type, handle) {
        let diagnostic_info = t.get_diag_info_mut();
        for warning in warnings {
            diagnostic_info.add_record(from_warning(warning));
        }
    }
}

pub fn set_diag_info_from_result(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    result: &OdbcResult<()>,
) {
    if handle.is_null() {
        return;
    }
    if let Some(t) = from_handle_type(handle_type, handle) {
        let diagnostic_info = t.get_diag_info_mut();
        match result {
            Ok(_) => {}
            Err(error) => {
                diagnostic_info.add_record(error.to_diagnostic_record());
            }
        }
    }
}

pub fn get_diag_info(
    handle_type: sql::HandleType,
    handle: sql::Handle,
) -> OdbcResult<DiagnosticInfo> {
    let t: &dyn WithDiagnosticInfo = match handle_type {
        sql::HandleType::Env => env_from_handle(handle),
        sql::HandleType::Dbc => conn_from_handle(handle),
        sql::HandleType::Stmt => stmt_from_handle(handle),
        _ => return InvalidHandleSnafu.fail(),
    };
    Ok(t.get_diag_info().clone())
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
    let diagnostic_info = get_diag_info(handle_type, handle)?;
    if rec_number <= 0 {
        return InvalidRecordNumberSnafu { number: rec_number }.fail();
    }

    if rec_number > diagnostic_info.records.len() as i16 {
        return NoMoreDataSnafu.fail();
    }

    let record = diagnostic_info
        .records
        .get((rec_number - 1) as usize)
        .unwrap();

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
pub fn get_diag_field<E: OdbcEncoding>(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    rec_number: sql::SmallInt,
    diag_identifier: sql::SmallInt,
    diag_info_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
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
                unsafe {
                    std::ptr::write(
                        diag_info_ptr as *mut sql::Integer,
                        diagnostic_info.header.number_of_records.unwrap_or(0),
                    );
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
                if let Some(ref dynamic_function) = diagnostic_info.header.dynamic_function_code {
                    write_string_bytes::<E>(
                        dynamic_function,
                        diag_info_ptr as *mut E::Char,
                        buffer_length,
                        string_length_ptr,
                        None,
                    );
                    Ok(())
                } else {
                    NoMoreDataSnafu.fail()
                }
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
                    None,
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
                    None,
                );
                Ok(())
            }
            DiagIdentifier::ClassOrigin | DiagIdentifier::SubclassOrigin => {
                let origin_str = match record.class_origin {
                    ClassOrigin::Odbc3_0 => "ODBC 3.0",
                    ClassOrigin::Iso9075 => "ISO 9075",
                };
                write_string_bytes::<E>(
                    origin_str,
                    diag_info_ptr as *mut E::Char,
                    buffer_length,
                    string_length_ptr,
                    None,
                );
                Ok(())
            }
            DiagIdentifier::ConnectionName => {
                write_string_bytes::<E>(
                    &record.connection_name,
                    diag_info_ptr as *mut E::Char,
                    buffer_length,
                    string_length_ptr,
                    None,
                );
                Ok(())
            }
            DiagIdentifier::ServerName => {
                write_string_bytes::<E>(
                    "",
                    diag_info_ptr as *mut E::Char,
                    buffer_length,
                    string_length_ptr,
                    None,
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
                    std::ptr::write(
                        diag_info_ptr as *mut sql::Integer,
                        record.row_number.unwrap_or(0),
                    );
                }
                Ok(())
            }
            _ => NoMoreDataSnafu.fail(),
        }
    }
}
