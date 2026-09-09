use arrow::array::{
    Array, ArrayRef, Decimal128Array, Int8Array, Int16Array, Int32Array, Int64Array, StructArray,
};
use arrow::datatypes::DataType;
use chrono::NaiveDateTime;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyNone, PyTuple};
use sf_types::{
    ReadArrowError, ReadArrowType, SnowflakeFixed, read_scaled_timestamp, read_struct_timestamp,
};

use crate::arrow::plan::SnowflakeFieldType;

pub(super) fn logical_mismatch_err(field_type: &SnowflakeFieldType, actual: &DataType) -> PyErr {
    PyValueError::new_err(format!(
        "logical/physical type mismatch: logical {}, got {actual:?}",
        field_type.logical_type_name()
    ))
}

pub(super) fn downcast_column<T: Array + Clone + 'static>(
    array: &ArrayRef,
    field_type: &SnowflakeFieldType,
) -> PyResult<T> {
    array
        .as_any()
        .downcast_ref::<T>()
        .cloned()
        .ok_or_else(|| logical_mismatch_err(field_type, array.data_type()))
}

pub(super) enum IntColumn {
    I8(Int8Array),
    I16(Int16Array),
    I32(Int32Array),
    I64(Int64Array),
    Decimal128(Decimal128Array),
}

impl IntColumn {
    pub(super) fn from_fixed(array: &ArrayRef, field_type: &SnowflakeFieldType) -> PyResult<Self> {
        Ok(match array.data_type() {
            DataType::Int8 => Self::I8(downcast_column(array, field_type)?),
            DataType::Int16 => Self::I16(downcast_column(array, field_type)?),
            DataType::Int32 => Self::I32(downcast_column(array, field_type)?),
            DataType::Int64 => Self::I64(downcast_column(array, field_type)?),
            DataType::Decimal128(_, _) => Self::Decimal128(downcast_column(array, field_type)?),
            other => return Err(logical_mismatch_err(field_type, other)),
        })
    }

    pub(super) fn get(&self, row: usize) -> Result<i128, ReadArrowError> {
        match self {
            Self::I8(array) => SnowflakeFixed.read_arrow_type(array, row),
            Self::I16(array) => SnowflakeFixed.read_arrow_type(array, row),
            Self::I32(array) => SnowflakeFixed.read_arrow_type(array, row),
            Self::I64(array) => SnowflakeFixed.read_arrow_type(array, row),
            Self::Decimal128(array) => SnowflakeFixed.read_arrow_type(array, row),
        }
    }
}

pub(super) enum TimestampColumn {
    Int64(Int64Array),
    Struct(StructArray),
}

impl TimestampColumn {
    pub(super) fn from_array(array: &ArrayRef, field_type: &SnowflakeFieldType) -> PyResult<Self> {
        Ok(match array.data_type() {
            DataType::Int64 => Self::Int64(downcast_column(array, field_type)?),
            DataType::Struct(_) => Self::Struct(downcast_column(array, field_type)?),
            other => return Err(logical_mismatch_err(field_type, other)),
        })
    }

    pub(super) fn get(&self, row: usize, scale: u32) -> Result<NaiveDateTime, ReadArrowError> {
        match self {
            Self::Int64(array) => read_scaled_timestamp(array, row, scale),
            Self::Struct(array) => read_struct_timestamp(array, row),
        }
    }
}

#[inline]
pub(super) fn py_none(py: Python<'_>) -> Bound<'_, PyAny> {
    PyNone::get(py).to_owned().into_any()
}

pub(super) fn py_decimal_from_coeff_exp<'py>(
    py: Python<'py>,
    coefficient: i128,
    exponent: i32,
) -> PyResult<Bound<'py, PyAny>> {
    let sign = u8::from(coefficient.is_negative());
    let mut digits = [0u8; 39];
    let mut n = coefficient.unsigned_abs();
    let mut start = digits.len();
    if n == 0 {
        start -= 1;
    } else {
        while n > 0 {
            start -= 1;
            digits[start] = (n % 10) as u8;
            n /= 10;
        }
    }
    let coeff = PyTuple::new(py, digits[start..].iter().copied())?;
    // CPython has no PyDecimal C API today; this constructs Decimal((sign, digits, exponent)).
    decimal_type(py)?.call1(((sign, coeff, exponent),))
}

fn decimal_type(py: Python<'_>) -> PyResult<&Bound<'_, PyAny>> {
    static DECIMAL: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
    let ty = DECIMAL.get_or_try_init(py, || {
        let decimal = py.import("decimal")?.getattr("Decimal")?.unbind();
        Ok::<_, PyErr>(decimal)
    })?;
    Ok(ty.bind(py))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{ArrayRef, Int32Array, Int64Array};
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

    use super::downcast_column;
    use crate::arrow::plan::SnowflakeFieldType;

    #[test]
    fn downcast_column_accepts_matching_physical_type() {
        Python::initialize();
        let array: ArrayRef = Arc::new(Int32Array::from(vec![Some(7)]));
        let field_type = SnowflakeFieldType::Number {
            scale: 0,
            precision: 38,
        };
        let got = downcast_column::<Int32Array>(&array, &field_type).unwrap();
        assert_eq!(got.value(0), 7);
    }

    #[test]
    fn downcast_column_rejects_physical_mismatch() {
        Python::initialize();
        let array: ArrayRef = Arc::new(Int64Array::from(vec![Some(7)]));
        let field_type = SnowflakeFieldType::Number {
            scale: 0,
            precision: 38,
        };
        let err = downcast_column::<Int32Array>(&array, &field_type).unwrap_err();
        Python::attach(|py| {
            assert!(
                err.is_instance_of::<PyValueError>(py),
                "expected PyValueError, got {err}"
            );
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(
                text.contains("logical/physical type mismatch")
                    && text.contains("Int64")
                    && text.contains("FIXED"),
                "got {text}"
            );
        });
    }
}
