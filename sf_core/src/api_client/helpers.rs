use arrow::{
    ffi::{FFI_ArrowArray, FFI_ArrowSchema},
    ffi_stream::FFI_ArrowArrayStream,
};

use crate::thrift_gen::database_driver_v1::{ArrowArrayPtr, ArrowArrayStreamPtr, ArrowSchemaPtr};

impl From<Box<ArrowArrayStreamPtr>> for *mut FFI_ArrowArrayStream {
    fn from(ptr: Box<ArrowArrayStreamPtr>) -> Self {
        unsafe { std::ptr::read(ptr.value.as_ptr() as *const *mut FFI_ArrowArrayStream) }
    }
}
#[allow(clippy::from_over_into)]
impl Into<*mut FFI_ArrowSchema> for ArrowSchemaPtr {
    fn into(self) -> *mut FFI_ArrowSchema {
        unsafe { std::ptr::read(self.value.as_ptr() as *const *mut FFI_ArrowSchema) }
    }
}

#[allow(clippy::from_over_into)]
impl Into<*mut FFI_ArrowArray> for ArrowArrayPtr {
    fn into(self) -> *mut FFI_ArrowArray {
        unsafe { std::ptr::read(self.value.as_ptr() as *const *mut FFI_ArrowArray) }
    }
}
