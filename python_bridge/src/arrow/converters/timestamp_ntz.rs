use arrow::array::ArrayRef;
use chrono::{Datelike, NaiveDateTime};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use sf_types::ReadArrowError;

use super::Column;
use crate::arrow::converters::util::{TimestampColumn, py_none};
use crate::arrow::plan::SnowflakeFieldType;

pub(crate) struct TimestampNtzColumn {
    values: TimestampColumn,
    scale: u32,
}

impl TimestampNtzColumn {
    pub(crate) fn to_py<'py>(&self, py: Python<'py>, row: usize) -> PyResult<Bound<'py, PyAny>> {
        match self.values.get(row, self.scale) {
            Ok(value) => TimestampNtzMaterializer::materialize(py, value),
            Err(ReadArrowError::NullValue { .. }) => Ok(py_none(py)),
            Err(err) => Err(PyValueError::new_err(err.to_string())),
        }
    }
}

struct TimestampNtzMaterializer;

impl TimestampNtzMaterializer {
    fn materialize<'py>(py: Python<'py>, value: NaiveDateTime) -> PyResult<Bound<'py, PyAny>> {
        let year = value.year();
        if !(1..=9999).contains(&year) {
            return Err(PyValueError::new_err(format!(
                "TIMESTAMP_NTZ {value} is outside the datetime.datetime year range 1..=9999"
            )));
        }
        Ok(value.into_pyobject(py)?.into_any())
    }
}

pub(super) fn from_column(
    array: &ArrayRef,
    field_type: &SnowflakeFieldType,
    scale: u32,
) -> PyResult<Column> {
    Ok(Column::TimestampNtz(TimestampNtzColumn {
        values: TimestampColumn::from_array(array, field_type)?,
        scale,
    }))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{Array, ArrayRef, Date32Array, Int32Array, Int64Array, StructArray};
    use arrow::buffer::NullBuffer;
    use arrow::datatypes::{DataType, Field, Schema};
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

    use crate::arrow::converters::ConversionContext;
    use crate::arrow::converters::test_util::{assert_py_datetime, assert_py_none};
    use crate::arrow::plan::SnowflakeFieldType;

    fn ntz(scale: u32) -> SnowflakeFieldType {
        SnowflakeFieldType::TimestampNtz { scale }
    }

    fn ntz_struct(epochs: Vec<Option<i64>>, fractions: Vec<Option<i32>>) -> ArrayRef {
        let nulls: Vec<bool> = epochs.iter().map(Option::is_some).collect();
        let fields = vec![
            Field::new("epoch", DataType::Int64, true),
            Field::new("fraction", DataType::Int32, true),
        ];
        Arc::new(
            StructArray::try_new(
                fields.into(),
                vec![
                    Arc::new(Int64Array::from(epochs)),
                    Arc::new(Int32Array::from(fractions)),
                ],
                Some(NullBuffer::from(nulls)),
            )
            .unwrap(),
        )
    }

    #[test]
    fn int64_converts_to_python_datetime_with_nulls() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let scale0: ArrayRef = Arc::new(Int64Array::from(vec![Some(1_453_357_964), None, Some(0)]));
        let scale0_column = ctx.converter_from_column(&scale0, &ntz(0)).unwrap();

        let scale9: ArrayRef = Arc::new(Int64Array::from(vec![Some(1_000_000_000_123_456_789)]));
        let scale9_column = ctx.converter_from_column(&scale9, &ntz(9)).unwrap();

        Python::attach(|py| {
            assert_py_datetime(
                &scale0_column.to_py(py, 0).unwrap(),
                (2016, 1, 21),
                (6, 32, 44, 0),
            );
            assert_py_none(&scale0_column.to_py(py, 1).unwrap());
            assert_py_datetime(
                &scale0_column.to_py(py, 2).unwrap(),
                (1970, 1, 1),
                (0, 0, 0, 0),
            );

            assert_py_datetime(
                &scale9_column.to_py(py, 0).unwrap(),
                (2001, 9, 9),
                (1, 46, 40, 123_456),
            );
        });
    }

    #[test]
    fn struct_converts_to_python_datetime_with_nulls() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array = ntz_struct(
            vec![Some(1_453_357_964), None, Some(1_000_000_000)],
            vec![Some(0), None, Some(123_456_789)],
        );
        let column = ctx.converter_from_column(&array, &ntz(9)).unwrap();

        Python::attach(|py| {
            assert_py_datetime(&column.to_py(py, 0).unwrap(), (2016, 1, 21), (6, 32, 44, 0));
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_py_datetime(
                &column.to_py(py, 2).unwrap(),
                (2001, 9, 9),
                (1, 46, 40, 123_456),
            );
        });
    }

    #[test]
    fn converts_python_datetime_year_bounds() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Int64Array::from(vec![
            Some(-62_135_596_800),
            Some(253_402_300_799),
        ]));
        let column = ctx.converter_from_column(&array, &ntz(0)).unwrap();

        Python::attach(|py| {
            assert_py_datetime(&column.to_py(py, 0).unwrap(), (1, 1, 1), (0, 0, 0, 0));
            assert_py_datetime(
                &column.to_py(py, 1).unwrap(),
                (9999, 12, 31),
                (23, 59, 59, 0),
            );
        });
    }

    #[test]
    fn rejects_physical_mismatch_for_timestamp_ntz() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let cases: Vec<ArrayRef> = vec![
            Arc::new(Date32Array::from(vec![Some(0)])),
            Arc::new(Int32Array::from(vec![Some(0)])),
        ];
        for array in cases {
            let err = match ctx.converter_from_column(&array, &ntz(0)) {
                Ok(_) => panic!(
                    "expected physical mismatch for TIMESTAMP_NTZ, got {:?}",
                    array.data_type()
                ),
                Err(err) => err,
            };
            Python::attach(|py| {
                assert!(
                    err.is_instance_of::<PyValueError>(py),
                    "expected PyValueError, got {err}"
                );
                let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
                assert!(
                    text.contains("logical/physical type mismatch")
                        && text.contains("TIMESTAMP_NTZ"),
                    "got {text}"
                );
            });
        }
    }

    #[test]
    fn rejects_datetimes_outside_python_year_range() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Int64Array::from(vec![
            Some(-62_135_683_200),
            Some(253_402_300_800),
        ]));
        let column = ctx.converter_from_column(&array, &ntz(0)).unwrap();

        Python::attach(|py| {
            for row in [0, 1] {
                let err = column.to_py(py, row).unwrap_err();
                assert!(
                    err.is_instance_of::<PyValueError>(py),
                    "expected PyValueError, got {err}"
                );
                let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
                assert!(
                    text.contains("datetime.datetime year range 1..=9999"),
                    "got {text}"
                );
            }
        });
    }

    #[test]
    fn rejects_raw_values_outside_chrono_range() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Int64Array::from(vec![Some(i64::MAX)]));
        let column = ctx.converter_from_column(&array, &ntz(0)).unwrap();

        Python::attach(|py| {
            let err = column.to_py(py, 0).unwrap_err();
            assert!(
                err.is_instance_of::<PyValueError>(py),
                "expected PyValueError, got {err}"
            );
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(
                text.contains("Invalid Arrow value") && text.contains("timestamp"),
                "got {text}"
            );
        });
    }
}
