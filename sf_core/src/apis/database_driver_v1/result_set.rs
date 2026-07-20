use std::sync::Arc;

use super::connection::{Connection, RefreshContext};
use super::error::*;
use super::global_state::{DatabaseDriverV1, WrapperPresets};
use super::query::build_reader_from_rowset_data;
use crate::chunks::{ChunkDownloadData, ChunkFormatKind, PrefetchConfig};
use crate::handle_manager::Handle;
use crate::query_types::statement_type::{
    DML_AFFECTED_ROWS_COLUMN_PREFIXES, DML_AFFECTED_ROWS_COLUMNS, QueryType, ResultKind,
};
use crate::rest::snowflake::query_response::{Data, RowType, RowsetData, Stats};
use crate::rest::snowflake::snowflake_get_query_result;
use arrow::array::RecordBatchReader;
use snafu::{OptionExt, ResultExt};
use tokio::sync::Mutex;

// --- Public types ---

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ColumnMetadata {
    pub name: String,
    pub r#type: String,
    pub precision: Option<i64>,
    pub scale: Option<i64>,
    pub length: Option<i64>,
    pub byte_length: Option<i64>,
    pub nullable: bool,
    pub dimension: Option<i64>,
    pub fixed: bool,
    pub column_src_database: String,
    pub column_src_schema: String,
    pub column_src_table: String,
    pub is_auto_increment: bool,
    pub ext_col_type_name: String,
    pub udt_output_type: String,
}

/// Metadata for a single result set (maps to proto ResultSetDescriptor).
#[derive(Clone)]
pub struct ResultSetDescriptor {
    pub query_id: String,
    pub columns: Vec<ColumnMetadata>,
    pub rows_affected: Option<i64>,
    pub row_count: Option<i64>,
    pub statement_type_id: Option<i64>,
    pub sql_state: Option<String>,
    pub stats: Option<Stats>,
    pub number_of_binds: i32,
    pub array_bind_supported: bool,
    pub binds: Vec<ColumnMetadata>,
}

/// A result set handle paired with its descriptor.
pub struct ResultSetInfo {
    pub handle: Handle,
    pub descriptor: ResultSetDescriptor,
}

/// Result of executing a query (maps to proto ExecuteQueryResponse).
pub enum ExecuteQueryResult {
    Single(ResultSetInfo),
    Multi {
        parent: ResultSetDescriptor,
        query_ids: Vec<String>,
        statement_type_ids: Vec<i64>,
    },
}

#[derive(Clone)]
pub enum InlineData {
    /// Base64-encoded Arrow IPC stream.
    ArrowIpc(String),
    /// JSON rowset (rows of nullable string cells).
    Json(Vec<Vec<Option<String>>>),
    None,
}

#[derive(Clone)]
pub struct ChunkData {
    pub format: ChunkFormatKind,
    pub inline: InlineData,
    pub remote_chunks: Vec<ChunkDownloadData>,
}

impl From<&RowsetData> for ChunkData {
    fn from(data: &RowsetData) -> Self {
        match data {
            RowsetData::ArrowSingleChunk { chunk_base64 } => ChunkData {
                format: ChunkFormatKind::ArrowIpc,
                inline: InlineData::ArrowIpc(chunk_base64.clone()),
                remote_chunks: Vec::new(),
            },
            RowsetData::ArrowMultiChunk {
                initial_base64_opt,
                chunk_download_data,
            } => ChunkData {
                format: ChunkFormatKind::ArrowIpc,
                inline: initial_base64_opt
                    .as_ref()
                    .map(|b| InlineData::ArrowIpc(b.clone()))
                    .unwrap_or(InlineData::None),
                remote_chunks: chunk_download_data.clone(),
            },
            RowsetData::JsonRowset { rowset, .. } => ChunkData {
                format: ChunkFormatKind::Json,
                inline: InlineData::Json(rowset.clone()),
                remote_chunks: Vec::new(),
            },
            RowsetData::JsonMultiChunk {
                rowset,
                chunk_download_data,
                ..
            } => ChunkData {
                format: ChunkFormatKind::Json,
                inline: InlineData::Json(rowset.clone()),
                remote_chunks: chunk_download_data.clone(),
            },
            RowsetData::Upload(_)
            | RowsetData::Download(_)
            | RowsetData::SchemaOnly { .. }
            | RowsetData::NoData => ChunkData {
                format: ChunkFormatKind::ArrowIpc,
                inline: InlineData::None,
                remote_chunks: Vec::new(),
            },
        }
    }
}

