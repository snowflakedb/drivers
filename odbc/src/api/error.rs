use std::{
    collections::HashSet,
    str::Utf8Error,
    string::{FromUtf8Error, FromUtf16Error},
    sync::LazyLock,
};

use crate::{
    api::{InfoType, SqlState, diagnostic::DiagnosticRecord, oauth},
    conversion::error::BindingError,
    conversion::{ConversionError, error::WriteOdbcError},
};
use arrow::error::ArrowError;
use odbc_sys as sql;
use proto_utils::ProtoError;
use sf_core::protobuf::generated::database_driver_v1::{
    ErrorTraceEntry, GenericError, InvalidParameterValue as ProtoInvalidParameterValue,
    MissingParameter as ProtoMissingParameter, StatusCode as ProtoStatusCode,
    driver_error::ErrorType,
};

use error_trace::ErrorTrace;
use sf_core::protobuf::generated::database_driver_v1::DriverException as ProtoDriverException;
use snafu::{Location, Snafu, location};
use strum_macros::{Display as StrumDisplay, EnumDiscriminants, EnumIter, IntoStaticStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IntoStaticStr, StrumDisplay, EnumIter)]
#[strum(serialize_all = "snake_case")]
pub enum ErrorSource {
    /// Couldn't establish or lost the link to the server (e.g. failed
    /// connection init, transport-level RPC failure).
    Connectivity,
    /// The server returned an error over the wire (auth, query-execution
    /// failure, generic protobuf application error, ...).
    ServerError,
    /// Value-shape / encoding errors detected by the wrapper (binding
    /// conversions, arrow / text decoding, fetch).
    DataConversion,
    /// Cursor or statement-state sequencing violations (no result set,
    /// cursor already open, no more data, statement not executed, ...).
    CursorState,
    /// Caller violated the ODBC contract (invalid handle, null pointer,
    /// bad parameter, sequence error on env/dbc freeing, ...).
    ApiMisuse,
    /// Connection-string / DSN / port parsing.
    ConfigParsing,
    /// Wrapper bugs / state corruption (lock poisoning, runtime not
    /// initialised, missing protobuf fields, internal arrow infrastructure
    /// failures).
    InternalError,
    /// A feature, attribute, or info type the wrapper does not (yet)
    /// implement.
    Unsupported,
    /// Errors that do not map to a more specific bucket (see
    /// [`OdbcError::error_source`]).
    Unknown,
}

