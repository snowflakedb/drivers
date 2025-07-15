#[derive(Debug)]
pub enum RestError {
    MissingParameter(String),
    InvalidArgument(String),
    Internal(String),
    Status(reqwest::StatusCode),
}

impl std::fmt::Display for RestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestError::MissingParameter(s) => write!(f, "Missing parameter: {s}"),
            RestError::InvalidArgument(s) => write!(f, "Invalid argument: {s}"),
            RestError::Internal(s) => write!(f, "Internal error: {s}"),
            RestError::Status(s) => write!(f, "Status: {s}"),
        }
    }
}

impl std::error::Error for RestError {}
