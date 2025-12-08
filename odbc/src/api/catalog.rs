///! ODBC Catalog Functions
///!
///! Implements catalog/metadata functions for ODBC: SQLTables, SQLColumns, etc.
use crate::api::api_utils::cstr_to_string;
use crate::api::error::Required;
use crate::api::types::{ConnectionState, LargeObjectSettings, Statement};
use crate::api::{OdbcError, OdbcResult, statement, stmt_from_handle};
use arrow::array::{ArrayRef, LargeStringArray, StringArray};
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use arrow::util::display::array_value_to_string;
use odbc_sys as sql;
use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::{
    StatementExecuteQueryRequest, StatementExecuteQueryResponse,
    StatementHandle as CoreStatementHandle, StatementNewRequest, StatementReleaseRequest,
    StatementSetSqlQueryRequest,
};
use std::{
    ffi::{CStr, CString},
    fs::OpenOptions,
    io::Write,
    os::raw::c_char,
};
use tracing;

const EMPTY_PROCEDURES_RESULT: &str = "
SELECT
    CAST(NULL AS STRING) AS PROCEDURE_CAT,
    CAST(NULL AS STRING) AS PROCEDURE_SCHEM,
    CAST(NULL AS STRING) AS PROCEDURE_NAME,
    CAST(0 AS INTEGER) AS NUM_INPUT_PARAMS,
    CAST(NULL AS INTEGER) AS NUM_OUTPUT_PARAMS,
    CAST(0 AS INTEGER) AS NUM_RESULT_SETS,
    CAST('' AS STRING) AS REMARKS,
    CAST(2 AS SMALLINT) AS PROCEDURE_TYPE,
    CAST(NULL AS STRING) AS SPECIFIC_NAME
WHERE 1=0
";

const EMPTY_PROCEDURE_COLUMNS_RESULT: &str = "
SELECT
    CAST(NULL AS STRING) AS PROCEDURE_CAT,
    CAST(NULL AS STRING) AS PROCEDURE_SCHEM,
    CAST(NULL AS STRING) AS PROCEDURE_NAME,
    CAST(NULL AS STRING) AS COLUMN_NAME,
    CAST(NULL AS SMALLINT) AS COLUMN_TYPE,
    CAST(NULL AS SMALLINT) AS DATA_TYPE,
    CAST(NULL AS STRING) AS TYPE_NAME,
    CAST(NULL AS NUMBER) AS COLUMN_SIZE,
    CAST(NULL AS NUMBER) AS BUFFER_LENGTH,
    CAST(NULL AS SMALLINT) AS DECIMAL_DIGITS,
    CAST(NULL AS SMALLINT) AS NUM_PREC_RADIX,
    CAST(NULL AS SMALLINT) AS NULLABLE,
    CAST(NULL AS STRING) AS REMARKS,
    CAST(NULL AS STRING) AS COLUMN_DEF,
    CAST(NULL AS SMALLINT) AS SQL_DATA_TYPE,
    CAST(NULL AS SMALLINT) AS SQL_DATETIME_SUB,
    CAST(NULL AS NUMBER) AS CHAR_OCTET_LENGTH,
    CAST(NULL AS NUMBER) AS ORDINAL_POSITION,
    CAST(NULL AS STRING) AS IS_NULLABLE,
    CAST(NULL AS STRING) AS SPECIFIC_NAME
WHERE 1=0
";

const EMPTY_COLUMN_PRIVILEGES_RESULT: &str = "
SELECT
    CAST(NULL AS STRING) AS TABLE_CAT,
    CAST(NULL AS STRING) AS TABLE_SCHEM,
    CAST(NULL AS STRING) AS TABLE_NAME,
    CAST(NULL AS STRING) AS COLUMN_NAME,
    CAST(NULL AS STRING) AS GRANTOR,
    CAST(NULL AS STRING) AS GRANTEE,
    CAST(NULL AS STRING) AS PRIVILEGE,
    CAST(NULL AS STRING) AS IS_GRANTABLE
WHERE 1=0
";

const EMPTY_TABLE_PRIVILEGES_RESULT: &str = "
SELECT
    CAST(NULL AS STRING) AS TABLE_CAT,
    CAST(NULL AS STRING) AS TABLE_SCHEM,
    CAST(NULL AS STRING) AS TABLE_NAME,
    CAST(NULL AS STRING) AS GRANTOR,
    CAST(NULL AS STRING) AS GRANTEE,
    CAST(NULL AS STRING) AS PRIVILEGE,
    CAST(NULL AS STRING) AS IS_GRANTABLE
WHERE 1=0
";

fn is_missing_catalog_error(error: &OdbcError) -> bool {
    let message = error.to_string();
    message.contains("does not exist or not authorized")
        || message.contains("Failed to read batches from query response")
}

fn log_unsupported_api(function_name: &str) {
    tracing::info!("Telemetry for unsupported API, function name: {function_name}");
}

/// SQLTables - List tables in the database
///
/// Returns a result set with table information:
/// TABLE_CAT, TABLE_SCHEM, TABLE_NAME, TABLE_TYPE, REMARKS
pub fn tables(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::Char,
    table_name_length: sql::SmallInt,
    _table_type: *const sql::Char,
    _table_type_length: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("tables: listing tables");

    let stmt = stmt_from_handle(statement_handle);

    // Convert C strings to Rust strings
    let catalog = effective_catalog(stmt, cstr_arg(catalog_name, catalog_name_length)?);
    let schema = cstr_arg(schema_name, schema_name_length)?;
    let table = cstr_arg(table_name, table_name_length)?;

    // Build SHOW TABLES query for Snowflake
    let mut query = String::from("SHOW TABLES");

    if let Some(db) = &catalog {
        if !db.is_empty() && db != "%" {
            query.push_str(&format!(" IN DATABASE {}", format_catalog_identifier(db)));
        }
    }

    if let Some(sch) = &schema {
        if !sch.is_empty() && sch != "%" {
            query.push_str(&format!(" IN SCHEMA {}", sch));
        }
    }

    if let Some(tbl) = &table {
        if !tbl.is_empty() && tbl != "%" {
            query.push_str(&format!(" LIKE '{}'", tbl));
        }
    }

    tracing::debug!("tables: executing query: {}", query);

    let query_cstr = std::ffi::CString::new(query).unwrap();
    let exec_result = statement::exec_direct(
        statement_handle,
        query_cstr.as_ptr() as *const sql::Char,
        sql::NTS as i32,
    );

    if let Err(ref err) = exec_result {
        tracing::error!("SQLColumns query failed: {}", err);
        eprintln!("SQLColumns error: {err}");
    }

    exec_result
}

