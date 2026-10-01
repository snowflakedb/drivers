use std::sync::Mutex;

use arrow::array::{Array, StructArray};
use arrow::datatypes::{Schema, SchemaRef};
use arrow::ffi::{FFI_ArrowSchema, to_ffi};
use arrow::record_batch::RecordBatch;
use pyo3::exceptions::{PyRuntimeError, PyStopIteration};
use pyo3::prelude::*;
use sf_core::utils::sync::MutexRecoverExt;

use super::table::TableConverter;
use crate::arrow::error::{PlanError, StreamError, wrap_row_conversion};
use crate::arrow::stream::RowStream;

#[derive(Debug)]
enum TakeError {
    Stream(StreamError),
    Plan(PlanError),
}

impl From<StreamError> for TakeError {
    fn from(error: StreamError) -> Self {
        Self::Stream(error)
    }
}

impl From<PlanError> for TakeError {
    fn from(error: PlanError) -> Self {
        Self::Plan(error)
    }
}

impl std::fmt::Display for TakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stream(error) => write!(f, "{error}"),
            Self::Plan(error) => write!(f, "{error}"),
        }
    }
}

impl From<TakeError> for PyErr {
    fn from(error: TakeError) -> Self {
        match error {
            TakeError::Stream(error) => error.into(),
            TakeError::Plan(error) => error.into(),
        }
    }
}

struct PyArrowTypes {
    record_batch: Py<PyAny>,
    schema: Py<PyAny>,
}

impl PyArrowTypes {
    fn import(py: Python<'_>) -> PyResult<Self> {
        let pyarrow = py.import("pyarrow")?;
        Ok(Self {
            record_batch: pyarrow.getattr("RecordBatch")?.unbind(),
            schema: pyarrow.getattr("Schema")?.unbind(),
        })
    }
}

#[pyclass(name = "ArrowStreamTableIterator")]
pub struct ArrowStreamTableIterator {
    stream: Mutex<RowStream>,
    converter: TableConverter,
    wire_schema: SchemaRef,
    pyarrow: Option<PyArrowTypes>,
    converted_schema: Option<Py<PyAny>>,
}

#[pymethods]
impl ArrowStreamTableIterator {
    #[new]
    #[pyo3(signature = (
        stream_ptr,
        session_timezone=None,
        number_to_decimal=false,
        force_microsecond_precision=false
    ))]
    pub(crate) fn new(
        _py: Python<'_>,
        stream_ptr: i64,
        session_timezone: Option<String>,
        number_to_decimal: bool,
        force_microsecond_precision: bool,
    ) -> PyResult<Self> {
        let stream = RowStream::from_stream_ptr(stream_ptr)?;
        Ok(Self {
            wire_schema: stream.schema(),
            stream: Mutex::new(stream),
            converter: TableConverter::new(
                number_to_decimal,
                force_microsecond_precision,
                session_timezone,
            ),
            pyarrow: None,
            converted_schema: None,
        })
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.next_batch(py)
            .map_err(|err| wrap_row_conversion(py, err))
    }

    fn get_converted_schema(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.converted_schema(py)
            .map_err(|err| wrap_row_conversion(py, err))
    }
}

impl ArrowStreamTableIterator {
    fn pyarrow(&mut self, py: Python<'_>) -> PyResult<&PyArrowTypes> {
        if self.pyarrow.is_none() {
            self.pyarrow = Some(PyArrowTypes::import(py)?);
        }
        Ok(self.pyarrow.as_ref().expect("imported above"))
    }

    fn take_converted_batch(&mut self) -> Result<Option<RecordBatch>, TakeError> {
        let Some(batch) = self.stream.lock_recover().load_next_batch()? else {
            return Ok(None);
        };
        Ok(Some(self.converter.convert_batch(batch)?))
    }

