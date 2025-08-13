// Arrow deserialization utilities
pub use arrow::record_batch::RecordBatch;

/// Trait for deserializing Arrow RecordBatch data into Rust structs
pub trait ArrowDeserialize: Sized {
    /// Deserializes a single row from a RecordBatch into Self
    fn deserialize_one(batch: &RecordBatch, row_index: usize) -> Result<Self, String>;

    /// Deserializes all rows from a RecordBatch into a Vec of Self
    fn deserialize_all(batch: &RecordBatch) -> Result<Vec<Self>, String> {
        let num_rows = batch.num_rows();
        let mut result_vec = Vec::with_capacity(num_rows);

        for i in 0..num_rows {
            result_vec.push(Self::deserialize_one(batch, i)?);
        }

        Ok(result_vec)
    }
}

// Re-export the derive macro
pub use arrow_deserialize_macro::ArrowDeserialize;
