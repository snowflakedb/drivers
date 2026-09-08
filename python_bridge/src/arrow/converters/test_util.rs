use pyo3::prelude::*;
use pyo3::types::{PyBool, PyByteArray, PyFloat, PyString};

pub(crate) fn assert_py_bool(value: &Bound<'_, PyAny>, expected: bool) {
    assert!(
        value.is_instance_of::<PyBool>(),
        "expected Python bool, got {}",
        value.get_type().name().unwrap()
    );
    assert_eq!(value.extract::<bool>().unwrap(), expected);
}

pub(crate) fn assert_py_float(value: &Bound<'_, PyAny>, expected: f64) {
    assert!(
        value.is_instance_of::<PyFloat>(),
        "expected Python float, got {}",
        value.get_type().name().unwrap()
    );
    assert_eq!(value.extract::<f64>().unwrap(), expected);
}

pub(crate) fn assert_py_str(value: &Bound<'_, PyAny>, expected: &str) {
    assert!(
        value.is_instance_of::<PyString>(),
        "expected Python str, got {}",
        value.get_type().name().unwrap()
    );
    assert_eq!(value.extract::<String>().unwrap(), expected);
}

pub(crate) fn assert_py_bytes(value: &Bound<'_, PyAny>, expected: &[u8]) {
    assert!(
        value.is_instance_of::<PyByteArray>(),
        "expected Python bytearray, got {}",
        value.get_type().name().unwrap()
    );
    assert_eq!(value.extract::<Vec<u8>>().unwrap(), expected);
}

pub(crate) fn assert_py_none(value: &Bound<'_, PyAny>) {
    assert!(value.is_none(), "expected None, got {value}");
}
