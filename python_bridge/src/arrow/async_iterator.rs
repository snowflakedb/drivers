use std::sync::Arc;

use arrow::datatypes::SchemaRef;
use arrow::record_batch::RecordBatch;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyType};
use pyo3_async_runtimes::tokio::future_into_py;
use sf_core::apis::database_driver_v1::AsyncArrowBatchFetcher;
use sf_core::handle_manager::Handle;
use tokio::sync::Mutex;
use tracing::instrument::WithSubscriber;

use crate::arrow::batch_converter::BatchConverter;
use crate::arrow::converters::ConversionContext;
use crate::arrow::error::{wrap_row_conversion, wrap_rows_conversion};
use crate::initialized_bridge;

#[pyclass(name = "AsyncArrowStreamIterator")]
pub struct AsyncArrowStreamIterator {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    source: BatchSource,
    context: ConversionContext,
    converter: BatchConverter,
    converted: Py<PyList>,
}

enum BatchSource {
    Fetcher(AsyncArrowBatchFetcher),
    #[cfg(test)]
    Static {
        schema: SchemaRef,
        batches: std::vec::IntoIter<RecordBatch>,
    },
}

impl BatchSource {
    fn schema(&self) -> SchemaRef {
        match self {
            Self::Fetcher(fetcher) => fetcher.schema(),
            #[cfg(test)]
            Self::Static { schema, .. } => schema.clone(),
        }
    }

    async fn next_batch(&mut self) -> PyResult<Option<RecordBatch>> {
        match self {
            Self::Fetcher(fetcher) => fetcher
                .next_batch()
                .await
                .map_err(|err| PyRuntimeError::new_err(err.to_string())),
            #[cfg(test)]
            Self::Static { batches, .. } => Ok(batches.next()),
        }
    }
}

#[pymethods]
impl AsyncArrowStreamIterator {
    /// Opens a prefetching iterator on the result set. `_cls` is the Python
    /// class object (`type`) that `#[classmethod]` passes in.
    #[classmethod]
    #[pyo3(signature = (handle_id, handle_magic, session_timezone=None, use_dict_result=false, use_numpy=false))]
    fn from_result_set<'py>(
        _cls: &Bound<'py, PyType>,
        py: Python<'py>,
        handle_id: i64,
        handle_magic: i64,
        session_timezone: Option<String>,
        use_dict_result: bool,
        use_numpy: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        future_into_py(py, async move {
            let Some(bridge) = initialized_bridge() else {
                return Err(PyRuntimeError::new_err("init was not called"));
            };
            let handle = Handle {
                id: handle_id as u64,
                magic: handle_magic as u64,
            };
            let fetcher = bridge
                .transport
                .database_driver()
                .result_set_get_async_stream(handle)
                .with_subscriber(bridge.dispatch.clone())
                .await
                .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;
            let iterator =
                Self::from_fetcher(fetcher, session_timezone, use_dict_result, use_numpy)?;
            Python::attach(|py| Py::new(py, iterator))
        })
    }

    /// Downloads the next non-empty batch into the buffer. Returns `False` at
    /// end of stream. This is the only method that performs I/O.
    fn next_batch<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move { inner.lock().await.refill().await })
    }

    #[pyo3(signature = (default=None))]
    fn take_row(&self, py: Python<'_>, default: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
        let mut guard = self.lock()?;
        if guard.converter.is_exhausted() {
            return Ok(default.unwrap_or_else(|| py.None()));
        }
        guard
            .converter
            .take_row(py)
            .map_err(|err| wrap_row_conversion(py, err))
    }

    /// Converts up to `count` buffered rows (all of them when `count` is
    /// `None`) into the held row list, and returns how many were added.
    #[pyo3(signature = (count=None))]
    fn convert(&self, py: Python<'_>, count: Option<usize>) -> PyResult<usize> {
        let mut guard = self.lock()?;
        let inner = &mut *guard;
        let want = count.unwrap_or_else(|| inner.converter.rows_remaining());

        let mut chunk = inner.converter.row_list(py);
        inner
            .converter
            .append_rows_column_major(py, &mut chunk, want)
            .map_err(|err| wrap_rows_conversion(py, err))?;
        let converted = chunk.len();
        inner
            .converted
            .bind(py)
            .call_method1(pyo3::intern!(py, "extend"), (chunk.into_list(),))?;
        Ok(converted)
    }

    fn take_converted(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let mut guard = self.lock()?;
        Ok(std::mem::replace(
            &mut guard.converted,
            PyList::empty(py).unbind(),
        ))
    }
}

