use crate::apis::database_driver_v1::ColumnMetadata as NativeColumnMetadata;
use crate::apis::database_driver_v1::ConnectionInfo;
use crate::apis::database_driver_v1::ExecuteQueryResult as NativeExecuteQueryResult;
use crate::apis::database_driver_v1::FetchChunkInput;
use crate::apis::database_driver_v1::Handle;
use crate::apis::database_driver_v1::ResolvedResultSet as NativeResolvedResultSet;
use crate::apis::database_driver_v1::ResultSetDescriptor as NativeResultSetDescriptor;
use crate::apis::database_driver_v1::Setting;
use crate::apis::database_driver_v1::error::{ConfigError, InvalidColumnMetadataSnafu, RestError};
use crate::apis::database_driver_v1::{ApiError, BindingType, DataPtr};
use crate::apis::database_driver_v1::{
    ValidationCode as CoreValidationCode, ValidationIssue as CoreValidationIssue,
    ValidationSeverity as CoreValidationSeverity,
};
use crate::chunks::{
    ArrowIpcEncodingSnafu, ChunkError, ChunkFormatKind, ChunkReadingSnafu,
    convert_string_rowset_to_arrow_reader,
};
use crate::protobuf::generated::database_driver_v1::*;
use crate::query_types::RowType;
use crate::rest::snowflake::error::SfError;
use crate::rest::snowflake::sql_state::sql_state_from_code;
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
impl<'a> From<BinaryDataPtr> for DataPtr<'a> {
    fn from(proto_ptr: BinaryDataPtr) -> Self {
        let ptr_bytes: [u8; 8] = proto_ptr
            .value
            .as_slice()
            .try_into()
            .expect("Pointer must be 8 bytes");
        let ptr_value = usize::from_le_bytes(ptr_bytes);
        let ptr = ptr_value as *const u8;
        DataPtr::new(ptr, proto_ptr.length)
    }
}

// Convert protobuf QueryBindings variant to internal BindingType.
impl<'a> From<query_bindings::BindingType> for BindingType<'a> {
    fn from(proto: query_bindings::BindingType) -> Self {
        match proto {
            query_bindings::BindingType::Json(ptr) => BindingType::Json(ptr.into()),
            query_bindings::BindingType::Csv(ptr) => BindingType::Csv(ptr.into()),
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
            .context(ArrowIpcEncodingSnafu)?;
        for batch in reader {
            // Iterator error = JSON rowset read/decode failure, not IPC encoding.
            let batch = batch.context(ChunkReadingSnafu)?;
            writer.write(&batch).context(ArrowIpcEncodingSnafu)?;
        }
        writer.finish().context(ArrowIpcEncodingSnafu)?;
    }
    Ok(BASE64.encode(&buf))
}

// ---------------------------------------------------------------------------
// Result / chunk / column conversions
// ---------------------------------------------------------------------------

impl From<result_chunk::Data> for FetchChunkInput {
    fn from(data: result_chunk::Data) -> Self {
        match data {
            result_chunk::Data::Inline(bytes) => FetchChunkInput::Inline(bytes),
            result_chunk::Data::Remote(remote) => {
                FetchChunkInput::Remote(crate::chunks::ChunkDownloadData {
                    url: remote.url,
                    row_count: 0,
                    uncompressed_size: 0,
                    compressed_size: 0,
                    headers: remote.headers,
                })
            }
        }
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
        ext_type_name: None,
        _fields: None,
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
            statement_type_id: d.statement_type_id,
            sql_state: d.sql_state,
            stats: native_stats_to_proto(&d.stats),
        }
    }
}

impl From<NativeExecuteQueryResult> for ExecuteQueryResponse {
    fn from(result: NativeExecuteQueryResult) -> Self {
        match result {
            NativeExecuteQueryResult::Single(descriptor) => ExecuteQueryResponse {
                result: Some(execute_query_response::Result::Single(descriptor.into())),
            },
            NativeExecuteQueryResult::Multi {
                parent,
                query_ids,
                statement_type_ids,
            } => ExecuteQueryResponse {
                result: Some(execute_query_response::Result::Multi(
                    MultiStatementResult {
                        parent: Some(parent.into()),
                        query_ids,
                        statement_type_ids,
                    },
                )),
            },
        }
    }
}

