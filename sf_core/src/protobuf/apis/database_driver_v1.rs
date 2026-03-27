use crate::apis::database_driver_v1::ApiError;
use crate::apis::database_driver_v1::ColumnMetadata as NativeColumnMetadata;
use crate::apis::database_driver_v1::ConnectionInfo;
use crate::apis::database_driver_v1::DatabaseDriverV1;
use crate::apis::database_driver_v1::ExecuteResult as NativeExecuteResult;
use crate::apis::database_driver_v1::FetchChunkInput;
use crate::apis::database_driver_v1::Handle;
use crate::apis::database_driver_v1::Setting;
use crate::apis::database_driver_v1::error::ConfigError;
use crate::apis::database_driver_v1::error::ConfigurationSnafu;
use crate::apis::database_driver_v1::error::RestError;
use crate::apis::database_driver_v1::{BindingType, DataPtr};
use crate::apis::database_driver_v1::{
    ValidationCode as CoreValidationCode, ValidationIssue as CoreValidationIssue,
    ValidationSeverity as CoreValidationSeverity,
};
use crate::config::config_manager;
use crate::config::path_resolver;
use crate::protobuf::generated::database_driver_v1::*;
use crate::rest::snowflake::error::SfError;
use arrow::ffi::FFI_ArrowSchema;
use arrow::ffi_stream::FFI_ArrowArrayStream;
use error_trace::ErrorTrace;
use snafu::ResultExt;
use std::sync::LazyLock;
use tracing::instrument;