impl AsyncArrowStreamIterator {
    fn from_fetcher(
        fetcher: AsyncArrowBatchFetcher,
        session_timezone: Option<String>,
        use_dict_result: bool,
        use_numpy: bool,
    ) -> PyResult<Self> {
        Self::from_source(
            BatchSource::Fetcher(fetcher),
            session_timezone,
            use_dict_result,
            use_numpy,
        )
    }

    fn from_source(
        source: BatchSource,
        session_timezone: Option<String>,
        use_dict_result: bool,
        use_numpy: bool,
    ) -> PyResult<Self> {
        let (context, converted) = Python::attach(|py| {
            let context = ConversionContext::from_flags(
                py,
                source.schema().as_ref(),
                session_timezone,
                use_dict_result,
                use_numpy,
            )?;
            Ok::<_, PyErr>((context, PyList::empty(py).unbind()))
        })?;
        Ok(Self {
            inner: Arc::new(Mutex::new(Inner {
                source,
                context,
                converter: BatchConverter::exhausted(),
                converted,
            })),
        })
    }

    fn lock(&self) -> PyResult<tokio::sync::MutexGuard<'_, Inner>> {
        self.inner
            .try_lock()
            .map_err(|_| PyRuntimeError::new_err("async Arrow iterator is already in use"))
    }
}

impl Inner {
    async fn refill(&mut self) -> PyResult<bool> {
        let Some(batch) = next_nonempty_batch(&mut self.source).await? else {
            return Ok(false);
        };
        self.converter = self.context.batch_converter(batch)?;
        Ok(true)
    }
}

