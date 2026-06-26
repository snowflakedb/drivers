//! GetObjects engine — implements the ADBC-shaped `ConnectionGetObjects` RPC.
//!
//! This is the shared metadata engine used by all driver wrappers (ODBC, JDBC, ADBC).
//! It executes `SHOW` commands against Snowflake and returns a nested Arrow result set.
//!
//! ## Depth constants
//! - `DEPTH_CATALOGS`   (1): list databases
//! - `DEPTH_DB_SCHEMAS` (2): list schemas
//! - `DEPTH_TABLES`     (3): list tables/views
//! - `DEPTH_COLUMNS`    (4): deferred
//!
//! ## Result schema (nested ADBC format)
//! `catalog_name: utf8`
//! `catalog_db_schemas: list<struct<db_schema_name, db_schema_tables: list<struct<...>>>>`
//!
//! See `nested_get_objects_schema()` and the `FIELD_*` constants for the exact schema.

use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};

use arrow::array::{
    Array, ArrayRef, LargeListArray, LargeStringArray, RecordBatch, RecordBatchReader, StringArray,
    StructArray, TimestampMicrosecondArray, TimestampNanosecondArray, new_empty_array,
};
use arrow::buffer::{NullBuffer, OffsetBuffer, ScalarBuffer};
use arrow::datatypes::{DataType, Field, Fields, Schema, SchemaRef};
use snafu::{OptionExt, ResultExt};
use tokio::sync::Mutex;

use super::connection::{Connection, with_valid_session};
use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::like_pattern;
use crate::chunks::PrefetchConfig;
use crate::handle_manager::Handle;
use crate::rest::snowflake::{
    QueryExecutionMode, QueryInput, RestError, snowflake_query_with_client,
};

// ---------------------------------------------------------------------------
// Depth constants (public — used by wrapper to map SQLTables special cases)
// ---------------------------------------------------------------------------

pub const DEPTH_CATALOGS: i32 = 1;
pub const DEPTH_DB_SCHEMAS: i32 = 2;
pub const DEPTH_TABLES: i32 = 3;
pub const DEPTH_COLUMNS: i32 = 4; // deferred

// ---------------------------------------------------------------------------
// Arrow field-name constants (public — wrapper flatten uses them to avoid drift)
// ---------------------------------------------------------------------------

pub const FIELD_CATALOG_NAME: &str = "catalog_name";
pub const FIELD_CATALOG_DB_SCHEMAS: &str = "catalog_db_schemas";
pub const FIELD_DB_SCHEMA_NAME: &str = "db_schema_name";
pub const FIELD_DB_SCHEMA_TABLES: &str = "db_schema_tables";
pub const FIELD_TABLE_NAME: &str = "table_name";
pub const FIELD_TABLE_TYPE: &str = "table_type";
pub const FIELD_TABLE_COLUMNS: &str = "table_columns";
pub const FIELD_TABLE_CONSTRAINTS: &str = "table_constraints";

// ---------------------------------------------------------------------------
// Nested ADBC Arrow schema (single source of truth, cached)
// ---------------------------------------------------------------------------

fn table_fields() -> Fields {
    Fields::from(vec![
        Field::new(FIELD_TABLE_NAME, DataType::Utf8, true),
        Field::new(FIELD_TABLE_TYPE, DataType::Utf8, true),
        Field::new(
            FIELD_TABLE_COLUMNS,
            DataType::LargeList(Arc::new(Field::new("item", DataType::Utf8, true))),
            true,
        ),
        Field::new(
            FIELD_TABLE_CONSTRAINTS,
            DataType::LargeList(Arc::new(Field::new("item", DataType::Utf8, true))),
            true,
        ),
    ])
}

fn schema_fields() -> Fields {
    Fields::from(vec![
        Field::new(FIELD_DB_SCHEMA_NAME, DataType::Utf8, true),
        Field::new(
            FIELD_DB_SCHEMA_TABLES,
            DataType::LargeList(Arc::new(Field::new(
                "item",
                DataType::Struct(table_fields()),
                true,
            ))),
            true,
        ),
    ])
}

/// The nested Arrow schema returned by `connection_get_objects`.
/// The wrapper flatten reads field names from the `FIELD_*` constants above,
/// never from hard-coded strings, to ensure producer/consumer stay in sync.
pub fn nested_get_objects_schema() -> SchemaRef {
    static SCHEMA: LazyLock<SchemaRef> = LazyLock::new(|| {
        Arc::new(Schema::new(vec![
            Field::new(FIELD_CATALOG_NAME, DataType::Utf8, true),
            Field::new(
                FIELD_CATALOG_DB_SCHEMAS,
                DataType::LargeList(Arc::new(Field::new(
                    "item",
                    DataType::Struct(schema_fields()),
                    true,
                ))),
                true,
            ),
        ]))
    });
    SCHEMA.clone()
}

// ---------------------------------------------------------------------------
// Public request type
// ---------------------------------------------------------------------------

pub struct GetObjectsRequest {
    pub conn_handle: Handle,
    pub depth: i32,
    pub catalog: Option<String>,
    pub db_schema: Option<String>,
    pub table_name: Option<String>,
    pub table_type: Vec<String>,
}

// ---------------------------------------------------------------------------
// kind → TABLE_TYPE normalization
// ---------------------------------------------------------------------------

