//! Catalog functions: SQLTables, SQLColumns, SQLGetTypeInfo, and related.
//!
//! `SQLTables` runs wrapper-owned `SHOW` queries (`SHOW DATABASES` /
//! `SHOW SCHEMAS` / `SHOW OBJECTS`), then maps rows into the flat 5-column
//! ODBC result set. Pattern-mode filters use the local case-sensitive
//! [`like_match`].
//!
//! `SQLColumns` likewise runs wrapper-owned `SHOW COLUMNS`, maps rows into
//! the flat 19-column ODBC result set, and re-filters with [`like_match`].
//!
//! `SQLGetTypeInfo` is entirely static — it returns a hard-coded table of the
//! 23 Snowflake SQL types, matching the legacy driver's `InitializeData()` in
//! `SFTypeInfoMetadataSource`. No server round-trip is needed.

use crate::api::encoding::OdbcEncoding;
use crate::api::error::{
    ArrowArrayStreamReaderCreationSnafu, AsyncInProgressSnafu, CursorAlreadyOpenSnafu,
    DisconnectedSnafu, InvalidDuringDaeSnafu, NullPointerSnafu, OdbcRuntimeSnafu,
    ShowKeysColumnMissingSnafu, ShowKeysInvalidKeySeqSnafu,
};
use crate::api::runtime::global;
use crate::api::statement::{execute_show_query_collect_batch, set_state_for_catalog};
use crate::api::utils::{ApiExitLog, ESCAPE_CHAR, catalog_arg_to_pattern, escape_like_wildcards};
use crate::api::{
    ConnectionState, ExecutionOrigin, OdbcResult, StatementInner, StatementState, stmt_from_handle,
};
use crate::conversion::{
    INTEGER_CONCISE_SQL_TYPE, NumericSettings, SMALLINT_CONCISE_SQL_TYPE,
    WVARCHAR_CONCISE_SQL_TYPE, column_size_from_field, decimal_digits_from_field,
    num_prec_radix_from_field, octet_length_from_field, sql_type_from_field, type_name_from_field,
    verbose_sql_type_from_field,
};
use arrow::array::{Array, ArrayRef, Int16Array, Int32Array, Int64Array, RecordBatch, StringArray};
use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use odbc_sys as sql;
use sf_core::protobuf::generated::database_driver_v1::{
    ConnectionGetInfoRequest, ConnectionGetParameterRequest, StatementHandle,
};
use snafu::{OptionExt, ResultExt};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// One flat `SQLTables` result row, in ODBC column order:
/// (TABLE_CAT, TABLE_SCHEM, TABLE_NAME, TABLE_TYPE, REMARKS).
type FlatTableRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

// ============================================================================
// ODBC SQLTables special-case sentinel values
// ============================================================================

const SQL_ALL_CATALOGS: &str = "%";
const SQL_ALL_SCHEMAS: &str = "%";
const SQL_ALL_TABLE_TYPES: &str = "%";

// ============================================================================
// ODBC 5-column result set schema
// ============================================================================

fn catalog_text_field(name: &str, char_length: u32) -> Field {
    let metadata: HashMap<String, String> = [
        ("logicalType".to_string(), "TEXT".to_string()),
        ("charLength".to_string(), char_length.to_string()),
    ]
    .into();
    Field::new(name, DataType::Utf8, true).with_metadata(metadata)
}

/// Catalog string column labeled `SQL_WVARCHAR` (−9) to match the reference
/// driver catalog IRD. Physical storage stays Utf8; only the concise type is overridden.
fn catalog_wvarchar_field(name: &str, char_length: u32) -> Field {
    let metadata: HashMap<String, String> = [
        ("logicalType".to_string(), "TEXT".to_string()),
        ("charLength".to_string(), char_length.to_string()),
        (
            "conciseSqlType".to_string(),
            WVARCHAR_CONCISE_SQL_TYPE.to_string(),
        ),
    ]
    .into();
    Field::new(name, DataType::Utf8, true).with_metadata(metadata)
}

fn flat_tables_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        catalog_wvarchar_field("TABLE_CAT", 255),
        catalog_wvarchar_field("TABLE_SCHEM", 255),
        catalog_wvarchar_field("TABLE_NAME", 255),
        catalog_wvarchar_field("TABLE_TYPE", 255),
        catalog_wvarchar_field("REMARKS", 65535),
    ]))
}

fn flat_columns_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        catalog_wvarchar_field("TABLE_CAT", 255),    // 1
        catalog_wvarchar_field("TABLE_SCHEM", 255),  // 2
        catalog_wvarchar_field("TABLE_NAME", 255),   // 3
        catalog_wvarchar_field("COLUMN_NAME", 255),  // 4
        catalog_smallint_field("DATA_TYPE"),         // 5
        catalog_wvarchar_field("TYPE_NAME", 255),    // 6
        catalog_int_field("COLUMN_SIZE"),            // 7
        catalog_int_field("BUFFER_LENGTH"),          // 8
        catalog_smallint_field("DECIMAL_DIGITS"),    // 9
        catalog_int_field("NUM_PREC_RADIX"),         // 10 (reference driver: INTEGER)
        catalog_smallint_field("NULLABLE"),          // 11
        catalog_wvarchar_field("REMARKS", 65535),    // 12
        catalog_wvarchar_field("COLUMN_DEF", 65535), // 13
        catalog_smallint_field("SQL_DATA_TYPE"),     // 14
        catalog_smallint_field("SQL_DATETIME_SUB"),  // 15
        catalog_int_field("CHAR_OCTET_LENGTH"),      // 16
        catalog_int_field("ORDINAL_POSITION"),       // 17
        catalog_wvarchar_field("IS_NULLABLE", 3),    // 18
        catalog_smallint_field("USER_DATA_TYPE"),    // 19
    ]))
}

// ============================================================================
// Entry point
// ============================================================================

/// Read an optional ODBC string arg (returns None when pointer is null, Some(str) otherwise).
fn read_opt_str<E: OdbcEncoding>(
    ptr: *const E::Char,
    length: sql::SmallInt,
) -> OdbcResult<Option<String>> {
    if ptr.is_null() {
        return Ok(None);
    }
    if length == 0 {
        return Ok(Some(String::new()));
    }
    Ok(Some(E::read_string(ptr, length as sql::Integer)?))
}

// Mirrors the ODBC SQLTables C entry point one-to-one, so the argument count is
// fixed by the spec rather than something we can reduce.
#[allow(clippy::too_many_arguments)]
pub fn tables<E: OdbcEncoding>(
    statement_handle: sql::Handle,
    catalog_name: *const E::Char,
    name_length1: sql::SmallInt,
    schema_name: *const E::Char,
    name_length2: sql::SmallInt,
    table_name: *const E::Char,
    name_length3: sql::SmallInt,
    table_type: *const E::Char,
    name_length4: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("SQLTables called");

    let catalog_raw = read_opt_str::<E>(catalog_name, name_length1)?;
    let schema_raw = read_opt_str::<E>(schema_name, name_length2)?;
    let table_raw = read_opt_str::<E>(table_name, name_length3)?;
    let type_raw = read_opt_str::<E>(table_type, name_length4)?;

    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let conn = dbc.connection.lock();
    let mut inner = guard.inner.lock();

    validate_catalog_stmt_ready(&inner)?;

    let conn_handle = match &conn.state {
        ConnectionState::Connected { conn_handle, .. } => *conn_handle,
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
    };

    let metadata_id = inner.metadata_id;
    let stmt_handle = guard.stmt_handle;
    drop(conn);

    let is_empty_str = |s: &Option<String>| s.as_deref() == Some("");

    // SQL_ALL_CATALOGS special case: catalog="%", schema="", table="", type=""
    if catalog_raw.as_deref() == Some(SQL_ALL_CATALOGS)
        && is_empty_str(&schema_raw)
        && is_empty_str(&table_raw)
        && is_empty_str(&type_raw)
    {
        return execute_show_all_catalogs(&mut inner, stmt_handle);
    }

    // SQL_ALL_SCHEMAS special case: schema="%", catalog="", table="", type=""
    if schema_raw.as_deref() == Some(SQL_ALL_SCHEMAS)
        && is_empty_str(&catalog_raw)
        && is_empty_str(&table_raw)
        && is_empty_str(&type_raw)
    {
        return execute_show_all_schemas(&mut inner, stmt_handle);
    }

    // SQL_ALL_TABLE_TYPES special case: type="%", catalog="", schema="", table=""
    if type_raw.as_deref() == Some(SQL_ALL_TABLE_TYPES)
        && is_empty_str(&catalog_raw)
        && is_empty_str(&schema_raw)
        && is_empty_str(&table_raw)
    {
        return set_static_table_types(&mut inner);
    }

    // Normal mode: apply SQL_ATTR_METADATA_ID
    //
    // Substitute a NULL catalog with the connection's current database (see
    // resolve_null_catalog_to_connection_context). A NULL schema is deliberately
    // left NULL so it matches every schema in that database, unless
    // CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX fills it below. Identifier mode
    // (metadata_id=TRUE) still requires non-NULL args → HY009.
    let catalog_raw = if metadata_id {
        catalog_raw
    } else {
        resolve_null_catalog_to_connection_context(catalog_raw, conn_handle)?
    };
    let mut catalog_pattern = catalog_arg_to_pattern(catalog_raw.as_deref(), metadata_id)?;
    let mut schema_pattern = catalog_arg_to_pattern(schema_raw.as_deref(), metadata_id)?;
    let table_pattern = catalog_arg_to_pattern(table_raw.as_deref(), metadata_id)?;

    // TableType is always a value list, not a pattern
    let table_types = parse_table_type_list(type_raw.as_deref());

    // Core GetObjects applied CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX after
    // the ODBC NULL-catalog substitution. Do not gate solely on schema — a
    // supplied schema must not skip catalog fill when catalog is still unresolved.
    fill_null_catalog_schema_from_connection_ctx(
        &mut catalog_pattern,
        &mut schema_pattern,
        conn_handle,
    )?;

    execute_show_tables(
        &mut inner,
        stmt_handle,
        catalog_pattern,
        schema_pattern,
        table_pattern,
        table_types,
    )
}

/// Substitute a NULL catalog with the connection's current database.
///
/// ODBC NULL means "use connection context" for catalog functions; an
/// account-wide SHOW omits databases the role can't see. This mirrors the
/// legacy driver's `SFSemantics::GetFilterForNullCatalog` (gated by its
/// `UseCurrentCatalog` config): it substitutes the **catalog only**. A NULL
/// schema is left NULL so it matches all schemas in the database — unless
/// `CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX` fills it in `tables()`.
fn resolve_null_catalog_to_connection_context(
    catalog_raw: Option<String>,
    conn_handle: sf_core::protobuf::generated::database_driver_v1::ConnectionHandle,
) -> OdbcResult<Option<String>> {
    if catalog_raw.is_some() {
        return Ok(catalog_raw);
    }

    let rt = global().context(OdbcRuntimeSnafu)?;
    let info = rt.block_on(async |c| {
        c.connection_get_info(ConnectionGetInfoRequest {
            conn_handle: Some(conn_handle),
            info_codes: vec![],
            include_master_token: false,
        })
        .await
    })?;

    // The server name is a literal identifier, not a user pattern; escape LIKE
    // metacharacters so is_exact_pattern() recovers it and issues IN DATABASE
    // rather than falling through to IN ACCOUNT when the name contains _ or %.
    Ok(info.database.map(|db| escape_like_wildcards(&db)))
}

// Mirrors the ODBC SQLColumns C entry point one-to-one.
#[allow(clippy::too_many_arguments)]
pub fn columns<E: OdbcEncoding>(
    statement_handle: sql::Handle,
    catalog_name: *const E::Char,
    name_length1: sql::SmallInt,
    schema_name: *const E::Char,
    name_length2: sql::SmallInt,
    table_name: *const E::Char,
    name_length3: sql::SmallInt,
    column_name: *const E::Char,
    name_length4: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("SQLColumns called");

    let catalog_raw = read_opt_str::<E>(catalog_name, name_length1)?;
    let schema_raw = read_opt_str::<E>(schema_name, name_length2)?;
    let table_raw = read_opt_str::<E>(table_name, name_length3)?;
    let column_raw = read_opt_str::<E>(column_name, name_length4)?;

    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let conn = dbc.connection.lock();
    let mut inner = guard.inner.lock();

    validate_catalog_stmt_ready(&inner)?;

    let conn_handle = match &conn.state {
        ConnectionState::Connected { conn_handle, .. } => *conn_handle,
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
    };

    let metadata_id = inner.metadata_id;
    let numeric_settings = conn.numeric_settings;
    let stmt_handle = guard.stmt_handle;
    drop(conn);

    // Substitute a NULL catalog with the connection's current database (see
    // resolve_null_catalog_to_connection_context). A NULL schema is deliberately
    // left NULL so it matches every schema in that database, unless
    // CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX fills it below. Identifier mode
    // (metadata_id=TRUE) still requires non-NULL args → HY009.
    let catalog_raw = if metadata_id {
        catalog_raw
    } else {
        resolve_null_catalog_to_connection_context(catalog_raw, conn_handle)?
    };
    let mut catalog_pattern = catalog_arg_to_pattern(catalog_raw.as_deref(), metadata_id)?;
    let mut schema_pattern = catalog_arg_to_pattern(schema_raw.as_deref(), metadata_id)?;
    let table_pattern = catalog_arg_to_pattern(table_raw.as_deref(), metadata_id)?;
    let column_pattern = catalog_arg_to_pattern(column_raw.as_deref(), metadata_id)?;

    // Core GetObjects applied CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX after
    // the ODBC NULL-catalog substitution (same as SQLTables).
    fill_null_catalog_schema_from_connection_ctx(
        &mut catalog_pattern,
        &mut schema_pattern,
        conn_handle,
    )?;

    execute_show_columns(
        &mut inner,
        stmt_handle,
        numeric_settings,
        catalog_pattern,
        schema_pattern,
        table_pattern,
        column_pattern,
    )
}

// ============================================================================
// SQLPrimaryKeys — SHOW PRIMARY KEYS
// ============================================================================

fn primary_keys_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        catalog_text_field("TABLE_CAT", 255),
        catalog_text_field("TABLE_SCHEM", 255),
        catalog_text_field("TABLE_NAME", 255),
        catalog_text_field("COLUMN_NAME", 255),
        catalog_key_seq_field("KEY_SEQ"),
        catalog_text_field("PK_NAME", 255),
    ]))
}

fn catalog_key_seq_field(name: &str) -> Field {
    let metadata: HashMap<String, String> = [
        ("logicalType".to_string(), "FIXED".to_string()),
        ("scale".to_string(), "0".to_string()),
        // precision = 5 decimal digits (the width of a SMALLINT), unrelated to
        // the SQL type code below.
        ("precision".to_string(), "5".to_string()),
        (
            "conciseSqlType".to_string(),
            SMALLINT_CONCISE_SQL_TYPE.to_string(),
        ),
    ]
    .into();
    Field::new(name, DataType::Int16, false).with_metadata(metadata)
}

fn catalog_nullable_smallint_field(name: &str) -> Field {
    let metadata: HashMap<String, String> = [
        ("logicalType".to_string(), "FIXED".to_string()),
        ("scale".to_string(), "0".to_string()),
        ("precision".to_string(), "5".to_string()),
        ("conciseSqlType".to_string(), "5".to_string()),
    ]
    .into();
    Field::new(name, DataType::Int16, true).with_metadata(metadata)
}

fn escape_snowflake_identifier(ident: &str) -> String {
    ident.replace('"', "\"\"")
}

/// Builds the `IN <scope>` clause for a `SHOW ... KEYS` query, picking the
/// narrowest object the caller resolved to.
///
/// `resolve_show_identifiers` runs first and fills a missing catalog/schema from
/// the connection context, so a `None` catalog here means the identifier is
/// *genuinely unresolved* (e.g. the connection has no current database). In that
/// case we deliberately widen to `account` scope and rely on the client-side
/// re-filter ([`ShowKeyScopeFilter`] / `ShowForeignKeyFilter`) to narrow the
/// result set — Snowflake `SHOW ... KEYS` supports only a single `IN` object and
/// has no `LIKE`, so there is no narrower server-side option.
fn build_show_in_scope(catalog: Option<&str>, schema: Option<&str>, table: Option<&str>) -> String {
    let catalog = catalog.filter(|s| !s.is_empty());
    let schema = schema.filter(|s| !s.is_empty());
    let table = table.filter(|s| !s.is_empty());

    match (catalog, schema, table) {
        (None, _, _) => "account".to_string(),
        (Some(db), None, _) => {
            format!("database \"{}\"", escape_snowflake_identifier(db))
        }
        (Some(db), Some(sch), None) => {
            format!(
                "schema \"{}\".\"{}\"",
                escape_snowflake_identifier(db),
                escape_snowflake_identifier(sch),
            )
        }
        (Some(db), Some(sch), Some(tbl)) => {
            format!(
                "table \"{}\".\"{}\".\"{}\"",
                escape_snowflake_identifier(db),
                escape_snowflake_identifier(sch),
                escape_snowflake_identifier(tbl),
            )
        }
    }
}

fn metadata_request_use_connection_ctx(
    conn_handle: sf_core::protobuf::generated::database_driver_v1::ConnectionHandle,
) -> OdbcResult<bool> {
    let rt = global().context(OdbcRuntimeSnafu)?;
    let resp = rt.block_on(async |c| {
        c.connection_get_parameter(ConnectionGetParameterRequest {
            conn_handle: Some(conn_handle),
            key: "CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX".to_string(),
        })
        .await
    });
    Ok(resp
        .ok()
        .and_then(|r| r.value)
        .is_some_and(|v| v.eq_ignore_ascii_case("true")))
}

/// Fill still-NULL catalog/schema patterns from the session when
/// `CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX` is enabled.
///
/// Shared by [`tables`] and [`columns`] so both stay aligned with core
/// GetObjects' `apply_connection_context` behavior after the ODBC NULL-catalog
/// substitution.
fn fill_null_catalog_schema_from_connection_ctx(
    catalog_pattern: &mut Option<String>,
    schema_pattern: &mut Option<String>,
    conn_handle: sf_core::protobuf::generated::database_driver_v1::ConnectionHandle,
) -> OdbcResult<()> {
    if catalog_pattern.is_some() && schema_pattern.is_some() {
        return Ok(());
    }
    if !metadata_request_use_connection_ctx(conn_handle)? {
        return Ok(());
    }

    let rt = global().context(OdbcRuntimeSnafu)?;
    let info = rt.block_on(async |c| {
        c.connection_get_info(ConnectionGetInfoRequest {
            conn_handle: Some(conn_handle),
            info_codes: vec![],
            include_master_token: false,
        })
        .await
    })?;
    if catalog_pattern.is_none()
        && let Some(db) = info.database
    {
        *catalog_pattern = Some(escape_like_wildcards(&db));
    }
    if schema_pattern.is_none()
        && let Some(sch) = info.schema
    {
        *schema_pattern = Some(escape_like_wildcards(&sch));
    }
    Ok(())
}

fn resolve_show_identifiers(
    catalog: Option<String>,
    schema: Option<String>,
    table: Option<&str>,
    conn_handle: sf_core::protobuf::generated::database_driver_v1::ConnectionHandle,
) -> OdbcResult<(Option<String>, Option<String>)> {
    if catalog.is_some() && schema.is_some() {
        return Ok((catalog, schema));
    }

    let table_specified = table.is_some_and(|t| !t.is_empty());
    let use_ctx = metadata_request_use_connection_ctx(conn_handle)?;
    let needs_info = catalog.is_none() || (schema.is_none() && (use_ctx || table_specified));
    if !needs_info {
        return Ok((catalog, schema));
    }

    let rt = global().context(OdbcRuntimeSnafu)?;
    let info = rt.block_on(async |c| {
        c.connection_get_info(ConnectionGetInfoRequest {
            conn_handle: Some(conn_handle),
            info_codes: vec![],
            include_master_token: false,
        })
        .await
    })?;

    let catalog = catalog.or(info.database);
    let schema = if use_ctx || table_specified {
        schema.or(info.schema)
    } else {
        schema
    };
    Ok((catalog, schema))
}

fn is_object_not_found_sql_state(state: &str) -> bool {
    matches!(state, "42000" | "42S02")
}

fn column_index_by_name(schema: &Schema, name: &str) -> Option<usize> {
    schema
        .fields()
        .iter()
        .position(|f| f.name().eq_ignore_ascii_case(name))
}

/// Resolves a required column in a `SHOW ... KEYS` result, failing with a typed
/// error that names both the command and the absent column when the server
/// response does not carry the layout the catalog mapping expects.
fn show_keys_column_index(
    schema: &Schema,
    command: &'static str,
    column: &'static str,
) -> OdbcResult<usize> {
    column_index_by_name(schema, column).context(ShowKeysColumnMissingSnafu { command, column })
}

fn utf8_value_at(batch: &RecordBatch, col: usize, row: usize) -> Option<String> {
    let array = batch.column(col);
    if array.is_null(row) {
        return None;
    }
    match array.data_type() {
        DataType::Utf8 => {
            let arr = array.as_any().downcast_ref::<StringArray>()?;
            Some(arr.value(row).to_string())
        }
        DataType::LargeUtf8 => {
            let arr = array
                .as_any()
                .downcast_ref::<arrow::array::LargeStringArray>()?;
            Some(arr.value(row).to_string())
        }
        _ => None,
    }
}

fn key_seq_value_at(batch: &RecordBatch, col: usize, row: usize) -> OdbcResult<i16> {
    let array = batch.column(col);
    if array.is_null(row) {
        return ShowKeysInvalidKeySeqSnafu.fail();
    }
    let value = match array.data_type() {
        DataType::Utf8 | DataType::LargeUtf8 => {
            utf8_value_at(batch, col, row).and_then(|s| s.parse::<i16>().ok())
        }
        DataType::Int16 => array
            .as_any()
            .downcast_ref::<Int16Array>()
            .map(|a| a.value(row)),
        DataType::Int32 => array
            .as_any()
            .downcast_ref::<Int32Array>()
            .and_then(|a| i16::try_from(a.value(row)).ok()),
        DataType::Int64 => array
            .as_any()
            .downcast_ref::<Int64Array>()
            .and_then(|a| i16::try_from(a.value(row)).ok()),
        _ => None,
    };
    value.context(ShowKeysInvalidKeySeqSnafu)
}

/// One flat `SQLPrimaryKeys` result row, in ODBC column order.
type PrimaryKeyRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    i16,
    Option<String>,
);

/// Folds a catalog identifier argument to its canonical Snowflake form for
/// `SQL_ATTR_METADATA_ID = TRUE` (identifier) mode, mirroring
/// [`catalog_arg_to_pattern`](crate::api::utils::catalog_arg_to_pattern):
///
/// - A quoted `"..."` identifier is case-sensitive: strip the surrounding
///   quotes and collapse `""` → `"`.
/// - An unquoted identifier is folded with `to_uppercase()` (Snowflake stores
///   unquoted identifiers uppercase). `to_uppercase()` — not
///   `to_ascii_uppercase()` — matches the rest of the driver; this is correct
///   for Snowflake because unquoted identifiers are ASCII-only.
///
/// Folding upfront (before `build_show_in_scope` and the client-side re-filter)
/// makes both the `SHOW ... IN <scope>` query and the row filter case-correct:
/// the scope object name and the compared identifiers all use the canonical
/// form the server echoes.
fn fold_identifier(s: &str) -> String {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s[1..s.len() - 1].replace("\"\"", "\"")
    } else {
        s.trim_end().to_uppercase()
    }
}

/// Re-applies requested identifiers as filters, since `build_show_in_scope` widens the `SHOW` scope when a catalog/schema can't be resolved.
///
/// Comparison is exact: in identifier mode the caller folds the requested
/// identifiers with [`fold_identifier`] before constructing this filter, and in
/// pattern mode ordinary arguments are case-sensitive per the ODBC spec.
struct ShowKeyScopeFilter<'a> {
    catalog: Option<&'a str>,
    schema: Option<&'a str>,
    table: Option<&'a str>,
}

impl ShowKeyScopeFilter<'_> {
    fn matches(
        &self,
        row_catalog: Option<&str>,
        row_schema: Option<&str>,
        row_table: Option<&str>,
    ) -> bool {
        field_matches(self.catalog, row_catalog)
            && field_matches(self.schema, row_schema)
            && field_matches(self.table, row_table)
    }
}

