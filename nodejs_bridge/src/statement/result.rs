use super::stream_state::StreamState;
use crate::DRIVER;
use crate::error::BridgeError;
use crate::session_params::KnownSessionParameters;
use napi::bindgen_prelude::spawn;
use napi::tokio::sync::{Notify, OnceCell};
use sf_core::apis::database_driver_v1::{ApiError, ExecuteQueryResult, ResultSetDescriptor};
use sf_core::handle_manager::Handle;
use std::future::Future;
use std::sync::Arc;

pub(crate) enum StatementOutcome {
    Rows(Box<ResultData>),
    AsyncSubmitted { query_id: String },
}

impl StatementOutcome {
    pub(crate) fn query_id(&self) -> &str {
        match self {
            Self::Rows(data) => &data.result_set_descriptor.query_id,
            Self::AsyncSubmitted { query_id } => query_id,
        }
    }

    pub(crate) fn rows(&self) -> Option<&ResultData> {
        match self {
            Self::Rows(data) => Some(data),
            Self::AsyncSubmitted { .. } => None,
        }
    }
}

pub(crate) struct ResultData {
    pub(crate) result_set_handle: Handle,
    pub(crate) result_set_descriptor: ResultSetDescriptor,
    pub(crate) session_params: Arc<KnownSessionParameters>,
    pub(crate) stream_state: Arc<StreamState>,
}

impl ResultData {
    pub(crate) async fn from_execute_query(
        result: ExecuteQueryResult,
        conn_handle: Handle,
    ) -> Result<Self, BridgeError> {
        let (result_set_handle, result_set_descriptor) = match result {
            ExecuteQueryResult::Single { info, .. } => (info.handle, info.descriptor),
            ExecuteQueryResult::Multi { .. } => {
                return Err(ApiError::invalid_argument(
                    "multi-statement results are not supported yet",
                )
                .into());
            }
        };

        // Snapshotted once here (rather than per-decoder-call) so every column
        // reader in this result set shares the same session-parameter snapshot.
        let session_params = Arc::new(KnownSessionParameters::from_connection(conn_handle).await?);
        let fetcher = DRIVER
            .result_set_get_async_stream(result_set_handle)
            .await?;

        Ok(Self {
            result_set_handle,
            result_set_descriptor,
            stream_state: Arc::new(StreamState::new(fetcher)),
            session_params,
        })
    }
}

/// A write-once cell holding the execution outcome, readable synchronously once
/// filled. `.get()` is a non-blocking sync read: `None` until resolved, then
/// `Some(Ok)` on success or `Some(Err)` on failure.
///
/// The outcome is stored in a [`OnceCell`] and set by the background task
/// spawned in [`Self::from_future`]. A [`Notify`] wakes any tasks parked in
/// [`Self::ready`] the moment the cell is filled.
#[derive(Clone)]
pub(super) struct StatementResult {
    cell: Arc<OnceCell<Result<StatementOutcome, BridgeError>>>,
    ready: Arc<Notify>,
}

impl StatementResult {
    pub(super) fn from_future(
        future: impl Future<Output = Result<StatementOutcome, BridgeError>> + Send + 'static,
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

    pub(super) async fn ready(&self) -> Result<&StatementOutcome, BridgeError> {
        loop {
            let notified = self.ready.notified();
            if let Some(result) = self.cell.get() {
                return result.as_ref().map_err(BridgeError::clone);
            }
            notified.await;
        }
    }

    pub(super) fn get(&self) -> Option<Result<&StatementOutcome, BridgeError>> {
        self.cell
            .get()
            .map(|result| result.as_ref().map_err(BridgeError::clone))
    }
}
