use snafu::{prelude::*, Location};

use crate::model::TraceLog;

#[derive(Snafu, Debug)]
pub enum UnixOdbcParserError {
    #[snafu(display("unixODBC trace parser is not yet implemented"))]
    NotImplemented {
        #[snafu(implicit)]
        location: Location,
    },
}

type Result<T> = std::result::Result<T, UnixOdbcParserError>;

pub fn parse_file(_path: &std::path::Path) -> Result<TraceLog> {
    NotImplementedSnafu.fail()
}

pub fn parse_str(_content: &str) -> Result<TraceLog> {
    NotImplementedSnafu.fail()
}