pub struct ChunkDataWithDescriptor {
    pub chunk_data: ChunkData,
    pub descriptor: ResultSetDescriptor,
}

// --- Internal types ---

/// Resources captured at result set creation time that are needed to build
/// the Arrow stream lazily. Snapshotted up front so the stream can be
/// constructed even after the originating connection has been closed.
pub(super) struct ReaderContext {
    pub http_client: reqwest::Client,
    pub prefetch_config: PrefetchConfig,
}

/// A handle-managed result set. The Arrow stream is built lazily from the stored
/// `RowsetData` when `result_set_get_stream` is called. Since the data is
/// preserved, the stream can be rebuilt on each call.
pub(super) struct ResultSet {
    pub descriptor: ResultSetDescriptor,
    pub data: RowsetData,
    pub reader_ctx: ReaderContext,
}

// --- Response parsing helpers ---

/// The statement type id to classify a response by: the server-provided
/// `statementTypeId`, falling back to the file-transfer `command` when the
/// server omits it (PUT/GET responses sometimes do). Both `response_to_descriptor`
/// and `calculate_rows_affected` key off this so classification never diverges.
fn effective_statement_type_id(data: &Data) -> Option<i64> {
    data.statement_type_id.or(match data.command.as_deref() {
        Some("UPLOAD") => Some(QueryType::PUT_FILES.raw()),
        Some("DOWNLOAD") => Some(QueryType::GET_FILES.raw()),
        _ => None,
    })
}

/// Calculate rows affected from a query response, keyed off the statement's
/// [`ResultKind`] (the shared `query_types::statement_type` classifier).
///
/// - `UpdateCount` (DML): sum the affected-row columns from the rowset.
/// - `Cursor` (SELECT / SHOW / file transfers / ...): the result-set size in
///   `data.total`.
/// - `NoResult` (DDL / TCL / unknown): `None`. Snowflake returns `total: 1` as a
///   generic success marker for these; surfacing it as `rows_affected = 1` is
///   misleading, so we report "not applicable" instead.
pub(super) fn calculate_rows_affected(data: &Data, statement_type_id: Option<i64>) -> Option<i64> {
    match QueryType::from_raw(statement_type_id).result_kind() {
        ResultKind::UpdateCount => Some(sum_dml_affected_rows(data)),
        ResultKind::Cursor => data.total,
        ResultKind::NoResult => None,
    }
}

/// Sum the integer cells of the DML affected-row columns in the first rowset row.
fn sum_dml_affected_rows(data: &Data) -> i64 {
    let (Some(rowset), Some(row_types)) = (&data.rowset, &data.row_type) else {
        return 0;
    };
    if rowset.is_empty() || rowset[0].is_empty() {
        return 0;
    }

    let mut affected_rows = 0i64;
    for (idx, col) in row_types.iter().enumerate() {
        let col_name = col.name.to_lowercase();
        if (DML_AFFECTED_ROWS_COLUMNS.contains(&col_name.as_str())
            || DML_AFFECTED_ROWS_COLUMN_PREFIXES
                .iter()
                .any(|p| col_name.starts_with(p)))
            && let Some(Some(value)) = rowset[0].get(idx)
            && let Ok(count) = value.parse::<i64>()
        {
            affected_rows += count;
        }
    }
    affected_rows
}

