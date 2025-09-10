use crate::c_api::CApiHandle;

pub struct HandleTransport {
    handle: CApiHandle,
}

impl HandleTransport {
    pub fn new(handle: CApiHandle) -> Self {
        HandleTransport { handle }
    }
}

impl std::io::Read for HandleTransport {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(unsafe { crate::c_api::sf_core_api_read(self.handle, buf.as_mut_ptr(), buf.len()) })
    }
}

impl std::io::Write for HandleTransport {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let size = unsafe {
            crate::c_api::sf_core_api_write(self.handle, buf.as_ptr() as *mut u8, buf.len())
        };
        Ok(size)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        crate::c_api::sf_core_api_flush(self.handle);
        Ok(())
    }
}
