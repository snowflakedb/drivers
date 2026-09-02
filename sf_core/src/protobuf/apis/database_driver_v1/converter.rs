use crate::apis::database_driver_v1::ChunkDataWithDescriptor;
use crate::apis::database_driver_v1::ColumnMetadata as NativeColumnMetadata;
use crate::apis::database_driver_v1::ConnectionInfo;
use crate::apis::database_driver_v1::ExecuteQueryResult as NativeExecuteQueryResult;
use crate::apis::database_driver_v1::Handle;
use crate::apis::database_driver_v1::InlineData;
use crate::apis::database_driver_v1::ResultSetDescriptor as NativeResultSetDescriptor;
use crate::apis::database_driver_v1::ResultSetInfo as NativeResultSetInfo;
use crate::apis::database_driver_v1::Setting;
use crate::apis::database_driver_v1::error::{
    CancellationAbortResult, InlineJsonEncodeSnafu, InvalidColumnMetadataSnafu,
};
use crate::apis::database_driver_v1::{ApiError, BindingType, DataPtr, ErrorKind as CoreErrorKind};
use crate::apis::database_driver_v1::{
    ValidationCode as CoreValidationCode, ValidationIssue as CoreValidationIssue,
    ValidationSeverity as CoreValidationSeverity,
};
use crate::chunks::{
    ArrowIpcEncodeSnafu, ChunkDownloadData, ChunkError, ChunkFormatKind, ChunkReadSnafu,
    FetchChunkInput, convert_string_rowset_to_arrow_reader,
};
use crate::protobuf::generated::database_driver_v1::result_chunk::Data;
use crate::protobuf::generated::database_driver_v1::*;
use crate::query_types::RowType;
use crate::rest::snowflake::AbortOutcome;
use arrow::array::RecordBatchReader;
use arrow::ffi::FFI_ArrowSchema;
use arrow::ffi_stream::FFI_ArrowArrayStream;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use error_trace::ErrorTrace;
use snafu::ResultExt;
// ---------------------------------------------------------------------------
// Arrow FFI pointer conversions
// ---------------------------------------------------------------------------

impl From<ArrowArrayStreamPtr> for *mut FFI_ArrowArrayStream {
    fn from(ptr: ArrowArrayStreamPtr) -> Self {
        unsafe { std::ptr::read(ptr.value.as_ptr() as *const *mut FFI_ArrowArrayStream) }
    }
}
#[allow(clippy::from_over_into)]
impl Into<*mut FFI_ArrowSchema> for ArrowSchemaPtr {
    fn into(self) -> *mut FFI_ArrowSchema {
        unsafe { std::ptr::read(self.value.as_ptr() as *const *mut FFI_ArrowSchema) }
    }
}

impl From<*mut FFI_ArrowArrayStream> for ArrowArrayStreamPtr {
    fn from(raw: *mut FFI_ArrowArrayStream) -> Self {
        let len = size_of::<*mut FFI_ArrowArrayStream>();
        let buf_ptr = std::ptr::addr_of!(raw) as *const u8;
        let slice = unsafe { std::slice::from_raw_parts(buf_ptr, len) };
        let vec = slice.to_vec();
        ArrowArrayStreamPtr { value: vec }
    }
}

// ---------------------------------------------------------------------------
// Protobuf ↔ native data pointer / binding conversions
// ---------------------------------------------------------------------------

// Convert protobuf BinaryDataPtr to internal DataPtr.
// Both represent a raw pointer + length; this avoids leaking protobuf types into core.
impl<'a> TryFrom<BinaryDataPtr> for DataPtr<'a> {
    type Error = String;

    fn try_from(proto_ptr: BinaryDataPtr) -> Result<Self, Self::Error> {
        let ptr_bytes: [u8; 8] = proto_ptr
            .value
            .as_slice()
            .try_into()
            .map_err(|_| format!("Pointer must be 8 bytes, got {}", proto_ptr.value.len()))?;
        let ptr_value = u64::from_le_bytes(ptr_bytes);
        let ptr = usize::try_from(ptr_value)
            .map_err(|_| format!("Serialized pointer 0x{ptr_value:X} does not fit in usize"))?
            as *const u8;
        Ok(DataPtr::new(ptr, proto_ptr.length))
    }
}

// Convert protobuf QueryBindings variant to internal BindingType.
impl<'a> TryFrom<query_bindings::BindingType> for BindingType<'a> {
    type Error = String;

    fn try_from(proto: query_bindings::BindingType) -> Result<Self, Self::Error> {
        match proto {
            query_bindings::BindingType::Json(ptr) => Ok(BindingType::Json(ptr.try_into()?)),
            query_bindings::BindingType::Csv(ptr) => Ok(BindingType::Csv(ptr.try_into()?)),
        }
    }
}

// ---------------------------------------------------------------------------
// Handle conversions (proto ↔ native)
// ---------------------------------------------------------------------------

impl From<DatabaseHandle> for Handle {
    fn from(handle: DatabaseHandle) -> Self {
        Handle {
            id: handle.id as u64,
            magic: handle.magic as u64,
        }
    }
}

impl From<Handle> for DatabaseHandle {
    fn from(handle: Handle) -> Self {
        DatabaseHandle {
            id: handle.id as i64,
            magic: handle.magic as i64,
        }
    }
}

impl From<ConnectionHandle> for Handle {
    fn from(handle: ConnectionHandle) -> Self {
        Handle {
            id: handle.id as u64,
            magic: handle.magic as u64,
        }
    }
}

impl From<Handle> for ConnectionHandle {
    fn from(handle: Handle) -> Self {
        ConnectionHandle {
            id: handle.id as i64,
            magic: handle.magic as i64,
        }
    }
}

impl From<StatementHandle> for Handle {
    fn from(handle: StatementHandle) -> Self {
        Handle {
            id: handle.id as u64,
            magic: handle.magic as u64,
        }
    }
}

impl From<Handle> for StatementHandle {
    fn from(handle: Handle) -> Self {
        StatementHandle {
            id: handle.id as i64,
            magic: handle.magic as i64,
        }
    }
}

impl From<ResultSetHandle> for Handle {
    fn from(handle: ResultSetHandle) -> Self {
        Handle {
            id: handle.id as u64,
            magic: handle.magic as u64,
        }
    }
}

impl From<Handle> for ResultSetHandle {
    fn from(handle: Handle) -> Self {
        ResultSetHandle {
            id: handle.id as i64,
            magic: handle.magic as i64,
        }
    }
}

impl From<UploadStreamHandle> for Handle {
    fn from(handle: UploadStreamHandle) -> Self {
        Handle {
            id: handle.id as u64,
            magic: handle.magic as u64,
        }
    }
}

