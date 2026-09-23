use std::sync::Arc;

use arrow::array::{ArrayRef, Float64Array};
use pyo3::prelude::*;
use pyo3::types::PyFloat;
use sf_types::SnowflakeReal;

use super::Column;
use super::decode::{PyMaterializer, TypedColumn};
use super::numpy::NumpyProvider;
use crate::arrow::converters::util::downcast_column;
use crate::arrow::plan::SnowflakeFieldType;

pub(super) fn from_column(
    array: &ArrayRef,
    field_type: &SnowflakeFieldType,
    numpy: Arc<NumpyProvider>,
    use_numpy: bool,
) -> PyResult<Column> {
    let array = downcast_column::<Float64Array>(array, field_type)?;
    if use_numpy {
        Ok(Column::RealNumpy(TypedColumn::new(
            array,
            SnowflakeReal,
            RealNumpyMaterializer { numpy },
        )))
    } else {
        Ok(Column::Real(TypedColumn::new(
            array,
            SnowflakeReal,
            RealMaterializer,
        )))
    }
}

pub(crate) struct RealMaterializer;

impl PyMaterializer<SnowflakeReal> for RealMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, value: f64) -> PyResult<Bound<'py, PyAny>> {
        Ok(PyFloat::new(py, value).into_any())
    }
}

pub(crate) struct RealNumpyMaterializer {
    numpy: Arc<NumpyProvider>,
}

impl PyMaterializer<SnowflakeReal> for RealNumpyMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, value: f64) -> PyResult<Bound<'py, PyAny>> {
        self.numpy.float64(py, value)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{ArrayRef, Float64Array};
    use arrow::datatypes::Schema;
    use pyo3::prelude::*;

    use crate::arrow::converters::ConversionContext;
    use crate::arrow::converters::test_util::{assert_np_float64, assert_py_float, assert_py_none};
    use crate::arrow::plan::SnowflakeFieldType;

    #[test]
    fn real_converts_to_python_float() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let array: ArrayRef = Arc::new(Float64Array::from(vec![Some(1.5), None, Some(0.0)]));
        let column = ctx
            .converter_from_column(&array, &SnowflakeFieldType::Real)
            .unwrap();

        Python::attach(|py| {
            assert_py_float(&column.to_py(py, 0).unwrap(), 1.5);
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_py_float(&column.to_py(py, 2).unwrap(), 0.0);
        });
    }

    #[test]
    fn real_converts_to_numpy_float64_with_specials() {
        Python::initialize();
        let context = ConversionContext::with_numpy(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Float64Array::from(vec![
            Some(1.5),
            None,
            Some(f64::NAN),
            Some(f64::INFINITY),
            Some(f64::NEG_INFINITY),
        ]));
        let column = context
            .converter_from_column(&array, &SnowflakeFieldType::Real)
            .unwrap();

        Python::attach(|py| {
            assert_np_float64(&column.to_py(py, 0).unwrap(), 1.5);
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_np_float64(&column.to_py(py, 2).unwrap(), f64::NAN);
            assert_np_float64(&column.to_py(py, 3).unwrap(), f64::INFINITY);
            assert_np_float64(&column.to_py(py, 4).unwrap(), f64::NEG_INFINITY);
        });
    }
}
