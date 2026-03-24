use std::{
    collections::HashSet,
    str::Utf8Error,
    string::{FromUtf8Error, FromUtf16Error},
    sync::LazyLock,
};

use crate::{
    api::{InfoType, SqlState, diagnostic::DiagnosticRecord},
    conversion::error::JsonBindingError,
    conversion::{ConversionError, error::WriteOdbcError},
};
use arrow::error::ArrowError;
use odbc_sys as sql;
use proto_utils::ProtoError;
use sf_core::protobuf::generated::database_driver_v1::{
    ErrorTraceEntry, GenericError, InvalidParameterValue as ProtoInvalidParameterValue,
    MissingParameter as ProtoMissingParameter, driver_error::ErrorType,
};

use error_trace::{ErrorTrace, format_error_trace};
use sf_core::protobuf::generated::database_driver_v1::DriverException as ProtoDriverException;
use snafu::{Location, Snafu, location};

#[derive(Snafu, Debug, ErrorTrace)]
#[snafu(visibility(pub))]
pub enum OdbcError {
    #[snafu(display("Connection is disconnected"))]
    Disconnected {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid handle"))]
    InvalidHandle {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid descriptor kind: {kind}"))]
    InvalidDescriptorKind {
        kind: u16,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid use of null pointer"))]
    NullPointer {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid string or buffer length: {length}"))]
    InvalidBufferLength {
        length: i64,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid application buffer type"))]
    InvalidApplicationBufferType {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid parameter type: {value}"))]
    InvalidParameterType {
        value: i16,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid SQL data type: {value}"))]
    InvalidSqlDataType {
        value: i16,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid record number: {number}"))]
    InvalidRecordNumber {
        number: sql::SmallInt,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid descriptor index: {number}"))]
    InvalidDescriptorIndex {
        number: sql::SmallInt,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid precision or scale value: {reason}"))]
    InvalidPrecisionOrScale {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid diagnostic identifier: {identifier}"))]
    InvalidDiagnosticIdentifier {
        identifier: sql::SmallInt,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unknown attribute: {attribute}"))]
    UnknownAttribute {
        attribute: i32,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported attribute: {attribute}"))]
    UnsupportedAttribute {
        attribute: i32,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported info type: {:?}", info_type))]
    UnsupportedInfoType {
        info_type: InfoType,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unknown info type: {info_type}"))]
    UnknownInfoType {
        info_type: u16,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Attribute cannot be set now: {attribute}"))]
    AttributeCannotBeSetNow {
        attribute: i32,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Parameter number cannot be 0"))]
    InvalidParameterNumber {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Statement not executed"))]
    StatementNotExecuted {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid cursor state: no result set associated with the statement"))]
    InvalidCursorState {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Statement is in error state"))]
    StatementErrorState {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Data not fetched yet"))]
    DataNotFetched {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("No more data available"))]
    NoMoreData {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid cursor position"))]
    InvalidCursorPosition {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "SQLFetch cannot be called after SQLExtendedFetch without closing the cursor"
    ))]
    MixedCursorFunctions {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Optional feature not implemented"))]
    UnsupportedFeature {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Fetch type out of range"))]
    FetchTypeOutOfRange {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("SQLFetch cannot be mixed with SQLExtendedFetch without closing cursor"))]
    ExtendedFetchUsed {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to parse port '{port}'"))]
    InvalidPort {
        port: String,
        source: std::num::ParseIntError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to set SQL query: {query}"))]
    SetSqlQuery {
        query: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to prepare statement: {statement}"))]
    PrepareStatement {
        statement: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to execute statement: {statement}"))]
    ExecuteStatement {
        statement: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to bind parameters: {parameters}"))]
    BindParameters {
        parameters: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Connection initialization failed: {connection}"))]
    ConnectionInit {
        connection: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error reading arrow value: {source:?}"))]
    ConversionError {
        #[snafu(source(from(ConversionError, Box::new)))]
        source: Box<ConversionError>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error binding JSON parameters: {source:?}"))]
    JsonBinding {
        source: JsonBindingError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error binding parameters: {parameters}"))]
    ParameterBinding {
        parameters: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error fetching data: {source}"))]
    FetchData {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Text conversion error: {source}"))]
    TextConversionFromUtf8 {
        source: FromUtf8Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Text conversion error: {source}"))]
    TextConversionFromUtf16 {
        source: FromUtf16Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Text conversion error: {source}"))]
    TextConversionUtf8 {
        source: Utf8Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error while creating arrow array stream reader: {source}"))]
    ArrowArrayStreamReaderCreation {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Received core protobuf error"))]
    CoreError {
        source: CoreProtobufError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("[Core] Required field missing: {message}"))]
    ProtoRequiredFieldMissing {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid FreeStmt option: {option}"))]
    InvalidFreeStmtOption {
        option: u16,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("ODBC runtime error"))]
    OdbcRuntime {
        source: crate::api::runtime::OdbcRuntimeError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Data source name not found: {dsn}"))]
    DataSourceNotFound {
        dsn: String,
        #[snafu(implicit)]
        location: Location,
    },
}

pub trait Required<T>: Sized {
    fn required(self, message: &str) -> Result<T, OdbcError>;
}

impl<T> Required<T> for Option<T> {
    #[track_caller]
    fn required(self, message: &str) -> Result<T, OdbcError> {
        self.ok_or_else(|| OdbcError::ProtoRequiredFieldMissing {
            message: message.to_string(),
            location: location!(),
        })
    }
}

static AUTHENTICATOR_PARAMETERS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let mut set = HashSet::new();
    set.insert("PRIV_KEY_FILE".to_string());
    set.insert("PRIVATE_KEY_FILE".to_string());
    set.insert("PRIV_KEY_FILE_PWD".to_string());
    set.insert("PRIV_KEY_BASE64".to_string());
    set.insert("PRIV_KEY_PWD".to_string());
    set.insert("PRIVATE_KEY".to_string());
    set.insert("PRIVATE_KEY_PASSWORD".to_string());
    set.insert("TOKEN".to_string());
    set.insert("AUTHENTICATOR".to_string());
    set.insert("USER".to_string());
    set.insert("PASSWORD".to_string());
    set
});

impl OdbcError {
    pub fn message_text(&self) -> String {
        let trace = self.error_trace();
        let error_message = trace
            .last()
            .map(|entry| entry.message.clone())
            .unwrap_or_default();
        let trace_text = format_error_trace(&trace);
        format!("{}\nTrace:\n{}", error_message, trace_text)
    }

    pub fn to_diagnostic_record(&self) -> DiagnosticRecord {
        DiagnosticRecord {
            message_text: self.message_text(),
            sql_state: self.to_sql_state(),
            native_error: self.to_native_error(),
            ..Default::default()
        }
    }

    pub fn to_sql_state(&self) -> SqlState {
        match self {
            OdbcError::Disconnected { .. } => SqlState::ConnectionDoesNotExist,
            OdbcError::InvalidHandle { .. } => SqlState::InvalidConnectionName,
            OdbcError::NullPointer { .. } => SqlState::InvalidUseOfNullPointer,
            OdbcError::InvalidDescriptorKind { .. } => SqlState::GeneralError,
            OdbcError::InvalidBufferLength { .. } => SqlState::InvalidStringOrBufferLength,
            OdbcError::InvalidApplicationBufferType { .. } => {
                SqlState::InvalidApplicationBufferType
            }
            OdbcError::InvalidParameterType { .. } => SqlState::InvalidParameterType,
            OdbcError::InvalidSqlDataType { .. } => SqlState::InvalidSqlDataType,
            OdbcError::InvalidRecordNumber { .. } => SqlState::InvalidDescriptorIndex,
            OdbcError::InvalidDiagnosticIdentifier { .. } => {
                SqlState::InvalidDescriptorFieldIdentifier
            }
            OdbcError::InvalidDescriptorIndex { .. } => SqlState::InvalidDescriptorIndex,
            OdbcError::InvalidPrecisionOrScale { .. } => SqlState::InvalidPrecisionOrScaleValue,
            OdbcError::UnknownAttribute { .. } => SqlState::InvalidAttributeOptionIdentifier,
            OdbcError::UnsupportedAttribute { .. } => SqlState::OptionalFeatureNotImplemented,
            OdbcError::UnsupportedInfoType { .. } => SqlState::OptionalFeatureNotImplemented,
            OdbcError::UnknownInfoType { .. } => SqlState::OptionalFeatureNotImplemented,
            OdbcError::AttributeCannotBeSetNow { .. } => SqlState::AttributeCannotBeSetNow,
            OdbcError::InvalidParameterNumber { .. } => SqlState::InvalidDescriptorIndex,
            OdbcError::StatementNotExecuted { .. } => SqlState::FunctionSequenceError,
            OdbcError::InvalidCursorState { .. } => SqlState::InvalidCursorState,
            OdbcError::DataNotFetched { .. } => SqlState::FunctionSequenceError,
            OdbcError::NoMoreData { .. } => SqlState::NoDataFound,
            OdbcError::InvalidCursorPosition { .. } => SqlState::InvalidCursorPosition,
            OdbcError::MixedCursorFunctions { .. } => SqlState::FunctionSequenceError,
            OdbcError::UnsupportedFeature { .. } => SqlState::OptionalFeatureNotImplemented,
            OdbcError::FetchTypeOutOfRange { .. } => SqlState::FetchTypeOutOfRange,
            OdbcError::ExtendedFetchUsed { .. } => SqlState::FunctionSequenceError,
            OdbcError::InvalidPort { .. } => SqlState::InvalidConnectionStringAttribute,
            OdbcError::SetSqlQuery { .. } => SqlState::SyntaxErrorOrAccessRuleViolation,
            OdbcError::PrepareStatement { .. } => SqlState::SyntaxErrorOrAccessRuleViolation,
            OdbcError::ExecuteStatement { .. } => SqlState::GeneralError,
            OdbcError::BindParameters { .. } => SqlState::WrongNumberOfParameters,
            OdbcError::ConnectionInit { .. } => SqlState::ClientUnableToEstablishConnection,
            OdbcError::ConversionError { source, .. } => match source.as_ref() {
                ConversionError::WriteOdbcValue { source, .. } => match source {
                    WriteOdbcError::InvalidValue { .. } => SqlState::InvalidCharacterValueForCast,
                    WriteOdbcError::NumericLiteralParsing { .. } => {
                        SqlState::InvalidCharacterValueForCast
                    }
                    WriteOdbcError::RustParsing { .. } => SqlState::NumericValueOutOfRange,
                    WriteOdbcError::NumericValueOutOfRange { .. } => {
                        SqlState::NumericValueOutOfRange
                    }
                    WriteOdbcError::IndicatorRequired { .. }
                    | WriteOdbcError::IndicatorVariableRequired { .. } => {
                        SqlState::IndicatorVariableRequiredButNotSupplied
                    }
                    WriteOdbcError::IntervalFieldOverflow { .. } => SqlState::IntervalFieldOverflow,
                    WriteOdbcError::UnsupportedOdbcType { .. } => {
                        SqlState::RestrictedDataTypeAttributeViolation
                    }
                },
                ConversionError::ReadArrowValue { .. } => SqlState::GeneralError,
                _ => SqlState::GeneralError,
            },
            OdbcError::ParameterBinding { .. } => SqlState::WrongNumberOfParameters,
            OdbcError::FetchData { .. } => SqlState::GeneralError,
            OdbcError::TextConversionUtf8 { .. } => SqlState::StringDataRightTruncated,
            OdbcError::TextConversionFromUtf8 { .. } => SqlState::StringDataRightTruncated,
            OdbcError::TextConversionFromUtf16 { .. } => SqlState::StringDataRightTruncated,
            OdbcError::JsonBinding { .. } => SqlState::GeneralError,
            OdbcError::CoreError {
                source: CoreProtobufError::Application { error, message, .. },
                ..
            } => match error.as_ref() {
                ErrorType::AuthError(_) => SqlState::InvalidAuthorizationSpecification,
                ErrorType::GenericError(_) => {
                    if message.contains("SQL compilation error") {
                        SqlState::SyntaxErrorOrAccessRuleViolation
                    } else {
                        SqlState::GeneralError
                    }
                }
                ErrorType::InvalidParameterValue(ProtoInvalidParameterValue {
                    parameter, ..
                }) => {
                    if AUTHENTICATOR_PARAMETERS.contains(&parameter.to_uppercase()) {
                        SqlState::InvalidAuthorizationSpecification
                    } else {
                        SqlState::InvalidConnectionStringAttribute
                    }
                }
                ErrorType::MissingParameter(ProtoMissingParameter { parameter }) => {
                    if AUTHENTICATOR_PARAMETERS.contains(&parameter.to_uppercase()) {
                        SqlState::InvalidAuthorizationSpecification
                    } else {
                        SqlState::InvalidConnectionStringAttribute
                    }
                }
                ErrorType::InternalError(_) => {
                    if message.contains("SQL compilation error") {
                        SqlState::SyntaxErrorOrAccessRuleViolation
                    } else {
                        SqlState::GeneralError
                    }
                }
                ErrorType::LoginError(_) => SqlState::InvalidAuthorizationSpecification,
            },
            OdbcError::CoreError { source, .. } => match source {
                CoreProtobufError::Transport { .. } => SqlState::ClientUnableToEstablishConnection,
                CoreProtobufError::Application { .. } => SqlState::GeneralError,
            },
            OdbcError::ProtoRequiredFieldMissing { .. } => SqlState::GeneralError,
            OdbcError::ArrowArrayStreamReaderCreation { .. } => SqlState::GeneralError,
            OdbcError::StatementErrorState { .. } => SqlState::GeneralError,
            OdbcError::InvalidFreeStmtOption { .. } => SqlState::InvalidAttributeOptionIdentifier,
            OdbcError::OdbcRuntime { .. } => SqlState::FunctionSequenceError,
            OdbcError::DataSourceNotFound { .. } => {
                SqlState::DataSourceNameNotFoundAndNoDefaultDriverSpecified
            }
        }
    }

    pub fn to_native_error(&self) -> sql::Integer {
        match self {
            OdbcError::CoreError { source, .. } => match source {
                CoreProtobufError::Application { error, .. } => match error.as_ref() {
                    ErrorType::LoginError(login_error) => login_error.code,
                    _ => 0,
                },
                CoreProtobufError::Transport { .. } => 0,
            },
            _ => 0,
        }
    }

    #[track_caller]
    pub fn from_protobuf_error(error: ProtoError<ProtoDriverException>) -> OdbcError {
        let loc = std::panic::Location::caller();
        let location = Location::new(loc.file(), loc.line(), loc.column());
        let core_error = match error {
            ProtoError::Application(driver_exception) => CoreProtobufError::Application {
                error: Box::new(
                    driver_exception
                        .error
                        .and_then(|error| error.error_type)
                        .unwrap_or(ErrorType::GenericError(GenericError {})),
                ),
                message: driver_exception.message,
                status_code: driver_exception.status_code,
                error_trace: driver_exception.error_trace,
                location,
            },
            ProtoError::Transport(message) => CoreProtobufError::Transport { message, location },
        };
        OdbcError::CoreError {
            source: core_error,
            location,
        }
    }
}

impl From<ProtoError<ProtoDriverException>> for OdbcError {
    #[track_caller]
    fn from(error: ProtoError<ProtoDriverException>) -> Self {
        OdbcError::from_protobuf_error(error)
    }
}

#[derive(Debug, Snafu)]
pub enum CoreProtobufError {
    #[snafu(display("Application error: {message}"))]
    Application {
        error: Box<ErrorType>,
        message: String,
        status_code: i32,
        error_trace: Vec<ErrorTraceEntry>,
        location: Location,
    },
    #[snafu(display("Transport error: {message}"))]
    Transport { message: String, location: Location },
}

impl ErrorTrace for CoreProtobufError {
    fn error_trace(&self) -> Vec<error_trace::ErrorTraceEntry> {
        match self {
            CoreProtobufError::Application {
                error_trace,
                message,
                location,
                ..
            } => {
                let mut trace: Vec<error_trace::ErrorTraceEntry> = error_trace
                    .iter()
                    .map(|entry| error_trace::ErrorTraceEntry {
                        location: error_trace::Location::new(
                            entry.file.clone(),
                            entry.line,
                            entry.column,
                        ),
                        message: entry.message.clone(),
                    })
                    .collect();
                if trace.is_empty() {
                    trace.push(error_trace::ErrorTraceEntry {
                        location: error_trace::Location::from(*location),
                        message: message.clone(),
                    });
                }
                trace
            }
            CoreProtobufError::Transport { message, location } => {
                vec![error_trace::ErrorTraceEntry {
                    location: error_trace::Location::from(*location),
                    message: message.clone(),
                }]
            }
        }
    }
}