impl From<Handle> for UploadStreamHandle {
    fn from(handle: Handle) -> Self {
        UploadStreamHandle {
            id: handle.id as i64,
            magic: handle.magic as i64,
        }
    }
}

impl From<DownloadStreamHandle> for Handle {
    fn from(handle: DownloadStreamHandle) -> Self {
        Handle {
            id: handle.id as u64,
            magic: handle.magic as u64,
        }
    }
}

impl From<Handle> for DownloadStreamHandle {
    fn from(handle: Handle) -> Self {
        DownloadStreamHandle {
            id: handle.id as i64,
            magic: handle.magic as i64,
        }
    }
}

// ---------------------------------------------------------------------------
// Chunk format conversions (native ↔ proto)
// ---------------------------------------------------------------------------

impl From<ChunkFormatKind> for ChunkFormat {
    fn from(value: ChunkFormatKind) -> Self {
        match value {
            ChunkFormatKind::ArrowIpc => ChunkFormat::ArrowIpc,
            ChunkFormatKind::Json => ChunkFormat::Json,
        }
    }
}

/// Proto `ChunkFormat` → `ChunkFormatKind`; returns `None` for unspecified/unknown values.
pub(super) fn proto_chunk_format_to_kind(value: i32) -> Option<ChunkFormatKind> {
    let format = ChunkFormat::try_from(value).unwrap_or(ChunkFormat::Unspecified);
    match format {
        ChunkFormat::ArrowIpc => Some(ChunkFormatKind::ArrowIpc),
        ChunkFormat::Json => Some(ChunkFormatKind::Json),
        ChunkFormat::Unspecified => None,
    }
}

// ---------------------------------------------------------------------------
// Abort-query outcome conversion (native ↔ proto)
// ---------------------------------------------------------------------------

impl From<AbortOutcome> for AbortQueryOutcome {
    fn from(value: AbortOutcome) -> Self {
        match value {
            AbortOutcome::Aborted => AbortQueryOutcome::Aborted,
            AbortOutcome::NotRunning => AbortQueryOutcome::NotRunning,
        }
    }
}

/// Encode a JSON rowset as a base64 Arrow IPC stream so inline chunks ship uniformly over the proto boundary.
pub(super) fn json_rowset_to_arrow_ipc_base64(
    rowset: &[Vec<Option<String>>],
    row_types: &[RowType],
) -> Result<String, ChunkError> {
    let reader = convert_string_rowset_to_arrow_reader(rowset, row_types)?;
    let schema = reader.schema();
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut writer = arrow_ipc::writer::StreamWriter::try_new(&mut buf, schema.as_ref())
            .context(ArrowIpcEncodeSnafu)?;
        for batch in reader {
            // Iterator error = JSON rowset read/decode failure, not IPC encoding.
            let batch = batch.context(ChunkReadSnafu)?;
            writer.write(&batch).context(ArrowIpcEncodeSnafu)?;
        }
        writer.finish().context(ArrowIpcEncodeSnafu)?;
    }
    Ok(BASE64.encode(&buf))
}

// ---------------------------------------------------------------------------
// Result / chunk / column conversions
// ---------------------------------------------------------------------------

impl TryFrom<&ResultChunk> for FetchChunkInput {
    type Error = DriverException;

    fn try_from(chunk: &ResultChunk) -> Result<Self, Self::Error> {
        let data = chunk.data.clone().ok_or_else(|| DriverException {
            message: "Chunk data is required".to_string(),
            kind: ErrorKind::InvalidArgument as i32,
            ..Default::default()
        })?;

        Ok(match data {
            Data::Inline(inline) => FetchChunkInput::Inline(inline),
            Data::Remote(remote_chunk) => FetchChunkInput::Remote(ChunkDownloadData {
                url: remote_chunk.url,
                row_count: 0, // row count is not needed for fetching
                uncompressed_size: remote_chunk.uncompressed_size,
                compressed_size: remote_chunk.compressed_size,
                headers: remote_chunk.headers,
            }),
        })
    }
}

impl From<NativeColumnMetadata> for ColumnMetadata {
    fn from(meta: NativeColumnMetadata) -> Self {
        ColumnMetadata {
            name: meta.name,
            r#type: meta.r#type,
            precision: meta.precision,
            scale: meta.scale,
            length: meta.length,
            byte_length: meta.byte_length,
            nullable: meta.nullable,
            dimension: meta.dimension,
            fixed: meta.fixed,
            column_src_database: meta.column_src_database,
            column_src_schema: meta.column_src_schema,
            column_src_table: meta.column_src_table,
            is_auto_increment: meta.is_auto_increment,
            ext_col_type_name: meta.ext_col_type_name,
            udt_output_type: meta.udt_output_type,
        }
    }
}

impl From<ColumnMetadata> for NativeColumnMetadata {
    fn from(meta: ColumnMetadata) -> Self {
        NativeColumnMetadata {
            name: meta.name,
            r#type: meta.r#type,
            precision: meta.precision,
            scale: meta.scale,
            length: meta.length,
            byte_length: meta.byte_length,
            nullable: meta.nullable,
            dimension: meta.dimension,
            fixed: meta.fixed,
            column_src_database: meta.column_src_database,
            column_src_schema: meta.column_src_schema,
            column_src_table: meta.column_src_table,
            is_auto_increment: meta.is_auto_increment,
            ext_col_type_name: meta.ext_col_type_name,
            udt_output_type: meta.udt_output_type,
        }
    }
}

/// Convert a native `ColumnMetadata` into `RowType` via the shared `query_response::RowType` parser.
// TODO: replace the transient shim with a shared type-string → RowType function.
#[allow(clippy::result_large_err)]
pub(super) fn column_metadata_to_row_type(
    column_metadata: &NativeColumnMetadata,
) -> Result<RowType, ApiError> {
    let temp_row_type = crate::rest::snowflake::query_response::RowType {
        name: column_metadata.name.clone(),
        scale: column_metadata.scale.map(|v| v as u64),
        nullable: column_metadata.nullable,
        type_: column_metadata.r#type.clone(),
        byte_length: column_metadata.byte_length.map(|v| v as u64),
        length: column_metadata.length.map(|v| v as u64),
        precision: column_metadata.precision.map(|v| v as u64),
        ext_type_name: if column_metadata.ext_col_type_name.is_empty() {
            None
        } else {
            Some(column_metadata.ext_col_type_name.clone())
        },
        vector_dimension: column_metadata.dimension.map(|v| v as u64),
        dimension: column_metadata.dimension.map(|v| v as u64),
        fixed: Some(column_metadata.fixed),
        database: Some(column_metadata.column_src_database.clone()),
        schema: Some(column_metadata.column_src_schema.clone()),
        table: Some(column_metadata.column_src_table.clone()),
        is_auto_increment: Some(column_metadata.is_auto_increment),
        output_type: if column_metadata.udt_output_type.is_empty() {
            None
        } else {
            Some(column_metadata.udt_output_type.clone())
        },
        fields: None,
    };
    (&temp_row_type)
        .try_into()
        .context(InvalidColumnMetadataSnafu {
            column: column_metadata.name.clone(),
        })
}

