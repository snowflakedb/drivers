//! ODBC C API functions
//!
//! This module provides the C API interface for ODBC functions.

#![allow(non_snake_case)]

use crate::api::CDataType;
use crate::api::encoding::WideChar;
use crate::api::{self, Narrow, ToSqlReturn, Wide};
use odbc_sys as sql;

/// Set the ODBC tracing dispatcher as the thread-local default for the
/// duration of the current C API call so that all `tracing::` output
/// (not just code inside [`OdbcGlobals::block_on`]) reaches the
/// configured log sink.
macro_rules! set_dispatch {
    () => {
        let _dispatch_guard = crate::api::runtime::dispatch_guard();
    };
}

/// Fire a fire-and-forget `api_call` telemetry event for an ODBC entry
/// point.
///
macro_rules! record_api {
    ($ht:expr, $h:expr, $name:literal) => {
        crate::api::telemetry::record_api_usage($ht, $h, $name);
    };
}

/// Fire a fire-and-forget `exception` telemetry event when an entry
/// point returned `Err`. Inserted right after the diagnostic record is
/// set.
macro_rules! record_err {
    ($ht:expr, $h:expr, $r:expr) => {
        if let Err(ref __err) = $r {
            crate::api::telemetry::record_wrapper_error($ht, $h, __err);
        }
    };
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLAllocEnv(output_handle: *mut sql::Handle) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Env, std::ptr::null_mut(), "SQLAllocEnv");
    let result = api::handle_allocation::sql_alloc_handle(
        sql::HandleType::Env,
        0 as sql::Handle,
        output_handle,
    );
    record_err!(sql::HandleType::Env, std::ptr::null_mut(), result);
    result.to_sql_code()
}
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLAllocConnect(
    environment_handle: sql::Handle,
    output_handle: *mut sql::Handle,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Env, environment_handle, "SQLAllocConnect");
    let result = api::handle_allocation::sql_alloc_handle(
        sql::HandleType::Dbc,
        environment_handle,
        output_handle,
    );
    record_err!(sql::HandleType::Env, environment_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLAllocHandle(
    handle_type: sql::HandleType,
    input_handle: sql::Handle,
    output_handle: *mut sql::Handle,
) -> sql::RetCode {
    set_dispatch!();
    // Use the *parent* handle for telemetry attribution: SQLAllocHandle(STMT, dbc)
    // is reportable against the connection that owns the soon-to-exist statement.
    record_api!(handle_type, input_handle, "SQLAllocHandle");
    let result = api::handle_allocation::sql_alloc_handle(handle_type, input_handle, output_handle);
    record_err!(handle_type, input_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLExecDirect(
    statement_handle: sql::Handle,
    statement_text: *const sql::Char,
    text_length: sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLExecDirect");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result =
        api::statement::exec_direct::<Narrow>(statement_handle, statement_text, text_length);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLExecDirectW(
    statement_handle: sql::Handle,
    statement_text: *const WideChar,
    text_length: sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLExecDirect");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::exec_direct::<Wide>(statement_handle, statement_text, text_length);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// ODBC catalog function: list tables matching pattern arguments.
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLTables(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    name_length1: sql::SmallInt,
    schema_name: *const sql::Char,
    name_length2: sql::SmallInt,
    table_name: *const sql::Char,
    name_length3: sql::SmallInt,
    table_type: *const sql::Char,
    name_length4: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLTables");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::tables::<Narrow>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        table_name,
        name_length3,
        table_type,
        name_length4,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLTablesW(
    statement_handle: sql::Handle,
    catalog_name: *const WideChar,
    name_length1: sql::SmallInt,
    schema_name: *const WideChar,
    name_length2: sql::SmallInt,
    table_name: *const WideChar,
    name_length3: sql::SmallInt,
    table_type: *const WideChar,
    name_length4: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLTables");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::tables::<Wide>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        table_name,
        name_length3,
        table_type,
        name_length4,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// ODBC catalog function: return information about supported SQL data types.
///
/// Both the ANSI and Unicode variants delegate to the same
/// `api::catalog::get_type_info` implementation; the DataType argument is an
/// integer and requires no string encoding.
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetTypeInfo(
    statement_handle: sql::Handle,
    data_type: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLGetTypeInfo");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::get_type_info(statement_handle, data_type);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLPrimaryKeys(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    name_length1: sql::SmallInt,
    schema_name: *const sql::Char,
    name_length2: sql::SmallInt,
    table_name: *const sql::Char,
    name_length3: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLPrimaryKeys");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::primary_keys::<Narrow>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        table_name,
        name_length3,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLPrimaryKeysW(
    statement_handle: sql::Handle,
    catalog_name: *const WideChar,
    name_length1: sql::SmallInt,
    schema_name: *const WideChar,
    name_length2: sql::SmallInt,
    table_name: *const WideChar,
    name_length3: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLPrimaryKeys");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::primary_keys::<Wide>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        table_name,
        name_length3,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// ODBC catalog function: return foreign-key metadata for PK/FK tables.
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLForeignKeys(
    statement_handle: sql::Handle,
    pk_catalog_name: *const sql::Char,
    name_length1: sql::SmallInt,
    pk_schema_name: *const sql::Char,
    name_length2: sql::SmallInt,
    pk_table_name: *const sql::Char,
    name_length3: sql::SmallInt,
    fk_catalog_name: *const sql::Char,
    name_length4: sql::SmallInt,
    fk_schema_name: *const sql::Char,
    name_length5: sql::SmallInt,
    fk_table_name: *const sql::Char,
    name_length6: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLForeignKeys");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::foreign_keys::<Narrow>(
        statement_handle,
        pk_catalog_name,
        name_length1,
        pk_schema_name,
        name_length2,
        pk_table_name,
        name_length3,
        fk_catalog_name,
        name_length4,
        fk_schema_name,
        name_length5,
        fk_table_name,
        name_length6,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLProcedures(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    name_length1: sql::SmallInt,
    schema_name: *const sql::Char,
    name_length2: sql::SmallInt,
    proc_name: *const sql::Char,
    name_length3: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLProcedures");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::procedures::<Narrow>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        proc_name,
        name_length3,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// ODBC catalog function: return foreign-key metadata for PK/FK tables.
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLForeignKeysW(
    statement_handle: sql::Handle,
    pk_catalog_name: *const WideChar,
    name_length1: sql::SmallInt,
    pk_schema_name: *const WideChar,
    name_length2: sql::SmallInt,
    pk_table_name: *const WideChar,
    name_length3: sql::SmallInt,
    fk_catalog_name: *const WideChar,
    name_length4: sql::SmallInt,
    fk_schema_name: *const WideChar,
    name_length5: sql::SmallInt,
    fk_table_name: *const WideChar,
    name_length6: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLForeignKeys");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::foreign_keys::<Wide>(
        statement_handle,
        pk_catalog_name,
        name_length1,
        pk_schema_name,
        name_length2,
        pk_table_name,
        name_length3,
        fk_catalog_name,
        name_length4,
        fk_schema_name,
        name_length5,
        fk_table_name,
        name_length6,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLProceduresW(
    statement_handle: sql::Handle,
    catalog_name: *const WideChar,
    name_length1: sql::SmallInt,
    schema_name: *const WideChar,
    name_length2: sql::SmallInt,
    proc_name: *const WideChar,
    name_length3: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLProcedures");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::procedures::<Wide>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        proc_name,
        name_length3,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// ODBC catalog function: list the input parameters, return value, and
/// result-set columns of stored procedures matching the pattern arguments.
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "system" fn SQLProcedureColumns(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    name_length1: sql::SmallInt,
    schema_name: *const sql::Char,
    name_length2: sql::SmallInt,
    proc_name: *const sql::Char,
    name_length3: sql::SmallInt,
    column_name: *const sql::Char,
    name_length4: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(
        sql::HandleType::Stmt,
        statement_handle,
        "SQLProcedureColumns"
    );
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::procedure_columns::<Narrow>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        proc_name,
        name_length3,
        column_name,
        name_length4,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "system" fn SQLProcedureColumnsW(
    statement_handle: sql::Handle,
    catalog_name: *const WideChar,
    name_length1: sql::SmallInt,
    schema_name: *const WideChar,
    name_length2: sql::SmallInt,
    proc_name: *const WideChar,
    name_length3: sql::SmallInt,
    column_name: *const WideChar,
    name_length4: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(
        sql::HandleType::Stmt,
        statement_handle,
        "SQLProcedureColumns"
    );
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::procedure_columns::<Wide>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        proc_name,
        name_length3,
        column_name,
        name_length4,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// ODBC catalog function: list columns matching pattern arguments.
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLColumns(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    name_length1: sql::SmallInt,
    schema_name: *const sql::Char,
    name_length2: sql::SmallInt,
    table_name: *const sql::Char,
    name_length3: sql::SmallInt,
    column_name: *const sql::Char,
    name_length4: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLColumns");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::columns::<Narrow>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        table_name,
        name_length3,
        column_name,
        name_length4,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetTypeInfoW(
    statement_handle: sql::Handle,
    data_type: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLGetTypeInfo");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::get_type_info(statement_handle, data_type);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// ODBC catalog function: return special columns (row identifiers / version columns).
/// Snowflake always returns an empty result set.
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSpecialColumns(
    statement_handle: sql::Handle,
    identifier_type: sql::SmallInt,
    catalog_name: *const sql::Char,
    name_length1: sql::SmallInt,
    schema_name: *const sql::Char,
    name_length2: sql::SmallInt,
    table_name: *const sql::Char,
    name_length3: sql::SmallInt,
    scope: sql::SmallInt,
    nullable: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLSpecialColumns");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::special_columns::<Narrow>(
        statement_handle,
        identifier_type,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        table_name,
        name_length3,
        scope,
        nullable,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSpecialColumnsW(
    statement_handle: sql::Handle,
    identifier_type: sql::SmallInt,
    catalog_name: *const WideChar,
    name_length1: sql::SmallInt,
    schema_name: *const WideChar,
    name_length2: sql::SmallInt,
    table_name: *const WideChar,
    name_length3: sql::SmallInt,
    scope: sql::SmallInt,
    nullable: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLSpecialColumns");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::special_columns::<Wide>(
        statement_handle,
        identifier_type,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        table_name,
        name_length3,
        scope,
        nullable,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// ODBC catalog function: return column-level privileges.
/// Snowflake always returns an empty result set.
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLColumnPrivileges(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    name_length1: sql::SmallInt,
    schema_name: *const sql::Char,
    name_length2: sql::SmallInt,
    table_name: *const sql::Char,
    name_length3: sql::SmallInt,
    column_name: *const sql::Char,
    name_length4: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(
        sql::HandleType::Stmt,
        statement_handle,
        "SQLColumnPrivileges"
    );
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::column_privileges::<Narrow>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        table_name,
        name_length3,
        column_name,
        name_length4,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLColumnPrivilegesW(
    statement_handle: sql::Handle,
    catalog_name: *const WideChar,
    name_length1: sql::SmallInt,
    schema_name: *const WideChar,
    name_length2: sql::SmallInt,
    table_name: *const WideChar,
    name_length3: sql::SmallInt,
    column_name: *const WideChar,
    name_length4: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(
        sql::HandleType::Stmt,
        statement_handle,
        "SQLColumnPrivileges"
    );
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::column_privileges::<Wide>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        table_name,
        name_length3,
        column_name,
        name_length4,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// ODBC catalog function: return table-level privileges.
/// Snowflake always returns an empty result set.
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLTablePrivileges(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    name_length1: sql::SmallInt,
    schema_name: *const sql::Char,
    name_length2: sql::SmallInt,
    table_name: *const sql::Char,
    name_length3: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(
        sql::HandleType::Stmt,
        statement_handle,
        "SQLTablePrivileges"
    );
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::table_privileges::<Narrow>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        table_name,
        name_length3,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLTablePrivilegesW(
    statement_handle: sql::Handle,
    catalog_name: *const WideChar,
    name_length1: sql::SmallInt,
    schema_name: *const WideChar,
    name_length2: sql::SmallInt,
    table_name: *const WideChar,
    name_length3: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(
        sql::HandleType::Stmt,
        statement_handle,
        "SQLTablePrivileges"
    );
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::table_privileges::<Wide>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        table_name,
        name_length3,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// ODBC catalog function: return index and statistics information.
/// Snowflake always returns an empty result set.
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLStatistics(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    name_length1: sql::SmallInt,
    schema_name: *const sql::Char,
    name_length2: sql::SmallInt,
    table_name: *const sql::Char,
    name_length3: sql::SmallInt,
    unique: sql::SmallInt,
    reserved: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLStatistics");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::statistics::<Narrow>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        table_name,
        name_length3,
        unique,
        reserved,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLStatisticsW(
    statement_handle: sql::Handle,
    catalog_name: *const WideChar,
    name_length1: sql::SmallInt,
    schema_name: *const WideChar,
    name_length2: sql::SmallInt,
    table_name: *const WideChar,
    name_length3: sql::SmallInt,
    unique: sql::SmallInt,
    reserved: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLStatistics");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::statistics::<Wide>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        table_name,
        name_length3,
        unique,
        reserved,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLColumnsW(
    statement_handle: sql::Handle,
    catalog_name: *const WideChar,
    name_length1: sql::SmallInt,
    schema_name: *const WideChar,
    name_length2: sql::SmallInt,
    table_name: *const WideChar,
    name_length3: sql::SmallInt,
    column_name: *const WideChar,
    name_length4: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLColumns");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::columns::<Wide>(
        statement_handle,
        catalog_name,
        name_length1,
        schema_name,
        name_length2,
        table_name,
        name_length3,
        column_name,
        name_length4,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLFreeHandle(
    handle_type: sql::HandleType,
    handle: sql::Handle,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(handle_type, handle, "SQLFreeHandle");
    api::handle_allocation::sql_free_handle(handle_type, handle).to_sql_code()
}
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLFreeStmt(
    statement_handle: sql::Handle,
    option: sql::USmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLFreeStmt");
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }

    // SQL_DROP (1) frees the statement handle — it is equivalent to
    // SQLFreeHandle(SQL_HANDLE_STMT), not one of free_stmt's live-statement options
    // (SQL_CLOSE / SQL_UNBIND / SQL_RESET_PARAMS). It is deprecated in ODBC 3.x: a 3.x
    // driver manager (e.g. unixODBC) remaps it to SQLFreeHandle so the driver never
    // sees it, but ODBC 2.x applications and iODBC pass it straight through to the
    // driver. Route it to the handle-lifecycle path and, like SQLFreeHandle above,
    // skip the clear/set diagnostic calls: the handle's diagnostic storage is gone
    // once it is freed, so touching it afterwards would operate on a stale handle.
    const SQL_DROP: sql::USmallInt = 1;
    if option == SQL_DROP {
        return api::handle_allocation::sql_free_handle(sql::HandleType::Stmt, statement_handle)
            .to_sql_code();
    }

    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::FreeStmtOption::try_from(option)
        .and_then(|opt| api::statement::free_stmt(statement_handle, opt));
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLCloseCursor(statement_handle: sql::Handle) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLCloseCursor");
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::close_cursor(statement_handle);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSetCursorName(
    statement_handle: sql::Handle,
    cursor_name: *const sql::Char,
    name_length: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLSetCursorName");
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::set_cursor_name::<Narrow>(
        statement_handle,
        cursor_name as sql::Pointer,
        name_length,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSetCursorNameW(
    statement_handle: sql::Handle,
    cursor_name: *const WideChar,
    name_length: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLSetCursorName");
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::set_cursor_name::<Wide>(
        statement_handle,
        cursor_name as sql::Pointer,
        name_length,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetCursorName(
    statement_handle: sql::Handle,
    cursor_name: *mut sql::Char,
    buffer_length: sql::SmallInt,
    name_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLGetCursorName");
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::statement::get_cursor_name::<Narrow>(
        statement_handle,
        cursor_name as sql::Pointer,
        buffer_length,
        name_length_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetCursorNameW(
    statement_handle: sql::Handle,
    cursor_name: *mut WideChar,
    buffer_length: sql::SmallInt,
    name_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLGetCursorName");
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::statement::get_cursor_name::<Wide>(
        statement_handle,
        cursor_name as sql::Pointer,
        buffer_length,
        name_length_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
///
/// ODBC allows SQLCancel to be called from a different thread.
/// Uses a two-path design via `Statement::cancel_token`:
/// - Path 1 (RPC in flight): cancels the token without touching the inner
///   Mutex. The executing thread observes cancellation via `tokio::select!`
///   and returns HY008.
/// - Path 2 (no RPC): locks the inner Mutex to check/restore NeedData state.
///   This path only runs in single-threaded DAE scenarios.
///
/// This function does not modify statement diagnostics. Any diagnostic
/// information related to cancellation must be produced by the executing
/// thread (e.g., returning HY008) or by code that can ensure exclusive
/// access to the statement.
///
/// NOTE: On Unix platforms, the Driver Manager (unixODBC/iODBC) updates its
/// internal state machine to close the cursor after SQLCancel, even though
/// this function is a no-op. Subsequent SQLFetch calls are rejected by the
/// DM with HY010 before reaching the driver. This is an ODBC 2.x behavior
/// that both Unix DMs implement regardless of SQL_ATTR_ODBC_VERSION.
///
/// KNOWN LIMITATION: Same-thread SQLCancel (for no-op / synchronous
/// cancel) also skips clearing stale diagnostics, which diverges from the
/// driver's pattern of clearing diagnostics on entry for every API call.
/// This means `SQLGetDiagRec` may return records from a previous call
/// after a successful `SQLCancel`. We accept this because we currently
/// cannot distinguish same-thread vs cross-thread callers.
///
/// TODO(SNOW-3258918): When async cancel is implemented, add thread-ID
/// tracking to distinguish same-thread vs cross-thread. Same-thread cancel
/// must clear_diag_info and post its own diagnostic records per spec. Only
/// cross-thread cancel skips diagnostics.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLCancel(statement_handle: sql::Handle) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLCancel");
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    let result = api::statement::cancel(statement_handle);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
///
/// `SQLCancelHandle` is the ODBC 3.8 generalization of `SQLCancel` that
/// accepts both statement and connection handles.
///
/// For **SQL_HANDLE_STMT** the behavior is identical to `SQLCancel`:
/// this calls `api::statement::cancel(handle)`, which follows the same
/// handle-to-statement path as `SQLCancel` and therefore has the same
/// aliasing caveat described there. Diagnostics are not touched
/// (same reasoning as `SQLCancel`).
///
/// For **SQL_HANDLE_DBC** this is currently a no-op returning SUCCESS.
/// Connection-level cancel (async connect, cross-thread
/// `SQLDriverConnect`) will be implemented after the connection state
/// machine is hardened (SNOW-3307201).
///
/// **SQL_HANDLE_ENV** and **SQL_HANDLE_DESC** return `SQL_ERROR` with
/// SQLSTATE HY092 per the ODBC 3.8 spec. Any truly unknown handle
/// type returns `SQL_INVALID_HANDLE`.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLCancelHandle(
    handle_type: sql::HandleType,
    handle: sql::Handle,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(handle_type, handle, "SQLCancelHandle");
    if handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    match handle_type {
        sql::HandleType::Stmt => {
            let result = api::statement::cancel(handle);
            record_err!(sql::HandleType::Stmt, handle, result);
            result.to_sql_code()
        }
        // TODO(SNOW-3307201): implement connection-level cancel after
        // the connection state machine is hardened.
        sql::HandleType::Dbc => {
            api::diagnostic::clear_diag_info(handle_type, handle);
            sql::SqlReturn::SUCCESS.0
        }
        sql::HandleType::Env | sql::HandleType::Desc => {
            api::diagnostic::clear_diag_info(handle_type, handle);
            let result: api::OdbcResult<()> = api::error::InvalidHandleTypeSnafu {
                handle_type: handle_type as i16,
            }
            .fail();
            api::diagnostic::set_diag_info_from_result(handle_type, handle, &result);
            record_err!(handle_type, handle, result);
            result.to_sql_code()
        }
        _ => sql::SqlReturn::INVALID_HANDLE.0,
    }
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLConnect(
    connection_handle: sql::Handle,
    server_name: *const sql::Char,
    name_length1: sql::SmallInt,
    user_name: *const sql::Char,
    name_length2: sql::SmallInt,
    authentication: *const sql::Char,
    name_length3: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::connect::<Narrow>(
        connection_handle,
        server_name,
        name_length1,
        user_name,
        name_length2,
        authentication,
        name_length3,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    // Record AFTER the call: there is no telemetry-eligible session until
    // connection_init has succeeded; the resolver returns None for a still-
    // Disconnected Dbc, so the failure-path event is silently dropped.
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLConnect");
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLConnectW(
    connection_handle: sql::Handle,
    server_name: *const WideChar,
    name_length1: sql::SmallInt,
    user_name: *const WideChar,
    name_length2: sql::SmallInt,
    authentication: *const WideChar,
    name_length3: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::connect::<Wide>(
        connection_handle,
        server_name,
        name_length1,
        user_name,
        name_length2,
        authentication,
        name_length3,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLConnect");
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSetEnvAttr(
    environment_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    string_length: sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Env, environment_handle, "SQLSetEnvAttr");
    if environment_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Env, environment_handle);
    let result =
        api::environment::set_env_attribute(environment_handle, attribute, value, string_length);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Env, environment_handle, &result);
    record_err!(sql::HandleType::Env, environment_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetEnvAttr(
    environment_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Env, environment_handle, "SQLGetEnvAttr");
    if environment_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Env, environment_handle);
    let result = api::environment::get_env_attribute(
        environment_handle,
        attribute,
        value,
        buffer_length,
        string_length_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Env, environment_handle, &result);
    record_err!(sql::HandleType::Env, environment_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetInfo(
    connection_handle: sql::Handle,
    info_type: sql::USmallInt,
    info_value_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLGetInfo");
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::get_info::<Narrow>(
        connection_handle,
        info_type,
        info_value_ptr,
        buffer_length,
        string_length_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetInfoW(
    connection_handle: sql::Handle,
    info_type: sql::USmallInt,
    info_value_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLGetInfo");
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::get_info::<Wide>(
        connection_handle,
        info_type,
        info_value_ptr,
        buffer_length,
        string_length_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetFunctions(
    connection_handle: sql::Handle,
    function_id: sql::USmallInt,
    supported_ptr: *mut sql::USmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLGetFunctions");
    if connection_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::get_functions(connection_handle, function_id, supported_ptr);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSetConnectAttr(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    string_length: sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLSetConnectAttr");
    if connection_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::set_connect_attr::<Narrow>(
        connection_handle,
        attribute,
        value,
        string_length,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSetConnectAttrW(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    string_length: sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLSetConnectAttr");
    if connection_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::set_connect_attr::<Wide>(
        connection_handle,
        attribute,
        value,
        string_length,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetConnectAttr(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLGetConnectAttr");
    if connection_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::get_connect_attr::<Narrow>(
        connection_handle,
        attribute,
        value,
        buffer_length,
        string_length_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetConnectAttrW(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLGetConnectAttr");
    if connection_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::get_connect_attr::<Wide>(
        connection_handle,
        attribute,
        value,
        buffer_length,
        string_length_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// Legacy ODBC 2.x entry point exported ONLY for iODBC on UNIX compatibility.
/// `SQLSetConnectAttr` with `SQL_NTS` so the encoding layer can run its
/// null-terminator scan.
///
/// Exporting this matters specifically for iODBC: when a caller invokes
/// `SQLSetConnectAttr` on an *unconnected* handle iODBC queues the call and
/// replays it after `SQLDriverConnect` succeeds. The replay code path
/// prefers `SQLSetConnectOption*` over `SQLSetConnectAttr*` when both are
/// available and - crucially - never drops the string length, because the
/// 2.x API doesn't take one in the first place. Without this shim iODBC
/// replays driver-defined string attributes (e.g.
/// `SQL_SF_CONN_ATTR_PRIV_KEY_BASE64`) via `SQLSetConnectAttrW` with
/// `strLength = 0`, the driver reads an empty payload, and JWT connects
/// fail with "Missing required parameter: private_key".
///
/// For numeric attributes `value` is the immediate `SQLULEN`; the
/// downstream dispatch in `set_connect_attr` keys on `attribute` and
/// ignores the length, so passing `SQL_NTS` here is harmless.
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSetConnectOption(
    connection_handle: sql::Handle,
    option: sql::USmallInt,
    value: sql::ULen,
) -> sql::RetCode {
    unsafe {
        SQLSetConnectAttr(
            connection_handle,
            sql::Integer::from(option),
            value as sql::Pointer,
            sql::NTS as sql::Integer,
        )
    }
}

/// Legacy ODBC 2.x entry point exported ONLY for iODBC on UNIX compatibility.
/// Wide-string counterpart of [`SQLSetConnectOption`]. See that function
/// for the iODBC attribute-replay rationale.
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSetConnectOptionW(
    connection_handle: sql::Handle,
    option: sql::USmallInt,
    value: sql::ULen,
) -> sql::RetCode {
    unsafe {
        SQLSetConnectAttrW(
            connection_handle,
            sql::Integer::from(option),
            value as sql::Pointer,
            sql::NTS as sql::Integer,
        )
    }
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLDriverConnect(
    connection_handle: sql::Handle,
    _window_handle: sql::Handle,
    in_connection_string: *const sql::Char,
    in_string_length: sql::SmallInt,
    out_connection_string: *mut sql::Char,
    buffer_length: sql::SmallInt,
    out_string_length: *mut sql::SmallInt,
    _driver_completion: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::driver_connect::<Narrow>(
        connection_handle,
        in_connection_string,
        in_string_length,
        out_connection_string,
        buffer_length,
        out_string_length,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    // Record AFTER the call: same rationale as SQLConnect.
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLDriverConnect");
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLDriverConnectW(
    connection_handle: sql::Handle,
    _window_handle: sql::Handle,
    in_connection_string: *const WideChar,
    in_string_length: sql::SmallInt,
    out_connection_string: *mut WideChar,
    buffer_length: sql::SmallInt,
    out_string_length: *mut sql::SmallInt,
    _driver_completion: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::driver_connect::<Wide>(
        connection_handle,
        in_connection_string,
        in_string_length,
        out_connection_string,
        buffer_length,
        out_string_length,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLDriverConnect");
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// Map a `browse_connect` outcome to the SQLBrowseConnect return code.
///
/// `SQLBrowseConnect` uses `SQL_NEED_DATA` where `SQLDriverConnect` would report
/// `01004` truncation, so the `NeedData` outcome cannot flow through the shared
/// `to_sql_code` pipeline (which only yields SUCCESS/ERROR/…). Errors map the
/// same way as any other connect error.
fn browse_connect_ret_code(
    result: api::OdbcResult<api::connection::BrowseOutcome>,
) -> sql::RetCode {
    match result {
        Ok(api::connection::BrowseOutcome::Complete) => sql::SqlReturn::SUCCESS.0,
        Ok(api::connection::BrowseOutcome::NeedData) => sql::SqlReturn::NEED_DATA.0,
        Err(err) => Err::<(), _>(err).to_sql_code(),
    }
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLBrowseConnect(
    connection_handle: sql::Handle,
    in_connection_string: *const sql::Char,
    in_string_length: sql::SmallInt,
    out_connection_string: *mut sql::Char,
    buffer_length: sql::SmallInt,
    out_string_length: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::browse_connect::<Narrow>(
        connection_handle,
        in_connection_string,
        in_string_length,
        out_connection_string,
        buffer_length,
        out_string_length,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLBrowseConnect");
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    browse_connect_ret_code(result)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLBrowseConnectW(
    connection_handle: sql::Handle,
    in_connection_string: *const WideChar,
    in_string_length: sql::SmallInt,
    out_connection_string: *mut WideChar,
    buffer_length: sql::SmallInt,
    out_string_length: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::browse_connect::<Wide>(
        connection_handle,
        in_connection_string,
        in_string_length,
        out_connection_string,
        buffer_length,
        out_string_length,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLBrowseConnect");
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    browse_connect_ret_code(result)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLDisconnect(connection_handle: sql::Handle) -> sql::RetCode {
    set_dispatch!();
    // Record BEFORE disconnect tears down the session so the resolver still
    // finds the connection in `Connected` state.
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLDisconnect");
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::disconnect(connection_handle);
    // Post diagnostics so SQLGetDiagRec surfaces SQLSTATE 25000 when disconnect
    // is refused because a transaction is still in process.
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    result.to_sql_code()
}

/// Shared dispatch for `SQLEndTran` and `SQLTransact`.
unsafe fn end_tran_dispatch(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    completion_type: sql::SmallInt,
) -> sql::RetCode {
    match handle_type {
        sql::HandleType::Dbc => {
            api::diagnostic::clear_diag_info(handle_type, handle);
            let result = api::connection::end_tran(handle, completion_type);
            api::diagnostic::set_diag_info_from_result(handle_type, handle, &result);
            record_err!(handle_type, handle, result);
            result.to_sql_code()
        }
        sql::HandleType::Env => {
            api::diagnostic::clear_diag_info(handle_type, handle);
            let result = api::connection::end_tran_env(handle, completion_type);
            api::diagnostic::set_diag_info_from_result(handle_type, handle, &result);
            record_err!(handle_type, handle, result);
            result.to_sql_code()
        }
        // SQL_HANDLE_STMT / SQL_HANDLE_DESC are not valid transaction handles.
        sql::HandleType::Stmt | sql::HandleType::Desc => {
            api::diagnostic::clear_diag_info(handle_type, handle);
            let result: api::OdbcResult<()> = api::error::InvalidHandleTypeSnafu {
                handle_type: handle_type as i16,
            }
            .fail();
            api::diagnostic::set_diag_info_from_result(handle_type, handle, &result);
            record_err!(handle_type, handle, result);
            result.to_sql_code()
        }
        _ => sql::SqlReturn::INVALID_HANDLE.0,
    }
}

/// # Safety
/// This function is called by the ODBC driver manager.
///
/// `SQLEndTran` commits or rolls back the open transaction. It accepts
/// **SQL_HANDLE_DBC** (the connection) and **SQL_HANDLE_ENV** (every
/// connection on the environment). **SQL_HANDLE_STMT** / **SQL_HANDLE_DESC**
/// are rejected with SQLSTATE HY092; a null handle returns
/// `SQL_INVALID_HANDLE`; an invalid completion type yields HY012 (posted by
/// `end_tran`/`end_tran_env`).
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLEndTran(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    completion_type: sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(handle_type, handle, "SQLEndTran");
    if handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    end_tran_dispatch(handle_type, handle, completion_type)
}

/// # Safety
/// This function is called by the ODBC driver manager.
///
/// ODBC 2.x transaction terminator. When `ConnectionHandle` is not
/// `SQL_NULL_HDBC`, behaves as `SQLEndTran(SQL_HANDLE_DBC, ConnectionHandle,
/// CompletionType)`; otherwise behaves as `SQLEndTran(SQL_HANDLE_ENV,
/// EnvironmentHandle, CompletionType)`. Returns `SQL_INVALID_HANDLE` when both
/// handles are null.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLTransact(
    environment_handle: sql::Handle,
    connection_handle: sql::Handle,
    completion_type: sql::USmallInt,
) -> sql::RetCode {
    set_dispatch!();
    let (handle_type, handle) = if !connection_handle.is_null() {
        (sql::HandleType::Dbc, connection_handle)
    } else if !environment_handle.is_null() {
        (sql::HandleType::Env, environment_handle)
    } else {
        return sql::SqlReturn::INVALID_HANDLE.0;
    };
    record_api!(handle_type, handle, "SQLTransact");
    end_tran_dispatch(handle_type, handle, completion_type as sql::SmallInt)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLFetch(statement_handle: sql::Handle) -> sql::RetCode {
    set_dispatch!();
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::data::fetch(statement_handle, &mut warnings);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLFetchScroll(
    statement_handle: sql::Handle,
    fetch_orientation: sql::SmallInt,
    _fetch_offset: sql::Len,
) -> sql::RetCode {
    set_dispatch!();
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::data::fetch_scroll(statement_handle, fetch_orientation, &mut warnings);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLExtendedFetch(
    statement_handle: sql::Handle,
    fetch_orientation: sql::SmallInt,
    fetch_offset: sql::Len,
    row_count_ptr: *mut sql::ULen,
    row_status_ptr: *mut sql::USmallInt,
) -> sql::RetCode {
    set_dispatch!();
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::data::extended_fetch(
        statement_handle,
        fetch_orientation,
        fetch_offset,
        row_count_ptr,
        row_status_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetData(
    statement_handle: sql::Handle,
    col_or_param_num: sql::USmallInt,
    target_type: CDataType,
    target_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
) -> sql::RetCode {
    set_dispatch!();
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::data::get_data(
        statement_handle,
        col_or_param_num,
        target_type,
        target_value_ptr,
        buffer_length,
        str_len_or_ind_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLColAttribute(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    field_identifier: sql::USmallInt,
    character_attribute_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    numeric_attribute_ptr: *mut sql::Len,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLColAttribute");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::utils::col_attribute::<Narrow>(
        statement_handle,
        column_number,
        field_identifier,
        character_attribute_ptr as *mut sql::Char,
        buffer_length,
        string_length_ptr,
        numeric_attribute_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLColAttributeW(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    field_identifier: sql::USmallInt,
    character_attribute_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    numeric_attribute_ptr: *mut sql::Len,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLColAttributeW");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::utils::col_attribute::<Wide>(
        statement_handle,
        column_number,
        field_identifier,
        character_attribute_ptr as *mut WideChar,
        buffer_length,
        string_length_ptr,
        numeric_attribute_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLColAttributes(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    field_identifier: sql::USmallInt,
    character_attribute_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    numeric_attribute_ptr: *mut sql::Len,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLColAttributes");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::utils::col_attribute::<Narrow>(
        statement_handle,
        column_number,
        field_identifier,
        character_attribute_ptr as *mut sql::Char,
        buffer_length,
        string_length_ptr,
        numeric_attribute_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLColAttributesW(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    field_identifier: sql::USmallInt,
    character_attribute_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    numeric_attribute_ptr: *mut sql::Len,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLColAttributesW");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::utils::col_attribute::<Wide>(
        statement_handle,
        column_number,
        field_identifier,
        character_attribute_ptr as *mut WideChar,
        buffer_length,
        string_length_ptr,
        numeric_attribute_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLDescribeCol(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    column_name: *mut sql::Char,
    buffer_length: sql::SmallInt,
    name_length_ptr: *mut sql::SmallInt,
    data_type_ptr: *mut sql::SmallInt,
    column_size_ptr: *mut sql::ULen,
    decimal_digits_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLDescribeCol");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::utils::describe_col::<Narrow>(
        statement_handle,
        column_number,
        column_name,
        buffer_length,
        name_length_ptr,
        data_type_ptr,
        column_size_ptr,
        decimal_digits_ptr,
        nullable_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLDescribeColW(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    column_name: *mut WideChar,
    buffer_length: sql::SmallInt,
    name_length_ptr: *mut sql::SmallInt,
    data_type_ptr: *mut sql::SmallInt,
    column_size_ptr: *mut sql::ULen,
    decimal_digits_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLDescribeCol");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::utils::describe_col::<Wide>(
        statement_handle,
        column_number,
        column_name,
        buffer_length,
        name_length_ptr,
        data_type_ptr,
        column_size_ptr,
        decimal_digits_ptr,
        nullable_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLNumResultCols(
    statement_handle: sql::Handle,
    column_count_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLNumResultCols");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::utils::num_result_cols(statement_handle, column_count_ptr);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLNumParams(
    statement_handle: sql::Handle,
    param_count_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLNumParams");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::num_params(statement_handle, param_count_ptr);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLDescribeParam(
    statement_handle: sql::Handle,
    parameter_number: sql::USmallInt,
    data_type_ptr: *mut sql::SmallInt,
    parameter_size_ptr: *mut sql::ULen,
    decimal_digits_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLDescribeParam");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::describe_param(
        statement_handle,
        parameter_number,
        data_type_ptr,
        parameter_size_ptr,
        decimal_digits_ptr,
        nullable_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLRowCount(
    statement_handle: sql::Handle,
    row_count_ptr: *mut sql::Len,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLRowCount");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::utils::row_count(statement_handle, row_count_ptr);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLBindParameter(
    statement_handle: sql::Handle,
    parameter_number: sql::USmallInt,
    input_output_type: sql::SmallInt,
    value_type: sql::SmallInt,
    parameter_type: sql::SmallInt,
    column_size: sql::ULen,
    decimal_digits: sql::SmallInt,
    parameter_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLBindParameter");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::bind_parameter(
        statement_handle,
        parameter_number,
        input_output_type,
        value_type,
        parameter_type,
        column_size,
        decimal_digits,
        parameter_value_ptr,
        buffer_length,
        str_len_or_ind_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLPrepare(
    statement_handle: sql::Handle,
    statement_text: *const sql::Char,
    text_length: sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLPrepare");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::prepare::<Narrow>(statement_handle, statement_text, text_length);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLPrepareW(
    statement_handle: sql::Handle,
    statement_text: *const WideChar,
    text_length: sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLPrepare");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::prepare::<Wide>(statement_handle, statement_text, text_length);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLParamData(
    statement_handle: sql::Handle,
    value_ptr_ptr: *mut sql::Pointer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLParamData");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::param_data(statement_handle, value_ptr_ptr);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLPutData(
    statement_handle: sql::Handle,
    data_ptr: sql::Pointer,
    str_len_or_ind: sql::Len,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLPutData");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::put_data(statement_handle, data_ptr, str_len_or_ind);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLExecute(statement_handle: sql::Handle) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLExecute");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::execute(statement_handle);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetDiagRec(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    rec_number: sql::SmallInt,
    sql_state: *mut sql::Char,
    native_error_ptr: *mut sql::Integer,
    message_text: *mut sql::Char,
    buffer_length: sql::SmallInt,
    text_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    let mut warnings = vec![];
    let result = unsafe {
        api::diagnostic::get_diag_rec::<Narrow>(
            handle_type,
            handle,
            rec_number,
            sql_state,
            native_error_ptr,
            message_text,
            buffer_length,
            text_length_ptr,
            &mut warnings,
        )
    };
    record_err!(handle_type, handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetDiagRecW(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    rec_number: sql::SmallInt,
    sql_state: *mut WideChar,
    native_error_ptr: *mut sql::Integer,
    message_text: *mut WideChar,
    buffer_length: sql::SmallInt,
    text_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    let mut warnings = vec![];
    let result = unsafe {
        api::diagnostic::get_diag_rec::<Wide>(
            handle_type,
            handle,
            rec_number,
            sql_state,
            native_error_ptr,
            message_text,
            buffer_length,
            text_length_ptr,
            &mut warnings,
        )
    };
    record_err!(handle_type, handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetDiagField(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    rec_number: sql::SmallInt,
    diag_identifier: sql::SmallInt,
    diag_info_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    let result = api::diagnostic::get_diag_field::<Narrow>(
        handle_type,
        handle,
        rec_number,
        diag_identifier,
        diag_info_ptr,
        buffer_length,
        string_length_ptr,
    );
    record_err!(handle_type, handle, result);
    result.to_sql_code()
}
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetDiagFieldW(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    rec_number: sql::SmallInt,
    diag_identifier: sql::SmallInt,
    diag_info_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    let result = api::diagnostic::get_diag_field::<Wide>(
        handle_type,
        handle,
        rec_number,
        diag_identifier,
        diag_info_ptr,
        buffer_length,
        string_length_ptr,
    );
    record_err!(handle_type, handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLBindCol(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    target_type: CDataType,
    target_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLBindCol");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::bind_col(
        statement_handle,
        column_number,
        target_type,
        target_value_ptr,
        buffer_length,
        str_len_or_ind_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSetStmtAttr(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    string_length: sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLSetStmtAttr");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::statement::set_stmt_attr(
        statement_handle,
        attribute,
        value_ptr,
        string_length,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSetStmtAttrW(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    string_length: sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLSetStmtAttr");
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::statement::set_stmt_attr(
        statement_handle,
        attribute,
        value_ptr,
        string_length,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// Legacy ODBC 2.x entry point. Kept as a shim that delegates to
/// `SQLSetStmtAttr` with `SQL_NTS`.
///
/// Companion to [`SQLSetConnectOption`]: exporting it advertises full
/// ODBC 2.x compatibility to iODBC, whose driver-loader probes for
/// `SQLSetStmtOption` alongside `SQLSetStmtAttr`. The driver's
/// `set_stmt_attr` dispatch ignores `string_length` for numeric attributes
/// (which is what every standard `SQL_ATTR_*` stmt attribute is), so the
/// `SQL_NTS` passed below is a no-op there.
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSetStmtOption(
    statement_handle: sql::Handle,
    option: sql::USmallInt,
    value: sql::ULen,
) -> sql::RetCode {
    unsafe {
        SQLSetStmtAttr(
            statement_handle,
            sql::Integer::from(option),
            value as sql::Pointer,
            sql::NTS as sql::Integer,
        )
    }
}

/// Wide-string counterpart of [`SQLSetStmtOption`].
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSetStmtOptionW(
    statement_handle: sql::Handle,
    option: sql::USmallInt,
    value: sql::ULen,
) -> sql::RetCode {
    unsafe {
        SQLSetStmtAttrW(
            statement_handle,
            sql::Integer::from(option),
            value as sql::Pointer,
            sql::NTS as sql::Integer,
        )
    }
}

/// ODBC 2.x deprecated entry point that maps four scroll-related parameters to
/// the equivalent `SQL_ATTR_*` statement attribute writes. Advertising this
/// function ensures `SQLGetFunctions(SQL_API_SQLSETSCROLLOPTIONS)` returns
/// `SQL_TRUE`, which is required by the comprehensive `SQLGetFunctions` test
/// suite.
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSetScrollOptions(
    statement_handle: sql::Handle,
    f_concurrency: sql::USmallInt,
    crow_keyset: sql::Len,
    crow_rowset: sql::USmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(
        sql::HandleType::Stmt,
        statement_handle,
        "SQLSetScrollOptions"
    );
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::statement::set_scroll_options(
        statement_handle,
        f_concurrency,
        crow_keyset,
        crow_rowset,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// ODBC 2.x deprecated entry point, superseded by `SQLSetStmtAttr` in ODBC
/// 3.x. Sets `SQL_ATTR_PARAMSET_SIZE` (`crow`) and
/// `SQL_ATTR_PARAMS_PROCESSED_PTR` (`pi_row`) as a single convenience call.
/// Many ODBC 2.x applications still invoke it directly, so the driver
/// exports it for compatibility.
///
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLParamOptions(
    statement_handle: sql::Handle,
    crow: sql::ULen,
    pi_row: *mut sql::ULen,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLParamOptions");
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    let ret = std::panic::catch_unwind(|| {
        api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
        let mut warnings = vec![];
        let result =
            api::statement::set_param_options(statement_handle, crow, pi_row, &mut warnings);
        api::diagnostic::set_diag_info_from_result(
            sql::HandleType::Stmt,
            statement_handle,
            &result,
        );
        api::diagnostic::set_diag_info_from_warnings(
            sql::HandleType::Stmt,
            statement_handle,
            &warnings,
        );
        record_err!(sql::HandleType::Stmt, statement_handle, result);
        result.to_sql_code_with_warnings(&warnings)
    });
    match ret {
        Ok(r) => r,
        Err(_) => sql::SqlReturn::ERROR.0,
    }
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetStmtAttr(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLGetStmtAttr");
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::statement::get_stmt_attr::<Narrow>(
        statement_handle,
        attribute,
        value_ptr,
        buffer_length,
        string_length_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetStmtAttrW(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLGetStmtAttr");
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::statement::get_stmt_attr::<Wide>(
        statement_handle,
        attribute,
        value_ptr,
        buffer_length,
        string_length_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLMoreResults(statement_handle: sql::Handle) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Stmt, statement_handle, "SQLMoreResults");
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::more_results(statement_handle);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    record_err!(sql::HandleType::Stmt, statement_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLNativeSql(
    connection_handle: sql::Handle,
    in_statement_text: *const sql::Char,
    text_length1: sql::Integer,
    out_statement_text: *mut sql::Char,
    buffer_length: sql::Integer,
    text_length2_ptr: *mut sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLNativeSql");
    if connection_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::native_sql::<Narrow>(
        connection_handle,
        in_statement_text,
        text_length1,
        out_statement_text,
        buffer_length,
        text_length2_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLNativeSqlW(
    connection_handle: sql::Handle,
    in_statement_text: *const WideChar,
    text_length1: sql::Integer,
    out_statement_text: *mut WideChar,
    buffer_length: sql::Integer,
    text_length2_ptr: *mut sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Dbc, connection_handle, "SQLNativeSql");
    if connection_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::native_sql::<Wide>(
        connection_handle,
        in_statement_text,
        text_length1,
        out_statement_text,
        buffer_length,
        text_length2_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    record_err!(sql::HandleType::Dbc, connection_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetDescField(
    descriptor_handle: sql::Handle,
    rec_number: sql::SmallInt,
    field_identifier: sql::SmallInt,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Desc, descriptor_handle, "SQLGetDescField");
    api::diagnostic::clear_diag_info(sql::HandleType::Desc, descriptor_handle);
    let mut warnings = vec![];
    let result = api::descriptor::get_desc_field::<api::encoding::Narrow>(
        descriptor_handle,
        rec_number,
        field_identifier,
        value_ptr,
        buffer_length,
        string_length_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Desc, descriptor_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Desc,
        descriptor_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Desc, descriptor_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetDescFieldW(
    descriptor_handle: sql::Handle,
    rec_number: sql::SmallInt,
    field_identifier: sql::SmallInt,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Desc, descriptor_handle, "SQLGetDescFieldW");
    api::diagnostic::clear_diag_info(sql::HandleType::Desc, descriptor_handle);
    let mut warnings = vec![];
    let result = api::descriptor::get_desc_field::<api::encoding::Wide>(
        descriptor_handle,
        rec_number,
        field_identifier,
        value_ptr,
        buffer_length,
        string_length_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Desc, descriptor_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Desc,
        descriptor_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Desc, descriptor_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetDescRec(
    descriptor_handle: sql::Handle,
    rec_number: sql::SmallInt,
    name: *mut sql::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    type_ptr: *mut sql::SmallInt,
    sub_type_ptr: *mut sql::SmallInt,
    length_ptr: *mut sql::Len,
    precision_ptr: *mut sql::SmallInt,
    scale_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Desc, descriptor_handle, "SQLGetDescRec");
    api::diagnostic::clear_diag_info(sql::HandleType::Desc, descriptor_handle);
    let mut warnings = vec![];
    let result = api::descriptor::get_desc_rec::<api::encoding::Narrow>(
        descriptor_handle,
        rec_number,
        name,
        buffer_length,
        string_length_ptr,
        type_ptr,
        sub_type_ptr,
        length_ptr,
        precision_ptr,
        scale_ptr,
        nullable_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Desc, descriptor_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Desc,
        descriptor_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Desc, descriptor_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLGetDescRecW(
    descriptor_handle: sql::Handle,
    rec_number: sql::SmallInt,
    name: *mut sql::WChar,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    type_ptr: *mut sql::SmallInt,
    sub_type_ptr: *mut sql::SmallInt,
    length_ptr: *mut sql::Len,
    precision_ptr: *mut sql::SmallInt,
    scale_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Desc, descriptor_handle, "SQLGetDescRecW");
    api::diagnostic::clear_diag_info(sql::HandleType::Desc, descriptor_handle);
    let mut warnings = vec![];
    let result = api::descriptor::get_desc_rec::<api::encoding::Wide>(
        descriptor_handle,
        rec_number,
        name,
        buffer_length,
        string_length_ptr,
        type_ptr,
        sub_type_ptr,
        length_ptr,
        precision_ptr,
        scale_ptr,
        nullable_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Desc, descriptor_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Desc,
        descriptor_handle,
        &warnings,
    );
    record_err!(sql::HandleType::Desc, descriptor_handle, result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSetDescRec(
    descriptor_handle: sql::Handle,
    rec_number: sql::SmallInt,
    type_: sql::SmallInt,
    sub_type: sql::SmallInt,
    length: sql::Len,
    precision: sql::SmallInt,
    scale: sql::SmallInt,
    data_ptr: sql::Pointer,
    string_length_ptr: *mut sql::Len,
    indicator_ptr: *mut sql::Len,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Desc, descriptor_handle, "SQLSetDescRec");
    api::diagnostic::clear_diag_info(sql::HandleType::Desc, descriptor_handle);
    let result = api::descriptor::set_desc_rec(
        descriptor_handle,
        rec_number,
        type_,
        sub_type,
        length,
        precision,
        scale,
        data_ptr,
        string_length_ptr,
        indicator_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Desc, descriptor_handle, &result);
    record_err!(sql::HandleType::Desc, descriptor_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLCopyDesc(
    source_desc_handle: sql::Handle,
    target_desc_handle: sql::Handle,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Desc, source_desc_handle, "SQLCopyDesc");
    api::diagnostic::clear_diag_info(sql::HandleType::Desc, target_desc_handle);
    let result = api::descriptor::copy_desc(source_desc_handle, target_desc_handle);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Desc, target_desc_handle, &result);
    record_err!(sql::HandleType::Desc, target_desc_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSetDescField(
    descriptor_handle: sql::Handle,
    rec_number: sql::SmallInt,
    field_identifier: sql::SmallInt,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Desc, descriptor_handle, "SQLSetDescField");
    api::diagnostic::clear_diag_info(sql::HandleType::Desc, descriptor_handle);
    let result = api::descriptor::set_desc_field::<Narrow>(
        descriptor_handle,
        rec_number,
        field_identifier,
        value_ptr,
        buffer_length,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Desc, descriptor_handle, &result);
    record_err!(sql::HandleType::Desc, descriptor_handle, result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SQLSetDescFieldW(
    descriptor_handle: sql::Handle,
    rec_number: sql::SmallInt,
    field_identifier: sql::SmallInt,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
) -> sql::RetCode {
    set_dispatch!();
    record_api!(sql::HandleType::Desc, descriptor_handle, "SQLSetDescFieldW");
    api::diagnostic::clear_diag_info(sql::HandleType::Desc, descriptor_handle);
    let result = api::descriptor::set_desc_field::<Wide>(
        descriptor_handle,
        rec_number,
        field_identifier,
        value_ptr,
        buffer_length,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Desc, descriptor_handle, &result);
    record_err!(sql::HandleType::Desc, descriptor_handle, result);
    result.to_sql_code()
}

// ============================================================================
// DllMain — capture module handle for dialog resources
// ============================================================================

#[cfg(target_os = "windows")]
pub(crate) static DLL_HINSTANCE: std::sync::atomic::AtomicPtr<core::ffi::c_void> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    h_instance: *mut core::ffi::c_void,
    reason: u32,
    _reserved: *mut core::ffi::c_void,
) -> i32 {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason == DLL_PROCESS_ATTACH {
        DLL_HINSTANCE.store(h_instance, std::sync::atomic::Ordering::Relaxed);
    }
    1 // TRUE
}

// ============================================================================
// Setup DLL API — ConfigDriver / ConfigDSN
//
// These functions are called by the ODBC Installer DLL, not the
// Driver Manager. They allow the ODBC Administrator UI to add, modify, and
// remove DSNs for this driver.
// ============================================================================

#[cfg(target_os = "windows")]
mod setup {
    use std::ptr;

    use crate::setup_common::{self, SQLRemoveDSNFromIniW, to_wide};

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MultiByteToWideChar(
            code_page: u32,
            dw_flags: u32,
            lp_multi_byte_str: *const u8,
            cb_multi_byte: i32,
            lp_wide_char_str: *mut u16,
            cch_wide_char: i32,
        ) -> i32;
    }

    const CP_ACP: u32 = 0;
    const ODBC_ADD_DSN: u16 = 1;
    const ODBC_CONFIG_DSN: u16 = 2;
    const ODBC_REMOVE_DSN: u16 = 3;

    /// Convert a Windows ANSI code page byte slice to a Rust String.
    unsafe fn acp_to_string(bytes: &[u8]) -> String {
        if bytes.is_empty() {
            return String::new();
        }
        let wide_len = unsafe {
            MultiByteToWideChar(
                CP_ACP,
                0,
                bytes.as_ptr(),
                bytes.len() as i32,
                ptr::null_mut(),
                0,
            )
        };
        if wide_len <= 0 {
            return String::from_utf8_lossy(bytes).into_owned();
        }
        let mut wide_buf = vec![0u16; wide_len as usize];
        unsafe {
            MultiByteToWideChar(
                CP_ACP,
                0,
                bytes.as_ptr(),
                bytes.len() as i32,
                wide_buf.as_mut_ptr(),
                wide_len,
            );
        }
        String::from_utf16_lossy(&wide_buf)
    }

    /// Parse a double-null-terminated wide attribute string into key-value pairs.
    unsafe fn parse_attributes_w(attrs: *const u16) -> Vec<(String, String)> {
        let mut result = Vec::new();
        if attrs.is_null() {
            return result;
        }
        let mut p = attrs;
        loop {
            if unsafe { *p } == 0 {
                break;
            }
            let start = p;
            let mut len = 0usize;
            while unsafe { *p } != 0 {
                len += 1;
                p = unsafe { p.add(1) };
            }
            let slice = unsafe { std::slice::from_raw_parts(start, len) };
            let s = String::from_utf16_lossy(slice);
            if let Some((k, v)) = s.split_once('=') {
                result.push((k.to_string(), v.to_string()));
            }
            p = unsafe { p.add(1) };
        }
        result
    }

    unsafe fn parse_attributes_a(attrs: *const u8) -> Vec<(String, String)> {
        let mut result = Vec::new();
        if attrs.is_null() {
            return result;
        }
        let mut p = attrs;
        loop {
            if unsafe { *p } == 0 {
                break;
            }
            let start = p;
            let mut len = 0usize;
            while unsafe { *p } != 0 {
                len += 1;
                p = unsafe { p.add(1) };
            }
            let slice = unsafe { std::slice::from_raw_parts(start, len) };
            let s = unsafe { acp_to_string(slice) };
            if let Some((k, v)) = s.split_once('=') {
                result.push((k.to_string(), v.to_string()));
            }
            p = unsafe { p.add(1) };
        }
        result
    }

    fn find_dsn(attrs: &[(String, String)]) -> Option<&str> {
        attrs
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("DSN"))
            .map(|(_, v)| v.as_str())
    }

    unsafe fn config_dsn_impl(
        hwnd_parent: *mut core::ffi::c_void,
        f_request: u16,
        driver: &str,
        attrs: &[(String, String)],
    ) -> bool {
        match f_request {
            ODBC_REMOVE_DSN => {
                let Some(dsn) = find_dsn(attrs) else {
                    return false;
                };
                let dsn_w = to_wide(dsn);
                unsafe { SQLRemoveDSNFromIniW(dsn_w.as_ptr()) != 0 }
            }
            ODBC_ADD_DSN | ODBC_CONFIG_DSN => {
                let dsn = find_dsn(attrs).unwrap_or("");
                let is_add = f_request == ODBC_ADD_DSN;

                if !hwnd_parent.is_null() {
                    unsafe {
                        crate::setup_dialog::show_config_dialog(
                            hwnd_parent,
                            is_add,
                            driver,
                            dsn,
                            attrs,
                        )
                    }
                } else if !dsn.is_empty() {
                    unsafe { setup_common::write_dsn_values(dsn, driver, attrs) }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// # Safety
    /// Called by the ODBC Installer DLL (Unicode variant).
    #[unsafe(no_mangle)]
    pub unsafe extern "system" fn ConfigDSNW(
        hwnd_parent: *mut core::ffi::c_void,
        f_request: u16,
        lpsz_driver: *const u16,
        lpsz_attributes: *const u16,
    ) -> i32 {
        let driver = if lpsz_driver.is_null() {
            String::new()
        } else {
            let mut len = 0;
            let mut p = lpsz_driver;
            while unsafe { *p } != 0 {
                len += 1;
                p = unsafe { p.add(1) };
            }
            String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(lpsz_driver, len) })
        };
        let attrs = unsafe { parse_attributes_w(lpsz_attributes) };
        i32::from(unsafe { config_dsn_impl(hwnd_parent, f_request, &driver, &attrs) })
    }

    /// # Safety
    /// Called by the ODBC Installer DLL (ANSI variant).
    /// TODO: Followup in SNOW-3441384. If possible, this should be removed.
    #[unsafe(no_mangle)]
    pub unsafe extern "system" fn ConfigDSN(
        hwnd_parent: *mut core::ffi::c_void,
        f_request: u16,
        lpsz_driver: *const u8,
        lpsz_attributes: *const u8,
    ) -> i32 {
        let driver = if lpsz_driver.is_null() {
            String::new()
        } else {
            let mut len = 0;
            let mut p = lpsz_driver;
            while unsafe { *p } != 0 {
                len += 1;
                p = unsafe { p.add(1) };
            }
            unsafe { acp_to_string(std::slice::from_raw_parts(lpsz_driver, len)) }
        };
        let attrs = unsafe { parse_attributes_a(lpsz_attributes) };
        i32::from(unsafe { config_dsn_impl(hwnd_parent, f_request, &driver, &attrs) })
    }

    /// # Safety
    /// Called by the ODBC Installer DLL for driver-level install/remove hooks.
    #[unsafe(no_mangle)]
    pub unsafe extern "system" fn ConfigDriver(
        _hwnd_parent: *mut core::ffi::c_void,
        _f_request: u16,
        _lpsz_driver: *const u8,
        _lpsz_args: *const u8,
        _lpsz_msg: *mut u8,
        _cb_msg_max: u16,
        _pcb_msg_out: *mut u16,
    ) -> i32 {
        if !_pcb_msg_out.is_null() {
            unsafe { ptr::write(_pcb_msg_out, 0) };
        }
        1 // TRUE
    }
}
