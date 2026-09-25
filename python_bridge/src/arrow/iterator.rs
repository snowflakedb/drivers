use std::sync::Mutex;

use pyo3::exceptions::PyStopIteration;
use pyo3::prelude::*;
use pyo3::types::PyList;
use sf_core::utils::sync::MutexRecoverExt;

use crate::arrow::batch_converter::BatchConverter;
use crate::arrow::converters::ConversionContext;
use crate::arrow::error::{wrap_row_conversion, wrap_rows_conversion};
use crate::arrow::stream::RowStream;

#[pyclass(name = "ArrowStreamIterator")]
pub struct ArrowStreamIterator {
    // Mutex here for PyO3 compatibility
    stream: Mutex<RowStream>,
    context: ConversionContext,
    converter: BatchConverter,
}

#[pymethods]
impl ArrowStreamIterator {
    #[new]
    #[pyo3(signature = (stream_ptr, session_timezone=None, use_dict_result=false, use_numpy=false))]
    pub(crate) fn new(
        py: Python<'_>,
        stream_ptr: i64,
        session_timezone: Option<String>,
        use_dict_result: bool,
        use_numpy: bool,
    ) -> PyResult<Self> {
        Self::construct(py, stream_ptr, session_timezone, use_dict_result, use_numpy)
            .map_err(|err| wrap_row_conversion(py, err))
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub(crate) fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.next_row(py)
            .map_err(|err| wrap_row_conversion(py, err))
    }

    fn fetch_many(&mut self, py: Python<'_>, size: usize) -> PyResult<Py<PyList>> {
        self.fetch_many_inner(py, size)
            .map_err(|err| wrap_rows_conversion(py, err))
    }

    pub(crate) fn fetch_all(&mut self, py: Python<'_>) -> PyResult<Py<PyList>> {
        self.fetch_all_inner(py)
            .map_err(|err| wrap_rows_conversion(py, err))
    }
}

impl ArrowStreamIterator {
    fn construct(
        py: Python<'_>,
        stream_ptr: i64,
        session_timezone: Option<String>,
        use_dict_result: bool,
        use_numpy: bool,
    ) -> PyResult<Self> {
        let stream = RowStream::from_stream_ptr(stream_ptr)?;
        let context = if use_dict_result {
            ConversionContext::with_dict_keys(
                py,
                stream.schema().as_ref(),
                session_timezone,
                use_numpy,
            )?
        } else {
            ConversionContext::with_session_timezone(
                stream.schema().as_ref(),
                session_timezone,
                use_numpy,
            )?
        };
        let mut this = Self {
            stream: Mutex::new(stream),
            context,
            converter: BatchConverter::exhausted(),
        };
        if let Some(converter) = this.load_next_batch_converter(py)? {
            this.converter = converter;
        }
        Ok(this)
    }

    fn next_row(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        if self.converter.is_exhausted() {
            match self.load_next_batch_converter(py)? {
                Some(next) => self.converter = next,
                None => return Err(PyStopIteration::new_err(())),
            }
        }
        Ok(Some(self.converter.take_row(py)?))
    }

    fn fetch_many_inner(&mut self, py: Python<'_>, size: usize) -> PyResult<Py<PyList>> {
        if size == 0 {
            return Ok(PyList::empty(py).unbind());
        }

        let mut result = self.converter.row_list(py);
        while result.len() < size {
            if self.converter.is_exhausted() {
                match self.load_next_batch_converter(py)? {
                    Some(next) => self.converter = next,
                    None => break,
                }
            }

            let want = size - result.len();
            self.converter
                .append_rows_column_major(py, &mut result, want)?;
        }

        Ok(result.into_list())
    }

    fn fetch_all_inner(&mut self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let mut result = self.converter.row_list(py);
        loop {
            if self.converter.is_exhausted() {
                match self.load_next_batch_converter(py)? {
                    Some(next) => self.converter = next,
                    None => break,
                }
            }

            let batch_rows = self.converter.rows_remaining();
            self.converter
                .append_rows_column_major(py, &mut result, batch_rows)?;
        }

        Ok(result.into_list())
    }