fn normalize_kind(kind: &str) -> &'static str {
    match kind.to_uppercase().as_str() {
        "TABLE" | "TRANSIENT TABLE" | "TEMPORARY TABLE" | "EXTERNAL TABLE" | "ICEBERG TABLE"
        | "EVENT TABLE" | "HYBRID TABLE" | "MATERIALIZED TABLE" => "TABLE",
        "VIEW" | "MATERIALIZED VIEW" | "SECURE VIEW" => "VIEW",
        _ => "TABLE",
    }
}

// ---------------------------------------------------------------------------
// SQL building helpers
// ---------------------------------------------------------------------------

/// Escape a Snowflake double-quoted identifier segment (`"` → `""`).
fn escape_dq(s: &str) -> String {
    s.replace('"', "\"\"")
}

/// Escape a SHOW LIKE pattern value for single-quoted Snowflake patterns.
///
/// Snowflake string literals treat `\` as an escape character by default, so a
/// literal backslash must be doubled before the value is wrapped in single
/// quotes. Order matters: escape `\` first, then `'`. Otherwise a trailing
/// backslash (e.g. `AB\`) would escape the closing quote and produce an
/// unterminated literal — which surfaces as SQLSTATE 42000 and is then swallowed
/// as an empty result set, silently returning wrong (empty) metadata.
fn escape_show_like(pattern: &str) -> String {
    pattern.replace('\\', "\\\\").replace('\'', "\\'")
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

impl DatabaseDriverV1 {
    pub async fn connection_get_objects(
        &self,
        req: GetObjectsRequest,
    ) -> Result<super::result_set::ResultSetInfo, ApiError> {
        let conn_ptr = self
            .connections
            .get_obj(req.conn_handle)
            .context(InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            })?;

        let (catalog_filter, schema_filter) = {
            let conn = conn_ptr.lock().await;
            apply_connection_context(&conn, req.catalog, req.db_schema).await
        };

        // `depth` arrives as a raw proto i32. Dispatch only the implemented depths
        // explicitly; reject COLUMNS (deferred) and unknown values rather than
        // letting them fall through to a TABLES catch-all.
        let batch = match req.depth {
            DEPTH_CATALOGS => fetch_catalogs(&conn_ptr, catalog_filter.as_deref()).await?,
            DEPTH_DB_SCHEMAS => {
                fetch_schemas(
                    &conn_ptr,
                    catalog_filter.as_deref(),
                    schema_filter.as_deref(),
                )
                .await?
            }
            DEPTH_TABLES => {
                let table_types = normalize_table_types(&req.table_type);
                fetch_tables(
                    &conn_ptr,
                    catalog_filter.as_deref(),
                    schema_filter.as_deref(),
                    req.table_name.as_deref(),
                    &table_types,
                )
                .await?
            }
            DEPTH_COLUMNS => {
                return InvalidArgumentSnafu {
                    argument: "GetObjects depth COLUMNS (4) is not implemented".to_string(),
                }
                .fail();
            }
            other => {
                return InvalidArgumentSnafu {
                    argument: format!("GetObjects depth {other} is invalid (expected 1..=3)"),
                }
                .fail();
            }
        };

        let http_client = {
            let conn = conn_ptr.lock().await;
            conn.http_client
                .clone()
                .context(ConnectionNotInitializedSnafu)?
        };

        self.register_arrow_batch_as_result_set(&batch, http_client)
    }
}

// ---------------------------------------------------------------------------
// Connection-context substitution
// ---------------------------------------------------------------------------

async fn apply_connection_context(
    conn: &Connection,
    catalog: Option<String>,
    schema: Option<String>,
) -> (Option<String>, Option<String>) {
    let use_ctx = {
        let cache = conn.session_parameters.read().await;
        cache
            .get("CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX")
            .map(|v| v.to_uppercase() == "TRUE")
            .unwrap_or(false)
    };

    if !use_ctx {
        return (catalog, schema);
    }

    let final_names = conn.final_session_names.read().ok();

    let resolved_catalog =
        catalog.or_else(|| final_names.as_ref().and_then(|n| n.database.clone()));
    let resolved_schema = schema.or_else(|| final_names.as_ref().and_then(|n| n.schema.clone()));

    (resolved_catalog, resolved_schema)
}

// ---------------------------------------------------------------------------
// CATALOGS depth
// ---------------------------------------------------------------------------

async fn fetch_catalogs(
    conn_ptr: &Arc<Mutex<Connection>>,
    catalog_filter: Option<&str>,
) -> Result<RecordBatch, ApiError> {
    let sql = "SHOW DATABASES IN ACCOUNT".to_string();
    let rows = execute_show(conn_ptr, &sql).await?;

    let catalog_names: Vec<Option<String>> = rows
        .iter()
        .filter_map(|row| {
            let name = get_column(row, "name")?;
            if let Some(pattern) = catalog_filter
                && !like_pattern::matches(pattern, name)
            {
                return None;
            }
            Some(Some(name.to_string()))
        })
        .collect();

    build_catalogs_batch(catalog_names)
}

// ---------------------------------------------------------------------------
// DB_SCHEMAS depth
// ---------------------------------------------------------------------------