/// SQLColumns - List columns in specified tables
///
/// Returns a result set with column information:
/// TABLE_CAT, TABLE_SCHEM, TABLE_NAME, COLUMN_NAME, DATA_TYPE, TYPE_NAME, etc.
pub fn columns(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::Char,
    table_name_length: sql::SmallInt,
    column_name: *const sql::Char,
    column_name_length: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("columns: listing columns");
    eprintln!("SQLColumns invoked");

    let stmt = stmt_from_handle(statement_handle);
    ensure_lob_settings(stmt)?;
    let (varchar_fallback_literal, binary_fallback_literal) =
        lob_fallback_literals(&stmt.conn.lob_settings);
    let (varchar_octet_literal, binary_octet_literal) = lob_octet_literals(&stmt.conn.lob_settings);
    let custom_sql_types = stmt.conn.use_custom_sql_types;
    let varchar_length_expr = build_length_expr(
        "base.character_maximum_length",
        &varchar_fallback_literal,
        stmt.conn.lob_settings.default_varchar_size,
    );
    let binary_length_expr = build_length_expr(
        "base.character_octet_length",
        &binary_fallback_literal,
        stmt.conn.lob_settings.default_binary_size,
    );
    tracing::debug!(
        "SQLColumns LOB fallbacks varchar_size={} binary_size={} varchar_octet={} binary_octet={}",
        varchar_fallback_literal,
        binary_fallback_literal,
        varchar_octet_literal,
        binary_octet_literal
    );
    eprintln!(
        "SQLColumns LOB fallbacks varchar_size={} binary_size={} varchar_octet={} binary_octet={}",
        varchar_fallback_literal,
        binary_fallback_literal,
        varchar_octet_literal,
        binary_octet_literal
    );
    let metadata_id = stmt.metadata_id;

    // Convert C strings to Rust strings
    let catalog = effective_catalog(stmt, cstr_arg(catalog_name, catalog_name_length)?);
    let schema = cstr_arg(schema_name, schema_name_length)?;
    let table = cstr_arg(table_name, table_name_length)?;
    let column = cstr_arg(column_name, column_name_length)?;

    if table.is_none() || table.as_deref() == Some("%") {
        tracing::error!("table_name is required for SQLColumns");
        return Ok(());
    }

    let resolved_catalog = resolve_catalog_reference(&catalog, metadata_id);
    let columns_view = info_schema_table("columns", resolved_catalog.as_deref());

    let mut filters = Vec::new();
    if let Some(filter) = build_filter("table_catalog", &catalog, metadata_id) {
        filters.push(filter);
    }
    if let Some(filter) = build_filter("table_schema", &schema, metadata_id) {
        filters.push(filter);
    }
    if let Some(filter) = build_filter("table_name", &table, metadata_id) {
        filters.push(filter);
    }
    if let Some(filter) = build_filter("column_name", &column, metadata_id) {
        filters.push(filter);
    }

    let where_clause = if filters.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", filters.join(" AND "))
    };

    let custom_type_cases = if custom_sql_types {
        "
    WHEN base.data_type IN ('GEOGRAPHY','GEOMETRY') THEN 2004
    WHEN base.data_type = 'ARRAY' THEN 2003
    WHEN base.data_type = 'OBJECT' THEN 2004
    WHEN base.data_type = 'VARIANT' THEN 2005
"
    } else {
        ""
    };

    let data_type_code = format!(
        "
CASE
    {custom_cases}    WHEN base.data_type = 'DECFLOAT' THEN 2
    WHEN base.data_type IN ('TEXT','VARCHAR','CHAR','STRING') THEN 12
    WHEN base.data_type IN ('BINARY','VARBINARY') THEN -3
    WHEN base.data_type IN ('NUMBER','DECIMAL','NUMERIC') THEN 3
    WHEN base.data_type IN ('FLOAT','DOUBLE','REAL') THEN 6
    WHEN base.data_type = 'BOOLEAN' THEN -7
    WHEN base.data_type = 'DATE' THEN 91
    WHEN base.data_type IN ('TIME') THEN 92
    WHEN base.data_type LIKE 'TIMESTAMP%' THEN 93
    ELSE 12
END",
        custom_cases = custom_type_cases
    );

    let column_size_expr = format!(
        "
CASE
    WHEN base.data_type = 'DECFLOAT' THEN 38
    WHEN base.data_type IN ('TEXT','VARCHAR','CHAR','STRING')
        THEN {varchar_length}
    WHEN base.data_type IN ('BINARY','VARBINARY')
        THEN {binary_length}
    WHEN base.data_type IN ('NUMBER','DECIMAL','NUMERIC','FLOAT','DOUBLE','REAL')
        THEN base.numeric_precision
    WHEN base.data_type IN ('TIME') THEN base.numeric_precision
    WHEN base.data_type LIKE 'TIMESTAMP%' THEN base.numeric_precision
    ELSE base.character_maximum_length
END",
        varchar_length = varchar_length_expr,
        binary_length = binary_length_expr
    );

    let char_octet_expr = format!(
        "
CASE
    WHEN base.data_type IN ('TEXT','VARCHAR','CHAR','STRING')
        THEN {varchar_length}
    WHEN base.data_type IN ('BINARY','VARBINARY')
        THEN {binary_length}
    ELSE NULL
END",
        varchar_length = varchar_length_expr.clone(),
        binary_length = binary_length_expr.clone()
    );

    let buffer_length_expr = format!(
        "
CASE
    WHEN base.data_type = 'DECFLOAT' THEN 19
    WHEN base.data_type IN ('TEXT','VARCHAR','CHAR','STRING')
        THEN {varchar_length}
    WHEN base.data_type IN ('BINARY','VARBINARY')
        THEN {binary_length}
    WHEN base.data_type IN ('NUMBER','DECIMAL','NUMERIC','FLOAT','DOUBLE','REAL')
        THEN base.numeric_precision
    WHEN base.data_type IN ('TIME') THEN base.numeric_precision
    WHEN base.data_type LIKE 'TIMESTAMP%' THEN base.numeric_precision
    ELSE base.character_maximum_length
END",
        varchar_length = varchar_length_expr.clone(),
        binary_length = binary_length_expr.clone()
    );

    let num_prec_radix = "
CASE
    WHEN base.data_type IN ('NUMBER','DECIMAL','NUMERIC','DECFLOAT') THEN 10
    WHEN base.data_type IN ('FLOAT','DOUBLE','REAL') THEN 2
    ELSE NULL
END";

    let type_name_expr = "
CASE
    WHEN base.data_type = 'DECFLOAT' THEN 'NUMERIC'
    ELSE base.data_type
END";

    let query = format!(
        r#"
WITH base AS (
    SELECT
        table_catalog,
        table_schema,
        table_name,
        column_name,
        data_type,
        ordinal_position,
        is_nullable,
        column_default,
        comment,
        character_maximum_length,
        character_octet_length,
        numeric_precision,
        numeric_scale
    FROM {columns_view}
    {where_clause}
)
SELECT
    base.table_catalog AS TABLE_CAT,
    base.table_schema AS TABLE_SCHEM,
    base.table_name   AS TABLE_NAME,
    base.column_name  AS COLUMN_NAME,
    {data_type_code} AS DATA_TYPE,
    {type_name}    AS TYPE_NAME,
    {column_size} AS COLUMN_SIZE,
    {buffer_length} AS BUFFER_LENGTH,
    base.numeric_scale AS DECIMAL_DIGITS,
    {num_prec_radix} AS NUM_PREC_RADIX,
    CASE WHEN base.is_nullable = 'YES' THEN 1 ELSE 0 END AS NULLABLE,
    base.comment AS REMARKS,
    base.column_default AS COLUMN_DEF,
    {data_type_code} AS SQL_DATA_TYPE,
    CAST(NULL AS SMALLINT) AS SQL_DATETIME_SUB,
    {char_octet} AS CHAR_OCTET_LENGTH,
    base.ordinal_position AS ORDINAL_POSITION,
    base.is_nullable AS IS_NULLABLE
FROM base
ORDER BY TABLE_CAT, TABLE_SCHEM, TABLE_NAME, ORDINAL_POSITION
"#,
        columns_view = columns_view,
        where_clause = where_clause,
        data_type_code = data_type_code,
        column_size = column_size_expr,
        buffer_length = buffer_length_expr,
        num_prec_radix = num_prec_radix,
        char_octet = char_octet_expr,
        type_name = type_name_expr
    );

    eprintln!("SQLColumns query: {query}");
    tracing::debug!("columns: executing query: {}", query);

    let query_cstr = std::ffi::CString::new(query).unwrap();
    statement::exec_direct(
        statement_handle,
        query_cstr.as_ptr() as *const sql::Char,
        sql::NTS as i32,
    )
}

