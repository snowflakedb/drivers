pub use crate::chunks::convert_string_rowset_to_arrow_reader;
use crate::query_types::RowType;
use arrow::array::Array;
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use hex::FromHexError;
use snafu::{Location, Snafu};
use std::collections::HashMap;
use std::sync::Arc;

/// Creates an Arrow Field from a RowType, embedding Snowflake-like metadata
pub fn create_field(row_type: &RowType) -> Result<Field, ArrowUtilsError> {
    create_field_with_type(row_type, None)
}

/// Creates an Arrow Field from a RowType, embedding Snowflake-like metadata
/// Takes specific_data_type to allow overriding the default type inference for FIXED types based on scale/precision
pub fn create_field_with_type(
    row_type: &RowType,
    data_type: Option<DataType>,
) -> Result<Field, ArrowUtilsError> {
    match row_type {
        RowType::Text {
            name,
            nullable,
            length,
            byte_length,
        } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "TEXT".to_string());
            metadata.insert("charLength".to_string(), length.to_string());
            metadata.insert("byteLength".to_string(), byte_length.to_string());
            Ok(
                Field::new(name, data_type.unwrap_or(DataType::Utf8), *nullable)
                    .with_metadata(metadata),
            )
        }
        RowType::Fixed {
            name,
            nullable,
            precision,
            scale,
        } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "FIXED".to_string());
            metadata.insert("scale".to_string(), scale.to_string());
            metadata.insert("precision".to_string(), precision.to_string());
            let data_type = match data_type {
                Some(dt) => Ok(dt),
                None => GenericSnafu {
                    message: "Data type must be provided for FIXED column".to_string(),
                }
                .fail(),
            }?;
            Ok(Field::new(name, data_type, *nullable).with_metadata(metadata))
        }
        RowType::Boolean { name, nullable } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "BOOLEAN".to_string());
            Ok(
                Field::new(name, data_type.unwrap_or(DataType::Boolean), *nullable)
                    .with_metadata(metadata),
            )
        }
        RowType::Real { name, nullable } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "REAL".to_string());
            Ok(
                Field::new(name, data_type.unwrap_or(DataType::Float64), *nullable)
                    .with_metadata(metadata),
            )
        }
        RowType::Date { name, nullable } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "DATE".to_string());
            Ok(
                Field::new(name, data_type.unwrap_or(DataType::Date32), *nullable)
                    .with_metadata(metadata),
            )
        }
        RowType::TimestampNtz {
            name,
            nullable,
            scale,
        }
        | RowType::TimestampLtz {
            name,
            nullable,
            scale,
        } => {
            let logical_type = match row_type {
                RowType::TimestampNtz { .. } => "TIMESTAMP_NTZ",
                RowType::TimestampLtz { .. } => "TIMESTAMP_LTZ",
                _ => unreachable!(),
            };
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), logical_type.to_string());
            metadata.insert("scale".to_string(), scale.to_string());
            let data_type = if scale <= &7 {
                data_type.unwrap_or(DataType::Int64)
            } else {
                data_type.unwrap_or_else(|| {
                    DataType::Struct(
                        vec![
                            Field::new("epoch", DataType::Int64, true)
                                .with_metadata(metadata.clone()),
                            Field::new("fraction", DataType::Int32, true)
                                .with_metadata(metadata.clone()),
                        ]
                        .into(),
                    )
                })
            };
            Ok(Field::new(name, data_type, *nullable).with_metadata(metadata))
        }
        RowType::TimestampTz {
            name,
            nullable,
            scale,
        } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "TIMESTAMP_TZ".to_string());
            metadata.insert("scale".to_string(), scale.to_string());
            let data_type = if scale <= &3 {
                DataType::Struct(
                    vec![
                        Field::new("epoch", DataType::Int64, true).with_metadata(metadata.clone()),
                        Field::new("timezone", DataType::Int32, true)
                            .with_metadata(metadata.clone()),
                    ]
                    .into(),
                )
            } else {
                DataType::Struct(
                    vec![
                        Field::new("epoch", DataType::Int64, true).with_metadata(metadata.clone()),
                        Field::new("fraction", DataType::Int32, true)
                            .with_metadata(metadata.clone()),
                        Field::new("timezone", DataType::Int32, true)
                            .with_metadata(metadata.clone()),
                    ]
                    .into(),
                )
            };
            Ok(Field::new(name, data_type, *nullable).with_metadata(metadata))
        }
        RowType::Time {
            name,
            nullable,
            scale,
        } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "TIME".to_string());
            metadata.insert("scale".to_string(), scale.to_string());
            let data_type = if scale <= &4 {
                data_type.unwrap_or(DataType::Int32)
            } else {
                data_type.unwrap_or(DataType::Int64)
            };
            Ok(Field::new(name, data_type, *nullable).with_metadata(metadata))
        }
        RowType::Binary {
            name,
            nullable,
            length,
            byte_length,
        } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "BINARY".to_string());
            metadata.insert("byteLength".to_string(), byte_length.to_string());
            metadata.insert("charLength".to_string(), length.to_string());
            Ok(
                Field::new(name, data_type.unwrap_or(DataType::Binary), *nullable)
                    .with_metadata(metadata),
            )
        }
        RowType::Decfloat { name, nullable } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "DECFLOAT".to_string());
            let data_type = data_type.unwrap_or_else(|| {
                DataType::Struct(
                    vec![
                        Field::new("exponent", DataType::Int16, false)
                            .with_metadata(metadata.clone()),
                        Field::new("significand", DataType::Binary, false)
                            .with_metadata(metadata.clone()),
                    ]
                    .into(),
                )
            });
            Ok(Field::new(name, data_type, *nullable).with_metadata(metadata))
        }
        RowType::Variant { name, nullable } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "VARIANT".to_string());
            Ok(
                Field::new(name, data_type.unwrap_or(DataType::Utf8), *nullable)
                    .with_metadata(metadata),
            )
        }
        RowType::Object { name, nullable } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "OBJECT".to_string());
            Ok(
                Field::new(name, data_type.unwrap_or(DataType::Utf8), *nullable)
                    .with_metadata(metadata),
            )
        }
        RowType::Array { name, nullable } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "ARRAY".to_string());
            Ok(
                Field::new(name, data_type.unwrap_or(DataType::Utf8), *nullable)
                    .with_metadata(metadata),
            )
        }
    }
}

