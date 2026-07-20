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
    Array, ArrayRef, BooleanArray, Int32Array, Int64Array, LargeListArray, LargeStringArray,
    RecordBatch, RecordBatchReader, StringArray, StructArray, TimestampMicrosecondArray,
    TimestampNanosecondArray, new_empty_array,
};
use arrow::buffer::{NullBuffer, OffsetBuffer, ScalarBuffer};
use arrow::datatypes::{DataType, Field, Fields, Schema, SchemaRef};
use arrow::error::ArrowError;
use snafu::{OptionExt, ResultExt};
use tokio::sync::Mutex;

use super::connection::{Connection, with_valid_session};
use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::like_pattern;
use crate::chunks::{ChunkError, PrefetchConfig};
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

// Column struct sub-fields (within table_columns list items)
pub const FIELD_COLUMN_NAME: &str = "column_name";
pub const FIELD_COLUMN_ORDINAL_POSITION: &str = "ordinal_position";
pub const FIELD_COLUMN_LOGICAL_TYPE: &str = "logical_type";
pub const FIELD_COLUMN_PRECISION: &str = "precision";
pub const FIELD_COLUMN_SCALE: &str = "scale";
pub const FIELD_COLUMN_CHAR_LENGTH: &str = "char_length";
pub const FIELD_COLUMN_BYTE_LENGTH: &str = "byte_length";
pub const FIELD_COLUMN_NULLABLE: &str = "nullable";
pub const FIELD_COLUMN_DEF: &str = "column_def";
pub const FIELD_COLUMN_REMARKS: &str = "remarks";

// ---------------------------------------------------------------------------
// Nested ADBC Arrow schema (single source of truth, cached)
// ---------------------------------------------------------------------------

fn column_fields() -> Fields {
    Fields::from(vec![
        Field::new(FIELD_COLUMN_NAME, DataType::Utf8, true),
        Field::new(FIELD_COLUMN_ORDINAL_POSITION, DataType::Int32, false),
        Field::new(FIELD_COLUMN_LOGICAL_TYPE, DataType::Utf8, true),
        Field::new(FIELD_COLUMN_PRECISION, DataType::Int32, true),
        Field::new(FIELD_COLUMN_SCALE, DataType::Int32, true),
        Field::new(FIELD_COLUMN_CHAR_LENGTH, DataType::Int64, true),
        Field::new(FIELD_COLUMN_BYTE_LENGTH, DataType::Int64, true),
        Field::new(FIELD_COLUMN_NULLABLE, DataType::Boolean, false),
        Field::new(FIELD_COLUMN_DEF, DataType::Utf8, true),
        Field::new(FIELD_COLUMN_REMARKS, DataType::Utf8, true),
    ])
}