fn native_stats_to_proto(
    stats: &Option<crate::rest::snowflake::query_response::Stats>,
) -> Option<QueryStats> {
    stats.as_ref().map(|s| QueryStats {
        num_rows_inserted: s.num_rows_inserted,
        num_rows_updated: s.num_rows_updated,
        num_rows_deleted: s.num_rows_deleted,
        num_dml_duplicates: s.num_dml_duplicates,
    })
}

impl From<NativeResultSetDescriptor> for ResultSetDescriptor {
    fn from(d: NativeResultSetDescriptor) -> Self {
        ResultSetDescriptor {
            query_id: d.query_id,
            columns: d.columns.into_iter().map(ColumnMetadata::from).collect(),
            rows_affected: d.rows_affected,
            row_count: d.row_count,
            statement_type_id: d.statement_type_id,
            sql_state: d.sql_state,
            stats: native_stats_to_proto(&d.stats),
        }
    }
}

/// `request_id` is `None` only for `connection_get_query_result`, which fetches
/// results for an already-executed query and never generates a new submission UUID.
/// Leaving it `None` there matches legacy `get_results_from_sfqid`, which never
/// set `_request_id` on the outer cursor.
impl From<NativeExecuteQueryResult> for ExecuteQueryResponse {
    fn from(result: NativeExecuteQueryResult) -> Self {
        let (proto_result, request_id) = match result {
            NativeExecuteQueryResult::Single { info, request_id } => (
                execute_query_response::Result::Single(info.into()),
                request_id,
            ),
            NativeExecuteQueryResult::Multi {
                parent,
                query_ids,
                statement_type_ids,
                request_id,
            } => (
                execute_query_response::Result::Multi(MultiStatementResult {
                    parent: Some(parent.into()),
                    query_ids,
                    statement_type_ids,
                }),
                request_id,
            ),
        };
        ExecuteQueryResponse {
            result: Some(proto_result),
            request_id: request_id.map(|id| id.to_string()),
        }
    }
}

/// Wraps a Rust-native `RecordBatchReader` in an Arrow C-stream and returns the
/// FFI pointer the proto responses carry. Keeping this conversion in the
/// protobuf layer lets the core API deal only in `RecordBatchReader`.
pub(super) fn reader_to_arrow_stream_ptr(
    reader: Box<dyn RecordBatchReader + Send>,
) -> ArrowArrayStreamPtr {
    Box::into_raw(Box::new(FFI_ArrowArrayStream::new(reader))).into()
}

impl From<Box<dyn RecordBatchReader + Send>> for ResultSetGetStreamResponse {
    fn from(reader: Box<dyn RecordBatchReader + Send>) -> Self {
        ResultSetGetStreamResponse {
            stream: Some(reader_to_arrow_stream_ptr(reader)),
        }
    }
}

impl From<NativeResultSetInfo> for ResultSetResponse {
    fn from(info: NativeResultSetInfo) -> Self {
        ResultSetResponse {
            result_set_handle: Some(info.handle.into()),
            result_descriptor: Some(info.descriptor.into()),
        }
    }
}

impl TryFrom<ChunkDataWithDescriptor> for ResultSetGetChunksResponse {
    type Error = DriverException;

    fn try_from(value: ChunkDataWithDescriptor) -> Result<Self, Self::Error> {
        let chunk_data = value.chunk_data;
        let descriptor = value.descriptor;

        let columns: Vec<ColumnMetadata> = descriptor
            .columns
            .iter()
            .cloned()
            .map(|c| c.into())
            .collect();

        let mut chunks = Vec::new();

        let inline_base64 = match &chunk_data.inline {
            InlineData::Json(rowset) => {
                let row_types: Vec<RowType> = descriptor
                    .columns
                    .iter()
                    .map(column_metadata_to_row_type)
                    .collect::<Result<Vec<_>, _>>()
                    .to_protobuf()?;
                Some(
                    json_rowset_to_arrow_ipc_base64(rowset, &row_types)
                        .context(InlineJsonEncodeSnafu)
                        .to_protobuf()?,
                )
            }
            InlineData::ArrowIpc(b64) => Some(b64.clone()),
            InlineData::None => None,
        };

        if let Some(base64_data) = inline_base64 {
            let remote_rows: i32 = chunk_data.remote_chunks.iter().map(|c| c.row_count).sum();
            let inline_row_count = descriptor
                .rows_affected
                .map(|total| (total as i32).saturating_sub(remote_rows))
                .unwrap_or(0);

            chunks.push(ResultChunk {
                format: ChunkFormat::ArrowIpc as i32,
                data: Some(result_chunk::Data::Inline(base64_data)),
                row_count: inline_row_count,
            });
        }

        for c in &chunk_data.remote_chunks {
            chunks.push(ResultChunk {
                format: ChunkFormat::from(chunk_data.format) as i32,
                data: Some(result_chunk::Data::Remote(RemoteChunk {
                    url: c.url.clone(),
                    headers: c.headers.clone(),
                    compressed_size: c.compressed_size,
                    uncompressed_size: c.uncompressed_size,
                })),
                row_count: c.row_count,
            });
        }

        Ok(ResultSetGetChunksResponse { chunks, columns })
    }
}

impl ConnectionGetInfoResponse {
    /// Build from `ConnectionInfo`, optionally revealing `master_token`.
    ///
    /// `master_token` is only populated when `include_master_token` is true
    /// to minimize accidental exposure of sensitive material.
    pub(super) fn from_info(info: ConnectionInfo, include_master_token: bool) -> Self {
        ConnectionGetInfoResponse {
            host: info.host,
            port: info.port,
            server_url: info.server_url,
            session_token: info.session_token.map(|t| t.reveal().to_string()),
            session_id: info.session_id,
            account: info.account,
            user: info.user,
            role: info.role,
            database: info.database,
            schema: info.schema,
            warehouse: info.warehouse,
            master_token: if include_master_token {
                info.master_token.map(|t| t.reveal().to_string())
            } else {
                None
            },
            user_agent: info.user_agent,
            proxy_host: info.proxy_host,
            proxy_port: info.proxy_port,
            proxy_user: info.proxy_user,
            proxy_password: info.proxy_password.map(|p| p.reveal().to_string()),
            no_proxy: info.no_proxy,
        }
    }
}

