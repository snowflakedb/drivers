//! Catalog functions: SQLTables, SQLGetTypeInfo, and related.
//!
//! The wrapper reads ODBC string arguments, maps them to patterns (via
//! `catalog_arg_to_pattern`), dispatches to the core `ConnectionGetObjects`
//! RPC, then flattens the nested ADBC-shaped Arrow result into the flat
//! 5-column ODBC result set.
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
use crate::api::statement::{
    collect_nested_batch, execute_show_query_collect_batch, set_state_for_catalog,
};
use crate::api::utils::{catalog_arg_to_pattern, escape_like_wildcards};
use crate::api::{
    ConnectionState, ExecutionOrigin, OdbcResult, StatementInner, StatementState, stmt_from_handle,
};
use crate::conversion::{
    NumericSettings, SMALLINT_CONCISE_SQL_TYPE, column_size_from_field, decimal_digits_from_field,
    num_prec_radix_from_field, octet_length_from_field, sql_type_from_field, type_name_from_field,
    verbose_sql_type_from_field,
};
use arrow::array::{
    Array, ArrayRef, BooleanArray, Int16Array, Int32Array, Int64Array, LargeListArray, RecordBatch,
    StringArray, StructArray,
};
use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use odbc_sys as sql;
use sf_core::apis::database_driver_v1::{
    DEPTH_CATALOGS, DEPTH_COLUMNS, DEPTH_DB_SCHEMAS, DEPTH_TABLES, FIELD_CATALOG_DB_SCHEMAS,
    FIELD_CATALOG_NAME, FIELD_COLUMN_BYTE_LENGTH, FIELD_COLUMN_CHAR_LENGTH, FIELD_COLUMN_DEF,
    FIELD_COLUMN_LOGICAL_TYPE, FIELD_COLUMN_NAME, FIELD_COLUMN_NULLABLE,
    FIELD_COLUMN_ORDINAL_POSITION, FIELD_COLUMN_PRECISION, FIELD_COLUMN_REMARKS,
    FIELD_COLUMN_SCALE, FIELD_DB_SCHEMA_NAME, FIELD_DB_SCHEMA_TABLES, FIELD_TABLE_COLUMNS,
    FIELD_TABLE_NAME, FIELD_TABLE_TYPE,
};
use sf_core::protobuf::generated::database_driver_v1::{
    ConnectionGetInfoRequest, ConnectionGetObjectsRequest, ConnectionGetParameterRequest,
    ResultSetGetStreamRequest, ResultSetHandle, ResultSetReleaseRequest,
};
use snafu::{OptionExt, ResultExt};
use std::collections::HashMap;
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

fn flat_tables_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        catalog_text_field("TABLE_CAT", 255),
        catalog_text_field("TABLE_SCHEM", 255),
        catalog_text_field("TABLE_NAME", 255),
        catalog_text_field("TABLE_TYPE", 255),
        catalog_text_field("REMARKS", 65535),
    ]))
}

