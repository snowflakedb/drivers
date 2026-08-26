#![cfg_attr(not(test), expect(dead_code))]

use arrow::array::{Array, ArrayRef};
use arrow::datatypes::DataType;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyNone;

use crate::arrow::plan::SnowflakeFieldType;

pub(super) fn logical_mismatch_err(field_type: &SnowflakeFieldType, actual: &DataType) -> PyErr {
    PyValueError::new_err(format!(
        "logical/physical type mismatch: logical {}, got {actual:?}",
        field_type.logical_type_name()
    ))
}

pub(super) fn downcast_column<T: Array + Clone + 'static>(
    array: &ArrayRef,
    field_type: &SnowflakeFieldType,
) -> PyResult<T> {
    array
        .as_any()
        .downcast_ref::<T>()
        .cloned()
        .ok_or_else(|| logical_mismatch_err(field_type, array.data_type()))
}

#[inline]
pub(super) fn py_none(py: Python<'_>) -> Bound<'_, PyAny> {
    PyNone::get(py).to_owned().into_any()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{ArrayRef, Int32Array, Int64Array};
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

    use super::downcast_column;
    use crate::arrow::plan::SnowflakeFieldType;

    #[test]
    fn downcast_column_accepts_matching_physical_type() {
        Python::initialize();
        let array: ArrayRef = Arc::new(Int32Array::from(vec![Some(7)]));
        let field_type = SnowflakeFieldType::Number {
            scale: 0,
            precision: 38,
        };
        let got = downcast_column::<Int32Array>(&array, &field_type).unwrap();
        assert_eq!(got.value(0), 7);
    }

    #[test]
    fn downcast_column_rejects_physical_mismatch() {
        Python::initialize();
        let array: ArrayRef = Arc::new(Int64Array::from(vec![Some(7)]));
        let field_type = SnowflakeFieldType::Number {
            scale: 0,
            precision: 38,
        };
        let err = downcast_column::<Int32Array>(&array, &field_type).unwrap_err();
        Python::attach(|py| {
            assert!(
                err.is_instance_of::<PyValueError>(py),
                "expected PyValueError, got {err}"
            );
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(
                text.contains("logical/physical type mismatch")
                    && text.contains("Int64")
                    && text.contains("FIXED"),
                "got {text}"
            );
        });
    }
}