    fn next_batch(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let converted = py.detach(|| self.take_converted_batch())?;
        let Some(converted) = converted else {
            return Err(PyStopIteration::new_err(()));
        };
        let types = self.pyarrow(py)?;
        export_record_batch(py, types, converted)
    }

    fn converted_schema(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        if let Some(schema) = &self.converted_schema {
            return Ok(schema.clone_ref(py));
        }
        let schema = self
            .converter
            .convert_schema(Schema::clone(self.wire_schema.as_ref()))?;
        let types = self.pyarrow(py)?;
        let imported = export_schema(py, types, &schema)?;
        self.converted_schema = Some(imported.clone_ref(py));
        Ok(imported)
    }
}

fn export_record_batch(
    py: Python<'_>,
    types: &PyArrowTypes,
    batch: RecordBatch,
) -> PyResult<Py<PyAny>> {
    let data = StructArray::from(batch).into_data();
    let (ffi_array, ffi_schema) = to_ffi(&data).map_err(ffi_error)?;
    let imported = types.record_batch.bind(py).call_method1(
        "_import_from_c",
        (ptr_to_usize(&ffi_array), ptr_to_usize(&ffi_schema)),
    )?;
    std::mem::forget(ffi_array);
    std::mem::forget(ffi_schema);
    Ok(imported.unbind())
}

fn export_schema(py: Python<'_>, types: &PyArrowTypes, schema: &Schema) -> PyResult<Py<PyAny>> {
    let ffi_schema = FFI_ArrowSchema::try_from(schema).map_err(ffi_error)?;
    let imported = types
        .schema
        .bind(py)
        .call_method1("_import_from_c", (ptr_to_usize(&ffi_schema),))?;
    std::mem::forget(ffi_schema);
    Ok(imported.unbind())
}

fn ptr_to_usize<T>(value: &T) -> usize {
    value as *const T as usize
}

