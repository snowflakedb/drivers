use crate::utils::sync::RwLockRecoverExt;
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
        let mut handles = self.handles.write_recover();

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
        let handles = self.handles.read_recover();

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

    /// Atomically deregisters and returns the value for `handle`. Only one caller
    /// can take a live handle; later lookups and takes fail.
    pub(crate) fn take_obj(&self, handle: Handle) -> Option<Arc<T>> {
        let span = span!(target: "handle_manager", Level::INFO, "HandleManager::take_obj", handle_id = handle.id, handle_magic = handle.magic);
        let _enter = span.enter();
        let index = handle.id as usize;
        let mut handles = self.handles.write_recover();

        if index >= handles.len() {
            tracing::error!("Handle index out of bounds, cannot take object");
            return None;
        }

        let handle_value = &mut handles[index];
        if handle_value.magic != handle.magic {
            tracing::error!("Handle magic mismatch, cannot take object");
            return None;
        }

        match handle_value.value.take() {
            Some(value) => {
                tracing::trace!(target: "handle_manager", "Handle taken successfully");
                Some(value)
            }
            None => {
                tracing::error!("Handle not found, cannot take object");
                None
            }
        }
    }

    pub fn delete_handle(&self, handle: Handle) -> bool {
        self.take_obj(handle).is_some()
    }

    /// Deregisters and returns every currently-live value matching `pred`,
    /// leaving non-matching handles untouched. Used by session reaping (e.g.
    /// `stream_transfer::reap_connection_streams`) to sweep up a connection's
    /// handles without knowing their ids up front. Matching values are
    /// swapped out via `Option::take` rather than removed from the backing
    /// `Vec`, preserving every other handle's `id` (its index).
    pub fn drain_matching<F: Fn(&T) -> bool>(&self, pred: F) -> Vec<Arc<T>> {
        let span = span!(target: "handle_manager", Level::INFO, "HandleManager::drain_matching");
        let _enter = span.enter();
        let mut handles = self.handles.write_recover();

        let mut drained = Vec::new();
        for handle_value in handles.iter_mut() {
            if handle_value.value.as_ref().is_some_and(|val| pred(val))
                && let Some(val) = handle_value.value.take()
            {
                drained.push(val);
            }
        }
        tracing::trace!(target: "handle_manager", "Drained {} handle(s)", drained.len());
        drained
    }
}
