use std::ops::Deref;
use std::sync::Arc;

use parking_lot::{ArcRwLockReadGuard, ArcRwLockWriteGuard, Mutex, RawRwLock, RwLock};

use crate::api::OdbcResult;
use crate::api::error::InvalidHandleSnafu;
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

struct AllocState {
    free_ids: Vec<usize>,
    next_id: usize,
}

pub struct HandleManager<T> {
    slots: Mutex<Vec<Arc<RwLock<Option<T>>>>>,
    alloc: Arc<Mutex<AllocState>>,
}

impl<T> HandleManager<T> {
    pub fn new() -> Self {
        Self {
            slots: Mutex::new(Vec::new()),
            alloc: Arc::new(Mutex::new(AllocState {
                free_ids: Vec::new(),
                next_id: 1,
            })),
        }
    }

    pub fn add(&self, value: T) -> OdbcResult<HandleId> {
        let mut alloc = self.alloc.lock();
        let id = alloc.free_ids.pop().unwrap_or_else(|| {
            let id = alloc.next_id;
            alloc.next_id += 1;
            id
        });
        drop(alloc);

        let mut slots = self.slots.lock();
        let idx = id - 1; // IDs are 1-based
        if idx >= slots.len() {
            slots.resize_with(idx + 1, || Arc::new(RwLock::new(None)));
        }
        slots[idx] = Arc::new(RwLock::new(Some(value)));
        Ok(HandleId { id })
    }

    pub fn get(&self, id: HandleId) -> OdbcResult<HandleGuard<T>> {
        let slot = {
            let slots = self.slots.lock();
            let idx = id.id.wrapping_sub(1);
            slots
                .get(idx)
                .cloned()
                .ok_or_else(|| InvalidHandleSnafu.build())?
        };
        let guard = slot.read_arc();
        if guard.is_none() {
            return Err(InvalidHandleSnafu.build());
        }
        Ok(HandleGuard { guard })
    }

    pub fn get_for_delete(&self, id: HandleId) -> OdbcResult<DeleteGuard<T>> {
        let slot = {
            let slots = self.slots.lock();
            let idx = id.id.wrapping_sub(1);
            slots
                .get(idx)
                .cloned()
                .ok_or_else(|| InvalidHandleSnafu.build())?
        };
        let guard = slot.write_arc();
        if guard.is_none() {
            return Err(InvalidHandleSnafu.build());
        }
        Ok(DeleteGuard {
            guard,
            id,
            alloc: Arc::clone(&self.alloc),
        })
    }
}

pub struct HandleGuard<T> {
    guard: ArcRwLockReadGuard<RawRwLock, Option<T>>,
}

impl<T> Deref for HandleGuard<T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: Read lock guarantees no concurrent write can set this to None.
        // Only get_for_delete (write lock) can set None, and our read lock prevents that.
        self.guard.as_ref().unwrap()
    }
}

pub struct DeleteGuard<T> {
    guard: ArcRwLockWriteGuard<RawRwLock, Option<T>>,
    id: HandleId,
    alloc: Arc<Mutex<AllocState>>,
}

impl<T> DeleteGuard<T> {
    pub fn value(&self) -> &T {
        self.guard.as_ref().unwrap()
    }

    pub fn delete(mut self) {
        *self.guard = None;
        self.alloc.lock().free_ids.push(self.id.id);
    }
}

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
    fn manager_add_get() {
        let mgr: HandleManager<i32> = HandleManager::new();
        let id = mgr.add(42).unwrap();
        assert_eq!(*mgr.get(id).unwrap(), 42);
    }

    #[test]
    fn manager_get_for_delete_and_delete() {
        let mgr: HandleManager<i32> = HandleManager::new();
        let id = mgr.add(42).unwrap();
        let guard = mgr.get_for_delete(id).unwrap();
        assert_eq!(*guard.value(), 42);
        guard.delete();
        assert!(mgr.get(id).is_err());
    }

    #[test]
    fn manager_delete_guard_drop_restores() {
        let mgr: HandleManager<i32> = HandleManager::new();
        let id = mgr.add(42).unwrap();
        {
            let _guard = mgr.get_for_delete(id).unwrap();
            // drop without calling delete()
        }
        assert_eq!(*mgr.get(id).unwrap(), 42);
    }

    #[test]
    fn manager_id_recycling() {
        let mgr: HandleManager<i32> = HandleManager::new();
        let id1 = mgr.add(1).unwrap();
        let id2 = mgr.add(2).unwrap();
        mgr.get_for_delete(id1).unwrap().delete();
        // Next add should reuse id1's slot
        let id3 = mgr.add(3).unwrap();
        assert_eq!(id3, id1);
        assert_eq!(*mgr.get(id3).unwrap(), 3);
        assert_eq!(*mgr.get(id2).unwrap(), 2);
    }

    #[test]
    fn manager_ids_start_at_one() {
        let mgr: HandleManager<i32> = HandleManager::new();
        let id = mgr.add(1).unwrap();
        assert_eq!(id, HandleId { id: 1 });
    }
}
