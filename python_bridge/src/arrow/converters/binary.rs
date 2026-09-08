use arrow::array::{ArrayRef, BinaryArray};
use pyo3::prelude::*;
use pyo3::types::PyByteArray;
use sf_types::SnowflakeBinary;

use super::Column;
use super::decode::{PyMaterializer, TypedColumn};
use crate::arrow::converters::util::downcast_column;
use crate::arrow::plan::SnowflakeFieldType;

pub(super) fn from_column(array: &ArrayRef, field_type: &SnowflakeFieldType) -> PyResult<Column> {
    downcast_column::<BinaryArray>(array, field_type)
        .map(|array| Column::Binary(TypedColumn::new(array, SnowflakeBinary, BinaryMaterializer)))
}

pub(crate) struct BinaryMaterializer;

impl PyMaterializer<SnowflakeBinary> for BinaryMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, value: &[u8]) -> PyResult<Bound<'py, PyAny>> {
        Ok(PyByteArray::new(py, value).into_any())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{ArrayRef, BinaryArray};
    use arrow::datatypes::Schema;
    use pyo3::prelude::*;

    use crate::arrow::converters::ConversionContext;
    use crate::arrow::converters::test_util::{assert_py_bytes, assert_py_none};
    use crate::arrow::plan::SnowflakeFieldType;

    fn binary() -> SnowflakeFieldType {
        SnowflakeFieldType::Binary { len: 8_388_608 }
    }

    #[test]
    fn binary_converts_to_python_bytearray() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let array: ArrayRef = Arc::new(BinaryArray::from_opt_vec(vec![
            Some(b"hello".as_slice()),
            None,
            Some(b"".as_slice()),
        ]));
        let column = ctx.converter_from_column(&array, &binary()).unwrap();

        Python::attach(|py| {
            assert_py_bytes(&column.to_py(py, 0).unwrap(), b"hello");
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_py_bytes(&column.to_py(py, 2).unwrap(), b"");
        });
    }
}
