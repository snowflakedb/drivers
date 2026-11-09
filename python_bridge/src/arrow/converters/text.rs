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
        PyString::from_bytes(py, value.as_bytes()).map(Bound::into_any)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use arrow::array::{ArrayRef, RecordBatch, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use pyo3::prelude::*;

    use crate::arrow::converters::ConversionContext;
    use crate::arrow::converters::test_util::{assert_py_none, assert_py_str};
    use crate::arrow::iterator::ArrowStreamIterator;
    use crate::arrow::plan::SnowflakeFieldType;
    use crate::arrow::test_support::stream_ptr_from_batches;

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

        let array: ArrayRef = Arc::new(StringArray::from(vec![
            Some("hello"),
            None,
            Some(""),
            Some("naïve — 日本語\0tail"),
        ]));
        let column = ctx.converter_from_column(&array, &varchar(false)).unwrap();

        Python::attach(|py| {
            assert_py_str(&column.to_py(py, 0).unwrap(), "hello");
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_py_str(&column.to_py(py, 2).unwrap(), "");
            assert_py_str(&column.to_py(py, 3).unwrap(), "naïve — 日本語\0tail");
        });
    }

    #[test]
    fn text_stays_python_str_with_numpy() {
        Python::initialize();
        let context = ConversionContext::with_numpy(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(StringArray::from(vec![Some("hello")]));
        let column = context
            .converter_from_column(&array, &varchar(false))
            .unwrap();

        Python::attach(|py| {
            assert_py_str(&column.to_py(py, 0).unwrap(), "hello");
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

    fn utf8_array_unchecked(cells: &[&[u8]]) -> StringArray {
        use arrow::buffer::{Buffer, OffsetBuffer};

        let mut offsets = vec![0i32];
        let mut data = Vec::new();
        for cell in cells {
            data.extend_from_slice(cell);
            offsets.push(i32::try_from(data.len()).unwrap());
        }
        let offsets = OffsetBuffer::new(offsets.into());
        // SAFETY: offsets are non-decreasing and inside `data`. The bytes are
        // not valid UTF-8, so `try_new` would reject the array.
        unsafe { StringArray::new_unchecked(offsets, Buffer::from(data), None) }
    }

    fn text_batch(cells: &[&[u8]]) -> (RecordBatch, std::sync::Arc<Schema>) {
        let schema = Arc::new(Schema::new(vec![
            Field::new("col", DataType::Utf8, true).with_metadata(HashMap::from([(
                "logicalType".to_string(),
                "TEXT".to_string(),
            )])),
        ]));
        let column: ArrayRef = Arc::new(utf8_array_unchecked(cells));
        let batch = RecordBatch::try_new(schema.clone(), vec![column]).unwrap();
        (batch, schema)
    }

    #[test]
    fn iterator_returns_valid_text_and_errors_on_invalid_utf8() {
        Python::initialize();
        let (batch, schema) = text_batch(&[b"hello", b"\xff"]);
        let mut iterator = Python::attach(|py| {
            ArrowStreamIterator::new(
                py,
                stream_ptr_from_batches(vec![batch], schema),
                None,
                false,
                false,
            )
            .unwrap()
        });

        Python::attach(|py| {
            let row = iterator.__next__(py).unwrap().unwrap();
            assert_py_str(&row.bind(py).get_item(0).unwrap(), "hello");

            let err = iterator.__next__(py).unwrap_err();
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(
                text.contains("'utf-8' codec can't decode byte 0xff"),
                "got {text}"
            );
        });
    }
}