pub(super) fn response_to_descriptor(
    data: &Data,
    wrapper_presets: &WrapperPresets,
) -> ResultSetDescriptor {
    let query_id = data.query_id.clone().unwrap_or_default();
    let statement_type_id = effective_statement_type_id(data);
    let rows_affected = calculate_rows_affected(data, statement_type_id);
    let columns = data
        .row_type
        .as_ref()
        .map(|row_types| row_types_to_columns(row_types))
        .unwrap_or_else(|| put_get_columns(data.command.as_deref(), wrapper_presets));
    let binds = data
        .meta_data_of_binds
        .as_ref()
        .map(|row_types| row_types_to_columns(row_types))
        .unwrap_or_default();

    ResultSetDescriptor {
        query_id,
        columns,
        rows_affected,
        row_count: data.total,
        statement_type_id,
        sql_state: data.sql_state.clone(),
        stats: data.stats.clone(),
        number_of_binds: data.number_of_binds.unwrap_or(0),
        array_bind_supported: data.array_bind_supported.unwrap_or(false),
        binds,
    }
}

/// Convert Snowflake `rowType`/`metaDataOfBinds` entries into [`ColumnMetadata`].
fn row_types_to_columns(row_types: &[RowType]) -> Vec<ColumnMetadata> {
    row_types
        .iter()
        .map(|rt| {
            let dimension = rt
                .dimension
                .filter(|&d| d > 0)
                .or(rt.vector_dimension)
                .map(|v| v as i64);
            ColumnMetadata {
                name: rt.name.clone(),
                r#type: rt
                    .ext_type_name
                    .as_ref()
                    .filter(|s| !s.is_empty())
                    .cloned()
                    .unwrap_or_else(|| rt.type_.clone()),
                precision: rt.precision.map(|v| v as i64),
                scale: rt.scale.map(|v| v as i64),
                length: rt.length.map(|v| v as i64),
                byte_length: rt.byte_length.map(|v| v as i64),
                nullable: rt.nullable,
                dimension,
                fixed: rt.fixed.unwrap_or(false),
                column_src_database: rt.database.clone().unwrap_or_default(),
                column_src_schema: rt.schema.clone().unwrap_or_default(),
                column_src_table: rt.table.clone().unwrap_or_default(),
                is_auto_increment: rt.is_auto_increment.unwrap_or(false),
                ext_col_type_name: rt
                    .ext_type_name
                    .as_ref()
                    .filter(|s| !s.is_empty())
                    .cloned()
                    .unwrap_or_default(),
                udt_output_type: rt.output_type.clone().unwrap_or_default(),
            }
        })
        .collect()
}

/// Return client-synthesized column metadata for PUT/GET commands,
/// which don't include `rowType` in the Snowflake response.
fn put_get_columns(command: Option<&str>, wrapper_presets: &WrapperPresets) -> Vec<ColumnMetadata> {
    use super::query::{download_column_metadata, upload_column_metadata};
    match command {
        Some("UPLOAD") => upload_column_metadata(wrapper_presets),
        Some("DOWNLOAD") => download_column_metadata(wrapper_presets),
        _ => Vec::new(),
    }
}

/// Build [`ReaderContext`] by snapshotting the HTTP client and prefetch
/// config from an active connection.
pub(super) async fn resolve_reader_ctx(
    conn: &Arc<Mutex<Connection>>,
) -> Result<ReaderContext, ApiError> {
    let conn_guard = conn.lock().await;
    let http_client = conn_guard
        .http_client
        .clone()
        .context(ConnectionNotInitializedSnafu)?;
    let session_params = conn_guard.session_parameters.read().await;
    let prefetch_config = PrefetchConfig::from_session_params(&session_params);
    Ok(ReaderContext {
        http_client,
        prefetch_config,
    })
}

