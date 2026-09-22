use std::sync::Arc;

use arrow::array::ArrayRef;
use chrono::{Datelike, NaiveDateTime, TimeZone, Utc};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyTzInfo;
use sf_types::ReadArrowError;

use super::Column;
use super::timezone::TimezoneProvider;
use crate::arrow::converters::util::{TimestampColumn, py_none};
use crate::arrow::plan::SnowflakeFieldType;

pub(crate) struct TimestampLtzColumn {
    values: TimestampColumn,
    scale: u32,
    timezone: Arc<TimezoneProvider>,
}

impl TimestampLtzColumn {
    pub(crate) fn to_py<'py>(&self, py: Python<'py>, row: usize) -> PyResult<Bound<'py, PyAny>> {
        match self.values.get(row, self.scale) {
            Ok(value) => {
                let tz = self.timezone.get(py)?;
                TimestampLtzMaterializer::materialize(py, value, &tz)
            }
            Err(ReadArrowError::NullValue { .. }) => Ok(py_none(py)),
            Err(err) => Err(PyValueError::new_err(err.to_string())),
        }
    }
}

struct TimestampLtzMaterializer;

impl TimestampLtzMaterializer {
    fn materialize<'py>(
        py: Python<'py>,
        value: NaiveDateTime,
        tz: &Bound<'py, PyTzInfo>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let utc_datetime = Utc.from_utc_datetime(&value);
        let year = utc_datetime.year();
        if !(1..=9999).contains(&year) {
            return Err(PyValueError::new_err(format!(
                "TIMESTAMP_LTZ {utc_datetime} is outside the datetime.datetime year range 1..=9999"
            )));
        }
        let datetime = utc_datetime
            .into_pyobject(py)?
            .into_any()
            .call_method1("astimezone", (tz,))?;
        let local_year: i32 = datetime.getattr("year")?.extract()?;
        if !(1..=9999).contains(&local_year) {
            return Err(PyValueError::new_err(format!(
                "TIMESTAMP_LTZ {datetime} is outside the datetime.datetime year range 1..=9999"
            )));
        }
        Ok(datetime)
    }
}

pub(super) fn from_column(
    array: &ArrayRef,
    field_type: &SnowflakeFieldType,
    scale: u32,
    timezone: Arc<TimezoneProvider>,
) -> PyResult<Column> {
    Ok(Column::TimestampLtz(TimestampLtzColumn {
        values: TimestampColumn::from_array(array, field_type)?,
        scale,
        timezone,
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
    use crate::arrow::converters::test_util::{assert_py_datetime_aware, assert_py_none};
    use crate::arrow::plan::SnowflakeFieldType;

    fn ltz(scale: u32) -> SnowflakeFieldType {
        SnowflakeFieldType::TimestampLtz { scale }
    }

    fn ltz_struct(epochs: Vec<Option<i64>>, fractions: Vec<Option<i32>>) -> ArrayRef {
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
    fn int64_converts_to_aware_python_datetime_with_nulls() {
        Python::initialize();
        let utc = ConversionContext::new(&Schema::empty()).unwrap();
        let new_york = ConversionContext::with_session_timezone(
            &Schema::empty(),
            Some("America/New_York".to_string()),
        )
        .unwrap();

        let scale0: ArrayRef = Arc::new(Int64Array::from(vec![
            Some(1_705_314_600),
            None,
            Some(1_705_314_600_123_456),
        ]));
        let utc_column = utc.converter_from_column(&scale0, &ltz(0)).unwrap();
        let utc_micros = utc.converter_from_column(&scale0, &ltz(6)).unwrap();
        let ny_column = new_york.converter_from_column(&scale0, &ltz(0)).unwrap();

        Python::attach(|py| {
            assert_py_datetime_aware(
                &utc_column.to_py(py, 0).unwrap(),
                (2024, 1, 15),
                (10, 30, 0, 0),
                "UTC",
            );
            assert_py_none(&utc_column.to_py(py, 1).unwrap());
            assert_py_datetime_aware(
                &utc_micros.to_py(py, 2).unwrap(),
                (2024, 1, 15),
                (10, 30, 0, 123_456),
                "UTC",
            );
            assert_py_datetime_aware(
                &ny_column.to_py(py, 0).unwrap(),
                (2024, 1, 15),
                (5, 30, 0, 0),
                "America/New_York",
            );
            assert_py_none(&ny_column.to_py(py, 1).unwrap());
        });
    }

    #[test]
    fn struct_converts_to_aware_python_datetime_with_nulls() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array = ltz_struct(
            vec![Some(1_705_314_600), None, Some(0)],
            vec![Some(123_456_789), None, Some(0)],
        );
        let column = ctx.converter_from_column(&array, &ltz(9)).unwrap();

        Python::attach(|py| {
            assert_py_datetime_aware(
                &column.to_py(py, 0).unwrap(),
                (2024, 1, 15),
                (10, 30, 0, 123_456),
                "UTC",
            );
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_py_datetime_aware(
                &column.to_py(py, 2).unwrap(),
                (1970, 1, 1),
                (0, 0, 0, 0),
                "UTC",
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
        let column = ctx.converter_from_column(&array, &ltz(0)).unwrap();

        Python::attach(|py| {
            assert_py_datetime_aware(
                &column.to_py(py, 0).unwrap(),
                (1, 1, 1),
                (0, 0, 0, 0),
                "UTC",
            );
            assert_py_datetime_aware(
                &column.to_py(py, 1).unwrap(),
                (9999, 12, 31),
                (23, 59, 59, 0),
                "UTC",
            );
        });
    }

    #[test]
    fn rejects_physical_mismatch_for_timestamp_ltz() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let cases: Vec<ArrayRef> = vec![
            Arc::new(Date32Array::from(vec![Some(0)])),
            Arc::new(Int32Array::from(vec![Some(0)])),
        ];
        for array in cases {
            let err = match ctx.converter_from_column(&array, &ltz(0)) {
                Ok(_) => panic!(
                    "expected physical mismatch for TIMESTAMP_LTZ, got {:?}",
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
                        && text.contains("TIMESTAMP_LTZ"),
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
        let column = ctx.converter_from_column(&array, &ltz(0)).unwrap();

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
        let column = ctx.converter_from_column(&array, &ltz(0)).unwrap();

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