fn flat_columns_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        catalog_text_field("TABLE_CAT", 255),        // 1
        catalog_text_field("TABLE_SCHEM", 255),      // 2
        catalog_text_field("TABLE_NAME", 255),       // 3
        catalog_text_field("COLUMN_NAME", 255),      // 4
        catalog_text_field("DATA_TYPE", 20),         // 5 SMALLINT returned as text
        catalog_text_field("TYPE_NAME", 255),        // 6
        catalog_text_field("COLUMN_SIZE", 20),       // 7 INTEGER returned as text
        catalog_text_field("BUFFER_LENGTH", 20),     // 8 INTEGER returned as text
        catalog_text_field("DECIMAL_DIGITS", 20),    // 9 SMALLINT nullable
        catalog_text_field("NUM_PREC_RADIX", 20),    // 10 SMALLINT nullable
        catalog_text_field("NULLABLE", 20),          // 11 SMALLINT (0/1)
        catalog_text_field("REMARKS", 65535),        // 12
        catalog_text_field("COLUMN_DEF", 65535),     // 13 nullable
        catalog_text_field("SQL_DATA_TYPE", 20),     // 14 SMALLINT
        catalog_text_field("SQL_DATETIME_SUB", 20),  // 15 SMALLINT nullable
        catalog_text_field("CHAR_OCTET_LENGTH", 20), // 16 INTEGER nullable
        catalog_text_field("ORDINAL_POSITION", 20),  // 17 INTEGER
        catalog_text_field("IS_NULLABLE", 3),        // 18 "YES"/"NO"/""
        catalog_text_field("USER_DATA_TYPE", 20),    // 19 driver-specific
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
    drop(conn);

    let is_empty_str = |s: &Option<String>| s.as_deref() == Some("");

    // SQL_ALL_CATALOGS special case: catalog="%", schema="", table="", type=""
    if catalog_raw.as_deref() == Some(SQL_ALL_CATALOGS)
        && is_empty_str(&schema_raw)
        && is_empty_str(&table_raw)
        && is_empty_str(&type_raw)
    {
        return execute_get_objects_and_flatten(
            &mut inner,
            conn_handle,
            DEPTH_CATALOGS,
            None,
            None,
            None,
            vec![],
        );
    }

    // SQL_ALL_SCHEMAS special case: schema="%", catalog="", table="", type=""
    if schema_raw.as_deref() == Some(SQL_ALL_SCHEMAS)
        && is_empty_str(&catalog_raw)
        && is_empty_str(&table_raw)
        && is_empty_str(&type_raw)
    {
        return execute_get_objects_and_flatten(
            &mut inner,
            conn_handle,
            DEPTH_DB_SCHEMAS,
            None,
            None,
            None,
            vec![],
        );
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
    // left NULL so it matches every schema in that database. Identifier mode
    // (metadata_id=TRUE) still requires non-NULL args → HY009.
    let catalog_raw = if metadata_id {
        catalog_raw
    } else {
        resolve_null_catalog_to_connection_context(catalog_raw, conn_handle)?
    };
    let catalog_pattern = catalog_arg_to_pattern(catalog_raw.as_deref(), metadata_id)?;
    let schema_pattern = catalog_arg_to_pattern(schema_raw.as_deref(), metadata_id)?;
    let table_pattern = catalog_arg_to_pattern(table_raw.as_deref(), metadata_id)?;

    // TableType is always a value list, not a pattern
    let table_types = parse_table_type_list(type_raw.as_deref());

    execute_get_objects_and_flatten(
        &mut inner,
        conn_handle,
        DEPTH_TABLES,
        catalog_pattern,
        schema_pattern,
        table_pattern,
        table_types,
    )
}

