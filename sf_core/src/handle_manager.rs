use crate::utils::sync::RwLockRecoverExt;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{Level, span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Handle {
    pub id: u64,
    pub magic: u64,
}

struct HandleValue<T> {
    magic: u64,
    value: Arc<T>,
}

struct Handles<T> {
    next_id: u64,
    live: HashMap<u64, HandleValue<T>>,
}

/// ids are never reused
pub struct HandleManager<T> {
    handles: RwLock<Handles<T>>,
}

impl<T> Default for HandleManager<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> HandleManager<T> {
    pub fn new() -> Self {
        HandleManager {
            handles: RwLock::new(Handles {
                next_id: 0,
                live: HashMap::new(),
            }),
        }
    }

    pub fn add_handle(&self, obj: T) -> Handle {
        let span = span!(target: "handle_manager", Level::INFO, "HandleManager::add_handle");
        let _enter = span.enter();
        let mut handles = self.handles.write_recover();

        let handle = Handle {
            id: handles.next_id,
            magic: rand::random::<u64>(),
        };
        handles.next_id += 1;
        handles.live.insert(
            handle.id,
            HandleValue {
                magic: handle.magic,
                value: Arc::new(obj),
            },
        );
        tracing::trace!(target: "handle_manager", "Handle {:?} added successfully", handle);
        handle
    }

    pub fn get_obj(&self, handle: Handle) -> Option<Arc<T>> {
        let span = span!(target: "handle_manager", Level::INFO, "HandleManager::get_obj", handle_id = handle.id, handle_magic = handle.magic);
        let _enter = span.enter();
        let handles = self.handles.read_recover();

        match handles.live.get(&handle.id) {
            Some(entry) if entry.magic == handle.magic => {
                tracing::trace!(target: "handle_manager", "Handle retrieved successfully");
                Some(entry.value.clone())
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
        let mut handles = self.handles.write_recover();

        match handles.live.get(&handle.id) {
            Some(entry) if entry.magic == handle.magic => {
                handles.live.remove(&handle.id);
                tracing::trace!(target: "handle_manager", "Handle deleted successfully");
                true
            }
            Some(_) => {
                tracing::error!("Handle magic mismatch, cannot delete handle");
                false
            }
            None => {
                tracing::error!("Handle not found, cannot delete handle");
                false
            }
        }
    }

    /// Deregisters and returns every currently-live value matching `pred`,
    /// leaving non-matching handles untouched. Used by session reaping (e.g.
    /// `stream_transfer::reap_connection_streams`) to sweep up a connection's
    /// handles without knowing their ids up front.
    pub fn drain_matching<F: Fn(&T) -> bool>(&self, pred: F) -> Vec<Arc<T>> {
        let span = span!(target: "handle_manager", Level::INFO, "HandleManager::drain_matching");
        let _enter = span.enter();
        let mut handles = self.handles.write_recover();

        let mut drained = Vec::new();
        handles.live.retain(|_, entry| {
            if pred(&entry.value) {
                drained.push(entry.value.clone());
                false
            } else {
                true
            }
        });
        tracing::trace!(target: "handle_manager", "Drained {} handle(s)", drained.len());
        drained
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn live_count<T>(manager: &HandleManager<T>) -> usize {
        manager.handles.read_recover().live.len()
    }

    fn with_magic(handle: Handle, magic: u64) -> Handle {
        Handle { magic, ..handle }
    }

    #[test]
    fn add_then_get_returns_value() {
        let manager = HandleManager::new();
        let handle = manager.add_handle("a");

        assert_eq!(manager.get_obj(handle).as_deref(), Some(&"a"));
    }

    #[test]
    fn add_assigns_sequential_ids() {
        let manager = HandleManager::new();

        let ids: Vec<u64> = (0..3).map(|i| manager.add_handle(i).id).collect();

        assert_eq!(ids, [0, 1, 2]);
    }

    #[test]
    fn get_unknown_id_returns_none() {
        let manager = HandleManager::<i32>::new();

        assert!(manager.get_obj(Handle { id: 0, magic: 0 }).is_none());
    }

    #[test]
    fn get_with_wrong_magic_returns_none() {
        let manager = HandleManager::new();
        let handle = manager.add_handle(1);

        assert!(
            manager
                .get_obj(with_magic(handle, handle.magic.wrapping_add(1)))
                .is_none()
        );
        assert!(manager.get_obj(handle).is_some());
    }

    #[test]
    fn delete_removes_entry() {
        let manager = HandleManager::new();
        let deleted = manager.add_handle(1);
        let kept = manager.add_handle(2);

        assert!(manager.delete_handle(deleted));

        assert!(manager.get_obj(deleted).is_none());
        assert_eq!(manager.get_obj(kept).as_deref(), Some(&2));
        assert_eq!(live_count(&manager), 1);
    }

    #[test]
    fn delete_twice_fails_second_time() {
        let manager = HandleManager::new();
        let handle = manager.add_handle(1);

        assert!(manager.delete_handle(handle));
        assert!(!manager.delete_handle(handle));
    }

    #[test]
    fn delete_unknown_id_fails() {
        let manager = HandleManager::<i32>::new();

        assert!(!manager.delete_handle(Handle { id: 42, magic: 0 }));
    }

    #[test]
    fn delete_with_wrong_magic_keeps_entry() {
        let manager = HandleManager::new();
        let handle = manager.add_handle(1);

        assert!(!manager.delete_handle(with_magic(handle, handle.magic.wrapping_add(1))));

        assert_eq!(manager.get_obj(handle).as_deref(), Some(&1));
        assert_eq!(live_count(&manager), 1);
    }

    #[test]
    fn ids_are_not_reused_after_delete() {
        let manager = HandleManager::new();
        let first = manager.add_handle(1);
        assert!(manager.delete_handle(first));

        let second = manager.add_handle(2);

        assert_eq!(second.id, first.id + 1);
        assert!(manager.get_obj(first).is_none());
    }

    #[test]
    fn deleted_value_outlives_handle_while_referenced() {
        let manager = HandleManager::new();
        let handle = manager.add_handle(String::from("value"));
        let held = manager.get_obj(handle);

        assert!(manager.delete_handle(handle));

        assert_eq!(held.as_deref().map(String::as_str), Some("value"));
    }

    #[test]
    fn add_delete_cycles_do_not_grow_storage() {
        let manager = HandleManager::new();

        for i in 0..1000 {
            let handle = manager.add_handle(i);
            assert!(manager.delete_handle(handle));
        }

        assert_eq!(live_count(&manager), 0);
    }

    #[test]
    fn drain_matching_removes_only_matches() {
        let manager = HandleManager::new();
        let handles: Vec<Handle> = (0..4).map(|i| manager.add_handle(i)).collect();

        let mut drained: Vec<i32> = manager
            .drain_matching(|v| v % 2 == 0)
            .into_iter()
            .map(|v| *v)
            .collect();
        drained.sort_unstable();

        assert_eq!(drained, [0, 2]);
        assert!(manager.get_obj(handles[0]).is_none());
        assert_eq!(manager.get_obj(handles[1]).as_deref(), Some(&1));
        assert!(manager.get_obj(handles[2]).is_none());
        assert_eq!(manager.get_obj(handles[3]).as_deref(), Some(&3));
        assert!(!manager.delete_handle(handles[0]));
    }

    #[test]
    fn drain_matching_without_matches_returns_empty() {
        let manager = HandleManager::new();
        manager.add_handle(1);

        assert!(manager.drain_matching(|_| false).is_empty());
        assert_eq!(live_count(&manager), 1);
    }

    #[test]
    fn drain_matching_on_empty_manager_returns_empty() {
        let manager = HandleManager::<i32>::new();

        assert!(manager.drain_matching(|_| true).is_empty());
    }

    #[test]
    fn drain_matching_skips_deleted_handles() {
        let manager = HandleManager::new();
        let deleted = manager.add_handle(1);
        manager.add_handle(2);
        assert!(manager.delete_handle(deleted));

        let drained: Vec<i32> = manager
            .drain_matching(|_| true)
            .into_iter()
            .map(|v| *v)
            .collect();

        assert_eq!(drained, [2]);
        assert_eq!(live_count(&manager), 0);
    }

    #[test]
    fn concurrent_add_and_delete_leave_no_entries() {
        let manager = Arc::new(HandleManager::new());

        let workers: Vec<_> = (0..8)
            .map(|t| {
                let manager = Arc::clone(&manager);
                std::thread::spawn(move || {
                    (0..100)
                        .map(|i| {
                            let handle = manager.add_handle(t * 100 + i);
                            manager.delete_handle(handle);
                            handle.id
                        })
                        .collect::<Vec<u64>>()
                })
            })
            .collect();
        let mut ids: Vec<u64> = workers
            .into_iter()
            .flat_map(|w| w.join().expect("worker thread panicked"))
            .collect();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), 800);
        assert_eq!(live_count(&manager), 0);
    }
}
