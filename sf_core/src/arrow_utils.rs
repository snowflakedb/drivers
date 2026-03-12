use crate::query_types::RowType;
use crate::rest::snowflake::query_response::JsonRowset;
use arrow::array::{Array, BooleanArray, Float64Array, Int8Array, Int64Array, StringArray};
use arrow::datatypes::{DataType, Date32Type, Field, Int32Type, Int64Type, Schema};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use snafu::{Location, ResultExt, Snafu};
use std::collections::HashMap;
use std::sync::Arc;

/// Creates an Arrow Field from a RowType, embedding Snowflake-like metadata
/// Takes specific_data_type to allow overriding the default type inference for FIXED types based on scale/precision
pub fn create_field_with_type(row_type: &RowType, data_type: DataType) -> Field {
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
            Field::new(name, data_type, *nullable).with_metadata(metadata)
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
            Field::new(name, data_type, *nullable).with_metadata(metadata)
        }
        RowType::Boolean { name, nullable } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "BOOLEAN".to_string());
            Field::new(name, data_type, *nullable).with_metadata(metadata)
        }
        RowType::Real { name, nullable } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "REAL".to_string());
            Field::new(name, data_type, *nullable).with_metadata(metadata)
        }
        RowType::Date { name, nullable } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "DATE".to_string());
            Field::new(name, data_type, *nullable).with_metadata(metadata)
        }
        RowType::TimestampNtz {
            name,
            nullable,
            scale,
        } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "TIMESTAMP_NTZ".to_string());
            metadata.insert("scale".to_string(), scale.to_string());
            let fields = vec![
                Field::new("epoch", DataType::Int64, false),
                Field::new("fraction", DataType::Int32, false),
            ];
            Field::new(name, DataType::Struct(fields.into()), *nullable).with_metadata(metadata)
        }
    }
}

/// Parses a decimal string like "123.45" into the unscaled i128 representation
/// that Arrow's Decimal128Array expects. For scale=2, "123.45" becomes 12345i128.
fn parse_decimal_str(v: &str, scale: u32) -> Result<i128, ArrowUtilsError> {
    if scale == 0 {
        return v.parse::<i128>().context(IntegerParsingSnafu {
            value: v.to_string(),
        });
    }

    let (integer_str, frac_str) = match v.split_once('.') {
        Some((int_part, frac_part)) => (int_part, frac_part),
        None => (v, ""),
    };

    let negative = integer_str.starts_with('-');
    let abs_int: i128 = integer_str
        .trim_start_matches('-')
        .parse::<i128>()
        .context(IntegerParsingSnafu {
            value: v.to_string(),
        })?;

    let frac_scaled: i128 = if frac_str.is_empty() {
        0
    } else {
        let scale_usize = scale as usize;
        // Pad with trailing zeros or truncate to match the target scale
        let adjusted = if frac_str.len() < scale_usize {
            format!("{:0<width$}", frac_str, width = scale_usize)
        } else {
            frac_str[..scale_usize].to_string()
        };
        adjusted.parse::<i128>().context(IntegerParsingSnafu {
            value: v.to_string(),
        })?
    };

    let unscaled = abs_int * 10i128.pow(scale) + frac_scaled;
    Ok(if negative { -unscaled } else { unscaled })
}

