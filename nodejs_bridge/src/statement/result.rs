use super::stream_state::StreamState;
use crate::session_params::SessionParams;
use napi::bindgen_prelude::spawn;
use napi::tokio::sync::{Notify, OnceCell};
use sf_core::apis::database_driver_v1::{ApiError, ResultSetDescriptor};
use sf_core::handle_manager::Handle;
use std::future::Future;
use std::sync::{Arc, Mutex};

pub(super) struct ResultData {
    pub(super) result_set_handle: Handle,
    pub(super) result_set_descriptor: ResultSetDescriptor,
    /// The single owner of this result set's session-parameter snapshot --
    /// `StreamState` doesn't hold its own copy; `get_next_row` clones this
    /// `Arc` per call and passes it into `next_row` as an argument instead.
    /// Held here, not on `StreamState`, so anything reachable from
    /// `Statement`'s synchronous getters (which read straight from
    /// `ResultData`, no lock) can see it without reaching into
    /// `stream_state`'s mutex from outside the `spawn_blocking` decode path.
    pub(super) session_params: Arc<SessionParams>,
    pub(super) stream_state: Arc<Mutex<StreamState>>,
}

/// A write-once cell holding the execution outcome, readable synchronously once
/// filled. `.get()` is a non-blocking sync read: `None` until resolved, then
/// `Some(Ok)` on success or `Some(Err)` on failure.
///
/// The outcome is stored in a [`OnceCell`] and set by the background task
/// spawned in [`Self::from_future`]. A [`Notify`] wakes any tasks parked in
/// [`Self::ready`] the moment the cell is filled.
pub(super) struct StatementResult {
    cell: Arc<OnceCell<Result<ResultData, ApiError>>>,
    ready: Arc<Notify>,
}

impl StatementResult {
    pub(super) fn from_future(
        future: impl Future<Output = Result<ResultData, ApiError>> + Send + 'static,
    ) -> Self {
        let cell = Arc::new(OnceCell::new());
        let ready = Arc::new(Notify::new());

        spawn({
            let cell = Arc::clone(&cell);
            let ready = Arc::clone(&ready);
            async move {
                let _ = cell.set(future.await);
                ready.notify_waiters();
            }
        });

        Self { cell, ready }
    }

    pub(super) async fn ready(&self) -> Result<&ResultData, &ApiError> {
        loop {
            let notified = self.ready.notified();
            if let Some(result) = self.cell.get() {
                return result.as_ref();
            }
            notified.await;
        }
    }

    pub(super) fn get(&self) -> Option<Result<&ResultData, &ApiError>> {
        self.cell.get().map(|result| result.as_ref())
    }
}