/// Client-side filter comparison for a single catalog identifier. `None` or an
/// empty string means "no filter" — mirroring `build_show_in_scope`, which also
/// treats an empty identifier as absent — so the scope and the re-filter agree.
fn field_matches(want: Option<&str>, got: Option<&str>) -> bool {
    match want {
        Some(want) if !want.is_empty() => got == Some(want),
        _ => true,
    }
}

fn map_show_primary_keys_to_odbc(
    batch: RecordBatch,
    filter: &ShowKeyScopeFilter<'_>,
) -> OdbcResult<RecordBatch> {
    let schema = primary_keys_schema();
    if batch.num_rows() == 0 {
        return Ok(RecordBatch::new_empty(schema));
    }

    let input = batch.schema();
    const SHOW_PRIMARY_KEYS: &str = "SHOW PRIMARY KEYS";
    let idx_db = show_keys_column_index(&input, SHOW_PRIMARY_KEYS, "database_name")?;
    let idx_schema = show_keys_column_index(&input, SHOW_PRIMARY_KEYS, "schema_name")?;
    let idx_table = show_keys_column_index(&input, SHOW_PRIMARY_KEYS, "table_name")?;
    let idx_column = show_keys_column_index(&input, SHOW_PRIMARY_KEYS, "column_name")?;
    let idx_key_seq = show_keys_column_index(&input, SHOW_PRIMARY_KEYS, "key_sequence")?;
    let idx_pk_name = show_keys_column_index(&input, SHOW_PRIMARY_KEYS, "constraint_name")?;

    let mut rows: Vec<PrimaryKeyRow> = Vec::with_capacity(batch.num_rows());
    for row in 0..batch.num_rows() {
        let table_cat = utf8_value_at(&batch, idx_db, row);
        let table_schem = utf8_value_at(&batch, idx_schema, row);
        let table_name = utf8_value_at(&batch, idx_table, row);
        if !filter.matches(
            table_cat.as_deref(),
            table_schem.as_deref(),
            table_name.as_deref(),
        ) {
            continue;
        }
        let column_name = utf8_value_at(&batch, idx_column, row);
        let key_seq = key_seq_value_at(&batch, idx_key_seq, row)?;
        let pk_name = utf8_value_at(&batch, idx_pk_name, row);
        rows.push((
            table_cat,
            table_schem,
            table_name,
            column_name,
            key_seq,
            pk_name,
        ));
    }

    // Preserve server SHOW order; the reference driver does not sort client-side.
    let table_cats: Vec<Option<&str>> = rows.iter().map(|r| r.0.as_deref()).collect();
    let table_schems: Vec<Option<&str>> = rows.iter().map(|r| r.1.as_deref()).collect();
    let table_names: Vec<Option<&str>> = rows.iter().map(|r| r.2.as_deref()).collect();
    let column_names: Vec<Option<&str>> = rows.iter().map(|r| r.3.as_deref()).collect();
    let key_seqs: Vec<Option<i16>> = rows.iter().map(|r| Some(r.4)).collect();
    let pk_names: Vec<Option<&str>> = rows.iter().map(|r| r.5.as_deref()).collect();

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(table_cats)) as ArrayRef,
            Arc::new(StringArray::from(table_schems)) as ArrayRef,
            Arc::new(StringArray::from(table_names)) as ArrayRef,
            Arc::new(StringArray::from(column_names)) as ArrayRef,
            Arc::new(Int16Array::from(key_seqs)) as ArrayRef,
            Arc::new(StringArray::from(pk_names)) as ArrayRef,
        ],
    )
    .context(crate::api::error::RecordBatchBuildSnafu)
}

/// Implements `SQLPrimaryKeys`: returns primary-key column metadata for a table.
#[allow(clippy::too_many_arguments)]
pub fn primary_keys<E: OdbcEncoding>(
    statement_handle: sql::Handle,
    catalog_name: *const E::Char,
    name_length1: sql::SmallInt,
    schema_name: *const E::Char,
    name_length2: sql::SmallInt,
    table_name: *const E::Char,
    name_length3: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::info!("SQLPrimaryKeys: entry");
    let _exit = ApiExitLog("SQLPrimaryKeys");

    let catalog_raw = read_opt_str::<E>(catalog_name, name_length1)?;
    let schema_raw = read_opt_str::<E>(schema_name, name_length2)?;
    let table_raw = read_opt_str::<E>(table_name, name_length3)?;
    if table_raw.is_none() {
        return NullPointerSnafu.fail();
    }

    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let conn = dbc.connection.lock();
    let mut inner = guard.inner.lock();

    validate_catalog_stmt_ready(&inner)?;

    let conn_handle = match &conn.state {
        ConnectionState::Connected { conn_handle, .. } => *conn_handle,
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
    };
    let metadata_id = inner.metadata_id;
    let stmt_handle = guard.stmt_handle;
    drop(conn);

    if metadata_id {
        if catalog_raw.is_none() {
            return NullPointerSnafu.fail();
        }
        if schema_raw.is_none() {
            return NullPointerSnafu.fail();
        }
        // A zero-length catalog/schema is treated as *absent* for SHOW scope
        // (`build_show_in_scope` filters empty strings out, widening to `account`
        // when both are empty — same as the legacy `PrimaryKeysMetadataSource`).
        // The client-side re-filter still receives `Some("")`, so no row matches
        // an empty catalog/schema name and the caller gets an empty result set
        // (SQL_SUCCESS, no rows) — the ODBC "tables without a catalog" case,
        // which does not exist in Snowflake. We do not reject it with HY090;
        // NULL (HY009) is handled above.
    }

    // In identifier mode, fold each argument to its canonical Snowflake form so
    // both the SHOW scope and the client-side re-filter are case-correct.
    let (catalog_raw, schema_raw, table_raw) = if metadata_id {
        (
            catalog_raw.map(|s| fold_identifier(&s)),
            schema_raw.map(|s| fold_identifier(&s)),
            table_raw.map(|s| fold_identifier(&s)),
        )
    } else {
        (catalog_raw, schema_raw, table_raw)
    };

    let (catalog_raw, schema_raw) =
        resolve_show_identifiers(catalog_raw, schema_raw, table_raw.as_deref(), conn_handle)?;

    let scope = build_show_in_scope(
        catalog_raw.as_deref(),
        schema_raw.as_deref(),
        table_raw.as_deref(),
    );
    let sql = format!("SHOW PRIMARY KEYS IN {scope}");

    let show_batch = match execute_show_query_collect_batch(stmt_handle, &sql) {
        Ok(batch) => batch,
        Err(e) => {
            if e.server_sql_state()
                .is_some_and(is_object_not_found_sql_state)
            {
                return set_static_empty_catalog_result(&mut inner, primary_keys_schema());
            }
            return Err(e);
        }
    };

    let filter = ShowKeyScopeFilter {
        catalog: catalog_raw.as_deref(),
        schema: schema_raw.as_deref(),
        table: table_raw.as_deref(),
    };
    let flat_batch = map_show_primary_keys_to_odbc(show_batch, &filter)?;
    let schema = flat_batch.schema();
    let reader = reader_from_record_batch(flat_batch, schema)?;
    set_state_for_catalog(
        &mut inner,
        StatementState::QueryExecuted {
            reader,
            rows_affected: Some(-1),
            origin: ExecutionOrigin::Direct,
        },
    );
    Ok(())
}

// ============================================================================
// SQLForeignKeys — SHOW IMPORTED / EXPORTED KEYS
// ============================================================================

// ODBC referential rule / deferrability constants (sqlext.h).
const SQL_CASCADE: i16 = 0;
const SQL_RESTRICT: i16 = 1;
const SQL_SET_NULL: i16 = 2;
const SQL_NO_ACTION: i16 = 3;
const SQL_SET_DEFAULT: i16 = 4;
const SQL_INITIALLY_DEFERRED: i16 = 5;
const SQL_INITIALLY_IMMEDIATE: i16 = 6;
const SQL_NOT_DEFERRABLE: i16 = 7;

fn foreign_keys_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        catalog_text_field("PKTABLE_CAT", 255),
        catalog_text_field("PKTABLE_SCHEM", 255),
        catalog_text_field("PKTABLE_NAME", 255),
        catalog_text_field("PKCOLUMN_NAME", 255),
        catalog_text_field("FKTABLE_CAT", 255),
        catalog_text_field("FKTABLE_SCHEM", 255),
        catalog_text_field("FKTABLE_NAME", 255),
        catalog_text_field("FKCOLUMN_NAME", 255),
        catalog_key_seq_field("KEY_SEQ"),
        catalog_nullable_smallint_field("UPDATE_RULE"),
        catalog_nullable_smallint_field("DELETE_RULE"),
        catalog_text_field("FK_NAME", 255),
        catalog_text_field("PK_NAME", 255),
        catalog_nullable_smallint_field("DEFERRABILITY"),
    ]))
}

/// Which `SHOW ... KEYS` direction to run, and hence which side scopes the query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShowKeySide {
    /// `SHOW EXPORTED KEYS` — scoped to the primary-key (referenced) side.
    Exported,
    /// `SHOW IMPORTED KEYS` — scoped to the foreign-key (referencing) side.
    Imported,
}

fn build_show_foreign_keys_command(
    pk_catalog: Option<&str>,
    pk_schema: Option<&str>,
    pk_table: Option<&str>,
    fk_catalog: Option<&str>,
    fk_schema: Option<&str>,
    fk_table: Option<&str>,
) -> String {
    let pk_has_table = pk_table.is_some_and(|s| !s.is_empty());
    let fk_has_table = fk_table.is_some_and(|s| !s.is_empty());
    // Side selection by which side carries a table name. The `SQLForeignKeys`
    // entry point already fails both-tables-NULL with HY009, so at least one is
    // present here.
    //
    // This is equivalent to the reference driver's bitmask rule
    // (`SFForeignKeysMetadataSource`: granularity table|schema|catalog = 1|2|4,
    // `SHOW EXPORTED` when `primary >= foreign`). After `resolve_show_identifiers`
    // a table-bearing side is always granularity 7 and any non-table side is at
    // most 6, so `primary >= foreign` picks exactly the side that has a table —
    // and the both-tables tie resolves to EXPORTED/PK, same as this match.
    debug_assert!(
        pk_has_table || fk_has_table,
        "SQLForeignKeys entry rejects both-tables-NULL with HY009"
    );
    let side = match (pk_has_table, fk_has_table) {
        (true, _) => ShowKeySide::Exported,
        (false, true) => ShowKeySide::Imported,
        // Unreachable in practice (see debug_assert); fall back to an
        // account-scoped EXPORTED scan rather than panicking on malformed input.
        (false, false) => ShowKeySide::Exported,
    };

    let (kind, catalog, schema, table) = match side {
        ShowKeySide::Exported => ("EXPORTED", pk_catalog, pk_schema, pk_table),
        ShowKeySide::Imported => ("IMPORTED", fk_catalog, fk_schema, fk_table),
    };
    // A `None`/unresolved catalog on the selected side widens the scope to
    // `account` inside `build_show_in_scope`; the client-side re-filter then
    // narrows the result back to the requested identifiers.
    format!(
        "SHOW {kind} KEYS IN {}",
        build_show_in_scope(catalog, schema, table)
    )
}

fn map_fk_update_delete_rule(rule: Option<&str>) -> Option<i16> {
    // Snowflake returns these keywords uppercase, but match case-insensitively so
    // a casing change in the server response can't silently drop the mapping.
    match rule?.to_ascii_uppercase().as_str() {
        "CASCADE" => Some(SQL_CASCADE),
        "NO ACTION" => Some(SQL_NO_ACTION),
        "SET NULL" => Some(SQL_SET_NULL),
        "SET DEFAULT" => Some(SQL_SET_DEFAULT),
        "RESTRICT" => Some(SQL_RESTRICT),
        _ => None,
    }
}

fn map_fk_deferrability(value: Option<&str>) -> Option<i16> {
    match value?.to_ascii_uppercase().as_str() {
        "INITIALLY DEFERRED" => Some(SQL_INITIALLY_DEFERRED),
        "INITIALLY IMMEDIATE" => Some(SQL_INITIALLY_IMMEDIATE),
        "NOT DEFERRABLE" => Some(SQL_NOT_DEFERRABLE),
        _ => None,
    }
}

/// One flat `SQLForeignKeys` result row, in ODBC column order.
struct ForeignKeyRow {
    pk_table_cat: Option<String>,
    pk_table_schem: Option<String>,
    pk_table_name: Option<String>,
    pk_column_name: Option<String>,
    fk_table_cat: Option<String>,
    fk_table_schem: Option<String>,
    fk_table_name: Option<String>,
    fk_column_name: Option<String>,
    key_seq: i16,
    update_rule: Option<i16>,
    delete_rule: Option<i16>,
    fk_name: Option<String>,
    pk_name: Option<String>,
    deferrability: Option<i16>,
}

/// Re-applies requested PK/FK identifiers as filters when the `SHOW` scope was widened.
struct ShowForeignKeyFilter<'a> {
    pk_catalog: Option<&'a str>,
    pk_schema: Option<&'a str>,
    pk_table: Option<&'a str>,
    fk_catalog: Option<&'a str>,
    fk_schema: Option<&'a str>,
    fk_table: Option<&'a str>,
}

impl ShowForeignKeyFilter<'_> {
    fn matches(
        &self,
        pk_catalog: Option<&str>,
        pk_schema: Option<&str>,
        pk_table: Option<&str>,
        fk_catalog: Option<&str>,
        fk_schema: Option<&str>,
        fk_table: Option<&str>,
    ) -> bool {
        field_matches(self.pk_catalog, pk_catalog)
            && field_matches(self.pk_schema, pk_schema)
            && field_matches(self.pk_table, pk_table)
            && field_matches(self.fk_catalog, fk_catalog)
            && field_matches(self.fk_schema, fk_schema)
            && field_matches(self.fk_table, fk_table)
    }
}

fn map_show_foreign_keys_to_odbc(
    batch: RecordBatch,
    filter: &ShowForeignKeyFilter<'_>,
) -> OdbcResult<RecordBatch> {
    let schema = foreign_keys_schema();
    if batch.num_rows() == 0 {
        return Ok(RecordBatch::new_empty(schema));
    }

    let input = batch.schema();
    let idx_pk_db = column_index_by_name(&input, "pk_database_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW KEYS: missing pk_database_name column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let idx_pk_schema = column_index_by_name(&input, "pk_schema_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW KEYS: missing pk_schema_name column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let idx_pk_table = column_index_by_name(&input, "pk_table_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW KEYS: missing pk_table_name column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let idx_pk_column = column_index_by_name(&input, "pk_column_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW KEYS: missing pk_column_name column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let idx_fk_db = column_index_by_name(&input, "fk_database_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW KEYS: missing fk_database_name column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let idx_fk_schema = column_index_by_name(&input, "fk_schema_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW KEYS: missing fk_schema_name column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let idx_fk_table = column_index_by_name(&input, "fk_table_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW KEYS: missing fk_table_name column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let idx_fk_column = column_index_by_name(&input, "fk_column_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW KEYS: missing fk_column_name column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let idx_key_seq = column_index_by_name(&input, "key_sequence").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW KEYS: missing key_sequence column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let idx_update_rule = column_index_by_name(&input, "update_rule").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW KEYS: missing update_rule column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let idx_delete_rule = column_index_by_name(&input, "delete_rule").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW KEYS: missing delete_rule column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let idx_fk_name = column_index_by_name(&input, "fk_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW KEYS: missing fk_name column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let idx_pk_name = column_index_by_name(&input, "pk_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW KEYS: missing pk_name column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let idx_deferrability = column_index_by_name(&input, "deferrability").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW KEYS: missing deferrability column".to_string(),
            location: snafu::location!(),
        }
    })?;

    let mut rows: Vec<ForeignKeyRow> = Vec::with_capacity(batch.num_rows());
    for row in 0..batch.num_rows() {
        let pk_table_cat = utf8_value_at(&batch, idx_pk_db, row);
        let pk_table_schem = utf8_value_at(&batch, idx_pk_schema, row);
        let pk_table_name = utf8_value_at(&batch, idx_pk_table, row);
        let fk_table_cat = utf8_value_at(&batch, idx_fk_db, row);
        let fk_table_schem = utf8_value_at(&batch, idx_fk_schema, row);
        let fk_table_name = utf8_value_at(&batch, idx_fk_table, row);
        if !filter.matches(
            pk_table_cat.as_deref(),
            pk_table_schem.as_deref(),
            pk_table_name.as_deref(),
            fk_table_cat.as_deref(),
            fk_table_schem.as_deref(),
            fk_table_name.as_deref(),
        ) {
            continue;
        }

        let pk_column_name = utf8_value_at(&batch, idx_pk_column, row);
        let fk_column_name = utf8_value_at(&batch, idx_fk_column, row);
        let key_seq = key_seq_value_at(&batch, idx_key_seq, row)?;
        let update_rule =
            map_fk_update_delete_rule(utf8_value_at(&batch, idx_update_rule, row).as_deref());
        let delete_rule =
            map_fk_update_delete_rule(utf8_value_at(&batch, idx_delete_rule, row).as_deref());
        let fk_name = utf8_value_at(&batch, idx_fk_name, row);
        let pk_name = utf8_value_at(&batch, idx_pk_name, row);
        let deferrability =
            map_fk_deferrability(utf8_value_at(&batch, idx_deferrability, row).as_deref());
        rows.push(ForeignKeyRow {
            pk_table_cat,
            pk_table_schem,
            pk_table_name,
            pk_column_name,
            fk_table_cat,
            fk_table_schem,
            fk_table_name,
            fk_column_name,
            key_seq,
            update_rule,
            delete_rule,
            fk_name,
            pk_name,
            deferrability,
        });
    }

    // Preserve server SHOW order; the reference driver does not sort client-side.
    let pk_table_cats: Vec<Option<&str>> = rows.iter().map(|r| r.pk_table_cat.as_deref()).collect();
    let pk_table_schems: Vec<Option<&str>> =
        rows.iter().map(|r| r.pk_table_schem.as_deref()).collect();
    let pk_table_names: Vec<Option<&str>> =
        rows.iter().map(|r| r.pk_table_name.as_deref()).collect();
    let pk_column_names: Vec<Option<&str>> =
        rows.iter().map(|r| r.pk_column_name.as_deref()).collect();
    let fk_table_cats: Vec<Option<&str>> = rows.iter().map(|r| r.fk_table_cat.as_deref()).collect();
    let fk_table_schems: Vec<Option<&str>> =
        rows.iter().map(|r| r.fk_table_schem.as_deref()).collect();
    let fk_table_names: Vec<Option<&str>> =
        rows.iter().map(|r| r.fk_table_name.as_deref()).collect();
    let fk_column_names: Vec<Option<&str>> =
        rows.iter().map(|r| r.fk_column_name.as_deref()).collect();
    let key_seqs: Vec<Option<i16>> = rows.iter().map(|r| Some(r.key_seq)).collect();
    let update_rules: Vec<Option<i16>> = rows.iter().map(|r| r.update_rule).collect();
    let delete_rules: Vec<Option<i16>> = rows.iter().map(|r| r.delete_rule).collect();
    let fk_names: Vec<Option<&str>> = rows.iter().map(|r| r.fk_name.as_deref()).collect();
    let pk_names: Vec<Option<&str>> = rows.iter().map(|r| r.pk_name.as_deref()).collect();
    let deferrabilities: Vec<Option<i16>> = rows.iter().map(|r| r.deferrability).collect();

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(pk_table_cats)) as ArrayRef,
            Arc::new(StringArray::from(pk_table_schems)) as ArrayRef,
            Arc::new(StringArray::from(pk_table_names)) as ArrayRef,
            Arc::new(StringArray::from(pk_column_names)) as ArrayRef,
            Arc::new(StringArray::from(fk_table_cats)) as ArrayRef,
            Arc::new(StringArray::from(fk_table_schems)) as ArrayRef,
            Arc::new(StringArray::from(fk_table_names)) as ArrayRef,
            Arc::new(StringArray::from(fk_column_names)) as ArrayRef,
            Arc::new(Int16Array::from(key_seqs)) as ArrayRef,
            Arc::new(Int16Array::from(update_rules)) as ArrayRef,
            Arc::new(Int16Array::from(delete_rules)) as ArrayRef,
            Arc::new(StringArray::from(fk_names)) as ArrayRef,
            Arc::new(StringArray::from(pk_names)) as ArrayRef,
            Arc::new(Int16Array::from(deferrabilities)) as ArrayRef,
        ],
    )
    .context(crate::api::error::RecordBatchBuildSnafu)
}

/// Implements `SQLForeignKeys`: returns foreign-key metadata for PK/FK tables.
#[allow(clippy::too_many_arguments)]
pub fn foreign_keys<E: OdbcEncoding>(
    statement_handle: sql::Handle,
    pk_catalog_name: *const E::Char,
    name_length1: sql::SmallInt,
    pk_schema_name: *const E::Char,
    name_length2: sql::SmallInt,
    pk_table_name: *const E::Char,
    name_length3: sql::SmallInt,
    fk_catalog_name: *const E::Char,
    name_length4: sql::SmallInt,
    fk_schema_name: *const E::Char,
    name_length5: sql::SmallInt,
    fk_table_name: *const E::Char,
    name_length6: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::info!("SQLForeignKeys: entry");
    let _exit = ApiExitLog("SQLForeignKeys");

    let pk_catalog_raw = read_opt_str::<E>(pk_catalog_name, name_length1)?;
    let pk_schema_raw = read_opt_str::<E>(pk_schema_name, name_length2)?;
    let pk_table_raw = read_opt_str::<E>(pk_table_name, name_length3)?;
    let fk_catalog_raw = read_opt_str::<E>(fk_catalog_name, name_length4)?;
    let fk_schema_raw = read_opt_str::<E>(fk_schema_name, name_length5)?;
    let fk_table_raw = read_opt_str::<E>(fk_table_name, name_length6)?;
    if pk_table_raw.is_none() && fk_table_raw.is_none() {
        return NullPointerSnafu.fail();
    }

    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let conn = dbc.connection.lock();
    let mut inner = guard.inner.lock();

    validate_catalog_stmt_ready(&inner)?;

    let conn_handle = match &conn.state {
        ConnectionState::Connected { conn_handle, .. } => *conn_handle,
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
    };
    let metadata_id = inner.metadata_id;
    let stmt_handle = guard.stmt_handle;
    drop(conn);

    if metadata_id {
        // Only the side whose table was supplied carries required identifier
        // args; a single-sided query intentionally leaves the other side NULL.
        // A zero-length catalog/schema is treated as *absent* (not HY090),
        // matching the legacy driver and SQLPrimaryKeys: `build_show_in_scope`
        // filters empty strings out, so an empty identifier widens the SHOW
        // scope. NULL still returns HY009.
        if pk_table_raw.is_some() {
            if pk_catalog_raw.is_none() {
                return NullPointerSnafu.fail();
            }
            if pk_schema_raw.is_none() {
                return NullPointerSnafu.fail();
            }
        }
        if fk_table_raw.is_some() {
            if fk_catalog_raw.is_none() {
                return NullPointerSnafu.fail();
            }
            if fk_schema_raw.is_none() {
                return NullPointerSnafu.fail();
            }
        }
    }

    // In identifier mode, fold each argument to its canonical Snowflake form so
    // both the SHOW scope and the client-side re-filter are case-correct.
    let (pk_catalog_raw, pk_schema_raw, pk_table_raw, fk_catalog_raw, fk_schema_raw, fk_table_raw) =
        if metadata_id {
            (
                pk_catalog_raw.map(|s| fold_identifier(&s)),
                pk_schema_raw.map(|s| fold_identifier(&s)),
                pk_table_raw.map(|s| fold_identifier(&s)),
                fk_catalog_raw.map(|s| fold_identifier(&s)),
                fk_schema_raw.map(|s| fold_identifier(&s)),
                fk_table_raw.map(|s| fold_identifier(&s)),
            )
        } else {
            (
                pk_catalog_raw,
                pk_schema_raw,
                pk_table_raw,
                fk_catalog_raw,
                fk_schema_raw,
                fk_table_raw,
            )
        };

    let (pk_catalog_raw, pk_schema_raw) = resolve_show_identifiers(
        pk_catalog_raw,
        pk_schema_raw,
        pk_table_raw.as_deref(),
        conn_handle,
    )?;
    let (fk_catalog_raw, fk_schema_raw) = resolve_show_identifiers(
        fk_catalog_raw,
        fk_schema_raw,
        fk_table_raw.as_deref(),
        conn_handle,
    )?;

    let sql = build_show_foreign_keys_command(
        pk_catalog_raw.as_deref(),
        pk_schema_raw.as_deref(),
        pk_table_raw.as_deref(),
        fk_catalog_raw.as_deref(),
        fk_schema_raw.as_deref(),
        fk_table_raw.as_deref(),
    );

    let show_batch = match execute_show_query_collect_batch(stmt_handle, &sql) {
        Ok(batch) => batch,
        Err(e) => {
            if e.server_sql_state()
                .is_some_and(is_object_not_found_sql_state)
            {
                return set_static_empty_catalog_result(&mut inner, foreign_keys_schema());
            }
            return Err(e);
        }
    };

    let filter = ShowForeignKeyFilter {
        pk_catalog: pk_catalog_raw.as_deref(),
        pk_schema: pk_schema_raw.as_deref(),
        pk_table: pk_table_raw.as_deref(),
        fk_catalog: fk_catalog_raw.as_deref(),
        fk_schema: fk_schema_raw.as_deref(),
        fk_table: fk_table_raw.as_deref(),
    };
    let flat_batch = map_show_foreign_keys_to_odbc(show_batch, &filter)?;
    let schema = flat_batch.schema();
    let reader = reader_from_record_batch(flat_batch, schema)?;
    set_state_for_catalog(
        &mut inner,
        StatementState::QueryExecuted {
            reader,
            rows_affected: Some(-1),
            origin: ExecutionOrigin::Direct,
        },
    );
    Ok(())
}