fn setting_to_json(setting: Setting) -> serde_json::Value {
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
fn flat_sections_to_nested_json(
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

// Handle conversions from protobuf types to internal Handle type
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

impl From<result_chunk::Data> for FetchChunkInput {
    fn from(data: result_chunk::Data) -> Self {
        match data {
            result_chunk::Data::Inline(bytes) => FetchChunkInput::Inline(bytes),
            result_chunk::Data::Remote(remote) => FetchChunkInput::Remote(
                crate::chunks::ChunkDownloadData::new(&remote.url, &remote.headers),
            ),
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

impl From<NativeExecuteResult> for ExecuteResult {
    fn from(result: NativeExecuteResult) -> Self {
        let stream_ptr: ArrowArrayStreamPtr = Box::into_raw(result.stream).into();
        ExecuteResult {
            stream: Some(stream_ptr),
            rows_affected: result.rows_affected,
            query_id: result.query_id,
            columns: result
                .columns
                .into_iter()
                .map(ColumnMetadata::from)
                .collect(),
            statement_type_id: result.statement_type_id,
            query: result.query,
            sql_state: result.sql_state,
            stats: result.stats.map(|s| QueryStats {
                num_rows_inserted: s.num_rows_inserted,
                num_rows_updated: s.num_rows_updated,
                num_rows_deleted: s.num_rows_deleted,
                num_dml_duplicates: s.num_dml_duplicates,
            }),
        }
    }
}

impl From<ConnectionInfo> for ConnectionGetInfoResponse {
    fn from(info: ConnectionInfo) -> Self {
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
        }
    }
}

// Convert ApiError to DriverException
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
        ApiError::Login {
            source: RestError::LoginError { message, code, .. },
            ..
        } => DriverError {
            error_type: Some(driver_error::ErrorType::LoginError(LoginError {
                message: message.clone(),
                code: *code,
            })),
        },
        ApiError::Login { source, .. } => DriverError {
            error_type: Some(driver_error::ErrorType::AuthError(AuthenticationError {
                detail: source.to_string(),
            })),
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
        ApiError::Base64Decoding { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::TokenCacheInitialization { source, .. } => DriverError {
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
    }
}

/// Extract the Snowflake server vendor code and SQL state from an ApiError, if available.
///
/// Only populates vendor_code/sql_state for query errors where the Snowflake server
/// code is the user-facing error number.  Login errors use client-side error codes
/// (mapped by the Python layer) so the server code is NOT surfaced here.
///
/// NOTE: currently handles `QueryFailed` and `AsyncQuery` variants only.
/// New query-related error variants should be added here as they are introduced.
fn extract_vendor_info(error: &ApiError) -> (Option<i32>, Option<String>) {
    match error {
        ApiError::Query {
            source: RestError::QueryFailed {
                code, sql_state, ..
            },
            ..
        } => (*code, sql_state.clone()),
        ApiError::Query {
            source:
                RestError::AsyncQuery {
                    source: SfError::SnowflakeBody { code, .. },
                    ..
                },
            ..
        } => (Some(*code), None),
        _ => (None, None),
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
        ApiError::Login {
            source: RestError::LoginError { .. },
            ..
        } => StatusCode::LoginError,
        ApiError::Login { .. } => StatusCode::AuthenticationError,
        ApiError::ConnectionLocking { .. } => StatusCode::InternalError,
        ApiError::StatementLocking { .. } => StatusCode::InternalError,
        ApiError::DatabaseLocking { .. } => StatusCode::InternalError,
        ApiError::QueryResponseProcessing { .. } => StatusCode::InternalError,
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
        ApiError::Base64Decoding { .. } => StatusCode::InternalError,
    };

    let (vendor_code, sql_state) = extract_vendor_info(&error);
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

#[allow(clippy::result_large_err)]
fn required<T>(value: Option<T>, message: &str) -> Result<T, DriverException> {
    value.ok_or_else(|| DriverException {
        message: message.to_string(),
        status_code: StatusCode::InvalidArgument as i32,
        ..Default::default()
    })
}

fn not_implemented(message: &str) -> DriverException {
    DriverException {
        message: message.to_string(),
        status_code: StatusCode::NotImplemented as i32,
        ..Default::default()
    }
}

// Trait for converting ApiError results to protobuf results
trait ToProtobuf<T> {
    #[allow(clippy::result_large_err)]
    fn to_protobuf(self) -> Result<T, DriverException>;
}

impl<T> ToProtobuf<T> for Result<T, ApiError> {
    #[allow(clippy::result_large_err)]
    fn to_protobuf(self) -> Result<T, DriverException> {
        self.map_err(to_driver_exception)
    }
}

fn config_setting_to_setting(cs: ConfigSetting) -> Option<Setting> {
    match cs.value? {
        config_setting::Value::StringValue(s) => Some(Setting::String(s)),
        config_setting::Value::IntValue(i) => Some(Setting::Int(i)),
        config_setting::Value::DoubleValue(d) => Some(Setting::Double(d)),
        config_setting::Value::BytesValue(b) => Some(Setting::Bytes(b)),
        config_setting::Value::BoolValue(b) => Some(Setting::Bool(b)),
    }
}

fn proto_options_to_hashmap(
    options: std::collections::HashMap<String, ConfigSetting>,
) -> std::collections::HashMap<String, Setting> {
    options
        .into_iter()
        .filter_map(|(k, v)| config_setting_to_setting(v).map(|s| (k, s)))
        .collect()
}

fn core_validation_issue_to_proto(issue: CoreValidationIssue) -> ValidationIssue {
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

pub struct DatabaseDriverImpl {
    driver: DatabaseDriverV1,
}

impl Default for DatabaseDriverImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl DatabaseDriverImpl {
    pub fn new() -> Self {
        Self {
            driver: DatabaseDriverV1::new(),
        }
    }
}

impl DatabaseDriver for DatabaseDriverImpl {
    #[instrument(name = "DatabaseDriverV1::database_new", skip(self, _input))]
    async fn database_new(
        &self,
        _input: DatabaseNewRequest,
    ) -> Result<DatabaseNewResponse, DriverException> {
        let handle = self.driver.database_new();
        Ok(DatabaseNewResponse {
            db_handle: Some(DatabaseHandle::from(handle)),
        })
    }

    #[instrument(
        name = "DatabaseDriverV1::database_set_option_string",
        skip(self, input)
    )]
    async fn database_set_option_string(
        &self,
        input: DatabaseSetOptionStringRequest,
    ) -> Result<DatabaseSetOptionStringResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;

        self.driver
            .database_set_option(db_handle.into(), input.key, Setting::String(input.value))
            .await
            .to_protobuf()?;

        Ok(DatabaseSetOptionStringResponse {})
    }

    #[instrument(
        name = "DatabaseDriverV1::database_set_option_bytes",
        skip(self, input)
    )]
    async fn database_set_option_bytes(
        &self,
        input: DatabaseSetOptionBytesRequest,
    ) -> Result<DatabaseSetOptionBytesResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;

        self.driver
            .database_set_option(db_handle.into(), input.key, Setting::Bytes(input.value))
            .await
            .to_protobuf()?;

        Ok(DatabaseSetOptionBytesResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::database_set_option_int", skip(self, input))]
    async fn database_set_option_int(
        &self,
        input: DatabaseSetOptionIntRequest,
    ) -> Result<DatabaseSetOptionIntResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;

        self.driver
            .database_set_option(db_handle.into(), input.key, Setting::Int(input.value))
            .await
            .to_protobuf()?;

        Ok(DatabaseSetOptionIntResponse {})
    }

    #[instrument(
        name = "DatabaseDriverV1::database_set_option_double",
        skip(self, input)
    )]
    async fn database_set_option_double(
        &self,
        input: DatabaseSetOptionDoubleRequest,
    ) -> Result<DatabaseSetOptionDoubleResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;

        self.driver
            .database_set_option(db_handle.into(), input.key, Setting::Double(input.value))
            .await
            .to_protobuf()?;

        Ok(DatabaseSetOptionDoubleResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::database_set_option_bool", skip(self, input))]
    async fn database_set_option_bool(
        &self,
        input: DatabaseSetOptionBoolRequest,
    ) -> Result<DatabaseSetOptionBoolResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;

        self.driver
            .database_set_option(db_handle.into(), input.key, Setting::Bool(input.value))
            .await
            .to_protobuf()?;

        Ok(DatabaseSetOptionBoolResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::database_set_options", skip(self, input))]
    async fn database_set_options(
        &self,
        input: DatabaseSetOptionsRequest,
    ) -> Result<DatabaseSetOptionsResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;
        let options = proto_options_to_hashmap(input.options);

        let warnings = self
            .driver
            .database_set_options(db_handle.into(), options)
            .await
            .to_protobuf()?;

        Ok(DatabaseSetOptionsResponse {
            warnings: warnings
                .into_iter()
                .map(core_validation_issue_to_proto)
                .collect(),
        })
    }

    #[instrument(name = "DatabaseDriverV1::database_init", skip(self, input))]
    async fn database_init(
        &self,
        input: DatabaseInitRequest,
    ) -> Result<DatabaseInitResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;

        self.driver.database_init(db_handle.into()).to_protobuf()?;
        Ok(DatabaseInitResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::database_release", skip(self, input))]
    async fn database_release(
        &self,
        input: DatabaseReleaseRequest,
    ) -> Result<DatabaseReleaseResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;

        self.driver
            .database_release(db_handle.into())
            .to_protobuf()?;
        Ok(DatabaseReleaseResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::database_fetch_chunk", skip(self, input))]
    async fn database_fetch_chunk(
        &self,
        input: DatabaseFetchChunkRequest,
    ) -> Result<DatabaseFetchChunkResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;
        let chunk = required(input.chunk, "Chunk is required")?;
        let chunk_data = required(chunk.data, "Chunk data is required")?;
        let fetch_input: FetchChunkInput = chunk_data.into();

        let stream = self
            .driver
            .database_fetch_chunk(db_handle.into(), fetch_input)
            .await
            .to_protobuf()?;

        let stream_ptr: ArrowArrayStreamPtr = Box::into_raw(stream).into();
        Ok(DatabaseFetchChunkResponse {
            stream: Some(stream_ptr),
        })
    }

    #[instrument(name = "DatabaseDriverV1::connection_new", skip(self, _input))]
    async fn connection_new(
        &self,
        _input: ConnectionNewRequest,
    ) -> Result<ConnectionNewResponse, DriverException> {
        let handle = self.driver.connection_new();
        Ok(ConnectionNewResponse {
            conn_handle: Some(ConnectionHandle::from(handle)),
        })
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_set_option_string",
        skip(self, input)
    )]
    async fn connection_set_option_string(
        &self,
        input: ConnectionSetOptionStringRequest,
    ) -> Result<ConnectionSetOptionStringResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        self.driver
            .connection_set_option(conn_handle.into(), input.key, Setting::String(input.value))
            .await
            .to_protobuf()?;

        Ok(ConnectionSetOptionStringResponse {})
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_set_option_bytes",
        skip(self, input)
    )]
    async fn connection_set_option_bytes(
        &self,
        input: ConnectionSetOptionBytesRequest,
    ) -> Result<ConnectionSetOptionBytesResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        self.driver
            .connection_set_option(conn_handle.into(), input.key, Setting::Bytes(input.value))
            .await
            .to_protobuf()?;

        Ok(ConnectionSetOptionBytesResponse {})
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_set_option_int",
        skip(self, input)
    )]
    async fn connection_set_option_int(
        &self,
        input: ConnectionSetOptionIntRequest,
    ) -> Result<ConnectionSetOptionIntResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        self.driver
            .connection_set_option(conn_handle.into(), input.key, Setting::Int(input.value))
            .await
            .to_protobuf()?;

        Ok(ConnectionSetOptionIntResponse {})
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_set_option_double",
        skip(self, input)
    )]
    async fn connection_set_option_double(
        &self,
        input: ConnectionSetOptionDoubleRequest,
    ) -> Result<ConnectionSetOptionDoubleResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        self.driver
            .connection_set_option(conn_handle.into(), input.key, Setting::Double(input.value))
            .await
            .to_protobuf()?;

        Ok(ConnectionSetOptionDoubleResponse {})
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_set_option_bool",
        skip(self, input)
    )]
    async fn connection_set_option_bool(
        &self,
        input: ConnectionSetOptionBoolRequest,
    ) -> Result<ConnectionSetOptionBoolResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        self.driver
            .connection_set_option(conn_handle.into(), input.key, Setting::Bool(input.value))
            .await
            .to_protobuf()?;

        Ok(ConnectionSetOptionBoolResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_set_options", skip(self, input))]
    async fn connection_set_options(
        &self,
        input: ConnectionSetOptionsRequest,
    ) -> Result<ConnectionSetOptionsResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;
        let options = proto_options_to_hashmap(input.options);

        let warnings = self
            .driver
            .connection_set_options(conn_handle.into(), options)
            .await
            .to_protobuf()?;

        Ok(ConnectionSetOptionsResponse {
            warnings: warnings
                .into_iter()
                .map(core_validation_issue_to_proto)
                .collect(),
        })
    }

    #[instrument(name = "DatabaseDriverV1::connection_init", skip(self, input))]
    async fn connection_init(
        &self,
        input: ConnectionInitRequest,
    ) -> Result<ConnectionInitResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let db_handle = required(input.db_handle, "Database handle is required")?;

        self.driver
            .connection_init(conn_handle.into(), db_handle.into())
            .await
            .to_protobuf()?;
        Ok(ConnectionInitResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_release", skip(self, input))]
    async fn connection_release(
        &self,
        input: ConnectionReleaseRequest,
    ) -> Result<ConnectionReleaseResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        self.driver
            .connection_release(conn_handle.into())
            .to_protobuf()?;
        Ok(ConnectionReleaseResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_info", skip(self, input))]
    async fn connection_get_info(
        &self,
        input: ConnectionGetInfoRequest,
    ) -> Result<ConnectionGetInfoResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let info = self
            .driver
            .connection_get_info(conn_handle.into())
            .await
            .to_protobuf()?;

        Ok(ConnectionGetInfoResponse::from(info))
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_objects", skip(self, _input))]
    async fn connection_get_objects(
        &self,
        _input: ConnectionGetObjectsRequest,
    ) -> Result<ConnectionGetObjectsResponse, DriverException> {
        Err(not_implemented(
            "connection_get_objects is not yet implemented",
        ))
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_get_table_schema",
        skip(self, _input)
    )]
    async fn connection_get_table_schema(
        &self,
        _input: ConnectionGetTableSchemaRequest,
    ) -> Result<ConnectionGetTableSchemaResponse, DriverException> {
        Err(not_implemented(
            "connection_get_table_schema is not yet implemented",
        ))
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_get_table_types",
        skip(self, _input)
    )]
    async fn connection_get_table_types(
        &self,
        _input: ConnectionGetTableTypesRequest,
    ) -> Result<ConnectionGetTableTypesResponse, DriverException> {
        Err(not_implemented(
            "connection_get_table_types is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::connection_commit", skip(self, _input))]
    async fn connection_commit(
        &self,
        _input: ConnectionCommitRequest,
    ) -> Result<ConnectionCommitResponse, DriverException> {
        Err(not_implemented("connection_commit is not yet implemented"))
    }

    #[instrument(name = "DatabaseDriverV1::connection_rollback", skip(self, _input))]
    async fn connection_rollback(
        &self,
        _input: ConnectionRollbackRequest,
    ) -> Result<ConnectionRollbackResponse, DriverException> {
        Err(not_implemented(
            "connection_rollback is not yet implemented",
        ))
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_set_session_parameters",
        skip(self, input)
    )]
    async fn connection_set_session_parameters(
        &self,
        input: ConnectionSetSessionParametersRequest,
    ) -> Result<ConnectionSetSessionParametersResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        self.driver
            .connection_set_session_parameters(conn_handle.into(), input.parameters)
            .await
            .to_protobuf()?;

        Ok(ConnectionSetSessionParametersResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_parameter", skip(self, input))]
    async fn connection_get_parameter(
        &self,
        input: ConnectionGetParameterRequest,
    ) -> Result<ConnectionGetParameterResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let value = self
            .driver
            .connection_get_parameter(conn_handle.into(), input.key)
            .await
            .to_protobuf()?;

        Ok(ConnectionGetParameterResponse { value })
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_validate_options",
        skip(self, input)
    )]
    async fn connection_validate_options(
        &self,
        input: ConnectionValidateOptionsRequest,
    ) -> Result<ConnectionValidateOptionsResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let issues = self
            .driver
            .connection_validate_options(conn_handle.into())
            .await
            .to_protobuf()?;

        Ok(ConnectionValidateOptionsResponse {
            issues: issues
                .into_iter()
                .map(core_validation_issue_to_proto)
                .collect(),
        })
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_get_query_result",
        skip(self, input)
    )]
    async fn connection_get_query_result(
        &self,
        input: ConnectionGetQueryResultRequest,
    ) -> Result<ConnectionGetQueryResultResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let result = self
            .driver
            .connection_get_query_result(conn_handle.into(), input.query_id)
            .await
            .to_protobuf()?;

        Ok(ConnectionGetQueryResultResponse {
            result: Some(result.into()),
        })
    }

    #[instrument(name = "DatabaseDriverV1::statement_new", skip(self, input))]
    async fn statement_new(
        &self,
        input: StatementNewRequest,
    ) -> Result<StatementNewResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let handle = self
            .driver
            .statement_new(conn_handle.into())
            .to_protobuf()?;
        Ok(StatementNewResponse {
            stmt_handle: Some(StatementHandle::from(handle)),
        })
    }

    #[instrument(name = "DatabaseDriverV1::statement_release", skip(self, input))]
    async fn statement_release(
        &self,
        input: StatementReleaseRequest,
    ) -> Result<StatementReleaseResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        self.driver
            .statement_release(stmt_handle.into())
            .to_protobuf()?;
        Ok(StatementReleaseResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_sql_query", skip(self, input))]
    async fn statement_set_sql_query(
        &self,
        input: StatementSetSqlQueryRequest,
    ) -> Result<StatementSetSqlQueryResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        self.driver
            .statement_set_sql_query(stmt_handle.into(), input.query)
            .await
            .to_protobuf()?;
        Ok(StatementSetSqlQueryResponse {})
    }

    #[instrument(
        name = "DatabaseDriverV1::statement_set_substrait_plan",
        skip(self, _input)
    )]
    async fn statement_set_substrait_plan(
        &self,
        _input: StatementSetSubstraitPlanRequest,
    ) -> Result<StatementSetSubstraitPlanResponse, DriverException> {
        // TODO: Implement when corresponding API method is available
        Err(not_implemented(
            "statement_set_substrait_plan is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::statement_prepare", skip(self, input))]
    async fn statement_prepare(
        &self,
        input: StatementPrepareRequest,
    ) -> Result<StatementPrepareResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;
        let result = self
            .driver
            .statement_prepare(stmt_handle.into())
            .await
            .to_protobuf()?;
        let result_ptr: ArrowArrayStreamPtr = Box::into_raw(result.stream).into();
        Ok(StatementPrepareResponse {
            result: Some(PrepareResult {
                stream: Some(result_ptr),
                columns: result.columns.into_iter().map(|cm| cm.into()).collect(),
            }),
        })
    }

    #[instrument(
        name = "DatabaseDriverV1::statement_set_option_string",
        skip(self, input)
    )]
    async fn statement_set_option_string(
        &self,
        input: StatementSetOptionStringRequest,
    ) -> Result<StatementSetOptionStringResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        self.driver
            .statement_set_option(stmt_handle.into(), input.key, Setting::String(input.value))
            .await
            .to_protobuf()?;

        Ok(StatementSetOptionStringResponse {})
    }

    #[instrument(
        name = "DatabaseDriverV1::statement_set_option_bytes",
        skip(self, input)
    )]
    async fn statement_set_option_bytes(
        &self,
        input: StatementSetOptionBytesRequest,
    ) -> Result<StatementSetOptionBytesResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        self.driver
            .statement_set_option(stmt_handle.into(), input.key, Setting::Bytes(input.value))
            .await
            .to_protobuf()?;

        Ok(StatementSetOptionBytesResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_option_int", skip(self, input))]
    async fn statement_set_option_int(
        &self,
        input: StatementSetOptionIntRequest,
    ) -> Result<StatementSetOptionIntResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        self.driver
            .statement_set_option(stmt_handle.into(), input.key, Setting::Int(input.value))
            .await
            .to_protobuf()?;

        Ok(StatementSetOptionIntResponse {})
    }

    #[instrument(
        name = "DatabaseDriverV1::statement_set_option_double",
        skip(self, input)
    )]
    async fn statement_set_option_double(
        &self,
        input: StatementSetOptionDoubleRequest,
    ) -> Result<StatementSetOptionDoubleResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        self.driver
            .statement_set_option(stmt_handle.into(), input.key, Setting::Double(input.value))
            .await
            .to_protobuf()?;

        Ok(StatementSetOptionDoubleResponse {})
    }

    #[instrument(
        name = "DatabaseDriverV1::statement_set_option_bool",
        skip(self, input)
    )]
    async fn statement_set_option_bool(
        &self,
        input: StatementSetOptionBoolRequest,
    ) -> Result<StatementSetOptionBoolResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        self.driver
            .statement_set_option(stmt_handle.into(), input.key, Setting::Bool(input.value))
            .await
            .to_protobuf()?;

        Ok(StatementSetOptionBoolResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_options", skip(self, input))]
    async fn statement_set_options(
        &self,
        input: StatementSetOptionsRequest,
    ) -> Result<StatementSetOptionsResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;
        let options = proto_options_to_hashmap(input.options);

        let warnings = self
            .driver
            .statement_set_options(stmt_handle.into(), options)
            .await
            .to_protobuf()?;

        Ok(StatementSetOptionsResponse {
            warnings: warnings
                .into_iter()
                .map(core_validation_issue_to_proto)
                .collect(),
        })
    }

    #[instrument(
        name = "DatabaseDriverV1::statement_get_parameter_schema",
        skip(self, _input)
    )]
    async fn statement_get_parameter_schema(
        &self,
        _input: StatementGetParameterSchemaRequest,
    ) -> Result<StatementGetParameterSchemaResponse, DriverException> {
        Err(not_implemented(
            "statement_get_parameter_schema is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::statement_execute_query", skip(self, input))]
    async fn statement_execute_query(
        &self,
        input: StatementExecuteQueryRequest,
    ) -> Result<StatementExecuteQueryResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        let bindings_opt = input
            .bindings
            .and_then(|b| b.binding_type)
            .map(BindingType::from);

        let result = self
            .driver
            .statement_execute_query(stmt_handle.into(), bindings_opt)
            .await
            .to_protobuf()?;

        Ok(StatementExecuteQueryResponse {
            result: Some(result.into()),
        })
    }

    #[instrument(
        name = "DatabaseDriverV1::statement_execute_partitions",
        skip(self, _input)
    )]
    async fn statement_execute_partitions(
        &self,
        _input: StatementExecutePartitionsRequest,
    ) -> Result<StatementExecutePartitionsResponse, DriverException> {
        Err(not_implemented(
            "statement_execute_partitions is not yet implemented",
        ))
    }

    #[instrument(
        name = "DatabaseDriverV1::statement_read_partition",
        skip(self, _input)
    )]
    async fn statement_read_partition(
        &self,
        _input: StatementReadPartitionRequest,
    ) -> Result<StatementReadPartitionResponse, DriverException> {
        Err(not_implemented(
            "statement_read_partition is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::statement_result_chunks", skip(self, input))]
    async fn statement_result_chunks(
        &self,
        input: StatementResultChunksRequest,
    ) -> Result<StatementResultChunksResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        let chunk_info = self
            .driver
            .statement_result_chunks(stmt_handle.into())
            .await
            .to_protobuf()?;

        let mut chunks = Vec::new();

        if let Some(base64_data) = chunk_info.initial_chunk_base64 {
            chunks.push(ResultChunk {
                format: ChunkFormat::ArrowIpc as i32,
                data: Some(result_chunk::Data::Inline(base64_data)),
            });
        }

        for c in &chunk_info.chunks {
            chunks.push(ResultChunk {
                format: ChunkFormat::ArrowIpc as i32,
                data: Some(result_chunk::Data::Remote(RemoteChunk {
                    url: c.url.clone(),
                    headers: c.headers.clone(),
                })),
            });
        }

        Ok(StatementResultChunksResponse {
            result: Some(ResultChunksResult { chunks }),
        })
    }

    #[instrument(name = "DatabaseDriverV1::config_load_all_sections", skip(self, input))]
    async fn config_load_all_sections(
        &self,
        input: ConfigLoadAllSectionsRequest,
    ) -> Result<ConfigLoadAllSectionsResponse, DriverException> {
        let all_sections = if input.config_file.is_some() || input.connections_file.is_some() {
            let paths = path_resolver::ConfigPaths {
                config_file: input.config_file.map(std::path::PathBuf::from),
                connections_file: input.connections_file.map(std::path::PathBuf::from),
            };
            config_manager::load_all_config_sections_with_paths(&paths)
        } else {
            config_manager::load_all_config_sections()
        }
        .context(ConfigurationSnafu)
        .to_protobuf()?;

        let nested_json = flat_sections_to_nested_json(all_sections);
        let config_json = serde_json::to_string(&nested_json).map_err(|e| DriverException {
            message: format!("Failed to serialize config to JSON: {e}"),
            status_code: StatusCode::InternalError as i32,
            ..Default::default()
        })?;

        Ok(ConfigLoadAllSectionsResponse { config_json })
    }

    #[instrument(name = "DatabaseDriverV1::config_get_paths", skip(self, _input))]
    async fn config_get_paths(
        &self,
        _input: ConfigGetPathsRequest,
    ) -> Result<ConfigGetPathsResponse, DriverException> {
        let paths = path_resolver::get_config_paths()
            .context(ConfigurationSnafu)
            .to_protobuf()?;

        let config_file = paths.config_file.ok_or_else(|| DriverException {
            message: "Configuration path for config file is unavailable".to_string(),
            status_code: StatusCode::InternalError as i32,
            ..Default::default()
        })?;

        let connections_file = paths.connections_file.ok_or_else(|| DriverException {
            message: "Configuration path for connections file is unavailable".to_string(),
            status_code: StatusCode::InternalError as i32,
            ..Default::default()
        })?;

        Ok(ConfigGetPathsResponse {
            config_file: config_file.to_string_lossy().into_owned(),
            connections_file: connections_file.to_string_lossy().into_owned(),
        })
    }
}