/// SQLColumnPrivileges - Get privileges on table columns
pub fn column_privileges(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::Char,
    table_name_length: sql::SmallInt,
    column_name: *const sql::Char,
    column_name_length: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("column_privileges: listing column privileges");
    log_unsupported_api("SQLColumnPrivileges");

    let stmt = stmt_from_handle(statement_handle);
    let metadata_id = stmt.metadata_id;

    let catalog = effective_catalog(stmt, cstr_arg(catalog_name, catalog_name_length)?);
    let schema = cstr_arg(schema_name, schema_name_length)?;
    let table = cstr_arg(table_name, table_name_length)?;
    let column = cstr_arg(column_name, column_name_length)?;

    let resolved_catalog = resolve_catalog_reference(&catalog, metadata_id);
    let privileges_view = info_schema_table("column_privileges", resolved_catalog.as_deref());

    let mut filters = Vec::new();
    if let Some(filter) = build_filter("table_catalog", &catalog, metadata_id) {
        filters.push(filter);
    }
    if let Some(filter) = build_filter("table_schema", &schema, metadata_id) {
        filters.push(filter);
    }
    if let Some(filter) = build_filter("table_name", &table, metadata_id) {
        filters.push(filter);
    }
    if let Some(filter) = build_filter("column_name", &column, metadata_id) {
        filters.push(filter);
    }
    let where_clause = if filters.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", filters.join(" AND "))
    };

    let query = format!(
        r#"
SELECT
    table_catalog AS TABLE_CAT,
    table_schema AS TABLE_SCHEM,
    table_name AS TABLE_NAME,
    column_name AS COLUMN_NAME,
    grantor AS GRANTOR,
    grantee AS GRANTEE,
    privilege_type AS PRIVILEGE,
    is_grantable AS IS_GRANTABLE
FROM {view}
{where_clause}
ORDER BY TABLE_CAT, TABLE_SCHEM, TABLE_NAME, COLUMN_NAME, PRIVILEGE
"#,
        view = privileges_view,
        where_clause = where_clause
    );

    tracing::debug!("column_privileges: executing query: {}", query);
    eprintln!("SQLColumnPrivileges query: {query}");
    let query_cstr = CString::new(query).unwrap();
    let mut result = statement::exec_direct(
        statement_handle,
        query_cstr.as_ptr() as *const sql::Char,
        sql::NTS as i32,
    );
    if let Err(ref err) = result {
        if resolved_catalog.is_some() && is_missing_catalog_error(err) {
            let empty_cstr = CString::new(EMPTY_COLUMN_PRIVILEGES_RESULT).unwrap();
            result = statement::exec_direct(
                statement_handle,
                empty_cstr.as_ptr() as *const sql::Char,
                sql::NTS as i32,
            );
        }
    }
    if let Err(ref err) = result {
        tracing::error!("column_privileges: exec_direct failed: {err}");
        eprintln!("SQLColumnPrivileges error: {err}");
    }
    result
}

/// SQLTablePrivileges - Get privileges on tables
pub fn table_privileges(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::Char,
    table_name_length: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("table_privileges: listing table privileges");
    log_unsupported_api("SQLTablePrivileges");

    let stmt = stmt_from_handle(statement_handle);
    let metadata_id = stmt.metadata_id;

    let catalog = effective_catalog(stmt, cstr_arg(catalog_name, catalog_name_length)?);
    let schema = cstr_arg(schema_name, schema_name_length)?;
    let table = cstr_arg(table_name, table_name_length)?;

    let resolved_catalog = resolve_catalog_reference(&catalog, metadata_id);
    let privileges_view = info_schema_table("table_privileges", resolved_catalog.as_deref());

    let mut filters = Vec::new();
    if let Some(filter) = build_filter("table_catalog", &catalog, metadata_id) {
        filters.push(filter);
    }
    if let Some(filter) = build_filter("table_schema", &schema, metadata_id) {
        filters.push(filter);
    }
    if let Some(filter) = build_filter("table_name", &table, metadata_id) {
        filters.push(filter);
    }
    let where_clause = if filters.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", filters.join(" AND "))
    };

    let query = format!(
        r#"
SELECT
    table_catalog AS TABLE_CAT,
    table_schema AS TABLE_SCHEM,
    table_name AS TABLE_NAME,
    grantor AS GRANTOR,
    grantee AS GRANTEE,
    privilege_type AS PRIVILEGE,
    is_grantable AS IS_GRANTABLE
FROM {view}
{where_clause}
ORDER BY TABLE_CAT, TABLE_SCHEM, TABLE_NAME, PRIVILEGE
"#,
        view = privileges_view,
        where_clause = where_clause
    );

    tracing::debug!("table_privileges: executing query: {}", query);
    eprintln!("SQLTablePrivileges query: {query}");
    let query_cstr = CString::new(query).unwrap();
    let mut result = statement::exec_direct(
        statement_handle,
        query_cstr.as_ptr() as *const sql::Char,
        sql::NTS as i32,
    );
    if let Err(ref err) = result {
        if resolved_catalog.is_some() && is_missing_catalog_error(err) {
            let empty_cstr = CString::new(EMPTY_TABLE_PRIVILEGES_RESULT).unwrap();
            result = statement::exec_direct(
                statement_handle,
                empty_cstr.as_ptr() as *const sql::Char,
                sql::NTS as i32,
            );
        }
    }
    if let Err(ref err) = result {
        tracing::error!("table_privileges: exec_direct failed: {err}");
        eprintln!("SQLTablePrivileges error: {err}");
    }
    result
}

/// SQLGetTypeInfo - Get information about supported data types
pub fn get_type_info(statement_handle: sql::Handle, data_type: sql::SmallInt) -> OdbcResult<()> {
    tracing::debug!("get_type_info: data_type={}", data_type);
    let _stmt = stmt_from_handle(statement_handle);

    let query = build_type_info_query(data_type);
    let query_cstr = std::ffi::CString::new(query).unwrap();

    statement::exec_direct(
        statement_handle,
        query_cstr.as_ptr() as *const sql::Char,
        sql::NTS as i32,
    )
}

/// SQLPrimaryKeys - Get primary key columns for a table
pub fn primary_keys(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::Char,
    table_name_length: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("primary_keys: getting primary keys");
    let stmt = stmt_from_handle(statement_handle);
    let metadata_id = stmt.metadata_id;

    let catalog = effective_catalog(stmt, cstr_arg(catalog_name, catalog_name_length)?);
    let schema = cstr_arg(schema_name, schema_name_length)?;
    let table = cstr_arg(table_name, table_name_length)?;
    let resolved_catalog = resolve_catalog_reference(&catalog, metadata_id);

    if table.as_deref().map(|s| s == "%").unwrap_or(true) {
        tracing::warn!("primary_keys: table name is required and cannot be '%'");
        return Ok(());
    }

    let constraints = info_schema_table("table_constraints", catalog.as_deref());
    let key_usage = info_schema_table("key_column_usage", catalog.as_deref());

    let mut constraint_filters = vec!["constraint_type = 'PRIMARY KEY'".to_string()];
    if let Some(db) = &catalog {
        constraint_filters.push(format!("table_catalog = '{}'", escape_literal(db)));
    }
    if let Some(sch) = &schema {
        if sch != "%" {
            constraint_filters.push(format!("table_schema = '{}'", escape_literal(sch)));
        }
    }
    if let Some(tbl) = &table {
        constraint_filters.push(format!("table_name = '{}'", escape_literal(tbl)));
    }
    let constraint_where = format!("WHERE {}", constraint_filters.join(" AND "));

    let mut result_filters = Vec::new();
    if let Some(sch) = &schema {
        if sch != "%" {
            result_filters.push(format!("kc.table_schema = '{}'", escape_literal(sch)));
        }
    }
    if let Some(tbl) = &table {
        result_filters.push(format!("kc.table_name = '{}'", escape_literal(tbl)));
    }
    let result_where = if result_filters.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", result_filters.join(" AND "))
    };

    let query = format!(
        r#"
WITH pk AS (
    SELECT constraint_catalog, constraint_schema, table_name, constraint_name
    FROM {constraints}
    {constraint_where}
)
SELECT
    kc.table_catalog AS TABLE_CAT,
    kc.table_schema AS TABLE_SCHEM,
    kc.table_name AS TABLE_NAME,
    kc.column_name AS COLUMN_NAME,
    kc.ordinal_position AS KEY_SEQ,
    kc.constraint_name AS PK_NAME
FROM {key_usage} kc
JOIN pk
  ON kc.constraint_name = pk.constraint_name
 AND kc.table_schema = pk.constraint_schema
 AND kc.table_name = pk.table_name
{result_where}
ORDER BY TABLE_NAME, KEY_SEQ
"#,
        constraints = constraints,
        constraint_where = constraint_where,
        key_usage = key_usage,
        result_where = result_where,
    );

    let query_cstr = std::ffi::CString::new(query).unwrap();
    let mut exec_result = statement::exec_direct(
        statement_handle,
        query_cstr.as_ptr() as *const sql::Char,
        sql::NTS as i32,
    );
    if let Err(ref err) = exec_result {
        if resolved_catalog.is_some() && is_missing_catalog_error(err) {
            let empty_cstr = std::ffi::CString::new(EMPTY_PROCEDURES_RESULT).unwrap();
            exec_result = statement::exec_direct(
                statement_handle,
                empty_cstr.as_ptr() as *const sql::Char,
                sql::NTS as i32,
            );
        }
    }
    if let Err(ref err) = exec_result {
        tracing::error!("procedures: exec_direct failed: {err}");
        eprintln!("SQLProcedures query failed: {err}");
    }
    exec_result
}

