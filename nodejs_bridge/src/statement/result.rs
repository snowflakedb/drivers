use crate::BRIDGE;
use crate::error::{BridgeError, ToJsError};
use crate::rows::StreamState;
use crate::session::Session;
use crate::session_params::KnownSessionParameters;
use napi::bindgen_prelude::spawn;
use napi::tokio::sync::{Mutex, MutexGuard, Notify, OnceCell};
use sf_core::apis::database_driver_v1::{ExecuteQueryResult, ResultSetDescriptor, ResultSetInfo};
use sf_core::handle_manager::Handle;
use std::future::Future;
use std::sync::Arc;

/// A write-once cell holding the result value, readable synchronously once
/// filled. `.get()` is a non-blocking sync read: `None` until resolved, then
/// `Some(Ok)` on success or `Some(Err)` on failure.
///
/// The value is stored in a [`OnceCell`] and set by the background task
/// spawned in [`Self::from_future`]. A [`Notify`] wakes any tasks parked in
/// [`Self::ready`] the moment the cell is filled.
#[derive(Clone)]
pub(super) struct StatementResult {
    cell: Arc<OnceCell<Result<ResultValue, BridgeError>>>,
    ready: Arc<Notify>,
}

impl StatementResult {
    pub(super) fn from_future(
        session: Session,
        future: impl Future<Output = Result<ResultValue, BridgeError>> + Send + 'static,
    ) -> Self {
        let cell = Arc::new(OnceCell::new());
        let ready = Arc::new(Notify::new());

        spawn({
            let cell = Arc::clone(&cell);
            let ready = Arc::clone(&ready);
            async move {
                let result = future.await;
                if let Some((exception_type, error_source)) =
                    result.as_ref().err().and_then(ToJsError::telemetry)
                {
                    session.record_wrapper_error(exception_type, error_source);
                }
                let _ = cell.set(result);
                ready.notify_waiters();
            }
        });

        Self { cell, ready }
    }

    pub(super) async fn ready(&self) -> Result<&ResultValue, BridgeError> {
        loop {
            let notified = self.ready.notified();
            if let Some(result) = self.cell.get() {
                return result.as_ref().map_err(reported);
            }
            notified.await;
        }
    }

    pub(super) fn get(&self) -> Option<Result<&ResultValue, BridgeError>> {
        self.cell
            .get()
            .map(|result| result.as_ref().map_err(reported))
    }
}

fn reported(error: &BridgeError) -> BridgeError {
    BridgeError::Reported(Arc::new(error.clone()))
}

pub(crate) enum ResultValue {
    QueryResult(Mutex<Box<QueryResult>>),
    AsyncSubmitted { query_id: String },
}

impl ResultValue {
    pub(crate) async fn from_execute_result(
        result: ExecuteQueryResult,
        conn_handle: Handle,
    ) -> Result<Self, BridgeError> {
        let query_result = QueryResult::from_execute_result(result, conn_handle).await?;
        Ok(Self::QueryResult(Mutex::new(Box::new(query_result))))
    }

    pub(crate) fn query_id(&self) -> String {
        match self {
            Self::QueryResult(query_result) => query_result
                .blocking_lock()
                .result_set_descriptor
                .query_id
                .clone(),
            Self::AsyncSubmitted { query_id } => query_id.clone(),
        }
    }

    // TODO: rewrite needed
    // blocking_lock would block JS event loop while next multi-statement result is fetching
    pub(crate) fn lock_query_result(&self) -> Option<MutexGuard<'_, Box<QueryResult>>> {
        match self {
            Self::QueryResult(query_result) => Some(query_result.blocking_lock()),
            Self::AsyncSubmitted { .. } => None,
        }
    }
}

pub(crate) struct QueryResult {
    result_set_handle: Option<Handle>,
    pub(crate) result_set_descriptor: ResultSetDescriptor,
    pub(crate) session_params: Arc<KnownSessionParameters>,
    pub(crate) stream_state: Arc<StreamState>,
    multi: Option<MultiStatementCursor>,
    conn_handle: Handle,
}