async fn next_nonempty_batch(source: &mut BatchSource) -> PyResult<Option<RecordBatch>> {
    loop {
        match source.next_batch().await? {
            Some(batch) if batch.num_rows() == 0 => continue,
            other => return Ok(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::future::Future;
    use std::sync::Arc;

    use arrow::array::BooleanArray;
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use pyo3::types::PyDict;

    use super::*;

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

    fn new_iterator(
        batches: Vec<RecordBatch>,
        schema: Arc<Schema>,
        use_dict_result: bool,
    ) -> AsyncArrowStreamIterator {
        Python::initialize();
        AsyncArrowStreamIterator::from_source(
            BatchSource::Static {
                schema,
                batches: batches.into_iter(),
            },
            None,
            use_dict_result,
            false,
        )
        .unwrap()
    }

    fn block_on<T>(future: impl Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    fn next_batch(iterator: &AsyncArrowStreamIterator) -> bool {
        block_on(async { iterator.inner.lock().await.refill().await.unwrap() })
    }

    fn take_row(iterator: &AsyncArrowStreamIterator, py: Python<'_>) -> Py<PyAny> {
        iterator.take_row(py, None).unwrap()
    }

    #[test]
    fn take_row_drains_the_buffered_batch_then_returns_the_default() {
        let schema = boolean_schema();
        let batch = boolean_batch(vec![Some(true), Some(false)], &schema);
        let iterator = new_iterator(vec![batch], schema, false);
        assert!(next_batch(&iterator));

        Python::attach(|py| {
            let first = take_row(&iterator, py);
            let second = take_row(&iterator, py);
            assert_eq!(first.bind(py).extract::<(bool,)>().unwrap(), (true,));
            assert_eq!(second.bind(py).extract::<(bool,)>().unwrap(), (false,));
            assert!(take_row(&iterator, py).is_none(py));
        });
    }

    #[test]
    fn take_row_does_not_cross_a_batch_boundary_on_its_own() {
        let schema = boolean_schema();
        let first = boolean_batch(vec![Some(true)], &schema);
        let second = boolean_batch(vec![Some(false)], &schema);
        let iterator = new_iterator(vec![first, second], schema, false);
        assert!(next_batch(&iterator));

        Python::attach(|py| {
            assert_eq!(
                take_row(&iterator, py)
                    .bind(py)
                    .extract::<(bool,)>()
                    .unwrap(),
                (true,)
            );
            assert!(take_row(&iterator, py).is_none(py));
        });

        assert!(next_batch(&iterator));
        Python::attach(|py| {
            assert_eq!(
                take_row(&iterator, py)
                    .bind(py)
                    .extract::<(bool,)>()
                    .unwrap(),
                (false,)
            );
        });
        assert!(!next_batch(&iterator));
    }

    #[test]
    fn convert_accumulates_across_batches_into_one_list() {
        let schema = boolean_schema();
        let first = boolean_batch(vec![Some(true)], &schema);
        let second = boolean_batch(vec![Some(false), None], &schema);
        let iterator = new_iterator(vec![first, second], schema, false);

        assert!(next_batch(&iterator));
        Python::attach(|py| assert_eq!(iterator.convert(py, None).unwrap(), 1));
        assert!(next_batch(&iterator));
        Python::attach(|py| assert_eq!(iterator.convert(py, None).unwrap(), 2));
        assert!(!next_batch(&iterator));

        Python::attach(|py| {
            let rows = iterator.take_converted(py).unwrap();
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
    fn convert_stops_at_the_requested_count() {
        let schema = boolean_schema();
        let batch = boolean_batch(vec![Some(true), Some(false), None], &schema);
        let iterator = new_iterator(vec![batch], schema, false);
        assert!(next_batch(&iterator));

        Python::attach(|py| {
            assert_eq!(iterator.convert(py, Some(2)).unwrap(), 2);
            assert_eq!(iterator.convert(py, Some(5)).unwrap(), 1);
            assert_eq!(iterator.convert(py, Some(5)).unwrap(), 0);
            assert_eq!(iterator.take_converted(py).unwrap().bind(py).len(), 3);
        });
    }

    #[test]
    fn take_converted_hands_over_the_list_and_installs_a_fresh_one() {
        let schema = boolean_schema();
        let batch = boolean_batch(vec![Some(true)], &schema);
        let iterator = new_iterator(vec![batch], schema, false);
        assert!(next_batch(&iterator));

        Python::attach(|py| {
            iterator.convert(py, None).unwrap();
            let handed_over = iterator.take_converted(py).unwrap();
            assert_eq!(handed_over.bind(py).len(), 1);

            assert!(iterator.take_converted(py).unwrap().bind(py).is_empty());
            assert_eq!(handed_over.bind(py).len(), 1);
        });
    }

    #[test]
    fn take_row_and_convert_share_one_cursor() {
        let schema = boolean_schema();
        let batch = boolean_batch(vec![Some(true), Some(false)], &schema);
        let iterator = new_iterator(vec![batch], schema, false);
        assert!(next_batch(&iterator));

        Python::attach(|py| {
            assert_eq!(
                take_row(&iterator, py)
                    .bind(py)
                    .extract::<(bool,)>()
                    .unwrap(),
                (true,)
            );
            assert_eq!(iterator.convert(py, None).unwrap(), 1);
            let rows = iterator.take_converted(py).unwrap();
            let rows = rows.bind(py);
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows.get_item(0).unwrap().extract::<(bool,)>().unwrap(),
                (false,)
            );
        });
    }

    #[test]
    fn empty_stream_and_leading_zero_row_batch() {
        let schema = boolean_schema();
        let empty = boolean_batch(Vec::new(), &schema);
        let empty_iterator = new_iterator(Vec::new(), schema.clone(), false);
        let skipped = new_iterator(
            vec![empty, boolean_batch(vec![Some(true)], &schema)],
            schema,
            false,
        );

        assert!(!next_batch(&empty_iterator));
        assert!(next_batch(&skipped));
        Python::attach(|py| {
            assert_eq!(
                take_row(&skipped, py)
                    .bind(py)
                    .extract::<(bool,)>()
                    .unwrap(),
                (true,)
            );
        });
    }

    #[test]
    fn dict_rows_last_write_wins_across_fetch_paths() {
        let schema = Arc::new(Schema::new(vec![boolean_field("n"), boolean_field("n")]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(BooleanArray::from(vec![Some(true), Some(true)])),
                Arc::new(BooleanArray::from(vec![Some(false), Some(false)])),
            ],
        )
        .unwrap();
        let iterator = new_iterator(vec![batch], schema, true);
        assert!(next_batch(&iterator));

        Python::attach(|py| {
            let first = take_row(&iterator, py);
            let first = first.bind(py).cast::<PyDict>().unwrap();
            assert_eq!(first.len(), 1);
            let first_value = first.get_item("n").unwrap().unwrap();
            assert!(!first_value.extract::<bool>().unwrap());

            assert_eq!(iterator.convert(py, None).unwrap(), 1);
            let rest = iterator.take_converted(py).unwrap();
            let rest = rest.bind(py);
            assert_eq!(rest.len(), 1);
            let second = rest.get_item(0).unwrap().cast_into::<PyDict>().unwrap();
            assert_eq!(second.len(), 1);
            let second_value = second.get_item("n").unwrap().unwrap();
            assert!(!second_value.extract::<bool>().unwrap());
        });
    }
}