impl From<crate::rest::snowflake::QueryStatusResult> for ConnectionGetQueryStatusResponse {
    fn from(result: crate::rest::snowflake::QueryStatusResult) -> Self {
        ConnectionGetQueryStatusResponse {
            status_name: result.status_name,
            error_code: result.error_code,
            error_message: result.error_message,
            end_time: result.end_time,
            start_time: result.start_time,
            total_duration: result.total_duration,
            query_id: result.query_id,
            session_id: result.session_id,
            sql_text: result.sql_text,
            warehouse_id: result.warehouse_id,
            warehouse_name: result.warehouse_name,
            warehouse_external_size: result.warehouse_external_size,
            warehouse_server_type: result.warehouse_server_type,
            state: result.state,
        }
    }
}

// ---------------------------------------------------------------------------
// Setting / config conversions
// ---------------------------------------------------------------------------

/// Recursively convert a TOML value into a JSON value, preserving all nesting.
///
/// Used to hand the full merged config document to Python via `json.loads`.
/// Unlike a scalar-only flattening, nested tables (e.g. `[cli.plugins.<name>]`)
/// and arrays are carried through intact.
pub(super) fn toml_value_to_json(value: &toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::Value::Number((*i).into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        // TOML datetimes have no JSON counterpart; emit their RFC 3339 string.
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(toml_value_to_json).collect())
        }
        toml::Value::Table(table) => serde_json::Value::Object(
            table
                .iter()
                .map(|(k, v)| (k.clone(), toml_value_to_json(v)))
                .collect(),
        ),
    }
}

pub(super) fn config_setting_to_setting(cs: ConfigSetting) -> Option<Setting> {
    match cs.value? {
        config_setting::Value::StringValue(s) => Some(Setting::String(s)),
        config_setting::Value::IntValue(i) => Some(Setting::Int(i)),
        config_setting::Value::DoubleValue(d) => Some(Setting::Double(d)),
        config_setting::Value::BytesValue(b) => Some(Setting::Bytes(b)),
        config_setting::Value::BoolValue(b) => Some(Setting::Bool(b)),
    }
}

pub(super) fn proto_options_to_hashmap(
    options: std::collections::HashMap<String, ConfigSetting>,
) -> std::collections::HashMap<String, Setting> {
    options
        .into_iter()
        .filter_map(|(k, v)| config_setting_to_setting(v).map(|s| (k, s)))
        .collect()
}

// ---------------------------------------------------------------------------
// Validation issue conversion
// ---------------------------------------------------------------------------

pub(super) fn core_validation_code_to_proto(code: CoreValidationCode) -> i32 {
    match code {
        CoreValidationCode::Unspecified => ValidationCode::Unspecified as i32,
        CoreValidationCode::MissingRequired => ValidationCode::MissingRequired as i32,
        CoreValidationCode::InvalidType => ValidationCode::InvalidType as i32,
        CoreValidationCode::InvalidValue => ValidationCode::InvalidValue as i32,
        CoreValidationCode::UnknownParameter => ValidationCode::UnknownParameter as i32,
        CoreValidationCode::DeprecatedParameter => ValidationCode::DeprecatedParameter as i32,
        CoreValidationCode::ConflictingParameters => ValidationCode::ConflictingParameters as i32,
        CoreValidationCode::ConflictingWifParameters => {
            ValidationCode::ConflictingWifParameters as i32
        }
    }
}

pub(super) fn core_validation_issue_to_proto(issue: CoreValidationIssue) -> ValidationIssue {
    let severity = match issue.severity {
        CoreValidationSeverity::Error => ValidationSeverity::Error as i32,
        CoreValidationSeverity::Warning => ValidationSeverity::Warning as i32,
    };
    ValidationIssue {
        severity,
        parameter: issue.parameter,
        message: issue.message,
        code: core_validation_code_to_proto(issue.code),
    }
}

// ---------------------------------------------------------------------------
// Error conversion (ApiError → DriverException)
// ---------------------------------------------------------------------------

impl From<CoreErrorKind> for ErrorKind {
    fn from(value: CoreErrorKind) -> Self {
        match value {
            CoreErrorKind::Unspecified => ErrorKind::Unspecified,
            CoreErrorKind::AuthenticationError => ErrorKind::AuthenticationError,
            CoreErrorKind::NotImplemented => ErrorKind::NotImplemented,
            CoreErrorKind::InvalidArgument => ErrorKind::InvalidArgument,
            CoreErrorKind::Io => ErrorKind::Io,
            CoreErrorKind::Cancelled => ErrorKind::Cancelled,
            CoreErrorKind::InternalError => ErrorKind::InternalError,
            CoreErrorKind::MissingParameter => ErrorKind::MissingParameter,
            CoreErrorKind::InvalidParameterValue => ErrorKind::InvalidParameterValue,
            CoreErrorKind::LoginError => ErrorKind::LoginError,
            CoreErrorKind::LocalFileNotFound => ErrorKind::LocalFileNotFound,
            CoreErrorKind::RemoteFileNotFound => ErrorKind::RemoteFileNotFound,
            CoreErrorKind::UnsupportedCompression => ErrorKind::UnsupportedCompression,
            CoreErrorKind::QueryFailed => ErrorKind::QueryFailed,
            CoreErrorKind::Timeout => ErrorKind::Timeout,
            CoreErrorKind::StageBinding => ErrorKind::StageBinding,
        }
    }
}

fn to_driver_exception(error: ApiError) -> DriverException {
    let error_trace = error
        .error_trace()
        .into_iter()
        .map(|entry| ErrorTraceEntry {
            file: entry.location.file,
            line: entry.location.line,
            column: entry.location.column,
            message: entry.message,
        })
        .collect();
    DriverException {
        message: error.to_string(),
        kind: ErrorKind::from(error.kind()) as i32,
        error_trace,
        vendor_code: error.vendor_code(),
        sql_state: error.sql_state(),
        root_cause: error.root_cause(),
        query_id: error.query_id(),
        request_id: error.request_id(),
        parameter: error.parameter(),
        parameter_value: error.parameter_value(),
        validation_code: error.validation_code().map(core_validation_code_to_proto),
        reauthentication_required: error.reauthentication_required(),
        cancellation_abort_outcome: error
            .cancellation_abort_outcome()
            .map(|a| cancel_abort_to_proto(a) as i32),
    }
}

fn cancel_abort_to_proto(abort: CancellationAbortResult) -> CancellationAbortOutcome {
    match abort {
        CancellationAbortResult::Aborted => CancellationAbortOutcome::Aborted,
        CancellationAbortResult::NotRunning => CancellationAbortOutcome::NotRunning,
        CancellationAbortResult::NotConfirmed => CancellationAbortOutcome::NotConfirmed,
    }
}

// ---------------------------------------------------------------------------
// ToProtobuf trait — converts Result<T, ApiError> → Result<T, DriverException>
// ---------------------------------------------------------------------------

