use arrow::array::ArrayRef;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use sf_types::{ReadArrowError, SnowflakeFixed};

use super::Column;
use super::decode::PyMaterializer;
use crate::arrow::converters::util::{IntColumn, py_decimal_from_coeff_exp, py_none};
use crate::arrow::plan::SnowflakeFieldType;

pub(crate) struct NumberColumn {
    values: IntColumn,
    materializer: NumberMaterializer,
}

impl NumberColumn {
    pub(crate) fn to_py<'py>(&self, py: Python<'py>, row: usize) -> PyResult<Bound<'py, PyAny>> {
        match self.values.get(row) {
            Ok(value) => self.materializer.materialize(py, value),
            Err(ReadArrowError::NullValue { .. }) => Ok(py_none(py)),
            Err(err) => Err(PyValueError::new_err(err.to_string())),
        }
    }
}

pub(super) fn from_column(
    array: &ArrayRef,
    field_type: &SnowflakeFieldType,
    scale: u32,
) -> PyResult<Column> {
    Ok(Column::Number(NumberColumn {
        values: IntColumn::from_fixed(array, field_type)?,
        materializer: NumberMaterializer { scale },
    }))
}

#[derive(Clone, Copy)]
pub(crate) struct NumberMaterializer {
    scale: u32,
}