impl From<NativeResolvedResultSet> for ResultSetResponse {
    fn from(result: NativeResolvedResultSet) -> Self {
        let stream_ptr: ArrowArrayStreamPtr = Box::into_raw(result.stream).into();
        ResultSetResponse {
            result_descriptor: Some(result.descriptor.into()),
            stream: Some(stream_ptr),
        }
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
        }
    }
}

impl From<crate::rest::snowflake::QueryStatusResult> for ConnectionGetQueryStatusResponse {
    fn from(result: crate::rest::snowflake::QueryStatusResult) -> Self {
        ConnectionGetQueryStatusResponse {
            status_name: result.status_name,
            error_code: result.error_code,
            error_message: result.error_message,
        }
    }
}

// ---------------------------------------------------------------------------
// Setting / config conversions
// ---------------------------------------------------------------------------

pub(super) fn setting_to_json(setting: Setting) -> serde_json::Value {
    match setting {
        Setting::String(s) => serde_json::Value::String(s),
        Setting::Int(i) => serde_json::json!(i),
        Setting::Double(d) => serde_json::json!(d),
        Setting::Bool(b) => serde_json::Value::Bool(b),
        Setting::Bytes(b) => serde_json::Value::String(String::from_utf8_lossy(&b).into_owned()),
    }
}

