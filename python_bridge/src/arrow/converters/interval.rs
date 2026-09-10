use arrow::array::ArrayRef;
use chrono::Duration;
use pyo3::exceptions::{PyOverflowError, PyValueError};
use pyo3::prelude::*;
use sf_types::ReadArrowError;

use super::Column;
use crate::arrow::converters::util::{IntColumn, py_none};
use crate::arrow::plan::SnowflakeFieldType;

// YEAR-MONTH is a signed interval str and DAY-TIME is datetime.timedelta
// (nanoseconds floored to microseconds). That is the same split as the old
// Python Arrow converters.

pub(crate) struct IntervalYearMonthColumn {
    values: IntColumn,
    scale: u32,
}

impl IntervalYearMonthColumn {
    pub(crate) fn to_py<'py>(&self, py: Python<'py>, row: usize) -> PyResult<Bound<'py, PyAny>> {
        match self.values.get(row) {
            Ok(months) => Ok(year_month_to_string(months, self.scale)
                .into_pyobject(py)?
                .into_any()),
            Err(ReadArrowError::NullValue { .. }) => Ok(py_none(py)),
            Err(err) => Err(PyValueError::new_err(err.to_string())),
        }
    }
}

pub(crate) struct IntervalDayTimeColumn {
    values: IntColumn,
}

impl IntervalDayTimeColumn {
    pub(crate) fn to_py<'py>(&self, py: Python<'py>, row: usize) -> PyResult<Bound<'py, PyAny>> {
        match self.values.get(row) {
            Ok(nanos) => timedelta_from_nanos(py, nanos),
            Err(ReadArrowError::NullValue { .. }) => Ok(py_none(py)),
            Err(err) => Err(PyValueError::new_err(err.to_string())),
        }
    }
}

/// Convert a year-month interval to a string.
///
/// `months`: The year-month interval value in months.
/// `scale`: The scale of the interval which represents subtype as follows:
/// - 0: INTERVAL YEAR TO MONTH
/// - 1: INTERVAL YEAR
/// - 2: INTERVAL MONTH
///
/// Returns the string representation of the interval.
fn year_month_to_string(months: i128, scale: u32) -> String {
    let sign = if months >= 0 { '+' } else { '-' };
    let interval = months.unsigned_abs();
    if scale == 2 {
        return format!("{sign}{interval}");
    }
    let years = interval / 12;
    if scale == 1 {
        return format!("{sign}{years}");
    }
    let remaining_months = interval % 12;
    format!("{sign}{years}-{remaining_months:02}")
}