async fn fetch_schemas(
    conn_ptr: &Arc<Mutex<Connection>>,
    catalog_filter: Option<&str>,
    schema_filter: Option<&str>,
) -> Result<RecordBatch, ApiError> {
    // Pick tightest scope: exact catalog -> IN DATABASE "db", else IN ACCOUNT
    let sql = if let Some(pattern) = catalog_filter {
        if let Some(literal) = like_pattern::is_exact(pattern) {
            if !literal.is_empty() {
                format!("SHOW SCHEMAS IN DATABASE \"{}\"", escape_dq(&literal))
            } else {
                "SHOW SCHEMAS IN ACCOUNT".to_string()
            }
        } else {
            "SHOW SCHEMAS IN ACCOUNT".to_string()
        }
    } else {
        "SHOW SCHEMAS IN ACCOUNT".to_string()
    };

    let rows = execute_show(conn_ptr, &sql).await?;

    let mut by_catalog: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for row in &rows {
        let db_name = match get_column(row, "database_name") {
            Some(n) => n.to_string(),
            None => continue,
        };
        let schema_name = match get_column(row, "name") {
            Some(n) => n.to_string(),
            None => continue,
        };

        if let Some(pattern) = catalog_filter
            && !like_pattern::matches(pattern, &db_name)
        {
            continue;
        }
        if let Some(pattern) = schema_filter
            && !like_pattern::matches(pattern, &schema_name)
        {
            continue;
        }

        by_catalog.entry(db_name).or_default().push(schema_name);
    }

    build_schemas_batch(by_catalog)
}

