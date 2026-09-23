use std::sync::Arc;

use arrow::array::{ArrayRef, Date32Array};
use chrono::{Datelike, NaiveDate};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use sf_types::SnowflakeDate;

use super::Column;
use super::decode::{PyMaterializer, TypedColumn};
use super::numpy::NumpyProvider;
use crate::arrow::converters::util::downcast_column;
use crate::arrow::plan::SnowflakeFieldType;

pub(super) fn from_column(
    array: &ArrayRef,
    field_type: &SnowflakeFieldType,
    numpy: Arc<NumpyProvider>,
    use_numpy: bool,
) -> PyResult<Column> {
    let array = downcast_column::<Date32Array>(array, field_type)?;
    if use_numpy {
        Ok(Column::DateNumpy(TypedColumn::new(
            array,
            SnowflakeDate,
            DateNumpyMaterializer { numpy },
        )))
    } else {
        Ok(Column::Date(TypedColumn::new(
            array,
            SnowflakeDate,
            DateMaterializer,
        )))
    }
}

pub(crate) struct DateMaterializer;

impl PyMaterializer<SnowflakeDate> for DateMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, value: NaiveDate) -> PyResult<Bound<'py, PyAny>> {
        let year = value.year();
        if !(1..=9999).contains(&year) {
            return Err(PyValueError::new_err(format!(
                "DATE {value} is outside the datetime.date year range 1..=9999"
            )));
        }
        Ok(value.into_pyobject(py)?.into_any())
    }
}

pub(crate) struct DateNumpyMaterializer {
    numpy: Arc<NumpyProvider>,
}

impl PyMaterializer<SnowflakeDate> for DateNumpyMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, value: NaiveDate) -> PyResult<Bound<'py, PyAny>> {
        let epoch =
            NaiveDate::from_ymd_opt(1970, 1, 1).expect("1970-01-01 is a valid Gregorian date");
        self.numpy
            .datetime64_d(py, value.signed_duration_since(epoch).num_days())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{ArrayRef, Date32Array, Int32Array};
    use arrow::datatypes::Schema;
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

    use crate::arrow::converters::ConversionContext;
    use crate::arrow::converters::test_util::{
        assert_np_datetime64_d, assert_py_date, assert_py_none,
    };
    use crate::arrow::plan::SnowflakeFieldType;

    #[test]
    fn date_converts_to_python_date_with_nulls() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let array: ArrayRef = Arc::new(Date32Array::from(vec![
            Some(0),
            None,
            Some(-1),
            Some(-719162),
            Some(2932896),
        ]));
        let column = ctx
            .converter_from_column(&array, &SnowflakeFieldType::Date)
            .unwrap();

        Python::attach(|py| {
            assert_py_date(&column.to_py(py, 0).unwrap(), 1970, 1, 1);
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_py_date(&column.to_py(py, 2).unwrap(), 1969, 12, 31);
            assert_py_date(&column.to_py(py, 3).unwrap(), 1, 1, 1);
            assert_py_date(&column.to_py(py, 4).unwrap(), 9999, 12, 31);
        });
    }

    #[test]
    fn date_converts_to_numpy_datetime64_d_with_nulls() {
        Python::initialize();
        let context = ConversionContext::with_numpy(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Date32Array::from(vec![
            Some(0),
            None,
            Some(-1),
            Some(-719162),
            Some(2932896),
        ]));
        let column = context
            .converter_from_column(&array, &SnowflakeFieldType::Date)
            .unwrap();

        Python::attach(|py| {
            assert_np_datetime64_d(&column.to_py(py, 0).unwrap(), 0);
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_np_datetime64_d(&column.to_py(py, 2).unwrap(), -1);
            assert_np_datetime64_d(&column.to_py(py, 3).unwrap(), -719162);
            assert_np_datetime64_d(&column.to_py(py, 4).unwrap(), 2932896);
        });
    }

    #[test]
    fn rejects_physical_mismatch_for_date() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Int32Array::from(vec![Some(0)]));
        let err = match ctx.converter_from_column(&array, &SnowflakeFieldType::Date) {
            Ok(_) => panic!("expected physical mismatch for Int32 DATE"),
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
                    && text.contains("DATE")
                    && text.contains("Int32"),
                "got {text}"
            );
        });
    }

    #[test]
    fn rejects_day_offset_outside_calendar_range() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Date32Array::from(vec![Some(i32::MAX)]));
        let column = ctx
            .converter_from_column(&array, &SnowflakeFieldType::Date)
            .unwrap();

        Python::attach(|py| {
            let err = column.to_py(py, 0).unwrap_err();
            assert!(
                err.is_instance_of::<PyValueError>(py),
                "expected PyValueError, got {err}"
            );
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(
                text.contains("Invalid Arrow value") && text.contains("DATE"),
                "got {text}"
            );
        });
    }

    #[test]
    fn rejects_dates_outside_python_date_year_range() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Date32Array::from(vec![Some(-719163), Some(2932897)]));
        let column = ctx
            .converter_from_column(&array, &SnowflakeFieldType::Date)
            .unwrap();

        Python::attach(|py| {
            for row in [0, 1] {
                let err = column.to_py(py, row).unwrap_err();
                assert!(
                    err.is_instance_of::<PyValueError>(py),
                    "expected PyValueError, got {err}"
                );
                let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
                assert!(
                    text.contains("datetime.date year range 1..=9999"),
                    "got {text}"
                );
            }
        });
    }
}
