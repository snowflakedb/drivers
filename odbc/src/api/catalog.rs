//! Catalog functions: SQLTables and related.
//!
//! The wrapper reads ODBC string arguments, maps them to patterns (via
//! `catalog_arg_to_pattern`), dispatches to the core `ConnectionGetObjects`
//! RPC, then flattens the nested ADBC-shaped Arrow result into the flat
//! 5-column ODBC result set.

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
use arrow::array::{Array, ArrayRef, LargeListArray, RecordBatch, StringArray};
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