/// Creates an Arrow array from column values and data type
fn create_column_array(
    values: Vec<Option<&str>>,
    row_type: &RowType,
) -> Result<(Field, Arc<dyn Array>), ArrowUtilsError> {
    match row_type {
        RowType::Text { .. } => Ok((
            create_field_with_type(row_type, DataType::Utf8),
            Arc::new(StringArray::from(values)),
        )),
        RowType::Fixed {
            scale, precision, ..
        } => {
            let decimal_values: Result<Vec<Option<i128>>, ArrowUtilsError> = values
                .into_iter()
                .map(|v| match v {
                    Some(s) => parse_decimal_str(s, *scale as u32).map(Some),
                    None => Ok(None),
                })
                .collect();

            let decimal_values = decimal_values?;
            let non_null_values: Vec<i128> = decimal_values.iter().filter_map(|v| *v).collect();

            if non_null_values.is_empty() {
                return Ok((
                    create_field_with_type(row_type, DataType::Int64), // TODO is it correct? We have to assume something, but it probably doesn't matter.
                    Arc::new(Int64Array::new_null(decimal_values.len())),
                ));
            }
            let min_value = *non_null_values.iter().min().unwrap();
            let max_value = *non_null_values.iter().max().unwrap();

            if min_value >= i8::MIN as i128 && max_value <= i8::MAX as i128 {
                let int8_values: Vec<Option<i8>> = decimal_values
                    .into_iter()
                    .map(|v| v.map(|x| x as i8))
                    .collect();
                Ok((
                    create_field_with_type(row_type, DataType::Int8),
                    Arc::new(Int8Array::from(int8_values)),
                ))
            } else if min_value >= i16::MIN as i128 && max_value <= i16::MAX as i128 {
                let int16_values: Vec<Option<i16>> = decimal_values
                    .into_iter()
                    .map(|v| v.map(|x| x as i16))
                    .collect();
                Ok((
                    create_field_with_type(row_type, DataType::Int16),
                    Arc::new(arrow::array::Int16Array::from(int16_values)),
                ))
            } else if min_value >= i32::MIN as i128 && max_value <= i32::MAX as i128 {
                let int32_values: Vec<Option<i32>> = decimal_values
                    .into_iter()
                    .map(|v| v.map(|x| x as i32))
                    .collect();
                Ok((
                    create_field_with_type(row_type, DataType::Int32),
                    Arc::new(arrow::array::Int32Array::from(int32_values)),
                ))
            } else if min_value >= i64::MIN as i128 && max_value <= i64::MAX as i128 {
                let int64_values: Vec<Option<i64>> = decimal_values
                    .into_iter()
                    .map(|v| v.map(|x| x as i64))
                    .collect();
                Ok((
                    create_field_with_type(row_type, DataType::Int64),
                    Arc::new(Int64Array::from(int64_values)),
                ))
            } else {
                Ok((
                    create_field_with_type(
                        row_type,
                        DataType::Decimal128(*precision as u8, *scale as i8),
                    ),
                    Arc::new(
                        arrow::array::Decimal128Array::from(decimal_values)
                            .with_precision_and_scale(*precision as u8, *scale as i8)
                            .expect("valid decimal precision/scale"),
                    ),
                ))
            }
        }
        RowType::Boolean { .. } => {
            let bool_values: Result<Vec<Option<bool>>, ArrowUtilsError> = values
                .into_iter()
                .map(|v| match v {
                    Some("true") => Ok(Some(true)),
                    Some("false") => Ok(Some(false)),
                    None => Ok(None),
                    Some(other) => BooleanParsingSnafu {
                        value: other.to_string(),
                    }
                    .fail(),
                })
                .collect();
            Ok((
                create_field_with_type(row_type, DataType::Boolean),
                Arc::new(BooleanArray::from(bool_values?)),
            ))
        }
        RowType::Real { .. } => {
            let float_values: Result<Vec<Option<f64>>, ArrowUtilsError> = values
                .into_iter()
                .map(|v| match v {
                    Some(s) => s.parse::<f64>().map(Some).context(FloatParsingSnafu {
                        value: s.to_string(),
                    }),
                    None => Ok(None),
                })
                .collect();
            Ok((
                create_field_with_type(row_type, DataType::Float64),
                Arc::new(Float64Array::from(float_values?)),
            ))
        }
        RowType::Date { .. } => {
            let day_values: Result<Vec<Option<i32>>, ArrowUtilsError> = values
                .into_iter()
                .map(|v| match v {
                    Some(s) => s.parse::<i32>().map(Some).context(IntegerParsingSnafu {
                        value: s.to_string(),
                    }),
                    None => Ok(None),
                })
                .collect();
            Ok((
                create_field_with_type(row_type, DataType::Date32),
                Arc::new(arrow::array::PrimitiveArray::<Date32Type>::from(
                    day_values?,
                )),
            ))
        }
        RowType::TimestampNtz { .. } => {
            let epoch: Arc<dyn Array> = Arc::new(arrow::array::PrimitiveArray::<Int64Type>::from(
                Vec::<i64>::new(),
            ));
            let fraction: Arc<dyn Array> = Arc::new(
                arrow::array::PrimitiveArray::<Int32Type>::from(Vec::<i32>::new()),
            );
            let values = vec![
                (Arc::new(Field::new("epoch", DataType::Int64, false)), epoch),
                (
                    Arc::new(Field::new("fraction", DataType::Int32, false)),
                    fraction,
                ),
            ];
            let data_type = DataType::Struct(
                vec![
                    Field::new("epoch", DataType::Int64, false),
                    Field::new("fraction", DataType::Int32, false),
                ]
                .into(),
            );
            Ok((
                create_field_with_type(row_type, data_type),
                Arc::new(arrow::array::StructArray::from(values)),
            ))
        }
    }
}