impl QueryResult {
    async fn from_execute_result(
        result: ExecuteQueryResult,
        conn_handle: Handle,
    ) -> Result<Self, BridgeError> {
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
                    Some(MultiStatementCursor {
                        query_ids,
                        index: 0,
                    }),
                )
            }
        };
        let (session_params, stream) = async {
            let session_params = KnownSessionParameters::from_connection(conn_handle).await?;
            let stream = BRIDGE
                .driver
                .result_set_get_async_stream(info.handle)
                .await?;
            Ok::<_, BridgeError>((session_params, stream))
        }
        .await
        .inspect_err(|_| {
            let _ = BRIDGE.driver.result_set_release(info.handle);
        })?;
        Ok(Self {
            session_params: Arc::new(session_params),
            stream_state: Arc::new(StreamState::new(stream, &info.descriptor.columns)),
            result_set_handle: Some(info.handle),
            result_set_descriptor: info.descriptor,
            multi,
            conn_handle,
        })
    }

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

    pub(crate) fn close(&mut self) {
        if let Some(handle) = self.result_set_handle.take() {
            let _ = BRIDGE.driver.result_set_release(handle);
        }
    }

    pub(crate) async fn next_result(&mut self) -> Result<bool, BridgeError> {
        let Some(query_id) = self.multi.as_mut().and_then(MultiStatementCursor::next) else {
            return Ok(false);
        };
        let info = result_set_info(self.conn_handle, &query_id).await?;
        self.close();
        self.result_set_handle = Some(info.handle);
        self.result_set_descriptor = info.descriptor;
        self.stream_state = Arc::new(StreamState::new(
            BRIDGE
                .driver
                .result_set_get_async_stream(info.handle)
                .await?,
            &self.result_set_descriptor.columns,
        ));
        Ok(true)
    }
}

impl Drop for QueryResult {
    fn drop(&mut self) {
        self.close();
    }
}

struct MultiStatementCursor {
    query_ids: Vec<String>,
    index: usize,
}

impl MultiStatementCursor {
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

async fn result_set_info(
    conn_handle: Handle,
    query_id: &str,
) -> Result<ResultSetInfo, BridgeError> {
    match BRIDGE
        .driver
        .connection_get_query_result(None, conn_handle, query_id.to_owned())
        .await?
    {
        ExecuteQueryResult::Single { info, .. } => Ok(info),
        ExecuteQueryResult::Multi { .. } => Err(BridgeError::Message(
            "nested multi-statement result is not supported".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Int32Array, RecordBatch};
    use arrow::datatypes::{DataType, Field, Schema};

    fn result_set() -> ResultSetInfo {
        let schema = Arc::new(Schema::new(vec![Field::new("N", DataType::Int32, false)]));
        let batch =
            RecordBatch::try_new(schema, vec![Arc::new(Int32Array::from(vec![1]))]).unwrap();
        BRIDGE
            .driver
            .register_arrow_batch_as_result_set(&batch, reqwest::Client::new())
            .unwrap()
    }

    fn single(info: ResultSetInfo) -> ExecuteQueryResult {
        ExecuteQueryResult::Single {
            info,
            request_id: None,
        }
    }

    fn is_released(handle: Handle) -> bool {
        BRIDGE.driver.result_set_release(handle).is_err()
    }

    async fn open_query_result(conn: Handle) -> (QueryResult, Handle) {
        let info = result_set();
        let handle = info.handle;
        let Ok(query_result) = QueryResult::from_execute_result(single(info), conn).await else {
            panic!("query result should open");
        };
        (query_result, handle)
    }

    #[tokio::test]
    async fn drop_releases_the_result_set() {
        let conn = BRIDGE.driver.connection_new();
        let (query_result, handle) = open_query_result(conn).await;

        drop(query_result);

        assert!(is_released(handle));
        BRIDGE.driver.connection_release(conn).unwrap();
    }

    #[tokio::test]
    async fn close_releases_the_result_set() {
        let conn = BRIDGE.driver.connection_new();
        let (mut query_result, handle) = open_query_result(conn).await;

        query_result.close();

        assert!(is_released(handle));
        BRIDGE.driver.connection_release(conn).unwrap();
    }

    #[tokio::test]
    async fn close_twice_then_drop_is_a_no_op() {
        let conn = BRIDGE.driver.connection_new();
        let (mut query_result, handle) = open_query_result(conn).await;
        query_result.close();
        let unrelated = result_set();

        query_result.close();
        drop(query_result);

        assert!(is_released(handle));
        assert!(!is_released(unrelated.handle));
        BRIDGE.driver.connection_release(conn).unwrap();
    }

    #[tokio::test]
    async fn open_failure_releases_the_result_set() {
        let conn = BRIDGE.driver.connection_new();
        BRIDGE.driver.connection_release(conn).unwrap();
        let info = result_set();
        let handle = info.handle;

        assert!(
            QueryResult::from_execute_result(single(info), conn)
                .await
                .is_err()
        );

        assert!(is_released(handle));
    }
}
