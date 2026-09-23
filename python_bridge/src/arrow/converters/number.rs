use std::sync::Arc;

use arrow::array::ArrayRef;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use sf_types::{ReadArrowError, SnowflakeFixed};

use super::Column;
use super::decode::PyMaterializer;
use super::numpy::NumpyProvider;
use crate::arrow::converters::util::{IntColumn, py_decimal_from_coeff_exp, py_none};
use crate::arrow::plan::SnowflakeFieldType;
use crate::arrow::scaled_f64::scaled_f64;

pub(crate) struct NumberColumn<M> {
    values: IntColumn,
    materializer: M,
}

impl<M: PyMaterializer<SnowflakeFixed>> NumberColumn<M> {
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
    numpy: Arc<NumpyProvider>,
    use_numpy: bool,
) -> PyResult<Column> {
    let values = IntColumn::from_fixed(array, field_type)?;
    // Numpy FIXED uses the same scale split as Python FIXED, but only on
    // integer physical widths: scale 0 is numpy.int64, scale > 0 is
    // numpy.float64. Decimal128 stays on the Python path because the
    // unscaled value can exceed int64 and float64 cannot carry 38 digits.
    if use_numpy && !matches!(values, IntColumn::Decimal128(_)) {
        if scale == 0 {
            return Ok(Column::NumberNumpyInt(NumberColumn {
                values,
                materializer: NumberNumpyIntMaterializer { numpy },
            }));
        }
        return Ok(Column::NumberNumpyFloat(NumberColumn {
            values,
            materializer: NumberNumpyFloatMaterializer { scale, numpy },
        }));
    }
    Ok(Column::Number(NumberColumn {
        values,
        materializer: NumberMaterializer { scale },
    }))
}

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

pub(crate) struct NumberNumpyIntMaterializer {
    numpy: Arc<NumpyProvider>,
}

impl PyMaterializer<SnowflakeFixed> for NumberNumpyIntMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, value: i128) -> PyResult<Bound<'py, PyAny>> {
        let value = i64::try_from(value)
            .map_err(|_| PyValueError::new_err("FIXED integer exceeds numpy.int64 range"))?;
        self.numpy.int64(py, value)
    }
}

pub(crate) struct NumberNumpyFloatMaterializer {
    scale: u32,
    numpy: Arc<NumpyProvider>,
}

impl PyMaterializer<SnowflakeFixed> for NumberNumpyFloatMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, value: i128) -> PyResult<Bound<'py, PyAny>> {
        self.numpy
            .float64(py, scaled_f64(value, -(self.scale as i32)))
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
    use crate::arrow::converters::test_util::{
        assert_np_float64, assert_np_int64, assert_py_decimal, assert_py_int, assert_py_none,
    };
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

    #[test]
    fn scale0_int_widths_convert_to_numpy_int64_with_nulls() {
        Python::initialize();
        let context = ConversionContext::with_numpy(&Schema::empty()).unwrap();

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
                let column = context
                    .converter_from_column(&array, &number(0, 38))
                    .unwrap();
                assert_np_int64(&column.to_py(py, 0).unwrap(), first);
                assert_py_none(&column.to_py(py, 1).unwrap());
                assert_np_int64(&column.to_py(py, 2).unwrap(), last);
            }
        });
    }

    #[test]
    fn scale0_decimal128_stays_python_int_with_numpy() {
        Python::initialize();
        let context = ConversionContext::with_numpy(&Schema::empty()).unwrap();

        let array: ArrayRef = Arc::new(
            Decimal128Array::from(vec![Some(0), None, Some(10_i128.pow(20))])
                .with_precision_and_scale(38, 0)
                .unwrap(),
        );
        let column = context
            .converter_from_column(&array, &number(0, 38))
            .unwrap();

        Python::attach(|py| {
            assert_py_int(&column.to_py(py, 0).unwrap(), 0);
            assert_py_none(&column.to_py(py, 1).unwrap());
            let big = column.to_py(py, 2).unwrap();
            assert!(
                big.is_instance_of::<PyInt>(),
                "expected Python int for Decimal128 with numpy, got {}",
                big.get_type().name().unwrap()
            );
            assert_eq!(big.str().unwrap().to_string(), "100000000000000000000");
        });
    }

    #[test]
    fn scale_gt0_int_widths_convert_to_numpy_float64() {
        Python::initialize();
        let context = ConversionContext::with_numpy(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Int64Array::from(vec![Some(123), None, Some(-123)]));
        let column = context
            .converter_from_column(&array, &number(2, 38))
            .unwrap();

        Python::attach(|py| {
            assert_np_float64(&column.to_py(py, 0).unwrap(), 1.23);
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_np_float64(&column.to_py(py, 2).unwrap(), -1.23);
        });
    }

    #[test]
    fn scale_gt0_decimal128_stays_python_decimal_with_numpy() {
        Python::initialize();
        let context = ConversionContext::with_numpy(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(
            Decimal128Array::from(vec![Some(123)])
                .with_precision_and_scale(38, 2)
                .unwrap(),
        );
        let column = context
            .converter_from_column(&array, &number(2, 38))
            .unwrap();

        Python::attach(|py| {
            assert_py_decimal(&column.to_py(py, 0).unwrap(), "1.23");
        });
    }

    #[test]
    fn scaled_numpy_float64_is_correctly_rounded() {
        Python::initialize();
        let context = ConversionContext::with_numpy(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Int64Array::from(vec![Some(123456789012345678_i64)]));
        let column = context
            .converter_from_column(&array, &number(3, 38))
            .unwrap();

        // 123456789012345.678 rounds to 123456789012345.67; `unscaled as f64 / 1e3`
        // rounds twice and lands on 123456789012345.69.
        Python::attach(|py| {
            assert_np_float64(&column.to_py(py, 0).unwrap(), 123456789012345.67);
        });
    }
}
