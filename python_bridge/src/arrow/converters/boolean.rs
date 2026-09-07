use arrow::array::{ArrayRef, BooleanArray};
use pyo3::prelude::*;
use pyo3::types::PyBool;
use sf_types::SnowflakeBoolean;

use super::Column;
use super::decode::{PyMaterializer, TypedColumn};
use crate::arrow::converters::util::downcast_column;
use crate::arrow::plan::SnowflakeFieldType;

pub(super) fn from_column(array: &ArrayRef, field_type: &SnowflakeFieldType) -> PyResult<Column> {
    downcast_column::<BooleanArray>(array, field_type)
        .map(|array| Column::Bool(TypedColumn::new(array, SnowflakeBoolean, BoolMaterializer)))
}

pub(crate) struct BoolMaterializer;

impl PyMaterializer<SnowflakeBoolean> for BoolMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, value: bool) -> PyResult<Bound<'py, PyAny>> {
        Ok(PyBool::new(py, value).to_owned().into_any())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{ArrayRef, BooleanArray};
    use arrow::datatypes::Schema;
    use pyo3::prelude::*;

    use crate::arrow::converters::ConversionContext;
    use crate::arrow::converters::test_util::{assert_py_bool, assert_py_none};
    use crate::arrow::plan::SnowflakeFieldType;

    #[test]
    fn boolean_array_converts_to_python_bool() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let array: ArrayRef = Arc::new(BooleanArray::from(vec![Some(true), None, Some(false)]));
        let column = ctx
            .converter_from_column(&array, &SnowflakeFieldType::Boolean)
            .unwrap();

        Python::attach(|py| {
            assert_py_bool(&column.to_py(py, 0).unwrap(), true);
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_py_bool(&column.to_py(py, 2).unwrap(), false);
        });
    }
}
