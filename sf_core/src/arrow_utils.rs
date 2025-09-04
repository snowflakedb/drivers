use crate::rest::snowflake::query_response::RowType;
use arrow::array::{Array, Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use snafu::{Location, ResultExt, Snafu};
use std::collections::HashMap;
use std::sync::Arc;

/// Maps Snowflake data types to Arrow data types
/// Only supports TEXT and FIXED (with scale 0) types
fn snowflake_type_to_arrow_type(row_type: &RowType) -> Result<DataType, ArrowUtilsError> {
    match row_type.type_.to_uppercase().as_str() {
        "TEXT" => Ok(DataType::Utf8),
        "FIXED" if row_type.scale == Some(0) => Ok(DataType::Int64),
        _ => UnsupportedTypeSnafu {
            snowflake_type: format!("{} (scale: {:?})", row_type.type_, row_type.scale),
        }
        .fail(),
    }
}

/// Creates an Arrow Field from a RowType, embedding Snowflake-like metadata
pub fn create_field(row_type: &RowType) -> Result<Field, ArrowUtilsError> {
    let arrow_type = snowflake_type_to_arrow_type(row_type)?;

    let mut metadata: HashMap<String, String> = HashMap::new();
    let logical_type = row_type.type_.to_uppercase();
    metadata.insert("logicalType".to_string(), logical_type.clone());

    match logical_type.as_str() {
        "TEXT" => {
            metadata.insert("physicalType".to_string(), "LOB".to_string());
            if let Some(len) = row_type.length {
                metadata.insert("charLength".to_string(), len.to_string());
            }
            if let Some(byte_len) = row_type.byte_length {
                metadata.insert("byteLength".to_string(), byte_len.to_string());
            }
        }
        "FIXED" => {
            let scale = row_type.scale.unwrap_or(0);
            metadata.insert("scale".to_string(), scale.to_string());
            if let Some(precision) = row_type.precision {
                metadata.insert("precision".to_string(), precision.to_string());
                metadata.insert(
                    "physicalType".to_string(),
                    physical_type_from_precision_signed(precision),
                );
            } else {
                metadata.insert("physicalType".to_string(), "SB8".to_string());
            }
        }
        _ => UnsupportedTypeSnafu {
            snowflake_type: format!("{} (scale: {:?})", row_type.type_, row_type.scale),
        }
        .fail()?,
    }

    Ok(Field::new(&row_type.name, arrow_type, row_type.nullable).with_metadata(metadata))
}

/// Creates an Arrow array from column values and data type
fn create_column_array(
    values: Vec<&str>,
    data_type: &DataType,
) -> Result<Arc<dyn Array>, ArrowUtilsError> {
    match data_type {
        DataType::Utf8 => Ok(Arc::new(StringArray::from(values))),
        DataType::Int64 => {
            // Convert string values to i64 and return parsing error if any value is invalid
            let int_values: Result<Vec<i64>, ArrowUtilsError> = values
                .into_iter()
                .map(|v| {
                    v.parse::<i64>().context(InvalidIntegerSnafu {
                        value: v.to_string(),
                    })
                })
                .collect();
            Ok(Arc::new(Int64Array::from(int_values?)))
        }
        _ => UnsupportedTypeSnafu {
            snowflake_type: format!("{data_type:?}"),
        }
        .fail(),
    }
}

/// Converts a string rowset with RowType metadata to Arrow format
/// Supports TEXT and FIXED (with scale 0) types, converting strings to appropriate Arrow types
/// Assumes rowset and row_types have been validated to have matching column counts
pub fn convert_string_rowset_to_arrow_reader(
    rowset: &[Vec<String>],
    row_types: &[RowType],
) -> Result<Box<dyn arrow::record_batch::RecordBatchReader + Send>, ArrowUtilsError> {
    // Create Arrow schema from RowType metadata
    let schema = create_schema(row_types)?;

    // Create Arrow arrays for each column
    let columns: Result<Vec<Arc<dyn Array>>, ArrowUtilsError> = row_types
        .iter()
        .enumerate()
        .map(|(col_idx, _row_type)| {
            let values: Vec<&str> = rowset.iter().map(|row| row[col_idx].as_str()).collect();
            let data_type = schema.field(col_idx).data_type();
            create_column_array(values, data_type)
        })
        .collect();

    let columns = columns?;

    boxed_arrow_reader(schema, columns).context(ArrowSnafu)
}

/// Creates an Arrow Schema from a list of RowType definitions
pub fn create_schema(row_types: &[RowType]) -> Result<Arc<Schema>, ArrowUtilsError> {
    let fields: Result<Vec<Field>, ArrowUtilsError> = row_types.iter().map(create_field).collect();
    let fields = fields?;
    Ok(Arc::new(Schema::new(fields)))
}

/// Heuristic mapping from decimal precision (digits) to signed physical storage type
fn physical_type_from_precision_signed(precision: u64) -> String {
    if precision <= 3 {
        "SB1".to_string()
    } else if precision <= 5 {
        "SB2".to_string()
    } else if precision <= 10 {
        "SB4".to_string()
    } else {
        "SB8".to_string()
    }
}

pub fn boxed_arrow_reader(
    schema: Arc<Schema>,
    columns: Vec<Arc<dyn Array>>,
) -> Result<Box<dyn arrow::record_batch::RecordBatchReader + Send>, ArrowError> {
    let batch = RecordBatch::try_new(schema.clone(), columns)?;
    Ok(Box::new(arrow::record_batch::RecordBatchIterator::new(
        vec![Ok(batch)],
        schema,
    )))
}

#[derive(Snafu, Debug)]
pub enum ArrowUtilsError {
    #[snafu(display("Arrow operation failed"))]
    Arrow {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse integer value: {value}"))]
    InvalidInteger {
        value: String,
        source: std::num::ParseIntError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Unsupported Snowflake type: {snowflake_type}. Only TEXT and FIXED with scale 0 are supported"
    ))]
    UnsupportedType {
        snowflake_type: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::snowflake::query_response::RowType;
    use arrow::array::{Int64Array, StringArray};
    use arrow::record_batch::RecordBatchReader;

    #[test]
    fn test_string_rowset_translation_with_metadata_small() {
        // Build a Snowflake-like rowset with elegant text and small integer sizes
        let rowset = vec![
            vec!["alpha.txt".to_string(), "7".to_string()], // SB1
            vec!["beta.md".to_string(), "123".to_string()], // SB2
            vec!["gamma.bin".to_string(), "32767".to_string()], // SB2
            vec!["delta.png".to_string(), "1024".to_string()], // SB2
        ];

        // Describe columns via RowType
        let row_types = vec![
            RowType::text("col_text", false, 16, 64),
            RowType::fixed("col_fixed", false, 5, 0),
        ];

        // Convert to Arrow reader
        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();

        // Validate schema and metadata
        let schema = reader.schema();
        let fields = schema.fields();
        assert_eq!(fields.len(), 2);

        // TEXT column
        assert_eq!(fields[0].name(), "col_text");
        assert_eq!(format!("{:?}", fields[0].data_type()), "Utf8");
        let meta0 = fields[0].metadata();
        assert_eq!(meta0.get("logicalType"), Some(&"TEXT".to_string()));
        assert_eq!(meta0.get("physicalType"), Some(&"LOB".to_string()));
        assert_eq!(meta0.get("charLength"), Some(&"16".to_string()));
        assert_eq!(meta0.get("byteLength"), Some(&"64".to_string()));

        // FIXED column
        assert_eq!(fields[1].name(), "col_fixed");
        assert_eq!(format!("{:?}", fields[1].data_type()), "Int64");
        let meta1 = fields[1].metadata();
        assert_eq!(meta1.get("logicalType"), Some(&"FIXED".to_string()));
        assert_eq!(meta1.get("scale"), Some(&"0".to_string()));
        assert_eq!(meta1.get("precision"), Some(&"5".to_string()));
        assert_eq!(meta1.get("physicalType"), Some(&"SB2".to_string()));

        // Validate values
        if let Some(Ok(batch)) = reader.next() {
            assert_eq!(batch.num_columns(), 2);
            assert_eq!(batch.num_rows(), 4);

            let col0 = batch
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            assert_eq!(col0.value(0), "alpha.txt");
            assert_eq!(col0.value(1), "beta.md");
            assert_eq!(col0.value(2), "gamma.bin");
            assert_eq!(col0.value(3), "delta.png");

            let col1 = batch
                .column(1)
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap();
            assert_eq!(col1.value(0), 7);
            assert_eq!(col1.value(1), 123);
            assert_eq!(col1.value(2), 32_767);
            assert_eq!(col1.value(3), 1_024);
        } else {
            panic!("Expected one record batch");
        }
    }

    #[test]
    fn test_string_rowset_translation_with_metadata_large() {
        // Build a Snowflake-like rowset with elegant text and varying integer sizes
        let rowset = vec![
            vec!["alpha/report.csv".to_string(), "7".to_string()], // SB1
            vec!["beta/readme.md".to_string(), "123".to_string()], // SB2
            vec!["gamma/data.bin".to_string(), "32767".to_string()], // SB2
            vec!["delta/image.png".to_string(), "2147483647".to_string()], // SB4
            vec![
                "epsilon/archive.tar.gz".to_string(),
                "9223372036854775807".to_string(), // SB8
            ],
        ];

        // Describe columns via RowType
        let row_types = vec![
            RowType::text("col_text", false, 64, 256),
            RowType::fixed("col_fixed", false, 19, 0),
        ];

        // Convert to Arrow reader
        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();

        // Validate schema and metadata
        let schema = reader.schema();
        let fields = schema.fields();
        assert_eq!(fields.len(), 2);

        // TEXT column
        assert_eq!(fields[0].name(), "col_text");
        assert_eq!(format!("{:?}", fields[0].data_type()), "Utf8");
        let meta0 = fields[0].metadata();
        assert_eq!(meta0.get("logicalType"), Some(&"TEXT".to_string()));
        assert_eq!(meta0.get("physicalType"), Some(&"LOB".to_string()));
        assert_eq!(meta0.get("charLength"), Some(&"64".to_string()));
        assert_eq!(meta0.get("byteLength"), Some(&"256".to_string()));

        // FIXED column
        assert_eq!(fields[1].name(), "col_fixed");
        assert_eq!(format!("{:?}", fields[1].data_type()), "Int64");
        let meta1 = fields[1].metadata();
        assert_eq!(meta1.get("logicalType"), Some(&"FIXED".to_string()));
        assert_eq!(meta1.get("scale"), Some(&"0".to_string()));
        assert_eq!(meta1.get("precision"), Some(&"19".to_string()));
        assert_eq!(meta1.get("physicalType"), Some(&"SB8".to_string()));

        // Validate values
        if let Some(Ok(batch)) = reader.next() {
            assert_eq!(batch.num_columns(), 2);
            assert_eq!(batch.num_rows(), 5);

            let col0 = batch
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            assert_eq!(col0.value(0), "alpha/report.csv");
            assert_eq!(col0.value(1), "beta/readme.md");
            assert_eq!(col0.value(2), "gamma/data.bin");
            assert_eq!(col0.value(3), "delta/image.png");
            assert_eq!(col0.value(4), "epsilon/archive.tar.gz");

            let col1 = batch
                .column(1)
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap();
            assert_eq!(col1.value(0), 7);
            assert_eq!(col1.value(1), 123);
            assert_eq!(col1.value(2), 32_767);
            assert_eq!(col1.value(3), 2_147_483_647);
            assert_eq!(col1.value(4), 9_223_372_036_854_775_807);
        } else {
            panic!("Expected one record batch");
        }
    }
}
