use crate::read_arrow::ExtractError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OdbcError {
    #[error("Connection is disconnected")]
    Disconnected,

    #[error("Invalid handle")]
    InvalidHandle,

    #[error("Unknown attribute: {0}")]
    UnknownAttribute(i32),

    #[error("Parameter number cannot be 0")]
    InvalidParameterNumber,

    #[error("Statement not executed")]
    StatementNotExecuted,

    #[error("Data not fetched yet")]
    DataNotFetched,

    #[error("Statement execution is done")]
    ExecutionDone,

    #[error("No more data available")]
    NoMoreData,

    #[error("Failed to parse port '{port}': {source}")]
    InvalidPort {
        port: String,
        source: std::num::ParseIntError,
    },

    #[error("Failed to set SQL query: {0}")]
    SetSqlQuery(String),

    #[error("Failed to prepare statement: {0}")]
    PrepareStatement(String),

    #[error("Failed to execute statement: {0}")]
    ExecuteStatement(String),

    #[error("Failed to bind parameters: {0}")]
    BindParameters(String),

    #[error("Connection initialization failed: {0}")]
    ConnectionInit(String),

    #[error("Error reading arrow value: {0:?}")]
    ArrowRead(ExtractError),

    #[error("Error binding parameters: {0}")]
    ParameterBinding(String),

    #[error("Error fetching data: {0}")]
    FetchData(String),

    #[error("Text conversion error: {0}")]
    TextConversion(String),
}

impl From<ExtractError> for OdbcError {
    fn from(e: ExtractError) -> Self {
        OdbcError::ArrowRead(e)
    }
}