// ============================================================================
// SQLProcedures — information_schema.procedures
// ============================================================================

/// ODBC 3.x `PROCEDURE_TYPE` value: the object has a return value (Snowflake
/// procedures always declare `RETURNS`, so this is always `SQL_PT_FUNCTION`).
const SQL_PT_FUNCTION: i16 = 2;

fn procedures_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        catalog_text_field("PROCEDURE_CAT", 255),   // 1
        catalog_text_field("PROCEDURE_SCHEM", 255), // 2
        catalog_text_field("PROCEDURE_NAME", 255),  // 3
        catalog_int_field("NUM_INPUT_PARAMS"),      // 4 (reference: INTEGER, arg count)
        catalog_int_field("NUM_OUTPUT_PARAMS"),     // 5 (reserved; always NULL)
        catalog_int_field("NUM_RESULT_SETS"),       // 6 (0/1, 1 iff table-valued)
        catalog_text_field("REMARKS", 65535),       // 7
        catalog_smallint_field("PROCEDURE_TYPE"),   // 8 (reference: SMALLINT)
    ]))
}

/// Escapes a value for embedding in a Snowflake single-quoted string literal.
///
/// Snowflake processes backslash escape sequences inside `'...'` literals, so
/// the backslash must be doubled *before* the single quote is doubled. Escaping
/// only `'` would (a) let a `\'` sequence close the literal early in pattern
/// mode, where `catalog_arg_to_pattern` passes the argument through verbatim,
/// and (b) break
/// exact-match in identifier mode, where `escape_like_wildcards` has already
/// backslash-escaped `_`/`%`: the string layer would strip that backslash and
/// re-expose the wildcard to `LIKE`. Doubling the backslash preserves it through
/// the string layer so `LIKE` (default escape `\`) treats `\_`/`\%` as literals.
fn escape_sql_string_literal(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "''")
}

/// Splits a comma-separated list on top-level commas only, so commas nested
/// inside parentheses (e.g. `NUMBER(10,2)`) do not split a token. Shared by
/// `argument_signature` and `TABLE(...)` return-type parsing.
fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth: i32 = 0;
    let mut start = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(s[start..i].trim());
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    let last = s[start..].trim();
    if !out.is_empty() || !last.is_empty() {
        out.push(last);
    }
    out
}

/// Extracts the text between the outermost parentheses of a signature such as
/// `(P1 VARCHAR, PAGE FLOAT)` → `P1 VARCHAR, PAGE FLOAT`. Returns `""` when the
/// parentheses are absent or empty (a zero-argument procedure yields `()`).
fn parenthesized_inner(sig: &str) -> &str {
    let sig = sig.trim();
    match (sig.find('('), sig.rfind(')')) {
        (Some(open), Some(close)) if open < close => sig[open + 1..close].trim(),
        _ => "",
    }
}

/// Counts the input parameters in an `argument_signature`. Argument types are
/// bare (no precision/scale), so the depth-aware split is defensive rather than
/// strictly required, but it keeps counting robust and matches PR2's tokenizer.
fn count_input_params(argument_signature: &str) -> i32 {
    let inner = parenthesized_inner(argument_signature);
    if inner.is_empty() {
        return 0;
    }
    split_top_level_commas(inner).len() as i32
}

/// SQLSTATEs that `SQLProcedures` maps to an empty result set instead of an
/// error, matching the reference `fetchProceduresFromBackend` catch: `02000`
/// (no data) plus `42000`/`42S02` (a filter matching no procedure, a
/// non-existent database, or an invalid identifier character). Kept separate
/// from [`is_object_not_found_sql_state`] so widening it does not change
/// `SQLPrimaryKeys`/`SQLForeignKeys` behavior.
fn is_procedures_empty_result_sql_state(state: &str) -> bool {
    state == "02000" || is_object_not_found_sql_state(state)
}

/// SQLSTATEs that `SQLTables` SHOW paths map to an empty result set instead of
/// an error. Matches core GetObjects `SHOW_NOT_FOUND_SQLSTATES` (`02000` /
/// `42000` / `42S02`) — the legacy "no metadata" treatment for SHOW queries.
/// Kept separate from [`is_object_not_found_sql_state`] so widening with `02000`
/// does not change `SQLPrimaryKeys`/`SQLForeignKeys` behavior.
///
/// `42000` specifically: Snowflake raises it when a `SHOW … IN DATABASE/SCHEMA`
/// names an object that does not exist or the role cannot see (e.g. a catalog
/// argument resolving to a dropped/invisible database). ODBC contract is to
/// return zero rows there, not an error, and legacy/core behave the same — so we
/// keep parity. The residual risk is that these SHOW statements are now
/// wrapper-constructed (see [`build_show_objects_sql`]): a *builder* bug that
/// emits malformed SQL would also surface as `42000` and be swallowed as an
/// empty result rather than a diagnosable error. That path is covered by the
/// `build_show_objects_sql` / `build_show_columns_sql` unit tests (which pin the
/// exact emitted SQL, including identifier escaping) so a builder regression is
/// caught there rather than silently degrading to "no tables" at runtime.
fn is_tables_empty_result_sql_state(state: &str) -> bool {
    state == "02000" || is_object_not_found_sql_state(state)
}

/// A procedure is table-valued (`NUM_RESULT_SETS = 1`) iff its return `data_type`
/// begins with `TABLE` (case-insensitive), e.g. `TABLE (ID NUMBER, NAME VARCHAR)`.
fn returns_table(data_type: &str) -> bool {
    let t = data_type.trim_start();
    t.len() >= 5 && t.as_bytes()[..5].eq_ignore_ascii_case(b"table")
}

/// Runs `SELECT database_name FROM information_schema.databases` to enumerate the
/// databases to union over when the caller passes a NULL catalog without
/// connection-context resolution (mirrors the reference `QueryDatabases`).
fn enumerate_databases(stmt_handle: StatementHandle) -> OdbcResult<Vec<String>> {
    let batch = execute_show_query_collect_batch(
        stmt_handle,
        "select database_name from information_schema.databases",
    )?;
    let mut dbs = Vec::with_capacity(batch.num_rows());
    for row in 0..batch.num_rows() {
        if let Some(name) = utf8_value_at(&batch, 0, row) {
            dbs.push(name);
        }
    }
    Ok(dbs)
}

/// Builds the `information_schema.procedures` query, unioning one subquery per
/// database. Matches the reference driver: `procedure_name`/`procedure_schema`
/// LIKE filters are only appended when meaningful (`proc != "%"`, non-empty).
fn build_procedures_query(
    db_names: &[String],
    schema_pattern: Option<&str>,
    proc_pattern: Option<&str>,
) -> String {
    let mut where_clauses: Vec<String> = Vec::new();
    if let Some(p) = proc_pattern
        && p != "%"
        && !p.is_empty()
    {
        where_clauses.push(format!(
            "procedure_name like '{}'",
            escape_sql_string_literal(p)
        ));
    }
    if let Some(s) = schema_pattern
        && !s.is_empty()
    {
        where_clauses.push(format!(
            "procedure_schema like '{}'",
            escape_sql_string_literal(s)
        ));
    }
    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!(" where {}", where_clauses.join(" AND "))
    };

    db_names
        .iter()
        .map(|db| {
            format!(
                "(select procedure_catalog, procedure_schema, procedure_name, \
                 argument_signature, data_type, comment from \"{}\".information_schema.procedures{})",
                escape_snowflake_identifier(db),
                where_sql
            )
        })
        .collect::<Vec<_>>()
        .join("\n union all \n")
}

fn procedures_column_index(schema: &Schema, column: &'static str) -> OdbcResult<usize> {
    column_index_by_name(schema, column).with_context(|| {
        crate::api::error::ProcedureMetadataParseSnafu {
            detail: format!("information_schema.procedures result is missing '{column}'"),
        }
    })
}

/// Maps the `information_schema.procedures` result to the 8-column ODBC
/// `SQLProcedures` result set. Preserves server/union order (no client sort),
/// matching the reference driver.
fn map_procedures_to_odbc(batch: RecordBatch) -> OdbcResult<RecordBatch> {
    let schema = procedures_schema();
    if batch.num_rows() == 0 {
        return Ok(RecordBatch::new_empty(schema));
    }

    let input = batch.schema();
    let idx_cat = procedures_column_index(&input, "procedure_catalog")?;
    let idx_schema = procedures_column_index(&input, "procedure_schema")?;
    let idx_name = procedures_column_index(&input, "procedure_name")?;
    let idx_args = procedures_column_index(&input, "argument_signature")?;
    let idx_data_type = procedures_column_index(&input, "data_type")?;
    let idx_comment = procedures_column_index(&input, "comment")?;

    let n = batch.num_rows();
    let mut cats: Vec<Option<String>> = Vec::with_capacity(n);
    let mut schems: Vec<Option<String>> = Vec::with_capacity(n);
    let mut names: Vec<Option<String>> = Vec::with_capacity(n);
    let mut num_inputs: Vec<Option<i32>> = Vec::with_capacity(n);
    let mut num_result_sets: Vec<Option<i32>> = Vec::with_capacity(n);
    let mut remarks: Vec<Option<String>> = Vec::with_capacity(n);

    for row in 0..n {
        cats.push(utf8_value_at(&batch, idx_cat, row));
        schems.push(utf8_value_at(&batch, idx_schema, row));
        names.push(utf8_value_at(&batch, idx_name, row));
        let arg_sig = utf8_value_at(&batch, idx_args, row).unwrap_or_default();
        num_inputs.push(Some(count_input_params(&arg_sig)));
        let data_type = utf8_value_at(&batch, idx_data_type, row).unwrap_or_default();
        num_result_sets.push(Some(if returns_table(&data_type) { 1 } else { 0 }));
        remarks.push(utf8_value_at(&batch, idx_comment, row));
    }

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(cats)) as ArrayRef,
            Arc::new(StringArray::from(schems)) as ArrayRef,
            Arc::new(StringArray::from(names)) as ArrayRef,
            Arc::new(Int32Array::from(num_inputs)) as ArrayRef,
            // NUM_OUTPUT_PARAMS: reserved, always NULL.
            Arc::new(Int32Array::from(vec![None; n])) as ArrayRef,
            Arc::new(Int32Array::from(num_result_sets)) as ArrayRef,
            Arc::new(StringArray::from(remarks)) as ArrayRef,
            // PROCEDURE_TYPE: SQL_PT_FUNCTION for every Snowflake procedure.
            Arc::new(Int16Array::from(vec![Some(SQL_PT_FUNCTION); n])) as ArrayRef,
        ],
    )
    .context(crate::api::error::RecordBatchBuildSnafu)
}

/// Implements `SQLProcedures`: lists stored procedures from
/// `information_schema.procedures`.
pub fn procedures<E: OdbcEncoding>(
    statement_handle: sql::Handle,
    catalog_name: *const E::Char,
    name_length1: sql::SmallInt,
    schema_name: *const E::Char,
    name_length2: sql::SmallInt,
    proc_name: *const E::Char,
    name_length3: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::info!("SQLProcedures: entry");
    let _exit = ApiExitLog("SQLProcedures");

    let catalog_raw = read_opt_str::<E>(catalog_name, name_length1)?;
    let schema_raw = read_opt_str::<E>(schema_name, name_length2)?;
    let proc_raw = read_opt_str::<E>(proc_name, name_length3)?;

    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let conn = dbc.connection.lock();
    let mut inner = guard.inner.lock();

    validate_catalog_stmt_ready(&inner)?;

    let conn_handle = match &conn.state {
        ConnectionState::Connected { conn_handle, .. } => *conn_handle,
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
    };
    let metadata_id = inner.metadata_id;
    let stmt_handle = guard.stmt_handle;
    drop(conn);

    // In identifier mode (SQL_ATTR_METADATA_ID = TRUE) every argument is a
    // required identifier: a NULL pointer is HY009.
    if metadata_id && (catalog_raw.is_none() || schema_raw.is_none() || proc_raw.is_none()) {
        return NullPointerSnafu.fail();
    }

    // Catalog is an exact database identifier (ODBC forbids a search pattern);
    // schema and proc name are LIKE patterns. Fold the catalog identifier in
    // identifier mode so the quoted `"db"` scope is case-correct.
    let mut db_name: Option<String> = if metadata_id {
        catalog_raw.as_deref().map(fold_identifier)
    } else {
        catalog_raw.clone()
    };
    let mut schema_pattern = catalog_arg_to_pattern(schema_raw.as_deref(), metadata_id)?;
    let proc_pattern = catalog_arg_to_pattern(proc_raw.as_deref(), metadata_id)?;

    // NULL catalog under CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX resolves to
    // the connection's current database (and schema, if schema is also NULL);
    // otherwise a NULL catalog enumerates every database (union all).
    if catalog_raw.is_none() && metadata_request_use_connection_ctx(conn_handle)? {
        let rt = global().context(OdbcRuntimeSnafu)?;
        let info = rt.block_on(async |c| {
            c.connection_get_info(ConnectionGetInfoRequest {
                conn_handle: Some(conn_handle),
                info_codes: vec![],
                include_master_token: false,
            })
            .await
        })?;
        db_name = info.database;
        if schema_raw.is_none() {
            schema_pattern = info.schema;
        }
    }

    let db_names = match db_name {
        Some(db) => vec![db],
        None => enumerate_databases(stmt_handle)?,
    };

    if db_names.is_empty() {
        return set_static_empty_catalog_result(&mut inner, procedures_schema());
    }

    let sql = build_procedures_query(
        &db_names,
        schema_pattern.as_deref(),
        proc_pattern.as_deref(),
    );

    let batch = match execute_show_query_collect_batch(stmt_handle, &sql) {
        Ok(batch) => batch,
        Err(e) => {
            if e.server_sql_state()
                .is_some_and(is_procedures_empty_result_sql_state)
            {
                return set_static_empty_catalog_result(&mut inner, procedures_schema());
            }
            return Err(e);
        }
    };

    let flat_batch = map_procedures_to_odbc(batch)?;
    let schema = flat_batch.schema();
    let reader = reader_from_record_batch(flat_batch, schema)?;
    set_state_for_catalog(
        &mut inner,
        StatementState::QueryExecuted {
            reader,
            rows_affected: Some(-1),
            origin: ExecutionOrigin::Direct,
        },
    );
    Ok(())
}

// ============================================================================
// SQLProcedureColumns — information_schema.procedures + signature parsing
// ============================================================================

// ODBC `COLUMN_TYPE` (pseudo-column kind) values from the SQLProcedureColumns
// spec. `SQL_PARAM_INPUT` (1) — Snowflake procedures take only IN parameters;
// `SQL_RESULT_COL` (3) — a column of a table-valued return; `SQL_RETURN_VALUE`
// (5) — the scalar return value.
const SQL_PARAM_INPUT: i16 = 1;
const SQL_RESULT_COL: i16 = 3;
const SQL_RETURN_VALUE: i16 = 5;
// `IS RESULT SET COLUMN` (driver-specific col 20) carries ODBC SQL_TRUE/FALSE.
const SQL_TRUE: i16 = 1;
const SQL_FALSE: i16 = 0;
// `NULLABLE` (col 12): Snowflake procedure parameters/results are always
// nullable (ODBC `SQL_NULLABLE`).
const SQL_NULLABLE: i16 = 1;

/// Default scale Snowflake applies to TIME/TIMESTAMP* when the type string omits
/// it (mirrors the reference driver's `TIMESTAMP_SCALE_DEFAULT`).
const SNOWFLAKE_DEFAULT_FRACTIONAL_SCALE: i32 = 9;
/// Default precision Snowflake applies to NUMBER/DECIMAL when omitted (mirrors
/// the reference driver's `NUMBER_PRECISION_DEFAULT`).
const SNOWFLAKE_DEFAULT_NUMBER_PRECISION: i32 = 38;

/// The 21-column `SQLProcedureColumns` result set: 19 ODBC 3.x spec columns plus
/// the two driver-specific trailing columns (`IS RESULT SET COLUMN`,
/// `USER_DATA_TYPE`) the reference driver appends. Numeric columns are emitted as
/// text and converted at `SQLGetData` time, matching the `SQLColumns` schema.
fn procedure_columns_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        catalog_text_field("PROCEDURE_CAT", 255),       // 1
        catalog_text_field("PROCEDURE_SCHEM", 255),     // 2
        catalog_text_field("PROCEDURE_NAME", 255),      // 3
        catalog_text_field("COLUMN_NAME", 255),         // 4 (NOT NULL; "" for return value)
        catalog_text_field("COLUMN_TYPE", 20),          // 5 SMALLINT
        catalog_text_field("DATA_TYPE", 20),            // 6 SMALLINT
        catalog_text_field("TYPE_NAME", 255),           // 7
        catalog_text_field("COLUMN_SIZE", 20),          // 8 INTEGER nullable
        catalog_text_field("BUFFER_LENGTH", 20),        // 9 INTEGER nullable
        catalog_text_field("DECIMAL_DIGITS", 20),       // 10 SMALLINT nullable
        catalog_text_field("NUM_PREC_RADIX", 20),       // 11 SMALLINT nullable
        catalog_text_field("NULLABLE", 20),             // 12 SMALLINT
        catalog_text_field("REMARKS", 65535),           // 13 nullable
        catalog_text_field("COLUMN_DEF", 65535),        // 14 nullable
        catalog_text_field("SQL_DATA_TYPE", 20),        // 15 SMALLINT
        catalog_text_field("SQL_DATETIME_SUB", 20),     // 16 SMALLINT nullable
        catalog_text_field("CHAR_OCTET_LENGTH", 20),    // 17 INTEGER nullable
        catalog_text_field("ORDINAL_POSITION", 20),     // 18 INTEGER
        catalog_text_field("IS_NULLABLE", 20),          // 19
        catalog_text_field("IS RESULT SET COLUMN", 20), // 20 driver-specific SMALLINT
        catalog_text_field("USER_DATA_TYPE", 20),       // 21 driver-specific SMALLINT
    ]))
}

/// A parsed procedure parameter or result-set column: its ODBC `COLUMN_NAME`
/// plus the raw Snowflake type string. `name` is empty for a scalar return value.
struct ProcColumn {
    name: String,
    type_str: String,
}

/// The shape of a procedure's `data_type` return descriptor.
enum ReturnShape {
    /// Scalar return value; carries the return type string.
    Scalar(String),
    /// Table-valued return (`RETURNS TABLE(...)`); carries the result-set columns.
    Table(Vec<ProcColumn>),
}

/// Splits one `NAME TYPE` token (e.g. `PAGE FLOAT`, `V DOUBLE PRECISION`) on the
/// first ASCII whitespace so multi-word type names stay intact. Returns `None`
/// for a token missing either half.
fn parse_named_column(token: &str) -> Option<ProcColumn> {
    let token = token.trim();
    let split = token.find(char::is_whitespace)?;
    let name = token[..split].trim();
    let type_str = token[split..].trim();
    if name.is_empty() || type_str.is_empty() {
        return None;
    }
    Some(ProcColumn {
        name: name.to_string(),
        type_str: type_str.to_string(),
    })
}

/// Parses an `argument_signature` (e.g. `(PNAME VARCHAR, PAGE FLOAT)`) into
/// ordered (name, type) pairs, tolerating nested `(...)` in a type via the
/// depth-aware comma split.
fn parse_argument_columns(argument_signature: &str) -> Vec<ProcColumn> {
    let inner = parenthesized_inner(argument_signature);
    if inner.is_empty() {
        return Vec::new();
    }
    split_top_level_commas(inner)
        .into_iter()
        .filter_map(parse_named_column)
        .collect()
}

/// Classifies a procedure's `data_type` as a scalar return value or a
/// table-valued return, extracting the result-set columns in the latter case.
fn parse_return_shape(data_type: &str) -> ReturnShape {
    if returns_table(data_type) {
        let inner = parenthesized_inner(data_type);
        // SNOW-1232955: a `RETURNS TABLE()` with an empty column list is valid.
        let cols = if inner.is_empty() {
            Vec::new()
        } else {
            split_top_level_commas(inner)
                .into_iter()
                .filter_map(parse_named_column)
                .collect()
        };
        ReturnShape::Table(cols)
    } else {
        ReturnShape::Scalar(data_type.trim().to_string())
    }
}

/// Splits a Snowflake type string such as `NUMBER(38,0)` / `VARCHAR(16777216)` /
/// `FLOAT` into (base, inner-args). Uses the last `)` so any nested parens are
/// tolerated.
fn split_type_and_args(type_str: &str) -> (&str, Option<&str>) {
    let ts = type_str.trim();
    match (ts.find('('), ts.rfind(')')) {
        (Some(o), Some(c)) if o < c => (ts[..o].trim(), Some(ts[o + 1..c].trim())),
        _ => (ts, None),
    }
}

/// Parses up to two comma-separated integer args (`precision[, scale]`) from a
/// type argument list. Absent or non-integer entries yield `None`.
fn parse_two_int_args(inner: Option<&str>) -> (Option<i32>, Option<i32>) {
    let Some(inner) = inner else {
        return (None, None);
    };
    let mut it = inner.split(',');
    let first = it.next().and_then(|s| s.trim().parse::<i32>().ok());
    let second = it.next().and_then(|s| s.trim().parse::<i32>().ok());
    (first, second)
}

/// Maps a Snowflake SQL type string (as it appears in `argument_signature` or
/// the `data_type` return column) to an Arrow [`Field`] carrying the
/// `logicalType`/precision/scale/length metadata the shared `*_from_field`
/// helpers consume. Routing through [`rehydrate_field`] keeps SQLProcedureColumns
/// type reporting identical to SQLColumns/SQLDescribeCol (one mapping, no drift).
fn field_from_sql_type_string(type_str: &str, numeric_settings: &NumericSettings) -> Field {
    let (base, inner) = split_type_and_args(type_str);
    let default_varchar = numeric_settings.max_varchar_size.min(i64::MAX as u64) as i64;
    // Timestamp/time scale: explicit arg, else Snowflake's default of 9.
    let fractional_scale = || {
        parse_two_int_args(inner)
            .0
            .unwrap_or(SNOWFLAKE_DEFAULT_FRACTIONAL_SCALE)
    };
    match base.to_ascii_uppercase().as_str() {
        "VARCHAR" | "CHAR" | "CHARACTER" | "STRING" | "TEXT" | "NVARCHAR" | "NCHAR"
        | "NVARCHAR2" | "CHAR VARYING" | "NCHAR VARYING" | "CHARACTER VARYING" => {
            let char_len = parse_two_int_args(inner)
                .0
                .map(i64::from)
                .unwrap_or(default_varchar);
            rehydrate_field("TEXT", None, None, Some(char_len), None, true)
        }
        "NUMBER" | "DECIMAL" | "NUMERIC" => {
            let (p, s) = parse_two_int_args(inner);
            rehydrate_field(
                "FIXED",
                Some(p.unwrap_or(SNOWFLAKE_DEFAULT_NUMBER_PRECISION)),
                Some(s.unwrap_or(0)),
                None,
                None,
                true,
            )
        }
        "INT" | "INTEGER" | "BIGINT" | "SMALLINT" | "TINYINT" | "BYTEINT" => rehydrate_field(
            "FIXED",
            Some(SNOWFLAKE_DEFAULT_NUMBER_PRECISION),
            Some(0),
            None,
            None,
            true,
        ),
        "FLOAT" | "FLOAT4" | "FLOAT8" | "DOUBLE" | "DOUBLE PRECISION" | "REAL" => {
            rehydrate_field("REAL", None, None, None, None, true)
        }
        "BOOLEAN" | "BOOL" => rehydrate_field("BOOLEAN", None, None, None, None, true),
        "DATE" => rehydrate_field("DATE", None, None, None, None, true),
        "TIME" => rehydrate_field("TIME", None, Some(fractional_scale()), None, None, true),
        "TIMESTAMP_LTZ" | "TIMESTAMPLTZ" => rehydrate_field(
            "TIMESTAMP_LTZ",
            None,
            Some(fractional_scale()),
            None,
            None,
            true,
        ),
        "TIMESTAMP_TZ" | "TIMESTAMPTZ" => rehydrate_field(
            "TIMESTAMP_TZ",
            None,
            Some(fractional_scale()),
            None,
            None,
            true,
        ),
        "TIMESTAMP_NTZ" | "TIMESTAMPNTZ" | "TIMESTAMP" | "DATETIME" => rehydrate_field(
            "TIMESTAMP_NTZ",
            None,
            Some(fractional_scale()),
            None,
            None,
            true,
        ),
        "BINARY" | "VARBINARY" => {
            let byte_len = parse_two_int_args(inner).0.map(i64::from);
            rehydrate_field("BINARY", None, None, None, byte_len, true)
        }
        "VARIANT" => rehydrate_field("VARIANT", None, None, Some(default_varchar), None, true),
        "OBJECT" => rehydrate_field("OBJECT", None, None, Some(default_varchar), None, true),
        "ARRAY" => rehydrate_field("ARRAY", None, None, Some(default_varchar), None, true),
        "VECTOR" => rehydrate_field("VECTOR", None, None, Some(default_varchar), None, true),
        // GEOGRAPHY/GEOMETRY and anything unrecognized fall back to a character
        // type, matching the reference driver's treatment of unmapped types.
        _ => rehydrate_field("TEXT", None, None, Some(default_varchar), None, true),
    }
}

