mod boolean;
mod context;
mod decode;
mod util;

#[cfg(test)]
mod test_util;

use arrow::array::BooleanArray;
use pyo3::prelude::*;

use sf_types::SnowflakeBoolean;

use self::boolean::BoolMaterializer;
use self::decode::TypedColumn;

pub(crate) use context::ConversionContext;

pub(crate) enum Column {
    Bool(TypedColumn<BooleanArray, SnowflakeBoolean, BoolMaterializer>),
}

impl Column {
    pub(crate) fn to_py<'py>(&self, py: Python<'py>, row: usize) -> PyResult<Bound<'py, PyAny>> {
        match self {
            Self::Bool(column) => column.to_py(py, row),
        }
    }
}