/// SQLStatistics - Get table statistics and indexes
pub fn statistics(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::Char,
    table_name_length: sql::SmallInt,
    unique: sql::USmallInt,
    reserved: sql::USmallInt,
) -> OdbcResult<()> {
    tracing::debug!(
        "statistics: getting table statistics, unique={}, reserved={}",
        unique,
        reserved
    );
    log_unsupported_api("SQLStatistics");

    let stmt = stmt_from_handle(statement_handle);

    let catalog = effective_catalog(stmt, cstr_arg(catalog_name, catalog_name_length)?);
    let schema = cstr_arg(schema_name, schema_name_length)?;
    let table = cstr_arg(table_name, table_name_length)?;

    if table.as_deref().map(|s| s == "%").unwrap_or(true) {
        tracing::warn!("statistics: table name is required and cannot be '%'");
        return Ok(());
    }

    let columns_view = info_schema_table("columns", catalog.as_deref());
    let mut filters = Vec::new();
    if let Some(db) = &catalog {
        filters.push(format!("table_catalog = '{}'", escape_literal(db)));
    }
    if let Some(sch) = &schema {
        if sch != "%" {
            filters.push(format!("table_schema = '{}'", escape_literal(sch)));
        }
    }
    if let Some(tbl) = &table {
        filters.push(format!("table_name = '{}'", escape_literal(tbl)));
    }
    let where_clause = if filters.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", filters.join(" AND "))
    };

    let query = format!(
        r#"
SELECT
    table_catalog AS TABLE_CAT,
    table_schema AS TABLE_SCHEM,
    table_name AS TABLE_NAME,
    1::SMALLINT AS NON_UNIQUE,
    NULL::STRING AS INDEX_QUALIFIER,
    'COLUMN_SCAN'::STRING AS INDEX_NAME,
    3::SMALLINT AS TYPE,
    ordinal_position::INTEGER AS ORDINAL_POSITION,
    column_name AS COLUMN_NAME,
    NULL::STRING AS ASC_OR_DESC,
    NULL::NUMBER AS CARDINALITY,
    NULL::NUMBER AS PAGES,
    NULL::STRING AS FILTER_CONDITION
FROM {columns}
{where_clause}
ORDER BY table_name, ordinal_position
"#,
        columns = columns_view,
        where_clause = where_clause,
    );

    let query_cstr = std::ffi::CString::new(query).unwrap();
    statement::exec_direct(
        statement_handle,
        query_cstr.as_ptr() as *const sql::Char,
        sql::NTS as i32,
    )
}

/// SQLForeignKeys - Get foreign key columns
pub fn foreign_keys(
    statement_handle: sql::Handle,
    pk_catalog_name: *const sql::Char,
    pk_catalog_name_length: sql::SmallInt,
    pk_schema_name: *const sql::Char,
    pk_schema_name_length: sql::SmallInt,
    pk_table_name: *const sql::Char,
    pk_table_name_length: sql::SmallInt,
    fk_catalog_name: *const sql::Char,
    fk_catalog_name_length: sql::SmallInt,
    fk_schema_name: *const sql::Char,
    fk_schema_name_length: sql::SmallInt,
    fk_table_name: *const sql::Char,
    fk_table_name_length: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("foreign_keys: getting foreign keys");

    let pk_catalog = cstr_arg(pk_catalog_name, pk_catalog_name_length)?;
    let pk_schema = cstr_arg(pk_schema_name, pk_schema_name_length)?;
    let pk_table = cstr_arg(pk_table_name, pk_table_name_length)?;
    let fk_catalog = cstr_arg(fk_catalog_name, fk_catalog_name_length)?;
    let fk_schema = cstr_arg(fk_schema_name, fk_schema_name_length)?;
    let fk_table = cstr_arg(fk_table_name, fk_table_name_length)?;

    let catalog = fk_catalog.clone().or(pk_catalog.clone());
    let constraints = info_schema_table("referential_constraints", catalog.as_deref());
    let key_usage = info_schema_table("key_column_usage", catalog.as_deref());

    let mut where_clauses = Vec::new();
    if let Some(schema) = &fk_schema {
        if schema != "%" {
            where_clauses.push(format!("kc.table_schema = '{}'", escape_literal(schema)));
        }
    }
    if let Some(table) = &fk_table {
        where_clauses.push(format!("kc.table_name = '{}'", escape_literal(table)));
    }
    if let Some(schema) = &pk_schema {
        if schema != "%" {
            where_clauses.push(format!("pk.table_schema = '{}'", escape_literal(schema)));
        }
    }
    if let Some(table) = &pk_table {
        where_clauses.push(format!("pk.table_name = '{}'", escape_literal(table)));
    }
    let where_clause = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let query = format!(
        r#"
WITH fk AS (
    SELECT
        constraint_catalog,
        constraint_schema,
        constraint_name,
        unique_constraint_catalog,
        unique_constraint_schema,
        unique_constraint_name
    FROM {constraints}
)
SELECT
    pk.table_catalog AS PKTABLE_CAT,
    pk.table_schema AS PKTABLE_SCHEM,
    pk.table_name AS PKTABLE_NAME,
    pk.column_name AS PKCOLUMN_NAME,
    kc.table_catalog AS FKTABLE_CAT,
    kc.table_schema AS FKTABLE_SCHEM,
    kc.table_name AS FKTABLE_NAME,
    kc.column_name AS FKCOLUMN_NAME,
    kc.ordinal_position AS KEY_SEQ,
    3::SMALLINT AS UPDATE_RULE,
    3::SMALLINT AS DELETE_RULE,
    kc.constraint_name AS FK_NAME,
    fk.unique_constraint_name AS PK_NAME,
    7::SMALLINT AS DEFERRABILITY
FROM {key_usage} kc
JOIN fk
  ON kc.constraint_catalog = fk.constraint_catalog
 AND kc.constraint_schema = fk.constraint_schema
 AND kc.constraint_name = fk.constraint_name
JOIN {key_usage} pk
  ON fk.unique_constraint_catalog = pk.constraint_catalog
 AND fk.unique_constraint_schema = pk.constraint_schema
 AND fk.unique_constraint_name = pk.constraint_name
 AND pk.ordinal_position = kc.position_in_unique_constraint
{where_clause}
ORDER BY FKTABLE_NAME, KEY_SEQ
"#,
        constraints = constraints,
        key_usage = key_usage,
        where_clause = where_clause,
    );

    let query_cstr = std::ffi::CString::new(query).unwrap();
    statement::exec_direct(
        statement_handle,
        query_cstr.as_ptr() as *const sql::Char,
        sql::NTS as i32,
    )
}