/// Fetch a query result from Snowflake by query_id via the connection,
/// returning the raw response `Data` for further processing.
pub(super) async fn fetch_query_response_data(
    conn_ptr: &Arc<Mutex<Connection>>,
    query_id: &str,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<Data, ApiError> {
    let (query_parameters, http_client, retry_policy) = {
        let conn = conn_ptr.lock().await;
        (
            conn.query_transport_parameters()?,
            conn.http_client
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
            conn.retry_policy.clone(),
        )
    };

    let response = {
        let mut ctx = RefreshContext::from_arc(conn_ptr).await?;
        let mut last_error = None;
        loop {
            let session_token = ctx.refresh_token(last_error).await?;
            match snowflake_get_query_result(
                &http_client,
                &query_parameters,
                session_token.reveal(),
                query_id,
                &retry_policy,
                cancel.clone(),
            )
            .await
            {
                Ok(response) => break response,
                Err(err) => {
                    last_error = Some(err);
                }
            }
        }
    };

    if response.success {
        let conn = conn_ptr.lock().await;
        conn.update_session_params_cache(
            "",
            response.data.parameters.as_ref(),
            &super::connection::FinalSessionNames {
                database: response.data.final_database_name.clone(),
                schema: response.data.final_schema_name.clone(),
                warehouse: response.data.final_warehouse_name.clone(),
                role: response.data.final_role_name.clone(),
            },
        )
        .await;
    }

    Ok(response.data)
}

/// Snapshots the inputs needed to lazily build a reader and releases the
/// per-result-set lock, so the (possibly network-bound) build never holds the
/// guard across an `.await`.
async fn snapshot_reader_inputs(
    rs_ptr: &Arc<Mutex<ResultSet>>,
) -> (
    RowsetData,
    reqwest::Client,
    PrefetchConfig,
    Vec<ColumnMetadata>,
) {
    let rs = rs_ptr.lock().await;
    (
        rs.data.clone(),
        rs.reader_ctx.http_client.clone(),
        rs.reader_ctx.prefetch_config.clone(),
        rs.descriptor.columns.clone(),
    )
}

// --- DatabaseDriverV1 impl ---

impl DatabaseDriverV1 {
    /// Builds a fresh Arrow [`RecordBatchReader`] for this result set, lazily
    /// from the stored `RowsetData`, so it can be requested multiple times. The
    /// protobuf layer wraps it in an `FFI_ArrowArrayStream` at the C boundary.
    ///
    /// Awaiting this only builds the reader and never blocks. Iterating it,
    /// however, must happen in a synchronous context: chunked result sets pull
    /// chunks via a blocking channel receiver, so draining from within an async
    /// runtime would call `blocking_recv` and panic. Drain after returning from
    /// `block_on` (keeping the runtime alive), on a dedicated `std::thread`, or
    /// via `tokio::task::spawn_blocking`.
    pub async fn result_set_get_stream(
        &self,
        result_handle: Handle,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<Box<dyn RecordBatchReader + Send>, ApiError> {
        let rs_ptr = self
            .results
            .get_obj(result_handle)
            .with_context(|| InvalidArgumentSnafu {
                argument: "result_handle: ResultSet handle not found".to_string(),
            })?;
        let (data, http_client, prefetch_config, columns) = snapshot_reader_inputs(&rs_ptr).await;

        let nullable_flags: Vec<bool> = columns.iter().map(|c| c.nullable).collect();
        let flags = if nullable_flags.is_empty() {
            None
        } else {
            Some(nullable_flags.as_slice())
        };
        let reader = build_reader_from_rowset_data(
            &data,
            http_client,
            &prefetch_config,
            &self.wrapper_presets,
            flags,
            cancel,
        )
        .await
        .context(QueryResponseProcessSnafu)?;
        Ok(reader)
    }

    /// Returns chunk metadata (inline data + remote chunk URLs) for this result set.
    ///
    /// Derives `ChunkData` from the stored `RowsetData` on demand.
    pub async fn result_set_get_chunks(
        &self,
        result_handle: Handle,
    ) -> Result<ChunkDataWithDescriptor, ApiError> {
        let rs_ptr = self
            .results
            .get_obj(result_handle)
            .with_context(|| InvalidArgumentSnafu {
                argument: "ResultSet handle not found".to_string(),
            })?;
        let result_set = rs_ptr.lock().await;

        let chunk_data = (&result_set.data).into();

        Ok(ChunkDataWithDescriptor {
            chunk_data,
            descriptor: result_set.descriptor.clone(),
        })
    }

    pub fn result_set_release(&self, result_handle: Handle) -> Result<(), ApiError> {
        if !self.results.delete_handle(result_handle) {
            return InvalidArgumentSnafu {
                argument: "ResultSet handle not found".to_string(),
            }
            .fail();
        }
        Ok(())
    }

    /// Builds an `ExecuteQueryResult` from pre-resolved `RowsetData`.
    ///
    /// Callers must resolve PUT/GET transfers and convert `Data` into `RowsetData`
    /// before calling this method.
    pub(super) fn build_execute_result(
        &self,
        rowset_data: RowsetData,
        descriptor: ResultSetDescriptor,
        reader_ctx: ReaderContext,
    ) -> ExecuteQueryResult {
        let result_set_handle = self.create_result_set(descriptor.clone(), rowset_data, reader_ctx);
        ExecuteQueryResult::Single(ResultSetInfo {
            handle: result_set_handle,
            descriptor,
        })
    }

    /// Creates a ResultSet and registers it in the handle manager.
    pub(super) fn create_result_set(
        &self,
        descriptor: ResultSetDescriptor,
        data: RowsetData,
        reader_ctx: ReaderContext,
    ) -> Handle {
        let result_set = ResultSet {
            descriptor,
            data,
            reader_ctx,
        };
        self.results.add_handle(Mutex::new(result_set))
    }

    /// Registers a pre-built Arrow RecordBatch as a streamable ResultSet.
    ///
    /// Serializes the batch to Arrow IPC, base64-encodes it as `ArrowSingleChunk`,
    /// and stores it in the handle manager. Used by synthetic result-set paths
    /// (e.g., `connection_get_objects`) that build results in memory rather than
    /// fetching from Snowflake.
    pub fn register_arrow_batch_as_result_set(
        &self,
        batch: &arrow::array::RecordBatch,
        http_client: reqwest::Client,
    ) -> Result<ResultSetInfo, ApiError> {
        use arrow::ipc::writer::StreamWriter;
        use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

        let schema = batch.schema();
        let mut buf = Vec::new();
        {
            let mut writer = StreamWriter::try_new(&mut buf, &schema).context(ArrowParseSnafu)?;
            writer.write(batch).context(ArrowParseSnafu)?;
            writer.finish().context(ArrowParseSnafu)?;
        }

        let chunk_base64 = BASE64.encode(&buf);
        let data = RowsetData::ArrowSingleChunk { chunk_base64 };

        let descriptor = ResultSetDescriptor {
            query_id: String::new(),
            columns: Vec::new(),
            rows_affected: Some(-1),
            row_count: None,
            statement_type_id: None,
            sql_state: None,
            stats: None,
            number_of_binds: 0,
            array_bind_supported: false,
            binds: Vec::new(),
        };

        let reader_ctx = ReaderContext {
            http_client,
            prefetch_config: PrefetchConfig::default(),
        };

        let handle = self.create_result_set(descriptor.clone(), data, reader_ctx);
        Ok(ResultSetInfo { handle, descriptor })
    }

    /// Creates a ResultSet by fetching data from Snowflake by query_id.
    ///
    /// This path is used for multi-statement child results and async query result
    /// retrieval — neither of which involves PUT/GET file transfers.
    pub async fn create_result_set_from_sfqid(
        &self,
        conn_handle: Handle,
        query_id: String,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<ResultSetInfo, ApiError> {
        let conn_ptr =
            self.connections
                .get_obj(conn_handle)
                .with_context(|| InvalidArgumentSnafu {
                    argument: "Connection handle not found".to_string(),
                })?;

        let data = fetch_query_response_data(&conn_ptr, &query_id, cancel).await?;
        let descriptor = response_to_descriptor(&data, &self.wrapper_presets);
        let reader_ctx = resolve_reader_ctx(&conn_ptr).await?;
        let handle =
            self.create_result_set(descriptor.clone(), data.into_rowset_data(), reader_ctx);

        Ok(ResultSetInfo { handle, descriptor })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::snowflake::query_response::Data;
    use arrow::array::{Array, Int64Array, RecordBatch, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};

    const JSON_ROWSET: &str = r#"{
        "queryResultFormat": "json",
        "rowset": [["1", "alice"], ["2", "bob"]],
        "rowtype": [
            {"name": "ID", "type": "FIXED", "nullable": false, "precision": 38, "scale": 0},
            {"name": "NAME", "type": "TEXT", "nullable": true, "length": 100, "byteLength": 400}
        ]
    }"#;

    // Arrow metadata only; the rows ship separately as the base64 IPC stream.
    const ARROW_DATA: &str = r#"{
        "queryResultFormat": "arrow",
        "rowtype": [
            {"name": "ID", "type": "FIXED", "nullable": false, "precision": 38, "scale": 0},
            {"name": "NAME", "type": "TEXT", "nullable": true, "length": 100, "byteLength": 400}
        ]
    }"#;

    /// Two-row `ID`/`NAME` batch encoded as a base64 Arrow IPC stream.
    fn arrow_ipc_rowset_base64() -> String {
        use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

        let schema = Arc::new(Schema::new(vec![
            Field::new("ID", DataType::Int64, false),
            Field::new("NAME", DataType::Utf8, true),
        ]));
        let columns: Vec<Arc<dyn Array>> = vec![
            Arc::new(Int64Array::from(vec![1_i64, 2])),
            Arc::new(StringArray::from(vec!["alice", "bob"])),
        ];
        let batch = RecordBatch::try_new(Arc::clone(&schema), columns)
            .expect("failed to build Arrow record batch fixture");

        let mut buf: Vec<u8> = Vec::new();
        {
            let mut writer = arrow_ipc::writer::StreamWriter::try_new(&mut buf, schema.as_ref())
                .expect("failed to create Arrow IPC stream writer");
            writer.write(&batch).expect("failed to write Arrow batch");
            writer.finish().expect("failed to finish Arrow IPC stream");
        }
        BASE64.encode(&buf)
    }

    /// Drains a `result_set_get_stream` reader and asserts the canonical
    /// two-row `ID`/`NAME` fixture. Prefetched readers pull batches via
    /// `blocking_recv`, which panics inside a Tokio runtime, so this drains in
    /// a sync context — callers must keep their runtime alive across the call.
    fn assert_id_name_reader(reader: Box<dyn RecordBatchReader + Send>) {
        let schema = reader.schema();
        assert_eq!(schema.fields().len(), 2);
        assert_eq!(schema.field(0).name(), "ID");
        assert_eq!(schema.field(0).data_type(), &DataType::Int64);
        assert_eq!(schema.field(1).name(), "NAME");
        assert_eq!(schema.field(1).data_type(), &DataType::Utf8);

        let batches = reader
            .collect::<Result<Vec<_>, _>>()
            .expect("draining the reader should not error");

        assert_eq!(batches.len(), 1);
        let batch = &batches[0];
        assert_eq!(batch.num_rows(), 2);
        let ids = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .expect("column 0 should be Int64");
        assert_eq!(ids.value(0), 1);
        assert_eq!(ids.value(1), 2);
        let names = batch
            .column(1)
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("column 1 should be Utf8");
        assert_eq!(names.value(0), "alice");
        assert_eq!(names.value(1), "bob");
    }

    #[test]
    fn result_set_get_stream_returns_reader_without_ffi() {
        let driver = DatabaseDriverV1::new();
        let data: Data = serde_json::from_str(JSON_ROWSET)
            .expect("fixture must deserialize into query_response::Data");
        let descriptor = response_to_descriptor(&data, &WrapperPresets::default());
        let reader_ctx = ReaderContext {
            http_client: reqwest::Client::new(),
            prefetch_config: PrefetchConfig::default(),
        };
        let handle = driver.create_result_set(descriptor, data.into_rowset_data(), reader_ctx);

        let runtime = tokio::runtime::Runtime::new().expect("failed to build tokio runtime");
        let reader: Box<dyn RecordBatchReader + Send> = runtime
            .block_on(
                driver.result_set_get_stream(handle, tokio_util::sync::CancellationToken::new()),
            )
            .expect("result_set_get_stream should succeed for an inline JSON rowset");

        assert_id_name_reader(reader);
        drop(runtime);
    }

    #[test]
    fn result_set_get_stream_reads_arrow_rowset() {
        let driver = DatabaseDriverV1::new();
        let data: Data = serde_json::from_str(ARROW_DATA)
            .expect("fixture must deserialize into query_response::Data");
        let descriptor = response_to_descriptor(&data, &WrapperPresets::default());
        let reader_ctx = ReaderContext {
            http_client: reqwest::Client::new(),
            prefetch_config: PrefetchConfig::default(),
        };
        // `ArrowMultiChunk` routes through `PrefetchChunkReader` (the
        // `blocking_recv` path in `prefetch.rs`), unlike `ArrowSingleChunk`,
        // which hands back a plain `StreamReader`. An inline initial batch with
        // no remote chunks keeps the test network-free while still exercising
        // the Arrow prefetch/drain path.
        let rowset_data = RowsetData::ArrowMultiChunk {
            initial_base64_opt: Some(arrow_ipc_rowset_base64()),
            chunk_download_data: Vec::new(),
        };
        let handle = driver.create_result_set(descriptor, rowset_data, reader_ctx);

        let runtime = tokio::runtime::Runtime::new().expect("failed to build tokio runtime");
        let reader: Box<dyn RecordBatchReader + Send> = runtime
            .block_on(
                driver.result_set_get_stream(handle, tokio_util::sync::CancellationToken::new()),
            )
            .expect("result_set_get_stream should succeed for an inline Arrow rowset");

        assert_id_name_reader(reader);
        drop(runtime);
    }

    #[test]
    fn response_to_descriptor_sets_row_count_from_total() {
        let json = r#"{
            "queryResultFormat": "json",
            "total": 42,
            "rowset": [["1"]],
            "rowtype": [
                {"name": "ID", "type": "FIXED", "nullable": false, "precision": 38, "scale": 0}
            ]
        }"#;
        let data: Data = serde_json::from_str(json).expect("fixture must deserialize");
        let descriptor = response_to_descriptor(&data, &WrapperPresets::default());
        assert_eq!(descriptor.row_count, Some(42));
    }

    #[test]
    fn response_to_descriptor_leaves_row_count_none_without_total() {
        let data: Data = serde_json::from_str(JSON_ROWSET).expect("fixture must deserialize");
        let descriptor = response_to_descriptor(&data, &WrapperPresets::default());
        assert_eq!(descriptor.row_count, None);
    }

    #[test]
    fn row_types_to_columns_prefers_ext_type_name_for_type() {
        use crate::rest::snowflake::query_response::RowType;

        let row_types = vec![RowType {
            name: "geo_col".to_string(),
            type_: "object".to_string(),
            nullable: true,
            ext_type_name: Some("GEOGRAPHY".to_string()),
            output_type: Some("binary".to_string()),
            ..Default::default()
        }];

        let columns = row_types_to_columns(&row_types);

        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].name, "geo_col");
        assert_eq!(columns[0].r#type, "GEOGRAPHY");
        assert_eq!(columns[0].ext_col_type_name, "GEOGRAPHY");
        assert_eq!(columns[0].udt_output_type, "binary");
    }
}
