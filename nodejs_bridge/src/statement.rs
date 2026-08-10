mod column_reader;
mod result;
mod stream_state;

use crate::DRIVER;
use crate::error::to_napi_err;
use crate::sql_value::SqlValue;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use result::{ResultData, StatementResult};
use sf_core::apis::database_driver_v1::{ApiError, ExecuteQueryResult};
use sf_core::handle_manager::Handle;
use snafu::location;
use std::future::Future;
use std::sync::{Arc, Mutex};
use stream_state::StreamState;

#[napi]
pub struct Statement {
    result: StatementResult,
    handle: Option<Handle>,
}

#[napi]
impl Statement {
    pub(crate) fn from_pending(
        handle: Option<Handle>,
        result_future: impl Future<Output = std::result::Result<ExecuteQueryResult, ApiError>>
        + Send
        + 'static,
    ) -> Self {
        Self {
            result: StatementResult::from_future(async move {
                result_data_from(result_future.await?).await
            }),
            handle,
        }
    }

    #[napi]
    pub async fn wait(&self) -> Result<()> {
        self.result.wait().await.as_ref().map_err(to_napi_err)?;
        Ok(())
    }

    // TODO:
    // - Benchmark per-row vs per-batch returns across NAPI for large results.
    //   Per-row keeps peak JS memory lower; per-batch cuts FFI crossings.
    // - Investigate how .unwrap() usage affects Node and how to properly handle such errors.
    // - Once we implement all data types, investigate whether we can use napi Either and have
    //   automatically generated JS and TypeScript types instead of SqlValue trait and unknown cast.
    #[napi(ts_return_type = "Promise<Array<unknown> | null>")]
    pub async fn get_next_row(&self) -> Result<Option<Vec<SqlValue>>> {
        let data = self.result.wait().await.map_err(to_napi_err)?;
        let stream_state = Arc::clone(&data.stream_state);

        // `next_row` may block; run on napi's blocking pool so the Node event
        // loop stays responsive.
        spawn_blocking(move || stream_state.lock().unwrap().next_row())
            .await
            .unwrap()
            .map_err(to_napi_err)
    }

    // TODO:
    // - reusable error handling
    // - maybe an util to get field value so we don't repeat the match
    #[napi]
    pub fn get_query_id(&self) -> Result<Option<String>> {
        match self.result.get() {
            None => Ok(None),
            Some(Ok(data)) => Ok(Some(data.result_set_descriptor.query_id.clone())),
            Some(Err(error)) => Err(to_napi_err(error)),
        }
    }

    #[napi]
    pub fn get_num_rows(&self) -> Result<Option<i64>> {
        match self.result.get() {
            None => Ok(None),
            Some(Ok(data)) => Ok(data.result_set_descriptor.row_count),
            Some(Err(error)) => Err(to_napi_err(error)),
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
        if let Some(handle) = self.handle {
            let _ = DRIVER.statement_cancel(handle).await;
        }
        Ok(())
    }
}

async fn result_data_from(result: ExecuteQueryResult) -> std::result::Result<ResultData, ApiError> {
    let (result_set_handle, result_set_descriptor) = match result {
        ExecuteQueryResult::Single(rs) => (rs.handle, rs.descriptor),
        ExecuteQueryResult::Multi { .. } => {
            return Err(ApiError::InvalidArgument {
                argument: "multi-statement results are not supported yet".to_string(),
                location: location!(),
            });
        }
    };

    let batch_reader = DRIVER.result_set_get_stream(result_set_handle).await?;

    Ok(ResultData {
        result_set_handle,
        result_set_descriptor,
        stream_state: Arc::new(Mutex::new(StreamState::new(batch_reader))),
    })
}
