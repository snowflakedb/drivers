mod boolean;
mod context;
mod decode;
mod real;
mod util;

#[cfg(test)]
mod test_util;

use arrow::array::{BooleanArray, Float64Array};
use pyo3::prelude::*;
use sf_types::{SnowflakeBoolean, SnowflakeReal};

use self::boolean::BoolMaterializer;
use self::decode::TypedColumn;
use self::real::RealMaterializer;

pub(crate) use context::ConversionContext;

pub(crate) enum Column {
    Bool(TypedColumn<BooleanArray, SnowflakeBoolean, BoolMaterializer>),
    Real(TypedColumn<Float64Array, SnowflakeReal, RealMaterializer>),
}

impl Column {
    pub(crate) fn to_py<'py>(&self, py: Python<'py>, row: usize) -> PyResult<Bound<'py, PyAny>> {
        match self {
            Self::Bool(column) => column.to_py(py, row),
            Self::Real(column) => column.to_py(py, row),
        }
    }
}
