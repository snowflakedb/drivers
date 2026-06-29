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
    DisconnectedSnafu, InvalidDuringDaeSnafu, OdbcRuntimeSnafu,
};
use crate::api::runtime::global;
use crate::api::statement::set_state_for_catalog;
use crate::api::utils::{catalog_arg_to_pattern, escape_like_wildcards};
use crate::api::{
    ConnectionState, ExecutionOrigin, OdbcResult, StatementInner, StatementState, stmt_from_handle,
};
use arrow::array::{
    Array, ArrayRef, Int16Array, Int32Array, LargeListArray, RecordBatch, StringArray,
};
use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use odbc_sys as sql;
use sf_core::apis::database_driver_v1::{
    DEPTH_CATALOGS, DEPTH_DB_SCHEMAS, DEPTH_TABLES, FIELD_CATALOG_DB_SCHEMAS, FIELD_CATALOG_NAME,
    FIELD_DB_SCHEMA_NAME, FIELD_DB_SCHEMA_TABLES, FIELD_TABLE_NAME, FIELD_TABLE_TYPE,
};
use sf_core::protobuf::generated::database_driver_v1::{
    ConnectionGetInfoRequest, ConnectionGetObjectsRequest, ResultSetGetStreamRequest,
    ResultSetHandle, ResultSetReleaseRequest,
};
use snafu::ResultExt;
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

fn collect_nested_batch(
    mut reader: Box<dyn arrow::array::RecordBatchReader + Send>,
) -> OdbcResult<RecordBatch> {
    let schema = reader.schema();
    let mut batches = vec![];
    for b in &mut *reader {
        let batch = b.context(crate::api::error::ArrowBatchReadSnafu)?;
        batches.push(batch);
    }
    if batches.is_empty() {
        Ok(RecordBatch::new_empty(schema))
    } else if batches.len() == 1 {
        Ok(batches.remove(0))
    } else {
        use arrow::compute::concat_batches;
        concat_batches(&schema, &batches).context(crate::api::error::ArrowBatchConcatSnafu)
    }
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

    if inner.state.as_ref().is_need_data() {
        return InvalidDuringDaeSnafu.fail();
    }
    if inner.state.as_ref().is_async_executing() {
        return AsyncInProgressSnafu.fail();
    }
    if inner.state.as_ref().has_open_cursor() {
        return CursorAlreadyOpenSnafu.fail();
    }

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