/// Minimal SQL `LIKE` matcher supporting `%` (any run, incl. empty), `_` (any
/// single char), and `\` as the escape char — matching `escape_like_wildcards`
/// and `catalog_arg_to_pattern`, which feed this.
///
/// Intentionally **case-sensitive**, mirroring `SQLPrimaryKeys`/the reference
/// driver rather than core's case-insensitive `like_pattern::matches`. The
/// `ColumnName` argument is pre-processed by `catalog_arg_to_pattern`, which
/// already folds unquoted identifiers to uppercase in identifier mode
/// (`METADATA_ID=TRUE`) while passing search patterns through verbatim in
/// pattern mode (`METADATA_ID=FALSE`). Because that upstream step performs the
/// ODBC-mandated case-folding, a case-sensitive matcher yields the correct,
/// spec-aligned result: identifier mode matches case-insensitively (the pattern
/// is already upper-cased and column names come upper-folded from
/// `information_schema`), while pattern mode stays case-sensitive like Snowflake
/// `LIKE`. Do not "align" this with core's case-insensitive matcher — that would
/// silently make pattern mode case-insensitive (the `SQLTables`/`SQLColumns`
/// divergence tracked separately).
fn like_match(pattern: &str, text: &str) -> bool {
    // Empty pattern never matches (aligned with core like_pattern::matches and
    // Snowflake identifiers — none are named "").
    if pattern.is_empty() {
        return false;
    }
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    // Backtrack point for the most recent `%`: the pattern index just past it and
    // the text index it is currently assumed to consume up to.
    let (mut star_pi, mut star_ti): (Option<usize>, usize) = (None, 0);
    while ti < t.len() {
        let matched = if pi < p.len() {
            match p[pi] {
                '\\' if pi + 1 < p.len() => {
                    if p[pi + 1] == t[ti] {
                        pi += 2;
                        ti += 1;
                        true
                    } else {
                        false
                    }
                }
                '%' => {
                    star_pi = Some(pi);
                    star_ti = ti;
                    pi += 1;
                    // `%` consumes nothing yet; re-loop without advancing `ti`.
                    continue;
                }
                '_' => {
                    pi += 1;
                    ti += 1;
                    true
                }
                c => {
                    if c == t[ti] {
                        pi += 1;
                        ti += 1;
                        true
                    } else {
                        false
                    }
                }
            }
        } else {
            false
        };
        if matched {
            continue;
        }
        // Mismatch: extend the last `%` by one text char, or fail.
        match star_pi {
            Some(sp) => {
                star_ti += 1;
                ti = star_ti;
                pi = sp + 1;
            }
            None => return false,
        }
    }
    // Trailing `%` in the pattern match the empty remainder.
    while pi < p.len() && p[pi] == '%' {
        pi += 1;
    }
    pi == p.len()
}

/// One output row of the 21-column `SQLProcedureColumns` result set. Named
/// fields (all text, converted at `SQLGetData` time) avoid the change
/// amplification of parallel-Vec builders.
struct FlatProcColumnRow {
    cat: Option<String>,
    schem: Option<String>,
    proc: Option<String>,
    col_name: Option<String>,
    column_type: Option<String>,
    data_type: Option<String>,
    type_name: Option<String>,
    col_size: Option<String>,
    buf_len: Option<String>,
    dec_digits: Option<String>,
    num_prec_radix: Option<String>,
    nullable: Option<String>,
    remarks: Option<String>,
    col_def: Option<String>,
    sql_data_type: Option<String>,
    sql_dt_sub: Option<String>,
    char_octet: Option<String>,
    ordinal: Option<String>,
    is_nullable: Option<String>,
    is_result_set_col: Option<String>,
    user_data_type: Option<String>,
}

/// Appends the SQLProcedureColumns rows for one `information_schema.procedures`
/// row, honoring the ODBC ordering: for a scalar procedure the return value
/// comes first (`ORDINAL_POSITION` 0) then the input parameters (1..); for a
/// table-valued procedure the result-set columns come first (1..) then the input
/// parameters (1..). The `column_pattern` `LIKE` filter is applied to
/// `COLUMN_NAME` — a non-`%` pattern therefore drops the empty-named return value.
#[allow(clippy::too_many_arguments)]
fn append_procedure_column_rows(
    rows: &mut Vec<FlatProcColumnRow>,
    cat: &Option<String>,
    schem: &Option<String>,
    proc_name: &Option<String>,
    argument_signature: &str,
    data_type: &str,
    column_pattern: Option<&str>,
    numeric_settings: &NumericSettings,
) {
    let params = parse_argument_columns(argument_signature);
    // (name, type_str, COLUMN_TYPE, ORDINAL_POSITION) in ODBC emit order.
    let mut entries: Vec<(String, String, i16, i32)> = Vec::new();
    match parse_return_shape(data_type) {
        ReturnShape::Scalar(ret_type) => {
            entries.push((String::new(), ret_type, SQL_RETURN_VALUE, 0));
            for (i, p) in params.iter().enumerate() {
                entries.push((
                    p.name.clone(),
                    p.type_str.clone(),
                    SQL_PARAM_INPUT,
                    (i + 1) as i32,
                ));
            }
        }
        ReturnShape::Table(cols) => {
            for (i, c) in cols.iter().enumerate() {
                entries.push((
                    c.name.clone(),
                    c.type_str.clone(),
                    SQL_RESULT_COL,
                    (i + 1) as i32,
                ));
            }
            for (i, p) in params.iter().enumerate() {
                entries.push((
                    p.name.clone(),
                    p.type_str.clone(),
                    SQL_PARAM_INPUT,
                    (i + 1) as i32,
                ));
            }
        }
    }

    for (col_name, type_str, column_type, ordinal) in entries {
        if let Some(pat) = column_pattern
            && !like_match(pat, &col_name)
        {
            continue;
        }
        let field = field_from_sql_type_string(&type_str, numeric_settings);
        let logical_type = field
            .metadata()
            .get("logicalType")
            .map(|s| s.as_str())
            .unwrap_or("");

        let data_type_val = sql_type_from_field(&field, numeric_settings)
            .ok()
            .map(|t| t.0.to_string());
        let type_name_val = type_name_from_field(&field, numeric_settings)
            .ok()
            .map(|s| s.to_string());
        let col_size_val = column_size_from_field(&field, numeric_settings)
            .ok()
            .map(|s| s.to_string());
        let buf_len_val = octet_length_from_field(&field, numeric_settings)
            .ok()
            .map(|s| s.to_string());
        let dec_digits_val = decimal_digits_from_field(&field, numeric_settings)
            .ok()
            .map(|s| s.to_string());
        let num_prec_radix_val = num_prec_radix_from_field(&field, numeric_settings)
            .ok()
            .and_then(|s| if s == 0 { None } else { Some(s.to_string()) });
        let sql_data_type_val = verbose_sql_type_from_field(&field, numeric_settings)
            .ok()
            .map(|t| t.0.to_string());
        let sql_dt_sub_val =
            sql_datetime_sub_from_logical_type(logical_type).map(|s| s.to_string());
        let char_octet_val = match logical_type {
            "TEXT" | "BINARY" => octet_length_from_field(&field, numeric_settings)
                .ok()
                .map(|s| s.to_string()),
            _ => None,
        };
        let is_result_set_col = if column_type == SQL_RESULT_COL {
            SQL_TRUE
        } else {
            SQL_FALSE
        };

        rows.push(FlatProcColumnRow {
            cat: cat.clone(),
            schem: schem.clone(),
            proc: proc_name.clone(),
            // COLUMN_NAME is NOT NULL per spec; the return value carries "".
            col_name: Some(col_name),
            column_type: Some(column_type.to_string()),
            data_type: data_type_val,
            type_name: type_name_val,
            col_size: col_size_val,
            buf_len: buf_len_val,
            dec_digits: dec_digits_val,
            num_prec_radix: num_prec_radix_val,
            // Snowflake procedure parameters and results are always nullable.
            nullable: Some(SQL_NULLABLE.to_string()),
            remarks: None,
            col_def: None,
            sql_data_type: sql_data_type_val,
            sql_dt_sub: sql_dt_sub_val,
            char_octet: char_octet_val,
            ordinal: Some(ordinal.to_string()),
            is_nullable: Some("YES".to_string()),
            is_result_set_col: Some(is_result_set_col.to_string()),
            user_data_type: Some("0".to_string()),
        });
    }
}

/// Maps the `information_schema.procedures` result to the 21-column
/// `SQLProcedureColumns` result set, parsing each procedure's argument signature
/// and return type into per-column rows. Preserves server/union order.
fn map_procedure_columns_to_odbc(
    batch: RecordBatch,
    column_pattern: Option<&str>,
    numeric_settings: &NumericSettings,
) -> OdbcResult<RecordBatch> {
    let schema = procedure_columns_schema();
    if batch.num_rows() == 0 {
        return Ok(RecordBatch::new_empty(schema));
    }

    let input = batch.schema();
    let idx_cat = procedures_column_index(&input, "procedure_catalog")?;
    let idx_schema = procedures_column_index(&input, "procedure_schema")?;
    let idx_name = procedures_column_index(&input, "procedure_name")?;
    let idx_args = procedures_column_index(&input, "argument_signature")?;
    let idx_data_type = procedures_column_index(&input, "data_type")?;

    let mut rows: Vec<FlatProcColumnRow> = Vec::new();
    for row in 0..batch.num_rows() {
        let cat = utf8_value_at(&batch, idx_cat, row);
        let schem = utf8_value_at(&batch, idx_schema, row);
        let name = utf8_value_at(&batch, idx_name, row);
        let arg_sig = utf8_value_at(&batch, idx_args, row).unwrap_or_default();
        let dtype = utf8_value_at(&batch, idx_data_type, row).unwrap_or_default();
        append_procedure_column_rows(
            &mut rows,
            &cat,
            &schem,
            &name,
            &arg_sig,
            &dtype,
            column_pattern,
            numeric_settings,
        );
    }

    build_procedure_columns_batch(schema, rows)
}

fn build_procedure_columns_batch(
    schema: SchemaRef,
    rows: Vec<FlatProcColumnRow>,
) -> OdbcResult<RecordBatch> {
    fn to_array(v: Vec<Option<String>>) -> ArrayRef {
        Arc::new(StringArray::from(v)) as ArrayRef
    }
    let n = rows.len();
    macro_rules! col {
        ($field:ident) => {{
            let mut v = Vec::with_capacity(n);
            for r in &rows {
                v.push(r.$field.clone());
            }
            to_array(v)
        }};
    }
    RecordBatch::try_new(
        schema,
        vec![
            col!(cat),
            col!(schem),
            col!(proc),
            col!(col_name),
            col!(column_type),
            col!(data_type),
            col!(type_name),
            col!(col_size),
            col!(buf_len),
            col!(dec_digits),
            col!(num_prec_radix),
            col!(nullable),
            col!(remarks),
            col!(col_def),
            col!(sql_data_type),
            col!(sql_dt_sub),
            col!(char_octet),
            col!(ordinal),
            col!(is_nullable),
            col!(is_result_set_col),
            col!(user_data_type),
        ],
    )
    .context(crate::api::error::RecordBatchBuildSnafu)
}

/// Implements `SQLProcedureColumns`: lists the input parameters, return value,
/// and result-set columns of stored procedures from
/// `information_schema.procedures`. Shares the query builder, database
/// enumeration, connection-context resolution, and silent-empty-on-error
/// handling with [`procedures`].
#[allow(clippy::too_many_arguments)]
pub fn procedure_columns<E: OdbcEncoding>(
    statement_handle: sql::Handle,
    catalog_name: *const E::Char,
    name_length1: sql::SmallInt,
    schema_name: *const E::Char,
    name_length2: sql::SmallInt,
    proc_name: *const E::Char,
    name_length3: sql::SmallInt,
    column_name: *const E::Char,
    name_length4: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::info!("SQLProcedureColumns: entry");
    let _exit = ApiExitLog("SQLProcedureColumns");

    let catalog_raw = read_opt_str::<E>(catalog_name, name_length1)?;
    let schema_raw = read_opt_str::<E>(schema_name, name_length2)?;
    let proc_raw = read_opt_str::<E>(proc_name, name_length3)?;
    let column_raw = read_opt_str::<E>(column_name, name_length4)?;

    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let conn = dbc.connection.lock();
    let mut inner = guard.inner.lock();

    validate_catalog_stmt_ready(&inner)?;

    let conn_handle = match &conn.state {
        ConnectionState::Connected { conn_handle, .. } => *conn_handle,
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
    };
    let metadata_id = inner.metadata_id;
    let numeric_settings = conn.numeric_settings;
    let stmt_handle = guard.stmt_handle;
    drop(conn);

    // Identifier mode (SQL_ATTR_METADATA_ID = TRUE): catalog, schema, and proc
    // name are all required identifiers — a NULL pointer is HY009. ColumnName may
    // still be NULL (means "all columns").
    if metadata_id && (catalog_raw.is_none() || schema_raw.is_none() || proc_raw.is_none()) {
        return NullPointerSnafu.fail();
    }

    let mut db_name: Option<String> = if metadata_id {
        catalog_raw.as_deref().map(fold_identifier)
    } else {
        catalog_raw.clone()
    };
    let mut schema_pattern = catalog_arg_to_pattern(schema_raw.as_deref(), metadata_id)?;
    let proc_pattern = catalog_arg_to_pattern(proc_raw.as_deref(), metadata_id)?;
    // ColumnName is a pattern-value arg; it filters COLUMN_NAME client-side after
    // the signature is parsed (the type strings never reach the server query).
    let column_pattern = catalog_arg_to_pattern(column_raw.as_deref(), metadata_id)?;

    if catalog_raw.is_none() && metadata_request_use_connection_ctx(conn_handle)? {
        let rt = global().context(OdbcRuntimeSnafu)?;
        let info = rt.block_on(async |c| {
            c.connection_get_info(ConnectionGetInfoRequest {
                conn_handle: Some(conn_handle),
                info_codes: vec![],
                include_master_token: false,
            })
            .await
        })?;
        db_name = info.database;
        if schema_raw.is_none() {
            schema_pattern = info.schema;
        }
    }

    let db_names = match db_name {
        Some(db) => vec![db],
        None => enumerate_databases(stmt_handle)?,
    };

    if db_names.is_empty() {
        return set_static_empty_catalog_result(&mut inner, procedure_columns_schema());
    }

    let sql = build_procedures_query(
        &db_names,
        schema_pattern.as_deref(),
        proc_pattern.as_deref(),
    );

    let batch = match execute_show_query_collect_batch(stmt_handle, &sql) {
        Ok(batch) => batch,
        Err(e) => {
            if e.server_sql_state()
                .is_some_and(is_procedures_empty_result_sql_state)
            {
                return set_static_empty_catalog_result(&mut inner, procedure_columns_schema());
            }
            return Err(e);
        }
    };

    let flat_batch =
        map_procedure_columns_to_odbc(batch, column_pattern.as_deref(), &numeric_settings)?;
    let schema = flat_batch.schema();
    let reader = reader_from_record_batch(flat_batch, schema)?;
    set_state_for_catalog(
        &mut inner,
        StatementState::QueryExecuted {
            reader,
            rows_affected: Some(-1),
            origin: ExecutionOrigin::Direct,
        },
    );
    Ok(())
}

// ============================================================================
// Wrapper-owned SHOW paths
// ============================================================================

fn execute_show_all_catalogs(
    inner: &mut StatementInner,
    stmt_handle: StatementHandle,
) -> OdbcResult<()> {
    // ODBC SQL_ALL_CATALOGS is an unconditional catalog enumeration — do not
    // narrow via CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX (unlike the old
    // GetObjects path, which applied apply_connection_context to every depth).
    let show_batch =
        match execute_show_query_collect_batch(stmt_handle, "SHOW DATABASES IN ACCOUNT") {
            Ok(batch) => batch,
            Err(e) => {
                if e.server_sql_state()
                    .is_some_and(is_tables_empty_result_sql_state)
                {
                    return set_static_empty_catalog_result(inner, flat_tables_schema());
                }
                return Err(e);
            }
        };

    let name_idx = column_index_by_name(&show_batch.schema(), "name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW DATABASES result missing 'name' column".to_string(),
            location: snafu::location!(),
        }
    })?;

    let mut cats: Vec<Option<String>> = Vec::with_capacity(show_batch.num_rows());
    let mut nones: Vec<Option<String>> = Vec::with_capacity(show_batch.num_rows());
    for row in 0..show_batch.num_rows() {
        cats.push(utf8_value_at(&show_batch, name_idx, row));
        nones.push(None);
    }

    let schema = flat_tables_schema();
    let flat_batch = build_flat_batch(
        schema.clone(),
        cats,
        nones.clone(),
        nones.clone(),
        nones.clone(),
        nones,
    )?;
    install_catalog_batch(inner, flat_batch, schema)
}

fn execute_show_all_schemas(
    inner: &mut StatementInner,
    stmt_handle: StatementHandle,
) -> OdbcResult<()> {
    // ODBC SQL_ALL_SCHEMAS is an unconditional schema enumeration — do not
    // narrow via CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX (unlike the old
    // GetObjects path, which applied apply_connection_context to every depth).
    let show_batch = match execute_show_query_collect_batch(stmt_handle, "SHOW SCHEMAS IN ACCOUNT")
    {
        Ok(batch) => batch,
        Err(e) => {
            if e.server_sql_state()
                .is_some_and(is_tables_empty_result_sql_state)
            {
                return set_static_empty_catalog_result(inner, flat_tables_schema());
            }
            return Err(e);
        }
    };

    let input = show_batch.schema();
    let db_idx = column_index_by_name(&input, "database_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW SCHEMAS result missing 'database_name' column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let name_idx = column_index_by_name(&input, "name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW SCHEMAS result missing 'name' column".to_string(),
            location: snafu::location!(),
        }
    })?;

    let mut cats: Vec<Option<String>> = Vec::with_capacity(show_batch.num_rows());
    let mut schms: Vec<Option<String>> = Vec::with_capacity(show_batch.num_rows());
    let mut nones: Vec<Option<String>> = Vec::with_capacity(show_batch.num_rows());
    for row in 0..show_batch.num_rows() {
        cats.push(utf8_value_at(&show_batch, db_idx, row));
        schms.push(utf8_value_at(&show_batch, name_idx, row));
        nones.push(None);
    }

    let schema = flat_tables_schema();
    let flat_batch = build_flat_batch(
        schema.clone(),
        cats,
        schms,
        nones.clone(),
        nones.clone(),
        nones,
    )?;
    install_catalog_batch(inner, flat_batch, schema)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TableTypeFilter {
    All,
    Explicit(Vec<String>),
    /// Caller supplied table-type keywords, but none were TABLE or VIEW.
    Unsupported,
}

fn normalize_table_types(types: &[String]) -> TableTypeFilter {
    if types.is_empty() {
        return TableTypeFilter::All;
    }
    if types.len() == 1 && types[0].trim() == "%" {
        return TableTypeFilter::All;
    }
    let normalized: Vec<String> = types
        .iter()
        .map(|t| t.trim().to_uppercase())
        .filter(|t| t == "TABLE" || t == "VIEW")
        .collect();

    if normalized.is_empty() {
        TableTypeFilter::Unsupported
    } else {
        TableTypeFilter::Explicit(normalized)
    }
}

/// Map a `SHOW OBJECTS` `kind` value to an ODBC `TABLE_TYPE`.
///
/// `SHOW OBJECTS.kind` is documented to return `TABLE`, `VIEW`, or
/// `PERSONAL DATABASE`; the finer table variants (transient/iceberg/…) come
/// from `SHOW TABLES`/`SHOW VIEWS` and are kept here defensively. Values that
/// are not a table or view (e.g. `PERSONAL DATABASE`, surfaced only under an
/// account-wide scan by a privileged role) return `None` so the caller skips
/// the row rather than reporting a non-table object as a queryable `TABLE`.
fn normalize_kind(kind: &str) -> Option<&'static str> {
    match kind.to_uppercase().as_str() {
        "TABLE" | "TRANSIENT TABLE" | "TEMPORARY TABLE" | "EXTERNAL TABLE" | "ICEBERG TABLE"
        | "EVENT TABLE" | "HYBRID TABLE" | "MATERIALIZED TABLE" => Some("TABLE"),
        "VIEW" | "MATERIALIZED VIEW" | "SECURE VIEW" => Some("VIEW"),
        "PERSONAL DATABASE" => None,
        // Unknown kinds default to TABLE (matches the prior permissive behavior
        // for table-like objects not yet enumerated above). Log so an
        // unexpected/new SHOW kind is diagnosable rather than silently
        // mislabeled as a TABLE to the application.
        _ => {
            tracing::debug!(
                kind = %kind,
                "SQLTables: unrecognized SHOW OBJECTS kind; defaulting TABLE_TYPE to TABLE"
            );
            Some("TABLE")
        }
    }
}

/// Returns `Some(literal)` when `pattern` has no unescaped `%`/`_`.
///
/// Recognized escapes match [`strip_escapes_for_show_like`]: `\%`, `\_`, `\\`,
/// and `\"`. Treating `\"` as a literal quote (rather than keeping the
/// backslash) keeps SHOW scope (`IN DATABASE "…"`) aligned with the coarse
/// LIKE pushdown built from the same pattern.
fn is_exact_pattern(pattern: &str) -> Option<String> {
    if pattern.is_empty() {
        return Some(String::new());
    }
    let mut result = String::with_capacity(pattern.len());
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ESCAPE_CHAR {
            match chars.next() {
                Some('%') => result.push('%'),
                Some('_') => result.push('_'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some(other) => {
                    result.push(ESCAPE_CHAR);
                    result.push(other);
                }
                None => result.push(ESCAPE_CHAR),
            }
        } else if c == '%' || c == '_' {
            return None;
        } else {
            result.push(c);
        }
    }
    Some(result)
}