pub(super) trait ToProtobuf<T> {
    #[allow(clippy::result_large_err)]
    fn to_protobuf(self) -> Result<T, DriverException>;
}

impl<T> ToProtobuf<T> for Result<T, ApiError> {
    #[allow(clippy::result_large_err)]
    fn to_protobuf(self) -> Result<T, DriverException> {
        self.map_err(to_driver_exception)
    }
}

impl From<ApiError> for DriverException {
    fn from(error: ApiError) -> Self {
        to_driver_exception(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apis::database_driver_v1::ApiError;
    use crate::apis::database_driver_v1::error::ConfigError;
    use crate::apis::database_driver_v1::error::RestError;
    use crate::rest::snowflake::{
        GS_CODE_UNAVAILABLE, MASTER_TOKEN_EXPIRED, QueryIds, SESSION_TOKEN_EXPIRED,
        SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED,
    };
    use snafu::Location;

    fn loc() -> Location {
        Location::new("test", 0, 0)
    }

    fn vendor_pair(err: &ApiError) -> (Option<i32>, Option<String>) {
        let snowflake_ctx = err.snowflake_context();
        (snowflake_ctx.vendor_code, snowflake_ctx.sql_state)
    }

    fn query_failed(code: Option<i32>, sql_state: Option<&str>) -> ApiError {
        ApiError::Query {
            location: loc(),
            source: Box::new(RestError::QueryFailed {
                message: "test".to_owned(),
                code,
                sql_state: sql_state.map(|s| s.to_owned()),
                ids: QueryIds::default(),
                location: loc(),
                query_context: None,
            }),
        }
    }

    fn login_error(code: i32, reauthentication_required: bool) -> ApiError {
        ApiError::Login {
            location: loc(),
            source: Box::new(RestError::LoginError {
                message: "test".to_owned(),
                code,
                reauthentication_required,
                location: loc(),
            }),
        }
    }

    fn master_token_terminal(code: Option<i32>) -> ApiError {
        ApiError::MasterTokenTerminal {
            master_token_gs_code: code,
            location: loc(),
        }
    }

    #[test]
    fn query_failed_passes_through_server_sql_state() {
        let err = query_failed(Some(1003), Some("42000"));
        assert_eq!(vendor_pair(&err), (Some(1003), Some("42000".to_owned())));
    }

    #[test]
    fn server_sql_state_wins_over_lookup_table() {
        // If the server provides "22000", trust it even though our table maps
        // 100038 → "22003". The wire value is authoritative.
        let err = query_failed(Some(100038), Some("22000"));
        assert_eq!(vendor_pair(&err), (Some(100038), Some("22000".to_owned())));
    }

    #[test]
    fn query_failed_falls_back_to_lookup_when_sql_state_missing() {
        let err = query_failed(Some(100038), None);
        assert_eq!(vendor_pair(&err), (Some(100038), Some("22003".to_owned())));

        let err = query_failed(Some(100078), None);
        assert_eq!(vendor_pair(&err), (Some(100078), Some("22001".to_owned())));

        let err = query_failed(Some(1003), None);
        assert_eq!(vendor_pair(&err), (Some(1003), Some("42000".to_owned())));
    }

    #[test]
    fn unknown_code_with_no_sql_state_stays_none() {
        let err = query_failed(Some(999_999), None);
        assert_eq!(vendor_pair(&err), (Some(999_999), None));
    }

    #[test]
    fn missing_code_and_sql_state_stays_none() {
        let err = query_failed(None, None);
        assert_eq!(vendor_pair(&err), (None, None));
    }

    #[test]
    fn login_credential_rejection_surfaces_code_and_authorization_sql_state() {
        let err = login_error(390100, false);
        assert_eq!(vendor_pair(&err), (Some(390100), Some("28000".to_owned())));
    }

    #[test]
    fn login_non_rejection_non_reauth_code_surfaces_code_with_no_sql_state() {
        // Neither a credential rejection nor reauth-shaped: sql_state stays
        // unset so callers apply their own default.
        let err = login_error(390111, false);
        assert_eq!(vendor_pair(&err), (Some(390111), None));
    }

    #[test]
    fn login_missing_code_sentinel_stays_none() {
        // GS_CODE_UNAVAILABLE is used when the server omitted or sent a
        // non-numeric code; treat as "no vendor code" so callers fall back
        // to their default errno.
        let err = login_error(GS_CODE_UNAVAILABLE, false);
        assert_eq!(vendor_pair(&err), (None, None));
    }

    #[test]
    fn master_token_terminal_populates_vendor_code_and_connection_sql_state() {
        // MasterTokenTerminal codes (390113/390114/390115) never indicate a
        // credential rejection, so sql_state is unconditionally "08001".
        let err = master_token_terminal(Some(390114));
        assert_eq!(vendor_pair(&err), (Some(390114), Some("08001".to_owned())));
    }

    #[test]
    fn master_token_terminal_with_no_server_code_still_gets_connection_sql_state() {
        // Client-predicted expiry with no server round-trip: no vendor_code
        // to surface, but the SQLSTATE classification doesn't depend on
        // having a code.
        let err = master_token_terminal(None);
        assert_eq!(vendor_pair(&err), (None, Some("08001".to_owned())));
    }

    #[test]
    fn login_reauth_flag_now_populates_vendor_code_and_connection_sql_state() {
        // reauthentication_required=true resolves to "08001" regardless of
        // CREDENTIAL_REJECTION_GS_CODES membership.
        let err = login_error(390195, true);
        assert_eq!(vendor_pair(&err), (Some(390195), Some("08001".to_owned())));
    }

    #[test]
    fn query_error_to_string_has_no_wrapper_prefixes() {
        // Server errors should surface verbatim: no "Query execution failed:"
        // and no "Query failed:" prefix — matching the legacy Python driver.
        let err = ApiError::Query {
            location: loc(),
            source: Box::new(RestError::QueryFailed {
                message: "SQL compilation error: Object 'FOO' does not exist.".to_owned(),
                code: Some(2003),
                sql_state: Some("42S02".to_owned()),
                ids: QueryIds::default(),
                query_context: None,
                location: loc(),
            }),
        };
        assert_eq!(
            err.to_string(),
            "SQL compilation error: Object 'FOO' does not exist."
        );
    }

    #[test]
    fn query_error_driver_exception_message_has_no_wrapper_prefixes() {
        // End-to-end: the DriverException.message field (what Python reads)
        // should contain only the server error, no wrapper prefixes.
        let err = ApiError::Query {
            location: loc(),
            source: Box::new(RestError::QueryFailed {
                message: "SQL compilation error: syntax error line 1".to_owned(),
                code: Some(1003),
                sql_state: Some("42000".to_owned()),
                ids: QueryIds::default(),
                query_context: None,
                location: loc(),
            }),
        };
        let exc = to_driver_exception(err);
        assert_eq!(exc.message, "SQL compilation error: syntax error line 1");
        assert_eq!(exc.kind, ErrorKind::QueryFailed as i32);
    }

    fn validation_error(issues: Vec<CoreValidationIssue>) -> ApiError {
        ApiError::Configuration {
            location: loc(),
            source: Box::from(ConfigError::Validation {
                issues,
                location: loc(),
            }),
        }
    }

    fn conflicting_parameters_issue(parameter: &str, message: &str) -> CoreValidationIssue {
        CoreValidationIssue {
            severity: CoreValidationSeverity::Error,
            parameter: parameter.to_owned(),
            message: message.to_owned(),
            code: CoreValidationCode::ConflictingParameters,
        }
    }

    fn conflicting_wif_parameters_issue(parameter: &str, message: &str) -> CoreValidationIssue {
        CoreValidationIssue {
            severity: CoreValidationSeverity::Error,
            parameter: parameter.to_owned(),
            message: message.to_owned(),
            code: CoreValidationCode::ConflictingWifParameters,
        }
    }

    #[test]
    fn validation_error_carries_first_issue_validation_code_on_the_wire() {
        // The WIF cross-param guards must surface their dedicated ValidationCode
        // so wrappers can discriminate without substring-matching the message.
        let err = validation_error(vec![conflicting_wif_parameters_issue(
            "workload_identity_provider",
            "workload_identity_provider was set but authenticator was not set to WORKLOAD_IDENTITY",
        )]);
        let exc = to_driver_exception(err);
        assert_eq!(exc.parameter.as_deref(), Some("workload_identity_provider"));
        assert_eq!(
            exc.validation_code,
            Some(ValidationCode::ConflictingWifParameters as i32)
        );
    }

    #[test]
    fn validation_error_with_multiple_issues_prefers_wif_conflict_issue() {
        // A WIF-conflict issue always wins the `parameter`/`code` slot over any
        // other Error-severity issue in the same batch, regardless of position
        // in `issues` — see `wif_conflict_wins_even_when_pushed_after_an_unrelated_issue`
        // for the ordering-sensitive case this guards against.
        let err = validation_error(vec![
            conflicting_wif_parameters_issue(
                "workload_identity_impersonation_path",
                "unsupported for OIDC",
            ),
            CoreValidationIssue {
                severity: CoreValidationSeverity::Error,
                parameter: "token".to_owned(),
                message: "Missing required parameter 'token'".to_owned(),
                code: CoreValidationCode::InvalidValue,
            },
        ]);
        let exc = to_driver_exception(err);
        assert_eq!(
            exc.parameter.as_deref(),
            Some("workload_identity_impersonation_path")
        );
        assert_eq!(
            exc.validation_code,
            Some(ValidationCode::ConflictingWifParameters as i32)
        );
        assert_eq!(
            exc.message,
            "Configuration error: Configuration validation failed (2 issue(s)): \
             workload_identity_impersonation_path: unsupported for OIDC; \
             token: Missing required parameter 'token'"
        );
    }

    #[test]
    fn wif_conflict_wins_even_when_pushed_after_an_unrelated_issue() {
        // Regression test: before the fix, `first_param`/`first_code` took
        // `issues.first()` unconditionally, so a non-WIF issue pushed before
        // the WIF-conflict issue would shadow it on the wire, silently
        // defeating `_is_wif_conflict` on the Python side.
        let err = validation_error(vec![
            CoreValidationIssue {
                severity: CoreValidationSeverity::Error,
                parameter: "some_unrelated_param".to_owned(),
                message: "unrelated invalid value".to_owned(),
                code: CoreValidationCode::InvalidValue,
            },
            conflicting_wif_parameters_issue(
                "workload_identity_provider",
                "workload_identity_provider was set but authenticator was not WORKLOAD_IDENTITY",
            ),
        ]);
        let exc = to_driver_exception(err);
        assert_eq!(exc.parameter.as_deref(), Some("workload_identity_provider"));
        assert_eq!(
            exc.validation_code,
            Some(ValidationCode::ConflictingWifParameters as i32)
        );
    }

    #[test]
    fn validation_error_for_private_key_conflict_does_not_get_wif_code() {
        // private_key + private_key_file is a ConflictingParameters issue that
        // shares the abstract bucket with the WIF guards but must NOT be
        // remapped to the WIF-specific code (no false positive).
        let err = validation_error(vec![conflicting_parameters_issue(
            "private_key",
            "Both 'private_key' and 'private_key_file' are set. Please provide only one.",
        )]);
        let exc = to_driver_exception(err);
        assert_eq!(exc.parameter.as_deref(), Some("private_key"));
        assert_eq!(
            exc.validation_code,
            Some(ValidationCode::ConflictingParameters as i32)
        );
    }

    #[test]
    fn validation_error_missing_required_takes_priority_over_code_field() {
        // Existing behavior: any MissingRequired issue routes to MissingParameter
        // instead of InvalidParameterValue, regardless of ValidationCode on other
        // issues. MissingParameter carries no validation_code.
        let err = validation_error(vec![
            conflicting_parameters_issue("workload_identity_provider", "conflict"),
            CoreValidationIssue {
                severity: CoreValidationSeverity::Error,
                parameter: "account".to_owned(),
                message: "Missing required parameter 'account'".to_owned(),
                code: CoreValidationCode::MissingRequired,
            },
        ]);
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::MissingParameter as i32);
        assert_eq!(exc.parameter.as_deref(), Some("account"));
        assert_eq!(exc.validation_code, None);
    }

    #[test]
    fn master_token_terminal_constructs_auth_error_with_reauth_required() {
        let err = master_token_terminal(Some(390113));
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::AuthenticationError as i32);
        assert!(exc.reauthentication_required);
        assert_eq!(exc.vendor_code, Some(390113));
    }

