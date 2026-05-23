pub mod iodbc;
pub mod unixodbc;
pub mod winodbc;

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

    #[snafu(display("Failed to parse Windows ODBC DM trace"))]
    WinOdbc {
        source: winodbc::WinOdbcParserError,
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
    if winodbc::looks_like_winodbc(content) {
        return Some(TraceFormat::WinOdbc);
    }

    let first_line = content.lines().find(|l| !l.trim().is_empty())?;
    if first_line.starts_with("** iODBC Trace file") {
        Some(TraceFormat::IOdbc)
    } else if first_line.starts_with("[ODBC]") {
        Some(TraceFormat::UnixOdbc)
    } else {
        None
    }
}

pub fn parse_file_auto(path: &std::path::Path) -> Result<TraceLog> {
    let content = read_trace_file(path).context(FileReadSnafu)?;
    let format = detect_format(&content).context(UnknownFormatSnafu)?;
    parse_str(&content, format)
}

fn read_trace_file(path: &std::path::Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let (decoded, _, _) = encoding_rs::UTF_16LE.decode(&bytes[2..]);
        Ok(decoded.into_owned())
    } else {
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

pub fn parse_file(path: &std::path::Path, format: TraceFormat) -> Result<TraceLog> {
    match format {
        TraceFormat::IOdbc => iodbc::parse_file(path).context(IodbcSnafu),
        TraceFormat::UnixOdbc => unixodbc::parse_file(path).context(UnixOdbcSnafu),
        TraceFormat::WinOdbc => {
            let content = read_trace_file(path).context(FileReadSnafu)?;
            winodbc::parse_str(&content).context(WinOdbcSnafu)
        }
    }
}

pub fn parse_str(content: &str, format: TraceFormat) -> Result<TraceLog> {
    match format {
        TraceFormat::IOdbc => iodbc::parse_str(content).context(IodbcSnafu),
        TraceFormat::UnixOdbc => unixodbc::parse_str(content).context(UnixOdbcSnafu),
        TraceFormat::WinOdbc => winodbc::parse_str(content).context(WinOdbcSnafu),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNIXODBC_SNIPPET: &str = "\
[ODBC][100][1774615098.100000][__handles.c][499]
\t\tExit:[SQL_SUCCESS]
\t\t\tEnvironment = 0xaaa
";

    const IODBC_SNIPPET: &str = "\
** iODBC Trace file
** Started on Mon Jan 1 00:00:00 2024
";

    const WINODBC_SNIPPET: &str = "\
b6dccb0d-462e-4 3834-1704\tENTER SQLAllocHandle 
\t\tSQLSMALLINT                  2 <SQL_HANDLE_DBC>
\t\tSQLHANDLE           0x0000000000000000
\t\tSQLHANDLE *         0x0000018E00866BE0
";

    #[test]
    fn test_detect_format_unixodbc() {
        assert_eq!(detect_format(UNIXODBC_SNIPPET), Some(TraceFormat::UnixOdbc));
    }

    #[test]
    fn test_detect_format_iodbc() {
        assert_eq!(detect_format(IODBC_SNIPPET), Some(TraceFormat::IOdbc));
    }

    #[test]
    fn test_detect_format_winodbc() {
        assert_eq!(detect_format(WINODBC_SNIPPET), Some(TraceFormat::WinOdbc));
    }

    #[test]
    fn test_detect_format_winodbc_does_not_false_match_unixodbc() {
        // Regression: winodbc detection runs before unixodbc/iodbc, so it must
        // not accidentally claim a unixODBC trace.
        assert_ne!(detect_format(UNIXODBC_SNIPPET), Some(TraceFormat::WinOdbc));
    }

    #[test]
    fn test_detect_format_winodbc_does_not_false_match_iodbc() {
        assert_ne!(detect_format(IODBC_SNIPPET), Some(TraceFormat::WinOdbc));
    }

    #[test]
    fn test_detect_format_unknown() {
        assert!(detect_format("random text without any header").is_none());
    }

    #[test]
    fn test_read_trace_file_strips_utf16_le_bom() {
        // Build a UTF-16 LE BOM + a small winodbc-looking trace.
        let mut bytes = vec![0xFF, 0xFE]; // UTF-16 LE BOM
        for ch in WINODBC_SNIPPET.encode_utf16() {
            bytes.extend_from_slice(&ch.to_le_bytes());
        }
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("trace.LOG");
        std::fs::write(&path, &bytes).expect("write");

        let content = read_trace_file(&path).expect("read");
        assert!(
            content.starts_with("b6dccb0d-462e-4 3834-1704\tENTER SQLAllocHandle"),
            "BOM-stripped UTF-16 content should start with the header, got: {:?}",
            &content.get(..60),
        );
        assert_eq!(
            detect_format(&content),
            Some(TraceFormat::WinOdbc),
            "decoded UTF-16 content must still detect as WinOdbc",
        );
    }
}