/// SQLSpecialColumns - Get special columns (row identifier, auto-update)
pub fn special_columns(
    statement_handle: sql::Handle,
    identifier_type: sql::USmallInt,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::Char,
    table_name_length: sql::SmallInt,
    scope: sql::USmallInt,
    nullable: sql::USmallInt,
) -> OdbcResult<()> {
    tracing::debug!(
        "special_columns: identifier_type={}, scope={}, nullable={}",
        identifier_type,
        scope,
        nullable
    );
    log_unsupported_api("SQLSpecialColumns");

    let stmt = stmt_from_handle(statement_handle);

    let catalog = effective_catalog(stmt, cstr_arg(catalog_name, catalog_name_length)?);
    let schema = cstr_arg(schema_name, schema_name_length)?;
    let table = cstr_arg(table_name, table_name_length)?;

    if table.as_deref().map(|s| s == "%").unwrap_or(true) {
        tracing::warn!("special_columns: table name is required and cannot be '%'");
        return Ok(());
    }

    let columns_view = info_schema_table("columns", catalog.as_deref());
    let mut filters = Vec::new();
    if let Some(db) = &catalog {
        filters.push(format!("table_catalog = '{}'", escape_literal(db)));
    }
    if let Some(sch) = &schema {
        if sch != "%" {
            filters.push(format!("table_schema = '{}'", escape_literal(sch)));
        }
    }
    if let Some(tbl) = &table {
        filters.push(format!("table_name = '{}'", escape_literal(tbl)));
    }

    let where_clause = if filters.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", filters.join(" AND "))
    };

    let query = format!(
        r#"
WITH ordered_columns AS (
    SELECT
        table_catalog,
        table_schema,
        table_name,
        column_name,
        data_type,
        character_maximum_length,
        numeric_precision,
        numeric_scale,
        ordinal_position,
        ROW_NUMBER() OVER (ORDER BY ordinal_position) AS rn
    FROM {columns}
    {where_clause}
)
SELECT
    {scope}::SMALLINT AS SCOPE,
    column_name AS COLUMN_NAME,
    CASE
        WHEN data_type IN ('TEXT', 'VARCHAR') THEN 12
        WHEN data_type = 'NUMBER' THEN 2
        WHEN data_type = 'BOOLEAN' THEN -7
        WHEN data_type LIKE 'TIMESTAMP%' THEN 93
        WHEN data_type = 'DATE' THEN 91
        WHEN data_type = 'FLOAT' THEN 8
        ELSE 12
    END AS DATA_TYPE,
    data_type AS TYPE_NAME,
    COALESCE(character_maximum_length, numeric_precision) AS COLUMN_SIZE,
    COALESCE(character_maximum_length, numeric_precision) AS BUFFER_LENGTH,
    COALESCE(numeric_scale, 0) AS DECIMAL_DIGITS,
    0::SMALLINT AS PSEUDO_COLUMN
FROM ordered_columns
WHERE rn = 1
"#,
        columns = columns_view,
        where_clause = where_clause,
        scope = scope,
    );

    let query_cstr = std::ffi::CString::new(query).unwrap();
    statement::exec_direct(
        statement_handle,
        query_cstr.as_ptr() as *const sql::Char,
        sql::NTS as i32,
    )
}

fn cstr_arg(ptr: *const sql::Char, length: sql::SmallInt) -> OdbcResult<Option<String>> {
    if ptr.is_null() {
        return Ok(None);
    }

    if length == 0 {
        return Ok(None);
    }

    if length == sql::NTS as i16 || length < 0 {
        let value = unsafe { CStr::from_ptr(ptr as *const c_char) }
            .to_string_lossy()
            .into_owned();
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    } else {
        let value = cstr_to_string(ptr, length as i32)?;
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }
}

fn effective_catalog<'a>(stmt: &Statement<'a>, catalog: Option<String>) -> Option<String> {
    match catalog {
        Some(value) => Some(value),
        None => {
            if stmt.conn.use_current_catalog {
                stmt.conn.current_catalog.clone()
            } else {
                None
            }
        }
    }
}

fn escape_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn build_length_expr(base_expr: &str, fallback_literal: &str, user_limit: Option<i64>) -> String {
    if let Some(limit) = user_limit {
        format!(
            "COALESCE(LEAST({base}, {limit}), {limit})",
            base = base_expr,
            limit = limit
        )
    } else if fallback_literal.eq_ignore_ascii_case("NULL") {
        base_expr.to_string()
    } else {
        format!(
            "COALESCE({base}, {fallback})",
            base = base_expr,
            fallback = fallback_literal
        )
    }
}

fn varchar_fallback_size(settings: &LargeObjectSettings) -> Option<i64> {
    settings.default_varchar_size.or_else(|| {
        if settings.enable_large_varchar_binary.unwrap_or(false) {
            settings.max_lob_size_in_memory
        } else {
            None
        }
    })
}

fn binary_fallback_size(settings: &LargeObjectSettings) -> Option<i64> {
    if let Some(value) = settings.default_binary_size {
        Some(value)
    } else if settings.enable_large_varchar_binary.unwrap_or(false) {
        settings.max_lob_size_in_memory.map(|max| max / 2)
    } else {
        None
    }
}

fn lob_fallback_literals(settings: &LargeObjectSettings) -> (String, String) {
    let varchar_literal = varchar_fallback_size(settings)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "NULL".to_string());
    let binary_literal = binary_fallback_size(settings)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "NULL".to_string());
    (varchar_literal, binary_literal)
}

fn lob_octet_literals(settings: &LargeObjectSettings) -> (String, String) {
    let varchar_literal = varchar_fallback_size(settings)
        .map(|value| value.saturating_mul(4).to_string())
        .unwrap_or_else(|| "NULL".to_string());
    let binary_literal = binary_fallback_size(settings)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "NULL".to_string());
    (varchar_literal, binary_literal)
}

fn ensure_lob_settings(stmt: &mut Statement) -> OdbcResult<()> {
    let needs_flag = stmt.conn.lob_settings.enable_large_varchar_binary.is_none();
    let needs_max = stmt.conn.lob_settings.max_lob_size_in_memory.is_none();

    if !needs_flag && !needs_max {
        return Ok(());
    }

    if needs_flag {
        if let Some(value) =
            fetch_parameter_string(stmt, "ENABLE_LARGE_VARCHAR_AND_BINARY_IN_RESULT")?
        {
            stmt.conn.lob_settings.enable_large_varchar_binary =
                Some(value.eq_ignore_ascii_case("true"));
        }
    }

    if needs_max {
        if let Some(value) = fetch_parameter_string(stmt, "MAX_LOB_SIZE_IN_MEMORY")? {
            if let Ok(num) = value.parse::<i64>() {
                stmt.conn.lob_settings.max_lob_size_in_memory = Some(num);
            }
        }
    }

    Ok(())
}

fn fetch_parameter_string(stmt: &mut Statement, param_name: &str) -> OdbcResult<Option<String>> {
    execute_show_parameter_query(&stmt.conn.state, param_name)
}

fn execute_show_parameter_query(
    state: &ConnectionState,
    param_name: &str,
) -> OdbcResult<Option<String>> {
    let conn_handle = match state {
        ConnectionState::Connected { conn_handle, .. } => *conn_handle,
        ConnectionState::Disconnected => {
            return Err(OdbcError::Disconnected {
                location: snafu::location!(),
            });
        }
    };

    let new_stmt = DatabaseDriverClient::statement_new(StatementNewRequest {
        conn_handle: Some(conn_handle),
    })?;
    let temp_handle = new_stmt
        .stmt_handle
        .required("temporary statement handle is required")?;

    let result = execute_scalar_query(temp_handle.clone(), param_name);

    let _ = DatabaseDriverClient::statement_release(StatementReleaseRequest {
        stmt_handle: Some(temp_handle),
    });

    result
}

fn execute_scalar_query(
    stmt_handle: CoreStatementHandle,
    param_name: &str,
) -> OdbcResult<Option<String>> {
    let query = format!(
        "SHOW PARAMETERS LIKE '{}'",
        escape_single_quotes(param_name)
    );

    DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
        stmt_handle: Some(stmt_handle.clone()),
        query: query.clone(),
    })?;

    let response = DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
        stmt_handle: Some(stmt_handle),
        describe_only: false,
    })?;

    extract_first_string(response, 1, &query)
}

fn extract_first_string(
    response: StatementExecuteQueryResponse,
    column_index: usize,
    context: &str,
) -> OdbcResult<Option<String>> {
    let result = match response.result {
        Some(result) => result,
        None => return Ok(None),
    };

    let stream_ptr = match result.stream {
        Some(stream) => stream,
        None => return Ok(None),
    };

    let raw_stream: *mut FFI_ArrowArrayStream = stream_ptr.into();
    let stream = unsafe { FFI_ArrowArrayStream::from_raw(raw_stream) };
    let mut reader =
        ArrowArrayStreamReader::try_new(stream).map_err(|_| OdbcError::ExecuteStatement {
            statement: context.to_string(),
            location: snafu::location!(),
        })?;

    while let Some(batch_result) = reader.next() {
        let batch = batch_result.map_err(|_| OdbcError::ExecuteStatement {
            statement: context.to_string(),
            location: snafu::location!(),
        })?;
        if batch.num_rows() == 0 || batch.num_columns() <= column_index {
            continue;
        }
        let value = get_string_from_array(batch.column(column_index), 0).map_err(|_| {
            OdbcError::ExecuteStatement {
                statement: context.to_string(),
                location: snafu::location!(),
            }
        })?;
        return Ok(value);
    }

    Ok(None)
}

