use error_trace::ErrorTrace;
use pyo3::exceptions::{PyRuntimeError, PyStopIteration, PyValueError};
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

    #[snafu(display("missing {key} metadata for column '{column}'"))]
    MissingMetadata {
        key: String,
        column: String,
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
    if err.is_instance_of::<PyStopIteration>(py) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use snafu::ResultExt;

    #[test]
    fn malformed_stream_maps_to_value_error() {
        Python::initialize();
        let err: PyErr = NullStreamPointerSnafu.build().into();
        Python::attach(|py| {
            assert!(err.is_instance_of::<PyValueError>(py));
            assert!(!err.is_instance_of::<PyRuntimeError>(py));
        });

        let err: PyErr = StreamNotReleasedSnafu.build().into();
        Python::attach(|py| {
            assert!(err.is_instance_of::<PyValueError>(py));
        });
    }

    #[test]
    fn reader_and_batch_failures_map_to_runtime_error() {
        Python::initialize();
        let arrow_err = arrow::error::ArrowError::ParseError("boom".into());
        let err: PyErr = Err::<(), _>(arrow_err)
            .context(ReaderCreateSnafu)
            .unwrap_err()
            .into();
        Python::attach(|py| {
            assert!(err.is_instance_of::<PyRuntimeError>(py));
            assert!(!err.is_instance_of::<PyValueError>(py));
        });

        let arrow_err = arrow::error::ArrowError::ParseError("boom".into());
        let err: PyErr = Err::<(), _>(arrow_err)
            .context(BatchReadSnafu)
            .unwrap_err()
            .into();
        Python::attach(|py| {
            assert!(err.is_instance_of::<PyRuntimeError>(py));
        });
    }

    #[test]
    fn stop_iteration_is_not_wrapped() {
        Python::initialize();
        Python::attach(|py| {
            let wrapped = wrap_row_conversion(py, PyStopIteration::new_err(()));
            assert!(
                wrapped.is_instance_of::<PyStopIteration>(py),
                "expected StopIteration, got {wrapped}"
            );
        });
    }

    #[test]
    fn conversion_wrap_is_interface_error_when_connector_importable() {
        Python::initialize();
        Python::attach(|py| {
            let Ok(errors) = py.import("snowflake.connector.errors") else {
                return;
            };
            let iface = errors.getattr("InterfaceError").unwrap();
            let wrapped = wrap_row_conversion(py, PyValueError::new_err("boom"));
            assert!(
                wrapped.is_instance(py, &iface),
                "expected InterfaceError, got {wrapped}"
            );
            let value = wrapped.value(py);
            let errno: i32 = value.getattr("errno").unwrap().extract().unwrap();
            assert_eq!(errno, ER_FAILED_TO_CONVERT_ROW_TO_PYTHON_TYPE);
            let text = value.str().unwrap().to_string_lossy().into_owned();
            assert!(
                text.contains("Failed to convert current row, cause: boom"),
                "got {text}"
            );
        });
    }

    #[test]
    fn bulk_wrap_uses_rows_wording_when_connector_importable() {
        Python::initialize();
        Python::attach(|py| {
            if py.import("snowflake.connector.errors").is_err() {
                return;
            }
            let wrapped = wrap_rows_conversion(py, PyValueError::new_err("boom"));
            let text = wrapped
                .value(py)
                .str()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            assert!(
                text.contains("Failed to convert rows, cause: boom"),
                "got {text}"
            );
        });
    }
}
