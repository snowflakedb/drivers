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
