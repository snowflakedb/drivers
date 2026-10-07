use error_trace::ErrorTrace;
use pyo3::exceptions::{PyRuntimeError, PyStopAsyncIteration, PyStopIteration, PyValueError};
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::PyType;
use snafu::{Location, Snafu};

const ER_FAILED_TO_CONVERT_ROW_TO_PYTHON_TYPE: i32 = 252005;

#[derive(Debug, Snafu, ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum StreamError {
    #[snafu(display("invalid ArrowArrayStream pointer: null"))]
    NullStreamPointer {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("invalid ArrowArrayStream pointer: release callback is null"))]
    StreamNotReleased {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("failed to create Arrow stream reader: {source}"))]
    ReaderCreate {
        source: arrow::error::ArrowError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("failed to read next record batch: {source}"))]
    BatchRead {
        source: arrow::error::ArrowError,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Snafu, ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum PlanError {
    #[snafu(display("missing logicalType metadata for column '{column}'"))]
    MissingLogicalType {
        column: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("unsupported logicalType '{logical_type}' for column '{column}'"))]
    UnsupportedLogicalType {
        logical_type: String,
        column: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("invalid {key} metadata '{value}'"))]
    InvalidMetadata {
        key: String,
        value: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "[Snowflake Exception] invalid scale value {scale} for {logical_type} column (expected 0-9)"
    ))]
    InvalidScale {
        scale: i32,
        logical_type: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("missing {key} metadata for column '{column}'"))]
    MissingMetadata {
        key: String,
        column: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("failed to convert Arrow column: {source}"))]
    ColumnConvert {
        source: arrow::error::ArrowError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "The total number of nanoseconds {epoch}{frac:09} overflows int64 range. If you use a timestamp with the nanosecond part over 6-digits in the Snowflake database, the timestamp must be between '1677-09-21 00:12:43.145224192' and '2262-04-11 23:47:16.854775807' to not overflow. Pass force_microsecond_precision=True to truncate to microseconds instead."
    ))]
    TimestampOverflow {
        epoch: i64,
        frac: i64,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("[Snowflake Exception] unknown byteLength({byte_length}) for TIMESTAMP_TZ"))]
    UnknownByteLength {
        byte_length: i32,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("[Snowflake Exception] unsupported Snowflake type: {logical_type}"))]
    UnsupportedSnowflakeType {
        logical_type: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("[Snowflake Exception] TIME value {value} does not fit in time32"))]
    TimeDoesNotFit {
        value: i64,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "[Snowflake Exception] nested Arrow type {data_type} is not supported for {logical_type} column"
    ))]
    NestedArrowType {
        logical_type: String,
        data_type: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "[Snowflake Exception] invalid arrow schema for map key/value: expected 2 entries, got {count}"
    ))]
    InvalidMapSchema {
        count: usize,
        #[snafu(implicit)]
        location: Location,
    },
}

impl From<StreamError> for PyErr {
    fn from(error: StreamError) -> Self {
        match error {
            e @ (StreamError::NullStreamPointer { .. } | StreamError::StreamNotReleased { .. }) => {
                PyValueError::new_err(e.to_string())
            }
            e @ (StreamError::ReaderCreate { .. } | StreamError::BatchRead { .. }) => {
                PyRuntimeError::new_err(e.to_string())
            }
        }
    }
}

impl From<PlanError> for PyErr {
    fn from(error: PlanError) -> Self {
        PyValueError::new_err(error.to_string())
    }
}

fn interface_error_type(py: Python<'_>) -> PyResult<&Bound<'_, PyType>> {
    static INTERFACE_ERROR: PyOnceLock<Py<PyType>> = PyOnceLock::new();
    INTERFACE_ERROR.import(py, "snowflake.connector.errors", "InterfaceError")
}

pub(crate) fn wrap_row_conversion(py: Python<'_>, err: PyErr) -> PyErr {
    wrap_conversion(py, err, "Failed to convert current row")
}

pub(crate) fn wrap_rows_conversion(py: Python<'_>, err: PyErr) -> PyErr {
    wrap_conversion(py, err, "Failed to convert rows")
}

fn wrap_conversion(py: Python<'_>, err: PyErr, prefix: &str) -> PyErr {
    if err.is_instance_of::<PyStopIteration>(py) || err.is_instance_of::<PyStopAsyncIteration>(py) {
        return err;
    }
    let Ok(cls) = interface_error_type(py) else {
        return err;
    };
    let cause = err
        .value(py)
        .str()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|_| err.to_string());
    let msg = format!("{prefix}, cause: {cause}");
    match cls.call1((msg, ER_FAILED_TO_CONVERT_ROW_TO_PYTHON_TYPE)) {
        Ok(exc) => PyErr::from_value(exc),
        Err(_) => err,
    }
}
