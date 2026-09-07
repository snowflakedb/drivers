use arrow::array::{ArrayRef, Float64Array};
use pyo3::prelude::*;
use pyo3::types::PyFloat;
use sf_types::SnowflakeReal;

use super::Column;
use super::decode::{PyMaterializer, TypedColumn};
use crate::arrow::converters::util::downcast_column;
use crate::arrow::plan::SnowflakeFieldType;

pub(super) fn from_column(array: &ArrayRef, field_type: &SnowflakeFieldType) -> PyResult<Column> {
    downcast_column::<Float64Array>(array, field_type)
        .map(|array| Column::Real(TypedColumn::new(array, SnowflakeReal, RealMaterializer)))
}

pub(crate) struct RealMaterializer;

impl PyMaterializer<SnowflakeReal> for RealMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, value: f64) -> PyResult<Bound<'py, PyAny>> {
        Ok(PyFloat::new(py, value).into_any())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{ArrayRef, Float64Array};
    use arrow::datatypes::Schema;
    use pyo3::prelude::*;

    use crate::arrow::converters::ConversionContext;
    use crate::arrow::converters::test_util::{assert_py_float, assert_py_none};
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
}