fn ffi_error(err: arrow::error::ArrowError) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use arrow::array::{Int32Array, Int64Array, StringArray, StructArray};
    use arrow::buffer::NullBuffer;
    use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
    use arrow::record_batch::RecordBatch;
    use pyo3::exceptions::PyStopIteration;

    use super::*;
    use crate::arrow::test_support::stream_ptr_from_batches;

    const EPOCH_2024: i64 = 1_705_314_600;
    const EPOCH_YEAR_9999: i64 = 253_402_300_799;

    fn metadata(logical: &str, extra: &[(&str, &str)]) -> HashMap<String, String> {
        let mut map = HashMap::from([("logicalType".to_string(), logical.to_string())]);
        for (key, value) in extra {
            map.insert((*key).to_string(), (*value).to_string());
        }
        map
    }

    fn new_iterator(stream_ptr: i64) -> ArrowStreamTableIterator {
        Python::attach(|py| {
            ArrowStreamTableIterator::new(py, stream_ptr, None, false, false).unwrap()
        })
    }

    fn ntz_struct_field(scale: &str) -> Field {
        Field::new(
            "ts",
            DataType::Struct(
                vec![
                    Field::new("epoch", DataType::Int64, true),
                    Field::new("fraction", DataType::Int32, true),
                ]
                .into(),
            ),
            true,
        )
        .with_metadata(metadata("TIMESTAMP_NTZ", &[("scale", scale)]))
    }

    fn ntz_struct(epochs: Vec<Option<i64>>, fractions: Vec<Option<i32>>) -> StructArray {
        let nulls: Vec<bool> = epochs.iter().map(Option::is_some).collect();
        StructArray::try_new(
            vec![
                Field::new("epoch", DataType::Int64, true),
                Field::new("fraction", DataType::Int32, true),
            ]
            .into(),
            vec![
                Arc::new(Int64Array::from(epochs)),
                Arc::new(Int32Array::from(fractions)),
            ],
            Some(NullBuffer::from(nulls)),
        )
        .unwrap()
    }

    fn ntz_batch(epochs: Vec<Option<i64>>, fractions: Vec<Option<i32>>) -> RecordBatch {
        let array = ntz_struct(epochs, fractions);
        RecordBatch::try_new(
            Arc::new(Schema::new(vec![ntz_struct_field("9")])),
            vec![Arc::new(array)],
        )
        .unwrap()
    }

    fn converted_type(batch: &RecordBatch) -> DataType {
        batch.schema().field(0).data_type().clone()
    }

    #[test]
    fn constructs_without_preloading_or_converting() {
        Python::initialize();
        let schema = Arc::new(Schema::new(vec![ntz_struct_field("9")]));
        let batch = ntz_batch(vec![Some(EPOCH_2024)], vec![Some(1)]);
        let mut iterator = new_iterator(stream_ptr_from_batches(vec![batch], schema));
        assert!(iterator.converted_schema.is_none());
        assert!(iterator.pyarrow.is_none());

        let first = iterator.take_converted_batch().unwrap().unwrap();
        assert_eq!(
            converted_type(&first),
            DataType::Timestamp(TimeUnit::Nanosecond, None)
        );
        assert!(iterator.converted_schema.is_none());
        assert!(iterator.pyarrow.is_none());
    }

    #[test]
    fn empty_stream_stops_and_still_converts_schema() {
        Python::initialize();
        let schema = Arc::new(Schema::new(vec![
            Field::new("t", DataType::Int64, true)
                .with_metadata(metadata("TIME", &[("scale", "9")])),
        ]));
        let mut iterator = new_iterator(stream_ptr_from_batches(vec![], schema));

        Python::attach(|py| {
            let err = iterator.__next__(py).unwrap_err();
            assert!(err.is_instance_of::<PyStopIteration>(py));
        });

        let converted = iterator
            .converter
            .convert_schema(Schema::clone(iterator.wire_schema.as_ref()))
            .unwrap();
        assert_eq!(
            converted.field(0).data_type(),
            &DataType::Time64(TimeUnit::Microsecond)
        );
    }

    #[test]
    fn decfloat_constructs_and_fails_on_convert() {
        Python::initialize();
        let schema = Arc::new(Schema::new(vec![
            Field::new("d", DataType::Utf8, true).with_metadata(metadata("DECFLOAT", &[])),
        ]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(StringArray::from(vec![Some("1E0")]))],
        )
        .unwrap();
        let mut iterator = new_iterator(stream_ptr_from_batches(vec![batch], schema));
        assert!(iterator.converted_schema.is_none());

        let schema_err = iterator
            .converter
            .convert_schema(Schema::clone(iterator.wire_schema.as_ref()))
            .unwrap_err();
        assert!(
            schema_err
                .to_string()
                .contains("unsupported Snowflake type: DECFLOAT")
        );
        let next_err = iterator.take_converted_batch().unwrap_err();
        assert!(
            next_err
                .to_string()
                .contains("unsupported Snowflake type: DECFLOAT")
        );
    }

    #[test]
    fn two_batches_convert_timestamp_units_independently() {
        Python::initialize();
        let schema = Arc::new(Schema::new(vec![ntz_struct_field("9")]));
        let in_range = ntz_batch(vec![Some(EPOCH_2024)], vec![Some(123_456_789)]);
        let overflow = ntz_batch(vec![Some(EPOCH_YEAR_9999)], vec![Some(123_456_000)]);
        let mut iterator = new_iterator(stream_ptr_from_batches(vec![in_range, overflow], schema));

        let first = iterator.take_converted_batch().unwrap().unwrap();
        assert_eq!(
            converted_type(&first),
            DataType::Timestamp(TimeUnit::Nanosecond, None)
        );
        let second = iterator.take_converted_batch().unwrap().unwrap();
        assert_eq!(
            converted_type(&second),
            DataType::Timestamp(TimeUnit::Microsecond, None)
        );
        assert!(iterator.take_converted_batch().unwrap().is_none());
    }
}
