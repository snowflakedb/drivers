//! NumPy publishes C entry points as the `_ARRAY_API` capsule on `multiarray`.
//! This module imports that module only to take the capsule, then converts cells
//! with `PyArray_Scalar`. Converters never call `numpy.int64` / `numpy.float64` /
//! `numpy.datetime64` in Python. rust-numpy is not used (not official PyO3).

use std::ffi::c_void;
use std::ptr;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyCapsule, PyCapsuleMethods};

/// Slot 60 is `PyArray_Scalar`. The C-API table is append-only, so the index is
/// the same on NumPy 1.23 and 2.x.
const PYARRAY_SCALAR_SLOT: usize = 60;

type PyArrayScalar = unsafe extern "C" fn(
    data: *mut c_void,
    descr: *mut pyo3::ffi::PyObject,
    base: *mut pyo3::ffi::PyObject,
) -> *mut pyo3::ffi::PyObject;

struct NumpyApi {
    _capsule: Py<PyAny>,
    pyarray_scalar: PyArrayScalar,
    int64_dtype: Py<PyAny>,
    float64_dtype: Py<PyAny>,
    datetime64_d_dtype: Py<PyAny>,
    datetime64_ns_dtype: Py<PyAny>,
}

pub(super) struct NumpyProvider {
    api: PyOnceLock<NumpyApi>,
}

impl NumpyProvider {
    pub(super) fn new() -> Self {
        Self {
            api: PyOnceLock::new(),
        }
    }

    pub(super) fn int64<'py>(&self, py: Python<'py>, value: i64) -> PyResult<Bound<'py, PyAny>> {
        self.scalar(py, value, |api| &api.int64_dtype)
    }

    pub(super) fn float64<'py>(&self, py: Python<'py>, value: f64) -> PyResult<Bound<'py, PyAny>> {
        self.scalar(py, value, |api| &api.float64_dtype)
    }

    pub(super) fn datetime64_d<'py>(
        &self,
        py: Python<'py>,
        days: i64,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.scalar(py, days, |api| &api.datetime64_d_dtype)
    }

    pub(super) fn datetime64_ns<'py>(
        &self,
        py: Python<'py>,
        nanos: i64,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.scalar(py, nanos, |api| &api.datetime64_ns_dtype)
    }

    fn scalar<'py, T: Copy>(
        &self,
        py: Python<'py>,
        value: T,
        dtype: impl FnOnce(&NumpyApi) -> &Py<PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let api = self.api.get_or_try_init(py, || load_api(py))?;
        let descr = dtype(api).bind(py);
        let mut value = value;
        // SAFETY: descr is a borrowed numpy.dtype (a PyArray_Descr) that outlives
        // the call. data points at a stack T that outlives the call. base is null.
        unsafe {
            let ptr = (api.pyarray_scalar)(
                (&raw mut value).cast::<c_void>(),
                descr.as_ptr(),
                ptr::null_mut(),
            );
            Bound::from_owned_ptr_or_err(py, ptr)
        }
    }
}

fn load_api(py: Python<'_>) -> PyResult<NumpyApi> {
    // NumPy 2.x still ships a `numpy.core` shim, so `_core` is tried first.
    // 1.x from 1.23 publishes the capsule on `numpy.core.multiarray`.
    let multiarray = py
        .import("numpy._core.multiarray")
        .or_else(|_| py.import("numpy.core.multiarray"))?;
    let capsule_obj = multiarray.getattr("_ARRAY_API")?;
    let capsule = capsule_obj.cast::<PyCapsule>()?;
    let table = capsule.pointer_checked(None)?;
    let pyarray_scalar = {
        let table = table.as_ptr().cast::<*mut c_void>();
        // SAFETY: `_ARRAY_API` is a C function-pointer table; slot 60 is
        // `PyArray_Scalar` on every NumPy this loader supports.
        let slot = unsafe { *table.add(PYARRAY_SCALAR_SLOT) };
        if slot.is_null() {
            return Err(PyValueError::new_err(
                "numpy _ARRAY_API PyArray_Scalar slot is null",
            ));
        }
        // SAFETY: slot 60 is `PyArray_Scalar` with the C signature above.
        unsafe { std::mem::transmute::<*mut c_void, PyArrayScalar>(slot) }
    };

    let numpy = py.import("numpy")?;
    let dtype = numpy.getattr("dtype")?;
    // A numpy.dtype is a PyArray_Descr. Fields are not read (layout changed in 2.0).
    let int64_dtype = dtype.call1(("int64",))?.unbind();
    let float64_dtype = dtype.call1(("float64",))?.unbind();
    // datetime64[D] is an 8-byte npy_datetime counting days from the Unix epoch.
    let datetime64_d_dtype = dtype.call1(("datetime64[D]",))?.unbind();
    // datetime64[ns] is an 8-byte npy_datetime counting nanoseconds from the Unix epoch.
    let datetime64_ns_dtype = dtype.call1(("datetime64[ns]",))?.unbind();
    Ok(NumpyApi {
        _capsule: capsule_obj.unbind(),
        pyarray_scalar,
        int64_dtype,
        float64_dtype,
        datetime64_d_dtype,
        datetime64_ns_dtype,
    })
}

#[cfg(test)]
mod tests {
    use pyo3::prelude::*;

    use super::NumpyProvider;

    fn numpy_type<'py>(py: Python<'py>, name: &str) -> Bound<'py, PyAny> {
        py.import("numpy").unwrap().getattr(name).unwrap()
    }

    #[test]
    fn int64_float64_and_datetime64_are_numpy_scalars() {
        Python::initialize();
        let provider = NumpyProvider::new();
        Python::attach(|py| {
            let int64 = provider.int64(py, 42).unwrap();
            assert!(
                int64.get_type().is(numpy_type(py, "int64")),
                "expected numpy.int64, got {}",
                int64.get_type().name().unwrap()
            );
            assert_eq!(int64.extract::<i64>().unwrap(), 42);

            let float64 = provider.float64(py, 1.5).unwrap();
            assert!(
                float64.get_type().is(numpy_type(py, "float64")),
                "expected numpy.float64, got {}",
                float64.get_type().name().unwrap()
            );
            assert_eq!(float64.extract::<f64>().unwrap(), 1.5);

            let date = provider.datetime64_d(py, 0).unwrap();
            let datetime64 = numpy_type(py, "datetime64");
            assert!(
                date.get_type().is(&datetime64),
                "expected numpy.datetime64, got {}",
                date.get_type().name().unwrap()
            );
            let expected = datetime64.call1((0, "D")).unwrap();
            assert!(date.eq(&expected).unwrap());

            let ntz = provider.datetime64_ns(py, 0).unwrap();
            assert!(
                ntz.get_type().is(&datetime64),
                "expected numpy.datetime64, got {}",
                ntz.get_type().name().unwrap()
            );
            let expected_ns = datetime64.call1((0, "ns")).unwrap();
            assert!(ntz.eq(&expected_ns).unwrap());
        });
    }

    #[test]
    fn array_api_capsule_is_found_via_numpy2_then_numpy1_import() {
        Python::initialize();
        Python::attach(|py| {
            let multiarray = py
                .import("numpy._core.multiarray")
                .or_else(|_| py.import("numpy.core.multiarray"))
                .unwrap();
            assert!(multiarray.getattr("_ARRAY_API").is_ok());
        });
    }
}
