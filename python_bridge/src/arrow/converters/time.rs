use arrow::array::ArrayRef;
use chrono::NaiveTime;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use sf_types::{InvalidArrowValueSnafu, ReadArrowError, split_time_raw};
use snafu::OptionExt;

use super::Column;
use crate::arrow::converters::util::{IntColumn, py_none};
use crate::arrow::plan::SnowflakeFieldType;

pub(crate) struct TimeColumn {
    values: IntColumn,
    scale: u32,
}

impl TimeColumn {
    pub(crate) fn to_py<'py>(&self, py: Python<'py>, row: usize) -> PyResult<Bound<'py, PyAny>> {
        match self
            .values
            .get(row)
            .and_then(|raw| naive_time_from_raw(raw, self.scale))
        {
            Ok(value) => Ok(value.into_pyobject(py)?.into_any()),
            Err(ReadArrowError::NullValue { .. }) => Ok(py_none(py)),
            Err(err) => Err(PyValueError::new_err(err.to_string())),
        }
    }
}

fn naive_time_from_raw(value: i128, scale: u32) -> Result<NaiveTime, ReadArrowError> {
    i64::try_from(value)
        .ok()
        .and_then(|raw| split_time_raw(raw, scale))
        .and_then(|(secs, nanos)| NaiveTime::from_num_seconds_from_midnight_opt(secs, nanos))
        .with_context(|| InvalidArrowValueSnafu {
            reason: format!("raw TIME value {value} with scale {scale} is not a valid time of day"),
        })
}

pub(super) fn from_column(
    array: &ArrayRef,
    field_type: &SnowflakeFieldType,
    scale: u32,
) -> PyResult<Column> {
    Ok(Column::Time(TimeColumn {
        values: IntColumn::from_fixed(array, field_type)?,
        scale,
    }))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{
        ArrayRef, Date32Array, Decimal128Array, Int8Array, Int16Array, Int32Array, Int64Array,
    };
    use arrow::datatypes::Schema;
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

    use crate::arrow::converters::ConversionContext;
    use crate::arrow::converters::test_util::{assert_py_none, assert_py_time};
    use crate::arrow::plan::SnowflakeFieldType;

    fn time(scale: u32) -> SnowflakeFieldType {
        SnowflakeFieldType::Time { scale }
    }

    #[test]
    fn time_converts_to_python_time_with_nulls() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let int32: ArrayRef = Arc::new(Int32Array::from(vec![
            Some(452_960_000),
            None,
            Some(863_999_999),
        ]));
        let int32_column = ctx.converter_from_column(&int32, &time(4)).unwrap();

        let int64: ArrayRef = Arc::new(Int64Array::from(vec![
            Some(45_296_123_456_789),
            None,
            Some(86_399_999_999_999),
        ]));
        let int64_column = ctx.converter_from_column(&int64, &time(9)).unwrap();

        Python::attach(|py| {
            assert_py_time(&int32_column.to_py(py, 0).unwrap(), 12, 34, 56, 0);
            assert_py_none(&int32_column.to_py(py, 1).unwrap());
            assert_py_time(&int32_column.to_py(py, 2).unwrap(), 23, 59, 59, 999_900);

            assert_py_time(&int64_column.to_py(py, 0).unwrap(), 12, 34, 56, 123_456);
            assert_py_none(&int64_column.to_py(py, 1).unwrap());
            assert_py_time(&int64_column.to_py(py, 2).unwrap(), 23, 59, 59, 999_999);
        });
    }

    #[test]
    fn integer_widths_convert_at_scale_0() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let cases: Vec<(ArrayRef, u8, u8, u8)> = vec![
            (Arc::new(Int8Array::from(vec![Some(12)])), 0, 0, 12),
            (Arc::new(Int16Array::from(vec![Some(3661)])), 1, 1, 1),
            (Arc::new(Int32Array::from(vec![Some(0)])), 0, 0, 0),
            (Arc::new(Int64Array::from(vec![Some(86_399)])), 23, 59, 59),
            (
                Arc::new(Decimal128Array::from(vec![Some(45_296i128)])),
                12,
                34,
                56,
            ),
        ];

        Python::attach(|py| {
            for (array, hour, minute, second) in cases {
                let column = ctx.converter_from_column(&array, &time(0)).unwrap();
                assert_py_time(&column.to_py(py, 0).unwrap(), hour, minute, second, 0);
            }
        });
    }

    #[test]
    fn scale0_converts_midnight_and_end_of_day() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Int32Array::from(vec![Some(0), None, Some(86_399)]));
        let column = ctx.converter_from_column(&array, &time(0)).unwrap();

        Python::attach(|py| {
            assert_py_time(&column.to_py(py, 0).unwrap(), 0, 0, 0, 0);
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_py_time(&column.to_py(py, 2).unwrap(), 23, 59, 59, 0);
        });
    }

    #[test]
    fn rejects_physical_mismatch_for_time() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Date32Array::from(vec![Some(0)]));
        let err = match ctx.converter_from_column(&array, &time(0)) {
            Ok(_) => panic!("expected physical mismatch for TIME, got Date32"),
            Err(err) => err,
        };
        Python::attach(|py| {
            assert!(
                err.is_instance_of::<PyValueError>(py),
                "expected PyValueError, got {err}"
            );
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(
                text.contains("logical/physical type mismatch") && text.contains("TIME"),
                "got {text}"
            );
        });
    }

    #[test]
    fn rejects_raw_values_outside_time_of_day() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Int64Array::from(vec![Some(-1), Some(86_400)]));
        let column = ctx.converter_from_column(&array, &time(0)).unwrap();

        Python::attach(|py| {
            for row in [0, 1] {
                let err = column.to_py(py, row).unwrap_err();
                assert!(
                    err.is_instance_of::<PyValueError>(py),
                    "expected PyValueError, got {err}"
                );
                let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
                assert!(
                    text.contains("Invalid Arrow value") && text.contains("TIME"),
                    "got {text}"
                );
            }
        });
    }
}
