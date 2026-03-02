use std::sync::{Arc, RwLock};
use tracing::{Level, span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Handle {
    pub id: u64,
    pub magic: u64,
}

struct HandleValue<T> {
    magic: u64,
    value: Option<Arc<T>>,
}

pub struct HandleManager<T> {
    handles: RwLock<Vec<HandleValue<T>>>,
    // TODO Add id recycling (ids are never reused, so we can run out of ids)
}

impl<T> Default for HandleManager<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> HandleManager<T> {
    pub const fn new() -> Self {
        HandleManager {
            handles: RwLock::new(Vec::new()),
        }
    }

    pub fn add_handle(&self, obj: T) -> Handle {
        let span = span!(target: "handle_manager", Level::INFO, "HandleManager::add_handle");
        let _enter = span.enter();
        let mut handles = self.handles.write().unwrap();

        let size = handles.len();
        let handle = Handle {
            id: size as u64,
            magic: rand::random::<u64>(),
        };
        let handle_value = HandleValue {
            magic: handle.magic,
            value: Some(Arc::new(obj)),
        };
        handles.push(handle_value);
        tracing::trace!(target: "handle_manager", "Handle {:?} added successfully", handle);
        handle
    }

    pub fn get_obj(&self, handle: Handle) -> Option<Arc<T>> {
        let span = span!(target: "handle_manager", Level::INFO, "HandleManager::get_obj", handle_id = handle.id, handle_magic = handle.magic);
        let _enter = span.enter();

        let index = handle.id as usize;
        let handles = self.handles.read().unwrap();

        if index >= handles.len() {
            tracing::error!("Handle index out of bounds, cannot get object");
            return None;
        }

        let handle_value = &handles[index];
        let magic = handle_value.magic;
        match handle_value.value.as_ref() {
            Some(val) if magic == handle.magic => {
                tracing::trace!(target: "handle_manager", "Handle retrieved successfully");
                Some(val.clone())
            }
            Some(_) => {
                tracing::error!("Handle magic mismatch, cannot get object");
                None
            }
            None => {
                tracing::error!("Handle not found, cannot get object");
                None
            }
        }
    }

    pub fn delete_handle(&self, handle: Handle) -> bool {
        let span = span!(target: "handle_manager", Level::INFO, "Deleting handle", handle_id = handle.id, handle_magic = handle.magic);
        let _enter = span.enter();
        let index = handle.id as usize;
        let mut handles = self.handles.write().unwrap();

        if index >= handles.len() {
            tracing::error!("Handle index out of bounds, cannot delete handle");
            return false;
        }

        let handle_value = &mut handles[index];
        let magic = handle_value.magic;

        if magic != handle.magic {
            tracing::error!("Handle magic mismatch, cannot delete handle");
            return false;
        }

        match handle_value.value.take() {
            Some(_) => {
                tracing::trace!(target: "handle_manager", "Handle deleted successfully");
                true
            }
            None => {
                tracing::error!("Handle not found, cannot delete handle");
                false
            }
        }
    }

    /// Find a handle for a given object by pointer equality.
    ///
    /// This searches through all handles and returns the handle that points
    /// to the same Arc<T> as the provided object (using Arc::ptr_eq).
    ///
    /// # Returns
    ///
    /// - `Some(Handle)` if a matching handle is found
    /// - `None` if no handle points to the same object
    pub fn find_handle_for_obj(&self, obj: &Arc<T>) -> Option<Handle> {
        let handles = self.handles.read().unwrap();

        for (index, handle_value) in handles.iter().enumerate() {
            if let Some(value) = &handle_value.value
                && Arc::ptr_eq(value, obj)
            {
                return Some(Handle {
                    id: index as u64,
                    magic: handle_value.magic,
                });
            }
        }

        None
    }
}
