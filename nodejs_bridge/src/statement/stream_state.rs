use super::column_reader::ColumnReader;
use crate::session_params::SessionParams;
use crate::sql_value::SqlValue;
use arrow::array::{RecordBatch, RecordBatchReader};
use arrow::error::ArrowError;

pub(super) struct StreamState {
    batch_reader: Box<dyn RecordBatchReader + Send>,
    current_batch: Option<CurrentBatch>,
}

impl StreamState {
    pub(super) fn new(batch_reader: Box<dyn RecordBatchReader + Send>) -> Self {
        Self {
            batch_reader,
            current_batch: None,
        }
    }

    /// `session_params` is a call-time argument, not a stored field --
    /// `ResultData::session_params` is its one owner, passed in fresh by the
    /// caller (a cheap `Arc` clone per call) rather than duplicated here.
    pub(super) fn next_row(
        &mut self,
        session_params: &SessionParams,
    ) -> Result<Option<Vec<SqlValue>>, ArrowError> {
        // Loop so zero-row batches are skipped rather than ending iteration.
        loop {
            if let Some(batch) = &mut self.current_batch {
                if let Some(row) = batch.next_row() {
                    return Ok(Some(row));
                }
                self.current_batch = None;
            }
            match self.batch_reader.next() {
                Some(batch) => {
                    self.current_batch = Some(CurrentBatch::from_batch(&batch?, session_params)?)
                }
                None => return Ok(None),
            }
        }
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
    fn from_batch(batch: &RecordBatch, session_params: &SessionParams) -> Result<Self, ArrowError> {
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

    fn next_row(&mut self) -> Option<Vec<SqlValue>> {
        if self.row_index >= self.num_rows {
            return None;
        }
        let row = self
            .column_readers
            .iter()
            .map(|reader| reader.read(self.row_index))
            .collect();
        self.row_index += 1;
        Some(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{BooleanArray, Int32Array};
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatchIterator;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn boolean_field() -> Field {
        let mut metadata = HashMap::new();
        metadata.insert("logicalType".to_string(), "BOOLEAN".to_string());
        Field::new("B", DataType::Boolean, false).with_metadata(metadata)
    }

    fn session_params(time_format: &str) -> SessionParams {
        SessionParams {
            time_format: Arc::from(time_format),
        }
    }

    /// Duplicates `column_reader.rs`'s `time_field` helper -- test modules
    /// are private, so it can't be shared.
    fn time_field(scale: &str) -> Field {
        let mut metadata = HashMap::new();
        metadata.insert("logicalType".to_string(), "TIME".to_string());
        metadata.insert("scale".to_string(), scale.to_string());
        Field::new("T", DataType::Int32, true).with_metadata(metadata)
    }

    /// `session_params` is a call-time argument now, not a field `next_row`
    /// could accidentally consume/move out of `self` -- the borrow checker
    /// already rules that bug class out. This just proves the batch-
    /// advancing loop itself still works correctly with the new signature.
    /// See `session_params_reach_time_decoder_across_multiple_batches` below
    /// for the version that exercises the real TIME arm end to end.
    #[test]
    fn next_row_advances_through_multiple_batches() {
        let schema = Arc::new(Schema::new(vec![boolean_field()]));
        let batch_a = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(BooleanArray::from(vec![true]))],
        )
        .unwrap();
        let batch_b = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(BooleanArray::from(vec![false]))],
        )
        .unwrap();
        let reader = RecordBatchIterator::new(vec![Ok(batch_a), Ok(batch_b)].into_iter(), schema);

        let params = session_params("HH24:MI:SS");
        let mut state = StreamState::new(Box::new(reader));

        assert!(
            matches!(
                state.next_row(&params).unwrap().as_deref(),
                Some([SqlValue::Bool(true)])
            ),
            "first batch's row should decode to Bool(true)"
        );
        assert!(
            matches!(
                state.next_row(&params).unwrap().as_deref(),
                Some([SqlValue::Bool(false)])
            ),
            "second batch's row should decode to Bool(false)"
        );
        assert!(
            state.next_row(&params).unwrap().is_none(),
            "stream should be exhausted after both batches are drained"
        );
    }

    /// Proves `session_params` doesn't just survive as an opaque value
    /// across batches — it actually reaches `ColumnReader::for_field`'s
    /// TIME arm and is honored by the decoder, for every batch, not just
    /// the first.
    #[test]
    fn session_params_reach_time_decoder_across_multiple_batches() {
        let schema = Arc::new(Schema::new(vec![time_field("3")]));
        // 10:30:00.123 at scale 3: secs=37_800, frac=123.
        let batch_a = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(Int32Array::from(vec![Some(37_800_123)]))],
        )
        .unwrap();
        // 11:00:00.456 at scale 3: secs=39_600, frac=456.
        let batch_b = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(Int32Array::from(vec![Some(39_600_456)]))],
        )
        .unwrap();
        let reader = RecordBatchIterator::new(vec![Ok(batch_a), Ok(batch_b)].into_iter(), schema);

        let params = session_params("HH24:MI:SS.FF3");
        let mut state = StreamState::new(Box::new(reader));

        assert!(
            matches!(
                state.next_row(&params).unwrap().as_deref(),
                Some([SqlValue::String(s)]) if s == "10:30:00.123"
            ),
            "first batch's TIME row should render 3 fractional digits per the threaded format"
        );
        assert!(
            matches!(
                state.next_row(&params).unwrap().as_deref(),
                Some([SqlValue::String(s)]) if s == "11:00:00.456"
            ),
            "second batch's TIME row should also honor the format — proves it reached \
             ColumnReader::for_field's TIME arm on every CurrentBatch::from_batch call, \
             not just the first"
        );
        assert!(
            state.next_row(&params).unwrap().is_none(),
            "stream should be exhausted after both batches are drained"
        );
    }
}