impl DatabaseDriverServer for DatabaseDriverImpl {}

impl ErrorTrace for DriverException {
    fn error_trace(&self) -> Vec<error_trace::ErrorTraceEntry> {
        self.error_trace
            .iter()
            .map(|entry| error_trace::ErrorTraceEntry {
                location: error_trace::Location::new(entry.file.clone(), entry.line, entry.column),
                message: entry.message.clone(),
            })
            .collect()
    }
}

pub type DatabaseDriverClient =
    crate::protobuf::generated::database_driver_v1::DatabaseDriverClient<
        crate::protobuf::apis::RustTransport,
    >;

pub fn database_driver_client() -> DatabaseDriverClient {
    DatabaseDriverClient::new(crate::protobuf::apis::RustTransport::new())
}

// Synchronous convenience wrappers used by Rust test helpers and small
// in-process smoke tests. Production callers should prefer the async
// client methods directly.
static BLOCKING_CLIENT_RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create blocking protobuf client runtime")
});

/// Blocking adapters for synchronous Rust test/support code that drives
/// the in-process protobuf client. Production async paths should call
/// the generated async client methods directly.
#[allow(clippy::result_large_err)]
pub trait DatabaseDriverClientBlockingExt {
    fn database_new_blocking(
        &self,
        input: DatabaseNewRequest,
    ) -> Result<DatabaseNewResponse, proto_utils::ProtoError<DriverException>>;
    fn database_init_blocking(
        &self,
        input: DatabaseInitRequest,
    ) -> Result<DatabaseInitResponse, proto_utils::ProtoError<DriverException>>;
    fn connection_new_blocking(
        &self,
        input: ConnectionNewRequest,
    ) -> Result<ConnectionNewResponse, proto_utils::ProtoError<DriverException>>;
    fn connection_init_blocking(
        &self,
        input: ConnectionInitRequest,
    ) -> Result<ConnectionInitResponse, proto_utils::ProtoError<DriverException>>;
    fn statement_new_blocking(
        &self,
        input: StatementNewRequest,
    ) -> Result<StatementNewResponse, proto_utils::ProtoError<DriverException>>;
    fn statement_execute_query_blocking(
        &self,
        input: StatementExecuteQueryRequest,
    ) -> Result<StatementExecuteQueryResponse, proto_utils::ProtoError<DriverException>>;
    fn statement_set_sql_query_blocking(
        &self,
        input: StatementSetSqlQueryRequest,
    ) -> Result<StatementSetSqlQueryResponse, proto_utils::ProtoError<DriverException>>;
    fn statement_release_blocking(
        &self,
        input: StatementReleaseRequest,
    ) -> Result<StatementReleaseResponse, proto_utils::ProtoError<DriverException>>;
    fn statement_result_chunks_blocking(
        &self,
        input: StatementResultChunksRequest,
    ) -> Result<StatementResultChunksResponse, proto_utils::ProtoError<DriverException>>;
    fn database_fetch_chunk_blocking(
        &self,
        input: DatabaseFetchChunkRequest,
    ) -> Result<DatabaseFetchChunkResponse, proto_utils::ProtoError<DriverException>>;
    fn connection_set_option_string_blocking(
        &self,
        input: ConnectionSetOptionStringRequest,
    ) -> Result<ConnectionSetOptionStringResponse, proto_utils::ProtoError<DriverException>>;
    fn connection_set_option_int_blocking(
        &self,
        input: ConnectionSetOptionIntRequest,
    ) -> Result<ConnectionSetOptionIntResponse, proto_utils::ProtoError<DriverException>>;
    fn connection_set_option_bytes_blocking(
        &self,
        input: ConnectionSetOptionBytesRequest,
    ) -> Result<ConnectionSetOptionBytesResponse, proto_utils::ProtoError<DriverException>>;
    fn statement_set_option_bool_blocking(
        &self,
        input: StatementSetOptionBoolRequest,
    ) -> Result<StatementSetOptionBoolResponse, proto_utils::ProtoError<DriverException>>;
    fn connection_release_blocking(
        &self,
        input: ConnectionReleaseRequest,
    ) -> Result<ConnectionReleaseResponse, proto_utils::ProtoError<DriverException>>;
    fn database_release_blocking(
        &self,
        input: DatabaseReleaseRequest,
    ) -> Result<DatabaseReleaseResponse, proto_utils::ProtoError<DriverException>>;
}

