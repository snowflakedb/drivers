use arrow::array::{ArrayRef, StringArray};
use pyo3::prelude::*;
use pyo3::types::PyString;
use sf_types::SnowflakeText;

use super::Column;
use super::decode::{PyMaterializer, TypedColumn};
use crate::arrow::converters::util::downcast_column;
use crate::arrow::plan::SnowflakeFieldType;

pub(super) fn from_column(array: &ArrayRef, field_type: &SnowflakeFieldType) -> PyResult<Column> {
    downcast_column::<StringArray>(array, field_type)
        .map(|array| Column::Text(TypedColumn::new(array, SnowflakeText, TextMaterializer)))
}

pub(crate) struct TextMaterializer;

impl PyMaterializer<SnowflakeText> for TextMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, value: &str) -> PyResult<Bound<'py, PyAny>> {
        Ok(PyString::new(py, value).into_any())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{ArrayRef, StringArray};
    use arrow::datatypes::Schema;
    use pyo3::prelude::*;

    use crate::arrow::converters::ConversionContext;
    use crate::arrow::converters::test_util::{assert_py_none, assert_py_str};
    use crate::arrow::plan::SnowflakeFieldType;

    fn varchar(is_semi_structured: bool) -> SnowflakeFieldType {
        SnowflakeFieldType::Varchar {
            len: 16_777_216,
            is_semi_structured,
        }
    }

    #[test]
    fn text_converts_to_python_str() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let array: ArrayRef = Arc::new(StringArray::from(vec![Some("hello"), None, Some("")]));
        let column = ctx.converter_from_column(&array, &varchar(false)).unwrap();

        Python::attach(|py| {
            assert_py_str(&column.to_py(py, 0).unwrap(), "hello");
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_py_str(&column.to_py(py, 2).unwrap(), "");
        });
    }

    #[test]
    fn semi_structured_varchar_uses_the_same_utf8_path() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let array: ArrayRef = Arc::new(StringArray::from(vec![Some("{\"a\":1}")]));
        let column = ctx.converter_from_column(&array, &varchar(true)).unwrap();

        Python::attach(|py| {
            assert_py_str(&column.to_py(py, 0).unwrap(), "{\"a\":1}");
        });
    }
}
