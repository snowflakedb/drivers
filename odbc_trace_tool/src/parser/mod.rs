pub mod iodbc;
pub mod unixodbc;

use snafu::{prelude::*, Location};

use crate::model::{TraceFormat, TraceLog};

#[derive(Snafu, Debug)]
pub enum ParserError {
    #[snafu(display("Failed to parse iODBC trace"))]
    Iodbc {
        source: iodbc::IodbcParserError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to parse unixODBC trace"))]
    UnixOdbc {
        source: unixodbc::UnixOdbcParserError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to read file for format detection"))]
    FileRead {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Could not detect trace format from file contents"))]
    UnknownFormat {
        #[snafu(implicit)]
        location: Location,
    },
}

type Result<T> = std::result::Result<T, ParserError>;

pub fn detect_format(content: &str) -> Option<TraceFormat> {
    let first_line = content.lines().next()?;
    if first_line.starts_with("** iODBC Trace file") {
        Some(TraceFormat::IOdbc)
    } else if first_line.starts_with("[ODBC]") {
        Some(TraceFormat::UnixOdbc)
    } else {
        None
    }
}

pub fn parse_file_auto(path: &std::path::Path) -> Result<TraceLog> {
    let content = std::fs::read_to_string(path).context(FileReadSnafu)?;
    let format = detect_format(&content).context(UnknownFormatSnafu)?;
    parse_str(&content, format)
}

pub fn parse_file(path: &std::path::Path, format: TraceFormat) -> Result<TraceLog> {
    match format {
        TraceFormat::IOdbc => iodbc::parse_file(path).context(IodbcSnafu),
        TraceFormat::UnixOdbc => unixodbc::parse_file(path).context(UnixOdbcSnafu),
    }
}

pub fn parse_str(content: &str, format: TraceFormat) -> Result<TraceLog> {
    match format {
        TraceFormat::IOdbc => iodbc::parse_str(content).context(IodbcSnafu),
        TraceFormat::UnixOdbc => unixodbc::parse_str(content).context(UnixOdbcSnafu),
    }
}
