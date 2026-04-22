use std::collections::HashMap;
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Weak};

use parking_lot::{ArcMutexGuard, Mutex, RawMutex, RwLock};

use crate::api::error::InvalidHandleSnafu;
use crate::api::{Dbc, Env, OdbcResult};
use odbc_sys as sql;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandleId {
    id: usize,
}

impl From<HandleId> for sql::Handle {
    fn from(handle_id: HandleId) -> Self {
        handle_id.id as sql::Handle
    }
}

impl From<sql::Handle> for HandleId {
    fn from(handle: sql::Handle) -> Self {
        Self {
            id: handle as usize,
        }
    }
}

pub struct RemovalGuard<T: Send + Sync + Clone> {
    value: Option<T>,
    guard: ArcMutexGuard<RawMutex, Option<T>>,
}

impl<T: Send + Sync + Clone> RemovalGuard<T> {
    pub fn complete(mut self) -> T {
        self.value
            .take()
            .expect("complete called on already-completed RemovalGuard")
    }
}

impl<T: Send + Sync + Clone> Drop for RemovalGuard<T> {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            *self.guard = Some(value);
        }
    }
}

impl<T: Send + Sync + Clone> Deref for RemovalGuard<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.value
            .as_ref()
            .expect("deref on completed RemovalGuard")
    }
}

// TODO Reclaim ids when handles are removed
pub struct HandleRegistry<T: Send + Sync + Clone> {
    handles: RwLock<HashMap<HandleId, Arc<Mutex<Option<T>>>>>,
    next_id: AtomicUsize,
}

impl<T: Send + Sync + Clone> HandleRegistry<T> {
    fn next_id(&self) -> HandleId {
        HandleId {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub fn new() -> Self {
        Self {
            handles: RwLock::new(HashMap::new()),
            next_id: AtomicUsize::new(1),
        }
    }

    pub fn add(&self, handle: T) -> OdbcResult<HandleId> {
        let mut handles = self.handles.write();
        let id = self.next_id();
        handles.insert(id, Arc::new(Mutex::new(Some(handle))));
        Ok(id)
    }

    pub fn get(&self, id: HandleId) -> OdbcResult<T> {
        let mutex = self
            .handles
            .read()
            .get(&id)
            .cloned()
            .ok_or_else(|| InvalidHandleSnafu.build())?;
        let value = mutex.lock();
        value
            .as_ref()
            .cloned()
            .ok_or_else(|| InvalidHandleSnafu.build())
    }

    pub fn remove(&self, id: HandleId) -> OdbcResult<RemovalGuard<T>> {
        let handle = self
            .handles
            .read()
            .get(&id)
            .cloned()
            .ok_or_else(|| InvalidHandleSnafu.build())?;
        let mut guard = handle.lock_arc();
        let value_opt = guard.take();
        if value_opt.is_some() {
            return Ok(RemovalGuard {
                value: value_opt,
                guard,
            });
        }
        InvalidHandleSnafu.fail()
    }

    #[allow(dead_code)]
    pub fn cleanup(&self, ids: &[HandleId]) -> OdbcResult<()> {
        let mut handles = self.handles.write();
        for id in ids {
            handles.remove(id);
        }
        Ok(())
    }
}

pub type EnvironmentHandleRegistry = HandleRegistry<Arc<Env>>;
pub type ConnectionHandleRegistry = HandleRegistry<Weak<Dbc>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_id_has_pointer_size() {
        assert_eq!(
            std::mem::size_of::<HandleId>(),
            std::mem::size_of::<sql::Handle>()
        );
    }

    #[test]
    fn registry_add_get_remove() {
        let registry: HandleRegistry<i32> = HandleRegistry::new();
        let id = registry.add(42).unwrap();
        assert_eq!(registry.get(id).unwrap(), 42);
        assert_eq!(registry.remove(id).unwrap().complete(), 42);
        assert!(registry.get(id).is_err());
    }

    #[test]
    fn registry_cleanup() {
        let registry: HandleRegistry<i32> = HandleRegistry::new();
        let id1 = registry.add(1).unwrap();
        let id2 = registry.add(2).unwrap();
        let id3 = registry.add(3).unwrap();
        registry.cleanup(&[id1, id3]).unwrap();
        assert!(registry.get(id1).is_err());
        assert_eq!(registry.get(id2).unwrap(), 2);
        assert!(registry.get(id3).is_err());
    }

    #[test]
    fn registry_ids_start_at_one() {
        let registry: HandleRegistry<i32> = HandleRegistry::new();
        let id = registry.add(1).unwrap();
        assert_eq!(id, HandleId { id: 1 });
    }
}
