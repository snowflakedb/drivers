use pyo3::prelude::*;
use pyo3::types::PyBool;

pub(crate) fn assert_py_bool(value: &Bound<'_, PyAny>, expected: bool) {
    assert!(
        value.is_instance_of::<PyBool>(),
        "expected Python bool, got {}",
        value.get_type().name().unwrap()
    );
    assert_eq!(value.extract::<bool>().unwrap(), expected);
}

pub(crate) fn assert_py_none(value: &Bound<'_, PyAny>) {
    assert!(value.is_none(), "expected None, got {value}");
}
