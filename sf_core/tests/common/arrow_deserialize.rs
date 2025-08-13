// Arrow deserialization utilities
pub use arrow::record_batch::RecordBatch;

/// Trait for deserializing Arrow RecordBatch data into Rust structs
///
/// This trait is automatically implemented when you derive `ArrowDeserialize` on a struct.
///
/// # Example
///
/// ```ignore
/// use common::arrow_deserialize::{ArrowDeserialize, RecordBatch};
///
/// #[derive(ArrowDeserialize)]
/// struct Person {
///     id: i64,
///     name: String,
/// }
///
/// fn process_batch(batch: &RecordBatch) {
///     // Deserialize a single row (row 0)
///     let person: Person = Person::deserialize_one(batch, 0).unwrap();
///     
///     // Deserialize all rows
///     let people: Vec<Person> = Person::deserialize_all(batch).unwrap();
/// }
/// ```
pub trait ArrowDeserialize: Sized {
    /// Deserializes a single row from a RecordBatch into Self
    ///
    /// # Arguments
    ///
    /// * `batch` - The Arrow RecordBatch to deserialize from
    /// * `row_index` - The index of the row to deserialize (0-based)
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Self` on success, or an error string on failure.
    ///
    /// # Errors
    ///
    /// Returns an error string if the conversion fails, for instance due to:
    /// - Schema mismatch (wrong number of columns)
    /// - Type casting errors (incompatible column types)
    /// - Missing or null data where non-null values are expected
    /// - Row index out of bounds
    fn deserialize_one(batch: &RecordBatch, row_index: usize) -> Result<Self, String>;

    /// Deserializes all rows from a RecordBatch into a Vec of Self
    ///
    /// This method has a default implementation that calls `deserialize_one` for each row.
    ///
    /// # Arguments
    ///
    /// * `batch` - The Arrow RecordBatch to deserialize from
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<Self>` on success, or an error string on failure.
    ///
    /// # Errors
    ///
    /// Returns an error string if the conversion fails for any row.
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
