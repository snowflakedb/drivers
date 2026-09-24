use std::sync::Arc;

use arrow::array::ArrayRef;
use chrono::Duration;
use pyo3::exceptions::{PyOverflowError, PyValueError};
use pyo3::prelude::*;
use sf_types::{ReadArrowError, SnowflakeFixed};

use super::Column;
use super::decode::PyMaterializer;
use super::numpy::NumpyProvider;
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

pub(crate) struct IntervalNumpyColumn<M> {
    values: IntColumn,
    materializer: M,
}

impl<M: PyMaterializer<SnowflakeFixed>> IntervalNumpyColumn<M> {
    pub(crate) fn to_py<'py>(&self, py: Python<'py>, row: usize) -> PyResult<Bound<'py, PyAny>> {
        match self.values.get(row) {
            Ok(value) => self.materializer.materialize(py, value),
            Err(ReadArrowError::NullValue { .. }) => Ok(py_none(py)),
            Err(err) => Err(PyValueError::new_err(err.to_string())),
        }
    }
}

pub(crate) struct IntervalYearMonthNumpyYMaterializer {
    numpy: Arc<NumpyProvider>,
}

impl PyMaterializer<SnowflakeFixed> for IntervalYearMonthNumpyYMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, months: i128) -> PyResult<Bound<'py, PyAny>> {
        self.numpy
            .timedelta64_y(py, i64_timedelta_count(months.div_euclid(12))?)
    }
}

pub(crate) struct IntervalYearMonthNumpyMMaterializer {
    numpy: Arc<NumpyProvider>,
}

impl PyMaterializer<SnowflakeFixed> for IntervalYearMonthNumpyMMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, months: i128) -> PyResult<Bound<'py, PyAny>> {
        self.numpy.timedelta64_m(py, i64_timedelta_count(months)?)
    }
}

pub(crate) struct IntervalDayTimeNumpyNsMaterializer {
    numpy: Arc<NumpyProvider>,
}

impl PyMaterializer<SnowflakeFixed> for IntervalDayTimeNumpyNsMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, nanos: i128) -> PyResult<Bound<'py, PyAny>> {
        self.numpy.timedelta64_ns(py, i64_timedelta_count(nanos)?)
    }
}

pub(crate) struct IntervalDayTimeNumpyMsMaterializer {
    numpy: Arc<NumpyProvider>,
}

impl PyMaterializer<SnowflakeFixed> for IntervalDayTimeNumpyMsMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, nanos: i128) -> PyResult<Bound<'py, PyAny>> {
        self.numpy
            .timedelta64_ms(py, i64_timedelta_count(nanos.div_euclid(1_000_000))?)
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

fn i64_timedelta_count(value: i128) -> PyResult<i64> {
    i64::try_from(value)
        .map_err(|_| PyOverflowError::new_err("numpy.timedelta64 duration out of range"))
}

pub(super) fn year_month_from_column(
    array: &ArrayRef,
    field_type: &SnowflakeFieldType,
    scale: u32,
    numpy: Arc<NumpyProvider>,
    use_numpy: bool,
) -> PyResult<Column> {
    let values = IntColumn::from_fixed(array, field_type)?;
    // Numpy YEAR-MONTH uses scale 1 as timedelta64[Y] (months/12) and every
    // other scale as timedelta64[M].
    if use_numpy {
        if scale == 1 {
            return Ok(Column::IntervalYearMonthNumpyY(IntervalNumpyColumn {
                values,
                materializer: IntervalYearMonthNumpyYMaterializer { numpy },
            }));
        }
        return Ok(Column::IntervalYearMonthNumpyM(IntervalNumpyColumn {
            values,
            materializer: IntervalYearMonthNumpyMMaterializer { numpy },
        }));
    }
    Ok(Column::IntervalYearMonth(IntervalYearMonthColumn {
        values,
        scale,
    }))
}