#[derive(Snafu, Debug, ErrorTrace, IntoStaticStr, EnumDiscriminants)]
#[snafu(visibility(pub))]
#[strum_discriminants(
    name(OdbcErrorKind),
    derive(strum_macros::EnumIter, strum_macros::EnumCount)
)]
pub enum OdbcError {
    #[snafu(display("Freeing environment failed: environment has connections"))]
    EnvironmentHasConnections {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Freeing connection failed: connection is still connected"))]
    ConnectionStillConnected {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Connection has no environment"))]
    ConnectionHasNoEnvironment {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to lock environment"))]
    EnvironmentLockPoisoned {
        #[snafu(implicit)]
        location: Location,
    },

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

    #[snafu(display("Invalid use of an automatically allocated descriptor handle"))]
    InvalidUseOfImplicitDescriptor {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid handle type for this operation: {handle_type}"))]
    InvalidHandleType {
        handle_type: i16,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid transaction operation code: {completion_type}"))]
    InvalidTransactionOperationCode {
        completion_type: i16,
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

    #[snafu(display("Attribute {attribute} is read-only and cannot be set"))]
    ReadOnlyAttribute {
        attribute: i32,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid descriptor field identifier: {field_id}"))]
    InvalidDescriptorFieldId {
        field_id: i16,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Cannot modify an implementation row descriptor"))]
    CannotModifyIrd {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Inconsistent descriptor information: {reason}"))]
    InconsistentDescriptorInfo {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("SQL_DESC_UNNAMED may only be set to SQL_UNNAMED, not SQL_NAMED"))]
    CannotSetUnnamedToNamed {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported attribute: {attribute}"))]
    UnsupportedAttribute {
        attribute: i32,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid attribute value {value} for attribute {attribute}"))]
    InvalidAttributeValue {
        attribute: i32,
        value: i64,
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

    #[snafu(display("Associated statement is not prepared"))]
    AssociatedStatementNotPrepared {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("COUNT field incorrect: {reason}"))]
    CountFieldIncorrect {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid catalog name: {name}"))]
    InvalidCatalogName {
        name: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid cursor state: no result set associated with the statement"))]
    InvalidCursorState {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid cursor state: cursor is already open"))]
    CursorAlreadyOpen {
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

    #[snafu(display("Internal driver error: {message}"))]
    InternalError {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("{command}: result set is missing the '{column}' column"))]
    ShowKeysColumnMissing {
        command: &'static str,
        column: &'static str,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("SHOW KEYS result: KEY_SEQ is missing, null, or not a valid SMALLINT"))]
    ShowKeysInvalidKeySeq {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("procedure metadata parse error: {detail}"))]
    ProcedureMetadataParse {
        detail: String,
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
        #[snafu(source(from(BindingError, Box::new)))]
        source: Box<BindingError>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error binding CSV parameters: {source:?}"))]
    CsvBinding {
        #[snafu(source(from(BindingError, Box::new)))]
        source: Box<BindingError>,
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

    #[snafu(display("Invalid wide-character code point U+{code_point:08X}"))]
    InvalidWideChar {
        code_point: u32,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error while creating arrow array stream reader: {source}"))]
    ArrowArrayStreamReaderCreation {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error reading arrow record batch: {source}"))]
    ArrowBatchRead {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error concatenating arrow record batches: {source}"))]
    ArrowBatchConcat {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error building arrow record batch: {source}"))]
    RecordBatchBuild {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },

    // `error_source()` returns [`ErrorSource::Unknown`]; the wire payload
    // uses `telemetry_classification` to map the inner
    // `CoreProtobufError` (Transport → connectivity, Application → server_error).
    #[snafu(display("Received core protobuf error"))]
    CoreError {
        source: Box<CoreProtobufError>,
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

    // Caller-initiated cancel; no dedicated bucket — maps to [`ErrorSource::Unknown`].
    #[snafu(display("Operation canceled"))]
    OperationCanceled {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid connection string: {reason}"))]
    InvalidConnectionString {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Data-at-execution required"))]
    DaeRequired {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "Function sequence error: cannot call this function during data-at-execution"
    ))]
    InvalidDuringDae {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Non-character and non-binary data sent in pieces"))]
    NonCharBinarySentInPieces {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Asynchronous operation still executing"))]
    StillExecuting {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Attempt to concatenate a null value"))]
    ConcatNullValue {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "Function sequence error: cannot call this function while async operation is in progress"
    ))]
    AsyncInProgress {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Function type out of range: {function_id}"))]
    FunctionTypeOutOfRange {
        function_id: u16,
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
    // Pull every recognised OAuth DSN key from the OAuth helper so a
    // future addition to `oauth::ALL_OAUTH_KEYS` automatically updates
    // the set used for SQLSTATE classification of auth-time errors.
    for &k in oauth::ALL_OAUTH_KEYS {
        set.insert(k.to_string());
    }
    set
});

/// Returns `true` when `state` is a syntactically valid ANSI/ODBC SQLSTATE:
/// exactly five characters, each an ASCII digit (`0-9`) or uppercase letter
/// (`A-Z`). Used to gate server-supplied values before forwarding them
/// verbatim to ODBC consumers, so the driver never emits non-conforming
/// SQLSTATEs in diagnostics.
fn is_well_formed_sql_state(state: &str) -> bool {
    state.len() == 5
        && state
            .bytes()
            .all(|b| b.is_ascii_digit() || b.is_ascii_uppercase())
}

fn binding_error_to_sql_state(source: &BindingError) -> SqlState {
    match source {
        BindingError::NumericMagnitudeOverflow { .. }
        | BindingError::BindingNumericOutOfRange { .. } => SqlState::NumericValueOutOfRange,
        BindingError::InvalidDatetimeValue { .. } => SqlState::InvalidDatetimeFormat,
        BindingError::DatetimeFieldOverflow { .. } => SqlState::DatetimeFieldOverflow,
        BindingError::UnsupportedCDataType { .. }
        | BindingError::UnsupportedParameterType { .. } => {
            SqlState::RestrictedDataTypeAttributeViolation
        }
        BindingError::InvalidBooleanValue { .. }
        | BindingError::InvalidNumericLiteral { .. }
        | BindingError::InvalidHexLiteral { .. }
        | BindingError::InvalidCharacterValueForCast { .. } => {
            SqlState::InvalidCharacterValueForCast
        }
        _ => SqlState::GeneralError,
    }
}

impl OdbcError {
    /// High-level telemetry bucket for this wrapper error.
    ///
    /// [`OdbcError::CoreError`] returns [`ErrorSource::Unknown`] here; use
    /// [`telemetry_classification`] for the in-band wire payload, which
    /// inspects the inner protobuf error.
    pub fn error_source(&self) -> ErrorSource {
        match self {
            OdbcError::EnvironmentHasConnections { .. } => ErrorSource::ApiMisuse,
            OdbcError::ConnectionStillConnected { .. } => ErrorSource::ApiMisuse,
            OdbcError::ConnectionHasNoEnvironment { .. } => ErrorSource::InternalError,
            OdbcError::EnvironmentLockPoisoned { .. } => ErrorSource::InternalError,
            OdbcError::Disconnected { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidHandle { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidUseOfImplicitDescriptor { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidHandleType { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidTransactionOperationCode { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidDescriptorKind { .. } => ErrorSource::ApiMisuse,
            OdbcError::NullPointer { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidBufferLength { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidApplicationBufferType { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidParameterType { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidSqlDataType { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidRecordNumber { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidDescriptorIndex { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidPrecisionOrScale { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidDiagnosticIdentifier { .. } => ErrorSource::ApiMisuse,
            OdbcError::UnknownAttribute { .. } => ErrorSource::ApiMisuse,
            OdbcError::ReadOnlyAttribute { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidDescriptorFieldId { .. } => ErrorSource::ApiMisuse,
            OdbcError::CannotModifyIrd { .. } => ErrorSource::ApiMisuse,
            OdbcError::InconsistentDescriptorInfo { .. } => ErrorSource::ApiMisuse,
            OdbcError::CannotSetUnnamedToNamed { .. } => ErrorSource::ApiMisuse,
            OdbcError::UnsupportedAttribute { .. } => ErrorSource::Unsupported,
            OdbcError::InvalidAttributeValue { .. } => ErrorSource::ApiMisuse,
            OdbcError::UnsupportedInfoType { .. } => ErrorSource::Unsupported,
            OdbcError::UnknownInfoType { .. } => ErrorSource::Unsupported,
            OdbcError::AttributeCannotBeSetNow { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidParameterNumber { .. } => ErrorSource::ApiMisuse,
            OdbcError::StatementNotExecuted { .. } => ErrorSource::CursorState,
            OdbcError::AssociatedStatementNotPrepared { .. } => ErrorSource::CursorState,
            OdbcError::CountFieldIncorrect { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidCatalogName { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidCursorState { .. } => ErrorSource::CursorState,
            OdbcError::CursorAlreadyOpen { .. } => ErrorSource::CursorState,
            OdbcError::StatementErrorState { .. } => ErrorSource::CursorState,
            OdbcError::DataNotFetched { .. } => ErrorSource::CursorState,
            OdbcError::NoMoreData { .. } => ErrorSource::CursorState,
            OdbcError::InvalidCursorPosition { .. } => ErrorSource::CursorState,
            OdbcError::MixedCursorFunctions { .. } => ErrorSource::CursorState,
            OdbcError::InternalError { .. } => ErrorSource::InternalError,
            OdbcError::ShowKeysColumnMissing { .. } => ErrorSource::InternalError,
            OdbcError::ShowKeysInvalidKeySeq { .. } => ErrorSource::InternalError,
            OdbcError::ProcedureMetadataParse { .. } => ErrorSource::InternalError,
            OdbcError::UnsupportedFeature { .. } => ErrorSource::Unsupported,
            OdbcError::FetchTypeOutOfRange { .. } => ErrorSource::CursorState,
            OdbcError::ExtendedFetchUsed { .. } => ErrorSource::CursorState,
            OdbcError::InvalidPort { .. } => ErrorSource::ConfigParsing,
            OdbcError::SetSqlQuery { .. } => ErrorSource::ServerError,
            OdbcError::PrepareStatement { .. } => ErrorSource::ServerError,
            OdbcError::ExecuteStatement { .. } => ErrorSource::ServerError,
            OdbcError::BindParameters { .. } => ErrorSource::DataConversion,
            OdbcError::ConnectionInit { .. } => ErrorSource::Connectivity,
            OdbcError::ConversionError { .. } => ErrorSource::DataConversion,
            OdbcError::JsonBinding { .. } => ErrorSource::DataConversion,
            OdbcError::CsvBinding { .. } => ErrorSource::DataConversion,
            OdbcError::ParameterBinding { .. } => ErrorSource::DataConversion,
            OdbcError::FetchData { .. } => ErrorSource::DataConversion,
            OdbcError::TextConversionFromUtf8 { .. } => ErrorSource::DataConversion,
            OdbcError::TextConversionFromUtf16 { .. } => ErrorSource::DataConversion,
            OdbcError::TextConversionUtf8 { .. } => ErrorSource::DataConversion,
            OdbcError::InvalidWideChar { .. } => ErrorSource::DataConversion,
            OdbcError::ArrowArrayStreamReaderCreation { .. } => ErrorSource::InternalError,
            OdbcError::ArrowBatchRead { .. } => ErrorSource::InternalError,
            OdbcError::ArrowBatchConcat { .. } => ErrorSource::InternalError,
            OdbcError::RecordBatchBuild { .. } => ErrorSource::InternalError,
            OdbcError::CoreError { .. } => ErrorSource::Unknown,
            OdbcError::ProtoRequiredFieldMissing { .. } => ErrorSource::InternalError,
            OdbcError::InvalidFreeStmtOption { .. } => ErrorSource::ApiMisuse,
            OdbcError::OdbcRuntime { .. } => ErrorSource::InternalError,
            OdbcError::DataSourceNotFound { .. } => ErrorSource::ConfigParsing,
            OdbcError::OperationCanceled { .. } => ErrorSource::Unknown,
            OdbcError::InvalidConnectionString { .. } => ErrorSource::ConfigParsing,
            OdbcError::DaeRequired { .. } => ErrorSource::ApiMisuse,
            OdbcError::InvalidDuringDae { .. } => ErrorSource::ApiMisuse,
            OdbcError::NonCharBinarySentInPieces { .. } => ErrorSource::ApiMisuse,
            OdbcError::ConcatNullValue { .. } => ErrorSource::ApiMisuse,
            OdbcError::StillExecuting { .. } => ErrorSource::ApiMisuse,
            OdbcError::AsyncInProgress { .. } => ErrorSource::ApiMisuse,
            OdbcError::FunctionTypeOutOfRange { .. } => ErrorSource::ApiMisuse,
        }
    }

    /// Map this error to the in-band telemetry spec's
    /// `(exception_type, error_source)` pair sent in
    /// `TelemetrySendWrapperErrorRequest`.
    ///
    /// - `exception_type` is the snafu variant name (no PII, no message
    ///   content), produced by `IntoStaticStr`. Renaming a variant
    ///   automatically updates this label, so there is no chance of drift.
    /// - `error_source` is the high-level [`ErrorSource`] bucket from
    ///   [`OdbcError::error_source`], except [`OdbcError::CoreError`] is
    ///   split by inner transport vs application failure.
    pub fn telemetry_classification(&self) -> (&'static str, ErrorSource) {
        let exception_type: &'static str = self.into();
        let error_source = match self {
            OdbcError::CoreError { source, .. } => match source.as_ref() {
                CoreProtobufError::Transport { .. } => ErrorSource::Connectivity,
                CoreProtobufError::Application { .. } => ErrorSource::ServerError,
            },
            _ => self.error_source(),
        };
        (exception_type, error_source)
    }

    pub fn message_text(&self) -> String {
        let trace = self.error_trace();
        let base = self.structured_message().unwrap_or_else(|| {
            trace
                .last()
                .map(|entry| entry.message.clone())
                .unwrap_or_default()
        });
        if crate::api::error_trace_flag::error_trace_enabled() && !trace.is_empty() {
            format!(
                "{base}\nerror trace:\n{}",
                error_trace::format_error_trace(&trace)
            )
        } else {
            base
        }
    }

    /// Extract a user-facing message from structured protobuf error fields
    /// when available; returns `None` to fall back to the generic trace-based
    /// message.
    fn structured_message(&self) -> Option<String> {
        match self {
            OdbcError::CoreError { source, .. } => match source.as_ref() {
                CoreProtobufError::Application { error, .. } => match error.as_ref() {
                    ErrorType::InvalidParameterValue(ProtoInvalidParameterValue {
                        explanation: Some(explanation),
                        ..
                    }) => Some(explanation.clone()),
                    ErrorType::MissingParameter(ProtoMissingParameter { parameter }) => {
                        Some(format!("Missing required parameter: {parameter}"))
                    }
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
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
            OdbcError::EnvironmentHasConnections { .. } => SqlState::FunctionSequenceError,
            OdbcError::ConnectionStillConnected { .. } => SqlState::FunctionSequenceError,
            OdbcError::ConnectionHasNoEnvironment { .. } => SqlState::GeneralError,
            OdbcError::EnvironmentLockPoisoned { .. } => SqlState::GeneralError,
            OdbcError::Disconnected { .. } => SqlState::ConnectionDoesNotExist,
            OdbcError::InvalidHandle { .. } => SqlState::InvalidConnectionName,
            OdbcError::InvalidUseOfImplicitDescriptor { .. } => {
                SqlState::InvalidUseOfAutomaticallyAllocatedDescriptorHandle
            }
            OdbcError::InvalidHandleType { .. } => SqlState::InvalidAttributeOptionIdentifier,
            OdbcError::InvalidTransactionOperationCode { .. } => {
                SqlState::InvalidTransactionOperationCode
            }
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
            OdbcError::ReadOnlyAttribute { .. } => SqlState::InvalidAttributeOptionIdentifier,
            OdbcError::InvalidDescriptorFieldId { .. } => {
                SqlState::InvalidDescriptorFieldIdentifier
            }
            OdbcError::CannotModifyIrd { .. } => SqlState::CannotModifyImplementationRowDescriptor,
            OdbcError::InconsistentDescriptorInfo { .. } => {
                SqlState::InconsistentDescriptorInformation
            }
            OdbcError::CannotSetUnnamedToNamed { .. } => SqlState::InvalidAttributeOptionIdentifier,
            OdbcError::UnsupportedAttribute { .. } => SqlState::OptionalFeatureNotImplemented,
            OdbcError::InvalidAttributeValue { .. } => SqlState::InvalidAttributeValue,
            OdbcError::UnsupportedInfoType { .. } => SqlState::OptionalFeatureNotImplemented,
            OdbcError::UnknownInfoType { .. } => SqlState::OptionalFeatureNotImplemented,
            OdbcError::AttributeCannotBeSetNow { .. } => SqlState::AttributeCannotBeSetNow,
            OdbcError::InvalidParameterNumber { .. } => SqlState::InvalidDescriptorIndex,
            OdbcError::StatementNotExecuted { .. } => SqlState::FunctionSequenceError,
            OdbcError::AssociatedStatementNotPrepared { .. } => {
                SqlState::AssociatedStatementIsNotPrepared
            }
            OdbcError::CountFieldIncorrect { .. } => SqlState::CountFieldIncorrect,
            OdbcError::InvalidCatalogName { .. } => SqlState::InvalidCatalogName,
            OdbcError::InvalidCursorState { .. } => SqlState::InvalidCursorState,
            OdbcError::CursorAlreadyOpen { .. } => SqlState::InvalidCursorState,
            OdbcError::DataNotFetched { .. } => SqlState::FunctionSequenceError,
            OdbcError::NoMoreData { .. } => SqlState::NoDataFound,
            OdbcError::InvalidCursorPosition { .. } => SqlState::InvalidCursorPosition,
            OdbcError::MixedCursorFunctions { .. } => SqlState::FunctionSequenceError,
            OdbcError::InternalError { .. } => SqlState::GeneralError,
            OdbcError::ShowKeysColumnMissing { .. } => SqlState::GeneralError,
            OdbcError::ShowKeysInvalidKeySeq { .. } => SqlState::GeneralError,
            OdbcError::ProcedureMetadataParse { .. } => SqlState::GeneralError,
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
                ConversionError::DatetimeOutOfSqlRange { .. } => SqlState::InvalidDatetimeFormat,
                ConversionError::ReadArrowValue { .. } => SqlState::GeneralError,
                _ => SqlState::GeneralError,
            },
            OdbcError::ParameterBinding { .. } => SqlState::WrongNumberOfParameters,
            OdbcError::FetchData { .. } => SqlState::GeneralError,
            OdbcError::TextConversionUtf8 { .. } => SqlState::StringDataRightTruncated,
            OdbcError::TextConversionFromUtf8 { .. } => SqlState::StringDataRightTruncated,
            OdbcError::TextConversionFromUtf16 { .. } => SqlState::StringDataRightTruncated,
            OdbcError::InvalidWideChar { .. } => SqlState::StringDataRightTruncated,
            OdbcError::JsonBinding { source, .. } | OdbcError::CsvBinding { source, .. } => {
                binding_error_to_sql_state(source)
            }
            OdbcError::CoreError { source, .. } => match source.as_ref() {
                CoreProtobufError::Transport { .. } => SqlState::ClientUnableToEstablishConnection,
                CoreProtobufError::Application {
                    error,
                    sql_state: server_sql_state,
                    ..
                } => {
                    // Forward the server's SQLSTATE verbatim when it's a
                    // well-formed 5-character ANSI/ODBC state outside the
                    // success/warning/no-data classes. The driver does not
                    // invent or override SQLSTATE classifications
                    // client-side — that responsibility belongs to the
                    // server (and to `sf_core::extract_vendor_info`, which
                    // fills in `sql_state` from the numeric error code on
                    // wire paths that drop it).
                    //
                    // Validation rules:
                    // - Must match `[0-9A-Z]{5}` exactly. SQLSTATE is
                    //   defined as five characters drawn from digits and
                    //   uppercase ASCII letters; anything else (lowercase,
                    //   punctuation, non-ASCII) is malformed and would
                    //   produce a non-spec SQLSTATE in diagnostics.
                    //   Malformed values fall through to the per-error
                    //   default (typically HY000).
                    // - "00xxx" (success) and "01xxx" (warning) must not appear in an
                    //   error record — callers would silently ignore the error.
                    // - "02xxx" (no-data) must be excluded: NoDataFound is not in
                    //   is_warning(), so is_error() treats it as an error, but ODBC
                    //   callers expect 02000 only on success returns (e.g. SQLFetch).
                    //
                    // SQLSTATEs the local enum doesn't recognise (e.g. the
                    // generic data-exception "22000") are passed through as
                    // `SqlState::Unknown`, which `as_str()` renders
                    // verbatim so consumers still see the server's code.
                    if let Some(state) = server_sql_state
                        && is_well_formed_sql_state(state)
                        && !state.starts_with("00")
                        && !state.starts_with("01")
                        && !state.starts_with("02")
                    {
                        // The `impl FromStr for SqlState` is currently
                        // infallible (every unrecognised code falls into
                        // `SqlState::Unknown`), but use `unwrap_or_else` to
                        // keep the driver panic-free if that contract ever
                        // changes.
                        return state
                            .parse()
                            .unwrap_or_else(|_| SqlState::Unknown(state.to_owned()));
                    }
                    match error.as_ref() {
                        ErrorType::AuthError(_) => SqlState::InvalidAuthorizationSpecification,
                        ErrorType::GenericError(_) | ErrorType::InternalError(_) => {
                            // No usable SQLSTATE on the wire and `sf_core`'s
                            // `extract_vendor_info` couldn't recover one
                            // from the numeric error code either, so HY000
                            // is the honest default. Do NOT sniff the
                            // message text — classification belongs to the
                            // server, not to the driver.
                            SqlState::GeneralError
                        }
                        ErrorType::InvalidParameterValue(ProtoInvalidParameterValue {
                            parameter,
                            ..
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
                        ErrorType::LoginError(_) => SqlState::InvalidAuthorizationSpecification,
                    }
                }
            },
            OdbcError::ProtoRequiredFieldMissing { .. } => SqlState::GeneralError,
            OdbcError::ArrowArrayStreamReaderCreation { .. } => SqlState::GeneralError,
            OdbcError::ArrowBatchRead { .. } => SqlState::GeneralError,
            OdbcError::ArrowBatchConcat { .. } => SqlState::GeneralError,
            OdbcError::RecordBatchBuild { .. } => SqlState::GeneralError,
            OdbcError::StatementErrorState { .. } => SqlState::GeneralError,
            OdbcError::InvalidFreeStmtOption { .. } => SqlState::InvalidAttributeOptionIdentifier,
            OdbcError::OdbcRuntime { .. } => SqlState::FunctionSequenceError,
            OdbcError::DataSourceNotFound { .. } => {
                SqlState::DataSourceNameNotFoundAndNoDefaultDriverSpecified
            }
            OdbcError::OperationCanceled { .. } => SqlState::OperationCanceled,
            OdbcError::InvalidConnectionString { .. } => SqlState::InvalidConnectionStringAttribute,
            OdbcError::DaeRequired { .. } => SqlState::GeneralError,
            OdbcError::InvalidDuringDae { .. } => SqlState::FunctionSequenceError,
            OdbcError::NonCharBinarySentInPieces { .. } => {
                SqlState::NonCharacterAndNonBinaryDataSentInPieces
            }
            OdbcError::ConcatNullValue { .. } => SqlState::AttemptToConcatenateNullValue,
            OdbcError::StillExecuting { .. } => SqlState::GeneralError,
            OdbcError::AsyncInProgress { .. } => SqlState::FunctionSequenceError,
            OdbcError::FunctionTypeOutOfRange { .. } => SqlState::FunctionTypeOutOfRange,
        }
    }

    pub fn to_native_error(&self) -> sql::Integer {
        match self {
            OdbcError::CoreError { source, .. } => match source.as_ref() {
                CoreProtobufError::Application { error, .. } => match error.as_ref() {
                    ErrorType::LoginError(login_error) => login_error.code,
                    _ => 0,
                },
                CoreProtobufError::Transport { .. } => 0,
            },
            _ => 0,
        }
    }

    pub fn query_id(&self) -> Option<&str> {
        match self {
            OdbcError::CoreError { source, .. } => match source.as_ref() {
                CoreProtobufError::Application { query_id, .. } => query_id.as_deref(),
                _ => None,
            },
            _ => None,
        }
    }

    /// Server-provided SQLSTATE from a core application error, when present.
    pub fn server_sql_state(&self) -> Option<&str> {
        match self {
            OdbcError::CoreError { source, .. } => match source.as_ref() {
                CoreProtobufError::Application { sql_state, .. } => sql_state.as_deref(),
                _ => None,
            },
            _ => None,
        }
    }

    #[track_caller]
    pub fn from_protobuf_error(error: ProtoError<ProtoDriverException>) -> OdbcError {
        let loc = std::panic::Location::caller();
        let location = Location::new(loc.file(), loc.line(), loc.column());
        let core_error = match error {
            ProtoError::Application(driver_exception) => {
                // A core-side cancellation must surface as OperationCanceled (HY008),
                // not a generic CoreError (HY000); the status_code carries the
                // classification the ApiError->protobuf boundary already erased.
                if driver_exception.status_code == ProtoStatusCode::Cancelled as i32 {
                    return OdbcError::OperationCanceled { location };
                }
                CoreProtobufError::Application {
                    error: Box::new(
                        driver_exception
                            .error
                            .and_then(|error| error.error_type)
                            .unwrap_or(ErrorType::GenericError(GenericError {})),
                    ),
                    message: driver_exception.message,
                    status_code: driver_exception.status_code,
                    error_trace: driver_exception.error_trace,
                    sql_state: driver_exception.sql_state,
                    query_id: driver_exception.query_id,
                    location,
                }
            }
            ProtoError::Transport(message) => CoreProtobufError::Transport { message, location },
        };
        OdbcError::CoreError {
            source: Box::new(core_error),
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
        /// ANSI SQL state forwarded from the server response, if present.
        sql_state: Option<String>,
        /// Snowflake Query ID from the failed query, if available.
        query_id: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversion::error::{
        BindingNumericOutOfRangeSnafu, DatetimeFieldOverflowSnafu, InvalidBooleanValueSnafu,
        InvalidCharacterValueForCastSnafu, InvalidNumericLiteralSnafu,
        NumericMagnitudeOverflowSnafu, UnsupportedCDataTypeSnafu, UnsupportedParameterTypeSnafu,
    };

    fn loc() -> Location {
        Location::new("test", 0, 0)
    }

    #[test]
    fn telemetry_classification_covers_each_error_source() {
        // data_conversion
        assert_eq!(
            OdbcError::ConversionError {
                source: Box::new(ConversionError::ArrowArrayDowncast {
                    expected_type: "Int32Array".into(),
                    location: loc(),
                }),
                location: loc(),
            }
            .telemetry_classification(),
            ("ConversionError", ErrorSource::DataConversion)
        );

        // api_misuse — Disconnected is caller invoking on a not-connected
        // session, NOT a connection pool / connectivity issue.
        assert_eq!(
            OdbcError::Disconnected { location: loc() }.telemetry_classification(),
            ("Disconnected", ErrorSource::ApiMisuse)
        );

        // api_misuse — bad handle
        assert_eq!(
            OdbcError::InvalidHandle { location: loc() }.telemetry_classification(),
            ("InvalidHandle", ErrorSource::ApiMisuse)
        );

        // config_parsing
        assert_eq!(
            OdbcError::InvalidConnectionString {
                reason: "bad".into(),
                location: loc(),
            }
            .telemetry_classification(),
            ("InvalidConnectionString", ErrorSource::ConfigParsing)
        );

        // internal_error — wrapper-side bug, not a result_processing event.
        assert_eq!(
            OdbcError::ProtoRequiredFieldMissing {
                message: "x".into(),
                location: loc(),
            }
            .telemetry_classification(),
            ("ProtoRequiredFieldMissing", ErrorSource::InternalError)
        );

        // connectivity — the only true "couldn't reach the server" variant.
        assert_eq!(
            OdbcError::ConnectionInit {
                connection: "x".into(),
                location: loc(),
            }
            .telemetry_classification(),
            ("ConnectionInit", ErrorSource::Connectivity)
        );

        // server_error — wrapper-level surface for server-rejected query.
        assert_eq!(
            OdbcError::PrepareStatement {
                statement: "select 1".into(),
                location: loc(),
            }
            .telemetry_classification(),
            ("PrepareStatement", ErrorSource::ServerError)
        );

        // cursor_state
        assert_eq!(
            OdbcError::InvalidCursorState { location: loc() }.telemetry_classification(),
            ("InvalidCursorState", ErrorSource::CursorState)
        );

        // unsupported
        assert_eq!(
            OdbcError::UnsupportedFeature { location: loc() }.telemetry_classification(),
            ("UnsupportedFeature", ErrorSource::Unsupported)
        );

        // CoreError special-case: Transport variant routes to connectivity.
        assert_eq!(
            OdbcError::CoreError {
                source: Box::new(CoreProtobufError::Transport {
                    message: "rpc dropped".into(),
                    location: loc(),
                }),
                location: loc(),
            }
            .telemetry_classification(),
            ("CoreError", ErrorSource::Connectivity)
        );

        // CoreError special-case: Application variant routes to server_error.
        assert_eq!(
            OdbcError::CoreError {
                source: Box::new(CoreProtobufError::Application {
                    error: Box::new(ErrorType::GenericError(
                        sf_core::protobuf::generated::database_driver_v1::GenericError {},
                    )),
                    message: "boom".into(),
                    status_code: 0,
                    error_trace: vec![],
                    sql_state: None,
                    query_id: None,
                    location: loc(),
                }),
                location: loc(),
            }
            .telemetry_classification(),
            ("CoreError", ErrorSource::ServerError)
        );

        // unknown — the lone fallback, used by OperationCanceled.
        assert_eq!(
            OdbcError::OperationCanceled { location: loc() }.telemetry_classification(),
            ("OperationCanceled", ErrorSource::Unknown)
        );
    }

    /// `Display` (used to write the wire-format string in
    /// `telemetry::record_wrapper_error`) and `IntoStaticStr` (the
    /// canonical `&'static str` form) MUST agree for every
    /// [`ErrorSource`] variant. This test catches accidental drift such
    /// as forgetting the `#[strum(serialize_all = "snake_case")]`
    /// umbrella, since both derives read it.
    #[test]
    fn error_source_wire_format_is_consistent() {
        use strum::IntoEnumIterator;
        for source in ErrorSource::iter() {
            let s: &'static str = source.into();
            assert_eq!(
                s,
                source.to_string(),
                "Display and IntoStaticStr disagree for {source:?}"
            );
            assert!(
                s.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
                "wire form for {source:?} ({s:?}) is not snake_case"
            );
        }
    }

    #[test]
    fn numeric_magnitude_overflow_maps_to_22003() {
        let json_err = NumericMagnitudeOverflowSnafu {
            reason: "test overflow".to_string(),
        }
        .build();
        let odbc_err = OdbcError::JsonBinding {
            source: Box::new(json_err),
            location: snafu::Location::new("test", 0, 0),
        };
        assert_eq!(odbc_err.to_sql_state(), SqlState::NumericValueOutOfRange);
    }

    #[test]
    fn server_generic_error_maps_to_hy000() {
        let odbc_err = OdbcError::CoreError {
            source: Box::new(CoreProtobufError::Application {
                error: Box::new(ErrorType::GenericError(
                    sf_core::protobuf::generated::database_driver_v1::GenericError {},
                )),
                message: "Some other server error".to_string(),
                status_code: 0,
                error_trace: vec![],
                sql_state: None,
                query_id: None,
                location: snafu::Location::new("test", 0, 0),
            }),
            location: snafu::Location::new("test", 0, 0),
        };
        assert_eq!(odbc_err.to_sql_state(), SqlState::GeneralError);
    }

    #[test]
    fn server_generic_error_does_not_sniff_message_text() {
        // Regression guard: this code path used to upgrade GenericError
        // to a more specific SqlState by substring-matching the
        // human-readable message ("SQL compilation error",
        // "out of representable range", "too long and would be
        // truncated"). That heuristic is gone — when sf_core supplies
        // no SQLSTATE, the answer is HY000 regardless of message
        // content. Classification belongs to the server.
        let sniffable_messages = [
            "SQL compilation error: invalid identifier 'X'",
            "Number out of representable range: type FIXED[SB2](3,0), value 99999",
            "String 'hello world' is too long and would be truncated",
        ];
        for message in sniffable_messages {
            let odbc_err = OdbcError::CoreError {
                source: Box::new(CoreProtobufError::Application {
                    error: Box::new(ErrorType::GenericError(
                        sf_core::protobuf::generated::database_driver_v1::GenericError {},
                    )),
                    message: message.to_string(),
                    status_code: 0,
                    error_trace: vec![],
                    sql_state: None,
                    query_id: None,
                    location: snafu::Location::new("test", 0, 0),
                }),
                location: snafu::Location::new("test", 0, 0),
            };
            assert_eq!(
                odbc_err.to_sql_state(),
                SqlState::GeneralError,
                "message {message:?} must not be sniffed for SQLSTATE",
            );
        }
    }

    #[test]
    fn server_unknown_sql_state_passes_through_verbatim() {
        // When the server sends a SQLSTATE that's not in our enum (here
        // "22000", the generic data-exception class that Snowflake returns
        // for some bind-time truncation failures), we forward it as-is
        // rather than masking it with a more-specific reclassification or
        // collapsing it to HY000.
        let odbc_err = OdbcError::CoreError {
            source: Box::new(CoreProtobufError::Application {
                error: Box::new(ErrorType::GenericError(
                    sf_core::protobuf::generated::database_driver_v1::GenericError {},
                )),
                message: "String 'hello world' is too long and would be truncated".to_string(),
                status_code: 0,
                error_trace: vec![],
                sql_state: Some("22000".to_string()),
                query_id: None,
                location: snafu::Location::new("test", 0, 0),
            }),
            location: snafu::Location::new("test", 0, 0),
        };
        assert_eq!(
            odbc_err.to_sql_state(),
            SqlState::Unknown("22000".to_string())
        );
        assert_eq!(odbc_err.to_sql_state().as_str(), "22000");
    }

    #[test]
    fn server_malformed_sql_state_falls_back_to_default() {
        // Anything outside `[0-9A-Z]{5}` is malformed: lowercase letters,
        // punctuation, padding, non-ASCII, wrong length. None of those may
        // be forwarded verbatim — the driver must fall back to the
        // per-error-type default (HY000 for GenericError) rather than
        // emitting a non-conforming SQLSTATE in diagnostics.
        let malformed = [
            "22a01",  // lowercase letter
            "22 01",  // embedded space
            "22-01",  // punctuation
            "2201",   // too short
            "220011", // too long
            "22ą01",  // non-ASCII
        ];
        for state in malformed {
            let odbc_err = OdbcError::CoreError {
                source: Box::new(CoreProtobufError::Application {
                    error: Box::new(ErrorType::GenericError(
                        sf_core::protobuf::generated::database_driver_v1::GenericError {},
                    )),
                    message: "boom".to_string(),
                    status_code: 0,
                    error_trace: vec![],
                    sql_state: Some(state.to_string()),
                    query_id: None,
                    location: snafu::Location::new("test", 0, 0),
                }),
                location: snafu::Location::new("test", 0, 0),
            };
            assert_eq!(
                odbc_err.to_sql_state(),
                SqlState::GeneralError,
                "malformed SQLSTATE {state:?} must not be forwarded; expected HY000 fallback",
            );
        }
    }

    #[test]
    fn invalid_boolean_value_maps_to_22018() {
        let json_err = InvalidBooleanValueSnafu {
            value: "hello".to_string(),
        }
        .build();
        let odbc_err = OdbcError::JsonBinding {
            source: Box::new(json_err),
            location: snafu::Location::new("test", 0, 0),
        };
        assert_eq!(
            odbc_err.to_sql_state(),
            SqlState::InvalidCharacterValueForCast
        );
    }

    #[test]
    fn invalid_numeric_literal_maps_to_22018() {
        let json_err = InvalidNumericLiteralSnafu {
            reason: "non-finite literal \"Infinity\"".to_string(),
        }
        .build();
        let odbc_err = OdbcError::JsonBinding {
            source: Box::new(json_err),
            location: snafu::Location::new("test", 0, 0),
        };
        assert_eq!(
            odbc_err.to_sql_state(),
            SqlState::InvalidCharacterValueForCast
        );
    }

    #[test]
    fn server_truncation_error_maps_to_22001() {
        // `sf_core::extract_vendor_info` populates `sql_state` from
        // Snowflake error code 100078 → "22001"; the ODBC layer trusts
        // that value without inspecting the human-readable message.
        let odbc_err = OdbcError::CoreError {
            source: Box::new(CoreProtobufError::Application {
                error: Box::new(ErrorType::GenericError(
                    sf_core::protobuf::generated::database_driver_v1::GenericError {},
                )),
                message: "String 'hello world' is too long and would be truncated".to_string(),
                status_code: 0,
                error_trace: vec![],
                sql_state: Some("22001".to_string()),
                query_id: None,
                location: snafu::Location::new("test", 0, 0),
            }),
            location: snafu::Location::new("test", 0, 0),
        };
        assert_eq!(odbc_err.to_sql_state(), SqlState::StringDataRightTruncation);
    }

    #[test]
    fn binding_numeric_out_of_range_maps_to_22003() {
        let json_err = BindingNumericOutOfRangeSnafu {
            reason: "SQL_C_BINARY buffer length 12 does not match SQL_DATE_STRUCT size (6)"
                .to_string(),
        }
        .build();
        let odbc_err = OdbcError::JsonBinding {
            source: Box::new(json_err),
            location: snafu::Location::new("test", 0, 0),
        };
        assert_eq!(odbc_err.to_sql_state(), SqlState::NumericValueOutOfRange);
    }

    #[test]
    fn unsupported_c_data_type_maps_to_07006() {
        let json_err = UnsupportedCDataTypeSnafu {
            c_type: crate::api::CDataType::Char,
        }
        .build();
        let odbc_err = OdbcError::JsonBinding {
            source: Box::new(json_err),
            location: snafu::Location::new("test", 0, 0),
        };
        assert_eq!(
            odbc_err.to_sql_state(),
            SqlState::RestrictedDataTypeAttributeViolation
        );
    }

    #[test]
    fn unsupported_parameter_type_maps_to_07006() {
        // Per the MS ODBC spec, "Restricted data type attribute violation"
        // (07006) is the right code when `ParameterType` is a valid driver
        // SQL type for which no conversion from the supplied `ValueType` is
        // available -- for example binding any C type to
        // `SQL_SF_TIMESTAMP_TZ` (2001), which the new driver does not yet
        // support.
        let json_err = UnsupportedParameterTypeSnafu {
            sql_type: odbc_sys::SqlDataType(2001),
        }
        .build();
        let odbc_err = OdbcError::JsonBinding {
            source: Box::new(json_err),
            location: snafu::Location::new("test", 0, 0),
        };
        assert_eq!(
            odbc_err.to_sql_state(),
            SqlState::RestrictedDataTypeAttributeViolation
        );
    }

    /// Pins the SQLSTATE for the "string didn't match the expected format
    /// for the SQL target" path -- 22018, the same class as
    /// `InvalidNumericLiteral` / `InvalidBooleanValue`. Used by
    /// `parse_tz_string_with_fallback` for SQL_C_CHAR / SQL_C_WCHAR bound
    /// to SQL_SF_TIMESTAMP_TZ when the input lacks both an offset suffix
    /// and a parseable offset-less shape. See PR #1005 review on
    /// `timestamp.rs:643`.
    #[test]
    fn invalid_character_value_for_cast_maps_to_22018() {
        let json_err = InvalidCharacterValueForCastSnafu {
            c_type: crate::api::CDataType::Char,
            value: "not-a-timestamp".to_string(),
            expected_format: "YYYY-MM-DD HH:MM:SS[.fff] +/-HH:MM",
        }
        .build();
        let odbc_err = OdbcError::JsonBinding {
            source: Box::new(json_err),
            location: snafu::Location::new("test", 0, 0),
        };
        assert_eq!(
            odbc_err.to_sql_state(),
            SqlState::InvalidCharacterValueForCast
        );
    }

    /// Pins the SQLSTATE for "the parsed datetime overflowed the wire
    /// format's representable range" -- 22008, distinct from 22018 (parse
    /// failure on the input string) and 07006 (the binding shape itself
    /// was wrong). Triggered by `write_timestamp_tz_wire` when
    /// `timestamp_nanos_opt()` returns `None`. See PR #1005 review on
    /// `timestamp.rs:643`.
    #[test]
    fn datetime_field_overflow_maps_to_22008() {
        let json_err = DatetimeFieldOverflowSnafu {
            reason: "TIMESTAMP_TZ UTC instant 9999-12-31 23:59:59 exceeds the i64 \
                     nanosecond epoch range supported by the wire format"
                .to_string(),
        }
        .build();
        let odbc_err = OdbcError::JsonBinding {
            source: Box::new(json_err),
            location: snafu::Location::new("test", 0, 0),
        };
        assert_eq!(odbc_err.to_sql_state(), SqlState::DatetimeFieldOverflow);
    }

    /// SNOW-3235557: the attribute get paths classify unknown identifiers into
    /// HYC00 (valid ODBC attribute, unsupported) vs HY092 (out-of-range id).
    /// Pin the variant→SQLSTATE mapping the classifier depends on, plus the
    /// read-only set rejection (also HY092).
    #[test]
    fn attribute_errors_map_to_expected_sql_states() {
        assert_eq!(
            OdbcError::UnsupportedAttribute {
                attribute: 16,
                location: loc(),
            }
            .to_sql_state(),
            SqlState::OptionalFeatureNotImplemented, // HYC00
        );
        assert_eq!(
            OdbcError::UnknownAttribute {
                attribute: 99999,
                location: loc(),
            }
            .to_sql_state(),
            SqlState::InvalidAttributeOptionIdentifier, // HY092
        );
        assert_eq!(
            OdbcError::ReadOnlyAttribute {
                attribute: 1209,
                location: loc(),
            }
            .to_sql_state(),
            SqlState::InvalidAttributeOptionIdentifier, // HY092
        );
    }
}
