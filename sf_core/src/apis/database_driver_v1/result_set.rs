use std::sync::Arc;

use super::connection::{Connection, RefreshContext};
use super::error::*;
use super::global_state::{DatabaseDriverV1, WrapperPresets};
use super::query::build_reader_from_rowset_data;
use crate::chunks::{ChunkDownloadData, ChunkFormatKind, PrefetchConfig};
use crate::handle_manager::Handle;
use crate::rest::snowflake::query_response::{Data, RowsetData, Stats};
use crate::rest::snowflake::snowflake_get_query_result;
use arrow::array::RecordBatchReader;
use arrow::ffi_stream::FFI_ArrowArrayStream;
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
}

/// Metadata for a single result set (maps to proto ResultSetDescriptor).
#[derive(Clone)]
pub struct ResultSetDescriptor {
    pub query_id: String,
    pub columns: Vec<ColumnMetadata>,
    pub rows_affected: Option<i64>,
    pub statement_type_id: Option<i64>,
    pub sql_state: Option<String>,
    pub stats: Option<Stats>,
    pub number_of_binds: i32,
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

// --- DML detection constants ---

const DML_AFFECTED_ROWS_COLUMNS: &[&str] = &[
    "number of rows updated",
    "number of multi-joined rows updated",
    "number of rows deleted",
];
const DML_AFFECTED_ROWS_COLUMN_PREFIXES: &[&str] = &["number of rows inserted"];

const STATEMENT_TYPE_ID_DML: i64 = 0x3000;
const STATEMENT_TYPE_ID_INSERT: i64 = 0x3100;
const STATEMENT_TYPE_ID_UPDATE: i64 = 0x3200;
const STATEMENT_TYPE_ID_DELETE: i64 = 0x3300;
const STATEMENT_TYPE_ID_MERGE: i64 = 0x3400;
const STATEMENT_TYPE_ID_MULTI_TABLE_INSERT: i64 = 0x3500;
const STATEMENT_TYPE_ID_GET_FILES: i64 = 0x7101;
const STATEMENT_TYPE_ID_PUT_FILES: i64 = 0x7102;

// --- Response parsing helpers ---

fn is_dml_statement(statement_type_id: Option<i64>) -> bool {
    statement_type_id.is_some_and(|type_id| {
        matches!(
            type_id,
            STATEMENT_TYPE_ID_DML
                | STATEMENT_TYPE_ID_INSERT
                | STATEMENT_TYPE_ID_UPDATE
                | STATEMENT_TYPE_ID_DELETE
                | STATEMENT_TYPE_ID_MERGE
                | STATEMENT_TYPE_ID_MULTI_TABLE_INSERT
        )
    })
}

/// Calculate rows affected based on statement type.
///
/// Returns `Some(count)` when rows affected is known, `None` when it is not
/// (when the statement type is unknown).
///
/// - For DML: Parse rowset columns to sum affected rows
/// - For SELECT and other queries: Use total field
/// - For unknown: Return None
pub(super) fn calculate_rows_affected(data: &Data) -> Option<i64> {
    if is_dml_statement(data.statement_type_id) {
        if let (Some(rowset), Some(row_types)) = (&data.rowset, &data.row_type)
            && !rowset.is_empty()
            && !rowset[0].is_empty()
        {
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
            return Some(affected_rows);
        }
        return Some(0);
    }

    data.total
}

pub(super) fn response_to_descriptor(
    data: &Data,
    wrapper_presets: &WrapperPresets,
) -> ResultSetDescriptor {
    let query_id = data.query_id.clone().unwrap_or_default();
    let rows_affected = calculate_rows_affected(data);
    let columns = data
        .row_type
        .as_ref()
        .map(|row_types| {
            row_types
                .iter()
                .map(|rt| ColumnMetadata {
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
                })
                .collect()
        })
        .unwrap_or_else(|| put_get_columns(data.command.as_deref(), wrapper_presets));

    let statement_type_id = data.statement_type_id.or(match data.command.as_deref() {
        Some("UPLOAD") => Some(STATEMENT_TYPE_ID_PUT_FILES),
        Some("DOWNLOAD") => Some(STATEMENT_TYPE_ID_GET_FILES),
        _ => None,
    });

    ResultSetDescriptor {
        query_id,
        columns,
        rows_affected,
        statement_type_id,
        sql_state: data.sql_state.clone(),
        stats: data.stats.clone(),
        number_of_binds: data.number_of_binds.unwrap_or(0),
    }
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
) -> (RowsetData, reqwest::Client, PrefetchConfig) {
    let rs = rs_ptr.lock().await;
    (
        rs.data.clone(),
        rs.reader_ctx.http_client.clone(),
        rs.reader_ctx.prefetch_config.clone(),
    )
}

// --- DatabaseDriverV1 impl ---

impl DatabaseDriverV1 {
    /// Builds and returns a fresh Arrow stream for this result set.
    ///
    /// The stream is constructed lazily from the stored `RowsetData`.
    /// Can be called multiple times — the source data is preserved.
    pub async fn result_set_get_stream(
        &self,
        result_handle: Handle,
    ) -> Result<Box<FFI_ArrowArrayStream>, ApiError> {
        let rs_ptr = self.results.get_obj(result_handle).ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "ResultSet handle not found".to_string(),
            }
            .build()
        })?;
        let (data, http_client, prefetch_config) = snapshot_reader_inputs(&rs_ptr).await;

        let reader = build_reader_from_rowset_data(
            &data,
            http_client,
            &prefetch_config,
            &self.wrapper_presets,
        )
        .await
        .context(QueryResponseProcessingSnafu)?;

        Ok(Box::new(FFI_ArrowArrayStream::new(reader)))
    }

    /// Builds and returns a fresh Arrow [`RecordBatchReader`] for this result
    /// set -- the Rust-native counterpart of [`Self::result_set_get_stream`]
    /// that skips the `FFI_ArrowArrayStream` wrapping. Built lazily from the
    /// stored `RowsetData`, so it can be requested multiple times.
    ///
    /// For chunked result sets the reader downloads/parses chunks lazily via a
    /// blocking channel receiver, so it must be drained from a synchronous
    /// context -- never from within an async runtime (a `tokio` task or
    /// `block_on`), which would panic.
    pub async fn result_set_get_reader(
        &self,
        result_handle: Handle,
    ) -> Result<Box<dyn RecordBatchReader + Send>, ApiError> {
        let rs_ptr = self.results.get_obj(result_handle).ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "ResultSet handle not found".to_string(),
            }
            .build()
        })?;
        let (data, http_client, prefetch_config) = snapshot_reader_inputs(&rs_ptr).await;

        build_reader_from_rowset_data(&data, http_client, &prefetch_config, &self.wrapper_presets)
            .await
            .context(QueryResponseProcessingSnafu)
    }

    /// Returns chunk metadata (inline data + remote chunk URLs) for this result set.
    ///
    /// Derives `ChunkData` from the stored `RowsetData` on demand.
    pub async fn result_set_get_chunks(
        &self,
        result_handle: Handle,
    ) -> Result<ChunkDataWithDescriptor, ApiError> {
        let rs_ptr = self.results.get_obj(result_handle).ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "ResultSet handle not found".to_string(),
            }
            .build()
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
            return Err(InvalidArgumentSnafu {
                argument: "ResultSet handle not found".to_string(),
            }
            .build());
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
    fn create_result_set(
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

    /// Creates a ResultSet by fetching data from Snowflake by query_id.
    ///
    /// This path is used for multi-statement child results and async query result
    /// retrieval — neither of which involves PUT/GET file transfers.
    pub async fn create_result_set_from_sfqid(
        &self,
        conn_handle: Handle,
        query_id: String,
    ) -> Result<ResultSetInfo, ApiError> {
        let conn_ptr = self.connections.get_obj(conn_handle).ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .build()
        })?;

        let data = fetch_query_response_data(&conn_ptr, &query_id).await?;
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
    use arrow::array::{Array, Int64Array, StringArray};
    use arrow::datatypes::DataType;

    const JSON_ROWSET: &str = r#"{
        "queryResultFormat": "json",
        "rowset": [["1", "alice"], ["2", "bob"]],
        "rowtype": [
            {"name": "ID", "type": "FIXED", "nullable": false, "precision": 38, "scale": 0},
            {"name": "NAME", "type": "TEXT", "nullable": true, "length": 100, "byteLength": 400}
        ]
    }"#;

    #[test]
    fn result_set_get_reader_returns_reader_without_ffi() {
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
        let reader: Box<dyn RecordBatchReader + Send + 'static> = runtime
            .block_on(driver.result_set_get_reader(handle))
            .expect("result_set_get_reader should succeed for an inline JSON rowset");

        let schema = reader.schema();
        assert_eq!(schema.fields().len(), 2);
        assert_eq!(schema.field(0).name(), "ID");
        assert_eq!(schema.field(0).data_type(), &DataType::Int64);
        assert_eq!(schema.field(1).name(), "NAME");
        assert_eq!(schema.field(1).data_type(), &DataType::Utf8);

        // Drain in a synchronous context (outside the runtime): the chunked
        // reader uses `blocking_recv` and would panic inside an async runtime.
        let batches = reader
            .collect::<Result<Vec<_>, _>>()
            .expect("draining the reader should not error");
        drop(runtime);

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
}