// ---------------------------------------------------------------------------
// TABLES depth
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum TableTypeFilter {
    All,
    Explicit(Vec<String>), // "TABLE" and/or "VIEW"
    /// Caller supplied table-type keywords, but none were TABLE or VIEW (e.g. "SYNONYM").
    /// Legacy returns an empty result set rather than falling back to All.
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

async fn fetch_tables(
    conn_ptr: &Arc<Mutex<Connection>>,
    catalog_filter: Option<&str>,
    schema_filter: Option<&str>,
    table_name_filter: Option<&str>,
    table_types: &TableTypeFilter,
) -> Result<RecordBatch, ApiError> {
    if matches!(table_types, TableTypeFilter::Unsupported) {
        return build_tables_batch(BTreeMap::new());
    }
    // Empty string means "match nothing"; skip the server query (like_pattern::matches("", _) is false).
    if matches!(table_name_filter, Some("")) {
        return build_tables_batch(BTreeMap::new());
    }

    let exact_catalog = catalog_filter
        .and_then(like_pattern::is_exact)
        .filter(|s| !s.is_empty());
    let exact_schema = schema_filter
        .and_then(like_pattern::is_exact)
        .filter(|s| !s.is_empty());

    // Always issue `SHOW OBJECTS`: it lists both tables and views with a
    // reliable `kind` column. `SHOW VIEWS` omits `kind` on some Snowflake
    // versions, so switching commands per requested type leaves the row
    // loop unable to tell tables and views apart. The TABLE/VIEW distinction
    // is applied client-side via the `kind`-derived `normalized_type` below.
    let like_clause = build_like_clause(table_name_filter);
    let scope = match (&exact_catalog, &exact_schema) {
        (Some(cat), Some(sch)) => {
            format!("IN SCHEMA \"{}\".\"{}\"", escape_dq(cat), escape_dq(sch))
        }
        (Some(cat), None) => format!("IN DATABASE \"{}\"", escape_dq(cat)),
        _ => "IN ACCOUNT".to_string(),
    };
    let rows = execute_show(
        conn_ptr,
        &format_show_sql("SHOW OBJECTS", &like_clause, &scope),
    )
    .await?;

    let mut by_cat_sch: BTreeMap<String, BTreeMap<String, Vec<(String, String)>>> = BTreeMap::new();

    for row in &rows {
        let db_name = match get_column(row, "database_name") {
            Some(n) => n.to_string(),
            None => continue,
        };
        let sch_name = match get_column(row, "schema_name") {
            Some(n) => n.to_string(),
            None => continue,
        };
        let tbl_name = match get_column(row, "name") {
            Some(n) => n.to_string(),
            None => continue,
        };
        let kind = get_column(row, "kind").unwrap_or("TABLE");
        let normalized_type = normalize_kind(kind).to_string();

        if let Some(pattern) = catalog_filter
            && !like_pattern::matches(pattern, &db_name)
        {
            continue;
        }
        if let Some(pattern) = schema_filter
            && !like_pattern::matches(pattern, &sch_name)
        {
            continue;
        }
        if let Some(pattern) = table_name_filter
            && !like_pattern::matches(pattern, &tbl_name)
        {
            continue;
        }
        if let TableTypeFilter::Explicit(allowed) = table_types
            && !allowed.contains(&normalized_type)
        {
            continue;
        }

        by_cat_sch
            .entry(db_name)
            .or_default()
            .entry(sch_name)
            .or_default()
            .push((tbl_name, normalized_type));
    }

    build_tables_batch(by_cat_sch)
}

fn build_like_clause(table_name_filter: Option<&str>) -> String {
    match table_name_filter {
        None => String::new(),
        Some(p) => {
            debug_assert!(
                !p.is_empty(),
                "empty table pattern is handled in fetch_tables before build_like_clause"
            );
            // Snowflake SHOW LIKE does not honor `\` escapes. Strip them for
            // coarse server-side narrowing; client-side `like_pattern::matches`
            // re-applies the original pattern (same strategy as the old ODBC driver).
            let coarse = like_pattern::strip_escapes_for_show_like(p);
            format!("LIKE '{}'", escape_show_like(&coarse))
        }
    }
}

/// Snowflake requires `LIKE` before `IN …` (e.g. `SHOW OBJECTS LIKE 'x' IN SCHEMA db.sch`).
fn format_show_sql(show_cmd: &str, like_clause: &str, scope: &str) -> String {
    if like_clause.is_empty() {
        format!("{show_cmd} {scope}")
    } else {
        format!("{show_cmd} {like_clause} {scope}")
    }
}

// ---------------------------------------------------------------------------
// SHOW query execution
// ---------------------------------------------------------------------------

/// SQLSTATEs the legacy ODBC driver treats as "no metadata" for SHOW queries.
const SHOW_NOT_FOUND_SQLSTATES: &[&str] = &["02000", "42000", "42S02"];

fn is_show_not_found_sql_state(sql_state: Option<&str>) -> bool {
    sql_state.is_some_and(|s| SHOW_NOT_FOUND_SQLSTATES.contains(&s))
}

fn api_error_sql_state(err: &ApiError) -> Option<&str> {
    let ApiError::Query { source, .. } = err else {
        return None;
    };
    let RestError::QueryFailed { sql_state, .. } = source.as_ref() else {
        return None;
    };
    sql_state.as_deref()
}

fn map_execute_show_error(
    err: ApiError,
    sql: &str,
) -> Result<Vec<Vec<(String, String)>>, ApiError> {
    if is_show_not_found_sql_state(api_error_sql_state(&err)) {
        tracing::debug!("SHOW query not found (returning empty): {sql}: {err}");
        Ok(Vec::new())
    } else {
        Err(err)
    }
}

/// Executes a SHOW query and returns rows as `Vec<Vec<(column_name, value)>>`.
/// Only maps the legacy "not found / no data" SQLSTATEs to an empty result;
/// all other failures propagate as [`ApiError`].
async fn execute_show(
    conn_ptr: &Arc<Mutex<Connection>>,
    sql: &str,
) -> Result<Vec<Vec<(String, String)>>, ApiError> {
    let (query_parameters, http_client, retry_policy, prefetch_config) = {
        let conn = conn_ptr.lock().await;
        let http_client = conn
            .http_client
            .clone()
            .context(ConnectionNotInitializedSnafu)?;
        let query_parameters = conn.query_transport_parameters()?;
        let retry_policy = conn.retry_policy.clone();
        let session_params = conn.session_parameters.read().await;
        let prefetch_config = PrefetchConfig::from_session_params(&session_params);
        (query_parameters, http_client, retry_policy, prefetch_config)
    };

    let sql_owned = sql.to_string();
    let query_input = QueryInput {
        sql: sql_owned.clone(),
        bindings: None,
        bind_stage: None,
        describe_only: None,
        query_parameters: None,
    };

    let response = with_valid_session(conn_ptr, |token| {
        let http_client = http_client.clone();
        let query_parameters = query_parameters.clone();
        let query_input = query_input.clone();
        let retry_policy = retry_policy.clone();
        async move {
            snowflake_query_with_client(
                &http_client,
                query_parameters,
                token.reveal(),
                query_input,
                &retry_policy,
                QueryExecutionMode::Blocking,
            )
            .await
        }
    })
    .await;

    let response = match response {
        Ok(resp) => resp,
        Err(err) => return map_execute_show_error(err, &sql_owned),
    };

    // Reuse the canonical reader, which downloads and concatenates external
    // result chunks for every rowset shape (JSON/Arrow, single/multi-chunk).
    // Account-wide `SHOW OBJECTS` spills to external chunks; parsing only the
    // inline rowset here would silently drop most rows.
    let rowset_data = response.data.into_rowset_data();
    let reader = super::query::read_batches(&rowset_data, http_client, &prefetch_config)
        .await
        .map_err(|e| {
            InvalidArgumentSnafu {
                argument: format!("SHOW result read failed: {e}"),
            }
            .build()
        })?;
    // The reader drains chunks via `blocking_recv`, which panics if polled on a
    // runtime worker; drain it on a blocking thread while downloads progress on
    // the async workers.
    let parsed = tokio::task::spawn_blocking(move || rows_from_reader(reader))
        .await
        .map_err(|e| {
            InvalidArgumentSnafu {
                argument: format!("SHOW reader join failed: {e}"),
            }
            .build()
        })??;
    tracing::debug!("SHOW query parsed {} rows: {sql_owned}", parsed.len());
    Ok(parsed)
}

// ---------------------------------------------------------------------------
// SHOW response parsing
// ---------------------------------------------------------------------------

/// Drains a record-batch reader into rows of `(column_name, stringified_value)`.
/// Column names are read from each batch's schema so the producer (SHOW result
/// metadata) and consumer (`get_column`) never drift.
fn rows_from_reader(
    reader: Box<dyn RecordBatchReader + Send>,
) -> Result<Vec<Vec<(String, String)>>, ApiError> {
    let mut rows = Vec::new();
    for batch_result in reader {
        let batch = batch_result.context(ArrowParsingSnafu)?;
        let names: Vec<String> = batch
            .schema()
            .fields()
            .iter()
            .map(|f| f.name().clone())
            .collect();
        let col_count = names.len().min(batch.num_columns());
        for row_idx in 0..batch.num_rows() {
            let mut row = Vec::with_capacity(col_count);
            for (col_idx, name) in names.iter().enumerate().take(col_count) {
                let val =
                    cell_as_string(batch.column(col_idx).as_ref(), row_idx).unwrap_or_default();
                row.push((name.clone(), val));
            }
            rows.push(row);
        }
    }
    Ok(rows)
}

fn cell_as_string(column: &dyn Array, row_idx: usize) -> Option<String> {
    if column.is_null(row_idx) {
        return None;
    }
    if let Some(array) = column.as_any().downcast_ref::<StringArray>() {
        return Some(array.value(row_idx).to_string());
    }
    if let Some(array) = column.as_any().downcast_ref::<LargeStringArray>() {
        return Some(array.value(row_idx).to_string());
    }
    if let Some(array) = column.as_any().downcast_ref::<TimestampMicrosecondArray>() {
        return Some(array.value(row_idx).to_string());
    }
    if let Some(array) = column.as_any().downcast_ref::<TimestampNanosecondArray>() {
        return Some(array.value(row_idx).to_string());
    }
    None
}

fn get_column<'a>(row: &'a [(String, String)], name: &str) -> Option<&'a str> {
    let name_upper = name.to_uppercase();
    row.iter()
        .find(|(k, _)| k.to_uppercase() == name_upper)
        .map(|(_, v)| v.as_str())
}

