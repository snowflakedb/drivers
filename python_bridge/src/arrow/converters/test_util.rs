//! Shared assertion helpers for converter compliance tests.

use pyo3::prelude::*;

pub(crate) fn assert_py_none(value: &Bound<'_, PyAny>) {
    assert!(value.is_none(), "expected None, got {value}");
}