pub(super) fn day_time_from_column(
    array: &ArrayRef,
    field_type: &SnowflakeFieldType,
    numpy: Arc<NumpyProvider>,
    use_numpy: bool,
) -> PyResult<Column> {
    let values = IntColumn::from_fixed(array, field_type)?;
    // Numpy DAY-TIME uses integer widths as timedelta64[ns] and Decimal128 as
    // timedelta64[ms] because the full nanosecond range does not fit in int64.
    if use_numpy {
        if matches!(values, IntColumn::Decimal128(_)) {
            return Ok(Column::IntervalDayTimeNumpyMs(IntervalNumpyColumn {
                values,
                materializer: IntervalDayTimeNumpyMsMaterializer { numpy },
            }));
        }
        return Ok(Column::IntervalDayTimeNumpyNs(IntervalNumpyColumn {
            values,
            materializer: IntervalDayTimeNumpyNsMaterializer { numpy },
        }));
    }
    Ok(Column::IntervalDayTime(IntervalDayTimeColumn { values }))
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
    use crate::arrow::converters::test_util::{
        assert_np_timedelta64, assert_py_none, assert_py_str, assert_py_timedelta,
    };
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
    fn year_month_converts_to_numpy_timedelta64_with_nulls() {
        Python::initialize();
        let context = ConversionContext::with_numpy(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Int64Array::from(vec![Some(14), None, Some(-14), Some(0)]));
        let column = context
            .converter_from_column(&array, &year_month(0))
            .unwrap();

        Python::attach(|py| {
            assert_np_timedelta64(&column.to_py(py, 0).unwrap(), 14, "M");
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_np_timedelta64(&column.to_py(py, 2).unwrap(), -14, "M");
            assert_np_timedelta64(&column.to_py(py, 3).unwrap(), 0, "M");
        });
    }

    #[test]
    fn year_month_scale_selects_numpy_year_or_month_unit() {
        Python::initialize();
        let context = ConversionContext::with_numpy(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Int32Array::from(vec![Some(12), Some(-24), Some(5)]));

        let year = context
            .converter_from_column(&array, &year_month(1))
            .unwrap();
        let month = context
            .converter_from_column(&array, &year_month(2))
            .unwrap();

        Python::attach(|py| {
            assert_np_timedelta64(&year.to_py(py, 0).unwrap(), 1, "Y");
            assert_np_timedelta64(&year.to_py(py, 1).unwrap(), -2, "Y");
            assert_np_timedelta64(&month.to_py(py, 2).unwrap(), 5, "M");
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
    fn day_time_int64_converts_to_numpy_timedelta64_ns_with_nulls() {
        Python::initialize();
        let context = ConversionContext::with_numpy(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Int64Array::from(vec![
            Some(1_000_000_000),
            None,
            Some(0),
            Some(1_234_567_890),
        ]));
        let column = context
            .converter_from_column(&array, &SnowflakeFieldType::IntervalDayTime)
            .unwrap();

        Python::attach(|py| {
            assert_np_timedelta64(&column.to_py(py, 0).unwrap(), 1_000_000_000, "ns");
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_np_timedelta64(&column.to_py(py, 2).unwrap(), 0, "ns");
            assert_np_timedelta64(&column.to_py(py, 3).unwrap(), 1_234_567_890, "ns");
        });
    }

    #[test]
    fn day_time_decimal128_converts_to_numpy_timedelta64_ms_with_floor() {
        Python::initialize();
        let context = ConversionContext::with_numpy(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Decimal128Array::from(vec![
            Some(1_000_000_000),
            Some(1_234_999_000),
            Some(-1_234_999_000),
        ]));
        let column = context
            .converter_from_column(&array, &SnowflakeFieldType::IntervalDayTime)
            .unwrap();

        Python::attach(|py| {
            assert_np_timedelta64(&column.to_py(py, 0).unwrap(), 1000, "ms");
            assert_np_timedelta64(&column.to_py(py, 1).unwrap(), 1234, "ms");
            assert_np_timedelta64(&column.to_py(py, 2).unwrap(), -1235, "ms");
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
    fn day_time_decimal128_overflows_numpy_timedelta64_range() {
        Python::initialize();
        let context = ConversionContext::with_numpy(&Schema::empty()).unwrap();
        let nanos = (i64::MAX as i128 + 1) * 1_000_000;
        let array: ArrayRef = Arc::new(Decimal128Array::from(vec![Some(nanos)]));
        let column = context
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