/// Substitute a NULL catalog with the connection's current database.
///
/// ODBC NULL means "use connection context" for catalog functions; the core
/// engine treats NULL as account-wide, which omits databases the role can't see
/// via account-wide SHOW. This mirrors the legacy driver's
/// `SFSemantics::GetFilterForNullCatalog` (gated by its `UseCurrentCatalog`
/// config): it substitutes the **catalog only**. A NULL schema is left NULL so it
/// matches all schemas in the database — legacy has no equivalent
/// null-schema override, and schema-from-session is the core's
/// `CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX` path, which we leave to the core.
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
    // metacharacters so the core's is_exact() recovers it and issues IN DATABASE
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

    if inner.state.as_ref().is_need_data() {
        return InvalidDuringDaeSnafu.fail();
    }
    if inner.state.as_ref().is_async_executing() {
        return AsyncInProgressSnafu.fail();
    }
    if inner.state.as_ref().has_open_cursor() {
        return CursorAlreadyOpenSnafu.fail();
    }

    let conn_handle = match &conn.state {
        ConnectionState::Connected { conn_handle, .. } => *conn_handle,
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
    };

    let metadata_id = inner.metadata_id;
    let numeric_settings = conn.numeric_settings;
    drop(conn);

    let catalog_raw = if metadata_id {
        catalog_raw
    } else {
        resolve_null_catalog_to_connection_context(catalog_raw, conn_handle)?
    };
    let catalog_pattern = catalog_arg_to_pattern(catalog_raw.as_deref(), metadata_id)?;
    let schema_pattern = catalog_arg_to_pattern(schema_raw.as_deref(), metadata_id)?;
    let table_pattern = catalog_arg_to_pattern(table_raw.as_deref(), metadata_id)?;
    let column_pattern = catalog_arg_to_pattern(column_raw.as_deref(), metadata_id)?;

    execute_get_columns_and_flatten(
        &mut inner,
        conn_handle,
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
        let field_matches = |want: Option<&str>, got: Option<&str>| match want {
            Some(want) => got == Some(want),
            None => true,
        };
        field_matches(self.catalog, row_catalog)
            && field_matches(self.schema, row_schema)
            && field_matches(self.table, row_table)
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

/// Logs `"{name}: exit"` at INFO when dropped — pair with an entry log at the top
/// of a public wrapper API function per the logging guidelines.
struct ApiExitLog(&'static str);

impl Drop for ApiExitLog {
    fn drop(&mut self) {
        tracing::info!("{}: exit", self.0);
    }
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
// Call core ConnectionGetObjects, fetch stream, flatten, set state
// ============================================================================

fn execute_get_objects_and_flatten(
    inner: &mut StatementInner,
    conn_handle: sf_core::protobuf::generated::database_driver_v1::ConnectionHandle,
    depth: i32,
    catalog: Option<String>,
    db_schema: Option<String>,
    table_name: Option<String>,
    table_type: Vec<String>,
) -> OdbcResult<()> {
    let rt = global().context(OdbcRuntimeSnafu)?;

    let response = rt.block_on(async |c| {
        c.connection_get_objects(ConnectionGetObjectsRequest {
            conn_handle: Some(conn_handle),
            depth,
            catalog,
            db_schema,
            table_name,
            table_type,
            column_name: None,
        })
        .await
    })?;

    let rs_handle: ResultSetHandle =
        response
            .result_set_handle
            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                message: "ConnectionGetObjects: missing result_set_handle".to_string(),
                location: snafu::location!(),
            })?;

    // Fetch the Arrow stream from the result set handle
    let stream_ptr = {
        let stream_resp = rt.block_on(async |c| {
            c.result_set_get_stream(ResultSetGetStreamRequest {
                result_set_handle: Some(rs_handle),
            })
            .await
        })?;
        // Release is best-effort
        let _ = rt.block_on(async |c| {
            c.result_set_release(ResultSetReleaseRequest {
                result_set_handle: Some(rs_handle),
            })
            .await
        });
        stream_resp
            .stream
            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                message: "ConnectionGetObjects: missing stream".to_string(),
                location: snafu::location!(),
            })?
    };

    // Convert to ArrowArrayStreamReader
    let raw_ptr: *mut FFI_ArrowArrayStream = stream_ptr.into();
    let owned_stream = unsafe { FFI_ArrowArrayStream::from_raw(raw_ptr) };
    let reader = ArrowArrayStreamReader::try_new(owned_stream)
        .context(ArrowArrayStreamReaderCreationSnafu)?;

    // Read all batches from the nested Arrow stream
    let nested_batch = collect_nested_batch(Box::new(reader))?;

    // Flatten the nested batch into the flat 5-col ODBC result
    let flat_batch = flatten_to_odbc(nested_batch, depth)?;

    // Create an ArrowArrayStreamReader from the flat RecordBatch
    let schema = flat_batch.schema();
    let flat_reader = reader_from_record_batch(flat_batch, schema)?;

    let new_state = StatementState::QueryExecuted {
        reader: flat_reader,
        rows_affected: Some(-1),
        origin: ExecutionOrigin::Direct,
    };
    set_state_for_catalog(inner, new_state);

    Ok(())
}