fn get_string_from_array(
    array: &ArrayRef,
    row: usize,
) -> Result<Option<String>, arrow::error::ArrowError> {
    if array.is_null(row) {
        return Ok(None);
    }
    if let Some(arr) = array.as_any().downcast_ref::<StringArray>() {
        return Ok(Some(arr.value(row).to_string()));
    }
    if let Some(arr) = array.as_any().downcast_ref::<LargeStringArray>() {
        return Ok(Some(arr.value(row).to_string()));
    }
    Ok(Some(array_value_to_string(array.as_ref(), row)?))
}

fn escape_single_quotes(input: &str) -> String {
    input.replace('\'', "''")
}

fn info_schema_table(table: &str, catalog: Option<&str>) -> String {
    if let Some(db) = catalog {
        format!(
            "{}.information_schema.{}",
            format_catalog_identifier(db),
            table
        )
    } else {
        format!("information_schema.{}", table)
    }
}

fn build_type_info_query(data_type: sql::SmallInt) -> String {
    // Column order per ODBC spec (19 columns)
    // TYPE_NAME, DATA_TYPE, COLUMN_SIZE, LITERAL_PREFIX, LITERAL_SUFFIX, CREATE_PARAMS,
    // NULLABLE, CASE_SENSITIVE, SEARCHABLE, UNSIGNED_ATTRIBUTE, FIXED_PREC_SCALE,
    // AUTO_UNIQUE_VALUE, LOCAL_TYPE_NAME, MINIMUM_SCALE, MAXIMUM_SCALE,
    // SQL_DATA_TYPE, SQL_DATETIME_SUB, NUM_PREC_RADIX, INTERVAL_PRECISION
    let base_query = r#"
WITH type_info AS (
    SELECT * FROM (
        SELECT
            'VARCHAR'::STRING AS TYPE_NAME,
            12::SMALLINT AS DATA_TYPE,
            16777216::INTEGER AS COLUMN_SIZE,
            ''''::STRING AS LITERAL_PREFIX,
            ''''::STRING AS LITERAL_SUFFIX,
            'length'::STRING AS CREATE_PARAMS,
            1::SMALLINT AS NULLABLE,
            1::SMALLINT AS CASE_SENSITIVE,
            3::SMALLINT AS SEARCHABLE,
            0::SMALLINT AS UNSIGNED_ATTRIBUTE,
            0::SMALLINT AS FIXED_PREC_SCALE,
            0::SMALLINT AS AUTO_UNIQUE_VALUE,
            'VARCHAR'::STRING AS LOCAL_TYPE_NAME,
            0::SMALLINT AS MINIMUM_SCALE,
            0::SMALLINT AS MAXIMUM_SCALE,
            12::SMALLINT AS SQL_DATA_TYPE,
            NULL::SMALLINT AS SQL_DATETIME_SUB,
            0::SMALLINT AS NUM_PREC_RADIX,
            NULL::SMALLINT AS INTERVAL_PRECISION
        UNION ALL
        SELECT
            'NUMERIC', 2, 38,
            NULL, NULL, 'precision,scale',
            1, 0, 3,
            0, 0, 0,
            'NUMERIC', 0, 8192,
            2, NULL, 10, NULL
        UNION ALL
        SELECT
            'BOOLEAN', -7, 1,
            NULL, NULL, NULL,
            1, 0, 3,
            0, 0, 0,
            'BOOLEAN', 0, 0,
            -7, NULL, 0, NULL
        UNION ALL
        SELECT
            'DATE', 91, 10,
            '''', '''', NULL,
            1, 0, 3,
            0, 0, 0,
            'DATE', 0, 0,
            9, 1, 0, NULL
        UNION ALL
        SELECT
            'TIMESTAMP_NTZ', 93, 29,
            '''', '''', NULL,
            1, 0, 3,
            0, 0, 0,
            'TIMESTAMP_NTZ', 0, 9,
            9, 3, 0, 9
        UNION ALL
        SELECT
            'FLOAT', 8, 53,
            NULL, NULL, NULL,
            1, 0, 3,
            0, 0, 0,
            'FLOAT', NULL, NULL,
            8, NULL, 2, NULL
        UNION ALL
        SELECT
            'BINARY', -3, 8388608,
            '0x', NULL, 'length',
            1, 0, 3,
            0, 0, 0,
            'BINARY', 0, 0,
            -3, NULL, 2, NULL
        UNION ALL
        SELECT
            'VARIANT', -150, NULL,
            NULL, NULL, NULL,
            1, 0, 3,
            0, 0, 0,
            'VARIANT', NULL, NULL,
            -150, NULL, 0, NULL
    )
)
SELECT
    TYPE_NAME, DATA_TYPE, COLUMN_SIZE, LITERAL_PREFIX, LITERAL_SUFFIX,
    CREATE_PARAMS, NULLABLE, CASE_SENSITIVE, SEARCHABLE, UNSIGNED_ATTRIBUTE,
    FIXED_PREC_SCALE, AUTO_UNIQUE_VALUE, LOCAL_TYPE_NAME, MINIMUM_SCALE,
    MAXIMUM_SCALE, SQL_DATA_TYPE, SQL_DATETIME_SUB, NUM_PREC_RADIX,
    INTERVAL_PRECISION
FROM type_info
"#;

    if data_type == 0 {
        base_query.to_string()
    } else {
        format!(
            "{base} WHERE DATA_TYPE = {code}",
            base = base_query,
            code = data_type
        )
    }
}

fn build_filter(column: &str, value: &Option<String>, metadata_id: bool) -> Option<String> {
    let raw_value = value.as_ref()?;
    if metadata_id {
        let trimmed_end = raw_value.trim_end();
        if trimmed_end.is_empty() {
            return None;
        }
        if has_invalid_identifier_leading_whitespace(trimmed_end) {
            return Some("1=0".to_string());
        }
        let ident = normalize_identifier(trimmed_end);
        if ident.is_empty() {
            Some("1=0".to_string())
        } else {
            Some(format!("{column} = '{}'", escape_literal(&ident)))
        }
    } else {
        let raw = raw_value.trim();
        if raw.is_empty() {
            return None;
        }
        if raw == "%" {
            return None;
        }
        let pattern = escape_like_pattern(raw);
        Some(format!("{column} ILIKE '{pattern}' ESCAPE '\\\\'"))
    }
}

fn normalize_identifier(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_uppercase()
    }
}

fn has_invalid_identifier_leading_whitespace(input: &str) -> bool {
    let trimmed_end = input.trim_end();
    if trimmed_end.is_empty() {
        return true;
    }
    let trimmed_start = trimmed_end.trim_start();
    let has_leading = trimmed_start.len() != trimmed_end.len();
    has_leading && !trimmed_start.starts_with('"')
}