#[allow(clippy::result_large_err)]
impl DatabaseDriverClientBlockingExt for DatabaseDriverClient {
    fn database_new_blocking(
        &self,
        input: DatabaseNewRequest,
    ) -> Result<DatabaseNewResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.database_new(input))
    }

    fn database_init_blocking(
        &self,
        input: DatabaseInitRequest,
    ) -> Result<DatabaseInitResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.database_init(input))
    }

    fn connection_new_blocking(
        &self,
        input: ConnectionNewRequest,
    ) -> Result<ConnectionNewResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.connection_new(input))
    }

    fn connection_init_blocking(
        &self,
        input: ConnectionInitRequest,
    ) -> Result<ConnectionInitResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.connection_init(input))
    }

    fn statement_new_blocking(
        &self,
        input: StatementNewRequest,
    ) -> Result<StatementNewResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.statement_new(input))
    }

    fn statement_execute_query_blocking(
        &self,
        input: StatementExecuteQueryRequest,
    ) -> Result<StatementExecuteQueryResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.statement_execute_query(input))
    }

    fn statement_set_sql_query_blocking(
        &self,
        input: StatementSetSqlQueryRequest,
    ) -> Result<StatementSetSqlQueryResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.statement_set_sql_query(input))
    }

    fn statement_release_blocking(
        &self,
        input: StatementReleaseRequest,
    ) -> Result<StatementReleaseResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.statement_release(input))
    }

    fn statement_result_chunks_blocking(
        &self,
        input: StatementResultChunksRequest,
    ) -> Result<StatementResultChunksResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.statement_result_chunks(input))
    }

    fn database_fetch_chunk_blocking(
        &self,
        input: DatabaseFetchChunkRequest,
    ) -> Result<DatabaseFetchChunkResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.database_fetch_chunk(input))
    }

    fn connection_set_option_string_blocking(
        &self,
        input: ConnectionSetOptionStringRequest,
    ) -> Result<ConnectionSetOptionStringResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.connection_set_option_string(input))
    }

    fn connection_set_option_int_blocking(
        &self,
        input: ConnectionSetOptionIntRequest,
    ) -> Result<ConnectionSetOptionIntResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.connection_set_option_int(input))
    }

    fn connection_set_option_bytes_blocking(
        &self,
        input: ConnectionSetOptionBytesRequest,
    ) -> Result<ConnectionSetOptionBytesResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.connection_set_option_bytes(input))
    }

    fn statement_set_option_bool_blocking(
        &self,
        input: StatementSetOptionBoolRequest,
    ) -> Result<StatementSetOptionBoolResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.statement_set_option_bool(input))
    }

    fn connection_release_blocking(
        &self,
        input: ConnectionReleaseRequest,
    ) -> Result<ConnectionReleaseResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.connection_release(input))
    }

    fn database_release_blocking(
        &self,
        input: DatabaseReleaseRequest,
    ) -> Result<DatabaseReleaseResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.database_release(input))
    }
}