// ---------------------------------------------------------------------------
// Arrow batch builders
// ---------------------------------------------------------------------------

fn build_catalogs_batch(catalog_names: Vec<Option<String>>) -> Result<RecordBatch, ApiError> {
    let schema = nested_get_objects_schema();
    let n = catalog_names.len();

    let mut cat_builder = arrow::array::StringBuilder::new();
    for name in &catalog_names {
        match name {
            Some(n) => cat_builder.append_value(n),
            None => cat_builder.append_null(),
        }
    }
    let cat_array: ArrayRef = Arc::new(cat_builder.finish());

    // catalog_db_schemas: all-null LargeList (CATALOGS depth doesn't populate schemas)
    let null_schemas = build_all_null_schema_list(n)?;

    RecordBatch::try_new(schema, vec![cat_array, null_schemas]).map_err(|e| {
        InvalidArgumentSnafu {
            argument: format!("Arrow error: {e}"),
        }
        .build()
    })
}

fn build_schemas_batch(by_catalog: BTreeMap<String, Vec<String>>) -> Result<RecordBatch, ApiError> {
    let schema = nested_get_objects_schema();

    let mut catalog_names: Vec<String> = Vec::new();
    let mut schema_lists: Vec<Vec<String>> = Vec::new();

    for (cat, schemas) in by_catalog {
        catalog_names.push(cat);
        schema_lists.push(schemas);
    }

    let mut cat_builder = arrow::array::StringBuilder::new();
    for name in &catalog_names {
        cat_builder.append_value(name);
    }
    let cat_array: ArrayRef = Arc::new(cat_builder.finish());

    let db_schemas_array = build_schema_list_array_null_tables(&schema_lists)?;

    RecordBatch::try_new(schema, vec![cat_array, db_schemas_array]).map_err(|e| {
        InvalidArgumentSnafu {
            argument: format!("Arrow error: {e}"),
        }
        .build()
    })
}

fn build_tables_batch(
    by_cat_sch: BTreeMap<String, BTreeMap<String, Vec<(String, String)>>>,
) -> Result<RecordBatch, ApiError> {
    let schema = nested_get_objects_schema();

    let mut catalog_names: Vec<String> = Vec::new();
    let mut schema_maps: Vec<BTreeMap<String, Vec<(String, String)>>> = Vec::new();

    for (cat, schemas) in by_cat_sch {
        catalog_names.push(cat);
        schema_maps.push(schemas);
    }

    let mut cat_builder = arrow::array::StringBuilder::new();
    for name in &catalog_names {
        cat_builder.append_value(name);
    }
    let cat_array: ArrayRef = Arc::new(cat_builder.finish());

    let db_schemas_array = build_full_schema_list_array(&schema_maps)?;

    RecordBatch::try_new(schema, vec![cat_array, db_schemas_array]).map_err(|e| {
        InvalidArgumentSnafu {
            argument: format!("Arrow error: {e}"),
        }
        .build()
    })
}

// ---------------------------------------------------------------------------
// Arrow array builder helpers
// ---------------------------------------------------------------------------

/// Builds an all-null `LargeList<Struct<schema_fields>>` with `count` entries.
fn build_all_null_schema_list(count: usize) -> Result<ArrayRef, ApiError> {
    // Empty-but-typed child struct: zero rows, but it must keep `schema_fields()`
    // so it matches the LargeList item type. `StructArray::new(fields, vec![], None)`
    // panics in Arrow 56 because an empty child-array vec can't carry a length.
    let child_typed = new_empty_array(&DataType::Struct(schema_fields()));

    let offsets = vec![0i64; count + 1];
    let null_buf = NullBuffer::new(arrow::buffer::BooleanBuffer::new(
        arrow::buffer::Buffer::from(vec![0u8; count.div_ceil(8)]),
        0,
        count,
    ));

    let list = LargeListArray::new(
        Arc::new(Field::new("item", DataType::Struct(schema_fields()), true)),
        OffsetBuffer::new(ScalarBuffer::from(offsets)),
        child_typed,
        Some(null_buf),
    );

    Ok(Arc::new(list))
}