fn strip_escapes_for_show_like(pattern: &str) -> String {
    let mut result = String::with_capacity(pattern.len());
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ESCAPE_CHAR {
            match chars.peek().copied() {
                Some(escaped @ ('%' | '_' | '\\' | '"')) => {
                    chars.next();
                    result.push(escaped);
                }
                _ => result.push(ESCAPE_CHAR),
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Escape a pattern for embedding in a single-quoted Snowflake `LIKE '…'`
/// literal. Delegates to [`escape_sql_string_literal`] (doubles `\` then `'`)
/// so SHOW LIKE pushdown uses the same SQL-string convention as the procedures
/// path rather than a parallel `\'`-style escape.
fn escape_show_like(pattern: &str) -> String {
    escape_sql_string_literal(pattern)
}

fn build_like_clause(pattern: Option<&str>) -> String {
    match pattern {
        None | Some("") => String::new(),
        Some(p) => {
            let coarse = strip_escapes_for_show_like(p);
            format!("LIKE '{}'", escape_show_like(&coarse))
        }
    }
}

fn format_show_sql(show_cmd: &str, like_clause: &str, scope: &str) -> String {
    if like_clause.is_empty() {
        format!("{show_cmd} {scope}")
    } else {
        format!("{show_cmd} {like_clause} {scope}")
    }
}

fn build_show_objects_sql(
    catalog: Option<&str>,
    schema: Option<&str>,
    table_name: Option<&str>,
) -> String {
    let exact_catalog = catalog.and_then(is_exact_pattern).filter(|s| !s.is_empty());
    let exact_schema = schema.and_then(is_exact_pattern).filter(|s| !s.is_empty());

    let like_clause = build_like_clause(table_name);
    let scope = match (&exact_catalog, &exact_schema) {
        (Some(cat), Some(sch)) => {
            format!(
                "IN SCHEMA \"{}\".\"{}\"",
                escape_snowflake_identifier(cat),
                escape_snowflake_identifier(sch)
            )
        }
        (Some(cat), None) => {
            format!("IN DATABASE \"{}\"", escape_snowflake_identifier(cat))
        }
        _ => "IN ACCOUNT".to_string(),
    };
    format_show_sql("SHOW OBJECTS", &like_clause, &scope)
}

/// Optional pattern filter: `None` matches all; otherwise case-sensitive LIKE.
fn pattern_matches(pattern: Option<&str>, value: &str) -> bool {
    match pattern {
        None => true,
        Some(p) => like_match(p, value),
    }
}

fn execute_show_tables(
    inner: &mut StatementInner,
    stmt_handle: StatementHandle,
    catalog: Option<String>,
    schema: Option<String>,
    table_name: Option<String>,
    table_types: Vec<String>,
) -> OdbcResult<()> {
    let type_filter = normalize_table_types(&table_types);
    if matches!(type_filter, TableTypeFilter::Unsupported) {
        return set_static_empty_catalog_result(inner, flat_tables_schema());
    }
    // An empty-string catalog, schema, or table pattern matches nothing
    // (`like_match("", non-empty)` is false), so the result is provably empty.
    // Short-circuit before issuing the SHOW — otherwise an empty catalog/schema
    // falls through `build_show_objects_sql` to an account-wide `IN ACCOUNT`
    // scan whose rows are all filtered out client-side anyway.
    if matches!(catalog.as_deref(), Some(""))
        || matches!(schema.as_deref(), Some(""))
        || matches!(table_name.as_deref(), Some(""))
    {
        return set_static_empty_catalog_result(inner, flat_tables_schema());
    }

    let sql = build_show_objects_sql(catalog.as_deref(), schema.as_deref(), table_name.as_deref());
    let show_batch = match execute_show_query_collect_batch(stmt_handle, &sql) {
        Ok(batch) => batch,
        Err(e) => {
            if e.server_sql_state()
                .is_some_and(is_tables_empty_result_sql_state)
            {
                return set_static_empty_catalog_result(inner, flat_tables_schema());
            }
            return Err(e);
        }
    };

    let flat_batch = map_show_objects_to_odbc(
        show_batch,
        catalog.as_deref(),
        schema.as_deref(),
        table_name.as_deref(),
        &type_filter,
    )?;
    let schema = flat_batch.schema();
    install_catalog_batch(inner, flat_batch, schema)
}

fn map_show_objects_to_odbc(
    batch: RecordBatch,
    catalog_filter: Option<&str>,
    schema_filter: Option<&str>,
    table_name_filter: Option<&str>,
    table_types: &TableTypeFilter,
) -> OdbcResult<RecordBatch> {
    let out_schema = flat_tables_schema();
    if batch.num_rows() == 0 {
        return Ok(RecordBatch::new_empty(out_schema));
    }

    let input = batch.schema();
    let db_idx = column_index_by_name(&input, "database_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW OBJECTS result missing 'database_name' column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let sch_idx = column_index_by_name(&input, "schema_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW OBJECTS result missing 'schema_name' column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let name_idx = column_index_by_name(&input, "name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW OBJECTS result missing 'name' column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let kind_idx = column_index_by_name(&input, "kind");

    let mut rows: Vec<FlatTableRow> = Vec::new();
    for row in 0..batch.num_rows() {
        let Some(db_name) = utf8_value_at(&batch, db_idx, row) else {
            continue;
        };
        let Some(sch_name) = utf8_value_at(&batch, sch_idx, row) else {
            continue;
        };
        let Some(tbl_name) = utf8_value_at(&batch, name_idx, row) else {
            continue;
        };
        let kind = kind_idx
            .and_then(|i| utf8_value_at(&batch, i, row))
            .unwrap_or_else(|| "TABLE".to_string());
        // Skip objects that are neither a TABLE nor a VIEW (e.g. PERSONAL DATABASE).
        let Some(normalized_type) = normalize_kind(&kind).map(str::to_string) else {
            continue;
        };

        if !pattern_matches(catalog_filter, &db_name) {
            continue;
        }
        if !pattern_matches(schema_filter, &sch_name) {
            continue;
        }
        if !pattern_matches(table_name_filter, &tbl_name) {
            continue;
        }
        if let TableTypeFilter::Explicit(allowed) = table_types
            && !allowed.contains(&normalized_type)
        {
            continue;
        }

        rows.push((
            Some(db_name),
            Some(sch_name),
            Some(tbl_name),
            Some(normalized_type),
            Some(String::new()),
        ));
    }

    rows.sort_by(|a, b| {
        let ta = a.3.as_deref().unwrap_or("");
        let tb = b.3.as_deref().unwrap_or("");
        let ca = a.0.as_deref().unwrap_or("");
        let cb = b.0.as_deref().unwrap_or("");
        let sa = a.1.as_deref().unwrap_or("");
        let sb = b.1.as_deref().unwrap_or("");
        let na = a.2.as_deref().unwrap_or("");
        let nb = b.2.as_deref().unwrap_or("");
        ta.cmp(tb)
            .then(ca.cmp(cb))
            .then(sa.cmp(sb))
            .then(na.cmp(nb))
    });

    let mut cats = Vec::with_capacity(rows.len());
    let mut schms = Vec::with_capacity(rows.len());
    let mut tbls = Vec::with_capacity(rows.len());
    let mut types = Vec::with_capacity(rows.len());
    let mut remarks = Vec::with_capacity(rows.len());
    for (c, s, t, ty, r) in rows {
        cats.push(c);
        schms.push(s);
        tbls.push(t);
        types.push(ty);
        remarks.push(r);
    }
    build_flat_batch(out_schema, cats, schms, tbls, types, remarks)
}

fn install_catalog_batch(
    inner: &mut StatementInner,
    flat_batch: RecordBatch,
    schema: SchemaRef,
) -> OdbcResult<()> {
    let flat_reader = reader_from_record_batch(flat_batch, schema)?;
    set_state_for_catalog(
        inner,
        StatementState::QueryExecuted {
            reader: flat_reader,
            rows_affected: Some(-1),
            origin: ExecutionOrigin::Direct,
        },
    );
    Ok(())
}

fn build_show_columns_sql(
    catalog: Option<&str>,
    schema: Option<&str>,
    table_name: Option<&str>,
    column_name: Option<&str>,
) -> String {
    let exact_catalog = catalog.and_then(is_exact_pattern).filter(|s| !s.is_empty());
    let exact_schema = schema.and_then(is_exact_pattern).filter(|s| !s.is_empty());
    let exact_table = table_name
        .and_then(is_exact_pattern)
        .filter(|s| !s.is_empty());

    let like_clause = build_like_clause(column_name);
    let scope = match (&exact_catalog, &exact_schema, &exact_table) {
        (Some(cat), Some(sch), Some(tbl)) => format!(
            "IN TABLE \"{}\".\"{}\".\"{}\"",
            escape_snowflake_identifier(cat),
            escape_snowflake_identifier(sch),
            escape_snowflake_identifier(tbl)
        ),
        (Some(cat), Some(sch), None) => format!(
            "IN SCHEMA \"{}\".\"{}\"",
            escape_snowflake_identifier(cat),
            escape_snowflake_identifier(sch)
        ),
        (Some(cat), None, _) => {
            format!("IN DATABASE \"{}\"", escape_snowflake_identifier(cat))
        }
        _ => "IN ACCOUNT".to_string(),
    };
    format_show_sql("SHOW COLUMNS", &like_clause, &scope)
}

fn execute_show_columns(
    inner: &mut StatementInner,
    stmt_handle: StatementHandle,
    numeric_settings: NumericSettings,
    catalog: Option<String>,
    schema: Option<String>,
    table_name: Option<String>,
    column_name: Option<String>,
) -> OdbcResult<()> {
    // Empty string means "match nothing" (like_match("", non-empty) is false).
    if matches!(table_name.as_deref(), Some("")) || matches!(column_name.as_deref(), Some("")) {
        return set_static_empty_catalog_result(inner, flat_columns_schema());
    }

    let sql = build_show_columns_sql(
        catalog.as_deref(),
        schema.as_deref(),
        table_name.as_deref(),
        column_name.as_deref(),
    );
    let show_batch = match execute_show_query_collect_batch(stmt_handle, &sql) {
        Ok(batch) => batch,
        Err(e) => {
            if e.server_sql_state()
                .is_some_and(is_tables_empty_result_sql_state)
            {
                return set_static_empty_catalog_result(inner, flat_columns_schema());
            }
            return Err(e);
        }
    };

    let flat_batch = map_show_columns_to_odbc(
        show_batch,
        &numeric_settings,
        catalog.as_deref(),
        schema.as_deref(),
        table_name.as_deref(),
        column_name.as_deref(),
    )?;
    let schema = flat_batch.schema();
    install_catalog_batch(inner, flat_batch, schema)
}

fn map_show_columns_to_odbc(
    batch: RecordBatch,
    numeric_settings: &NumericSettings,
    catalog_filter: Option<&str>,
    schema_filter: Option<&str>,
    table_name_filter: Option<&str>,
    column_name_filter: Option<&str>,
) -> OdbcResult<RecordBatch> {
    let out_schema = flat_columns_schema();
    if batch.num_rows() == 0 {
        return build_flat_columns_batch(out_schema, vec![]);
    }

    let input = batch.schema();
    let db_idx = column_index_by_name(&input, "database_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW COLUMNS result missing 'database_name' column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let sch_idx = column_index_by_name(&input, "schema_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW COLUMNS result missing 'schema_name' column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let tbl_idx = column_index_by_name(&input, "table_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW COLUMNS result missing 'table_name' column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let col_idx = column_index_by_name(&input, "column_name").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW COLUMNS result missing 'column_name' column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let data_type_idx = column_index_by_name(&input, "data_type").ok_or_else(|| {
        crate::api::error::OdbcError::InternalError {
            message: "SHOW COLUMNS result missing 'data_type' column".to_string(),
            location: snafu::location!(),
        }
    })?;
    let default_idx = column_index_by_name(&input, "default");
    let comment_idx = column_index_by_name(&input, "comment");

    // Group by (catalog, schema, table) for ODBC-mandated lexicographic order.
    //
    // ORDINAL_POSITION is assigned 1,2,3… among the columns kept per table, in
    // SHOW COLUMNS order, resetting at each table boundary. This intentionally
    // matches the legacy Snowflake ODBC driver, whose ColumnsMetadataSource
    // pushes `SHOW COLUMNS LIKE '<pattern>'` server-side and increments its
    // ordinal only over the returned (matched) rows. It is a deliberate
    // deviation from the strict ODBC spec ("ordinal position of the column in
    // the table"): a ColumnName filter renumbers the survivors rather than
    // reporting their absolute table position. Kept for old-driver parity —
    // do not switch to absolute position without a BehaviorDifferences entry.
    let mut by_cat_sch_tbl: BTreeMap<
        String,
        BTreeMap<String, BTreeMap<String, Vec<FlatColumnRow>>>,
    > = BTreeMap::new();

    for row in 0..batch.num_rows() {
        let Some(db_name) = utf8_value_at(&batch, db_idx, row) else {
            continue;
        };
        let Some(sch_name) = utf8_value_at(&batch, sch_idx, row) else {
            continue;
        };
        let Some(tbl_name) = utf8_value_at(&batch, tbl_idx, row) else {
            continue;
        };
        let Some(col_name) = utf8_value_at(&batch, col_idx, row) else {
            continue;
        };
        let data_type_json =
            utf8_value_at(&batch, data_type_idx, row).unwrap_or_else(|| "{}".to_string());
        let column_def = default_idx
            .and_then(|i| utf8_value_at(&batch, i, row))
            .filter(|s| !s.is_empty());
        let remarks = comment_idx
            .and_then(|i| utf8_value_at(&batch, i, row))
            .filter(|s| !s.is_empty());

        if !pattern_matches(catalog_filter, &db_name) {
            continue;
        }
        if !pattern_matches(schema_filter, &sch_name) {
            continue;
        }
        if !pattern_matches(table_name_filter, &tbl_name) {
            continue;
        }
        if !pattern_matches(column_name_filter, &col_name) {
            continue;
        }

        let table_cols = by_cat_sch_tbl
            .entry(db_name.clone())
            .or_default()
            .entry(sch_name.clone())
            .or_default()
            .entry(tbl_name.clone())
            .or_default();
        let ordinal_position = (table_cols.len() + 1) as i32;
        let descriptor = decode_data_type_json(
            &data_type_json,
            col_name,
            ordinal_position,
            column_def,
            remarks,
        );
        table_cols.push(flat_row_from_descriptor(
            db_name,
            sch_name,
            tbl_name,
            &descriptor,
            numeric_settings,
        ));
    }

    let mut rows: Vec<FlatColumnRow> = Vec::new();
    for schemas in by_cat_sch_tbl.into_values() {
        for tables in schemas.into_values() {
            for cols in tables.into_values() {
                rows.extend(cols);
            }
        }
    }
    build_flat_columns_batch(out_schema, rows)
}

// ============================================================================
// SQLColumns type mapping (SHOW COLUMNS JSON → flat 19-col ODBC row)
// ============================================================================

/// Parsed representation of the `data_type` JSON blob from `SHOW COLUMNS`.
/// Only the fields needed to reconstruct the ODBC type metadata are captured;
/// unknown fields are ignored (serde default).
///
/// Keep in sync with the same-named struct in
/// `sf_core::apis::database_driver_v1::get_objects`.
#[derive(Debug, serde::Deserialize)]
struct ShowColumnDataType {
    #[serde(rename = "type")]
    type_: String,
    nullable: Option<bool>,
    precision: Option<i64>,
    scale: Option<i64>,
    #[serde(rename = "byteLength")]
    byte_length: Option<i64>,
    /// charLength (TEXT) — present in newer Snowflake responses.
    #[serde(rename = "charLength")]
    char_length: Option<i64>,
    /// length (TEXT) — legacy alias for charLength; used when charLength absent.
    length: Option<i64>,
}

/// Decoded column descriptor: a single `SHOW COLUMNS` row with the
/// Snowflake-wire `data_type` JSON fully parsed. Owned by the ODBC wrapper so
/// the SQLColumns path does not depend on `sf_core`'s GetObjects internals.
///
/// Keep in sync with the same-named type in
/// `sf_core::apis::database_driver_v1::get_objects`.
#[derive(Debug)]
struct ColumnDescriptor {
    column_name: String,
    ordinal_position: i32,
    logical_type: String,
    precision: Option<i32>,
    scale: Option<i32>,
    char_length: Option<i64>,
    byte_length: Option<i64>,
    nullable: bool,
    column_def: Option<String>,
    remarks: Option<String>,
}

/// Decode the `data_type` JSON blob from `SHOW COLUMNS` into a
/// [`ColumnDescriptor`].
///
/// Keep in sync with `sf_core::apis::database_driver_v1::get_objects::decode_data_type_json`
/// (and its companion `ShowColumnDataType` / `ColumnDescriptor`). Copied into the
/// ODBC wrapper so SQLColumns does not reach into GetObjects internals; prune the
/// core copy once no other consumer needs it.
///
/// Three design points:
///
/// 1. **TEXT char length** — prefers `charLength` (present in modern Snowflake
///    responses); falls back to `length`, which is a legacy alias for the same
///    value used by older server versions. Non-TEXT types intentionally ignore
///    `length` because it carries different semantics there (e.g. byte-length
///    for BINARY vs. char-length for TEXT).
///
/// 2. **Parse failure** — on any JSON parse error (malformed blob, schema
///    change, exotic type not yet in the struct) the function returns
///    `logical_type = "UNKNOWN"` and `nullable = true` rather than propagating
///    an error. This lets the wrapper fall back gracefully for exotic types
///    (INTERVAL, VECTOR, GEOGRAPHY, GEOMETRY) whose `data_type` blobs may not
///    match the fields captured here; the column maps to NULL in the ODBC
///    output rather than poisoning the whole result set.
///
/// 3. **Precision/scale narrowing** — the JSON blob carries i64 values; we
///    narrow to i32 for the descriptor. Snowflake's precision is at most 38
///    (FIXED) and scale at most 37, both well within `i32::MAX`, so this is
///    always in range in practice. A value that nonetheless overflows i32
///    degrades to `None` (unknown) via a checked conversion rather than
///    wrapping to a garbage number — handled in all builds, not just dev.
fn decode_data_type_json(
    json: &str,
    column_name: String,
    ordinal_position: i32,
    column_def: Option<String>,
    remarks: Option<String>,
) -> ColumnDescriptor {
    let dt: ShowColumnDataType = match serde_json::from_str(json) {
        Ok(v) => v,
        // See design point 2 above: preserve the column in an "unknown" state
        // rather than dropping it or surfacing an error to the ODBC caller.
        Err(_) => {
            return ColumnDescriptor {
                column_name,
                ordinal_position,
                logical_type: "UNKNOWN".to_string(),
                precision: None,
                scale: None,
                char_length: None,
                byte_length: None,
                nullable: true,
                column_def,
                remarks,
            };
        }
    };

    // Design point 1: prefer charLength; fall back to length for TEXT only.
    let char_length = dt.char_length.or_else(|| {
        dt.type_
            .eq_ignore_ascii_case("TEXT")
            .then_some(dt.length)
            .flatten()
    });

    ColumnDescriptor {
        column_name,
        ordinal_position,
        logical_type: dt.type_,
        // Design point 3: checked i64→i32 narrowing; overflow degrades to None
        // (unknown) in every build rather than wrapping to garbage.
        precision: dt.precision.and_then(|p| i32::try_from(p).ok()),
        scale: dt.scale.and_then(|s| i32::try_from(s).ok()),
        char_length,
        byte_length: dt.byte_length,
        nullable: dt.nullable.unwrap_or(true),
        column_def,
        remarks,
    }
}

/// Rehydrate an Arrow `Field` from the canonical column struct metadata so
/// the existing `*_from_field` helpers (which read `logicalType`, `precision`,
/// `scale`, `charLength`, `byteLength` from field metadata) work without any
/// JSON parsing here in the wrapper.
fn rehydrate_field(
    logical_type: &str,
    precision: Option<i32>,
    scale: Option<i32>,
    char_length: Option<i64>,
    byte_length: Option<i64>,
    nullable: bool,
) -> Field {
    let mut meta = HashMap::new();
    meta.insert("logicalType".to_string(), logical_type.to_string());
    if let Some(p) = precision {
        meta.insert("precision".to_string(), p.to_string());
    }
    if let Some(s) = scale {
        meta.insert("scale".to_string(), s.to_string());
    }
    if let Some(cl) = char_length {
        meta.insert("charLength".to_string(), cl.to_string());
    }
    if let Some(bl) = byte_length {
        meta.insert("byteLength".to_string(), bl.to_string());
    }
    // DataType::Utf8 is a placeholder; *_from_field reads metadata, not the Arrow
    // data type, for type classification.
    Field::new("col", DataType::Utf8, nullable).with_metadata(meta)
}

/// Maps a Snowflake logical type to the external / friendly name that
/// `SQLColumns` column 6 (`TYPE_NAME`) must report, mirroring the reference
/// driver's `ColumnMetadata::deriveODBCTypeInfo` / `getExtTypeName` (default
/// ODBC-v3 path, no custom SQL data types).
///
/// This is deliberately **separate** from `SnowflakeFieldType::type_name`
/// (the `SQLColAttribute(SQL_DESC_TYPE_NAME)` path on query columns), which
/// returns SDK-style labels (`BIT`, `TYPE_DATE`, `TYPE_TIMESTAMP`, …) and must
/// not change. The reference driver depends on the external names here; routing
/// SQLColumns through the ColAttribute helper collapsed semi-structured types
/// to `VARCHAR` and turned `GEOGRAPHY` into `NULL`. See SNOW-3899531.
fn catalog_type_name_from_logical_type(logical_type: &str) -> String {
    match logical_type {
        "TEXT" => "VARCHAR",
        "FIXED" => "DECIMAL",
        "REAL" => "DOUBLE",
        "BINARY" => "BINARY",
        "BOOLEAN" => "BOOLEAN",
        "DATE" => "DATE",
        "TIME" => "TIME",
        "TIMESTAMP_NTZ" | "TIMESTAMP_LTZ" | "TIMESTAMP_TZ" | "TIMESTAMP" => "TIMESTAMP",
        "VARIANT" => "VARIANT",
        "ARRAY" => "ARRAY",
        "OBJECT" => "STRUCT",
        "GEOGRAPHY" => "GEOGRAPHY",
        "GEOMETRY" => "GEOMETRY",
        // Absent logical type → safe VARCHAR fallback; any other exotic/unknown
        // type → its uppercased logical name (matching OLD's else branch).
        "" => "VARCHAR",
        other => return other.to_ascii_uppercase(),
    }
    .to_string()
}

/// Returns the datetime subcode for `SQL_DATETIME_SUB` (col 15), or `None`
/// for types where `SQL_DATA_TYPE != SQL_DATETIME`.
fn sql_datetime_sub_from_logical_type(logical_type: &str) -> Option<i16> {
    match logical_type {
        "DATE" => Some(1),                                             // SQL_CODE_DATE
        "TIME" => Some(2),                                             // SQL_CODE_TIME
        "TIMESTAMP_NTZ" | "TIMESTAMP_LTZ" | "TIMESTAMP_TZ" => Some(3), // SQL_CODE_TIMESTAMP
        _ => None,
    }
}

/// One output row of the 19-column `SQLColumns` result set.
/// Named fields prevent change-amplification when columns are added or
/// reordered (no parallel-Vec zip chains, no nested tuple destructuring).
struct FlatColumnRow {
    cat: Option<String>,
    schem: Option<String>,
    tbl: Option<String>,
    col_name: Option<String>,
    data_type: Option<i16>,
    type_name: Option<String>,
    col_size: Option<i32>,
    buf_len: Option<i32>,
    dec_digits: Option<i16>,
    num_prec_radix: Option<i32>,
    nullable: Option<i16>,
    remarks: Option<String>,
    col_def: Option<String>,
    sql_data_type: Option<i16>,
    sql_dt_sub: Option<i16>,
    char_octet: Option<i32>,
    ordinal: Option<i32>,
    is_nullable: Option<String>,
    user_data_type: Option<i16>,
}

fn flat_row_from_descriptor(
    cat: String,
    schem: String,
    tbl: String,
    desc: &ColumnDescriptor,
    numeric_settings: &NumericSettings,
) -> FlatColumnRow {
    let field = rehydrate_field(
        &desc.logical_type,
        desc.precision,
        desc.scale,
        desc.char_length,
        desc.byte_length,
        desc.nullable,
    );

    let data_type_val = sql_type_from_field(&field, numeric_settings)
        .ok()
        .map(|t| t.0);
    // TYPE_NAME (col 6) reports the Snowflake external / friendly name
    // (BOOLEAN, TIMESTAMP, VARIANT, STRUCT, ARRAY, GEOGRAPHY, …) — NOT the SDK
    // label from `type_name_from_field`, which is the
    // SQLColAttribute(SQL_DESC_TYPE_NAME) contract. SNOW-3899531.
    let type_name_val = Some(catalog_type_name_from_logical_type(&desc.logical_type));
    let col_size_val = column_size_from_field(&field, numeric_settings)
        .ok()
        .and_then(|s| i32::try_from(s).ok());
    let buf_len_val = octet_length_from_field(&field, numeric_settings)
        .ok()
        .and_then(|s| i32::try_from(s).ok());
    // DECIMAL_DIGITS: scale 0 is a valid, meaningful value for exact-numeric
    // columns (e.g. NUMBER(38,0)) — report it as 0, not NULL. The helper
    // returns Err for types where DECIMAL_DIGITS is inapplicable (→ NULL).
    let dec_digits_val = decimal_digits_from_field(&field, numeric_settings).ok();
    // NUM_PREC_RADIX: only ever 2, 10, or inapplicable (→ NULL); 0 is never
    // a meaningful value, so collapsing 0 → NULL is harmless here.
    let num_prec_radix_val = num_prec_radix_from_field(&field, numeric_settings)
        .ok()
        .and_then(|s| if s == 0 { None } else { i32::try_from(s).ok() });
    let sql_data_type_val = verbose_sql_type_from_field(&field, numeric_settings)
        .ok()
        .map(|t| t.0);
    let sql_dt_sub_val = sql_datetime_sub_from_logical_type(&desc.logical_type);
    let char_octet_val = match desc.logical_type.as_str() {
        "TEXT" | "BINARY" => octet_length_from_field(&field, numeric_settings)
            .ok()
            .and_then(|s| i32::try_from(s).ok()),
        _ => None,
    };

    let nullable_val: i16 = if desc.nullable { 1 } else { 0 };
    let is_nullable_str = if desc.nullable { "YES" } else { "NO" };
    // USER_DATA_TYPE: mirror DATA_TYPE (driver-specific; cell-value semantics
    // tracked separately in SNOW-3899721).
    let user_data_type_val = data_type_val;

    FlatColumnRow {
        cat: Some(cat),
        schem: Some(schem),
        tbl: Some(tbl),
        col_name: Some(desc.column_name.clone()),
        data_type: data_type_val,
        type_name: type_name_val,
        col_size: col_size_val,
        buf_len: buf_len_val,
        dec_digits: dec_digits_val,
        num_prec_radix: num_prec_radix_val,
        nullable: Some(nullable_val),
        remarks: desc.remarks.clone(),
        col_def: desc.column_def.clone(),
        sql_data_type: sql_data_type_val,
        sql_dt_sub: sql_dt_sub_val,
        char_octet: char_octet_val,
        ordinal: Some(desc.ordinal_position),
        is_nullable: Some(is_nullable_str.to_string()),
        user_data_type: user_data_type_val,
    }
}

fn build_flat_columns_batch(
    schema: SchemaRef,
    rows: Vec<FlatColumnRow>,
) -> OdbcResult<RecordBatch> {
    fn str_col(v: Vec<Option<String>>) -> ArrayRef {
        Arc::new(StringArray::from(v)) as ArrayRef
    }
    fn i16_col(v: Vec<Option<i16>>) -> ArrayRef {
        Arc::new(Int16Array::from(v)) as ArrayRef
    }
    fn i32_col(v: Vec<Option<i32>>) -> ArrayRef {
        Arc::new(Int32Array::from(v)) as ArrayRef
    }
    let n = rows.len();
    let mut cats = Vec::with_capacity(n);
    let mut schms = Vec::with_capacity(n);
    let mut tbls = Vec::with_capacity(n);
    let mut col_names = Vec::with_capacity(n);
    let mut data_types = Vec::with_capacity(n);
    let mut type_names = Vec::with_capacity(n);
    let mut col_sizes = Vec::with_capacity(n);
    let mut buf_lens = Vec::with_capacity(n);
    let mut dec_digits = Vec::with_capacity(n);
    let mut num_prec_radixes = Vec::with_capacity(n);
    let mut nullables = Vec::with_capacity(n);
    let mut remarks = Vec::with_capacity(n);
    let mut col_defs = Vec::with_capacity(n);
    let mut sql_data_types = Vec::with_capacity(n);
    let mut sql_dt_subs = Vec::with_capacity(n);
    let mut char_octets = Vec::with_capacity(n);
    let mut ordinals = Vec::with_capacity(n);
    let mut is_nullables = Vec::with_capacity(n);
    let mut user_data_types = Vec::with_capacity(n);
    for r in rows {
        cats.push(r.cat);
        schms.push(r.schem);
        tbls.push(r.tbl);
        col_names.push(r.col_name);
        data_types.push(r.data_type);
        type_names.push(r.type_name);
        col_sizes.push(r.col_size);
        buf_lens.push(r.buf_len);
        dec_digits.push(r.dec_digits);
        num_prec_radixes.push(r.num_prec_radix);
        nullables.push(r.nullable);
        remarks.push(r.remarks);
        col_defs.push(r.col_def);
        sql_data_types.push(r.sql_data_type);
        sql_dt_subs.push(r.sql_dt_sub);
        char_octets.push(r.char_octet);
        ordinals.push(r.ordinal);
        is_nullables.push(r.is_nullable);
        user_data_types.push(r.user_data_type);
    }
    RecordBatch::try_new(
        schema,
        vec![
            str_col(cats),
            str_col(schms),
            str_col(tbls),
            str_col(col_names),
            i16_col(data_types),
            str_col(type_names),
            i32_col(col_sizes),
            i32_col(buf_lens),
            i16_col(dec_digits),
            i32_col(num_prec_radixes),
            i16_col(nullables),
            str_col(remarks),
            str_col(col_defs),
            i16_col(sql_data_types),
            i16_col(sql_dt_subs),
            i32_col(char_octets),
            i32_col(ordinals),
            str_col(is_nullables),
            i16_col(user_data_types),
        ],
    )
    .context(crate::api::error::RecordBatchBuildSnafu)
}

/// Wrap a RecordBatch in an ArrowArrayStreamReader.
fn reader_from_record_batch(
    batch: RecordBatch,
    schema: SchemaRef,
) -> OdbcResult<ArrowArrayStreamReader> {
    use arrow::record_batch::RecordBatchIterator;
    let iter = RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema);
    let ffi_stream = FFI_ArrowArrayStream::new(
        Box::new(iter) as Box<dyn arrow::array::RecordBatchReader + Send>
    );
    // `try_new` takes the stream by value; pass it directly. A
    // `Box::into_raw` + `from_raw` round-trip would leak the heap box, since
    // `from_raw` moves the struct out without reclaiming the allocation.
    ArrowArrayStreamReader::try_new(ffi_stream).context(ArrowArrayStreamReaderCreationSnafu)
}

fn build_flat_batch(
    schema: SchemaRef,
    cats: Vec<Option<String>>,
    schms: Vec<Option<String>>,
    tbls: Vec<Option<String>>,
    types: Vec<Option<String>>,
    remarks: Vec<Option<String>>,
) -> OdbcResult<RecordBatch> {
    fn to_array(v: Vec<Option<String>>) -> ArrayRef {
        Arc::new(StringArray::from(v)) as ArrayRef
    }

    RecordBatch::try_new(
        schema,
        vec![
            to_array(cats),
            to_array(schms),
            to_array(tbls),
            to_array(types),
            to_array(remarks),
        ],
    )
    .context(crate::api::error::RecordBatchBuildSnafu)
}

// ============================================================================
// Static TABLE_TYPES result (SQL_ALL_TABLE_TYPES special case)
// ============================================================================

fn set_static_table_types(inner: &mut StatementInner) -> OdbcResult<()> {
    let schema = flat_tables_schema();
    let types = vec![Some("TABLE".to_string()), Some("VIEW".to_string())];
    let nones: Vec<Option<String>> = vec![None, None];

    let flat_batch = build_flat_batch(
        schema.clone(),
        nones.clone(),
        nones.clone(),
        nones.clone(),
        types,
        nones,
    )?;

    let flat_reader = reader_from_record_batch(flat_batch, schema)?;

    set_state_for_catalog(
        inner,
        StatementState::QueryExecuted {
            reader: flat_reader,
            rows_affected: Some(-1),
            origin: ExecutionOrigin::Direct,
        },
    );
    Ok(())
}

// ============================================================================
// TableType value list parsing
// ============================================================================

fn parse_table_type_list(types: Option<&str>) -> Vec<String> {
    match types {
        None | Some("") => vec![],
        Some(s) => s
            .split(',')
            .map(|t| t.trim().trim_matches('\'').to_uppercase())
            .filter(|t| !t.is_empty())
            .collect(),
    }
}

// ============================================================================
// Shared helpers for empty-result catalog functions
// ============================================================================

fn validate_catalog_stmt_ready(inner: &StatementInner) -> OdbcResult<()> {
    if inner.state.as_ref().is_need_data() {
        return InvalidDuringDaeSnafu.fail();
    }
    if inner.state.as_ref().is_async_executing() {
        return AsyncInProgressSnafu.fail();
    }
    if inner.state.as_ref().has_open_cursor() {
        return CursorAlreadyOpenSnafu.fail();
    }
    Ok(())
}

fn set_static_empty_catalog_result(
    inner: &mut StatementInner,
    schema: SchemaRef,
) -> OdbcResult<()> {
    let batch = RecordBatch::new_empty(schema.clone());
    let reader = reader_from_record_batch(batch, schema)?;
    set_state_for_catalog(
        inner,
        StatementState::QueryExecuted {
            reader,
            rows_affected: Some(-1),
            origin: ExecutionOrigin::Direct,
        },
    );
    Ok(())
}

// ============================================================================
// SQLSpecialColumns — empty result set (Snowflake has no row identifiers)
// ============================================================================

fn special_columns_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        catalog_smallint_field("SCOPE"),
        catalog_text_field("COLUMN_NAME", 128),
        catalog_smallint_field("DATA_TYPE"),
        catalog_text_field("TYPE_NAME", 128),
        catalog_int_field("COLUMN_SIZE"),
        catalog_int_field("BUFFER_LENGTH"),
        catalog_smallint_field("DECIMAL_DIGITS"),
        catalog_smallint_field("PSEUDO_COLUMN"),
    ]))
}