fn execute_get_columns_and_flatten(
    inner: &mut StatementInner,
    conn_handle: sf_core::protobuf::generated::database_driver_v1::ConnectionHandle,
    numeric_settings: NumericSettings,
    catalog: Option<String>,
    db_schema: Option<String>,
    table_name: Option<String>,
    column_name: Option<String>,
) -> OdbcResult<()> {
    let rt = global().context(OdbcRuntimeSnafu)?;

    let response = rt.block_on(async |c| {
        c.connection_get_objects(ConnectionGetObjectsRequest {
            conn_handle: Some(conn_handle),
            depth: DEPTH_COLUMNS,
            catalog,
            db_schema,
            table_name,
            table_type: vec![],
            column_name,
        })
        .await
    })?;

    let rs_handle: ResultSetHandle =
        response
            .result_set_handle
            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                message: "ConnectionGetObjects: missing result_set_handle".to_string(),
                location: snafu::location!(),
            })?;

    let stream_ptr = {
        let stream_resp = rt.block_on(async |c| {
            c.result_set_get_stream(ResultSetGetStreamRequest {
                result_set_handle: Some(rs_handle),
            })
            .await
        })?;
        let _ = rt.block_on(async |c| {
            c.result_set_release(ResultSetReleaseRequest {
                result_set_handle: Some(rs_handle),
            })
            .await
        });
        stream_resp
            .stream
            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                message: "ConnectionGetObjects: missing stream".to_string(),
                location: snafu::location!(),
            })?
    };

    let raw_ptr: *mut FFI_ArrowArrayStream = stream_ptr.into();
    let owned_stream = unsafe { FFI_ArrowArrayStream::from_raw(raw_ptr) };
    let reader = ArrowArrayStreamReader::try_new(owned_stream)
        .context(ArrowArrayStreamReaderCreationSnafu)?;

    let nested_batch = collect_nested_batch(Box::new(reader))?;
    let flat_batch = flatten_columns_to_odbc(nested_batch, &numeric_settings)?;

    let schema = flat_batch.schema();
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
// Flatten nested ADBC Arrow → flat 19-col SQLColumns result
// ============================================================================

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
    user_data_type: Option<String>,
}

