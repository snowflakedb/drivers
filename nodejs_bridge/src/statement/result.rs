use super::stream_state::StreamState;
use crate::DRIVER;
use crate::error::BridgeError;
use crate::session_params::KnownSessionParameters;
use napi::bindgen_prelude::spawn;
use napi::tokio::sync::{Mutex, MutexGuard, Notify, OnceCell};
use sf_core::apis::database_driver_v1::{ExecuteQueryResult, ResultSetDescriptor, ResultSetInfo};
use sf_core::handle_manager::Handle;
use std::future::Future;
use std::sync::Arc;

pub(crate) enum StatementOutcome {
    Rows(Mutex<Box<ResultData>>),
    AsyncSubmitted { query_id: String },
}

impl StatementOutcome {
    pub(crate) fn query_id(&self) -> String {
        match self {
            Self::Rows(mutex) => mutex.blocking_lock().result_set_descriptor.query_id.clone(),
            Self::AsyncSubmitted { query_id } => query_id.clone(),
        }
    }

    pub(crate) fn rows(&self) -> Option<MutexGuard<'_, Box<ResultData>>> {
        match self {
            Self::Rows(mutex) => Some(mutex.blocking_lock()),
            Self::AsyncSubmitted { .. } => None,
        }
    }
}

pub(crate) struct ResultData {
    pub(crate) result_set_handle: Handle,
    pub(crate) result_set_descriptor: ResultSetDescriptor,
    pub(crate) session_params: Arc<KnownSessionParameters>,
    pub(crate) stream_state: Arc<StreamState>,
    multi: Option<MultiStmtResultState>,
    conn_handle: Handle,
}

struct MultiStmtResultState {
    query_ids: Vec<String>,
    index: usize,
}

impl MultiStmtResultState {
    fn has_next(&self) -> bool {
        self.index + 1 < self.query_ids.len()
    }

    fn next(&mut self) -> Option<String> {
        if !self.has_next() {
            return None;
        }
        self.index += 1;
        Some(self.query_ids[self.index].clone())
    }
}

impl ResultData {
    pub(crate) fn has_next(&self) -> bool {
        if let Some(multi) = &self.multi {
            multi.has_next()
        } else {
            false
        }
    }

    pub(crate) fn is_multi_statement(&self) -> bool {
        self.multi.is_some()
    }

    pub(crate) fn close(&self) {
        let _ = DRIVER.result_set_release(self.result_set_handle);
    }

    pub(crate) async fn next_result(&mut self) -> Result<bool, BridgeError> {
        let Some(query_id) = self.multi.as_mut().and_then(MultiStmtResultState::next) else {
            return Ok(false);
        };
        let info = result_set_info(self.conn_handle, &query_id).await?;
        self.close();
        self.result_set_handle = info.handle;
        self.result_set_descriptor = info.descriptor;
        self.stream_state = Arc::new(StreamState::new(
            DRIVER.result_set_get_async_stream(info.handle).await?,
        ));
        Ok(true)
    }

    pub(crate) async fn from_execute_result(
        result: ExecuteQueryResult,
        conn_handle: Handle,
    ) -> Result<Self, BridgeError> {
        let session_params = Arc::new(KnownSessionParameters::from_connection(conn_handle).await?);
        let (info, multi) = match result {
            ExecuteQueryResult::Single { info, .. } => (info, None),
            ExecuteQueryResult::Multi { query_ids, .. } => {
                let first_query_id = query_ids.first().cloned().ok_or_else(|| {
                    BridgeError::Message(
                        "multi-statement result contained no child query IDs".to_string(),
                    )
                })?;
                (
                    result_set_info(conn_handle, &first_query_id).await?,
                    Some(MultiStmtResultState {
                        query_ids,
                        index: 0,
                    }),
                )
            }
        };
        Ok(Self {
            stream_state: Arc::new(StreamState::new(
                DRIVER.result_set_get_async_stream(info.handle).await?,
            )),
            result_set_handle: info.handle,
            result_set_descriptor: info.descriptor,
            session_params,
            multi,
            conn_handle,
        })
    }
}

async fn result_set_info(
    conn_handle: Handle,
    query_id: &str,
) -> Result<ResultSetInfo, BridgeError> {
    match DRIVER
        .connection_get_query_result(None, conn_handle, query_id.to_owned())
        .await?
    {
        ExecuteQueryResult::Single { info, .. } => Ok(info),
        ExecuteQueryResult::Multi { .. } => Err(BridgeError::Message(
            "nested multi-statement result is not supported".to_string(),
        )),
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