/// Builds `LargeList<Struct<db_schema_name, db_schema_tables(null)>>` for DB_SCHEMAS depth.
fn build_schema_list_array_null_tables(schema_lists: &[Vec<String>]) -> Result<ArrayRef, ApiError> {
    let total_schemas: usize = schema_lists.iter().map(|s| s.len()).sum();
    let mut schema_names: Vec<&str> = Vec::with_capacity(total_schemas);
    let mut cat_offsets = Vec::with_capacity(schema_lists.len() + 1);
    cat_offsets.push(0i64);

    for schemas in schema_lists {
        for s in schemas {
            schema_names.push(s.as_str());
        }
        cat_offsets.push(cat_offsets.last().copied().unwrap_or(0) + schemas.len() as i64);
    }

    // db_schema_tables: all-null LargeList per schema. Empty-but-typed child
    // struct (zero rows, retaining `table_fields()`); see build_all_null_schema_list.
    let table_struct_child = new_empty_array(&DataType::Struct(table_fields()));
    let sch_offsets = vec![0i64; total_schemas + 1];
    let tables_null = NullBuffer::new(arrow::buffer::BooleanBuffer::new(
        arrow::buffer::Buffer::from(vec![0u8; total_schemas.div_ceil(8)]),
        0,
        total_schemas,
    ));
    let tables_list = LargeListArray::new(
        Arc::new(Field::new("item", DataType::Struct(table_fields()), true)),
        OffsetBuffer::new(ScalarBuffer::from(sch_offsets)),
        table_struct_child,
        Some(tables_null),
    );

    let name_array: ArrayRef = Arc::new(StringArray::from(schema_names));
    let tables_array: ArrayRef = Arc::new(tables_list);
    let schemas_struct = StructArray::new(schema_fields(), vec![name_array, tables_array], None);

    let list = LargeListArray::new(
        Arc::new(Field::new("item", DataType::Struct(schema_fields()), true)),
        OffsetBuffer::new(ScalarBuffer::from(cat_offsets)),
        Arc::new(schemas_struct),
        None,
    );

    Ok(Arc::new(list))
}