/// Creates an Arrow Schema from a list of RowType definitions
pub fn create_schema(row_types: &[(RowType, DataType)]) -> Result<Arc<Schema>, ArrowUtilsError> {
    let fields: Vec<Field> = row_types
        .iter()
        .map(|(r, d)| create_field_with_type(r, Some(d.clone())))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Arc::new(Schema::new(fields)))
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

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum ArrowUtilsError {
    #[snafu(display("Arrow operation failed: {source}"))]
    Arrow {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse integer value: {value}"))]
    IntegerParsing {
        value: String,
        source: std::num::ParseIntError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse float value: {value}"))]
    FloatParsing {
        value: String,
        source: std::num::ParseFloatError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse decfloat value: {value}"))]
    DecfloatParsing {
        value: String,
        source: std::num::ParseIntError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse boolean value: {value}"))]
    BooleanParsing {
        value: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse binary value"))]
    BinaryParsing {
        source: FromHexError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Unsupported data type '{data_type}' for row type '{row_type}'"))]
    UnsupportedDataType {
        data_type: String,
        row_type: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Unexpected error: {message}"))]
    Generic {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{BooleanArray, Float64Array, Int64Array, StringArray};
    use arrow::record_batch::RecordBatchReader;

    #[test]
    fn test_string_rowset_translation_with_metadata_small() {
        // Build a Snowflake-like rowset
        let rowset = vec![
            vec![Some("alpha.txt".to_string()), Some("7".to_string())], // SB1
            vec![Some("beta.md".to_string()), Some("123".to_string())], // SB2
            vec![Some("gamma.bin".to_string()), Some("32767".to_string())], // SB2
            vec![Some("delta.png".to_string()), Some("1024".to_string())], // SB2
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
        assert_eq!(meta0.get("charLength"), Some(&"16".to_string()));
        assert_eq!(meta0.get("byteLength"), Some(&"64".to_string()));

        // FIXED column
        assert_eq!(fields[1].name(), "col_fixed");
        assert_eq!(*fields[1].data_type(), DataType::Int64);
        let meta1 = fields[1].metadata();
        assert_eq!(meta1.get("logicalType"), Some(&"FIXED".to_string()));
        assert_eq!(meta1.get("scale"), Some(&"0".to_string()));
        assert_eq!(meta1.get("precision"), Some(&"5".to_string()));

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
        // Build a Snowflake-like rowset
        let rowset = vec![
            vec![Some("alpha/report.csv".to_string()), Some("7".to_string())], // SB1
            vec![Some("beta/readme.md".to_string()), Some("123".to_string())], // SB2
            vec![
                Some("gamma/data.bin".to_string()),
                Some("32767".to_string()),
            ], // SB2
            vec![
                Some("delta/image.png".to_string()),
                Some("2147483647".to_string()),
            ], // SB4
            vec![
                Some("epsilon/archive.tar.gz".to_string()),
                Some("9223372036854775807".to_string()), // SB8
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
        assert_eq!(meta0.get("charLength"), Some(&"64".to_string()));
        assert_eq!(meta0.get("byteLength"), Some(&"256".to_string()));

        // FIXED column
        assert_eq!(fields[1].name(), "col_fixed");
        assert_eq!(*fields[1].data_type(), DataType::Int64);
        let meta1 = fields[1].metadata();
        assert_eq!(meta1.get("logicalType"), Some(&"FIXED".to_string()));
        assert_eq!(meta1.get("scale"), Some(&"0".to_string()));
        assert_eq!(meta1.get("precision"), Some(&"19".to_string()));

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

    #[test]
    fn test_null_values_in_text_column() {
        let rowset = vec![
            vec![Some("hello".to_string())],
            vec![None],
            vec![Some("world".to_string())],
        ];
        let row_types = vec![RowType::text("col", true, 16, 64)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(batch.num_rows(), 3);
        assert!(col.is_valid(0));
        assert_eq!(col.value(0), "hello");
        assert!(col.is_null(1));
        assert!(col.is_valid(2));
        assert_eq!(col.value(2), "world");
    }

    #[test]
    fn test_null_values_in_fixed_column() {
        let rowset = vec![
            vec![Some("42".to_string())],
            vec![None],
            vec![Some("100".to_string())],
        ];
        let row_types = vec![RowType::fixed("col", true, 5, 0)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        assert_eq!(batch.num_rows(), 3);
        let col = batch.column(0);
        let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
        assert!(arr.is_valid(0));
        assert_eq!(arr.value(0), 42);
        assert!(arr.is_null(1));
        assert!(arr.is_valid(2));
        assert_eq!(arr.value(2), 100);
    }

    #[test]
    fn test_all_null_fixed_column() {
        let rowset = vec![vec![None], vec![None]];
        let row_types = vec![RowType::fixed("col", true, 10, 0)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        assert_eq!(batch.num_rows(), 2);
        let col = batch.column(0);
        assert_eq!(col.null_count(), 2);
        assert_eq!(*col.data_type(), DataType::Int64);
    }

    #[test]
    fn test_all_null_fixed_column_small_precision() {
        let rowset = vec![vec![None], vec![None]];
        let row_types = vec![RowType::fixed("col", true, 2, 0)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        assert_eq!(batch.num_rows(), 2);
        let col = batch.column(0);
        assert_eq!(col.null_count(), 2);
        assert_eq!(*col.data_type(), DataType::Int64);
    }

    #[test]
    fn test_all_null_fixed_column_with_scale() {
        let rowset = vec![vec![None], vec![None]];
        let row_types = vec![RowType::fixed("col", true, 10, 2)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        assert_eq!(batch.num_rows(), 2);
        let col = batch.column(0);
        assert_eq!(col.null_count(), 2);
        assert_eq!(*col.data_type(), DataType::Int64);
        let schema = batch.schema();
        let field = schema.field(0);
        assert_eq!(field.metadata().get("scale").unwrap(), "2");
    }

    #[test]
    fn test_null_values_in_fixed_column_medium() {
        let rowset = vec![
            vec![Some("1000".to_string())],
            vec![None],
            vec![Some("2000".to_string())],
        ];
        let row_types = vec![RowType::fixed("col_fixed", true, 5, 0)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        if let Some(Ok(batch)) = reader.next() {
            let col = batch
                .column(0)
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap();
            assert_eq!(col.len(), 3);
            assert!(!col.is_null(0));
            assert_eq!(col.value(0), 1000);
            assert!(col.is_null(1));
            assert!(!col.is_null(2));
            assert_eq!(col.value(2), 2000);
        } else {
            panic!("Expected one record batch");
        }
    }

    #[test]
    fn test_null_values_in_fixed_column_large() {
        let rowset = vec![
            vec![Some("100000".to_string())],
            vec![None],
            vec![Some("200000".to_string())],
        ];
        let row_types = vec![RowType::fixed("col_fixed", true, 10, 0)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        if let Some(Ok(batch)) = reader.next() {
            let col = batch
                .column(0)
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap();
            assert_eq!(col.len(), 3);
            assert!(!col.is_null(0));
            assert_eq!(col.value(0), 100000);
            assert!(col.is_null(1));
            assert!(!col.is_null(2));
            assert_eq!(col.value(2), 200000);
        } else {
            panic!("Expected one record batch");
        }
    }

    #[test]
    fn test_null_values_in_fixed_column_int64() {
        let rowset = vec![
            vec![Some("3000000000".to_string())],
            vec![None],
            vec![Some("4000000000".to_string())],
        ];
        let row_types = vec![RowType::fixed("col_fixed", true, 19, 0)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        if let Some(Ok(batch)) = reader.next() {
            let col = batch
                .column(0)
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap();
            assert_eq!(col.len(), 3);
            assert!(!col.is_null(0));
            assert_eq!(col.value(0), 3_000_000_000);
            assert!(col.is_null(1));
            assert!(!col.is_null(2));
            assert_eq!(col.value(2), 4_000_000_000);
        } else {
            panic!("Expected one record batch");
        }
    }

    #[test]
    fn test_all_null_fixed_column_high_precision() {
        let rowset = vec![vec![None], vec![None]];
        let row_types = vec![RowType::fixed("col", true, 20, 0)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        assert_eq!(batch.num_rows(), 2);
        let col = batch.column(0);
        assert_eq!(col.null_count(), 2);
        assert_eq!(*col.data_type(), DataType::Int64);
    }

    #[test]
    fn test_null_values_in_fixed_column_decimal128() {
        let rowset = vec![
            vec![Some("12345678901234567890".to_string())],
            vec![None],
            vec![Some("98765432109876543210".to_string())],
        ];
        let row_types = vec![RowType::fixed("col_fixed", true, 38, 0)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        if let Some(Ok(batch)) = reader.next() {
            let col = batch
                .column(0)
                .as_any()
                .downcast_ref::<arrow::array::Decimal128Array>()
                .unwrap();
            assert_eq!(col.len(), 3);
            assert!(!col.is_null(0));
            assert_eq!(col.value(0), 12_345_678_901_234_567_890);
            assert!(col.is_null(1));
            assert!(!col.is_null(2));
            assert_eq!(col.value(2), 98_765_432_109_876_543_210);
        } else {
            panic!("Expected one record batch");
        }
    }

    #[test]
    fn test_fixed_column_with_scale() {
        let rowset = vec![
            vec![Some("123.45".to_string())],
            vec![None],
            vec![Some("678.90".to_string())],
        ];
        let row_types = vec![RowType::fixed("col_fixed", true, 10, 2)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        if let Some(Ok(batch)) = reader.next() {
            let col = batch
                .column(0)
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap();
            assert_eq!(*col.data_type(), DataType::Int64);
            assert_eq!(col.len(), 3);
            assert!(!col.is_null(0));
            assert_eq!(col.value(0), 12345);
            assert!(col.is_null(1));
            assert!(!col.is_null(2));
            assert_eq!(col.value(2), 67890);
        } else {
            panic!("Expected one record batch");
        }
    }

    #[test]
    fn test_fixed_column_with_scale_falls_back_to_decimal128_for_large_values() {
        let rowset = vec![vec![Some("99999999999999999.99".to_string())], vec![None]];
        let row_types = vec![RowType::fixed("col_fixed", true, 38, 2)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        if let Some(Ok(batch)) = reader.next() {
            let col = batch
                .column(0)
                .as_any()
                .downcast_ref::<arrow::array::Decimal128Array>()
                .unwrap();
            assert_eq!(*col.data_type(), DataType::Decimal128(38, 2));
            assert_eq!(col.len(), 2);
            assert!(!col.is_null(0));
            assert_eq!(col.value(0), 9999999999999999999i128);
            assert!(col.is_null(1));
        } else {
            panic!("Expected one record batch");
        }
    }

    #[test]
    fn test_null_values_in_boolean_column() {
        let rowset = vec![
            vec![Some("true".to_string())],
            vec![None],
            vec![Some("false".to_string())],
        ];
        let row_types = vec![RowType::boolean("col", true)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<BooleanArray>()
            .unwrap();
        assert_eq!(batch.num_rows(), 3);
        assert!(col.is_valid(0));
        assert!(col.value(0));
        assert!(col.is_null(1));
        assert!(col.is_valid(2));
        assert!(!col.value(2));
    }

    #[test]
    fn test_null_values_in_real_column() {
        let rowset = vec![
            vec![Some("1.5".to_string())],
            vec![None],
            vec![Some("2.5".to_string())],
        ];
        let row_types = vec![RowType::real("col", true)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap();
        assert_eq!(batch.num_rows(), 3);
        assert!(col.is_valid(0));
        assert!((col.value(0) - 1.5).abs() < f64::EPSILON);
        assert!(col.is_null(1));
        assert!(col.is_valid(2));
        assert!((col.value(2) - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_null_values_in_date_column() {
        let rowset = vec![
            vec![Some("19000".to_string())],
            vec![None],
            vec![Some("19500".to_string())],
        ];
        let row_types = vec![RowType::date("col", true)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch.column(0);
        assert_eq!(batch.num_rows(), 3);
        assert!(col.is_valid(0));
        assert!(col.is_null(1));
        assert!(col.is_valid(2));
    }

    #[test]
    fn test_null_values_in_timestamp_ntz_int64() {
        let rowset = vec![
            vec![Some("1234567.890".to_string())],
            vec![None],
            vec![Some("9876543.210".to_string())],
        ];
        // scale <= 7 uses Int64 representation
        let row_types = vec![RowType::timestamp_ntz("col", true, 3)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        assert_eq!(batch.num_rows(), 3);
        assert!(col.is_valid(0));
        assert!(col.is_null(1));
        assert!(col.is_valid(2));
    }

    #[test]
    fn test_null_values_in_timestamp_ltz_int64() {
        let rowset = vec![
            vec![Some("1234567.890".to_string())],
            vec![None],
            vec![Some("9876543.210".to_string())],
        ];
        let row_types = vec![RowType::timestamp_ltz("col", true, 3)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        assert_eq!(batch.num_rows(), 3);
        assert!(col.is_valid(0));
        assert!(col.is_null(1));
        assert!(col.is_valid(2));
    }

    #[test]
    fn test_null_values_in_timestamp_ntz_struct_column() {
        let rowset = vec![
            vec![Some("1609459200.123456789".to_string())],
            vec![None],
            vec![Some("1609545600.987654321".to_string())],
        ];
        let row_types = vec![RowType::timestamp_ntz("col_ts", true, 9)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        if let Some(Ok(batch)) = reader.next() {
            let col = batch
                .column(0)
                .as_any()
                .downcast_ref::<arrow::array::StructArray>()
                .unwrap();
            assert_eq!(col.len(), 3);
            assert!(!col.is_null(0));
            assert!(col.is_null(1));
            assert!(!col.is_null(2));

            let epoch_col = col.column(0).as_any().downcast_ref::<Int64Array>().unwrap();
            assert_eq!(epoch_col.value(0), 1609459200);
            assert_eq!(epoch_col.value(2), 1609545600);
        } else {
            panic!("Expected one record batch");
        }
    }

    #[test]
    fn test_null_values_in_timestamp_ltz_struct() {
        let rowset = vec![
            vec![Some("1234567.890000000".to_string())],
            vec![None],
            vec![Some("9876543.210000000".to_string())],
        ];
        let row_types = vec![RowType::timestamp_ltz("col", true, 9)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch.column(0);
        assert_eq!(batch.num_rows(), 3);
        assert!(col.is_valid(0));
        assert!(col.is_null(1));
        assert!(col.is_valid(2));
    }

    #[test]
    fn test_mixed_nulls_across_multiple_columns() {
        // Simulates a SHOW SCHEMAS-like result with mixed types and nulls
        let rowset = vec![
            vec![
                Some("schema_a".to_string()),
                Some("5".to_string()),
                Some("a comment".to_string()),
            ],
            vec![Some("schema_b".to_string()), None, None],
        ];
        let row_types = vec![
            RowType::text("name", false, 64, 256),
            RowType::fixed("count", true, 5, 0),
            RowType::text("comment", true, 256, 1024),
        ];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.num_columns(), 3);

        // name column: no nulls
        let names = batch
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert!(names.is_valid(0));
        assert!(names.is_valid(1));
        assert_eq!(names.value(0), "schema_a");
        assert_eq!(names.value(1), "schema_b");

        // count column: second row null
        let counts = batch.column(1);
        assert!(counts.is_valid(0));
        assert!(counts.is_null(1));

        // comment column: second row null
        let comments = batch
            .column(2)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert!(comments.is_valid(0));
        assert_eq!(comments.value(0), "a comment");
        assert!(comments.is_null(1));
    }
}
