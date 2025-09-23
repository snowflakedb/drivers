use std::{collections::HashSet, str::Utf8Error, string::FromUtf8Error};

use crate::{
    api::{SqlState, diagnostic::DiagnosticRecord},
    read_arrow::ExtractError,
    write_arrow::ArrowBindingError,
};
use arrow::error::ArrowError;
use lazy_static::lazy_static;
use odbc_sys as sql;
use proto_utils::ProtoError;
use sf_core::{
    protobuf_gen::database_driver_v1::{
        GenericError, InvalidParameterValue as ProtoInvalidParameterValue,
        LoginError as ProtoLoginError, MissingParameter as ProtoMissingParameter,
        driver_error::ErrorType,
    },
    thrift_gen::database_driver_v1::{
        DriverError as ThriftDriverError, DriverException, InvalidParameterValue, LoginError,
        MissingParameter, StatusCode,
    },
};

use sf_core::protobuf_gen::database_driver_v1::DriverException as ProtoDriverException;
use snafu::{Location, Snafu, location};

#[derive(Snafu, Debug)]
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

    #[snafu(display("Invalid record number: {number}"))]
    InvalidRecordNumber {
        number: sql::SmallInt,
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

    #[snafu(display("Statement execution is done"))]
    ExecutionDone {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("No more data available"))]
    NoMoreData {
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
    ArrowRead {
        source: ExtractError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error binding arrow parameters: {source:?}"))]
    ArrowBinding {
        source: ArrowBindingError,
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

    #[snafu(display("[Core] {message}\n report: {report}"))]
    ThriftDriverException {
        message: String,
        report: String,
        status_code: StatusCode,
        error: Box<ThriftDriverError>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Thrift communication error"))]
    ThriftCommunication {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("[Core] {message}\n report: {report}"))]
    ProtoDriverException {
        message: String,
        report: String,
        status_code: i32,
        error: Box<ErrorType>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("[Core] Protocol transport error: {message}"))]
    ProtoTransport {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("[Core] Required field missing: {message}"))]
    ProtoRequiredFieldMissing {
        message: String,
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

lazy_static! {
    static ref AUTHENTICATOR_PARAMETERS: HashSet<String> = {
        let mut set = HashSet::new();
        set.insert("PRIV_KEY_FILE".to_string());
        set.insert("PRIVATE_KEY_FILE".to_string());
        set.insert("PRIV_KEY_FILE_PWD".to_string());
        set.insert("TOKEN".to_string());
        set.insert("AUTHENTICATOR".to_string());
        set.insert("USER".to_string());
        set.insert("PASSWORD".to_string());
        set
    };
}

impl OdbcError {
    pub fn to_diagnostic_record(&self) -> DiagnosticRecord {
        DiagnosticRecord {
            message_text: self.to_string(),
            sql_state: self.to_sql_state(),
            native_error: self.to_native_error(),
            ..Default::default()
        }
    }

    pub fn to_sql_state(&self) -> SqlState {
        match self {
            OdbcError::Disconnected { .. } => SqlState::ConnectionDoesNotExist,
            OdbcError::InvalidHandle { .. } => SqlState::InvalidConnectionName,
            OdbcError::InvalidRecordNumber { .. } => SqlState::InvalidDescriptorIndex,
            OdbcError::InvalidDiagnosticIdentifier { .. } => {
                SqlState::InvalidDescriptorFieldIdentifier
            }
            OdbcError::UnknownAttribute { .. } => SqlState::GeneralError,
            OdbcError::InvalidParameterNumber { .. } => SqlState::WrongNumberOfParameters,
            OdbcError::StatementNotExecuted { .. } => SqlState::FunctionSequenceError,
            OdbcError::DataNotFetched { .. } => SqlState::FunctionSequenceError,
            OdbcError::ExecutionDone { .. } => SqlState::FunctionSequenceError,
            OdbcError::NoMoreData { .. } => SqlState::NoDataFound,
            OdbcError::InvalidPort { .. } => SqlState::InvalidConnectionStringAttribute,
            OdbcError::SetSqlQuery { .. } => SqlState::SyntaxErrorOrAccessRuleViolation,
            OdbcError::PrepareStatement { .. } => SqlState::SyntaxErrorOrAccessRuleViolation,
            OdbcError::ExecuteStatement { .. } => SqlState::GeneralError,
            OdbcError::BindParameters { .. } => SqlState::WrongNumberOfParameters,
            OdbcError::ConnectionInit { .. } => SqlState::ClientUnableToEstablishConnection,
            OdbcError::ArrowRead { .. } => SqlState::GeneralError,
            OdbcError::ParameterBinding { .. } => SqlState::WrongNumberOfParameters,
            OdbcError::FetchData { .. } => SqlState::GeneralError,
            OdbcError::TextConversionUtf8 { .. } => SqlState::StringDataRightTruncated,
            OdbcError::TextConversionFromUtf8 { .. } => SqlState::StringDataRightTruncated,
            OdbcError::ThriftDriverException { error, .. } => {
                // Map DriverException StatusCode to appropriate SQL states
                match *error.clone() {
                    ThriftDriverError::AuthError(_) => SqlState::InvalidAuthorizationSpecification,
                    ThriftDriverError::GenericError(_) => SqlState::GeneralError,
                    ThriftDriverError::InvalidParameterValue(InvalidParameterValue {
                        parameter,
                        ..
                    }) => {
                        if AUTHENTICATOR_PARAMETERS.contains(&parameter.to_uppercase()) {
                            SqlState::InvalidAuthorizationSpecification
                        } else {
                            SqlState::InvalidConnectionStringAttribute
                        }
                    }
                    ThriftDriverError::MissingParameter(MissingParameter { parameter }) => {
                        if AUTHENTICATOR_PARAMETERS.contains(&parameter.to_uppercase()) {
                            SqlState::InvalidAuthorizationSpecification
                        } else {
                            SqlState::InvalidConnectionStringAttribute
                        }
                    }
                    ThriftDriverError::InternalError(_) => SqlState::GeneralError,
                    ThriftDriverError::LoginError(_) => SqlState::InvalidAuthorizationSpecification,
                }
            }
            OdbcError::ArrowBinding { .. } => SqlState::GeneralError,
            OdbcError::ThriftCommunication { .. } => SqlState::ClientUnableToEstablishConnection,
            OdbcError::ProtoDriverException { error, .. } => match *error.clone() {
                ErrorType::AuthError(_) => SqlState::InvalidAuthorizationSpecification,
                ErrorType::GenericError(_) => SqlState::GeneralError,
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
                ErrorType::InternalError(_) => SqlState::GeneralError,
                ErrorType::LoginError(_) => SqlState::InvalidAuthorizationSpecification,
            },
            OdbcError::ProtoTransport { .. } => SqlState::ClientUnableToEstablishConnection,
            OdbcError::ProtoRequiredFieldMissing { .. } => SqlState::GeneralError,
            OdbcError::ArrowArrayStreamReaderCreation { .. } => SqlState::GeneralError,
            OdbcError::StatementErrorState { .. } => SqlState::GeneralError,
        }
    }

    pub fn to_native_error(&self) -> sql::Integer {
        match self {
            OdbcError::ThriftDriverException { error, .. } => match *error.clone() {
                ThriftDriverError::LoginError(LoginError { code, .. }) => code,
                _ => 0,
            },
            OdbcError::ProtoDriverException { error, .. } => match *error.clone() {
                ErrorType::LoginError(ProtoLoginError { code, .. }) => code,
                _ => 0,
            },
            _ => 0,
        }
    }

    /// Convert a thrift::Error into the appropriate OdbcError variant
    #[track_caller]
    #[allow(dead_code)]
    pub fn from_thrift_error(error: thrift::Error) -> Self {
        match error {
            thrift::Error::User(boxed_error) => {
                // Try to downcast to DriverException
                if let Some(driver_exception) = boxed_error.downcast_ref::<DriverException>() {
                    OdbcError::ThriftDriverException {
                        message: driver_exception.message.clone(),
                        status_code: driver_exception.status_code,
                        error: Box::new(driver_exception.error.clone()),
                        location: location!(),
                        report: driver_exception.report.clone(),
                    }
                } else {
                    ThriftCommunicationSnafu {
                        message: boxed_error.to_string(),
                    }
                    .build()
                }
            }
            err => ThriftCommunicationSnafu {
                message: err.to_string(),
            }
            .build(),
        }
    }

    #[track_caller]
    pub fn from_protobuf_error(error: ProtoError<ProtoDriverException>) -> OdbcError {
        let location = location!();
        match error {
            ProtoError::Application(driver_exception) => OdbcError::ProtoDriverException {
                message: driver_exception.message,
                status_code: driver_exception.status_code,
                error: Box::new(
                    driver_exception
                        .error
                        .and_then(|error| error.error_type)
                        .unwrap_or(ErrorType::GenericError(GenericError {})),
                ),
                location,
                report: driver_exception.report,
            },
            ProtoError::Transport(message) => OdbcError::ProtoTransport { message, location },
        }
    }
}

impl From<ProtoError<ProtoDriverException>> for OdbcError {
    #[track_caller]
    fn from(error: ProtoError<ProtoDriverException>) -> Self {
        OdbcError::from_protobuf_error(error)
    }
}