/// Builds the full nested `LargeList<Struct<schema_fields>>` for TABLES depth.
fn build_full_schema_list_array(
    schema_maps: &[BTreeMap<String, Vec<(String, String)>>],
) -> Result<ArrayRef, ApiError> {
    // Flatten all schemas across catalogs
    let total_schemas: usize = schema_maps.iter().map(|m| m.len()).sum();
    let total_tables: usize = schema_maps
        .iter()
        .flat_map(|m| m.values())
        .map(|t| t.len())
        .sum();

    let mut cat_offsets: Vec<i64> = Vec::with_capacity(schema_maps.len() + 1);
    cat_offsets.push(0);
    let mut sch_offsets: Vec<i64> = Vec::with_capacity(total_schemas + 1);
    sch_offsets.push(0);

    let mut all_schema_names: Vec<&str> = Vec::with_capacity(total_schemas);
    let mut all_table_names: Vec<&str> = Vec::with_capacity(total_tables);
    let mut all_table_types: Vec<&str> = Vec::with_capacity(total_tables);

    // We need to borrow from the input, so collect intermediate vecs first
    let schema_name_strs: Vec<Vec<&str>> = schema_maps
        .iter()
        .map(|m| m.keys().map(|s| s.as_str()).collect())
        .collect();
    let table_vecs: Vec<Vec<&[(String, String)]>> = schema_maps
        .iter()
        .map(|m| m.values().map(|t| t.as_slice()).collect())
        .collect();

    for (i, schemas) in schema_name_strs.iter().enumerate() {
        for (j, &sch_name) in schemas.iter().enumerate() {
            all_schema_names.push(sch_name);
            let tables = table_vecs[i][j];
            for (tbl_name, tbl_type) in tables {
                all_table_names.push(tbl_name.as_str());
                all_table_types.push(tbl_type.as_str());
            }
            sch_offsets.push(sch_offsets.last().copied().unwrap_or(0) + tables.len() as i64);
        }
        cat_offsets.push(cat_offsets.last().copied().unwrap_or(0) + schemas.len() as i64);
    }

    // Build empty columns/constraints lists for each table
    let empty_str_child = Arc::new(StringArray::from(Vec::<&str>::new())) as ArrayRef;
    let cols_offsets = vec![0i64; total_tables + 1];
    let constraints_offsets = vec![0i64; total_tables + 1];

    let cols_list = LargeListArray::new(
        Arc::new(Field::new("item", DataType::Utf8, true)),
        OffsetBuffer::new(ScalarBuffer::from(cols_offsets)),
        empty_str_child.clone(),
        None,
    );
    let constraints_list = LargeListArray::new(
        Arc::new(Field::new("item", DataType::Utf8, true)),
        OffsetBuffer::new(ScalarBuffer::from(constraints_offsets)),
        empty_str_child,
        None,
    );

    let tables_struct = StructArray::new(
        table_fields(),
        vec![
            Arc::new(StringArray::from(all_table_names)) as ArrayRef,
            Arc::new(StringArray::from(all_table_types)) as ArrayRef,
            Arc::new(cols_list) as ArrayRef,
            Arc::new(constraints_list) as ArrayRef,
        ],
        None,
    );

    let schemas_struct = StructArray::new(
        schema_fields(),
        vec![
            Arc::new(StringArray::from(all_schema_names)) as ArrayRef,
            Arc::new(LargeListArray::new(
                Arc::new(Field::new("item", DataType::Struct(table_fields()), true)),
                OffsetBuffer::new(ScalarBuffer::from(sch_offsets)),
                Arc::new(tables_struct),
                None,
            )) as ArrayRef,
        ],
        None,
    );

    let cat_list = LargeListArray::new(
        Arc::new(Field::new("item", DataType::Struct(schema_fields()), true)),
        OffsetBuffer::new(ScalarBuffer::from(cat_offsets)),
        Arc::new(schemas_struct),
        None,
    );

    Ok(Arc::new(cat_list))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::ipc::reader::StreamReader;

    // --- Schema contract ---

    #[test]
    fn nested_schema_has_expected_fields() {
        let schema = nested_get_objects_schema();
        assert_eq!(schema.fields().len(), 2);
        assert_eq!(schema.field(0).name(), FIELD_CATALOG_NAME);
        assert_eq!(schema.field(1).name(), FIELD_CATALOG_DB_SCHEMAS);

        // catalog_db_schemas is a LargeList
        let schemas_field = schema.field(1);
        assert!(matches!(schemas_field.data_type(), DataType::LargeList(_)));

        // Inner struct must have db_schema_name and db_schema_tables
        if let DataType::LargeList(item_field) = schemas_field.data_type() {
            if let DataType::Struct(schema_struct_fields) = item_field.data_type() {
                let names: Vec<&str> = schema_struct_fields
                    .iter()
                    .map(|f| f.name().as_str())
                    .collect();
                assert!(names.contains(&FIELD_DB_SCHEMA_NAME));
                assert!(names.contains(&FIELD_DB_SCHEMA_TABLES));
            } else {
                panic!("Expected Struct inside LargeList");
            }
        }
    }

    #[test]
    fn nested_schema_table_struct_has_expected_fields() {
        let schema = nested_get_objects_schema();
        let schemas_list = schema.field(1);
        if let DataType::LargeList(item) = schemas_list.data_type()
            && let DataType::Struct(schema_struct_fields) = item.data_type()
        {
            let tables_field = schema_struct_fields
                .iter()
                .find(|f| f.name() == FIELD_DB_SCHEMA_TABLES)
                .expect("db_schema_tables field missing");
            if let DataType::LargeList(table_item) = tables_field.data_type()
                && let DataType::Struct(tbl_fields) = table_item.data_type()
            {
                let names: Vec<&str> = tbl_fields.iter().map(|f| f.name().as_str()).collect();
                assert!(names.contains(&FIELD_TABLE_NAME));
                assert!(names.contains(&FIELD_TABLE_TYPE));
                assert!(names.contains(&FIELD_TABLE_COLUMNS));
                assert!(names.contains(&FIELD_TABLE_CONSTRAINTS));
                return;
            }
        }
        panic!("Unexpected schema shape");
    }

    // --- kind → TABLE_TYPE normalization ---

    #[test]
    fn kind_normalization_table_family() {
        for kind in &[
            "TABLE",
            "table",
            "TRANSIENT TABLE",
            "TEMPORARY TABLE",
            "EXTERNAL TABLE",
            "ICEBERG TABLE",
            "EVENT TABLE",
            "HYBRID TABLE",
            "MATERIALIZED TABLE",
        ] {
            assert_eq!(normalize_kind(kind), "TABLE", "kind={kind}");
        }
    }

    #[test]
    fn kind_normalization_view_family() {
        for kind in &["VIEW", "view", "MATERIALIZED VIEW", "SECURE VIEW"] {
            assert_eq!(normalize_kind(kind), "VIEW", "kind={kind}");
        }
    }

    #[test]
    fn kind_normalization_unknown_defaults_to_table() {
        assert_eq!(normalize_kind("DYNAMIC TABLE"), "TABLE");
        assert_eq!(normalize_kind("SOMETHING_UNKNOWN"), "TABLE");
    }

    // --- table_type normalization ---

    #[test]
    fn table_type_empty_list_means_all() {
        assert_eq!(normalize_table_types(&[]), TableTypeFilter::All);
    }

    #[test]
    fn table_type_percent_means_all() {
        assert_eq!(
            normalize_table_types(&["%".to_string()]),
            TableTypeFilter::All
        );
    }

    #[test]
    fn table_type_explicit_table_and_view() {
        let filter = normalize_table_types(&["TABLE".to_string(), "VIEW".to_string()]);
        assert!(matches!(filter, TableTypeFilter::Explicit(ref v) if v.len() == 2));
    }

    #[test]
    fn table_type_case_insensitive_normalization() {
        let filter = normalize_table_types(&["table".to_string()]);
        assert_eq!(filter, TableTypeFilter::Explicit(vec!["TABLE".to_string()]));
    }

    #[test]
    fn table_type_unsupported_type_yields_unsupported() {
        let filter = normalize_table_types(&["BASE TABLE".to_string()]);
        assert_eq!(filter, TableTypeFilter::Unsupported);
        let filter = normalize_table_types(&["SYNONYM".to_string()]);
        assert_eq!(filter, TableTypeFilter::Unsupported);
    }

    #[test]
    fn table_type_comma_separated_unsupported_if_no_table_or_view() {
        let filter = normalize_table_types(&["SYSTEM TABLE".to_string(), "SYNONYM".to_string()]);
        assert_eq!(filter, TableTypeFilter::Unsupported);
    }

    #[test]
    fn show_not_found_sql_states_are_recognized() {
        assert!(is_show_not_found_sql_state(Some("02000")));
        assert!(is_show_not_found_sql_state(Some("42000")));
        assert!(is_show_not_found_sql_state(Some("42S02")));
        assert!(!is_show_not_found_sql_state(Some("42501")));
        assert!(!is_show_not_found_sql_state(None));
    }

    #[test]
    fn map_execute_show_error_propagates_non_not_found_errors() {
        use snafu::IntoError;
        let err = QuerySnafu.into_error(RestError::QueryFailed {
            message: "permission denied".to_string(),
            code: Some(3001),
            sql_state: Some("42501".to_string()),
            query_id: None,
            location: snafu::Location::new("test", 1, 1),
        });
        assert!(map_execute_show_error(err, "SHOW TABLES").is_err());
    }

    #[test]
    fn map_execute_show_error_swallows_not_found_sql_states() {
        use snafu::IntoError;
        let err = QuerySnafu.into_error(RestError::QueryFailed {
            message: "does not exist".to_string(),
            code: Some(2003),
            sql_state: Some("42S02".to_string()),
            query_id: None,
            location: snafu::Location::new("test", 1, 1),
        });
        assert!(
            map_execute_show_error(err, "SHOW TABLES")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn empty_table_name_pattern_matches_nothing() {
        assert!(!like_pattern::matches("", "BASICTABLE"));
    }

    // --- Synthetic result-set round-trip ---

    #[test]
    fn register_arrow_batch_round_trips_schema_and_rows() {
        use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

        let schema = nested_get_objects_schema();
        let empty_batch = RecordBatch::new_empty(schema.clone());

        // Serialize to Arrow IPC and base64 (the same path as register_arrow_batch_as_result_set)
        use arrow::ipc::writer::StreamWriter;
        let mut buf = Vec::new();
        {
            let mut writer = StreamWriter::try_new(&mut buf, &schema).unwrap();
            writer.write(&empty_batch).unwrap();
            writer.finish().unwrap();
        }

        // Read back
        let decoded = BASE64.decode(BASE64.encode(&buf)).unwrap();
        let mut reader = StreamReader::try_new(std::io::Cursor::new(&decoded), None).unwrap();
        let rt_schema = reader.schema();

        // Schema must match exactly
        assert_eq!(rt_schema.field(0).name(), FIELD_CATALOG_NAME);
        assert_eq!(rt_schema.field(1).name(), FIELD_CATALOG_DB_SCHEMAS);
        assert_eq!(rt_schema.fields().len(), schema.fields().len());

        // Empty batch: no records
        assert!(reader.next().is_none() || reader.next().is_none());
    }

    #[test]
    fn format_show_sql_like_precedes_scope() {
        assert_eq!(
            format_show_sql("SHOW OBJECTS", "LIKE 'T%'", "IN SCHEMA \"DB\".\"SCH\""),
            "SHOW OBJECTS LIKE 'T%' IN SCHEMA \"DB\".\"SCH\""
        );
        assert_eq!(
            format_show_sql("SHOW TABLES", "", "IN DATABASE \"DB\""),
            "SHOW TABLES IN DATABASE \"DB\""
        );
    }

    #[test]
    fn build_like_clause_none_yields_empty_string() {
        assert_eq!(build_like_clause(None), "");
    }

    #[test]
    fn build_like_clause_escape_free_pattern_is_pushed_to_server() {
        let clause = build_like_clause(Some("MY%TABLE"));
        assert!(clause.starts_with("LIKE '"));
        assert!(clause.contains("MY%TABLE"));
    }

    #[test]
    fn build_like_clause_escape_pattern_pushes_stripped_coarse_pattern() {
        let clause = build_like_clause(Some("MY\\_TABLE"));
        assert_eq!(clause, "LIKE 'MY_TABLE'");
    }

    #[test]
    fn build_like_clause_escaped_percent_pushes_stripped_pattern() {
        let clause = build_like_clause(Some("100\\%"));
        assert_eq!(clause, "LIKE '100%'");
    }

    #[test]
    fn escape_show_like_doubles_backslash_before_quoting() {
        // A literal backslash must be doubled so it cannot escape the wrapping
        // single quote. `\` first, then `'`.
        assert_eq!(escape_show_like("AB\\"), "AB\\\\");
        assert_eq!(escape_show_like("a\\b"), "a\\\\b");
        assert_eq!(escape_show_like("o'brien"), "o\\'brien");
        // Backslash adjacent to a quote stays well-formed: `\` -> `\\`, then `'` -> `\'`.
        assert_eq!(escape_show_like("a\\'b"), "a\\\\\\'b");
    }

    #[test]
    fn build_like_clause_trailing_backslash_yields_well_formed_sql() {
        // Regression: a pattern ending in a lone backslash (e.g. table_name `AB\`)
        // survives strip_escapes_for_show_like, and must not escape the closing
        // quote. Expected SQL is `LIKE 'AB\\'` (one logical trailing backslash).
        let clause = build_like_clause(Some("AB\\"));
        assert_eq!(clause, "LIKE 'AB\\\\'");
        // The quote count is balanced (open + close only), proving the literal
        // is terminated rather than running past the closing quote.
        assert_eq!(clause.matches('\'').count(), 2);
    }

    #[test]
    fn rows_from_reader_round_trip() {
        use crate::chunks::single_chunk_reader;
        use arrow::ipc::writer::StreamWriter;
        use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

        let schema = Arc::new(Schema::new(vec![
            Field::new("name", DataType::Utf8, true),
            Field::new("database_name", DataType::Utf8, true),
        ]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(vec!["MY_TABLE"])) as ArrayRef,
                Arc::new(StringArray::from(vec!["DB"])) as ArrayRef,
            ],
        )
        .unwrap();

        let mut buf = Vec::new();
        {
            let mut writer = StreamWriter::try_new(&mut buf, &schema).unwrap();
            writer.write(&batch).unwrap();
            writer.finish().unwrap();
        }
        let chunk_base64 = BASE64.encode(&buf);
        let reader = single_chunk_reader(&chunk_base64).unwrap();
        let rows = rows_from_reader(reader).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0].1, "MY_TABLE");
        assert_eq!(rows[0][1].1, "DB");
    }
}
