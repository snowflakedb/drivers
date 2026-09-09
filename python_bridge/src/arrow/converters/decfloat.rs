use arrow::array::{ArrayRef, StructArray};
use pyo3::prelude::*;
use sf_types::SnowflakeDecfloat;

use super::Column;
use super::decode::{PyMaterializer, TypedColumn};
use crate::arrow::converters::util::{downcast_column, py_decimal_from_coeff_exp};
use crate::arrow::plan::SnowflakeFieldType;

pub(super) fn from_column(array: &ArrayRef, field_type: &SnowflakeFieldType) -> PyResult<Column> {
    downcast_column::<StructArray>(array, field_type).map(|array| {
        Column::Decfloat(TypedColumn::new(
            array,
            SnowflakeDecfloat,
            DecfloatMaterializer,
        ))
    })
}

pub(crate) struct DecfloatMaterializer;

impl PyMaterializer<SnowflakeDecfloat> for DecfloatMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, value: (i128, i16)) -> PyResult<Bound<'py, PyAny>> {
        let (significand, exponent) = value;
        py_decimal_from_coeff_exp(py, significand, i32::from(exponent))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{ArrayRef, BinaryArray, Float64Array, Int16Array, StructArray};
    use arrow::buffer::NullBuffer;
    use arrow::datatypes::{DataType, Field, Schema};
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

    use crate::arrow::converters::ConversionContext;
    use crate::arrow::converters::test_util::{assert_py_decimal, assert_py_none};
    use crate::arrow::plan::SnowflakeFieldType;

    fn decfloat() -> SnowflakeFieldType {
        SnowflakeFieldType::Decfloat { precision: 38 }
    }

    fn sig(value: i128) -> Vec<u8> {
        value.to_be_bytes().to_vec()
    }

    fn decfloat_struct(exponents: Vec<Option<i16>>, significands: Vec<Option<&[u8]>>) -> ArrayRef {
        let nulls: Vec<bool> = exponents.iter().map(Option::is_some).collect();
        let fields = vec![
            Field::new("exponent", DataType::Int16, true),
            Field::new("significand", DataType::Binary, true),
        ];
        Arc::new(
            StructArray::try_new(
                fields.into(),
                vec![
                    Arc::new(Int16Array::from(exponents)),
                    Arc::new(BinaryArray::from(significands)),
                ],
                Some(NullBuffer::from(nulls)),
            )
            .unwrap(),
        )
    }

    #[test]
    fn struct_array_converts_to_python_decimal_with_nulls() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let pos = sig(314_159);
        let neg = sig(-271_828);
        let array = decfloat_struct(
            vec![Some(-5), None, Some(-5)],
            vec![Some(&pos), None, Some(&neg)],
        );
        let column = ctx.converter_from_column(&array, &decfloat()).unwrap();

        Python::attach(|py| {
            assert_py_decimal(&column.to_py(py, 0).unwrap(), "3.14159");
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_py_decimal(&column.to_py(py, 2).unwrap(), "-2.71828");
        });
    }

    #[test]
    fn empty_significand_is_zero() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let array = decfloat_struct(vec![Some(5)], vec![Some(&[])]);
        let column = ctx.converter_from_column(&array, &decfloat()).unwrap();

        Python::attach(|py| {
            assert_py_decimal(&column.to_py(py, 0).unwrap(), "0E+5");
        });
    }

    #[test]
    fn extreme_exponents_convert_to_python_decimal() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let one = sig(1);
        let array = decfloat_struct(
            vec![Some(16_384), Some(-16_383)],
            vec![Some(&one), Some(&one)],
        );
        let column = ctx.converter_from_column(&array, &decfloat()).unwrap();

        Python::attach(|py| {
            assert_py_decimal(&column.to_py(py, 0).unwrap(), "1E+16384");
            assert_py_decimal(&column.to_py(py, 1).unwrap(), "1E-16383");
        });
    }

    #[test]
    fn preserves_38_digits_under_default_decimal_precision() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let unscaled = 12345678901234567890123456789012345678_i128;
        let bytes = sig(unscaled);
        let array = decfloat_struct(vec![Some(0)], vec![Some(&bytes)]);
        let column = ctx.converter_from_column(&array, &decfloat()).unwrap();

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
                "12345678901234567890123456789012345678",
            );
        });
    }

    #[test]
    fn rejects_physical_mismatch_for_decfloat() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Float64Array::from(vec![Some(1.0)]));
        let err = match ctx.converter_from_column(&array, &decfloat()) {
            Ok(_) => panic!("expected physical mismatch for Float64 DECFLOAT"),
            Err(err) => err,
        };
        Python::attach(|py| {
            assert!(
                err.is_instance_of::<PyValueError>(py),
                "expected PyValueError, got {err}"
            );
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(
                text.contains("logical/physical type mismatch") && text.contains("DECFLOAT"),
                "got {text}"
            );
        });
    }

    #[test]
    fn missing_struct_field_is_a_value_error() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let fields = vec![Field::new("exponent", DataType::Int16, true)];
        let array: ArrayRef = Arc::new(
            StructArray::try_new(
                fields.into(),
                vec![Arc::new(Int16Array::from(vec![Some(0)])) as _],
                None,
            )
            .unwrap(),
        );
        let column = ctx.converter_from_column(&array, &decfloat()).unwrap();

        Python::attach(|py| {
            let err = column.to_py(py, 0).unwrap_err();
            assert!(
                err.is_instance_of::<PyValueError>(py),
                "expected PyValueError, got {err}"
            );
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(
                text.contains("DECFLOAT struct missing 'significand' field"),
                "got {text}"
            );
        });
    }
}
