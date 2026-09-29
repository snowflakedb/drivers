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
use crate::error::{BridgeError, ToJsError, async_to_js};
use crate::session::Ready;
use crate::session_params::KnownSessionParameters;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use result::{ResultData, StatementOutcome, StatementResult};
use sf_core::apis::database_driver_v1::{AsyncExecuteResult, ExecuteQueryResult};
use sf_core::apis::operation_ctx::OperationCtx;
use std::future::Future;
use std::sync::Arc;

#[napi]
pub struct Statement {
    result: StatementResult,
    operation_ctx: Option<Arc<OperationCtx>>,
}

#[napi]
impl Statement {
    pub(crate) fn from_query_result(
        operation_ctx: Option<Arc<OperationCtx>>,
        result_future: impl Future<
            Output = std::result::Result<(Ready, ExecuteQueryResult), BridgeError>,
        > + Send
        + 'static,
    ) -> Self {
        Self {
            result: StatementResult::from_future(async move {
                let (ready, result) = result_future.await?;
                Ok(StatementOutcome::Rows(Box::new(
                    ResultData::from_execute_query(result, ready.connection()).await?,
                )))
            }),
            operation_ctx,
        }
    }

    pub(crate) fn from_async_exec(
        operation_ctx: Option<Arc<OperationCtx>>,
        result_future: impl Future<Output = std::result::Result<AsyncExecuteResult, BridgeError>>
        + Send
        + 'static,
    ) -> Self {
        Self {
            result: StatementResult::from_future(async move {
                Ok(StatementOutcome::AsyncSubmitted {
                    query_id: result_future.await?.query_id,
                })
            }),
            operation_ctx,
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
            let outcome = result.ready().await?;
            let Some(data) = outcome.rows() else {
                return Ok(false);
            };
            data.stream_state
                .fetch_next_batch(&data.session_params)
                .await
        })
    }

    fn ready_rows(&self, env: &Env) -> Result<Option<&ResultData>> {
        match self.result.get() {
            None => Ok(None),
            Some(Ok(outcome)) => Ok(outcome.rows()),
            Some(Err(error)) => Err(error.to_js_error(*env)),
        }
    }

    /// Returns the next row of the current batch, or `null` once that batch
    /// is drained. Call [`fetch_next_batch`](Self::fetch_next_batch) to load
    /// another.
    #[napi]
    pub fn get_next_row<'env>(&self, env: &'env Env) -> Result<Option<Array<'env>>> {
        match self.ready_rows(env)? {
            Some(data) => data.stream_state.next_row(env),
            None => Ok(None),
        }
    }

    // TODO:
    // - reusable error handling
    // - maybe an util to get field value so we don't repeat the match
    #[napi]
    pub fn get_query_id(&self, _env: &Env) -> Result<Option<String>> {
        match self.result.get() {
            None => Ok(None),
            Some(Ok(outcome)) => Ok(Some(outcome.query_id().to_string())),
            Some(Err(BridgeError::Core(api_error))) => Ok(api_error.query_id()),
            Some(Err(_)) => Ok(None),
        }
    }

    #[napi]
    pub fn get_num_rows(&self, env: &Env) -> Result<Option<i64>> {
        Ok(self
            .ready_rows(env)?
            .and_then(|data| data.result_set_descriptor.row_count))
    }

    /// Not part of the driver's public API. Callers are suposed toinvoke this only after the
    /// statement has finished, so a result that is not yet ready is a programming error
    #[napi]
    pub fn get_session_parameters_snapshot(&self, env: &Env) -> Result<KnownSessionParameters> {
        match self.result.get() {
            None => Err(BridgeError::Message(
                "session parameters snapshot requested before the statement finished".to_string(),
            )
            .to_js_error(*env)),
            Some(Ok(outcome)) => match outcome.rows() {
                Some(data) => Ok((*data.session_params).clone()),
                None => Err(BridgeError::Message(
                    "session parameters snapshot requested on a statement with no result set"
                        .to_string(),
                )
                .to_js_error(*env)),
            },
            Some(Err(error)) => Err(error.to_js_error(*env)),
        }
    }

    #[napi]
    pub fn get_columns(&self, env: &Env) -> Result<Option<Vec<Column>>> {
        Ok(self.ready_rows(env)?.map(|data| {
            data.result_set_descriptor
                .columns
                .iter()
                .enumerate()
                .map(|(i, meta)| Column::from_metadata(i as u32, meta))
                .collect()
        }))
    }

    #[napi]
    pub fn get_column(&self, env: &Env, identifier: Either<String, u32>) -> Result<Option<Column>> {
        let Some(data) = self.ready_rows(env)? else {
            return Ok(None);
        };
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

    // TODO: instead of Node calling close, maybe we should call it when the result set is
    // processed from the bridge
    #[napi]
    pub fn close(&self) -> Result<()> {
        if let Some(Ok(outcome)) = self.result.get()
            && let Some(data) = outcome.rows()
        {
            let _ = DRIVER.result_set_release(data.result_set_handle);
        }
        Ok(())
    }

    // TODO: surface genuine cancel failures (e.g. CancelTimeout) instead of
    // swallowing the outcome.
    #[napi]
    pub async fn cancel(&self) -> Result<()> {
        if let Some(operation_ctx) = &self.operation_ctx {
            operation_ctx.cancel();
        }
        Ok(())
    }
}
