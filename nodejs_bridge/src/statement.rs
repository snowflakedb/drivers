mod stream_state;

use crate::DRIVER;
use crate::error::to_napi_err;
use arrow::array::RecordBatchReader;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use sf_core::handle_manager::Handle;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use stream_state::StreamState;

#[napi]
pub struct Statement {
    handle: Handle,
    result_set_handle: Handle,
    stream_state: Arc<Mutex<StreamState>>,
}

#[napi]
impl Statement {
    pub(crate) fn new(
        handle: Handle,
        result_set_handle: Handle,
        batch_reader: Box<dyn RecordBatchReader + Send>,
    ) -> Self {
        Self {
            handle,
            result_set_handle,
            stream_state: Arc::new(Mutex::new(StreamState::new(batch_reader))),
        }
    }

    #[napi]
    // TODO:
    // - Benchmark per-row vs per-batch returns across NAPI for large results.
    //   Per-row keeps peak JS memory lower; per-batch cuts FFI crossings.
    // - Investigate how .unwrap() usage affects Node and how to properly handle such errors.
    pub async fn get_next_row(&self) -> Result<Option<HashMap<String, i64>>> {
        let state = Arc::clone(&self.stream_state);

        // `next_row` may block; run on the blocking pool so the Node event
        // loop stays responsive.
        tokio::task::spawn_blocking(move || state.lock().unwrap().next_row())
            .await
            .unwrap()
            .map_err(to_napi_err)
    }

    #[napi]
    pub fn close(&mut self) -> Result<()> {
        let _ = DRIVER.result_set_release(self.result_set_handle);
        let _ = DRIVER.statement_release(self.handle);
        Ok(())
    }
}
