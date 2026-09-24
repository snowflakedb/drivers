use super::column_reader::ColumnReader;
use crate::error::BridgeError;
use crate::session_params::KnownSessionParameters;
use arrow::array::RecordBatch;
use arrow::error::ArrowError;
use napi::bindgen_prelude::{Array as JsArray, Env};
use sf_core::apis::database_driver_v1::AsyncArrowBatchFetcher;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

/// Parks one query result's current batch so `next_row` can decode JS values
/// on the main thread while `fetch_next_batch` awaits the next batch off it.
pub(super) struct StreamState {
    fetcher: AsyncMutex<AsyncArrowBatchFetcher>,
    batch: Mutex<Option<CurrentBatch>>,
}

impl StreamState {
    pub(super) fn new(fetcher: AsyncArrowBatchFetcher) -> Self {
        Self {
            fetcher: AsyncMutex::new(fetcher),
            batch: Mutex::new(None),
        }
    }

    /// Pulls the next non-empty batch and installs it, returning `false` once
    /// the stream is drained.
    ///
    /// Skipping zero-row batches here rather than letting them through means a
    /// `true` return always guarantees at least one row, so callers can't spin
    /// between this and [`next_row`](Self::next_row) without making progress.
    pub(super) async fn fetch_next_batch(
        &self,
        session_params: &Arc<KnownSessionParameters>,
    ) -> Result<bool, BridgeError> {
        let prepared = {
            let mut fetcher = self.fetcher.lock().await;

            if self.rows_remaining().is_some_and(|remaining| remaining > 0) {
                return Ok(true);
            }

            loop {
                match fetcher.next_batch().await? {
                    Some(batch) if batch.num_rows() > 0 => {
                        break Some(
                            CurrentBatch::from_batch(&batch, session_params)
                                .map_err(|e| BridgeError::Message(e.to_string()))?,
                        );
                    }
                    Some(_) => {}
                    None => break None,
                }
            }
        };

        let mut batch = self.batch.lock().unwrap();
        *batch = prepared;
        Ok(batch.is_some())
    }

    /// Decodes one row of the resident batch into a fresh JS array, or `None`
    /// once that batch is drained and the caller should refill via
    /// [`fetch_next_batch`](Self::fetch_next_batch).
    pub(super) fn next_row<'env>(&self, env: &'env Env) -> napi::Result<Option<JsArray<'env>>> {
        match self.batch.lock().unwrap().as_mut() {
            Some(batch) => batch.next_row(env),
            None => Ok(None),
        }
    }

    fn rows_remaining(&self) -> Option<usize> {
        self.batch
            .lock()
            .unwrap()
            .as_ref()
            .map(CurrentBatch::rows_remaining)
    }
}

/// The batch currently being drained, plus a cursor into it. The
/// `row_index < num_rows` bound is enforced solely by [`next_row`](Self::next_row),
/// so the cursor can never point past the batch.
struct CurrentBatch {
    column_readers: Vec<ColumnReader>,
    num_rows: usize,
    row_index: usize,
}

impl CurrentBatch {
    fn from_batch(
        batch: &RecordBatch,
        session_params: &Arc<KnownSessionParameters>,
    ) -> Result<Self, ArrowError> {
        let column_readers = batch
            .schema()
            .fields()
            .iter()
            .enumerate()
            .map(|(index, field)| {
                ColumnReader::for_field(field, batch.column(index), session_params)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(ArrowError::ComputeError)?;
        Ok(Self {
            column_readers,
            num_rows: batch.num_rows(),
            row_index: 0,
        })
    }

    fn next_row<'env>(&mut self, env: &'env Env) -> napi::Result<Option<JsArray<'env>>> {
        if self.row_index >= self.num_rows {
            return Ok(None);
        }
        let mut row = env.create_array(self.column_readers.len() as u32)?;
        for (index, reader) in self.column_readers.iter().enumerate() {
            row.set(index as u32, reader.read(self.row_index))?;
        }
        self.row_index += 1;
        Ok(Some(row))
    }

    fn rows_remaining(&self) -> usize {
        self.num_rows - self.row_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::BooleanArray;
    use arrow::datatypes::{DataType, Field, Schema};
    use std::collections::HashMap;
    use std::sync::Arc;

    fn boolean_field() -> Field {
        let mut metadata = HashMap::new();
        metadata.insert("logicalType".to_string(), "BOOLEAN".to_string());
        Field::new("B", DataType::Boolean, false).with_metadata(metadata)
    }

    /// BOOLEAN columns ignore the format; `fetch_next_batch` still requires
    /// `KnownSessionParameters` to build the resident batch's column readers.
    fn session_params() -> Arc<KnownSessionParameters> {
        Arc::new(KnownSessionParameters::defaults())
    }

    fn boolean_batch(schema: &Arc<Schema>, values: Vec<bool>) -> RecordBatch {
        RecordBatch::try_new(schema.clone(), vec![Arc::new(BooleanArray::from(values))]).unwrap()
    }

    fn state_over(schema: Arc<Schema>, batches: Vec<RecordBatch>) -> StreamState {
        StreamState::new(AsyncArrowBatchFetcher::from_batches(schema, batches))
    }

    fn fetch_ok(result: Result<bool, BridgeError>) -> bool {
        result.unwrap_or_else(|_| panic!("fetch_next_batch should succeed"))
    }

    /// A `true` return has to guarantee at least one decodable row, otherwise
    /// the JS loop -- which refills only when `next_row` returns null -- would
    /// spin between the two calls without ever making progress.
    #[tokio::test]
    async fn fetch_next_batch_skips_zero_row_batches() {
        let schema = Arc::new(Schema::new(vec![boolean_field()]));
        let state = state_over(
            schema.clone(),
            vec![
                boolean_batch(&schema, vec![]),
                boolean_batch(&schema, vec![]),
                boolean_batch(&schema, vec![true]),
            ],
        );

        assert!(fetch_ok(state.fetch_next_batch(&session_params()).await));
        assert_eq!(
            state.rows_remaining(),
            Some(1),
            "the two empty batches should have been skipped past, not installed"
        );
    }

    /// A duplicate fetch -- two `read()` calls racing on the JS side, say --
    /// must not pull a second batch over the top of an undrained one, which
    /// would silently drop every row still resident.
    #[tokio::test]
    async fn fetch_next_batch_is_idempotent_while_rows_are_still_resident() {
        let schema = Arc::new(Schema::new(vec![boolean_field()]));
        let state = state_over(
            schema.clone(),
            vec![
                boolean_batch(&schema, vec![true, true]),
                boolean_batch(&schema, vec![false]),
            ],
        );
        let params = session_params();

        assert!(fetch_ok(state.fetch_next_batch(&params).await));
        assert_eq!(state.rows_remaining(), Some(2));

        assert!(
            fetch_ok(state.fetch_next_batch(&params).await),
            "a redundant fetch should report the resident batch, not the stream state"
        );
        assert_eq!(
            state.rows_remaining(),
            Some(2),
            "the resident batch's undecoded rows should survive a redundant fetch"
        );
    }

    #[tokio::test]
    async fn fetch_next_batch_reports_exhaustion_when_every_batch_is_empty() {
        let schema = Arc::new(Schema::new(vec![boolean_field()]));
        let state = state_over(schema.clone(), vec![boolean_batch(&schema, vec![])]);

        assert!(
            !fetch_ok(state.fetch_next_batch(&session_params()).await),
            "a stream of only empty batches should be reported as exhausted"
        );
    }
}
