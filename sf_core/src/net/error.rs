//! Common error types for network operations.

use snafu::{Location, Snafu};
use std::io;

/// Error type for TCP connection operations.
#[derive(Debug, Snafu)]
pub enum TcpConnectorError {
    #[snafu(display("DNS resolution failed for {host}: {message}"))]
    DnsResolution {
        host: String,
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Connection failed to {host}:{port}: {message}"))]
    ConnectionFailed {
        host: String,
        port: u16,
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Connection timed out to {host}:{port}"))]
    Timeout {
        host: String,
        port: u16,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("I/O error: {message}"))]
    Io {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
}

impl From<io::Error> for TcpConnectorError {
    fn from(err: io::Error) -> Self {
        TcpConnectorError::Io {
            message: err.to_string(),
            location: Location::new(file!(), line!(), column!()),
        }
    }
}