fn flatten_columns_to_odbc(
    batch: RecordBatch,
    numeric_settings: &NumericSettings,
) -> OdbcResult<RecordBatch> {
    let schema = flat_columns_schema();

    if batch.num_rows() == 0 {
        return build_flat_columns_batch(schema, vec![]);
    }

    // Rows arrive in (catalog, schema, table, ordinal) order because core builds
    // them from BTreeMaps (lexicographic) with sequential per-table ordinals.
    // No sort is needed here.
    let mut rows: Vec<FlatColumnRow> = Vec::new();

    let cat_arr = batch
        .column(0)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| crate::api::error::OdbcError::InternalError {
            message: "Expected StringArray for catalog_name".to_string(),
            location: snafu::location!(),
        })?;

    let schemas_list = batch
        .column(1)
        .as_any()
        .downcast_ref::<LargeListArray>()
        .ok_or_else(|| crate::api::error::OdbcError::InternalError {
            message: "Expected LargeListArray for catalog_db_schemas".to_string(),
            location: snafu::location!(),
        })?;

    for cat_idx in 0..batch.num_rows() {
        let cat_name = if cat_arr.is_null(cat_idx) {
            None
        } else {
            Some(cat_arr.value(cat_idx).to_string())
        };

        if schemas_list.is_null(cat_idx) {
            continue;
        }

        let sch_start = schemas_list.value_offsets()[cat_idx] as usize;
        let sch_end = schemas_list.value_offsets()[cat_idx + 1] as usize;

        let schemas_struct = schemas_list
            .values()
            .as_any()
            .downcast_ref::<StructArray>()
            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                message: "Expected StructArray for catalog_db_schemas values".to_string(),
                location: snafu::location!(),
            })?;

        let schema_name_arr = schemas_struct
            .column_by_name(FIELD_DB_SCHEMA_NAME)
            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                message: format!("Missing {FIELD_DB_SCHEMA_NAME}"),
                location: snafu::location!(),
            })?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                message: format!("Expected StringArray for {FIELD_DB_SCHEMA_NAME}"),
                location: snafu::location!(),
            })?;

        let tables_list = schemas_struct
            .column_by_name(FIELD_DB_SCHEMA_TABLES)
            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                message: format!("Missing {FIELD_DB_SCHEMA_TABLES}"),
                location: snafu::location!(),
            })?
            .as_any()
            .downcast_ref::<LargeListArray>()
            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                message: format!("Expected LargeListArray for {FIELD_DB_SCHEMA_TABLES}"),
                location: snafu::location!(),
            })?;

        for sch_idx in sch_start..sch_end {
            let sch_name = if schema_name_arr.is_null(sch_idx) {
                None
            } else {
                Some(schema_name_arr.value(sch_idx).to_string())
            };

            if tables_list.is_null(sch_idx) {
                continue;
            }

            let tbl_start = tables_list.value_offsets()[sch_idx] as usize;
            let tbl_end = tables_list.value_offsets()[sch_idx + 1] as usize;

            let tables_struct = tables_list
                .values()
                .as_any()
                .downcast_ref::<StructArray>()
                .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                    message: "Expected StructArray for table values".to_string(),
                    location: snafu::location!(),
                })?;

            let tbl_name_arr = tables_struct
                .column_by_name(FIELD_TABLE_NAME)
                .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                    message: format!("Missing {FIELD_TABLE_NAME}"),
                    location: snafu::location!(),
                })?
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                    message: format!("Expected StringArray for {FIELD_TABLE_NAME}"),
                    location: snafu::location!(),
                })?;

            let cols_list = tables_struct
                .column_by_name(FIELD_TABLE_COLUMNS)
                .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                    message: format!("Missing {FIELD_TABLE_COLUMNS}"),
                    location: snafu::location!(),
                })?
                .as_any()
                .downcast_ref::<LargeListArray>()
                .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                    message: format!("Expected LargeListArray for {FIELD_TABLE_COLUMNS}"),
                    location: snafu::location!(),
                })?;

            for tbl_idx in tbl_start..tbl_end {
                let tbl_name = if tbl_name_arr.is_null(tbl_idx) {
                    None
                } else {
                    Some(tbl_name_arr.value(tbl_idx).to_string())
                };

                if cols_list.is_null(tbl_idx) {
                    continue;
                }

                let col_start = cols_list.value_offsets()[tbl_idx] as usize;
                let col_end = cols_list.value_offsets()[tbl_idx + 1] as usize;

                let cols_struct = cols_list
                    .values()
                    .as_any()
                    .downcast_ref::<StructArray>()
                    .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                        message: format!("Expected StructArray for {FIELD_TABLE_COLUMNS} values"),
                        location: snafu::location!(),
                    })?;

                // Pre-downcast all column arrays once per table.
                macro_rules! col_arr {
                    ($field:expr, $ty:ty) => {
                        cols_struct
                            .column_by_name($field)
                            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                                message: format!("Missing column field {}", $field),
                                location: snafu::location!(),
                            })?
                            .as_any()
                            .downcast_ref::<$ty>()
                            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                                message: format!("Wrong array type for {}", $field),
                                location: snafu::location!(),
                            })?
                    };
                }

                let col_name_arr = col_arr!(FIELD_COLUMN_NAME, StringArray);
                let ordinal_arr = col_arr!(FIELD_COLUMN_ORDINAL_POSITION, Int32Array);
                let logical_type_arr = col_arr!(FIELD_COLUMN_LOGICAL_TYPE, StringArray);
                let precision_arr = col_arr!(FIELD_COLUMN_PRECISION, Int32Array);
                let scale_arr = col_arr!(FIELD_COLUMN_SCALE, Int32Array);
                let char_len_arr = col_arr!(FIELD_COLUMN_CHAR_LENGTH, Int64Array);
                let byte_len_arr = col_arr!(FIELD_COLUMN_BYTE_LENGTH, Int64Array);
                let nullable_arr = col_arr!(FIELD_COLUMN_NULLABLE, BooleanArray);
                let col_def_arr = col_arr!(FIELD_COLUMN_DEF, StringArray);
                let remarks_arr = col_arr!(FIELD_COLUMN_REMARKS, StringArray);

                for col_idx in col_start..col_end {
                    let col_name = if col_name_arr.is_null(col_idx) {
                        None
                    } else {
                        Some(col_name_arr.value(col_idx).to_string())
                    };
                    let ordinal = ordinal_arr.value(col_idx);
                    let logical_type = if logical_type_arr.is_null(col_idx) {
                        ""
                    } else {
                        logical_type_arr.value(col_idx)
                    };
                    let precision = if precision_arr.is_null(col_idx) {
                        None
                    } else {
                        Some(precision_arr.value(col_idx))
                    };
                    let scale = if scale_arr.is_null(col_idx) {
                        None
                    } else {
                        Some(scale_arr.value(col_idx))
                    };
                    let char_length = if char_len_arr.is_null(col_idx) {
                        None
                    } else {
                        Some(char_len_arr.value(col_idx))
                    };
                    let byte_length = if byte_len_arr.is_null(col_idx) {
                        None
                    } else {
                        Some(byte_len_arr.value(col_idx))
                    };
                    let nullable = nullable_arr.value(col_idx);
                    let col_def = if col_def_arr.is_null(col_idx) {
                        None
                    } else {
                        Some(col_def_arr.value(col_idx).to_string())
                    };
                    let col_remarks = if remarks_arr.is_null(col_idx) {
                        None
                    } else {
                        Some(remarks_arr.value(col_idx).to_string())
                    };

                    let field = rehydrate_field(
                        logical_type,
                        precision,
                        scale,
                        char_length,
                        byte_length,
                        nullable,
                    );

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
                    // DECIMAL_DIGITS: scale 0 is a valid, meaningful value for exact-numeric
                    // columns (e.g. NUMBER(38,0)) — report it as "0", not NULL. The helper
                    // returns Err for types where DECIMAL_DIGITS is inapplicable (→ NULL).
                    let dec_digits_val = decimal_digits_from_field(&field, numeric_settings)
                        .ok()
                        .map(|s| s.to_string());
                    // NUM_PREC_RADIX: only ever 2, 10, or inapplicable (→ NULL); 0 is never
                    // a meaningful value, so collapsing 0 → NULL is harmless here.
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

                    let nullable_str = if nullable { "1" } else { "0" };
                    let is_nullable_str = if nullable { "YES" } else { "NO" };
                    // USER_DATA_TYPE: mirror DATA_TYPE (driver-specific; tests only assert presence).
                    let user_data_type_val = data_type_val.clone();

                    rows.push(FlatColumnRow {
                        cat: cat_name.clone(),
                        schem: sch_name.clone(),
                        tbl: tbl_name.clone(),
                        col_name,
                        data_type: data_type_val,
                        type_name: type_name_val,
                        col_size: col_size_val,
                        buf_len: buf_len_val,
                        dec_digits: dec_digits_val,
                        num_prec_radix: num_prec_radix_val,
                        nullable: Some(nullable_str.to_string()),
                        remarks: col_remarks,
                        col_def,
                        sql_data_type: sql_data_type_val,
                        sql_dt_sub: sql_dt_sub_val,
                        char_octet: char_octet_val,
                        ordinal: Some(ordinal.to_string()),
                        is_nullable: Some(is_nullable_str.to_string()),
                        user_data_type: user_data_type_val,
                    });
                }
            }
        }
    }

    build_flat_columns_batch(schema, rows)
}