fn escape_like_pattern(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\'' => escaped.push_str("''"),
            '\\' => escaped.push_str("\\\\"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn format_catalog_identifier(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        return trimmed.to_string();
    }
    if needs_identifier_quotes(trimmed) {
        let escaped = trimmed.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        trimmed.to_uppercase()
    }
}

fn needs_identifier_quotes(value: &str) -> bool {
    value.chars().any(|ch| {
        ch.is_lowercase() || ch == ' ' || ch == '-' || ch == '$' || ch == '/' || ch == '.'
    })
}

fn resolve_catalog_reference(value: &Option<String>, metadata_id: bool) -> Option<String> {
    match value {
        Some(v) => {
            if metadata_id {
                let ident = normalize_identifier(v);
                if ident.is_empty() { None } else { Some(ident) }
            } else {
                let trimmed = v.trim();
                if trimmed.is_empty() || trimmed == "%" || trimmed.contains('%') {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
        }
        None => None,
    }
}

/// SQLProcedures - List stored procedures
pub fn procedures(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    proc_name: *const sql::Char,
    proc_name_length: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("procedures: listing stored procedures");
    eprintln!("SQLProcedures invoked");

    let stmt = stmt_from_handle(statement_handle);
    let metadata_id = stmt.metadata_id;

    let catalog = cstr_arg(catalog_name, catalog_name_length)?;
    let schema = cstr_arg(schema_name, schema_name_length)?;
    let procedure = cstr_arg(proc_name, proc_name_length)?;

    let resolved_catalog = resolve_catalog_reference(&catalog, metadata_id);
    let procedures_view = info_schema_table("procedures", resolved_catalog.as_deref());
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_debug.log")
    {
        let _ = writeln!(
            f,
            "SQLProcedures catalog_raw={catalog:?} resolved={resolved_catalog:?} metadata_id={metadata_id}"
        );
    }

    let mut filters = Vec::new();
    if let Some(filter) = build_filter("procedure_catalog", &catalog, metadata_id) {
        filters.push(filter);
    }
    if let Some(filter) = build_filter("procedure_schema", &schema, metadata_id) {
        filters.push(filter);
    }
    if let Some(filter) = build_filter("procedure_name", &procedure, metadata_id) {
        filters.push(filter);
    }

    let where_clause = if filters.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", filters.join(" AND "))
    };

    let query = format!(
        r#"
WITH base AS (
    SELECT procedure_catalog,
           procedure_schema,
           procedure_name,
           argument_signature,
           data_type,
           comment
    FROM {view}
    {where_clause}
)
SELECT
    procedure_catalog AS PROCEDURE_CAT,
    procedure_schema AS PROCEDURE_SCHEM,
    procedure_name AS PROCEDURE_NAME,
    IFF(argument_signature IS NULL OR argument_signature = '()',
        0,
        ARRAY_SIZE(
            SPLIT(
                REGEXP_REPLACE(
                    SUBSTR(argument_signature, 2, GREATEST(LEN(argument_signature) - 2, 0)),
                    '(\\d),(\\d)',
                    '\\1;\\2'
                ),
                ','
            )
        )
    ) AS NUM_INPUT_PARAMS,
    CAST(NULL AS INTEGER) AS NUM_OUTPUT_PARAMS,
    IFF(STARTSWITH(data_type, 'TABLE'), 1, 0) AS NUM_RESULT_SETS,
    COALESCE(comment, '') AS REMARKS,
    CAST(2 AS SMALLINT) AS PROCEDURE_TYPE,
    procedure_name AS SPECIFIC_NAME
FROM base
ORDER BY PROCEDURE_SCHEM, PROCEDURE_NAME
"#,
        view = procedures_view,
        where_clause = where_clause
    );

    let query_cstr = std::ffi::CString::new(query).unwrap();
    let mut exec_result = statement::exec_direct(
        statement_handle,
        query_cstr.as_ptr() as *const sql::Char,
        sql::NTS as i32,
    );
    if let Err(ref err) = exec_result {
        if resolved_catalog.is_some() && is_missing_catalog_error(err) {
            tracing::debug!(
                "procedures: catalog {:?} missing, returning empty result",
                resolved_catalog
            );
            if let Ok(mut f) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rust_debug.log")
            {
                let _ = writeln!(
                    f,
                    "SQLProcedures fallback catalog={resolved_catalog:?} error={err:?}"
                );
            }
            let empty_cstr = std::ffi::CString::new(EMPTY_PROCEDURES_RESULT).unwrap();
            let fallback_result = statement::exec_direct(
                statement_handle,
                empty_cstr.as_ptr() as *const sql::Char,
                sql::NTS as i32,
            );
            if let Err(ref fallback_err) = fallback_result {
                if let Ok(mut f) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/rust_debug.log")
                {
                    let _ = writeln!(
                        f,
                        "SQLProcedures fallback execution failed catalog={resolved_catalog:?} err={fallback_err:?}"
                    );
                }
            }
            exec_result = fallback_result;
        }
    }
    if let Err(ref err) = exec_result {
        tracing::error!("procedures: exec_direct failed: {err}");
        eprintln!("SQLProcedures query failed: {err}");
    }
    exec_result
}

/// SQLProcedureColumns - List procedure columns/parameters
pub fn procedure_columns(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    proc_name: *const sql::Char,
    proc_name_length: sql::SmallInt,
    column_name: *const sql::Char,
    column_name_length: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("procedure_columns: listing procedure columns");
    eprintln!("SQLProcedureColumns invoked");

    let stmt = stmt_from_handle(statement_handle);
    let metadata_id = stmt.metadata_id;

    let catalog = cstr_arg(catalog_name, catalog_name_length)?;
    let schema = cstr_arg(schema_name, schema_name_length)?;
    let procedure = cstr_arg(proc_name, proc_name_length)?;
    let column = cstr_arg(column_name, column_name_length)?;

    let resolved_catalog = resolve_catalog_reference(&catalog, metadata_id);
    let procedures_view = info_schema_table("procedures", resolved_catalog.as_deref());

    let mut filters = Vec::new();
    if let Some(filter) = build_filter("procedure_catalog", &catalog, metadata_id) {
        filters.push(filter);
    }
    if let Some(filter) = build_filter("procedure_schema", &schema, metadata_id) {
        filters.push(filter);
    }
    if let Some(filter) = build_filter("procedure_name", &procedure, metadata_id) {
        filters.push(filter);
    }

    let where_clause = if filters.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", filters.join(" AND "))
    };

    let column_filter = build_filter("COLUMN_NAME", &column, metadata_id)
        .map(|cond| format!("WHERE {cond}"))
        .unwrap_or_default();

    let data_type_expr = "CASE
            WHEN base_type IN ('VARCHAR', 'STRING', 'VARIANT', 'OBJECT', 'ARRAY', 'GEOGRAPHY') THEN 12
            WHEN base_type IN ('NUMBER', 'DECIMAL', 'NUMERIC') THEN 3
            WHEN base_type IN ('FLOAT', 'DOUBLE') THEN 8
            WHEN base_type = 'BOOLEAN' THEN -7
            WHEN base_type = 'BINARY' THEN -2
            WHEN base_type = 'DATE' THEN 91
            WHEN base_type = 'TIME' THEN 92
            WHEN base_type LIKE 'TIMESTAMP%' THEN 93
            ELSE 12
        END";

    let type_name_expr = "CASE
            WHEN base_type = 'OBJECT' THEN 'STRUCT'
            WHEN base_type = 'STRING' THEN 'VARCHAR'
            WHEN base_type = 'FLOAT' THEN 'DOUBLE'
            WHEN base_type IN ('NUMBER', 'NUMERIC') THEN 'DECIMAL'
            WHEN base_type LIKE 'TIMESTAMP%' THEN 'TIMESTAMP'
            ELSE base_type
        END";

    let column_size_expr = "CASE
            WHEN base_type IN ('VARCHAR', 'STRING') THEN COALESCE(precision, 16777216)
            WHEN base_type = 'BINARY' THEN COALESCE(precision, 8388608)
            WHEN base_type IN ('NUMBER', 'DECIMAL', 'NUMERIC') THEN COALESCE(precision, 38)
            WHEN base_type IN ('FLOAT', 'DOUBLE') THEN 38
            WHEN base_type = 'BOOLEAN' THEN 1
            WHEN base_type = 'DATE' THEN 10
            WHEN base_type = 'TIME' THEN 18
            WHEN base_type LIKE 'TIMESTAMP%' THEN 35
            ELSE 0
        END";

    let buffer_length_expr = "CASE
            WHEN base_type IN ('VARCHAR', 'STRING') THEN COALESCE(precision, 16777216)
            WHEN base_type = 'BINARY' THEN COALESCE(precision, 8388608)
            WHEN base_type IN ('NUMBER', 'DECIMAL', 'NUMERIC') THEN 16
            WHEN base_type IN ('FLOAT', 'DOUBLE') THEN 8
            WHEN base_type = 'BOOLEAN' THEN 1
            WHEN base_type = 'DATE' THEN 10
            WHEN base_type = 'TIME' THEN 18
            WHEN base_type LIKE 'TIMESTAMP%' THEN 35
            ELSE 0
        END";

    let decimal_digits_expr = "CASE
            WHEN base_type IN ('NUMBER', 'DECIMAL', 'NUMERIC') THEN COALESCE(scale, 0)
            WHEN base_type LIKE 'TIMESTAMP%' THEN COALESCE(scale, 9)
            ELSE 0
        END";

    let num_prec_radix_expr = "CASE
            WHEN base_type IN ('NUMBER', 'DECIMAL', 'NUMERIC', 'FLOAT', 'DOUBLE') THEN 10
            ELSE NULL
        END";

    let datetime_sub_expr = "CASE
            WHEN base_type = 'DATE' THEN 1
            WHEN base_type = 'TIME' THEN 2
            WHEN base_type LIKE 'TIMESTAMP%' THEN 3
            ELSE NULL
        END";

    let char_octet_length_expr = "CASE
            WHEN base_type IN ('VARCHAR', 'STRING') THEN COALESCE(precision, 16777216)
            WHEN base_type = 'BINARY' THEN COALESCE(precision, 8388608)
            WHEN base_type IN ('VARIANT', 'OBJECT', 'ARRAY', 'GEOGRAPHY') THEN 0
            ELSE NULL
        END";

    let query = format!(
        r#"
WITH base AS (
    SELECT procedure_catalog,
           procedure_schema,
           procedure_name,
           argument_signature,
           data_type,
           comment
    FROM {view}
    {where_clause}
),
arg_parts AS (
    SELECT
        procedure_catalog,
        procedure_schema,
        procedure_name,
        index + 1 AS ordinal_position,
        TRIM(REPLACE(value::string, ';', ',')) AS arg_text
    FROM base,
         LATERAL FLATTEN(
             INPUT => IFF(
                 argument_signature IS NULL OR argument_signature = '()',
                 ARRAY_CONSTRUCT(),
                 SPLIT(
                     REGEXP_REPLACE(
                         SUBSTR(argument_signature, 2, GREATEST(LEN(argument_signature) - 2, 0)),
                         '(\\d),(\\d)',
                         '\\1;\\2'
                     ),
                     ','
                 )
             )
         )
),
args AS (
    SELECT
        procedure_catalog,
        procedure_schema,
        procedure_name,
        ordinal_position,
        SPLIT_PART(arg_text, ' ', 1) AS column_name_raw,
        LTRIM(SUBSTR(arg_text, LEN(SPLIT_PART(arg_text, ' ', 1)) + 1)) AS type_text_raw
    FROM arg_parts
    WHERE TRIM(arg_text) <> ''
),
result_parts AS (
    SELECT
        procedure_catalog,
        procedure_schema,
        procedure_name,
        index + 1 AS ordinal_position,
        TRIM(REPLACE(value::string, ';', ',')) AS col_text
    FROM base,
         LATERAL FLATTEN(
             INPUT => IFF(
                 STARTSWITH(data_type, 'TABLE'),
                 SPLIT(
                     REGEXP_REPLACE(
                         SUBSTR(
                             data_type,
                             POSITION('(' IN data_type) + 1,
                             GREATEST(LEN(data_type) - POSITION('(' IN data_type) - 1, 0)
                         ),
                         '(\\d),(\\d)',
                         '\\1;\\2'
                     ),
                     ','
                 ),
                 ARRAY_CONSTRUCT()
             )
         )
),
result_cols AS (
    SELECT
        procedure_catalog,
        procedure_schema,
        procedure_name,
        ordinal_position,
        SPLIT_PART(col_text, ' ', 1) AS column_name_raw,
        LTRIM(SUBSTR(col_text, LEN(SPLIT_PART(col_text, ' ', 1)) + 1)) AS type_text_raw
    FROM result_parts
    WHERE TRIM(col_text) <> ''
),
return_rows AS (
    SELECT
        procedure_catalog,
        procedure_schema,
        procedure_name,
        0 AS ordinal_position,
        '' AS column_name_raw,
        data_type AS type_text_raw
    FROM base
    WHERE NOT STARTSWITH(data_type, 'TABLE')
),
unioned AS (
    SELECT *, 1 AS column_type, 0 AS resultset_flag FROM args
    UNION ALL
    SELECT *, 3 AS column_type, 1 AS resultset_flag FROM result_cols
    UNION ALL
    SELECT *, 5 AS column_type, 0 AS resultset_flag FROM return_rows
),
type_attrs AS (
    SELECT
        procedure_catalog,
        procedure_schema,
        procedure_name,
        ordinal_position,
        column_type,
        resultset_flag,
        REGEXP_REPLACE(column_name_raw, '^"|"$', '') AS column_name,
        type_text_raw,
        COALESCE(UPPER(REGEXP_SUBSTR(type_text_raw, '^[A-Z_]+')), 'VARCHAR') AS base_type,
        TRY_TO_NUMBER(REGEXP_SUBSTR(type_text_raw, '\\((\\d+)', 1, 1, 'e', 1)) AS precision,
        TRY_TO_NUMBER(REGEXP_SUBSTR(type_text_raw, '\\((\\d+),(\\d+)\\)', 1, 1, 'e', 2)) AS scale
    FROM unioned
),
final_rows AS (
    SELECT
        procedure_catalog AS PROCEDURE_CAT,
        procedure_schema AS PROCEDURE_SCHEM,
        procedure_name AS PROCEDURE_NAME,
        column_name AS COLUMN_NAME,
        column_type AS COLUMN_TYPE,
        {data_type_expr} AS DATA_TYPE,
        {type_name_expr} AS TYPE_NAME,
        {column_size_expr} AS COLUMN_SIZE,
        {buffer_length_expr} AS BUFFER_LENGTH,
        {decimal_digits_expr} AS DECIMAL_DIGITS,
        {num_prec_radix_expr} AS NUM_PREC_RADIX,
        CAST(1 AS SMALLINT) AS NULLABLE,
        CAST(NULL AS STRING) AS REMARKS,
        CAST(NULL AS STRING) AS COLUMN_DEF,
        {data_type_expr} AS SQL_DATA_TYPE,
        {datetime_sub_expr} AS SQL_DATETIME_SUB,
        {char_octet_length_expr} AS CHAR_OCTET_LENGTH,
        ordinal_position AS ORDINAL_POSITION,
        'YES' AS IS_NULLABLE,
        CASE WHEN resultset_flag = 1 THEN '1' ELSE '0' END AS IS_RESULTSET,
        CAST(0 AS INTEGER) AS USER_DATA_TYPE
    FROM type_attrs
)
SELECT *
FROM final_rows
{column_filter}
ORDER BY PROCEDURE_SCHEM,
         PROCEDURE_NAME,
         CASE COLUMN_TYPE WHEN 5 THEN 0 WHEN 3 THEN 1 ELSE 2 END,
         ORDINAL_POSITION
"#,
        view = procedures_view,
        where_clause = where_clause,
        data_type_expr = data_type_expr,
        type_name_expr = type_name_expr,
        column_size_expr = column_size_expr,
        buffer_length_expr = buffer_length_expr,
        decimal_digits_expr = decimal_digits_expr,
        num_prec_radix_expr = num_prec_radix_expr,
        datetime_sub_expr = datetime_sub_expr,
        char_octet_length_expr = char_octet_length_expr,
        column_filter = column_filter
    );

    if let Err(e) = std::fs::write(
        "/Users/snoonan/repos/universal-driver/target/procedure_columns.sql",
        &query,
    ) {
        tracing::warn!("procedure_columns: failed to write debug SQL: {e}");
    }

    let query_cstr = std::ffi::CString::new(query).unwrap();
    let mut exec_result = statement::exec_direct(
        statement_handle,
        query_cstr.as_ptr() as *const sql::Char,
        sql::NTS as i32,
    );
    if let Err(ref err) = exec_result {
        if resolved_catalog.is_some() && is_missing_catalog_error(err) {
            tracing::debug!(
                "procedure_columns: catalog {:?} missing, returning empty result",
                resolved_catalog
            );
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rust_debug.log")
            {
                let _ = writeln!(
                    f,
                    "SQLProcedureColumns fallback catalog={resolved_catalog:?} error={err:?}"
                );
            }
            let empty_cstr = std::ffi::CString::new(EMPTY_PROCEDURE_COLUMNS_RESULT).unwrap();
            let fallback_result = statement::exec_direct(
                statement_handle,
                empty_cstr.as_ptr() as *const sql::Char,
                sql::NTS as i32,
            );
            if let Err(ref fallback_err) = fallback_result {
                if let Ok(mut f) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/rust_debug.log")
                {
                    let _ = writeln!(
                        f,
                        "SQLProcedureColumns fallback execution failed catalog={resolved_catalog:?} err={fallback_err:?}"
                    );
                }
            }
            exec_result = fallback_result;
        }
    }
    if let Err(ref err) = exec_result {
        tracing::error!("procedure_columns: exec_direct failed: {err}");
        eprintln!("SQLProcedureColumns query failed: {err}");
    }
    exec_result
}
