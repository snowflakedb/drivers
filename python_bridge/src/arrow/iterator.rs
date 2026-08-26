use std::sync::Mutex;

use pyo3::exceptions::PyStopIteration;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};
use sf_core::utils::sync::MutexRecoverExt;

use crate::arrow::batch_converter::{BatchConverter, BatchPyList};
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
    #[pyo3(signature = (stream_ptr, session_timezone=None))]
    pub(crate) fn new(
        py: Python<'_>,
        stream_ptr: i64,
        session_timezone: Option<String>,
    ) -> PyResult<Self> {
        let _ = session_timezone;
        Self::construct(py, stream_ptr).map_err(|err| wrap_row_conversion(py, err))
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub(crate) fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyTuple>>> {
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
    fn construct(py: Python<'_>, stream_ptr: i64) -> PyResult<Self> {
        let stream = RowStream::from_stream_ptr(stream_ptr)?;
        let context = ConversionContext::new(stream.schema().as_ref())?;
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

    fn next_row(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyTuple>>> {
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

        let mut result = BatchPyList::new(py, self.converter.column_count());
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
        let mut result = BatchPyList::new(py, self.converter.column_count());
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
