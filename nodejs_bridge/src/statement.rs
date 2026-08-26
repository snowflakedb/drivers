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
use crate::error::to_napi_err;
use crate::session_params::SessionParams;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use result::{ResultData, StatementResult};
use sf_core::apis::database_driver_v1::{ApiError, ExecuteQueryResult};
use sf_core::handle_manager::Handle;
use snafu::location;
use std::future::Future;
use std::sync::Arc;
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
        conn_handle: Handle,
        result_future: impl Future<Output = std::result::Result<ExecuteQueryResult, ApiError>>
        + Send
        + 'static,
    ) -> Self {
        Self {
            result: StatementResult::from_future(async move {
                result_data_from(result_future.await?, conn_handle).await
            }),
            handle,
        }
    }

    #[napi]
    pub async fn wait_for_completion(&self) -> Result<()> {
        self.result.ready().await.as_ref().map_err(to_napi_err)?;
        Ok(())
    }

    /// Loads the next batch of rows. Returns `false` when the result set is
    /// exhausted. Drain the loaded batch with
    /// [`get_next_row`](Self::get_next_row) before calling this again.
    #[napi]
    pub async fn fetch_next_batch(&self) -> Result<bool> {
        let data = self.result.ready().await.map_err(to_napi_err)?;
        let stream_state = Arc::clone(&data.stream_state);
        let session_params = Arc::clone(&data.session_params);

        // `fetch_next_batch` may block on a chunk download; run on napi's
        // blocking pool so the Node event loop stays responsive.
        spawn_blocking(move || stream_state.fetch_next_batch(&session_params))
            .await
            // TODO: Investigate how .unwrap() usage affects Node and how to properly
            // handle such errors.
            .unwrap()
            .map_err(to_napi_err)
    }

    /// Returns the next row of the current batch, or `null` once that batch
    /// is drained. Call [`fetch_next_batch`](Self::fetch_next_batch) to load
    /// another.
    #[napi]
    pub fn get_next_row<'env>(&self, env: &'env Env) -> Result<Option<Array<'env>>> {
        match self.result.get() {
            None => Ok(None),
            Some(Ok(data)) => data.stream_state.next_row(env),
            Some(Err(error)) => Err(to_napi_err(error)),
        }
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

    #[napi]
    pub fn get_columns(&self) -> Result<Option<Vec<Column>>> {
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
            Some(Err(error)) => Err(to_napi_err(error)),
        }
    }

    #[napi]
    pub fn get_column(&self, identifier: Either<String, u32>) -> Result<Option<Column>> {
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
