mod column;
mod column_reader;
mod column_reader_util;
mod decfloat;
mod js_cell;
mod result;
mod stream_state;
mod time_format;

pub use column::Column;

use crate::DRIVER;
use crate::error::{ToJsError, async_to_js};
use crate::session_params::SessionParams;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use result::{ResultData, StatementResult};
use sf_core::apis::database_driver_v1::{ApiError, ExecuteQueryResult};
use sf_core::apis::operation_ctx::OperationCtx;
use sf_core::handle_manager::Handle;
use snafu::location;
use std::future::Future;
use std::sync::Arc;
use stream_state::StreamState;

#[napi]
pub struct Statement {
    result: StatementResult,
    ctx: Option<Arc<OperationCtx>>,
}

enum FetchBatchError {
    Api(Arc<ApiError>),
    Plain(String),
}

impl ToJsError for FetchBatchError {
    fn to_js_error(&self, env: Env) -> napi::Error {
        match self {
            FetchBatchError::Api(e) => e.to_js_error(env),
            FetchBatchError::Plain(message) => napi::Error::from_reason(message.clone()),
        }
    }
}

#[napi]
impl Statement {
    pub(crate) fn from_pending(
        conn_handle: Handle,
        ctx: Option<Arc<OperationCtx>>,
        result_future: impl Future<Output = std::result::Result<ExecuteQueryResult, ApiError>>
        + Send
        + 'static,
    ) -> Self {
        Self {
            result: StatementResult::from_future(async move {
                result_data_from(result_future.await?, conn_handle).await
            }),
            ctx,
        }
    }

    #[napi]
    pub fn wait_for_completion(&self, env: &Env) -> Result<AsyncBlock<()>> {
        let result = self.result.clone();
        async_to_js(env, async move { result.ready().await.map(|_| ()) })
    }

    /// Loads the next batch of rows. Returns `false` when the result set is
    /// exhausted. Drain the loaded batch with
    /// [`get_next_row`](Self::get_next_row) before calling this again.
    #[napi]
    pub fn fetch_next_batch(&self, env: &Env) -> Result<AsyncBlock<bool>> {
        let result = self.result.clone();
        async_to_js(env, async move {
            let data = result.ready().await.map_err(FetchBatchError::Api)?;
            let stream_state = Arc::clone(&data.stream_state);
            let session_params = Arc::clone(&data.session_params);
            // `fetch_next_batch` may block on a chunk download; run it on the
            // blocking pool so it doesn't tie up napi's async runtime worker
            // threads.
            spawn_blocking(move || stream_state.fetch_next_batch(&session_params))
                .await
                .map_err(|e| FetchBatchError::Plain(e.to_string()))?
                .map_err(|e| FetchBatchError::Plain(e.to_string()))
        })
    }

    /// Returns the next row of the current batch, or `null` once that batch
    /// is drained. Call [`fetch_next_batch`](Self::fetch_next_batch) to load
    /// another.
    #[napi]
    pub fn get_next_row<'env>(&self, env: &'env Env) -> Result<Option<Array<'env>>> {
        match self.result.get() {
            None => Ok(None),
            Some(Ok(data)) => data.stream_state.next_row(env),
            Some(Err(error)) => Err(error.to_js_error(*env)),
        }
    }

    // TODO:
    // - reusable error handling
    // - maybe an util to get field value so we don't repeat the match
    #[napi]
    pub fn get_query_id(&self, env: &Env) -> Result<Option<String>> {
        match self.result.get() {
            None => Ok(None),
            Some(Ok(data)) => Ok(Some(data.result_set_descriptor.query_id.clone())),
            Some(Err(error)) => Err(error.to_js_error(*env)),
        }
    }

    #[napi]
    pub fn get_num_rows(&self, env: &Env) -> Result<Option<i64>> {
        match self.result.get() {
            None => Ok(None),
            Some(Ok(data)) => Ok(data.result_set_descriptor.row_count),
            Some(Err(error)) => Err(error.to_js_error(*env)),
        }
    }

    #[napi]
    pub fn get_columns(&self, env: &Env) -> Result<Option<Vec<Column>>> {
        match self.result.get() {
            None => Ok(None),
            Some(Ok(data)) => Ok(Some(
                data.result_set_descriptor
                    .columns
                    .iter()
                    .enumerate()
                    .map(|(i, meta)| Column::from_metadata(i as u32, meta))
                    .collect(),
            )),
            Some(Err(error)) => Err(error.to_js_error(*env)),
        }
    }

    #[napi]
    pub fn get_column(&self, env: &Env, identifier: Either<String, u32>) -> Result<Option<Column>> {
        match self.result.get() {
            None => Ok(None),
            Some(Ok(data)) => {
                let columns = &data.result_set_descriptor.columns;
                let column = match identifier {
                    Either::A(name) => columns
                        .iter()
                        .enumerate()
                        .find(|(_, meta)| meta.name == name)
                        .map(|(i, meta)| Column::from_metadata(i as u32, meta)),
                    Either::B(index) => columns
                        .get(index as usize)
                        .map(|meta| Column::from_metadata(index, meta)),
                };
                Ok(column)
            }
            Some(Err(error)) => Err(error.to_js_error(*env)),
        }
    }

    // TODO: instead of Node calling close, maybe we should call it when the result set is
    // processed from the bridge
    #[napi]
    pub fn close(&self) -> Result<()> {
        if let Some(Ok(data)) = self.result.get() {
            let _ = DRIVER.result_set_release(data.result_set_handle);
        }
        Ok(())
    }

    // TODO: surface genuine cancel failures (e.g. CancelTimeout) instead of
    // swallowing the outcome.
    #[napi]
    pub async fn cancel(&self) -> Result<()> {
        if let Some(ctx) = &self.ctx {
            ctx.cancel();
        }
        Ok(())
    }
}

async fn result_data_from(
    result: ExecuteQueryResult,
    conn_handle: Handle,
) -> std::result::Result<ResultData, ApiError> {
    let (result_set_handle, result_set_descriptor) = match result {
        ExecuteQueryResult::Single { info, .. } => (info.handle, info.descriptor),
        ExecuteQueryResult::Multi { .. } => {
            return Err(ApiError::InvalidArgument {
                argument: "multi-statement results are not supported yet".to_string(),
                location: location!(),
            });
        }
    };

    // Snapshotted once here (rather than per-decoder-call) so every column
    // reader in this result set shares the same session-parameter snapshot.
    let session_params = Arc::new(SessionParams::from_connection(conn_handle).await?);
    let batch_reader = DRIVER.result_set_get_stream(result_set_handle).await?;

    Ok(ResultData {
        result_set_handle,
        result_set_descriptor,
        stream_state: Arc::new(StreamState::new(batch_reader)),
        session_params,
    })
}
