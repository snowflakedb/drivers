mod binary;
mod boolean;
mod context;
mod date;
mod decfloat;
mod decode;
mod number;
mod real;
mod text;
mod util;

#[cfg(test)]
mod test_util;

use arrow::array::{
    BinaryArray, BooleanArray, Date32Array, Float64Array, StringArray, StructArray,
};
use pyo3::prelude::*;
use sf_types::{
    SnowflakeBinary, SnowflakeBoolean, SnowflakeDate, SnowflakeDecfloat, SnowflakeReal,
    SnowflakeText,
};

use self::binary::BinaryMaterializer;
use self::boolean::BoolMaterializer;
use self::date::DateMaterializer;
use self::decfloat::DecfloatMaterializer;
use self::decode::TypedColumn;
use self::number::NumberColumn;
use self::real::RealMaterializer;
use self::text::TextMaterializer;

pub(crate) use context::ConversionContext;

pub(crate) enum Column {
    Bool(TypedColumn<BooleanArray, SnowflakeBoolean, BoolMaterializer>),
    Number(NumberColumn),
    Real(TypedColumn<Float64Array, SnowflakeReal, RealMaterializer>),
    Text(TypedColumn<StringArray, SnowflakeText, TextMaterializer>),
    Binary(TypedColumn<BinaryArray, SnowflakeBinary, BinaryMaterializer>),
    Decfloat(TypedColumn<StructArray, SnowflakeDecfloat, DecfloatMaterializer>),
    Date(TypedColumn<Date32Array, SnowflakeDate, DateMaterializer>),
}

impl Column {
    pub(crate) fn to_py<'py>(&self, py: Python<'py>, row: usize) -> PyResult<Bound<'py, PyAny>> {
        match self {
            Self::Bool(column) => column.to_py(py, row),
            Self::Number(column) => column.to_py(py, row),
            Self::Real(column) => column.to_py(py, row),
            Self::Text(column) => column.to_py(py, row),
            Self::Binary(column) => column.to_py(py, row),
            Self::Decfloat(column) => column.to_py(py, row),
            Self::Date(column) => column.to_py(py, row),
        }
    }
}
