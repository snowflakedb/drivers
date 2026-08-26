//! Adapter from [`odbc_decode`](crate::arrow::odbc_decode) to Python cells.
//!
//! A decoder produces an intermediate Rust representation; a
//! [`PyMaterializer`] turns that value into a Python object. Null → `None`
//! and exception mapping live here, not in `odbc_decode`.
//!
//! Production callers are `Column` variants; those land with each converter.

#![cfg_attr(not(test), expect(dead_code))]

use arrow::array::Array;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use super::util::py_none;
use crate::arrow::odbc_decode::{DecodeError, ReadArrowType, SnowflakeType};

pub(crate) trait PyMaterializer<D: SnowflakeType>: Send + Sync {
    fn materialize<'py>(
        &self,
        py: Python<'py>,
        value: D::Representation<'_>,
    ) -> PyResult<Bound<'py, PyAny>>;
}

pub(crate) struct TypedColumn<A, D, M> {
    array: A,
    decoder: D,
    materializer: M,
}

impl<A, D, M> TypedColumn<A, D, M> {
    pub(super) fn new(array: A, decoder: D, materializer: M) -> Self {
        Self {
            array,
            decoder,
            materializer,
        }
    }
}

impl<A, D, M> TypedColumn<A, D, M>
where
    A: Array,
    D: ReadArrowType<A>,
    M: PyMaterializer<D>,
{
    pub(super) fn to_py<'py>(&self, py: Python<'py>, row: usize) -> PyResult<Bound<'py, PyAny>> {
        match decode_optional(&self.decoder, &self.array, row)? {
            None => Ok(py_none(py)),
            Some(value) => self.materializer.materialize(py, value),
        }
    }
}

/// Read one cell, mapping a decode-layer null to `None`.
fn decode_optional<'a, T, A>(
    decoder: &T,
    array: &'a A,
    row: usize,
) -> PyResult<Option<T::Representation<'a>>>
where
    T: ReadArrowType<A>,
{
    match decoder.read_arrow_type(array, row) {
        Ok(value) => Ok(Some(value)),
        Err(DecodeError::NullValue { .. }) => Ok(None),
        Err(err) => Err(PyValueError::new_err(err.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use arrow::array::{Array, Int32Array};
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;
    use pyo3::types::PyInt;

    use super::{PyMaterializer, TypedColumn, decode_optional};
    use crate::arrow::converters::test_util::assert_py_none;
    use crate::arrow::odbc_decode::{
        DecodeError, InvalidArrowValueSnafu, NullValueSnafu, ReadArrowType, SnowflakeType,
    };

    /// Test double — not a production datatype reader.
    struct Probe;

    impl SnowflakeType for Probe {
        type Representation<'a> = i32;
    }

    impl ReadArrowType<Int32Array> for Probe {
        fn read_arrow_type(&self, array: &Int32Array, row_idx: usize) -> Result<i32, DecodeError> {
            if array.is_null(row_idx) {
                return NullValueSnafu.fail();
            }
            let value = array.value(row_idx);
            if value == i32::MIN {
                return InvalidArrowValueSnafu { reason: "sentinel" }.fail();
            }
            Ok(value)
        }
    }

    struct ProbeMaterializer;

    impl PyMaterializer<Probe> for ProbeMaterializer {
        fn materialize<'py>(&self, py: Python<'py>, value: i32) -> PyResult<Bound<'py, PyAny>> {
            Ok(value.into_pyobject(py)?.into_any())
        }
    }

    fn probe_column() -> TypedColumn<Int32Array, Probe, ProbeMaterializer> {
        TypedColumn::new(
            Int32Array::from(vec![Some(7), None, Some(i32::MIN)]),
            Probe,
            ProbeMaterializer,
        )
    }

    #[test]
    fn decode_optional_maps_present_null_and_invalid() {
        Python::initialize();
        let array = Int32Array::from(vec![Some(7), None, Some(i32::MIN)]);

        assert_eq!(decode_optional(&Probe, &array, 0).unwrap(), Some(7));
        assert_eq!(decode_optional(&Probe, &array, 1).unwrap(), None);

        let err = decode_optional(&Probe, &array, 2).unwrap_err();
        Python::attach(|py| {
            assert!(
                err.is_instance_of::<PyValueError>(py),
                "expected PyValueError, got {err}"
            );
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(text.contains("Invalid Arrow value: sentinel"), "got {text}");
        });
    }

    #[test]
    fn typed_column_materializes_present_null_and_invalid() {
        Python::initialize();
        let column = probe_column();

        Python::attach(|py| {
            let present = column.to_py(py, 0).unwrap();
            assert!(
                present.is_instance_of::<PyInt>(),
                "expected Python int, got {}",
                present.get_type().name().unwrap()
            );
            assert_eq!(present.extract::<i32>().unwrap(), 7);

            assert_py_none(&column.to_py(py, 1).unwrap());

            let err = column.to_py(py, 2).unwrap_err();
            assert!(
                err.is_instance_of::<PyValueError>(py),
                "expected PyValueError, got {err}"
            );
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(text.contains("Invalid Arrow value: sentinel"), "got {text}");
        });
    }
}