/// Converts a string rowset with RowType metadata to Arrow format
/// Supports TEXT and FIXED (with scale 0) types, converting strings to appropriate Arrow types
/// Assumes rowset and row_types have been validated to have matching column counts
pub fn convert_string_rowset_to_arrow_reader(
    rowset: &JsonRowset,
    row_types: &[RowType],
) -> Result<Box<dyn arrow::record_batch::RecordBatchReader + Send>, ArrowUtilsError> {
    // Create Arrow arrays for each column
    #[allow(clippy::type_complexity)]
    let schema_and_columns: Result<Vec<(Field, Arc<dyn Array>)>, ArrowUtilsError> = row_types
        .iter()
        .enumerate()
        .map(|(col_idx, row_type)| {
            let values: Vec<Option<&str>> =
                rowset.iter().map(|row| row[col_idx].as_deref()).collect();
            create_column_array(values, row_type)
        })
        .collect();

    let (fields, columns): (Vec<Field>, Vec<Arc<dyn Array>>) =
        schema_and_columns?.into_iter().unzip();
    let schema = Arc::new(Schema::new(fields));

    boxed_arrow_reader(schema, columns).context(ArrowSnafu)
}

/// Creates an Arrow Schema from a list of RowType definitions
pub fn create_schema(row_types: &[(RowType, DataType)]) -> Result<Arc<Schema>, ArrowUtilsError> {
    let fields: Vec<Field> = row_types
        .iter()
        .map(|(r, d)| create_field_with_type(r, d.clone()))
        .collect();
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
    #[snafu(display("Arrow operation failed"))]
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
    #[snafu(display("Failed to parse boolean value: {value}"))]
    BooleanParsing {
        value: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{
        Array, BooleanArray, Decimal128Array, Float64Array, Int8Array, Int16Array, Int32Array,
        Int64Array, StringArray,
    };
    use arrow::datatypes::Date32Type;
    use arrow::record_batch::RecordBatchReader;

    #[test]
    fn test_string_rowset_translation_with_metadata_small() {
        let rowset = vec![
            vec![Some("alpha.txt".to_string()), Some("7".to_string())],
            vec![Some("beta.md".to_string()), Some("123".to_string())],
            vec![Some("gamma.bin".to_string()), Some("32767".to_string())],
            vec![Some("delta.png".to_string()), Some("1024".to_string())],
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
        assert_eq!(format!("{:?}", fields[1].data_type()), "Int16");
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
                .downcast_ref::<Int16Array>()
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
        let rowset = vec![
            vec![Some("alpha/report.csv".to_string()), Some("7".to_string())],
            vec![Some("beta/readme.md".to_string()), Some("123".to_string())],
            vec![
                Some("gamma/data.bin".to_string()),
                Some("32767".to_string()),
            ],
            vec![
                Some("delta/image.png".to_string()),
                Some("2147483647".to_string()),
            ],
            vec![
                Some("epsilon/archive.tar.gz".to_string()),
                Some("9223372036854775807".to_string()),
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
        assert_eq!(format!("{:?}", fields[1].data_type()), "Int64");
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
    fn test_null_handling_real() {
        let rowset = vec![
            vec![Some("3.125".to_string())],
            vec![None],
            vec![Some("99.5".to_string())],
        ];
        let row_types = vec![RowType::real("col_real", true)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap();
        assert_eq!(col.len(), 3);
        assert!(!col.is_null(0));
        assert_eq!(col.value(0), 3.125);
        assert!(col.is_null(1));
        assert!(!col.is_null(2));
        assert_eq!(col.value(2), 99.5);
    }

    #[test]
    fn test_null_handling_date() {
        let rowset = vec![
            vec![Some("19000".to_string())],
            vec![None],
            vec![Some("20000".to_string())],
        ];
        let row_types = vec![RowType::date("col_date", true)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<arrow::array::PrimitiveArray<Date32Type>>()
            .unwrap();
        assert_eq!(col.len(), 3);
        assert!(!col.is_null(0));
        assert_eq!(col.value(0), 19000);
        assert!(col.is_null(1));
        assert!(!col.is_null(2));
        assert_eq!(col.value(2), 20000);
    }

    #[test]
    fn test_null_handling_fixed_all_nulls() {
        let rowset = vec![vec![None], vec![None]];
        let row_types = vec![RowType::fixed("col_fixed", true, 5, 0)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .expect("all-nulls Fixed should produce Int64Array fallback type");
        assert_eq!(col.len(), 2);
        assert!(col.is_null(0));
        assert!(col.is_null(1));
    }

    #[test]
    fn test_null_handling_multi_column() {
        let rowset = vec![
            vec![
                Some("hello".to_string()),
                Some("42".to_string()),
                Some("true".to_string()),
            ],
            vec![None, None, None],
            vec![
                Some("world".to_string()),
                Some("-7".to_string()),
                Some("false".to_string()),
            ],
        ];
        let row_types = vec![
            RowType::text("col_text", true, 16, 64),
            RowType::fixed("col_fixed", true, 5, 0),
            RowType::boolean("col_bool", true),
        ];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();
        assert_eq!(batch.num_columns(), 3);
        assert_eq!(batch.num_rows(), 3);

        let text_col = batch
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert!(!text_col.is_null(0));
        assert_eq!(text_col.value(0), "hello");
        assert!(text_col.is_null(1));
        assert!(!text_col.is_null(2));
        assert_eq!(text_col.value(2), "world");

        let fixed_col = batch
            .column(1)
            .as_any()
            .downcast_ref::<Int8Array>()
            .unwrap();
        assert!(!fixed_col.is_null(0));
        assert_eq!(fixed_col.value(0), 42);
        assert!(fixed_col.is_null(1));
        assert!(!fixed_col.is_null(2));
        assert_eq!(fixed_col.value(2), -7);

        let bool_col = batch
            .column(2)
            .as_any()
            .downcast_ref::<BooleanArray>()
            .unwrap();
        assert!(!bool_col.is_null(0));
        assert!(bool_col.value(0));
        assert!(bool_col.is_null(1));
        assert!(!bool_col.is_null(2));
        assert!(!bool_col.value(2));
    }

    #[test]
    fn test_null_handling_fixed_int64_narrowing() {
        let rowset = vec![
            vec![None],
            vec![Some("9223372036854775807".to_string())],
            vec![None],
            vec![Some("42".to_string())],
        ];
        let row_types = vec![RowType::fixed("col_fixed", true, 19, 0)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .expect("should narrow to Int64Array based on non-null values only");
        assert_eq!(col.len(), 4);
        assert!(col.is_null(0));
        assert!(!col.is_null(1));
        assert_eq!(col.value(1), 9_223_372_036_854_775_807);
        assert!(col.is_null(2));
        assert!(!col.is_null(3));
        assert_eq!(col.value(3), 42);
    }

    #[test]
    fn test_null_handling_fixed_decimal128() {
        let rowset = vec![
            vec![Some("170141183460469231731687303715884105727".to_string())],
            vec![None],
        ];
        let row_types = vec![RowType::fixed("col_fixed", true, 38, 0)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .expect("should fall through to Decimal128Array for values exceeding i64 range");
        assert_eq!(col.len(), 2);
        assert!(!col.is_null(0));
        assert_eq!(
            col.value(0),
            170_141_183_460_469_231_731_687_303_715_884_105_727
        );
        assert!(col.is_null(1));
    }

    #[test]
    fn test_null_handling_all_nulls_boolean() {
        let rowset = vec![vec![None], vec![None], vec![None]];
        let row_types = vec![RowType::boolean("col_bool", true)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<BooleanArray>()
            .unwrap();
        assert_eq!(col.len(), 3);
        assert!(col.is_null(0));
        assert!(col.is_null(1));
        assert!(col.is_null(2));
    }

    #[test]
    fn test_null_handling_staggered_nulls_across_columns() {
        let rowset = vec![
            vec![None, Some("42".to_string())],
            vec![Some("hello".to_string()), None],
        ];
        let row_types = vec![
            RowType::text("col_text", true, 16, 64),
            RowType::fixed("col_fixed", true, 5, 0),
        ];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();
        assert_eq!(batch.num_columns(), 2);
        assert_eq!(batch.num_rows(), 2);

        let text_col = batch
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert!(text_col.is_null(0));
        assert!(!text_col.is_null(1));
        assert_eq!(text_col.value(1), "hello");

        let fixed_col = batch
            .column(1)
            .as_any()
            .downcast_ref::<Int8Array>()
            .unwrap();
        assert!(!fixed_col.is_null(0));
        assert_eq!(fixed_col.value(0), 42);
        assert!(fixed_col.is_null(1));
    }

    #[test]
    fn test_null_handling_fixed_i16_narrowing() {
        let rowset = vec![
            vec![Some("1000".to_string())],
            vec![None],
            vec![Some("-1000".to_string())],
        ];
        let row_types = vec![RowType::fixed("col_fixed", true, 5, 0)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int16Array>()
            .expect("values in i16 range should narrow to Int16Array");
        assert_eq!(col.len(), 3);
        assert!(!col.is_null(0));
        assert_eq!(col.value(0), 1000);
        assert!(col.is_null(1));
        assert!(!col.is_null(2));
        assert_eq!(col.value(2), -1000);
    }

    #[test]
    fn test_null_handling_fixed_i32_narrowing() {
        let rowset = vec![
            vec![Some("100000".to_string())],
            vec![None],
            vec![Some("-100000".to_string())],
        ];
        let row_types = vec![RowType::fixed("col_fixed", true, 10, 0)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int32Array>()
            .expect("values in i32 range should narrow to Int32Array");
        assert_eq!(col.len(), 3);
        assert!(!col.is_null(0));
        assert_eq!(col.value(0), 100_000);
        assert!(col.is_null(1));
        assert!(!col.is_null(2));
        assert_eq!(col.value(2), -100_000);
    }

    #[test]
    fn test_null_handling_all_nulls_text() {
        let rowset = vec![vec![None], vec![None]];
        let row_types = vec![RowType::text("col_text", true, 16, 64)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(col.len(), 2);
        assert!(col.is_null(0));
        assert!(col.is_null(1));
    }

    #[test]
    fn test_null_handling_all_nulls_real() {
        let rowset = vec![vec![None], vec![None]];
        let row_types = vec![RowType::real("col_real", true)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap();
        assert_eq!(col.len(), 2);
        assert!(col.is_null(0));
        assert!(col.is_null(1));
    }

    #[test]
    fn test_null_handling_all_nulls_date() {
        let rowset = vec![vec![None], vec![None]];
        let row_types = vec![RowType::date("col_date", true)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let batch = reader.next().unwrap().unwrap();

        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<arrow::array::PrimitiveArray<Date32Type>>()
            .unwrap();
        assert_eq!(col.len(), 2);
        assert!(col.is_null(0));
        assert!(col.is_null(1));
    }
}
