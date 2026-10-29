use std::collections::HashMap;
use std::sync::Mutex;

use sf_core::utils::sync::MutexRecoverExt;

const MAX_ALLOCATED_BYTES: u64 = 4 * 1024 * 1024 * 1024;

struct Registry {
    next_handle: u64,
    allocated_bytes: u64,
    buffers: HashMap<u64, NativeBuffer>,
}

pub(crate) struct NativeBuffer {
    bytes: Box<[u8]>,
}

impl NativeBuffer {
    fn address(&self) -> usize {
        self.bytes.as_ptr() as usize
    }

    fn len(&self) -> u64 {
        self.bytes.len() as u64
    }
}

pub(crate) struct NativeBufferRegistry {
    inner: Mutex<Registry>,
    max_allocated_bytes: u64,
}

impl NativeBufferRegistry {
    pub(crate) fn new() -> Self {
        Self::with_limit(MAX_ALLOCATED_BYTES)
    }

    fn with_limit(max_allocated_bytes: u64) -> Self {
        Self {
            inner: Mutex::new(Registry {
                next_handle: 1,
                allocated_bytes: 0,
                buffers: HashMap::new(),
            }),
            max_allocated_bytes,
        }
    }

    pub(crate) fn allocate(&self, bytes: Vec<u8>) -> Result<(u64, usize), String> {
        let buffer = NativeBuffer {
            bytes: bytes.into_boxed_slice(),
        };
        let length = buffer.len();
        let address = buffer.address();
        let mut registry = self.inner.lock_recover();
        let Some(total) = registry
            .allocated_bytes
            .checked_add(length)
            .filter(|total| *total <= self.max_allocated_bytes)
        else {
            let allocated = registry.allocated_bytes;
            let limit = self.max_allocated_bytes;
            return Err(format!(
                "Native binding buffer limit exceeded: allocated={allocated}, requested={length}, limit={limit}"
            ));
        };
        let handle = registry.next_handle;
        registry.next_handle = registry.next_handle.wrapping_add(1);
        registry.allocated_bytes = total;
        registry.buffers.insert(handle, buffer);
        Ok((handle, address))
    }

    pub(crate) fn release(&self, handle: u64) {
        let mut registry = self.inner.lock_recover();
        if let Some(buffer) = registry.buffers.remove(&handle) {
            registry.allocated_bytes -= buffer.len();
        }
    }

    pub(crate) fn allocated_bytes(&self) -> u64 {
        self.inner.lock_recover().allocated_bytes
    }

    #[cfg(test)]
    fn address_of(&self, handle: u64) -> Option<usize> {
        self.inner
            .lock_recover()
            .buffers
            .get(&handle)
            .map(|buffer| buffer.address())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_address_stays_valid_until_release() {
        let registry = NativeBufferRegistry::new();
        let (handle, address) = registry.allocate(vec![1, 2, 3]).expect("buffer allocates");
        assert_ne!(address, 0);
        assert_eq!(registry.address_of(handle), Some(address));
        assert_eq!(registry.allocated_bytes(), 3);
        registry.release(handle);
        assert_eq!(registry.address_of(handle), None);
        assert_eq!(registry.allocated_bytes(), 0);
    }

    #[test]
    fn allocate_rejects_when_the_byte_limit_would_be_exceeded() {
        let registry = NativeBufferRegistry::with_limit(4);
        let (handle, _) = registry.allocate(vec![1, 2, 3]).expect("buffer allocates");
        let error = registry
            .allocate(vec![4, 5])
            .expect_err("allocation over the limit");
        assert_eq!(
            error,
            "Native binding buffer limit exceeded: allocated=3, requested=2, limit=4"
        );
        assert_eq!(registry.allocated_bytes(), 3);
        registry.release(handle);
        assert_eq!(registry.allocated_bytes(), 0);
        registry
            .allocate(vec![4, 5])
            .expect("allocation fits after release");
    }

    #[test]
    fn allocate_accepts_exactly_the_byte_limit() {
        let registry = NativeBufferRegistry::with_limit(4);
        registry
            .allocate(vec![1, 2, 3, 4])
            .expect("allocation equal to the limit");
        assert_eq!(registry.allocated_bytes(), 4);
        let error = registry
            .allocate(vec![5])
            .expect_err("one byte over the limit");
        assert_eq!(
            error,
            "Native binding buffer limit exceeded: allocated=4, requested=1, limit=4"
        );
        assert_eq!(registry.allocated_bytes(), 4);
    }

    #[test]
    fn zero_length_buffers_get_distinct_handles_and_count_no_bytes() {
        // Empty Box<[u8]> uses the dangling alignment pointer (address 1 for u8), so
        // every zero-length allocation shares that address. The handle is the identity.
        let registry = NativeBufferRegistry::with_limit(0);
        let (first, first_address) = registry.allocate(Vec::new()).expect("empty buffer");
        let (second, second_address) = registry.allocate(Vec::new()).expect("empty buffer");
        assert_ne!(first, 0);
        assert_ne!(first, second);
        assert_ne!(first_address, 0);
        assert_ne!(second_address, 0);
        assert_eq!(registry.allocated_bytes(), 0);
        registry.release(first);
        registry.release(second);
        assert_eq!(registry.allocated_bytes(), 0);
    }

    #[test]
    fn release_ignores_unknown_and_already_released_handles() {
        let registry = NativeBufferRegistry::new();
        let (handle, _) = registry.allocate(vec![1, 2]).expect("buffer allocates");
        let (other, other_address) = registry.allocate(vec![3]).expect("buffer allocates");
        registry.release(0);
        registry.release(u64::MAX);
        registry.release(handle);
        registry.release(handle);
        assert_eq!(registry.allocated_bytes(), 1);
        assert_eq!(registry.address_of(other), Some(other_address));
    }
}