impl PyMaterializer<SnowflakeFixed> for NumberMaterializer {
    // The C++ nanoarrow converters split FIXED the same way: scale 0 is a
    // Python int, scale > 0 is decimal.Decimal((sign, digits, -scale)).
    fn materialize<'py>(&self, py: Python<'py>, value: i128) -> PyResult<Bound<'py, PyAny>> {
        if self.scale == 0 {
            return Ok(value.into_pyobject(py)?.into_any());
        }
        py_decimal_from_coeff_exp(py, value, -(self.scale as i32))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{
        ArrayRef, Decimal128Array, Float64Array, Int8Array, Int16Array, Int32Array, Int64Array,
    };
    use arrow::datatypes::Schema;
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;
    use pyo3::types::{PyFloat, PyInt};

    use crate::arrow::converters::ConversionContext;
    use crate::arrow::converters::test_util::{assert_py_decimal, assert_py_int, assert_py_none};
    use crate::arrow::plan::SnowflakeFieldType;

    fn number(scale: u32, precision: u32) -> SnowflakeFieldType {
        SnowflakeFieldType::Number { scale, precision }
    }

    #[test]
    fn scale0_int_widths_convert_to_python_int_with_nulls() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let cases: Vec<(ArrayRef, i64, i64)> = vec![
            (
                Arc::new(Int8Array::from(vec![Some(i8::MIN), None, Some(i8::MAX)])),
                i8::MIN as i64,
                i8::MAX as i64,
            ),
            (
                Arc::new(Int16Array::from(vec![Some(i16::MIN), None, Some(i16::MAX)])),
                i16::MIN as i64,
                i16::MAX as i64,
            ),
            (
                Arc::new(Int32Array::from(vec![Some(i32::MIN), None, Some(i32::MAX)])),
                i32::MIN as i64,
                i32::MAX as i64,
            ),
            (
                Arc::new(Int64Array::from(vec![Some(i64::MIN), None, Some(i64::MAX)])),
                i64::MIN,
                i64::MAX,
            ),
        ];

        Python::attach(|py| {
            for (array, first, last) in cases {
                let column = ctx.converter_from_column(&array, &number(0, 38)).unwrap();
                assert_py_int(&column.to_py(py, 0).unwrap(), first);
                assert_py_none(&column.to_py(py, 1).unwrap());
                assert_py_int(&column.to_py(py, 2).unwrap(), last);
            }
        });
    }

    #[test]
    fn scale0_decimal128_converts_to_python_int() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let array: ArrayRef = Arc::new(
            Decimal128Array::from(vec![Some(0), None, Some(-1), Some(10_i128.pow(20))])
                .with_precision_and_scale(38, 0)
                .unwrap(),
        );
        let column = ctx.converter_from_column(&array, &number(0, 38)).unwrap();

        Python::attach(|py| {
            assert_py_int(&column.to_py(py, 0).unwrap(), 0);
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_py_int(&column.to_py(py, 2).unwrap(), -1);
            let big = column.to_py(py, 3).unwrap();
            assert!(
                big.is_instance_of::<PyInt>(),
                "expected Python int for large Decimal128, got {}",
                big.get_type().name().unwrap()
            );
            assert_eq!(big.str().unwrap().to_string(), "100000000000000000000");
        });
    }

    #[test]
    fn scale_gt0_int_widths_convert_to_python_decimal() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let cases: Vec<(ArrayRef, &str, &str)> = vec![
            (
                Arc::new(Int8Array::from(vec![Some(12), None, Some(-12)])),
                "0.12",
                "-0.12",
            ),
            (
                Arc::new(Int16Array::from(vec![Some(12), None, Some(-12)])),
                "0.12",
                "-0.12",
            ),
            (
                Arc::new(Int32Array::from(vec![Some(12), None, Some(-12)])),
                "0.12",
                "-0.12",
            ),
            (
                Arc::new(Int64Array::from(vec![Some(123), None, Some(-123)])),
                "1.23",
                "-1.23",
            ),
        ];

        Python::attach(|py| {
            for (array, first, neg) in cases {
                let column = ctx.converter_from_column(&array, &number(2, 38)).unwrap();
                assert_py_decimal(&column.to_py(py, 0).unwrap(), first);
                assert_py_none(&column.to_py(py, 1).unwrap());
                assert_py_decimal(&column.to_py(py, 2).unwrap(), neg);
            }
        });
    }

    #[test]
    fn scale_gt0_decimal128_converts_to_python_decimal() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let array: ArrayRef = Arc::new(
            Decimal128Array::from(vec![Some(123), None, Some(-150), Some(10_i128.pow(20) + 5)])
                .with_precision_and_scale(38, 2)
                .unwrap(),
        );
        let column = ctx.converter_from_column(&array, &number(2, 38)).unwrap();

        Python::attach(|py| {
            assert_py_decimal(&column.to_py(py, 0).unwrap(), "1.23");
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_py_decimal(&column.to_py(py, 2).unwrap(), "-1.50");
            let big = column.to_py(py, 3).unwrap();
            assert_eq!(big.get_type().name().unwrap(), "Decimal");
            assert_py_decimal(&big, "1000000000000000000.05");
        });
    }

    #[test]
    fn scale_gt0_decimal128_preserves_38_digits_under_default_decimal_precision() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let unscaled = 12345678901234567890123456789012345678_i128;
        let array: ArrayRef = Arc::new(
            Decimal128Array::from(vec![Some(unscaled)])
                .with_precision_and_scale(38, 2)
                .unwrap(),
        );
        let column = ctx.converter_from_column(&array, &number(2, 38)).unwrap();

        Python::attach(|py| {
            py.import("decimal")
                .unwrap()
                .getattr("getcontext")
                .unwrap()
                .call0()
                .unwrap()
                .setattr("prec", 28)
                .unwrap();
            assert_py_decimal(
                &column.to_py(py, 0).unwrap(),
                "123456789012345678901234567890123456.78",
            );
        });
    }

    #[test]
    fn scale_gt0_keeps_trailing_zeros_as_decimal_not_int_or_float() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let array: ArrayRef = Arc::new(Int64Array::from(vec![Some(100)]));
        let column = ctx.converter_from_column(&array, &number(2, 38)).unwrap();

        Python::attach(|py| {
            let value = column.to_py(py, 0).unwrap();
            assert_eq!(value.get_type().name().unwrap(), "Decimal");
            assert!(!value.is_instance_of::<PyInt>());
            assert!(!value.is_instance_of::<PyFloat>());
            assert_py_decimal(&value, "1.00");
        });
    }

    #[test]
    fn rejects_physical_mismatch_for_fixed() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Float64Array::from(vec![Some(1.0)]));
        let err = match ctx.converter_from_column(&array, &number(0, 38)) {
            Ok(_) => panic!("expected physical mismatch for Float64 FIXED"),
            Err(err) => err,
        };
        Python::attach(|py| {
            assert!(
                err.is_instance_of::<PyValueError>(py),
                "expected PyValueError, got {err}"
            );
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(
                text.contains("logical/physical type mismatch") && text.contains("FIXED"),
                "got {text}"
            );
        });
    }
}