    #[test]
    fn master_token_terminal_with_no_server_code_still_sets_reauth_required() {
        // Client-side-predicted expiry: no server round-trip, so no GS code
        // exists anywhere for this error — but the discriminant is still
        // `true`, since the session genuinely can't be renewed either way.
        // The real code (when there is one) travels via vendor_code, not
        // through this field, so its absence here doesn't affect the flag.
        let err = master_token_terminal(None);
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::AuthenticationError as i32);
        assert!(exc.reauthentication_required);
        assert_eq!(exc.vendor_code, None);
    }

    #[test]
    fn login_reauth_flag_constructs_auth_error_not_login_error() {
        let err = login_error(390195, true);
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::AuthenticationError as i32);
        assert!(exc.reauthentication_required);
        assert_eq!(exc.vendor_code, Some(390195));
    }

    #[test]
    fn login_error_with_flag_false_stays_a_plain_login_error() {
        // 390100 is not reauth-shaped either way — a weaker guard than 390195
        // below, which IS reauth-shaped, so this also pins that conversion
        // trusts the flag rather than re-deriving reauth-ness from the code.
        for code in [390100, 390195] {
            let err = login_error(code, false);
            let exc = to_driver_exception(err);
            assert_eq!(
                exc.kind,
                ErrorKind::LoginError as i32,
                "expected LoginError for code={code}"
            );
            assert!(!exc.reauthentication_required);
            assert_eq!(exc.vendor_code, Some(code));
        }
    }

    #[test]
    fn direct_invalid_parameter_value_error_has_no_validation_code() {
        // ConfigError::InvalidParameterValue does not originate from
        // validate_settings, so it carries no ValidationCode.
        let err = ApiError::Configuration {
            location: loc(),
            source: Box::from(ConfigError::InvalidParameterValue {
                parameter: "authenticator".to_owned(),
                value: "BAD".to_owned(),
                explanation: "not supported".to_owned(),
                location: loc(),
            }),
        };
        let exc = to_driver_exception(err);
        assert_eq!(exc.parameter.as_deref(), Some("authenticator"));
        assert_eq!(exc.parameter_value.as_deref(), Some("BAD"));
        assert_eq!(exc.validation_code, None);
    }

    #[test]
    fn conflicting_parameters_copies_fields_from_the_error() {
        let err = ApiError::Configuration {
            location: loc(),
            source: Box::from(ConfigError::ConflictingParameters {
                parameter: "private_key".to_owned(),
                value: "private_key_file".to_owned(),
                explanation:
                    "Both 'private_key' and 'private_key_file' are set. Please provide only one."
                        .to_owned(),
                location: loc(),
            }),
        };
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::InvalidParameterValue as i32);
        assert_eq!(exc.parameter.as_deref(), Some("private_key"));
        assert_eq!(exc.parameter_value.as_deref(), Some("private_key_file"));
        assert_eq!(exc.validation_code, None);
    }

    #[test]
    fn invalid_wif_provider_message_lists_allowed_values() {
        let exc = to_driver_exception(ApiError::InvalidWifProvider {
            provider: "invalid".to_owned(),
            location: loc(),
        });

        assert_eq!(
            exc.message,
            format!(
                "Invalid workload_identity_provider: 'invalid'. Allowed values: {}",
                crate::config::rest_parameters::WifProvider::allowed_values()
            )
        );
    }

    #[test]
    fn query_failed_populates_query_id_and_request_id() {
        // A synchronous query failure carries both the server-assigned query_id
        // and the client-generated request_id all the way to DriverException.
        let request_id = uuid::Uuid::new_v4();
        let err = ApiError::Query {
            location: loc(),
            source: Box::new(RestError::QueryFailed {
                message: "SQL compilation error".to_owned(),
                code: Some(1003),
                sql_state: Some("42000".to_owned()),
                ids: QueryIds {
                    request_id: Some(request_id),
                    query_id: Some("01abc-def-12345".to_owned()),
                },
                query_context: None,
                location: loc(),
            }),
        };
        assert_eq!(
            err.snowflake_context().query_id.as_deref(),
            Some("01abc-def-12345")
        );
        assert_eq!(
            err.snowflake_context().request_id,
            Some(request_id.to_string())
        );

        let exc = to_driver_exception(err);
        assert_eq!(exc.query_id, Some("01abc-def-12345".to_owned()));
        assert_eq!(exc.request_id, Some(request_id.to_string()));
    }

    #[test]
    fn query_failed_without_ids_leaves_fields_unset() {
        // When the server omits the query_id and no request_id was recorded,
        // both fields stay None on DriverException.
        let err = query_failed(Some(1003), Some("42000"));
        let snowflake_ctx = err.snowflake_context();
        assert_eq!(snowflake_ctx.query_id, None);
        assert_eq!(snowflake_ctx.request_id, None);

        let exc = to_driver_exception(err);
        assert_eq!(exc.query_id, None);
        assert_eq!(exc.request_id, None);
    }

    #[test]
    fn query_timeout_carries_request_id_and_hyt00() {
        let err = crate::apis::database_driver_v1::error::QueryTimeoutSnafu {
            budget: std::time::Duration::from_secs(1),
            request_id: "req-123".to_owned(),
        }
        .build();
        let exc = to_driver_exception(err);
        assert_eq!(exc.request_id.as_deref(), Some("req-123"));
        assert_eq!(exc.sql_state.as_deref(), Some("HYT00"));
        assert_eq!(exc.query_id, None);
        assert_eq!(exc.kind, ErrorKind::Timeout as i32);
        assert_eq!(exc.vendor_code, None);
    }

    #[test]
    fn cancel_timeout_maps_to_timeout_kind_and_hyt00() {
        let err = crate::apis::database_driver_v1::error::CancelTimeoutSnafu {
            timeout: std::time::Duration::from_secs(30),
            request_id: "req-cancel".to_owned(),
        }
        .build();
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::Timeout as i32);
        assert_eq!(exc.sql_state.as_deref(), Some("HYT00"));
        assert_eq!(exc.request_id.as_deref(), Some("req-cancel"));
        assert_eq!(exc.vendor_code, None);
    }

    #[test]
    fn stage_binding_maps_to_stage_binding_kind() {
        let err = ApiError::StageBinding {
            location: loc(),
            source: Box::new(crate::stage_binding::StageBindingError::Disabled { location: loc() }),
        };
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::StageBinding as i32);
        assert_eq!(exc.sql_state, None);
        assert_eq!(exc.vendor_code, None);
    }

    #[test]
    fn login_operation_timeout_maps_to_timeout_kind_and_hyt00() {
        let err = ApiError::Login {
            location: loc(),
            source: Box::new(RestError::OperationTimeout {
                operation: "login".to_owned(),
                budget: std::time::Duration::from_secs(1),
                ids: QueryIds::default(),
                location: loc(),
            }),
        };
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::Timeout as i32);
        assert_eq!(exc.sql_state.as_deref(), Some("HYT00"));
        assert_eq!(exc.request_id, None);
        assert_eq!(exc.query_id, None);
        assert_eq!(exc.vendor_code, None);
    }

    #[test]
    fn query_failed_maps_to_query_failed_kind() {
        let exc = to_driver_exception(query_failed(Some(1003), None));
        assert_eq!(exc.kind, ErrorKind::QueryFailed as i32);
    }

    #[test]
    fn query_operation_timeout_maps_to_timeout_kind() {
        let err = ApiError::Query {
            location: loc(),
            source: Box::new(RestError::OperationTimeout {
                operation: "query".to_owned(),
                budget: std::time::Duration::from_secs(1),
                ids: QueryIds::default(),
                location: loc(),
            }),
        };
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::Timeout as i32);
        assert_eq!(exc.sql_state.as_deref(), Some("HYT00"));
    }

    #[test]
    fn query_http_retry_maps_to_io_kind() {
        let err = ApiError::Query {
            location: loc(),
            source: Box::new(RestError::HttpRetry {
                context: "query",
                ids: QueryIds::default(),
                source: crate::http::retry::HttpError::MaxAttempts {
                    attempts: 3,
                    last_status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
                    location: loc(),
                },
                location: loc(),
            }),
        };
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::Io as i32);
    }

    #[test]
    fn session_expired_response_maps_to_authentication_kind_with_390112() {
        let err = ApiError::Query {
            location: loc(),
            source: Box::new(RestError::SessionExpired { location: loc() }),
        };
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::AuthenticationError as i32);
        assert_eq!(exc.vendor_code, Some(SESSION_TOKEN_EXPIRED));
        assert_eq!(
            exc.sql_state.as_deref(),
            Some(SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED)
        );
    }

    #[test]
    fn master_token_terminal_response_maps_to_authentication_kind() {
        let err = ApiError::Query {
            location: loc(),
            source: Box::new(RestError::MasterTokenTerminal {
                code: 390114,
                location: loc(),
            }),
        };
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::AuthenticationError as i32);
        assert_eq!(exc.vendor_code, Some(390114));
        assert_eq!(exc.sql_state.as_deref(), Some("08001"));
    }

    #[test]
    fn session_refresh_failed_passes_vendor_code_through_session_refresh() {
        let err = ApiError::SessionRefresh {
            location: loc(),
            source: Box::new(RestError::SessionRefreshFailed {
                message: "expired".to_owned(),
                code: 390111,
                location: loc(),
            }),
        };
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::AuthenticationError as i32);
        assert_eq!(exc.vendor_code, Some(390111));
        assert_eq!(
            exc.sql_state.as_deref(),
            Some(SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED)
        );
    }

    #[test]
    fn token_request_failed_passes_vendor_code_through_token_request() {
        let err = ApiError::TokenRequest {
            location: loc(),
            source: Box::new(RestError::TokenRequestFailed {
                operation: "RENEW".to_owned(),
                message: "expired".to_owned(),
                code: 390111,
                location: loc(),
            }),
        };
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::AuthenticationError as i32);
        assert_eq!(exc.vendor_code, Some(390111));
        assert_eq!(
            exc.sql_state.as_deref(),
            Some(SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED)
        );
    }

    #[test]
    fn session_refresh_master_token_terminal_keeps_code() {
        let err = ApiError::SessionRefresh {
            location: loc(),
            source: Box::new(RestError::MasterTokenTerminal {
                code: MASTER_TOKEN_EXPIRED,
                location: loc(),
            }),
        };
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::AuthenticationError as i32);
        assert_eq!(exc.vendor_code, Some(MASTER_TOKEN_EXPIRED));
        assert_eq!(
            exc.sql_state.as_deref(),
            Some(SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED)
        );
    }

    #[test]
    fn query_poll_deadline_maps_to_timeout_kind_and_keeps_ids() {
        let request_id = uuid::Uuid::new_v4();
        let err = ApiError::Query {
            location: loc(),
            source: Box::new(RestError::OperationTimeout {
                operation: "statement poll".to_owned(),
                budget: std::time::Duration::from_secs(1),
                ids: QueryIds {
                    request_id: Some(request_id),
                    query_id: Some("01abc-def-12345".to_owned()),
                },
                location: loc(),
            }),
        };
        let exc = to_driver_exception(err);
        assert_eq!(exc.kind, ErrorKind::Timeout as i32);
        assert_eq!(exc.sql_state.as_deref(), Some("HYT00"));
        assert_eq!(exc.request_id, Some(request_id.to_string()));
        assert_eq!(exc.query_id.as_deref(), Some("01abc-def-12345"));
    }

    #[test]
    fn non_query_error_leaves_ids_unset() {
        // Non-query errors (e.g. invalid argument) never carry query ids.
        let err = crate::apis::database_driver_v1::error::InvalidArgumentSnafu {
            argument: "bad".to_owned(),
        }
        .build();
        let snowflake_ctx = err.snowflake_context();
        assert_eq!(snowflake_ctx.query_id, None);
        assert_eq!(snowflake_ctx.request_id, None);

        let exc = to_driver_exception(err);
        assert_eq!(exc.query_id, None);
        assert_eq!(exc.request_id, None);
    }

    #[test]
    fn file_transfer_io_error_maps_to_io_kind_not_internal_error() {
        // A local file-transfer I/O failure (e.g. permission denied reading
        // the source file) is an environmental/transfer fault, not an
        // internal driver bug — it must map to `ErrorKind::Io`
        // (-> `OperationalError` in Python), matching the reference
        // connector's own classification for the same class of failure.
        use crate::apis::database_driver_v1::error::QueryResponseProcessingError;
        use crate::file_manager::FileManagerError;

        let upload_err = ApiError::QueryResponseProcess {
            location: loc(),
            source: Box::new(QueryResponseProcessingError::FileUpload {
                source: FileManagerError::Io {
                    source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
                    location: loc(),
                },
                location: loc(),
            }),
        };
        assert_eq!(to_driver_exception(upload_err).kind, ErrorKind::Io as i32);

        let download_err = ApiError::QueryResponseProcess {
            location: loc(),
            source: Box::new(QueryResponseProcessingError::FileDownload {
                source: FileManagerError::Io {
                    source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
                    location: loc(),
                },
                location: loc(),
            }),
        };
        assert_eq!(to_driver_exception(download_err).kind, ErrorKind::Io as i32);
    }

    #[test]
    fn connection_lock_maps_to_internal_error() {
        let err = ApiError::ConnectionLock { location: loc() };
        assert_eq!(
            to_driver_exception(err).kind,
            ErrorKind::InternalError as i32
        );
    }

    #[tokio::test]
    async fn workload_identity_attestation_kind_is_invalid_parameter_value() {
        use crate::config::rest_parameters::{WifProvider, WorkloadIdentityConfig};
        use crate::rest::snowflake::workload_identity;

        let config = WorkloadIdentityConfig {
            provider: WifProvider::Oidc,
            entra_resource: None,
            impersonation_path: Vec::new(),
            aws_use_outbound_token: false,
            oidc_token: None,
        };
        let client = reqwest::Client::new();
        let source = workload_identity::create_attestation(&client, &config)
            .await
            .expect_err("OIDC provider with no token must fail");

        let err = ApiError::WorkloadIdentityAttestation {
            location: loc(),
            source: Box::new(source),
        };
        assert_eq!(
            to_driver_exception(err).kind,
            ErrorKind::InvalidParameterValue as i32
        );
    }
}