fn build_flat_columns_batch(
    schema: SchemaRef,
    rows: Vec<FlatColumnRow>,
) -> OdbcResult<RecordBatch> {
    fn to_array(v: Vec<Option<String>>) -> ArrayRef {
        Arc::new(StringArray::from(v)) as ArrayRef
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
            to_array(cats),
            to_array(schms),
            to_array(tbls),
            to_array(col_names),
            to_array(data_types),
            to_array(type_names),
            to_array(col_sizes),
            to_array(buf_lens),
            to_array(dec_digits),
            to_array(num_prec_radixes),
            to_array(nullables),
            to_array(remarks),
            to_array(col_defs),
            to_array(sql_data_types),
            to_array(sql_dt_subs),
            to_array(char_octets),
            to_array(ordinals),
            to_array(is_nullables),
            to_array(user_data_types),
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

// ============================================================================
// Flatten nested ADBC Arrow → flat 5-col ODBC result
// ============================================================================

fn flatten_to_odbc(batch: RecordBatch, depth: i32) -> OdbcResult<RecordBatch> {
    let schema = flat_tables_schema();

    let mut cats: Vec<Option<String>> = Vec::new();
    let mut schms: Vec<Option<String>> = Vec::new();
    let mut tbls: Vec<Option<String>> = Vec::new();
    let mut types: Vec<Option<String>> = Vec::new();
    let mut remarks: Vec<Option<String>> = Vec::new();

    if batch.num_rows() == 0 {
        return build_flat_batch(schema, cats, schms, tbls, types, remarks);
    }

    // catalog_name column (index 0)
    let cat_arr = batch
        .column(0)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| crate::api::error::OdbcError::InternalError {
            message: format!(
                "Expected StringArray for {}, got {:?}",
                FIELD_CATALOG_NAME,
                batch.column(0).data_type()
            ),
            location: snafu::location!(),
        })?;

    // catalog_db_schemas column (index 1) as LargeListArray
    let schemas_list = batch
        .column(1)
        .as_any()
        .downcast_ref::<LargeListArray>()
        .ok_or_else(|| crate::api::error::OdbcError::InternalError {
            message: format!(
                "Expected LargeListArray for {}, got {:?}",
                FIELD_CATALOG_DB_SCHEMAS,
                batch.column(1).data_type()
            ),
            location: snafu::location!(),
        })?;

    for cat_idx in 0..batch.num_rows() {
        let cat_name = if cat_arr.is_null(cat_idx) {
            None
        } else {
            Some(cat_arr.value(cat_idx).to_string())
        };

        if depth == DEPTH_CATALOGS {
            cats.push(cat_name);
            schms.push(None);
            tbls.push(None);
            types.push(None);
            remarks.push(None);
            continue;
        }

        if schemas_list.is_null(cat_idx) {
            continue;
        }

        let sch_start = schemas_list.value_offsets()[cat_idx] as usize;
        let sch_end = schemas_list.value_offsets()[cat_idx + 1] as usize;

        use arrow::array::StructArray;
        let schemas_struct = schemas_list
            .values()
            .as_any()
            .downcast_ref::<StructArray>()
            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                message: "Expected StructArray for catalog_db_schemas values".to_string(),
                location: snafu::location!(),
            })?;

        let schema_name_arr = schemas_struct
            .column_by_name(FIELD_DB_SCHEMA_NAME)
            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                message: format!("Missing {} column", FIELD_DB_SCHEMA_NAME),
                location: snafu::location!(),
            })?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                message: format!("Expected StringArray for {}", FIELD_DB_SCHEMA_NAME),
                location: snafu::location!(),
            })?;

        let tables_list = schemas_struct
            .column_by_name(FIELD_DB_SCHEMA_TABLES)
            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                message: format!("Missing {} column", FIELD_DB_SCHEMA_TABLES),
                location: snafu::location!(),
            })?
            .as_any()
            .downcast_ref::<LargeListArray>()
            .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                message: format!("Expected LargeListArray for {}", FIELD_DB_SCHEMA_TABLES),
                location: snafu::location!(),
            })?;

        for sch_idx in sch_start..sch_end {
            let sch_name = if schema_name_arr.is_null(sch_idx) {
                None
            } else {
                Some(schema_name_arr.value(sch_idx).to_string())
            };

            if depth == DEPTH_DB_SCHEMAS {
                cats.push(cat_name.clone());
                schms.push(sch_name);
                tbls.push(None);
                types.push(None);
                remarks.push(None);
                continue;
            }

            // DEPTH_TABLES
            if tables_list.is_null(sch_idx) {
                continue;
            }

            let tbl_start = tables_list.value_offsets()[sch_idx] as usize;
            let tbl_end = tables_list.value_offsets()[sch_idx + 1] as usize;

            let tables_struct = tables_list
                .values()
                .as_any()
                .downcast_ref::<StructArray>()
                .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                    message: "Expected StructArray for table values".to_string(),
                    location: snafu::location!(),
                })?;

            let tbl_name_arr = tables_struct
                .column_by_name(FIELD_TABLE_NAME)
                .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                    message: format!("Missing {} column", FIELD_TABLE_NAME),
                    location: snafu::location!(),
                })?
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                    message: format!("Expected StringArray for {}", FIELD_TABLE_NAME),
                    location: snafu::location!(),
                })?;

            let tbl_type_arr = tables_struct
                .column_by_name(FIELD_TABLE_TYPE)
                .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                    message: format!("Missing {} column", FIELD_TABLE_TYPE),
                    location: snafu::location!(),
                })?
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| crate::api::error::OdbcError::InternalError {
                    message: format!("Expected StringArray for {}", FIELD_TABLE_TYPE),
                    location: snafu::location!(),
                })?;

            for tbl_idx in tbl_start..tbl_end {
                let tbl_name = if tbl_name_arr.is_null(tbl_idx) {
                    None
                } else {
                    Some(tbl_name_arr.value(tbl_idx).to_string())
                };
                let tbl_type = if tbl_type_arr.is_null(tbl_idx) {
                    None
                } else {
                    Some(tbl_type_arr.value(tbl_idx).to_string())
                };

                cats.push(cat_name.clone());
                schms.push(sch_name.clone());
                tbls.push(tbl_name);
                types.push(tbl_type);
                remarks.push(Some(String::new()));
            }
        }
    }

    // Sort by TABLE_TYPE, TABLE_CAT, TABLE_SCHEM, TABLE_NAME per ODBC spec
    if depth == DEPTH_TABLES && !cats.is_empty() {
        let mut rows: Vec<FlatTableRow> = cats
            .drain(..)
            .zip(schms.drain(..))
            .zip(tbls.drain(..))
            .zip(types.drain(..))
            .zip(remarks.drain(..))
            .map(|((((c, s), t), ty), r)| (c, s, t, ty, r))
            .collect();
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
        for (c, s, t, ty, r) in rows {
            cats.push(c);
            schms.push(s);
            tbls.push(t);
            types.push(ty);
            remarks.push(r);
        }
    }

    build_flat_batch(schema, cats, schms, tbls, types, remarks)
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
    fn all_types_returns_23_rows_and_20_columns() {
        let all: Vec<&TypeInfoRow> = ALL_SF_TYPE_INFO.iter().collect();
        let batch = build_type_info_batch(&all).expect("batch build failed");
        assert_eq!(batch.num_rows(), 23);
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
        // (types[0..22]).
        let expected: &[i16] = &[
            1, 2, 3, 4, -5, 6, 7, 8, 12, -2, -3, 91, 92, 2000, 2002, 2001, 93, 2003, 2004, 2005,
            -8, -9, -7,
        ];
        let actual: Vec<i16> = ALL_SF_TYPE_INFO.iter().map(|r| r.data_type).collect();
        assert_eq!(actual, expected);
    }
}
