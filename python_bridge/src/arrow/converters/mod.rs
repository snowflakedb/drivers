mod binary;
mod boolean;
mod context;
mod decode;
mod number;
mod real;
mod text;
mod util;

#[cfg(test)]
mod test_util;

use arrow::array::{BinaryArray, BooleanArray, Float64Array, StringArray};
use pyo3::prelude::*;
use sf_types::{SnowflakeBinary, SnowflakeBoolean, SnowflakeReal, SnowflakeText};

use self::binary::BinaryMaterializer;
use self::boolean::BoolMaterializer;
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
}

impl Column {
    pub(crate) fn to_py<'py>(&self, py: Python<'py>, row: usize) -> PyResult<Bound<'py, PyAny>> {
        match self {
            Self::Bool(column) => column.to_py(py, row),
            Self::Number(column) => column.to_py(py, row),
            Self::Real(column) => column.to_py(py, row),
            Self::Text(column) => column.to_py(py, row),
            Self::Binary(column) => column.to_py(py, row),
        }
    }
}