#[allow(clippy::too_many_arguments)]
pub fn special_columns<E: OdbcEncoding>(
    statement_handle: sql::Handle,
    _identifier_type: sql::SmallInt,
    catalog_name: *const E::Char,
    name_length1: sql::SmallInt,
    schema_name: *const E::Char,
    name_length2: sql::SmallInt,
    table_name: *const E::Char,
    name_length3: sql::SmallInt,
    _scope: sql::SmallInt,
    _nullable: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("SQLSpecialColumns called");

    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let conn = dbc.connection.lock();
    let mut inner = guard.inner.lock();

    validate_catalog_stmt_ready(&inner)?;

    match &conn.state {
        ConnectionState::Connected { .. } => {}
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
    }
    drop(conn);

    let _ = read_opt_str::<E>(catalog_name, name_length1)?;
    let _ = read_opt_str::<E>(schema_name, name_length2)?;
    let table = read_opt_str::<E>(table_name, name_length3)?;

    if table.is_none() {
        return NullPointerSnafu.fail();
    }

    set_static_empty_catalog_result(&mut inner, special_columns_schema())
}

// ============================================================================
// SQLColumnPrivileges — empty result set (no column-level privileges in SF)
// ============================================================================

fn column_privileges_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        catalog_text_field("TABLE_CAT", 255),
        catalog_text_field("TABLE_SCHEM", 255),
        catalog_text_field("TABLE_NAME", 255),
        catalog_text_field("COLUMN_NAME", 255),
        catalog_text_field("GRANTOR", 255),
        catalog_text_field("GRANTEE", 255),
        catalog_text_field("PRIVILEGE", 255),
        catalog_text_field("IS_GRANTABLE", 3),
    ]))
}

#[allow(clippy::too_many_arguments)]
pub fn column_privileges<E: OdbcEncoding>(
    statement_handle: sql::Handle,
    catalog_name: *const E::Char,
    name_length1: sql::SmallInt,
    schema_name: *const E::Char,
    name_length2: sql::SmallInt,
    table_name: *const E::Char,
    name_length3: sql::SmallInt,
    column_name: *const E::Char,
    name_length4: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("SQLColumnPrivileges called");

    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let conn = dbc.connection.lock();
    let mut inner = guard.inner.lock();

    validate_catalog_stmt_ready(&inner)?;

    match &conn.state {
        ConnectionState::Connected { .. } => {}
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
    }
    drop(conn);

    let _ = read_opt_str::<E>(catalog_name, name_length1)?;
    let _ = read_opt_str::<E>(schema_name, name_length2)?;
    let table = read_opt_str::<E>(table_name, name_length3)?;
    let _ = read_opt_str::<E>(column_name, name_length4)?;

    if table.is_none() {
        return NullPointerSnafu.fail();
    }

    set_static_empty_catalog_result(&mut inner, column_privileges_schema())
}

// ============================================================================
// SQLTablePrivileges — empty result set (no table-level privileges in SF)
// ============================================================================

fn table_privileges_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        catalog_text_field("TABLE_CAT", 255),
        catalog_text_field("TABLE_SCHEM", 255),
        catalog_text_field("TABLE_NAME", 255),
        catalog_text_field("GRANTOR", 255),
        catalog_text_field("GRANTEE", 255),
        catalog_text_field("PRIVILEGE", 255),
        catalog_text_field("IS_GRANTABLE", 3),
    ]))
}

#[allow(clippy::too_many_arguments)]
pub fn table_privileges<E: OdbcEncoding>(
    statement_handle: sql::Handle,
    catalog_name: *const E::Char,
    name_length1: sql::SmallInt,
    schema_name: *const E::Char,
    name_length2: sql::SmallInt,
    table_name: *const E::Char,
    name_length3: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("SQLTablePrivileges called");

    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let conn = dbc.connection.lock();
    let mut inner = guard.inner.lock();

    validate_catalog_stmt_ready(&inner)?;

    match &conn.state {
        ConnectionState::Connected { .. } => {}
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
    }
    drop(conn);

    let _ = read_opt_str::<E>(catalog_name, name_length1)?;
    let _ = read_opt_str::<E>(schema_name, name_length2)?;
    let _ = read_opt_str::<E>(table_name, name_length3)?;

    set_static_empty_catalog_result(&mut inner, table_privileges_schema())
}

// ============================================================================
// SQLStatistics — empty result set (Snowflake has no index statistics)
// ============================================================================

fn statistics_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        catalog_text_field("TABLE_CAT", 255),
        catalog_text_field("TABLE_SCHEM", 255),
        catalog_text_field("TABLE_NAME", 255),
        catalog_smallint_field("NON_UNIQUE"),
        catalog_text_field("INDEX_QUALIFIER", 255),
        catalog_text_field("INDEX_NAME", 255),
        catalog_smallint_field("TYPE"),
        catalog_smallint_field("ORDINAL_POSITION"),
        catalog_text_field("COLUMN_NAME", 255),
        catalog_text_field("ASC_OR_DESC", 1),
        catalog_int_field("CARDINALITY"),
        catalog_int_field("PAGES"),
        catalog_text_field("FILTER_CONDITION", 255),
    ]))
}

#[allow(clippy::too_many_arguments)]
pub fn statistics<E: OdbcEncoding>(
    statement_handle: sql::Handle,
    catalog_name: *const E::Char,
    name_length1: sql::SmallInt,
    schema_name: *const E::Char,
    name_length2: sql::SmallInt,
    table_name: *const E::Char,
    name_length3: sql::SmallInt,
    _unique: sql::SmallInt,
    _reserved: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("SQLStatistics called");

    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let conn = dbc.connection.lock();
    let mut inner = guard.inner.lock();

    validate_catalog_stmt_ready(&inner)?;

    match &conn.state {
        ConnectionState::Connected { .. } => {}
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
    }
    drop(conn);

    let _ = read_opt_str::<E>(catalog_name, name_length1)?;
    let _ = read_opt_str::<E>(schema_name, name_length2)?;
    let table = read_opt_str::<E>(table_name, name_length3)?;

    // TableName is a required (ordinary) argument for SQLStatistics — the ODBC
    // spec forbids a null pointer, so NULL must yield HY009. This check is not a
    // Driver Manager responsibility, so it surfaces to the driver under iODBC.
    if table.is_none() {
        return NullPointerSnafu.fail();
    }

    set_static_empty_catalog_result(&mut inner, statistics_schema())
}

// ============================================================================
// SQLGetTypeInfo — static type table
// ============================================================================

/// Arrow field for a nullable SMALLINT catalog column (FIXED/scale=0/precision=5).
fn catalog_smallint_field(name: &str) -> Field {
    let metadata: std::collections::HashMap<String, String> = [
        ("logicalType".to_string(), "FIXED".to_string()),
        ("scale".to_string(), "0".to_string()),
        ("precision".to_string(), "5".to_string()),
        (
            "conciseSqlType".to_string(),
            SMALLINT_CONCISE_SQL_TYPE.to_string(),
        ),
    ]
    .into();
    Field::new(name, DataType::Int16, true).with_metadata(metadata)
}

/// Arrow field for a nullable INTEGER catalog column (FIXED/scale=0/precision=10).
fn catalog_int_field(name: &str) -> Field {
    let metadata: std::collections::HashMap<String, String> = [
        ("logicalType".to_string(), "FIXED".to_string()),
        ("scale".to_string(), "0".to_string()),
        ("precision".to_string(), "10".to_string()),
        (
            "conciseSqlType".to_string(),
            INTEGER_CONCISE_SQL_TYPE.to_string(),
        ),
    ]
    .into();
    Field::new(name, DataType::Int32, true).with_metadata(metadata)
}

/// 20-column schema for `SQLGetTypeInfo` result sets (19 ODBC 3.x standard
/// columns + driver-specific `USER_DATA_TYPE`).
fn type_info_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        catalog_text_field("TYPE_NAME", 128),
        catalog_smallint_field("DATA_TYPE"),
        catalog_int_field("COLUMN_SIZE"),
        catalog_text_field("LITERAL_PREFIX", 32),
        catalog_text_field("LITERAL_SUFFIX", 32),
        catalog_text_field("CREATE_PARAMS", 128),
        catalog_smallint_field("NULLABLE"),
        catalog_smallint_field("CASE_SENSITIVE"),
        catalog_smallint_field("SEARCHABLE"),
        catalog_smallint_field("UNSIGNED_ATTRIBUTE"),
        catalog_smallint_field("FIXED_PREC_SCALE"),
        catalog_smallint_field("AUTO_UNIQUE_VALUE"),
        catalog_text_field("LOCAL_TYPE_NAME", 128),
        catalog_smallint_field("MINIMUM_SCALE"),
        catalog_smallint_field("MAXIMUM_SCALE"),
        catalog_smallint_field("SQL_DATA_TYPE"),
        catalog_smallint_field("SQL_DATETIME_SUB"),
        catalog_int_field("NUM_PREC_RADIX"),
        catalog_int_field("INTERVAL_PRECISION"),
        catalog_smallint_field("USER_DATA_TYPE"),
    ]))
}

// ODBC searchability constants
const SEARCHABLE: i16 = 3; // SQL_SEARCHABLE
const PRED_BASIC: i16 = 2; // SQL_PRED_BASIC (all comparison ops except LIKE)

// ODBC verbose datetime type and its sub-codes
const SQL_DATETIME: i16 = 9;
const CODE_DATE: i16 = 1; // SQL_CODE_DATE
const CODE_TIME: i16 = 2; // SQL_CODE_TIME
const CODE_TIMESTAMP: i16 = 3; // SQL_CODE_TIMESTAMP

// ODBC ALL_TYPES sentinel — fDataType value meaning "return all types".
const SQL_ALL_TYPES: sql::SmallInt = 0;

/// One row in the `SQLGetTypeInfo` result set.
///
/// `AUTO_UNIQUE_VALUE` and `INTERVAL_PRECISION` are always NULL for all
/// Snowflake types and are omitted from the row struct; `build_type_info_batch`
/// fills those columns with `None` unconditionally.
struct TypeInfoRow {
    type_name: &'static str,
    /// Concise SQL data type code (ODBC DATA_TYPE column).
    data_type: i16,
    column_size: i32,
    literal_prefix: Option<&'static str>,
    literal_suffix: Option<&'static str>,
    create_params: Option<&'static str>,
    nullable: i16,
    case_sensitive: i16,
    searchable: i16,
    unsigned_attribute: Option<i16>,
    fixed_prec_scale: i16,
    local_type_name: Option<&'static str>,
    minimum_scale: Option<i16>,
    maximum_scale: Option<i16>,
    /// Verbose SQL data type (SQL_DATETIME for date/time types, same as
    /// `data_type` for all others).
    sql_data_type: i16,
    /// Sub-code for datetime types; NULL for all other types.
    sql_datetime_sub: Option<i16>,
    /// Radix for numeric types; NULL for non-numeric types.
    num_prec_radix: Option<i32>,
    user_data_type: i16,
}

