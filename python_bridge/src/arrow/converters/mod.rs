mod context;
mod decode;
mod util;

#[cfg(test)]
mod test_util;

use pyo3::prelude::*;

pub(crate) use context::ConversionContext;

pub(crate) enum Column {}

impl Column {
    pub(crate) fn to_py<'py>(&self, _py: Python<'py>, _row: usize) -> PyResult<Bound<'py, PyAny>> {
        match *self {}
    }
}
