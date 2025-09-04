use std::{collections::HashSet, str::Utf8Error, string::FromUtf8Error};

use crate::{
    api::{SqlState, diagnostic::DiagnosticRecord},
    read_arrow::ExtractError,
    write_arrow::ArrowBindingError,
};
use arrow::error::ArrowError;
use lazy_static::lazy_static;
use odbc_sys as sql;
use sf_core::thrift_gen::database_driver_v1::{
    DriverError, DriverException, InvalidParameterValue, LoginError, MissingParameter, StatusCode,
};
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

    #[snafu(display("[Core] {message}\n report: {report}"))]
    ThriftDriverException {
        message: String,
        report: String,
        status_code: StatusCode,
        error: Box<DriverError>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Thrift communication error"))]
    ThriftCommunication {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
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
                    DriverError::AuthError(_) => SqlState::InvalidAuthorizationSpecification,
                    DriverError::GenericError(_) => SqlState::GeneralError,
                    DriverError::InvalidParameterValue(InvalidParameterValue {
                        parameter, ..
                    }) => {
                        if AUTHENTICATOR_PARAMETERS.contains(&parameter.to_uppercase()) {
                            SqlState::InvalidAuthorizationSpecification
                        } else {
                            SqlState::InvalidConnectionStringAttribute
                        }
                    }
                    DriverError::MissingParameter(MissingParameter { parameter }) => {
                        if AUTHENTICATOR_PARAMETERS.contains(&parameter.to_uppercase()) {
                            SqlState::InvalidAuthorizationSpecification
                        } else {
                            SqlState::InvalidConnectionStringAttribute
                        }
                    }
                    DriverError::InternalError(_) => SqlState::GeneralError,
                    DriverError::LoginError(_) => SqlState::InvalidAuthorizationSpecification,
                }
            }
            OdbcError::ArrowBinding { .. } => SqlState::GeneralError,
            OdbcError::ThriftCommunication { .. } => SqlState::ClientUnableToEstablishConnection,
        }
    }

    pub fn to_native_error(&self) -> sql::Integer {
        match self {
            OdbcError::ThriftDriverException { error, .. } => match *error.clone() {
                DriverError::LoginError(LoginError { code, .. }) => code,
                _ => 0,
            },
            _ => 0,
        }
    }

    /// Convert a thrift::Error into the appropriate OdbcError variant
    #[track_caller]
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
}