    fn load_next_batch_converter(&mut self, py: Python<'_>) -> PyResult<Option<BatchConverter>> {
        // Release the GIL while reading the next batch from the Arrow C stream.
        // ``lock_recover`` avoids panicking off-GIL if a prior panic poisoned the mutex.
        let batch = py.detach(|| self.stream.lock_recover().load_next_batch())?;

        match batch {
            Some(batch) => Ok(Some(self.context.batch_converter(batch)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use arrow::array::{BooleanArray, Int64Array, RecordBatch};
    use arrow::datatypes::{DataType, Field, Schema};
    use pyo3::types::{PyDateAccess, PyDateTime, PyDict, PyTimeAccess, PyTzInfoAccess};

    use super::*;
    use crate::arrow::test_support::stream_ptr_from_batches;

    fn new_iterator(stream_ptr: i64) -> ArrowStreamIterator {
        Python::attach(|py| ArrowStreamIterator::new(py, stream_ptr, None, false, false).unwrap())
    }

    fn new_dict_iterator(stream_ptr: i64) -> ArrowStreamIterator {
        Python::attach(|py| ArrowStreamIterator::new(py, stream_ptr, None, true, false).unwrap())
    }

    fn boolean_field(name: &str) -> Field {
        Field::new(name, DataType::Boolean, true).with_metadata(HashMap::from([(
            "logicalType".to_string(),
            "BOOLEAN".to_string(),
        )]))
    }

    fn boolean_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![boolean_field("b")]))
    }

    fn boolean_batch(values: Vec<Option<bool>>, schema: &Arc<Schema>) -> RecordBatch {
        RecordBatch::try_new(schema.clone(), vec![Arc::new(BooleanArray::from(values))]).unwrap()
    }

    fn dict_bool(row: &Bound<'_, PyDict>) -> Option<bool> {
        let value = row.get_item("b").unwrap().unwrap();
        if value.is_none() {
            None
        } else {
            Some(value.extract::<bool>().unwrap())
        }
    }

    #[test]
    fn fetch_all_concatenates_arrow_batches() {
        Python::initialize();
        let schema = boolean_schema();
        let first = boolean_batch(vec![Some(true), Some(false)], &schema);
        let second = boolean_batch(vec![None, Some(true)], &schema);
        let mut iterator = new_iterator(stream_ptr_from_batches(vec![first, second], schema));

        Python::attach(|py| {
            let rows = iterator.fetch_all(py).unwrap();
            let rows = rows.bind(py);
            assert_eq!(rows.len(), 4);
            assert_eq!(
                rows.get_item(0).unwrap().extract::<(bool,)>().unwrap(),
                (true,)
            );
            assert_eq!(
                rows.get_item(1).unwrap().extract::<(bool,)>().unwrap(),
                (false,)
            );
            assert!(rows.get_item(2).unwrap().get_item(0).unwrap().is_none());
            assert_eq!(
                rows.get_item(3).unwrap().extract::<(bool,)>().unwrap(),
                (true,)
            );
        });
    }

    #[test]
    fn fetch_many_spans_arrow_batches() {
        Python::initialize();
        let schema = boolean_schema();
        let first = boolean_batch(vec![Some(true)], &schema);
        let second = boolean_batch(vec![Some(false), None], &schema);
        let mut iterator = new_iterator(stream_ptr_from_batches(vec![first, second], schema));

        Python::attach(|py| {
            let rows = iterator.fetch_many(py, 3).unwrap();
            let rows = rows.bind(py);
            assert_eq!(rows.len(), 3);
            assert_eq!(
                rows.get_item(0).unwrap().extract::<(bool,)>().unwrap(),
                (true,)
            );
            assert_eq!(
                rows.get_item(1).unwrap().extract::<(bool,)>().unwrap(),
                (false,)
            );
            assert!(rows.get_item(2).unwrap().get_item(0).unwrap().is_none());
        });
    }

    #[test]
    fn fetch_many_returns_up_to_size() {
        Python::initialize();
        let schema = boolean_schema();
        let batch = boolean_batch(vec![Some(true), Some(false)], &schema);
        let mut iterator = new_iterator(stream_ptr_from_batches(vec![batch], schema));

        Python::attach(|py| {
            let rows = iterator.fetch_many(py, 5).unwrap();
            assert_eq!(rows.bind(py).len(), 2);
            assert_eq!(
                rows.bind(py)
                    .get_item(0)
                    .unwrap()
                    .extract::<(bool,)>()
                    .unwrap(),
                (true,)
            );
            assert_eq!(
                rows.bind(py)
                    .get_item(1)
                    .unwrap()
                    .extract::<(bool,)>()
                    .unwrap(),
                (false,)
            );
        });
    }

    #[test]
    fn dict_rows_materialize_across_next_fetch_many_and_fetch_all() {
        Python::initialize();
        let schema = boolean_schema();
        let batch = boolean_batch(vec![Some(true), Some(false), None, Some(true)], &schema);
        let mut iterator = new_dict_iterator(stream_ptr_from_batches(vec![batch], schema));

        Python::attach(|py| {
            let first = iterator.__next__(py).unwrap().unwrap();
            let first = first.bind(py).cast::<PyDict>().unwrap();
            assert_eq!(dict_bool(first), Some(true));

            let middle = iterator.fetch_many(py, 2).unwrap();
            let middle = middle.bind(py);
            let second = middle.get_item(0).unwrap().cast_into::<PyDict>().unwrap();
            assert_eq!(dict_bool(&second), Some(false));
            let third = middle.get_item(1).unwrap().cast_into::<PyDict>().unwrap();
            assert_eq!(dict_bool(&third), None);

            let rest = iterator.fetch_all(py).unwrap();
            let rest = rest.bind(py);
            assert_eq!(rest.len(), 1);
            let fourth = rest.get_item(0).unwrap().cast_into::<PyDict>().unwrap();
            assert_eq!(dict_bool(&fourth), Some(true));
        });
    }

    #[test]
    fn dict_duplicate_column_names_last_write_wins() {
        Python::initialize();
        let schema = Arc::new(Schema::new(vec![boolean_field("n"), boolean_field("n")]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(BooleanArray::from(vec![Some(true)])),
                Arc::new(BooleanArray::from(vec![Some(false)])),
            ],
        )
        .unwrap();
        let mut iterator =
            new_dict_iterator(stream_ptr_from_batches(vec![batch.clone()], schema.clone()));

        Python::attach(|py| {
            let row = iterator.__next__(py).unwrap().unwrap();
            let row = row.bind(py).cast::<PyDict>().unwrap();
            assert_eq!(row.len(), 1);
            let value = row.get_item("n").unwrap().unwrap();
            assert!(!value.extract::<bool>().unwrap());
        });

        let mut iterator = new_dict_iterator(stream_ptr_from_batches(vec![batch], schema));
        Python::attach(|py| {
            let rows = iterator.fetch_all(py).unwrap();
            let row = rows
                .bind(py)
                .get_item(0)
                .unwrap()
                .cast_into::<PyDict>()
                .unwrap();
            assert_eq!(row.len(), 1);
            let value = row.get_item("n").unwrap().unwrap();
            assert!(!value.extract::<bool>().unwrap());
        });
    }

    #[test]
    fn timestamp_ltz_uses_session_timezone_from_iterator() {
        Python::initialize();
        let schema = Arc::new(Schema::new(vec![
            Field::new("ts", DataType::Int64, true).with_metadata(HashMap::from([
                ("logicalType".to_string(), "TIMESTAMP_LTZ".to_string()),
                ("scale".to_string(), "0".to_string()),
            ])),
        ]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(Int64Array::from(vec![Some(1_705_314_600)]))],
        )
        .unwrap();
        let mut iterator = Python::attach(|py| {
            ArrowStreamIterator::new(
                py,
                stream_ptr_from_batches(vec![batch], schema),
                Some("America/New_York".to_string()),
                false,
                false,
            )
            .unwrap()
        });

        Python::attach(|py| {
            let row = iterator.__next__(py).unwrap().unwrap();
            let value = row.bind(py).get_item(0).unwrap();
            let datetime = value.cast::<PyDateTime>().unwrap();
            assert_eq!(
                (
                    datetime.get_year(),
                    datetime.get_month(),
                    datetime.get_day(),
                    datetime.get_hour(),
                    datetime.get_minute(),
                    datetime.get_second(),
                    datetime.get_microsecond()
                ),
                (2024, 1, 15, 5, 30, 0, 0)
            );
            let tzinfo = datetime.get_tzinfo().expect("expected tz-aware datetime");
            let tz_name: String = tzinfo.getattr("key").unwrap().extract().unwrap();
            assert_eq!(tz_name, "America/New_York");
        });
    }
}