fn table_fields() -> Fields {
    Fields::from(vec![
        Field::new(FIELD_TABLE_NAME, DataType::Utf8, true),
        Field::new(FIELD_TABLE_TYPE, DataType::Utf8, true),
        Field::new(
            FIELD_TABLE_COLUMNS,
            DataType::LargeList(Arc::new(Field::new(
                "item",
                DataType::Struct(column_fields()),
                true,
            ))),
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
    pub column_name: Option<String>,
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
        cancel: tokio_util::sync::CancellationToken,
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
            DEPTH_CATALOGS => {
                fetch_catalogs(&conn_ptr, catalog_filter.as_deref(), cancel.clone()).await?
            }
            DEPTH_DB_SCHEMAS => {
                fetch_schemas(
                    &conn_ptr,
                    catalog_filter.as_deref(),
                    schema_filter.as_deref(),
                    cancel.clone(),
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
                    cancel.clone(),
                )
                .await?
            }
            DEPTH_COLUMNS => {
                fetch_columns(
                    &conn_ptr,
                    catalog_filter.as_deref(),
                    schema_filter.as_deref(),
                    req.table_name.as_deref(),
                    req.column_name.as_deref(),
                    cancel,
                )
                .await?
            }
            other => {
                return InvalidArgumentSnafu {
                    argument: format!("GetObjects depth {other} is invalid (expected 1..=4)"),
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
    cancel: tokio_util::sync::CancellationToken,
) -> Result<RecordBatch, ApiError> {
    let sql = "SHOW DATABASES IN ACCOUNT".to_string();
    let rows = execute_show(conn_ptr, &sql, cancel).await?;

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
    cancel: tokio_util::sync::CancellationToken,
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

    let rows = execute_show(conn_ptr, &sql, cancel).await?;

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
    cancel: tokio_util::sync::CancellationToken,
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
        cancel,
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

/// Build a `LIKE '…'` clause for `SHOW` commands.
///
/// Returns an empty string for `None` or `Some("")` so callers can pass the
/// pattern directly without pre-filtering. Snowflake SHOW LIKE does not honour
/// `\` escapes, so they are stripped for coarse server-side narrowing;
/// client-side `like_pattern::matches` re-applies the original pattern.
fn build_like_clause(pattern: Option<&str>) -> String {
    match pattern {
        None | Some("") => String::new(),
        Some(p) => {
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
    log_query_text: bool,
) -> Result<Vec<Vec<(String, String)>>, ApiError> {
    if is_show_not_found_sql_state(api_error_sql_state(&err)) {
        if log_query_text {
            tracing::info!("SHOW query not found (returning empty): {sql}: {err}");
        } else {
            tracing::debug!("SHOW query not found (returning empty): {err}");
        }
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
    cancel: tokio_util::sync::CancellationToken,
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
        let cancel = cancel.clone();
        async move {
            snowflake_query_with_client(
                &http_client,
                query_parameters,
                token.reveal(),
                query_input,
                &retry_policy,
                QueryExecutionMode::Blocking,
                cancel,
            )
            .await
        }
    })
    .await;

    let response = match response {
        Ok(resp) => resp,
        Err(err) => {
            return map_execute_show_error(err, &sql_owned, query_parameters.log_query_text);
        }
    };

    // Reuse the canonical reader, which downloads and concatenates external
    // result chunks for every rowset shape (JSON/Arrow, single/multi-chunk).
    // Account-wide `SHOW OBJECTS` spills to external chunks; parsing only the
    // inline rowset here would silently drop most rows.
    let rowset_data = response.data.into_rowset_data();
    let reader =
        super::query::read_batches(&rowset_data, http_client, &prefetch_config, None, cancel.clone())
            .await
            .map_err(|e| {
                if e.is_cancelled() {
                    return CancelledSnafu.build();
                }
                InvalidArgumentSnafu {
                    argument: format!("SHOW result read failed: {e}"),
                }
                .build()
            })?;
    // The reader drains chunks via `blocking_recv`, which panics if polled on a
    // runtime worker; drain it on a blocking thread while downloads progress on
    // the async workers.
    let parsed = tokio::task::spawn_blocking(move || rows_from_reader(reader, cancel))
        .await
        .map_err(|e| {
            InvalidArgumentSnafu {
                argument: format!("SHOW reader join failed: {e}"),
            }
            .build()
        })??;
    if query_parameters.log_query_text {
        tracing::info!("SHOW query parsed {} rows: {sql_owned}", parsed.len());
    } else {
        tracing::debug!("SHOW query parsed {} rows", parsed.len());
    }
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
    cancel: tokio_util::sync::CancellationToken,
) -> Result<Vec<Vec<(String, String)>>, ApiError> {
    let mut rows = Vec::new();
    for batch_result in reader {
        // Check on the success path too: draining already-buffered batches
        // yields no error, so an error-only check would let cancellation slip
        // through and the SHOW parse complete instead of surfacing CANCELLED.
        if cancel.is_cancelled() {
            return CancelledSnafu.fail();
        }
        let batch = match batch_result {
            Ok(batch) => batch,
            // A cancelled chunk download reaches the reader as a `ChunkError::Cancelled`
            // boxed inside `ArrowError::ExternalError`; unwrap it so cancellation surfaces
            // as CANCELLED instead of being flattened into `ApiError::ArrowParse`.
            Err(e) if arrow_error_is_cancelled(&e) => return CancelledSnafu.fail(),
            Err(e) => return Err(e).context(ArrowParseSnafu),
        };
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

fn arrow_error_is_cancelled(err: &ArrowError) -> bool {
    matches!(
        err,
        ArrowError::ExternalError(source)
            if source
                .downcast_ref::<ChunkError>()
                .is_some_and(ChunkError::is_cancelled)
    )
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

    // Build an empty columns list for each table.
    // table_columns uses the canonical column Struct shape so the schema
    // matches what DEPTH_COLUMNS returns.
    let empty_col_child = new_empty_array(&DataType::Struct(column_fields()));
    let cols_offsets = vec![0i64; total_tables + 1];
    let cols_list = LargeListArray::new(
        Arc::new(Field::new("item", DataType::Struct(column_fields()), true)),
        OffsetBuffer::new(ScalarBuffer::from(cols_offsets)),
        empty_col_child,
        None,
    );

    Ok(assemble_schema_list_array(
        all_schema_names,
        all_table_names,
        all_table_types,
        cat_offsets,
        sch_offsets,
        cols_list,
        total_tables,
    ))
}

/// Assemble the catalog→schema→table nested `LargeList<Struct<…>>` from prebuilt
/// per-level accumulators. Shared by [`build_full_schema_list_array`] (empty
/// `table_columns`) and [`build_full_columns_schema_list_array`] (populated
/// `table_columns`); `table_constraints` is always an empty Utf8 list.
///
/// Centralizing the struct assembly here means a new field on `table_fields()`
/// or `schema_fields()` is a single-site change. The per-depth accumulation
/// loops stay in their own builders because their inputs are different shapes
/// (`Vec<(name, type)>` vs. `BTreeMap<table, Vec<ColumnDescriptor>>`).
fn assemble_schema_list_array<'a>(
    all_schema_names: Vec<&'a str>,
    all_table_names: Vec<&'a str>,
    all_table_types: Vec<&'a str>,
    cat_offsets: Vec<i64>,
    sch_offsets: Vec<i64>,
    table_columns_list: LargeListArray,
    total_tables: usize,
) -> ArrayRef {
    // table_constraints: empty Utf8 list per table.
    let empty_str_child = Arc::new(StringArray::from(Vec::<&str>::new())) as ArrayRef;
    let constraints_offsets = vec![0i64; total_tables + 1];
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
            Arc::new(table_columns_list) as ArrayRef,
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

    Arc::new(cat_list)
}

// ---------------------------------------------------------------------------
// COLUMNS depth
// ---------------------------------------------------------------------------

/// Parsed representation of the `data_type` JSON blob from SHOW COLUMNS.
/// Only the fields needed to reconstruct the canonical Arrow type-metadata are
/// captured; unknown fields are ignored (serde default).
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

/// Decoded canonical column descriptor — the sf_core representation of a
/// single SHOW COLUMNS row, with Snowflake-wire JSON fully parsed.
/// No raw JSON crosses into the `odbc` crate.
#[derive(Debug)]
pub struct ColumnDescriptor {
    pub column_name: String,
    pub ordinal_position: i32,
    pub logical_type: String,
    pub precision: Option<i32>,
    pub scale: Option<i32>,
    pub char_length: Option<i64>,
    pub byte_length: Option<i64>,
    pub nullable: bool,
    pub column_def: Option<String>,
    pub remarks: Option<String>,
}

/// Decode the `data_type` JSON blob from SHOW COLUMNS into a `ColumnDescriptor`.
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
///    match the fields captured here; `SnowflakeFieldType::from_field` will
///    simply return an error for "UNKNOWN" and the column will map to NULL in
///    the ODBC output rather than poisoning the whole result set.
///
/// 3. **Precision/scale narrowing** — the JSON blob carries i64 values; we
///    narrow to i32 for the canonical struct. Snowflake's precision is at most
///    38 (FIXED) and scale at most 37, both well within i32::MAX, so this is
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

async fn fetch_columns(
    conn_ptr: &Arc<Mutex<Connection>>,
    catalog_filter: Option<&str>,
    schema_filter: Option<&str>,
    table_name_filter: Option<&str>,
    column_name_filter: Option<&str>,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<RecordBatch, ApiError> {
    // Empty string means "match nothing".
    if matches!(table_name_filter, Some("")) || matches!(column_name_filter, Some("")) {
        return build_columns_batch(BTreeMap::new());
    }

    let exact_catalog = catalog_filter
        .and_then(like_pattern::is_exact)
        .filter(|s| !s.is_empty());
    let exact_schema = schema_filter
        .and_then(like_pattern::is_exact)
        .filter(|s| !s.is_empty());
    let exact_table = table_name_filter
        .and_then(like_pattern::is_exact)
        .filter(|s| !s.is_empty());

    // Coarse column-name pushdown; build_like_clause handles None/"" → no LIKE.
    let col_like_clause = build_like_clause(column_name_filter);

    // Pick the tightest scope available.
    let scope = match (&exact_catalog, &exact_schema, &exact_table) {
        (Some(cat), Some(sch), Some(tbl)) => format!(
            "IN TABLE \"{}\".\"{}\".\"{}\"",
            escape_dq(cat),
            escape_dq(sch),
            escape_dq(tbl)
        ),
        (Some(cat), Some(sch), None) => {
            format!("IN SCHEMA \"{}\".\"{}\"", escape_dq(cat), escape_dq(sch))
        }
        (Some(cat), None, _) => format!("IN DATABASE \"{}\"", escape_dq(cat)),
        _ => "IN ACCOUNT".to_string(),
    };

    let sql = format_show_sql("SHOW COLUMNS", &col_like_clause, &scope);
    let rows = execute_show(conn_ptr, &sql, cancel).await?;

    // Group columns by (catalog, schema, table), preserving SHOW row order for
    // ordinal_position assignment. BTreeMap gives deterministic lexicographic sort.
    let mut by_cat_sch_tbl: BTreeMap<
        String,
        BTreeMap<String, BTreeMap<String, Vec<ColumnDescriptor>>>,
    > = BTreeMap::new();

    for row in &rows {
        let db_name = match get_column(row, "database_name") {
            Some(n) => n.to_string(),
            None => continue,
        };
        let sch_name = match get_column(row, "schema_name") {
            Some(n) => n.to_string(),
            None => continue,
        };
        let tbl_name = match get_column(row, "table_name") {
            Some(n) => n.to_string(),
            None => continue,
        };
        let col_name = match get_column(row, "column_name") {
            Some(n) => n.to_string(),
            None => continue,
        };
        let data_type_json = get_column(row, "data_type").unwrap_or("{}");
        let column_def = get_column(row, "default")
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());
        let remarks = get_column(row, "comment")
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());

        // Client-side filters.
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
        if let Some(pattern) = column_name_filter
            && !like_pattern::matches(pattern, &col_name)
        {
            continue;
        }

        // Ordinal position = 1-based index within this (db, schema, table) group.
        let table_cols = by_cat_sch_tbl
            .entry(db_name)
            .or_default()
            .entry(sch_name)
            .or_default()
            .entry(tbl_name)
            .or_default();
        let ordinal_position = (table_cols.len() + 1) as i32;

        let descriptor = decode_data_type_json(
            data_type_json,
            col_name,
            ordinal_position,
            column_def,
            remarks,
        );
        table_cols.push(descriptor);
    }

    build_columns_batch(by_cat_sch_tbl)
}

fn build_columns_batch(
    by_cat_sch_tbl: BTreeMap<String, BTreeMap<String, BTreeMap<String, Vec<ColumnDescriptor>>>>,
) -> Result<RecordBatch, ApiError> {
    let schema = nested_get_objects_schema();

    let mut catalog_names: Vec<String> = Vec::new();
    let mut schema_maps: Vec<BTreeMap<String, BTreeMap<String, Vec<ColumnDescriptor>>>> =
        Vec::new();

    for (cat, schemas) in by_cat_sch_tbl {
        catalog_names.push(cat);
        schema_maps.push(schemas);
    }

    let mut cat_builder = arrow::array::StringBuilder::new();
    for name in &catalog_names {
        cat_builder.append_value(name);
    }
    let cat_array: ArrayRef = Arc::new(cat_builder.finish());

    let db_schemas_array = build_full_columns_schema_list_array(&schema_maps)?;

    let batch =
        RecordBatch::try_new(schema, vec![cat_array, db_schemas_array]).context(ArrowParseSnafu)?;
    Ok(batch)
}

/// Builds the full nested `LargeList<Struct<schema_fields>>` for COLUMNS depth,
/// where `table_columns` is populated with real `ColumnDescriptor` data.
fn build_full_columns_schema_list_array(
    schema_maps: &[BTreeMap<String, BTreeMap<String, Vec<ColumnDescriptor>>>],
) -> Result<ArrayRef, ApiError> {
    let total_schemas: usize = schema_maps.iter().map(|m| m.len()).sum();
    let total_tables: usize = schema_maps
        .iter()
        .flat_map(|m| m.values())
        .map(|t| t.len())
        .sum();
    let total_columns: usize = schema_maps
        .iter()
        .flat_map(|m| m.values())
        .flat_map(|t| t.values())
        .map(|c| c.len())
        .sum();

    let mut cat_offsets: Vec<i64> = Vec::with_capacity(schema_maps.len() + 1);
    cat_offsets.push(0);
    let mut sch_offsets: Vec<i64> = Vec::with_capacity(total_schemas + 1);
    sch_offsets.push(0);
    let mut tbl_offsets: Vec<i64> = Vec::with_capacity(total_tables + 1);
    tbl_offsets.push(0);

    let mut all_schema_names: Vec<&str> = Vec::with_capacity(total_schemas);
    let mut all_table_names: Vec<&str> = Vec::with_capacity(total_tables);
    let mut all_table_types: Vec<&str> = Vec::with_capacity(total_tables);

    // Column-level accumulators (one entry per column across all tables).
    let mut col_names: Vec<Option<&str>> = Vec::with_capacity(total_columns);
    let mut col_ordinals: Vec<i32> = Vec::with_capacity(total_columns);
    let mut col_logical_types: Vec<Option<&str>> = Vec::with_capacity(total_columns);
    let mut col_precisions: Vec<Option<i32>> = Vec::with_capacity(total_columns);
    let mut col_scales: Vec<Option<i32>> = Vec::with_capacity(total_columns);
    let mut col_char_lengths: Vec<Option<i64>> = Vec::with_capacity(total_columns);
    let mut col_byte_lengths: Vec<Option<i64>> = Vec::with_capacity(total_columns);
    let mut col_nullables: Vec<bool> = Vec::with_capacity(total_columns);
    let mut col_defs: Vec<Option<&str>> = Vec::with_capacity(total_columns);
    let mut col_remarks_vec: Vec<Option<&str>> = Vec::with_capacity(total_columns);

    // Iterate deterministically (BTreeMap is sorted).
    for schema_map in schema_maps {
        for (sch_name, table_map) in schema_map {
            all_schema_names.push(sch_name.as_str());
            for (tbl_name, columns) in table_map {
                all_table_names.push(tbl_name.as_str());
                all_table_types.push(""); // TABLE_TYPE not populated for DEPTH_COLUMNS
                for col in columns {
                    col_names.push(Some(col.column_name.as_str()));
                    col_ordinals.push(col.ordinal_position);
                    col_logical_types.push(Some(col.logical_type.as_str()));
                    col_precisions.push(col.precision);
                    col_scales.push(col.scale);
                    col_char_lengths.push(col.char_length);
                    col_byte_lengths.push(col.byte_length);
                    col_nullables.push(col.nullable);
                    col_defs.push(col.column_def.as_deref());
                    col_remarks_vec.push(col.remarks.as_deref());
                }
                tbl_offsets.push(tbl_offsets.last().copied().unwrap_or(0) + columns.len() as i64);
            }
            sch_offsets.push(sch_offsets.last().copied().unwrap_or(0) + table_map.len() as i64);
        }
        cat_offsets.push(cat_offsets.last().copied().unwrap_or(0) + schema_map.len() as i64);
    }

    // Build the column struct array.
    let cols_struct = StructArray::new(
        column_fields(),
        vec![
            Arc::new(StringArray::from(col_names)) as ArrayRef,
            Arc::new(Int32Array::from(col_ordinals)) as ArrayRef,
            Arc::new(StringArray::from(col_logical_types)) as ArrayRef,
            Arc::new(Int32Array::from(col_precisions)) as ArrayRef,
            Arc::new(Int32Array::from(col_scales)) as ArrayRef,
            Arc::new(Int64Array::from(col_char_lengths)) as ArrayRef,
            Arc::new(Int64Array::from(col_byte_lengths)) as ArrayRef,
            Arc::new(BooleanArray::from(col_nullables)) as ArrayRef,
            Arc::new(StringArray::from(col_defs)) as ArrayRef,
            Arc::new(StringArray::from(col_remarks_vec)) as ArrayRef,
        ],
        None,
    );

    // table_columns: LargeList over the populated column struct.
    let cols_list = LargeListArray::new(
        Arc::new(Field::new("item", DataType::Struct(column_fields()), true)),
        OffsetBuffer::new(ScalarBuffer::from(tbl_offsets)),
        Arc::new(cols_struct),
        None,
    );

    Ok(assemble_schema_list_array(
        all_schema_names,
        all_table_names,
        all_table_types,
        cat_offsets,
        sch_offsets,
        cols_list,
        total_tables,
    ))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::ipc::reader::StreamReader;

    struct OneErrorReader {
        schema: SchemaRef,
        err: Option<ArrowError>,
    }

    impl Iterator for OneErrorReader {
        type Item = Result<RecordBatch, ArrowError>;
        fn next(&mut self) -> Option<Self::Item> {
            self.err.take().map(Err)
        }
    }

    impl RecordBatchReader for OneErrorReader {
        fn schema(&self) -> SchemaRef {
            self.schema.clone()
        }
    }

    fn one_error_reader(err: ArrowError) -> Box<dyn RecordBatchReader + Send> {
        let schema = Arc::new(Schema::new(vec![Field::new("name", DataType::Utf8, true)]));
        Box::new(OneErrorReader {
            schema,
            err: Some(err),
        })
    }

    struct OkBatchReader {
        schema: SchemaRef,
        batch: Option<RecordBatch>,
    }

    impl Iterator for OkBatchReader {
        type Item = Result<RecordBatch, ArrowError>;
        fn next(&mut self) -> Option<Self::Item> {
            self.batch.take().map(Ok)
        }
    }

    impl RecordBatchReader for OkBatchReader {
        fn schema(&self) -> SchemaRef {
            self.schema.clone()
        }
    }

    #[test]
    fn rows_from_reader_maps_cancelled_chunk_to_cancelled() {
        let cancelled = ChunkError::Cancelled {
            location: snafu::Location::default(),
        };
        let reader = one_error_reader(ArrowError::ExternalError(Box::new(cancelled)));

        let err = rows_from_reader(reader, tokio_util::sync::CancellationToken::new())
            .expect_err("cancelled drain must surface an error");

        assert!(
            matches!(err, ApiError::Cancelled { .. }),
            "expected ApiError::Cancelled, got {err:?}"
        );
        assert!(err.is_cancelled());
    }

    #[test]
    fn rows_from_reader_maps_other_arrow_error_to_arrow_parse() {
        let reader = one_error_reader(ArrowError::ComputeError("boom".to_string()));

        let err = rows_from_reader(reader, tokio_util::sync::CancellationToken::new())
            .expect_err("reader error must surface");

        assert!(
            matches!(err, ApiError::ArrowParse { .. }),
            "expected ApiError::ArrowParse, got {err:?}"
        );
        assert!(!err.is_cancelled());
    }

    #[test]
    fn rows_from_reader_cancelled_token_during_successful_drain() {
        let schema = Arc::new(Schema::new(vec![Field::new("name", DataType::Utf8, true)]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(StringArray::from(vec!["a", "b"])) as ArrayRef],
        )
        .unwrap();
        let reader = Box::new(OkBatchReader {
            schema,
            batch: Some(batch),
        });

        let cancel = tokio_util::sync::CancellationToken::new();
        cancel.cancel();
        let err = rows_from_reader(reader, cancel)
            .expect_err("cancelled token must surface even on a successful drain");

        assert!(
            matches!(err, ApiError::Cancelled { .. }),
            "expected ApiError::Cancelled, got {err:?}"
        );
    }

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
        assert!(map_execute_show_error(err, "SHOW TABLES", false).is_err());
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
            map_execute_show_error(err, "SHOW TABLES", false)
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
        let reader = single_chunk_reader(&chunk_base64, None).unwrap();
        let rows = rows_from_reader(reader, tokio_util::sync::CancellationToken::new()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0].1, "MY_TABLE");
        assert_eq!(rows[0][1].1, "DB");
    }

    // --- decode_data_type_json ---

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
    fn decoder_boolean_type() {
        let json = r#"{"type":"BOOLEAN","nullable":true}"#;
        let d = decode_data_type_json(json, "FLAG".to_string(), 1, None, None);
        assert_eq!(d.logical_type, "BOOLEAN");
        assert_eq!(d.precision, None);
        assert_eq!(d.scale, None);
    }

    #[test]
    fn decoder_real_type() {
        let json = r#"{"type":"REAL","nullable":false}"#;
        let d = decode_data_type_json(json, "PRICE".to_string(), 1, None, None);
        assert_eq!(d.logical_type, "REAL");
        assert!(!d.nullable);
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

    // --- columns schema shape ---

    #[test]
    fn table_columns_field_is_struct_list() {
        let fields = table_fields();
        let cols_field = fields
            .iter()
            .find(|f| f.name() == FIELD_TABLE_COLUMNS)
            .expect("table_columns missing");
        if let DataType::LargeList(item) = cols_field.data_type() {
            assert!(
                matches!(item.data_type(), DataType::Struct(_)),
                "table_columns item should be Struct, got {:?}",
                item.data_type()
            );
        } else {
            panic!(
                "table_columns should be LargeList, got {:?}",
                cols_field.data_type()
            );
        }
    }

    #[test]
    fn column_struct_has_expected_fields() {
        let fields = column_fields();
        let names: Vec<&str> = fields.iter().map(|f| f.name().as_str()).collect();
        assert!(names.contains(&FIELD_COLUMN_NAME));
        assert!(names.contains(&FIELD_COLUMN_ORDINAL_POSITION));
        assert!(names.contains(&FIELD_COLUMN_LOGICAL_TYPE));
        assert!(names.contains(&FIELD_COLUMN_PRECISION));
        assert!(names.contains(&FIELD_COLUMN_SCALE));
        assert!(names.contains(&FIELD_COLUMN_CHAR_LENGTH));
        assert!(names.contains(&FIELD_COLUMN_BYTE_LENGTH));
        assert!(names.contains(&FIELD_COLUMN_NULLABLE));
        assert!(names.contains(&FIELD_COLUMN_DEF));
        assert!(names.contains(&FIELD_COLUMN_REMARKS));
    }

    #[test]
    fn build_columns_batch_empty_produces_valid_batch() {
        let batch = build_columns_batch(BTreeMap::new()).unwrap();
        assert_eq!(batch.num_rows(), 0);
        assert_eq!(batch.schema().field(0).name(), FIELD_CATALOG_NAME);
    }

    #[test]
    fn build_columns_batch_single_column() {
        let json = r#"{"type":"FIXED","nullable":false,"precision":38,"scale":0}"#;
        let col = decode_data_type_json(json, "ID".to_string(), 1, None, None);
        let mut tbl_map: BTreeMap<String, Vec<ColumnDescriptor>> = BTreeMap::new();
        tbl_map.insert("MYTABLE".to_string(), vec![col]);
        let mut sch_map: BTreeMap<String, BTreeMap<String, Vec<ColumnDescriptor>>> =
            BTreeMap::new();
        sch_map.insert("PUBLIC".to_string(), tbl_map);
        let mut by_cat: BTreeMap<
            String,
            BTreeMap<String, BTreeMap<String, Vec<ColumnDescriptor>>>,
        > = BTreeMap::new();
        by_cat.insert("MYDB".to_string(), sch_map);

        let batch = build_columns_batch(by_cat).unwrap();
        // 1 catalog row
        assert_eq!(batch.num_rows(), 1);

        // Drill down to verify the column struct is present.
        let schemas_list = batch
            .column(1)
            .as_any()
            .downcast_ref::<LargeListArray>()
            .expect("catalog_db_schemas should be LargeListArray");
        let schemas_struct = schemas_list
            .values()
            .as_any()
            .downcast_ref::<StructArray>()
            .expect("schemas values should be StructArray");
        let tables_list = schemas_struct
            .column_by_name(FIELD_DB_SCHEMA_TABLES)
            .unwrap()
            .as_any()
            .downcast_ref::<LargeListArray>()
            .expect("db_schema_tables should be LargeListArray");
        let tables_struct = tables_list
            .values()
            .as_any()
            .downcast_ref::<StructArray>()
            .expect("table values should be StructArray");
        let cols_list = tables_struct
            .column_by_name(FIELD_TABLE_COLUMNS)
            .unwrap()
            .as_any()
            .downcast_ref::<LargeListArray>()
            .expect("table_columns should be LargeListArray");
        let cols_struct = cols_list
            .values()
            .as_any()
            .downcast_ref::<StructArray>()
            .expect("column values should be StructArray");

        assert_eq!(cols_struct.len(), 1, "expected 1 column");

        let col_name_arr = cols_struct
            .column_by_name(FIELD_COLUMN_NAME)
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(col_name_arr.value(0), "ID");

        let ordinal_arr = cols_struct
            .column_by_name(FIELD_COLUMN_ORDINAL_POSITION)
            .unwrap()
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        assert_eq!(ordinal_arr.value(0), 1);

        let logical_type_arr = cols_struct
            .column_by_name(FIELD_COLUMN_LOGICAL_TYPE)
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(logical_type_arr.value(0), "FIXED");
    }
}