/// Convert the flat dot-separated section map from `load_all_config_sections`
/// into a nested JSON object that Python can consume directly via `json.loads`.
pub(super) fn flat_sections_to_nested_json(
    flat: std::collections::HashMap<String, std::collections::HashMap<String, Setting>>,
) -> serde_json::Value {
    let mut root = serde_json::Map::new();

    for (section_name, settings) in flat {
        let settings_map: serde_json::Map<String, serde_json::Value> = settings
            .into_iter()
            .map(|(k, v)| (k, setting_to_json(v)))
            .collect();

        if section_name.is_empty() {
            for (k, v) in settings_map {
                root.insert(k, v);
            }
            continue;
        }

        let parts: Vec<&str> = section_name.split('.').collect();
        let mut current = &mut root;
        for part in &parts[..parts.len() - 1] {
            current = current
                .entry(part.to_string())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
                .as_object_mut()
                .expect("intermediate path segment must be an object");
        }

        let last = parts.last().expect("section_name is non-empty");
        if let Some(existing) = current.get_mut(*last) {
            if let Some(obj) = existing.as_object_mut() {
                for (k, v) in settings_map {
                    obj.insert(k, v);
                }
            }
        } else {
            current.insert(last.to_string(), serde_json::Value::Object(settings_map));
        }
    }

    serde_json::Value::Object(root)
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

pub(super) fn core_validation_issue_to_proto(issue: CoreValidationIssue) -> ValidationIssue {
    let severity = match issue.severity {
        CoreValidationSeverity::Error => ValidationSeverity::Error as i32,
        CoreValidationSeverity::Warning => ValidationSeverity::Warning as i32,
    };
    let code = match issue.code {
        CoreValidationCode::Unspecified => ValidationCode::Unspecified as i32,
        CoreValidationCode::MissingRequired => ValidationCode::MissingRequired as i32,
        CoreValidationCode::InvalidType => ValidationCode::InvalidType as i32,
        CoreValidationCode::InvalidValue => ValidationCode::InvalidValue as i32,
        CoreValidationCode::UnknownParameter => ValidationCode::UnknownParameter as i32,
        CoreValidationCode::DeprecatedParameter => ValidationCode::DeprecatedParameter as i32,
        CoreValidationCode::ConflictingParameters => ValidationCode::ConflictingParameters as i32,
    };
    ValidationIssue {
        severity,
        parameter: issue.parameter,
        message: issue.message,
        code,
    }
}

// ---------------------------------------------------------------------------
// Error conversion (ApiError → DriverError / DriverException)
// ---------------------------------------------------------------------------

fn to_driver_error(error: &ApiError) -> DriverError {
    match error {
        ApiError::GenericError { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::GenericError(GenericError {})),
        },
        ApiError::RuntimeCreation { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::Configuration {
            source:
                ConfigError::InvalidParameterValue {
                    parameter,
                    value,
                    explanation,
                    ..
                },
            ..
        } => DriverError {
            error_type: Some(driver_error::ErrorType::InvalidParameterValue(
                InvalidParameterValue {
                    parameter: parameter.clone(),
                    value: value.clone(),
                    explanation: Some(explanation.clone()),
                },
            )),
        },
        ApiError::Configuration {
            source: ConfigError::MissingParameter { parameter, .. },
            ..
        } => DriverError {
            error_type: Some(driver_error::ErrorType::MissingParameter(
                MissingParameter {
                    parameter: parameter.clone(),
                },
            )),
        },
        ApiError::Configuration {
            source: ConfigError::ConflictingParameters { explanation, .. },
            ..
        } => DriverError {
            error_type: Some(driver_error::ErrorType::InvalidParameterValue(
                InvalidParameterValue {
                    parameter: "private_key/private_key_file".to_string(),
                    value: "(both set)".to_string(),
                    explanation: Some(explanation.clone()),
                },
            )),
        },
        ApiError::Configuration {
            source: ConfigError::ValidationFailed { issues, .. },
            ..
        } => {
            let summary = issues
                .iter()
                .map(|issue| format!("{}: {}", issue.parameter, issue.message))
                .collect::<Vec<_>>()
                .join("; ");
            let first_param = issues
                .first()
                .map(|issue| issue.parameter.clone())
                .unwrap_or_default();
            let first_missing_param = issues
                .iter()
                .find(|issue| issue.code == CoreValidationCode::MissingRequired)
                .map(|issue| issue.parameter.clone())
                .unwrap_or_else(|| first_param.clone());
            if issues
                .iter()
                .any(|i| i.code == CoreValidationCode::MissingRequired)
            {
                DriverError {
                    error_type: Some(driver_error::ErrorType::MissingParameter(
                        MissingParameter {
                            parameter: first_missing_param,
                        },
                    )),
                }
            } else {
                DriverError {
                    error_type: Some(driver_error::ErrorType::InvalidParameterValue(
                        InvalidParameterValue {
                            parameter: first_param,
                            value: String::new(),
                            explanation: Some(summary),
                        },
                    )),
                }
            }
        }
        ApiError::InvalidArgument { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::Login { source, .. } => match source.as_ref() {
            RestError::LoginError { message, code, .. } => DriverError {
                error_type: Some(driver_error::ErrorType::LoginError(LoginError {
                    message: message.clone(),
                    code: *code,
                })),
            },
            _ => DriverError {
                error_type: Some(driver_error::ErrorType::AuthError(AuthenticationError {
                    detail: source.to_string(),
                })),
            },
        },
        ApiError::ConnectionLocking { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::StatementLocking { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::DatabaseLocking { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::QueryResponseProcessing { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::ConnectionNotInitialized { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::TlsClientCreation { source, .. } => DriverError {
            error_type: Some(driver_error::ErrorType::AuthError(AuthenticationError {
                detail: source.to_string(),
            })),
        },
        ApiError::SessionRefresh { source, .. } => DriverError {
            error_type: Some(driver_error::ErrorType::AuthError(AuthenticationError {
                detail: source.to_string(),
            })),
        },
        ApiError::Statement { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::Query { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::MasterTokenExpired { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::AuthError(AuthenticationError {
                detail: "Master token expired, full re-authentication required".to_string(),
            })),
        },
        ApiError::InvalidRefreshState { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::ChunkFetch { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::ArrowParsing { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::JsonChunkDecoding { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::InlineJsonEncoding { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::InvalidColumnMetadata { column, .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InvalidParameterValue(
                InvalidParameterValue {
                    parameter: format!("column: {column}"),
                    value: String::new(),
                    explanation: Some("Failed to parse column metadata".to_string()),
                },
            )),
        },
        ApiError::Base64Decoding { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::UnsupportedQueryResultFormat { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::TokenCacheInitialization { source, .. } => DriverError {
            error_type: Some(driver_error::ErrorType::AuthError(AuthenticationError {
                detail: source.to_string(),
            })),
        },
        ApiError::HttpRequest { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::GenericError(GenericError {})),
        },
        ApiError::TokenRequest { source, .. } => DriverError {
            error_type: Some(driver_error::ErrorType::AuthError(AuthenticationError {
                detail: source.to_string(),
            })),
        },
        ApiError::Configuration {
            source: ConfigError::ConfigFileRead { .. },
            ..
        }
        | ApiError::Configuration {
            source: ConfigError::TomlParse { .. },
            ..
        }
        | ApiError::Configuration {
            source: ConfigError::InsecurePermissions { .. },
            ..
        }
        | ApiError::Configuration {
            source: ConfigError::ConfigDirNotFound { .. },
            ..
        } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::Configuration {
            source: ConfigError::ConnectionNotFound { name, .. },
            ..
        } => DriverError {
            error_type: Some(driver_error::ErrorType::MissingParameter(
                MissingParameter {
                    parameter: format!("connection: {}", name),
                },
            )),
        },
        ApiError::ConnectionClosed { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::GenericError(GenericError {})),
        },
        ApiError::LogoutFailed { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::GenericError(GenericError {})),
        },
    }
}

/// Extract the Snowflake server vendor code and SQL state from an ApiError, if available.
///
/// Only populates vendor_code/sql_state for query errors where the Snowflake server
/// code is the user-facing error number.  Login errors use client-side error codes
/// (mapped by the Python layer) so the server code is NOT surfaced here.
///
/// SQLSTATE resolution order (first hit wins):
///   1. The `sqlState` the server included in its response (verbatim).
///   2. `sql_state_from_code` lookup against the numeric Snowflake error
///      code, which covers paths (async-poll, query-monitoring) that drop
///      `sqlState` on the wire but keep the error code.
///
/// We deliberately do NOT inspect the human-readable message text:
/// classification belongs to the server, and substring matching on
/// English error messages is locale-fragile and false-positive-prone.
///
/// Centralising this here means downstream consumers (ODBC, JDBC, ADBC) can
/// rely on `sql_state` as the single source of truth for error
/// classification.
///
/// NOTE: currently handles `QueryFailed` and `AsyncQuery` variants only.
/// New query-related error variants should be added here as they are
/// introduced.
fn extract_vendor_info(error: &ApiError) -> (Option<i32>, Option<String>) {
    let (code, sql_state) = match error {
        ApiError::Query { source, .. } => match source.as_ref() {
            RestError::QueryFailed {
                code, sql_state, ..
            } => (*code, sql_state.clone()),
            RestError::AsyncQuery {
                source: SfError::SnowflakeBody { code, .. },
                ..
            } => (Some(*code), None),
            _ => (None, None),
        },
        _ => (None, None),
    };

    let sql_state = sql_state.or_else(|| code.and_then(sql_state_from_code).map(|s| s.to_owned()));

    (code, sql_state)
}

fn extract_query_id(error: &ApiError) -> Option<String> {
    match error {
        ApiError::Query { source, .. } => match source.as_ref() {
            RestError::QueryFailed { query_id, .. } => query_id.clone(),
            _ => None,
        },
        _ => None,
    }
}

fn to_driver_exception(error: ApiError) -> DriverException {
    let status_code = match &error {
        ApiError::GenericError { .. } => StatusCode::GenericError,
        ApiError::RuntimeCreation { .. } => StatusCode::InternalError,
        ApiError::Configuration {
            source: ConfigError::InvalidParameterValue { .. },
            ..
        } => StatusCode::InvalidParameterValue,
        ApiError::Configuration {
            source: ConfigError::MissingParameter { .. },
            ..
        } => StatusCode::MissingParameter,
        ApiError::Configuration {
            source: ConfigError::ConflictingParameters { .. },
            ..
        } => StatusCode::InvalidParameterValue,
        ApiError::Configuration {
            source: ConfigError::ConfigFileRead { .. },
            ..
        } => StatusCode::InternalError,
        ApiError::Configuration {
            source: ConfigError::TomlParse { .. },
            ..
        } => StatusCode::InternalError,
        ApiError::Configuration {
            source: ConfigError::InsecurePermissions { .. },
            ..
        } => StatusCode::InternalError,
        ApiError::Configuration {
            source: ConfigError::ConfigDirNotFound { .. },
            ..
        } => StatusCode::InternalError,
        ApiError::Configuration {
            source: ConfigError::ConnectionNotFound { .. },
            ..
        } => StatusCode::MissingParameter,
        ApiError::Configuration {
            source: ConfigError::ValidationFailed { issues, .. },
            ..
        } if issues
            .iter()
            .any(|i| i.code == CoreValidationCode::MissingRequired) =>
        {
            StatusCode::MissingParameter
        }
        ApiError::Configuration {
            source: ConfigError::ValidationFailed { .. },
            ..
        } => StatusCode::InvalidParameterValue,
        ApiError::InvalidArgument { .. } => StatusCode::InvalidArgument,
        ApiError::Login { source, .. } => match source.as_ref() {
            RestError::LoginError { .. } => StatusCode::LoginError,
            _ => StatusCode::AuthenticationError,
        },
        ApiError::ConnectionLocking { .. } => StatusCode::InternalError,
        ApiError::StatementLocking { .. } => StatusCode::InternalError,
        ApiError::DatabaseLocking { .. } => StatusCode::InternalError,
        ApiError::QueryResponseProcessing {
            source: boxed_error,
            ..
        } => {
            use crate::apis::database_driver_v1::error::QueryResponseProcessingError;
            use crate::compression_types::CompressionTypeError;
            use crate::file_manager::FileManagerError;

            match boxed_error.as_ref() {
                QueryResponseProcessingError::FileUpload { source, .. }
                | QueryResponseProcessingError::FileDownload { source, .. } => match source {
                    FileManagerError::NoFilesMatched { .. } => StatusCode::LocalFileNotFound,
                    FileManagerError::CompressionType {
                        source: CompressionTypeError::UnsupportedCompressionType { .. },
                        ..
                    } => StatusCode::UnsupportedCompression,
                    _ => StatusCode::InternalError,
                },
                QueryResponseProcessingError::RemoteFileNotFound { .. } => {
                    StatusCode::RemoteFileNotFound
                }
                _ => StatusCode::InternalError,
            }
        }
        ApiError::ConnectionNotInitialized { .. } => StatusCode::InternalError,
        ApiError::TlsClientCreation { .. } => StatusCode::AuthenticationError,
        ApiError::SessionRefresh { .. } => StatusCode::AuthenticationError,
        ApiError::Statement { .. } => StatusCode::InternalError,
        ApiError::Query { .. } => StatusCode::InternalError,
        ApiError::MasterTokenExpired { .. } => StatusCode::AuthenticationError,
        ApiError::InvalidRefreshState { .. } => StatusCode::InternalError,
        ApiError::TokenCacheInitialization { .. } => StatusCode::AuthenticationError,
        ApiError::ChunkFetch { .. } => StatusCode::InternalError,
        ApiError::ArrowParsing { .. } => StatusCode::InternalError,
        ApiError::JsonChunkDecoding { .. } => StatusCode::InternalError,
        ApiError::InlineJsonEncoding { .. } => StatusCode::InternalError,
        ApiError::InvalidColumnMetadata { .. } => StatusCode::InvalidArgument,
        ApiError::Base64Decoding { .. } => StatusCode::InternalError,
        ApiError::UnsupportedQueryResultFormat { .. } => StatusCode::InternalError,
        ApiError::HttpRequest { .. } => StatusCode::GenericError,
        ApiError::TokenRequest { .. } => StatusCode::AuthenticationError,
        ApiError::ConnectionClosed { .. } => StatusCode::InvalidArgument,
        ApiError::LogoutFailed { .. } => StatusCode::InternalError,
    };

    let (vendor_code, sql_state) = extract_vendor_info(&error);
    let query_id = extract_query_id(&error);
    let message = error.to_string();
    let root_cause = extract_root_cause(&error);
    let driver_error = to_driver_error(&error);

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
        message,
        status_code: status_code as i32,
        error: Some(driver_error),
        error_trace,
        vendor_code,
        sql_state,
        root_cause,
        query_id,
    }
}

/// Walk the `source()` chain to the deepest error and return its message.
/// Returns `None` when the error has no source (i.e. the message itself is
/// already the root cause).
fn extract_root_cause(error: &dyn std::error::Error) -> Option<String> {
    let mut deepest: Option<&dyn std::error::Error> = None;
    let mut current = error.source();
    while let Some(cause) = current {
        deepest = Some(cause);
        current = cause.source();
    }
    deepest.map(|e| e.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::snowflake::error::SfError;
    use snafu::Location;

    fn loc() -> Location {
        Location::new("test", 0, 0)
    }

    fn query_failed(code: Option<i32>, sql_state: Option<&str>) -> ApiError {
        ApiError::Query {
            location: loc(),
            source: Box::new(RestError::QueryFailed {
                message: "test".to_owned(),
                code,
                sql_state: sql_state.map(|s| s.to_owned()),
                query_id: None,
                location: loc(),
            }),
        }
    }

    fn async_query(code: i32) -> ApiError {
        ApiError::Query {
            location: loc(),
            source: Box::new(RestError::AsyncQuery {
                location: loc(),
                source: SfError::SnowflakeBody {
                    code,
                    message: "test".to_owned(),
                    location: loc(),
                },
            }),
        }
    }

    #[test]
    fn query_failed_passes_through_server_sql_state() {
        let err = query_failed(Some(1003), Some("42000"));
        assert_eq!(
            extract_vendor_info(&err),
            (Some(1003), Some("42000".to_owned()))
        );
    }

    #[test]
    fn server_sql_state_wins_over_lookup_table() {
        // If the server provides "22000", trust it even though our table maps
        // 100038 → "22003". The wire value is authoritative.
        let err = query_failed(Some(100038), Some("22000"));
        assert_eq!(
            extract_vendor_info(&err),
            (Some(100038), Some("22000".to_owned()))
        );
    }

    #[test]
    fn query_failed_falls_back_to_lookup_when_sql_state_missing() {
        let err = query_failed(Some(100038), None);
        assert_eq!(
            extract_vendor_info(&err),
            (Some(100038), Some("22003".to_owned()))
        );

        let err = query_failed(Some(100078), None);
        assert_eq!(
            extract_vendor_info(&err),
            (Some(100078), Some("22001".to_owned()))
        );

        let err = query_failed(Some(1003), None);
        assert_eq!(
            extract_vendor_info(&err),
            (Some(1003), Some("42000".to_owned()))
        );
    }

    #[test]
    fn unknown_code_with_no_sql_state_stays_none() {
        let err = query_failed(Some(999_999), None);
        assert_eq!(extract_vendor_info(&err), (Some(999_999), None));
    }

    #[test]
    fn missing_code_and_sql_state_stays_none() {
        let err = query_failed(None, None);
        assert_eq!(extract_vendor_info(&err), (None, None));
    }

    #[test]
    fn async_query_path_now_supplies_sql_state() {
        // Previously this path returned (Some(code), None) unconditionally —
        // the regression this fix is targeting.
        let err = async_query(1003);
        assert_eq!(
            extract_vendor_info(&err),
            (Some(1003), Some("42000".to_owned()))
        );

        let err = async_query(100038);
        assert_eq!(
            extract_vendor_info(&err),
            (Some(100038), Some("22003".to_owned()))
        );
    }

    #[test]
    fn async_query_unknown_code_keeps_sql_state_none() {
        let err = async_query(424_242);
        assert_eq!(extract_vendor_info(&err), (Some(424_242), None));
    }
}