/// Hard-coded Snowflake type table, in the legacy insertion order.
///
/// The ordering matches `ApiCatchTest.cpp`'s expected sequence for
/// `SQL_ALL_TYPES`, which intentionally deviates from the ODBC spec's
/// DATA_TYPE-ascending recommendation.
static ALL_SF_TYPE_INFO: &[TypeInfoRow] = &[
    // ── CHAR ─────────────────────────────────────────────────────────────────
    TypeInfoRow {
        type_name: "CHAR",
        data_type: 1,
        column_size: 134_217_728,
        literal_prefix: Some("'"),
        literal_suffix: Some("'"),
        create_params: Some("LENGTH"),
        nullable: 1,
        case_sensitive: 1,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("CHAR"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: 1,
        sql_datetime_sub: None,
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── NUMERIC ──────────────────────────────────────────────────────────────
    // DECFLOAT maps to NUMERIC; MAXIMUM_SCALE=8192 is a known reference-driver
    // quirk preserved here for compatibility.
    TypeInfoRow {
        type_name: "NUMERIC",
        data_type: 2,
        column_size: 38,
        literal_prefix: None,
        literal_suffix: None,
        create_params: Some("precision,scale"),
        nullable: 1,
        case_sensitive: 0,
        searchable: PRED_BASIC,
        unsigned_attribute: Some(0),
        fixed_prec_scale: 0,
        local_type_name: Some("NUMERIC"),
        minimum_scale: Some(0),
        maximum_scale: Some(8192),
        sql_data_type: 2,
        sql_datetime_sub: None,
        num_prec_radix: Some(10),
        user_data_type: 0,
    },
    // ── DECIMAL ──────────────────────────────────────────────────────────────
    // NUMBER maps to DECIMAL.
    TypeInfoRow {
        type_name: "DECIMAL",
        data_type: 3,
        column_size: 38,
        literal_prefix: None,
        literal_suffix: None,
        create_params: Some("precision,scale"),
        nullable: 1,
        case_sensitive: 0,
        searchable: PRED_BASIC,
        unsigned_attribute: Some(0),
        fixed_prec_scale: 0,
        local_type_name: Some("DECIMAL"),
        minimum_scale: Some(0),
        maximum_scale: Some(38),
        sql_data_type: 3,
        sql_datetime_sub: None,
        num_prec_radix: Some(10),
        user_data_type: 0,
    },
    // ── INTEGER ───────────────────────────────────────────────────────────────
    TypeInfoRow {
        type_name: "INTEGER",
        data_type: 4,
        column_size: 10,
        literal_prefix: None,
        literal_suffix: None,
        create_params: None,
        nullable: 1,
        case_sensitive: 0,
        searchable: PRED_BASIC,
        unsigned_attribute: Some(0),
        fixed_prec_scale: 0,
        local_type_name: Some("INTEGER"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: 4,
        sql_datetime_sub: None,
        num_prec_radix: Some(2),
        user_data_type: 0,
    },
    // ── BIGINT ────────────────────────────────────────────────────────────────
    TypeInfoRow {
        type_name: "BIGINT",
        data_type: -5,
        column_size: 19,
        literal_prefix: None,
        literal_suffix: None,
        create_params: None,
        nullable: 1,
        case_sensitive: 0,
        searchable: PRED_BASIC,
        unsigned_attribute: Some(0),
        fixed_prec_scale: 0,
        local_type_name: Some("BIGINT"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: -5,
        sql_datetime_sub: None,
        num_prec_radix: Some(2),
        user_data_type: 0,
    },
    // ── FLOAT ─────────────────────────────────────────────────────────────────
    TypeInfoRow {
        type_name: "FLOAT",
        data_type: 6,
        column_size: 15,
        literal_prefix: None,
        literal_suffix: None,
        create_params: None,
        nullable: 1,
        case_sensitive: 0,
        searchable: PRED_BASIC,
        unsigned_attribute: Some(0),
        fixed_prec_scale: 0,
        local_type_name: Some("FLOAT"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: 6,
        sql_datetime_sub: None,
        num_prec_radix: Some(2),
        user_data_type: 0,
    },
    // ── REAL ──────────────────────────────────────────────────────────────────
    TypeInfoRow {
        type_name: "REAL",
        data_type: 7,
        column_size: 7,
        literal_prefix: None,
        literal_suffix: None,
        create_params: None,
        nullable: 1,
        case_sensitive: 0,
        searchable: PRED_BASIC,
        unsigned_attribute: Some(0),
        fixed_prec_scale: 0,
        local_type_name: Some("REAL"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: 7,
        sql_datetime_sub: None,
        num_prec_radix: Some(2),
        user_data_type: 0,
    },
    // ── DOUBLE ────────────────────────────────────────────────────────────────
    TypeInfoRow {
        type_name: "DOUBLE",
        data_type: 8,
        column_size: 15,
        literal_prefix: None,
        literal_suffix: None,
        create_params: None,
        nullable: 1,
        case_sensitive: 0,
        searchable: PRED_BASIC,
        unsigned_attribute: Some(0),
        fixed_prec_scale: 0,
        local_type_name: Some("DOUBLE"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: 8,
        sql_datetime_sub: None,
        num_prec_radix: Some(2),
        user_data_type: 0,
    },
    // ── VARCHAR ───────────────────────────────────────────────────────────────
    TypeInfoRow {
        type_name: "VARCHAR",
        data_type: 12,
        column_size: 134_217_728,
        literal_prefix: Some("'"),
        literal_suffix: Some("'"),
        create_params: Some("max length"),
        nullable: 1,
        case_sensitive: 1,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("VARCHAR"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: 12,
        sql_datetime_sub: None,
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── BINARY ────────────────────────────────────────────────────────────────
    // literal_suffix is Some("") (empty string) not None; the test expects
    // indicator=0 (non-null, zero-length) not SQL_NULL_DATA.
    TypeInfoRow {
        type_name: "BINARY",
        data_type: -2,
        column_size: 67_108_864,
        literal_prefix: Some("0x"),
        literal_suffix: Some(""),
        create_params: Some("LENGTH"),
        nullable: 1,
        case_sensitive: 1,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("BINARY"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: -2,
        sql_datetime_sub: None,
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── VARBINARY ─────────────────────────────────────────────────────────────
    TypeInfoRow {
        type_name: "VARBINARY",
        data_type: -3,
        column_size: 67_108_864,
        literal_prefix: Some("0x"),
        literal_suffix: Some(""),
        create_params: Some("max length"),
        nullable: 1,
        case_sensitive: 1,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("VARBINARY"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: -3,
        sql_datetime_sub: None,
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── DATE ──────────────────────────────────────────────────────────────────
    TypeInfoRow {
        type_name: "DATE",
        data_type: 91,
        column_size: 10,
        literal_prefix: Some("'"),
        literal_suffix: Some("'"),
        create_params: None,
        nullable: 1,
        case_sensitive: 0,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("TYPE_DATE"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: SQL_DATETIME,
        sql_datetime_sub: Some(CODE_DATE),
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── TIME ──────────────────────────────────────────────────────────────────
    TypeInfoRow {
        type_name: "TIME",
        data_type: 92,
        column_size: 18,
        literal_prefix: Some("'"),
        literal_suffix: Some("'"),
        create_params: None,
        nullable: 1,
        case_sensitive: 0,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("TYPE_TIME"),
        minimum_scale: Some(0),
        maximum_scale: Some(0),
        sql_data_type: SQL_DATETIME,
        sql_datetime_sub: Some(CODE_TIME),
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── TIMESTAMP_LTZ (Snowflake vendor type 2000) ────────────────────────────
    TypeInfoRow {
        type_name: "TIMESTAMP_LTZ",
        data_type: 2000,
        column_size: 35,
        literal_prefix: Some("'"),
        literal_suffix: Some("'"),
        create_params: None,
        nullable: 1,
        case_sensitive: 0,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("STAMP_LTZ"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: 2000,
        sql_datetime_sub: None,
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── TIMESTAMP_NTZ (Snowflake vendor type 2002) ────────────────────────────
    // NTZ appears before TZ in the legacy insertion order (indices 14 vs 15 in
    // the ordering test).
    TypeInfoRow {
        type_name: "TIMESTAMP_NTZ",
        data_type: 2002,
        column_size: 35,
        literal_prefix: Some("'"),
        literal_suffix: Some("'"),
        create_params: None,
        nullable: 1,
        case_sensitive: 0,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("STAMP_NTZ"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: 2002,
        sql_datetime_sub: None,
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── TIMESTAMP_TZ (Snowflake vendor type 2001) ─────────────────────────────
    TypeInfoRow {
        type_name: "TIMESTAMP_TZ",
        data_type: 2001,
        column_size: 35,
        literal_prefix: Some("'"),
        literal_suffix: Some("'"),
        create_params: None,
        nullable: 1,
        case_sensitive: 0,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("STAMP_TZ"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: 2001,
        sql_datetime_sub: None,
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── TIMESTAMP (SQL_TYPE_TIMESTAMP = 93) ───────────────────────────────────
    TypeInfoRow {
        type_name: "TIMESTAMP",
        data_type: 93,
        column_size: 35,
        literal_prefix: Some("'"),
        literal_suffix: Some("'"),
        create_params: None,
        nullable: 1,
        case_sensitive: 0,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("TYPE_TIMESTAMP"),
        minimum_scale: Some(0),
        maximum_scale: Some(0),
        sql_data_type: SQL_DATETIME,
        sql_datetime_sub: Some(CODE_TIMESTAMP),
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── ARRAY (Snowflake vendor type 2003) ────────────────────────────────────
    TypeInfoRow {
        type_name: "ARRAY",
        data_type: 2003,
        column_size: 134_217_728,
        literal_prefix: Some("'"),
        literal_suffix: Some("'"),
        create_params: Some("max length"),
        nullable: 1,
        case_sensitive: 1,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("OWN"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: 2003,
        sql_datetime_sub: None,
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── OBJECT (Snowflake vendor type 2004) ───────────────────────────────────
    TypeInfoRow {
        type_name: "OBJECT",
        data_type: 2004,
        column_size: 134_217_728,
        literal_prefix: Some("'"),
        literal_suffix: Some("'"),
        create_params: Some("max length"),
        nullable: 1,
        case_sensitive: 1,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("OWN"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: 2004,
        sql_datetime_sub: None,
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── VARIANT (Snowflake vendor type 2005) ──────────────────────────────────
    TypeInfoRow {
        type_name: "VARIANT",
        data_type: 2005,
        column_size: 134_217_728,
        literal_prefix: Some("'"),
        literal_suffix: Some("'"),
        create_params: Some("max length"),
        nullable: 1,
        case_sensitive: 1,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("OWN"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: 2005,
        sql_datetime_sub: None,
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── VECTOR (Snowflake vendor type 2006) ───────────────────────────────────
    TypeInfoRow {
        type_name: "VECTOR",
        data_type: 2006,
        column_size: 134_217_728,
        literal_prefix: Some("'"),
        literal_suffix: Some("'"),
        create_params: Some("max length"),
        nullable: 1,
        case_sensitive: 0,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("OWN"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: 2006,
        sql_datetime_sub: None,
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── WCHAR (SQL_WCHAR = -8) ────────────────────────────────────────────────
    TypeInfoRow {
        type_name: "CHAR",
        data_type: -8,
        column_size: 134_217_728,
        literal_prefix: Some("'"),
        literal_suffix: Some("'"),
        create_params: Some("LENGTH"),
        nullable: 1,
        case_sensitive: 1,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("WCHAR"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: -8,
        sql_datetime_sub: None,
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── WVARCHAR (SQL_WVARCHAR = -9) ──────────────────────────────────────────
    TypeInfoRow {
        type_name: "VARCHAR",
        data_type: -9,
        column_size: 134_217_728,
        literal_prefix: Some("'"),
        literal_suffix: Some("'"),
        create_params: Some("LENGTH"),
        nullable: 1,
        case_sensitive: 1,
        searchable: SEARCHABLE,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("WVARCHAR"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: -9,
        sql_datetime_sub: None,
        num_prec_radix: None,
        user_data_type: 0,
    },
    // ── BOOLEAN (SQL_BIT = -7) ────────────────────────────────────────────────
    TypeInfoRow {
        type_name: "BOOLEAN",
        data_type: -7,
        column_size: 1,
        literal_prefix: None,
        literal_suffix: None,
        create_params: None,
        nullable: 1,
        case_sensitive: 0,
        searchable: PRED_BASIC,
        unsigned_attribute: None,
        fixed_prec_scale: 0,
        local_type_name: Some("BIT"),
        minimum_scale: None,
        maximum_scale: None,
        sql_data_type: -7,
        sql_datetime_sub: None,
        num_prec_radix: None,
        user_data_type: 0,
    },
];

/// Build a 20-column `RecordBatch` from the given slice of `TypeInfoRow`
/// references. `AUTO_UNIQUE_VALUE` (col 12) and `INTERVAL_PRECISION` (col 19)
/// are always `None` for all Snowflake types.
fn build_type_info_batch(rows: &[&TypeInfoRow]) -> OdbcResult<RecordBatch> {
    let schema = type_info_schema();

    let type_names: Vec<Option<&str>> = rows.iter().map(|r| Some(r.type_name)).collect();
    let data_types: Vec<Option<i16>> = rows.iter().map(|r| Some(r.data_type)).collect();
    let column_sizes: Vec<Option<i32>> = rows.iter().map(|r| Some(r.column_size)).collect();
    let literal_prefixes: Vec<Option<&str>> = rows.iter().map(|r| r.literal_prefix).collect();
    let literal_suffixes: Vec<Option<&str>> = rows.iter().map(|r| r.literal_suffix).collect();
    let create_params: Vec<Option<&str>> = rows.iter().map(|r| r.create_params).collect();
    let nullables: Vec<Option<i16>> = rows.iter().map(|r| Some(r.nullable)).collect();
    let case_sensitives: Vec<Option<i16>> = rows.iter().map(|r| Some(r.case_sensitive)).collect();
    let searchables: Vec<Option<i16>> = rows.iter().map(|r| Some(r.searchable)).collect();
    let unsigned_attrs: Vec<Option<i16>> = rows.iter().map(|r| r.unsigned_attribute).collect();
    let fixed_prec_scales: Vec<Option<i16>> =
        rows.iter().map(|r| Some(r.fixed_prec_scale)).collect();
    let auto_unique_values: Vec<Option<i16>> = rows.iter().map(|_| None).collect();
    let local_type_names: Vec<Option<&str>> = rows.iter().map(|r| r.local_type_name).collect();
    let minimum_scales: Vec<Option<i16>> = rows.iter().map(|r| r.minimum_scale).collect();
    let maximum_scales: Vec<Option<i16>> = rows.iter().map(|r| r.maximum_scale).collect();
    let sql_data_types: Vec<Option<i16>> = rows.iter().map(|r| Some(r.sql_data_type)).collect();
    let sql_datetime_subs: Vec<Option<i16>> = rows.iter().map(|r| r.sql_datetime_sub).collect();
    let num_prec_radixes: Vec<Option<i32>> = rows.iter().map(|r| r.num_prec_radix).collect();
    let interval_precisions: Vec<Option<i32>> = rows.iter().map(|_| None).collect();
    let user_data_types: Vec<Option<i16>> = rows.iter().map(|r| Some(r.user_data_type)).collect();

    fn str_col(v: Vec<Option<&str>>) -> ArrayRef {
        Arc::new(StringArray::from(v)) as ArrayRef
    }
    fn i16_col(v: Vec<Option<i16>>) -> ArrayRef {
        Arc::new(Int16Array::from(v)) as ArrayRef
    }
    fn i32_col(v: Vec<Option<i32>>) -> ArrayRef {
        Arc::new(Int32Array::from(v)) as ArrayRef
    }

    RecordBatch::try_new(
        schema,
        vec![
            str_col(type_names),
            i16_col(data_types),
            i32_col(column_sizes),
            str_col(literal_prefixes),
            str_col(literal_suffixes),
            str_col(create_params),
            i16_col(nullables),
            i16_col(case_sensitives),
            i16_col(searchables),
            i16_col(unsigned_attrs),
            i16_col(fixed_prec_scales),
            i16_col(auto_unique_values),
            str_col(local_type_names),
            i16_col(minimum_scales),
            i16_col(maximum_scales),
            i16_col(sql_data_types),
            i16_col(sql_datetime_subs),
            i32_col(num_prec_radixes),
            i32_col(interval_precisions),
            i16_col(user_data_types),
        ],
    )
    .context(crate::api::error::RecordBatchBuildSnafu)
}

/// Implements `SQLGetTypeInfo`: returns a static result set describing
/// Snowflake's supported SQL data types.
///
/// When `data_type == SQL_ALL_TYPES` (0), all 23 rows are returned in legacy
/// insertion order. For any other value, only the row whose `DATA_TYPE` column
/// matches is returned. An unknown type yields an empty result set (legacy
/// behavior; the ODBC spec allows `HY004` but compatibility requires success).
pub fn get_type_info(statement_handle: sql::Handle, data_type: sql::SmallInt) -> OdbcResult<()> {
    tracing::debug!("SQLGetTypeInfo called");

    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let conn = dbc.connection.lock();
    let mut inner = guard.inner.lock();

    validate_catalog_stmt_ready(&inner)?;

    match &conn.state {
        ConnectionState::Connected { .. } => {}
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
    }
    drop(conn);

    let filtered: Vec<&TypeInfoRow> = if data_type == SQL_ALL_TYPES {
        ALL_SF_TYPE_INFO.iter().collect()
    } else {
        ALL_SF_TYPE_INFO
            .iter()
            .filter(|r| r.data_type == data_type)
            .collect()
    };

    let schema = type_info_schema();
    let batch = build_type_info_batch(&filtered)?;
    let reader = reader_from_record_batch(batch, schema)?;

    set_state_for_catalog(
        &mut inner,
        StatementState::QueryExecuted {
            reader,
            rows_affected: Some(-1),
            origin: ExecutionOrigin::Direct,
        },
    );

    Ok(())
}

#[cfg(test)]
mod type_info_tests {
    use super::*;

    #[test]
    fn all_types_returns_24_rows_and_20_columns() {
        let all: Vec<&TypeInfoRow> = ALL_SF_TYPE_INFO.iter().collect();
        let batch = build_type_info_batch(&all).expect("batch build failed");
        assert_eq!(batch.num_rows(), 24);
        assert_eq!(batch.num_columns(), 20);
        assert_eq!(type_info_schema().fields().len(), 20);
    }

    #[test]
    fn filter_by_specific_type_returns_one_row() {
        let rows: Vec<&TypeInfoRow> = ALL_SF_TYPE_INFO
            .iter()
            .filter(|r| r.data_type == 2) // SQL_NUMERIC
            .collect();
        let batch = build_type_info_batch(&rows).expect("batch build failed");
        assert_eq!(batch.num_rows(), 1);
    }

    #[test]
    fn filter_by_unknown_type_returns_empty() {
        let rows: Vec<&TypeInfoRow> = ALL_SF_TYPE_INFO
            .iter()
            .filter(|r| r.data_type == 9999)
            .collect();
        let batch = build_type_info_batch(&rows).expect("batch build failed");
        assert_eq!(batch.num_rows(), 0);
    }

    #[test]
    fn insertion_order_matches_ordering_test_expectation() {
        // Authoritative order from get_type_info_tests.cpp ordering test
        // (types[0..23], extended to include VECTOR 2006).
        let expected: &[i16] = &[
            1, 2, 3, 4, -5, 6, 7, 8, 12, -2, -3, 91, 92, 2000, 2002, 2001, 93, 2003, 2004, 2005,
            2006, -8, -9, -7,
        ];
        let actual: Vec<i16> = ALL_SF_TYPE_INFO.iter().map(|r| r.data_type).collect();
        assert_eq!(actual, expected);
    }
}

#[cfg(test)]
mod fk_command_tests {
    use super::*;

    #[test]
    fn pk_table_only_uses_exported_scoped_to_pk() {
        let sql =
            build_show_foreign_keys_command(Some("DB"), Some("SCH"), Some("PKT"), None, None, None);
        assert_eq!(sql, "SHOW EXPORTED KEYS IN table \"DB\".\"SCH\".\"PKT\"");
    }

    #[test]
    fn fk_table_only_uses_imported_scoped_to_fk() {
        let sql =
            build_show_foreign_keys_command(None, None, None, Some("DB"), Some("SCH"), Some("FKT"));
        assert_eq!(sql, "SHOW IMPORTED KEYS IN table \"DB\".\"SCH\".\"FKT\"");
    }

    #[test]
    fn both_tables_tie_breaks_to_exported_pk_side() {
        let sql = build_show_foreign_keys_command(
            Some("DB"),
            Some("SCH"),
            Some("PKT"),
            Some("DB"),
            Some("SCH"),
            Some("FKT"),
        );
        assert_eq!(sql, "SHOW EXPORTED KEYS IN table \"DB\".\"SCH\".\"PKT\"");
    }

    #[test]
    fn unresolved_catalog_widens_to_account() {
        // PK table present but catalog unresolved (no current database): the
        // scope widens to account and the client-side filter narrows it back.
        let sql = build_show_foreign_keys_command(None, None, Some("PKT"), None, None, None);
        assert_eq!(sql, "SHOW EXPORTED KEYS IN account");
    }
}
#[cfg(test)]
mod procedures_tests {
    use super::*;

    #[test]
    fn split_top_level_commas_ignores_nested_parens() {
        // Live-pinned: table return columns use bare types, but a scalar
        // NUMBER(p,s) must never be split on its inner comma.
        assert_eq!(
            split_top_level_commas("ID NUMBER(38,0), NAME VARCHAR"),
            vec!["ID NUMBER(38,0)", "NAME VARCHAR"]
        );
        assert_eq!(split_top_level_commas("NUMBER(10,2)"), vec!["NUMBER(10,2)"]);
        assert!(split_top_level_commas("").is_empty());
        assert_eq!(split_top_level_commas("A"), vec!["A"]);
    }

    #[test]
    fn parenthesized_inner_extracts_argument_list() {
        assert_eq!(
            parenthesized_inner("(PNAME VARCHAR, PAGE FLOAT)"),
            "PNAME VARCHAR, PAGE FLOAT"
        );
        assert_eq!(parenthesized_inner("()"), "");
        assert_eq!(parenthesized_inner("no parens"), "");
    }

    #[test]
    fn count_input_params_matches_live_signatures() {
        // Live-pinned argument_signature strings from information_schema.
        assert_eq!(count_input_params("(P1 VARCHAR)"), 1);
        assert_eq!(count_input_params("(PNAME VARCHAR, PAGE FLOAT)"), 2);
        assert_eq!(count_input_params("(PID NUMBER, PNAME VARCHAR)"), 2);
        assert_eq!(count_input_params("()"), 0);
    }

    #[test]
    fn returns_table_detects_table_valued_return() {
        // Live-pinned: table returns render as "TABLE (…)" with a space.
        assert!(returns_table("TABLE (ID NUMBER, NAME VARCHAR)"));
        assert!(returns_table("table(x int)"));
        assert!(!returns_table("VARCHAR(134217728)"));
        assert!(!returns_table("NUMBER(38,0)"));
    }

    #[test]
    fn escape_sql_string_literal_doubles_single_quotes() {
        assert_eq!(escape_sql_string_literal("O'Brien%"), "O''Brien%");
        assert_eq!(escape_sql_string_literal("PROC%"), "PROC%");
    }

    #[test]
    fn escape_sql_string_literal_escapes_backslash_before_quote() {
        // The escaped quote must not close the literal.
        assert_eq!(
            escape_sql_string_literal("a\\' union all"),
            "a\\\\'' union all"
        );
        // Identifier mode: escape_like_wildcards produces MY\_PROC; the backslash
        // must survive the Snowflake string layer so LIKE sees an escaped '_'.
        assert_eq!(escape_sql_string_literal("MY\\_PROC"), "MY\\\\_PROC");
    }

    #[test]
    fn build_procedures_query_escapes_injection_payload() {
        let sql =
            build_procedures_query(&["DB".to_string()], None, Some("a\\' union all select 1--"));
        // The lone escaped quote is neutralized (backslash doubled, quote doubled),
        // so no bare quote can terminate the literal early.
        assert!(sql.contains("procedure_name like 'a\\\\'' union all select 1--'"));
    }

    #[test]
    fn build_procedures_query_single_db_with_filters() {
        let sql = build_procedures_query(
            &["ODBCMETADATATESTDB".to_string()],
            Some("CATALOGTESTS"),
            Some("BASICPROC"),
        );
        assert!(sql.contains("\"ODBCMETADATATESTDB\".information_schema.procedures"));
        assert!(sql.contains("procedure_name like 'BASICPROC'"));
        assert!(sql.contains("procedure_schema like 'CATALOGTESTS'"));
        assert!(!sql.contains("union all"));
    }

    #[test]
    fn build_procedures_query_percent_proc_pattern_is_not_filtered() {
        // A "%" procedure pattern matches everything, so no WHERE is emitted for it.
        let sql = build_procedures_query(&["DB".to_string()], None, Some("%"));
        assert!(!sql.contains("procedure_name like"));
        assert!(!sql.contains("where"));
    }

    #[test]
    fn build_procedures_query_unions_multiple_dbs() {
        let sql = build_procedures_query(&["A".to_string(), "B".to_string()], None, None);
        assert!(sql.contains("\"A\".information_schema.procedures"));
        assert!(sql.contains("\"B\".information_schema.procedures"));
        assert!(sql.contains("union all"));
    }

    #[test]
    fn procedures_empty_result_sql_states_include_no_data() {
        assert!(is_procedures_empty_result_sql_state("02000"));
        assert!(is_procedures_empty_result_sql_state("42000"));
        assert!(is_procedures_empty_result_sql_state("42S02"));
        assert!(!is_procedures_empty_result_sql_state("22007"));
        // The shared keys matcher must NOT swallow 02000 (scope isolation).
        assert!(!is_object_not_found_sql_state("02000"));
    }

    #[test]
    fn map_procedures_to_odbc_emits_eight_columns() {
        use arrow::array::StringArray;
        let input_schema = Arc::new(Schema::new(vec![
            Field::new("procedure_catalog", DataType::Utf8, true),
            Field::new("procedure_schema", DataType::Utf8, true),
            Field::new("procedure_name", DataType::Utf8, true),
            Field::new("argument_signature", DataType::Utf8, true),
            Field::new("data_type", DataType::Utf8, true),
            Field::new("comment", DataType::Utf8, true),
        ]));
        let batch = RecordBatch::try_new(
            input_schema,
            vec![
                Arc::new(StringArray::from(vec![Some("DB"), Some("DB")])) as ArrayRef,
                Arc::new(StringArray::from(vec![Some("S"), Some("S")])) as ArrayRef,
                Arc::new(StringArray::from(vec![Some("SCALARP"), Some("TABLEP")])) as ArrayRef,
                Arc::new(StringArray::from(vec![
                    Some("(P1 VARCHAR, P2 NUMBER)"),
                    Some("()"),
                ])) as ArrayRef,
                Arc::new(StringArray::from(vec![
                    Some("NUMBER(38,0)"),
                    Some("TABLE (ID NUMBER, NAME VARCHAR)"),
                ])) as ArrayRef,
                Arc::new(StringArray::from(vec![None::<&str>, None::<&str>])) as ArrayRef,
            ],
        )
        .unwrap();

        let out = map_procedures_to_odbc(batch).expect("map failed");
        assert_eq!(out.num_columns(), 8);
        assert_eq!(out.num_rows(), 2);

        let num_inputs = out.column(3).as_any().downcast_ref::<Int32Array>().unwrap();
        assert_eq!(num_inputs.value(0), 2); // scalar proc: 2 params
        assert_eq!(num_inputs.value(1), 0); // table proc: 0 params

        // NUM_OUTPUT_PARAMS (col 5) is always NULL.
        assert!(out.column(4).is_null(0));

        let num_result_sets = out.column(5).as_any().downcast_ref::<Int32Array>().unwrap();
        assert_eq!(num_result_sets.value(0), 0); // scalar return
        assert_eq!(num_result_sets.value(1), 1); // table-valued return

        let proc_type = out.column(7).as_any().downcast_ref::<Int16Array>().unwrap();
        assert_eq!(proc_type.value(0), SQL_PT_FUNCTION);
    }
}

#[cfg(test)]
mod procedure_columns_tests {
    use super::*;
    use crate::conversion::NumericSettings;

    // Column indices into the 21-column SQLProcedureColumns result set.
    const COL_NAME: usize = 3;
    const COL_TYPE: usize = 4;
    const DATA_TYPE: usize = 5;
    const ORDINAL: usize = 17;
    const IS_NULLABLE: usize = 18;
    const IS_RESULT_SET_COL: usize = 19;

    fn cell(batch: &RecordBatch, col: usize, row: usize) -> Option<String> {
        utf8_value_at(batch, col, row)
    }

    fn meta<'a>(field: &'a Field, key: &str) -> Option<&'a str> {
        field.metadata().get(key).map(|s| s.as_str())
    }

    #[test]
    fn parse_named_column_splits_on_first_whitespace() {
        let c = parse_named_column("PNAME VARCHAR").unwrap();
        assert_eq!(c.name, "PNAME");
        assert_eq!(c.type_str, "VARCHAR");
        // Multi-word type names stay intact.
        let c = parse_named_column("V DOUBLE PRECISION").unwrap();
        assert_eq!(c.name, "V");
        assert_eq!(c.type_str, "DOUBLE PRECISION");
        assert!(parse_named_column("NAKED").is_none());
    }

    #[test]
    fn parse_argument_columns_extracts_ordered_params() {
        let cols = parse_argument_columns("(PNAME VARCHAR, PAGE FLOAT)");
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0].name, "PNAME");
        assert_eq!(cols[1].name, "PAGE");
        assert!(parse_argument_columns("()").is_empty());
    }

    #[test]
    fn parse_return_shape_distinguishes_scalar_and_table() {
        match parse_return_shape("VARCHAR(16777216)") {
            ReturnShape::Scalar(t) => assert_eq!(t, "VARCHAR(16777216)"),
            ReturnShape::Table(_) => panic!("expected scalar"),
        }
        match parse_return_shape("TABLE (ID NUMBER, NAME VARCHAR)") {
            ReturnShape::Table(cols) => {
                assert_eq!(cols.len(), 2);
                assert_eq!(cols[0].name, "ID");
                assert_eq!(cols[1].name, "NAME");
            }
            ReturnShape::Scalar(_) => panic!("expected table"),
        }
        // SNOW-1232955: an empty return-table column list is valid.
        match parse_return_shape("TABLE ()") {
            ReturnShape::Table(cols) => assert!(cols.is_empty()),
            ReturnShape::Scalar(_) => panic!("expected table"),
        }
    }

    #[test]
    fn field_from_sql_type_string_maps_logical_types() {
        let ns = NumericSettings::default();
        let f = field_from_sql_type_string("VARCHAR(100)", &ns);
        assert_eq!(meta(&f, "logicalType"), Some("TEXT"));
        assert_eq!(meta(&f, "charLength"), Some("100"));

        // Bare VARCHAR (as in argument_signature) falls back to the max size.
        let f = field_from_sql_type_string("VARCHAR", &ns);
        assert_eq!(meta(&f, "logicalType"), Some("TEXT"));
        assert!(meta(&f, "charLength").is_some());

        let f = field_from_sql_type_string("NUMBER(38,0)", &ns);
        assert_eq!(meta(&f, "logicalType"), Some("FIXED"));
        assert_eq!(meta(&f, "precision"), Some("38"));
        assert_eq!(meta(&f, "scale"), Some("0"));

        // INTEGER normalizes to NUMBER(38,0).
        let f = field_from_sql_type_string("INTEGER", &ns);
        assert_eq!(meta(&f, "logicalType"), Some("FIXED"));
        assert_eq!(meta(&f, "precision"), Some("38"));

        assert_eq!(
            meta(&field_from_sql_type_string("FLOAT", &ns), "logicalType"),
            Some("REAL")
        );
        assert_eq!(
            meta(&field_from_sql_type_string("BOOLEAN", &ns), "logicalType"),
            Some("BOOLEAN")
        );

        let f = field_from_sql_type_string("TIMESTAMP_NTZ(9)", &ns);
        assert_eq!(meta(&f, "logicalType"), Some("TIMESTAMP_NTZ"));
        assert_eq!(meta(&f, "scale"), Some("9"));

        // Bare TIMESTAMP defaults to NTZ with scale 9.
        let f = field_from_sql_type_string("TIMESTAMP", &ns);
        assert_eq!(meta(&f, "logicalType"), Some("TIMESTAMP_NTZ"));
        assert_eq!(meta(&f, "scale"), Some("9"));
    }

    #[test]
    fn catalog_type_name_maps_logical_types_to_external_names() {
        // SNOW-3899531: SQLColumns TYPE_NAME must report Snowflake external /
        // friendly names, matching the reference driver — NOT the SDK labels
        // (BIT / TYPE_DATE / TYPE_TIMESTAMP) used by SQLColAttribute.
        let cases = [
            ("TEXT", "VARCHAR"),
            ("FIXED", "DECIMAL"),
            ("REAL", "DOUBLE"),
            ("BINARY", "BINARY"),
            ("BOOLEAN", "BOOLEAN"),
            ("DATE", "DATE"),
            ("TIME", "TIME"),
            ("TIMESTAMP_NTZ", "TIMESTAMP"),
            ("TIMESTAMP_LTZ", "TIMESTAMP"),
            ("TIMESTAMP_TZ", "TIMESTAMP"),
            ("TIMESTAMP", "TIMESTAMP"),
            ("VARIANT", "VARIANT"),
            ("ARRAY", "ARRAY"),
            ("OBJECT", "STRUCT"),
            ("GEOGRAPHY", "GEOGRAPHY"),
            ("GEOMETRY", "GEOMETRY"),
        ];
        for (logical, expected) in cases {
            assert_eq!(
                catalog_type_name_from_logical_type(logical),
                expected,
                "logical type {logical} should map to {expected}"
            );
        }

        // Semi-structured types must not collapse to VARCHAR, and GEOGRAPHY
        // must not be NULL — the two regressions this fix targets.
        assert_ne!(catalog_type_name_from_logical_type("OBJECT"), "VARCHAR");
        assert_ne!(catalog_type_name_from_logical_type("VARIANT"), "VARCHAR");

        // Absent logical type → safe VARCHAR fallback; unknown exotic type →
        // its uppercased logical name (OLD's else branch).
        assert_eq!(catalog_type_name_from_logical_type(""), "VARCHAR");
        assert_eq!(catalog_type_name_from_logical_type("vector"), "VECTOR");
    }

    #[test]
    fn like_match_supports_wildcards_and_escape() {
        // "%" (and NULL upstream) match everything, including the empty return
        // value name — so the return value is only dropped by a specific pattern.
        assert!(like_match("%", ""));
        assert!(like_match("%", "PNAME"));
        assert!(like_match("PNAME", "PNAME"));
        assert!(!like_match("PNAME", ""));
        assert!(!like_match("", ""));
        assert!(!like_match("", "PNAME"));
        assert!(!like_match("PNAME", "PAGE"));
        assert!(like_match("P%", "PAGE"));
        assert!(like_match("P_GE", "PAGE"));
        assert!(!like_match("P_AGE", "PAGE"));
        // Escaped wildcard is a literal (identifier-mode inputs are pre-escaped).
        assert!(like_match("A\\%B", "A%B"));
        assert!(!like_match("A\\%B", "AXB"));
    }

    fn procedures_batch() -> RecordBatch {
        let input_schema = Arc::new(Schema::new(vec![
            Field::new("procedure_catalog", DataType::Utf8, true),
            Field::new("procedure_schema", DataType::Utf8, true),
            Field::new("procedure_name", DataType::Utf8, true),
            Field::new("argument_signature", DataType::Utf8, true),
            Field::new("data_type", DataType::Utf8, true),
            Field::new("comment", DataType::Utf8, true),
        ]));
        RecordBatch::try_new(
            input_schema,
            vec![
                Arc::new(StringArray::from(vec![Some("DB"), Some("DB")])) as ArrayRef,
                Arc::new(StringArray::from(vec![Some("S"), Some("S")])) as ArrayRef,
                Arc::new(StringArray::from(vec![Some("SCALARP"), Some("TABLEP")])) as ArrayRef,
                Arc::new(StringArray::from(vec![
                    Some("(PNAME VARCHAR, PAGE FLOAT)"),
                    Some("(X NUMBER)"),
                ])) as ArrayRef,
                Arc::new(StringArray::from(vec![
                    Some("VARCHAR(16777216)"),
                    Some("TABLE (ID NUMBER, NAME VARCHAR)"),
                ])) as ArrayRef,
                Arc::new(StringArray::from(vec![None::<&str>, None::<&str>])) as ArrayRef,
            ],
        )
        .unwrap()
    }

    #[test]
    fn map_procedure_columns_emits_twenty_one_columns_and_orders_rows() {
        let ns = NumericSettings::default();
        let out = map_procedure_columns_to_odbc(procedures_batch(), None, &ns).expect("map failed");
        assert_eq!(out.num_columns(), 21);
        // SCALARP: return value + 2 params (3) ; TABLEP: 2 result cols + 1 param (3).
        assert_eq!(out.num_rows(), 6);

        // Scalar procedure: return value first, empty name, COLUMN_TYPE=RETURN_VALUE,
        // ORDINAL=0; its DATA_TYPE is populated (VARCHAR return type).
        assert_eq!(cell(&out, COL_NAME, 0).as_deref(), Some(""));
        assert_eq!(cell(&out, COL_TYPE, 0), Some(SQL_RETURN_VALUE.to_string()));
        assert_eq!(cell(&out, ORDINAL, 0).as_deref(), Some("0"));
        assert!(cell(&out, DATA_TYPE, 0).is_some());
        assert_eq!(cell(&out, IS_NULLABLE, 0).as_deref(), Some("YES"));
        assert_eq!(
            cell(&out, IS_RESULT_SET_COL, 0),
            Some(SQL_FALSE.to_string())
        );

        // Input parameters follow in declaration order (ordinals 1, 2).
        assert_eq!(cell(&out, COL_NAME, 1).as_deref(), Some("PNAME"));
        assert_eq!(cell(&out, COL_TYPE, 1), Some(SQL_PARAM_INPUT.to_string()));
        assert_eq!(cell(&out, ORDINAL, 1).as_deref(), Some("1"));
        assert_eq!(cell(&out, COL_NAME, 2).as_deref(), Some("PAGE"));
        assert_eq!(cell(&out, ORDINAL, 2).as_deref(), Some("2"));

        // Table-valued procedure: result columns first (COLUMN_TYPE=RESULT_COL,
        // ordinals 1..), then the input parameter (ordinal 1).
        assert_eq!(cell(&out, COL_NAME, 3).as_deref(), Some("ID"));
        assert_eq!(cell(&out, COL_TYPE, 3), Some(SQL_RESULT_COL.to_string()));
        assert_eq!(cell(&out, ORDINAL, 3).as_deref(), Some("1"));
        assert_eq!(cell(&out, IS_RESULT_SET_COL, 3), Some(SQL_TRUE.to_string()));
        assert_eq!(cell(&out, COL_NAME, 4).as_deref(), Some("NAME"));
        assert_eq!(cell(&out, COL_TYPE, 4), Some(SQL_RESULT_COL.to_string()));
        assert_eq!(cell(&out, COL_NAME, 5).as_deref(), Some("X"));
        assert_eq!(cell(&out, COL_TYPE, 5), Some(SQL_PARAM_INPUT.to_string()));
        assert_eq!(cell(&out, ORDINAL, 5).as_deref(), Some("1"));
    }

    #[test]
    fn map_procedure_columns_applies_column_name_filter() {
        let ns = NumericSettings::default();
        // A specific ColumnName pattern drops the empty-named return value and
        // keeps only matching parameters/result columns.
        let out = map_procedure_columns_to_odbc(procedures_batch(), Some("PNAME"), &ns)
            .expect("map failed");
        assert_eq!(out.num_rows(), 1);
        assert_eq!(cell(&out, COL_NAME, 0).as_deref(), Some("PNAME"));

        // "%" keeps everything (including the return value).
        let out =
            map_procedure_columns_to_odbc(procedures_batch(), Some("%"), &ns).expect("map failed");
        assert_eq!(out.num_rows(), 6);
    }

    #[test]
    fn map_procedure_columns_empty_batch_yields_empty_result() {
        let ns = NumericSettings::default();
        let empty = RecordBatch::new_empty(procedures_batch().schema());
        let out = map_procedure_columns_to_odbc(empty, None, &ns).expect("map failed");
        assert_eq!(out.num_columns(), 21);
        assert_eq!(out.num_rows(), 0);
    }
}

#[cfg(test)]
mod sqltables_tests {
    use super::*;

    #[test]
    fn like_match_is_case_sensitive() {
        // SNOW-3780463: pattern mode must not fold case.
        assert!(like_match("BASICTABLE", "BASICTABLE"));
        assert!(!like_match("basictable", "BASICTABLE"));
        assert!(!like_match("BASICTABLE", "basictable"));
        assert!(like_match("basic%", "basictable"));
        assert!(!like_match("basic%", "BASICTABLE"));
    }

    #[test]
    fn normalize_kind_maps_table_and_view_variants() {
        assert_eq!(normalize_kind("TABLE"), Some("TABLE"));
        assert_eq!(normalize_kind("TRANSIENT TABLE"), Some("TABLE"));
        assert_eq!(normalize_kind("ICEBERG TABLE"), Some("TABLE"));
        assert_eq!(normalize_kind("VIEW"), Some("VIEW"));
        assert_eq!(normalize_kind("MATERIALIZED VIEW"), Some("VIEW"));
        assert_eq!(normalize_kind("SECURE VIEW"), Some("VIEW"));
        assert_eq!(normalize_kind("unknown"), Some("TABLE"));
    }

    #[test]
    fn normalize_kind_skips_non_table_view_objects() {
        assert_eq!(normalize_kind("PERSONAL DATABASE"), None);
        assert_eq!(normalize_kind("personal database"), None);
    }

    #[test]
    fn normalize_table_types_empty_and_percent_mean_all() {
        assert_eq!(normalize_table_types(&[]), TableTypeFilter::All);
        assert_eq!(
            normalize_table_types(&["%".to_string()]),
            TableTypeFilter::All
        );
    }

    #[test]
    fn normalize_table_types_explicit_and_unsupported() {
        assert_eq!(
            normalize_table_types(&["table".to_string(), "VIEW".to_string()]),
            TableTypeFilter::Explicit(vec!["TABLE".to_string(), "VIEW".to_string()])
        );
        assert_eq!(
            normalize_table_types(&["SYNONYM".to_string()]),
            TableTypeFilter::Unsupported
        );
    }

    #[test]
    fn format_show_sql_like_precedes_scope() {
        assert_eq!(
            format_show_sql("SHOW OBJECTS", "LIKE 'T%'", "IN SCHEMA \"DB\".\"SCH\""),
            "SHOW OBJECTS LIKE 'T%' IN SCHEMA \"DB\".\"SCH\""
        );
        assert_eq!(
            format_show_sql("SHOW OBJECTS", "", "IN DATABASE \"DB\""),
            "SHOW OBJECTS IN DATABASE \"DB\""
        );
    }

    #[test]
    fn build_show_objects_sql_picks_tightest_exact_scope() {
        assert_eq!(
            build_show_objects_sql(Some("DB"), Some("SCH"), Some("T%")),
            "SHOW OBJECTS LIKE 'T%' IN SCHEMA \"DB\".\"SCH\""
        );
        assert_eq!(
            build_show_objects_sql(Some("DB"), None, None),
            "SHOW OBJECTS IN DATABASE \"DB\""
        );
        assert_eq!(
            build_show_objects_sql(Some("DB%"), None, None),
            "SHOW OBJECTS IN ACCOUNT"
        );
        // Escaped underscore is exact (is_exact_pattern recovers the literal).
        assert_eq!(
            build_show_objects_sql(Some("SNOWFLAKE\\_SAMPLE\\_DATA"), None, None),
            "SHOW OBJECTS IN DATABASE \"SNOWFLAKE_SAMPLE_DATA\""
        );
        // Injection-critical: a double quote in an exact catalog/schema name must
        // be doubled ("" ) by escape_snowflake_identifier so it cannot break out
        // of the quoted IN DATABASE/IN SCHEMA identifier.
        assert_eq!(
            build_show_objects_sql(Some("D\"B"), None, None),
            "SHOW OBJECTS IN DATABASE \"D\"\"B\""
        );
        assert_eq!(
            build_show_objects_sql(Some("A\"B"), Some("C\"D"), Some("T%")),
            "SHOW OBJECTS LIKE 'T%' IN SCHEMA \"A\"\"B\".\"C\"\"D\""
        );
    }

    #[test]
    fn build_show_columns_sql_picks_tightest_exact_scope() {
        assert_eq!(
            build_show_columns_sql(Some("DB"), Some("SCH"), Some("TBL"), Some("C%")),
            "SHOW COLUMNS LIKE 'C%' IN TABLE \"DB\".\"SCH\".\"TBL\""
        );
        assert_eq!(
            build_show_columns_sql(Some("DB"), Some("SCH"), Some("T%"), None),
            "SHOW COLUMNS IN SCHEMA \"DB\".\"SCH\""
        );
        assert_eq!(
            build_show_columns_sql(Some("DB"), None, None, None),
            "SHOW COLUMNS IN DATABASE \"DB\""
        );
        assert_eq!(
            build_show_columns_sql(Some("DB%"), None, None, Some("ID")),
            "SHOW COLUMNS LIKE 'ID' IN ACCOUNT"
        );
        // Escaped underscore is exact (is_exact_pattern recovers the literal).
        assert_eq!(
            build_show_columns_sql(
                Some("SNOWFLAKE\\_SAMPLE\\_DATA"),
                Some("TPCH\\_SF1"),
                Some("ORDERS"),
                None
            ),
            "SHOW COLUMNS IN TABLE \"SNOWFLAKE_SAMPLE_DATA\".\"TPCH_SF1\".\"ORDERS\""
        );
    }

    #[test]
    fn build_like_clause_strips_escapes_for_coarse_pushdown() {
        assert_eq!(build_like_clause(None), "");
        assert_eq!(build_like_clause(Some("")), "");
        assert_eq!(build_like_clause(Some("MY\\_TABLE")), "LIKE 'MY_TABLE'");
        assert_eq!(build_like_clause(Some("100\\%")), "LIKE '100%'");
        // Trailing backslash must not escape the closing quote.
        assert_eq!(build_like_clause(Some("AB\\")), "LIKE 'AB\\\\'");
        // Injection-critical: an embedded apostrophe must be doubled so it cannot
        // close the LIKE '…' literal early.
        assert_eq!(build_like_clause(Some("O'Brien")), "LIKE 'O''Brien'");
    }

    #[test]
    fn is_exact_pattern_detects_wildcards_and_unescapes() {
        assert_eq!(is_exact_pattern("FOO"), Some("FOO".to_string()));
        assert_eq!(is_exact_pattern("FOO%"), None);
        assert_eq!(is_exact_pattern("FOO\\_BAR"), Some("FOO_BAR".to_string()));
        assert_eq!(is_exact_pattern(""), Some(String::new()));
        // Escaped quote must strip like strip_escapes_for_show_like, so SHOW
        // scope and coarse LIKE agree on the literal identifier.
        assert_eq!(is_exact_pattern("FOO\\\"BAR"), Some("FOO\"BAR".to_string()));
        assert_eq!(
            is_exact_pattern("FOO\\\"BAR"),
            Some(strip_escapes_for_show_like("FOO\\\"BAR"))
        );
    }

    #[test]
    fn escape_show_like_matches_sql_string_literal_escaping() {
        assert_eq!(
            escape_show_like("O'Brien"),
            escape_sql_string_literal("O'Brien")
        );
        assert_eq!(escape_show_like("a\\b"), escape_sql_string_literal("a\\b"));
        // Quote-doubling (not backslash-quote) for the embedded apostrophe.
        assert_eq!(escape_show_like("O'Brien"), "O''Brien");
    }

    #[test]
    fn pattern_matches_none_matches_all_and_empty_matches_nothing() {
        assert!(pattern_matches(None, "ANY"));
        assert!(!pattern_matches(Some(""), "ANY"));
        assert!(!pattern_matches(Some(""), ""));
        assert!(pattern_matches(Some("%"), "ANY"));
        assert!(!pattern_matches(Some("basictable"), "BASICTABLE"));
    }

    #[test]
    fn tables_empty_result_sql_states_include_no_data() {
        assert!(is_tables_empty_result_sql_state("02000"));
        assert!(is_tables_empty_result_sql_state("42000"));
        assert!(is_tables_empty_result_sql_state("42S02"));
        assert!(!is_tables_empty_result_sql_state("22007"));
        // Keys matcher must stay narrower (no 02000).
        assert!(!is_object_not_found_sql_state("02000"));
    }
}

#[cfg(test)]
mod sqlcolumns_decode_tests {
    use super::*;

    #[test]
    fn decoder_fixed_type() {
        let json = r#"{"type":"FIXED","nullable":true,"fixed":true,"precision":38,"scale":0}"#;
        let d = decode_data_type_json(json, "ID".to_string(), 1, None, None);
        assert_eq!(d.logical_type, "FIXED");
        assert_eq!(d.precision, Some(38));
        assert_eq!(d.scale, Some(0));
        assert!(d.nullable);
        assert_eq!(d.char_length, None);
        assert_eq!(d.byte_length, None);
    }

    #[test]
    fn decoder_text_type_char_length_preferred() {
        let json = r#"{"type":"TEXT","nullable":false,"precision":16777216,"scale":0,"length":16777216,"byteLength":16777216,"charLength":16777216}"#;
        let d = decode_data_type_json(json, "NAME".to_string(), 2, None, None);
        assert_eq!(d.logical_type, "TEXT");
        assert_eq!(d.char_length, Some(16777216));
        assert_eq!(d.byte_length, Some(16777216));
        assert!(!d.nullable);
    }

    #[test]
    fn decoder_text_type_falls_back_to_length() {
        let json = r#"{"type":"TEXT","nullable":true,"length":255}"#;
        let d = decode_data_type_json(json, "C".to_string(), 1, None, None);
        assert_eq!(d.logical_type, "TEXT");
        assert_eq!(d.char_length, Some(255));
    }

    #[test]
    fn decoder_non_text_type_ignores_length() {
        // `length` is a TEXT-only alias for charLength; non-TEXT types must not
        // adopt it as char_length.
        let json = r#"{"type":"BINARY","nullable":true,"length":8388608}"#;
        let d = decode_data_type_json(json, "B".to_string(), 1, None, None);
        assert_eq!(d.logical_type, "BINARY");
        assert_eq!(d.char_length, None);
    }

    #[test]
    fn decoder_boolean_type() {
        let json = r#"{"type":"BOOLEAN","nullable":true}"#;
        let d = decode_data_type_json(json, "FLAG".to_string(), 1, None, None);
        assert_eq!(d.logical_type, "BOOLEAN");
        assert_eq!(d.precision, None);
        assert_eq!(d.scale, None);
    }

    #[test]
    fn decoder_timestamp_ntz() {
        let json = r#"{"type":"TIMESTAMP_NTZ","nullable":true,"precision":0,"scale":9}"#;
        let d = decode_data_type_json(json, "TS".to_string(), 1, None, None);
        assert_eq!(d.logical_type, "TIMESTAMP_NTZ");
        assert_eq!(d.scale, Some(9));
    }

    #[test]
    fn decoder_unknown_json_produces_unknown_logical_type() {
        let d = decode_data_type_json("not valid json{{", "X".to_string(), 1, None, None);
        assert_eq!(d.logical_type, "UNKNOWN");
        assert!(d.nullable);
    }

    #[test]
    fn decoder_overflowing_precision_and_scale_degrade_to_none() {
        // precision/scale beyond i32::MAX must not wrap to a garbage i32 in
        // release builds — the checked narrowing yields None (unknown) instead.
        let json = r#"{"type":"FIXED","nullable":true,"precision":9999999999,"scale":9999999999}"#;
        let d = decode_data_type_json(json, "BIG".to_string(), 1, None, None);
        assert_eq!(d.logical_type, "FIXED");
        assert_eq!(d.precision, None);
        assert_eq!(d.scale, None);
    }

    #[test]
    fn decoder_preserves_ordinal_and_identity() {
        let json = r#"{"type":"FIXED","nullable":true,"precision":10,"scale":2}"#;
        let d = decode_data_type_json(
            json,
            "AMT".to_string(),
            5,
            Some("0".to_string()),
            Some("doc".to_string()),
        );
        assert_eq!(d.column_name, "AMT");
        assert_eq!(d.ordinal_position, 5);
        assert_eq!(d.column_def, Some("0".to_string()));
        assert_eq!(d.remarks, Some("doc".to_string()));
    }

    #[test]
    fn decoder_missing_nullable_defaults_true() {
        let json = r#"{"type":"FIXED","precision":10,"scale":0}"#;
        let d = decode_data_type_json(json, "N".to_string(), 1, None, None);
        assert!(d.nullable);
    }
}