fn timedelta_from_nanos(py: Python<'_>, nanos: i128) -> PyResult<Bound<'_, PyAny>> {
    let micros = i64::try_from(nanos.div_euclid(1000))
        .map_err(|_| PyOverflowError::new_err("timedelta duration out of range"))?;
    Duration::microseconds(micros)
        .into_pyobject(py)
        .map(Bound::into_any)
}

pub(super) fn year_month_from_column(
    array: &ArrayRef,
    field_type: &SnowflakeFieldType,
    scale: u32,
) -> PyResult<Column> {
    Ok(Column::IntervalYearMonth(IntervalYearMonthColumn {
        values: IntColumn::from_fixed(array, field_type)?,
        scale,
    }))
}

pub(super) fn day_time_from_column(
    array: &ArrayRef,
    field_type: &SnowflakeFieldType,
) -> PyResult<Column> {
    Ok(Column::IntervalDayTime(IntervalDayTimeColumn {
        values: IntColumn::from_fixed(array, field_type)?,
    }))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{
        ArrayRef, Date32Array, Decimal128Array, Int8Array, Int16Array, Int32Array, Int64Array,
    };
    use arrow::datatypes::Schema;
    use pyo3::exceptions::{PyOverflowError, PyValueError};
    use pyo3::prelude::*;

    use crate::arrow::converters::ConversionContext;
    use crate::arrow::converters::test_util::{assert_py_none, assert_py_str, assert_py_timedelta};
    use crate::arrow::plan::SnowflakeFieldType;

    fn year_month(scale: u32) -> SnowflakeFieldType {
        SnowflakeFieldType::IntervalYearMonth { scale }
    }

    #[test]
    fn year_month_converts_to_string_with_nulls() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Int64Array::from(vec![Some(14), None, Some(-14), Some(0)]));
        let column = ctx.converter_from_column(&array, &year_month(0)).unwrap();

        Python::attach(|py| {
            assert_py_str(&column.to_py(py, 0).unwrap(), "+1-02");
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_py_str(&column.to_py(py, 2).unwrap(), "-1-02");
            assert_py_str(&column.to_py(py, 3).unwrap(), "+0-00");
        });
    }

    #[test]
    fn year_month_scale_selects_year_or_month_literal() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Int32Array::from(vec![Some(12), Some(-24), Some(5)]));

        let year = ctx.converter_from_column(&array, &year_month(1)).unwrap();
        let month = ctx.converter_from_column(&array, &year_month(2)).unwrap();

        Python::attach(|py| {
            assert_py_str(&year.to_py(py, 0).unwrap(), "+1");
            assert_py_str(&year.to_py(py, 1).unwrap(), "-2");
            assert_py_str(&month.to_py(py, 2).unwrap(), "+5");
        });
    }

    #[test]
    fn year_month_integer_widths_convert() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let cases: Vec<(ArrayRef, &str)> = vec![
            (Arc::new(Int8Array::from(vec![Some(7)])), "+0-07"),
            (Arc::new(Int16Array::from(vec![Some(24)])), "+2-00"),
            (Arc::new(Int32Array::from(vec![Some(-1)])), "-0-01"),
            (Arc::new(Int64Array::from(vec![Some(100)])), "+8-04"),
            (Arc::new(Decimal128Array::from(vec![Some(12i128)])), "+1-00"),
        ];

        Python::attach(|py| {
            for (array, expected) in cases {
                let column = ctx.converter_from_column(&array, &year_month(0)).unwrap();
                assert_py_str(&column.to_py(py, 0).unwrap(), expected);
            }
        });
    }

    #[test]
    fn day_time_converts_to_timedelta_with_nulls() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Int64Array::from(vec![
            Some(1_000_000_000),
            None,
            Some(0),
            Some(1_500_000),
            Some(86_400_000_000_000),
        ]));
        let column = ctx
            .converter_from_column(&array, &SnowflakeFieldType::IntervalDayTime)
            .unwrap();

        Python::attach(|py| {
            assert_py_timedelta(&column.to_py(py, 0).unwrap(), 0, 1, 0);
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_py_timedelta(&column.to_py(py, 2).unwrap(), 0, 0, 0);
            assert_py_timedelta(&column.to_py(py, 3).unwrap(), 0, 0, 1500);
            assert_py_timedelta(&column.to_py(py, 4).unwrap(), 1, 0, 0);
        });
    }

    #[test]
    fn day_time_floors_nanoseconds_toward_negative_infinity() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Int64Array::from(vec![Some(999), Some(-1)]));
        let column = ctx
            .converter_from_column(&array, &SnowflakeFieldType::IntervalDayTime)
            .unwrap();

        Python::attach(|py| {
            assert_py_timedelta(&column.to_py(py, 0).unwrap(), 0, 0, 0);
            assert_py_timedelta(&column.to_py(py, 1).unwrap(), -1, 86399, 999_999);
        });
    }

    #[test]
    fn day_time_decimal128_beyond_i64_converts() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let nanos = i64::MAX as i128 + 1_000;
        let array: ArrayRef = Arc::new(Decimal128Array::from(vec![Some(nanos)]));
        let column = ctx
            .converter_from_column(&array, &SnowflakeFieldType::IntervalDayTime)
            .unwrap();

        Python::attach(|py| {
            let value = column.to_py(py, 0).unwrap();
            let kwargs = pyo3::types::PyDict::new(py);
            kwargs
                .set_item("microseconds", nanos.div_euclid(1000))
                .unwrap();
            let expected = py
                .import("datetime")
                .unwrap()
                .getattr("timedelta")
                .unwrap()
                .call((), Some(&kwargs))
                .unwrap();
            assert!(
                value.eq(&expected).unwrap(),
                "got {value}, expected {expected}"
            );
        });
    }

    #[test]
    fn day_time_overflows_python_timedelta_range() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let nanos = 1_000_000_000i128 * 86_400 * 1_000_000_000;
        let array: ArrayRef = Arc::new(Decimal128Array::from(vec![Some(nanos)]));
        let column = ctx
            .converter_from_column(&array, &SnowflakeFieldType::IntervalDayTime)
            .unwrap();

        Python::attach(|py| {
            let err = column.to_py(py, 0).unwrap_err();
            assert!(
                err.is_instance_of::<PyOverflowError>(py),
                "expected OverflowError, got {err}"
            );
        });
    }

    #[test]
    fn rejects_physical_mismatch_for_interval() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Date32Array::from(vec![Some(0)]));

        for field_type in [year_month(0), SnowflakeFieldType::IntervalDayTime] {
            let err = match ctx.converter_from_column(&array, &field_type) {
                Ok(_) => panic!("expected physical mismatch for {field_type:?}"),
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
                        && text.contains(field_type.logical_type_name()),
                    "got {text}"
                );
            });
        }
    }
}
