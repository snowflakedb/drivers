use arrow::array::{
    Array, BinaryBuilder, BooleanArray, Date32Array, Decimal128Array, Decimal128Builder,
    Float32Array, Float64Array, Int32Builder, Int64Array, Int64Builder, StringArray, StructArray,
    Time64NanosecondArray, TimestampNanosecondArray,
};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use snafu::{GenerateImplicitData, Location, ResultExt, Snafu};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

use crate::query_types::RowType;

/// Creates an Arrow Field from a RowType, embedding Snowflake-like metadata
pub fn create_field(row_type: &RowType) -> Field {
    use std::io::Write;
    match row_type {
        RowType::Fixed {
            name,
            nullable,
            precision,
            scale,
            original_type,
        } => {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rust_debug.log")
                .and_then(|mut f| writeln!(f, "DEBUG create_field: Fixed name={name}, precision={precision}, scale={scale}"));
            // If scale > 0, create Decimal128 type, not Int64
            if *scale > 0 {
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/rust_debug.log")
                    .and_then(|mut f| {
                        writeln!(
                            f,
                            "DEBUG: Creating Decimal128 for scale={scale}, precision={precision}"
                        )
                    });
                // Ensure precision is at least 1 (Arrow requires precision between 1 and 38)
                let precision_u8 = std::cmp::max(1, *precision as u8);
                let arrow_type = DataType::Decimal128(precision_u8, *scale as i8);
                let mut metadata = HashMap::new();
                metadata.insert("logicalType".to_string(), "FIXED".to_string());
                metadata.insert("scale".to_string(), scale.to_string());
                metadata.insert("precision".to_string(), precision.to_string());
                if let Some(orig) = original_type {
                    metadata.insert("snowflakeType".to_string(), orig.clone());
                }
                Field::new(name, arrow_type, *nullable).with_metadata(metadata)
            } else {
                // For scale = 0, always use Int64. If values don't fit at runtime,
                // create_column_array will fall back to String.
                tracing::debug!("Creating Int64 type for scale=0, precision={precision}");
                let arrow_type = DataType::Int64;
                let mut metadata = HashMap::new();
                metadata.insert("logicalType".to_string(), "FIXED".to_string());
                metadata.insert("scale".to_string(), scale.to_string());
                metadata.insert("precision".to_string(), precision.to_string());
                // Map precision to Snowflake physical storage type (SB1/SB2/SB4/SB8)
                metadata.insert(
                    "physicalType".to_string(),
                    physical_type_from_precision_signed(*precision),
                );
                if let Some(orig) = original_type {
                    metadata.insert("snowflakeType".to_string(), orig.clone());
                }
                Field::new(name, arrow_type, *nullable).with_metadata(metadata)
            }
        }
        RowType::Decimal {
            name,
            nullable,
            precision,
            scale,
        } => {
            // Ensure precision is at least 1 (Arrow requires precision between 1 and 38)
            let precision_u8 = std::cmp::max(1, *precision as u8);
            let arrow_type = DataType::Decimal128(precision_u8, *scale as i8);
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "DECIMAL".to_string());
            Field::new(name, arrow_type, *nullable).with_metadata(metadata)
        }
        RowType::Real { name, nullable } => Field::new(name, DataType::Float32, *nullable),
        RowType::Double { name, nullable } => Field::new(name, DataType::Float64, *nullable),
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
            // Snowflake reports TEXT physical type as LOB for rowset responses
            metadata.insert("physicalType".to_string(), "LOB".to_string());
            Field::new(name, DataType::Utf8, *nullable).with_metadata(metadata)
        }
        RowType::Binary {
            name,
            nullable,
            length,
        } => {
            let mut metadata = HashMap::new();
            metadata.insert("length".to_string(), length.to_string());
            Field::new(name, DataType::Binary, *nullable).with_metadata(metadata)
        }
        RowType::VarBinary {
            name,
            nullable,
            max_length,
        } => {
            let mut metadata = HashMap::new();
            metadata.insert("maxLength".to_string(), max_length.to_string());
            Field::new(name, DataType::Binary, *nullable).with_metadata(metadata)
        }
        RowType::Boolean { name, nullable } => Field::new(name, DataType::Boolean, *nullable),
        RowType::Date { name, nullable } => Field::new(name, DataType::Date32, *nullable),
        RowType::Time {
            name,
            nullable,
            precision,
        } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "TIME".to_string());
            metadata.insert("scale".to_string(), precision.to_string());
            Field::new(name, DataType::Time64(TimeUnit::Nanosecond), *nullable)
                .with_metadata(metadata)
        }
        RowType::TimestampNtz {
            name,
            nullable,
            precision,
        } => {
            let mut field_meta = HashMap::new();
            field_meta.insert("logicalType".to_string(), "TIMESTAMP_NTZ".to_string());
            field_meta.insert("scale".to_string(), precision.to_string());
            // Snowflake reports TIMESTAMP precision as 29 (YYYY-MM-DD HH:MM:SS.SSSSSSSSS)
            field_meta.insert("precision".to_string(), "29".to_string());
            field_meta.insert("charLength".to_string(), "29".to_string());
            field_meta.insert("byteLength".to_string(), "16".to_string());
            Field::new(
                name,
                DataType::Struct(timestamp_struct_fields("TIMESTAMP_NTZ", false).into()),
                *nullable,
            )
            .with_metadata(field_meta)
        }
        RowType::TimestampLtz {
            name,
            nullable,
            precision,
        } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "TIMESTAMP_LTZ".to_string());
            metadata.insert("scale".to_string(), precision.to_string());
            metadata.insert("precision".to_string(), "29".to_string());
            metadata.insert("charLength".to_string(), "29".to_string());
            metadata.insert("byteLength".to_string(), "16".to_string());
            Field::new(
                name,
                DataType::Struct(timestamp_struct_fields("TIMESTAMP_LTZ", false).into()),
                *nullable,
            )
            .with_metadata(metadata)
        }
        RowType::TimestampTz {
            name,
            nullable,
            precision,
        } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "TIMESTAMP_TZ".to_string());
            metadata.insert("scale".to_string(), precision.to_string());
            metadata.insert("precision".to_string(), "29".to_string());
            metadata.insert("charLength".to_string(), "29".to_string());
            metadata.insert("byteLength".to_string(), "16".to_string());
            Field::new(
                name,
                DataType::Struct(timestamp_struct_fields("TIMESTAMP_TZ", true).into()),
                *nullable,
            )
            .with_metadata(metadata)
        }
        RowType::Variant { name, nullable } => {
            // For now, store as UTF8 JSON string - will upgrade to Union later
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "VARIANT".to_string());
            Field::new(name, DataType::Utf8, *nullable).with_metadata(metadata)
        }
        RowType::Object { name, nullable } => {
            // For now, store as UTF8 JSON string - will upgrade to Map/Struct later
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "OBJECT".to_string());
            Field::new(name, DataType::Utf8, *nullable).with_metadata(metadata)
        }
        RowType::Array { name, nullable } => {
            // For now, store as UTF8 JSON string - will upgrade to List later
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "ARRAY".to_string());
            Field::new(name, DataType::Utf8, *nullable).with_metadata(metadata)
        }
        RowType::Geography { name, nullable } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "GEOGRAPHY".to_string());
            metadata.insert("extTypeName".to_string(), "GEOGRAPHY".to_string());
            Field::new(name, DataType::Utf8, *nullable).with_metadata(metadata)
        }
        RowType::Geometry { name, nullable } => {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), "GEOMETRY".to_string());
            metadata.insert("extTypeName".to_string(), "GEOMETRY".to_string());
            Field::new(name, DataType::Utf8, *nullable).with_metadata(metadata)
        }
    }
}

/// Creates an Arrow array from column values and data type
fn create_column_array(
    values: Vec<Option<&str>>,
    row_type: &RowType,
) -> Result<Arc<dyn Array>, ArrowUtilsError> {
    match row_type {
        RowType::Fixed {
            precision, scale, ..
        } => {
            if *scale > 0 {
                let mut builder = Decimal128Builder::with_capacity(values.len())
                    .with_precision_and_scale(*precision as u8, *scale as i8)
                    .context(ArrowSnafu)?;
                for opt in values {
                    if let Some(v) = opt {
                        let float_val = v.trim().parse::<f64>().context(FloatParsingSnafu {
                            value: v.to_string(),
                        })?;
                        let scale_factor = 10_i128.pow(*scale as u32);
                        builder.append_value((float_val * scale_factor as f64).round() as i128);
                    } else {
                        builder.append_null();
                    }
                }
                Ok(Arc::new(builder.finish()))
            } else {
                let mut parsed: Vec<Option<i64>> = Vec::with_capacity(values.len());
                let mut needs_string = false;

                for opt in &values {
                    if let Some(v) = opt {
                        match v.trim().parse::<i64>() {
                            Ok(i) => parsed.push(Some(i)),
                            Err(_) => {
                                needs_string = true;
                                break;
                            }
                        }
                    } else {
                        parsed.push(None);
                    }
                }

                if needs_string {
                    let string_values: Vec<Option<String>> = values
                        .into_iter()
                        .map(|opt| opt.map(|v| v.to_string()))
                        .collect();
                    Ok(Arc::new(StringArray::from(string_values)))
                } else {
                    Ok(Arc::new(Int64Array::from(parsed)))
                }
            }
        }
        RowType::Decimal {
            precision, scale, ..
        } => {
            let mut builder = Decimal128Builder::with_capacity(values.len())
                .with_precision_and_scale(*precision as u8, *scale as i8)
                .context(ArrowSnafu)?;
            for opt in values {
                if let Some(v) = opt {
                    let float_val = v.trim().parse::<f64>().context(FloatParsingSnafu {
                        value: v.to_string(),
                    })?;
                    let scale_factor = 10_i128.pow(*scale as u32);
                    builder.append_value((float_val * scale_factor as f64).round() as i128);
                } else {
                    builder.append_null();
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        RowType::Real { .. } => {
            let float_values: Result<Vec<Option<f32>>, _> = values
                .into_iter()
                .map(|opt| {
                    opt.map(|v| {
                        v.trim().parse::<f32>().context(FloatParsingSnafu {
                            value: v.to_string(),
                        })
                    })
                    .transpose()
                })
                .collect();
            Ok(Arc::new(Float32Array::from(float_values?)))
        }
        RowType::Double { .. } => {
            let float_values: Result<Vec<Option<f64>>, _> = values
                .into_iter()
                .map(|opt| {
                    opt.map(|v| {
                        v.trim().parse::<f64>().context(FloatParsingSnafu {
                            value: v.to_string(),
                        })
                    })
                    .transpose()
                })
                .collect();
            Ok(Arc::new(Float64Array::from(float_values?)))
        }
        RowType::Text { .. } => {
            let string_values: Vec<Option<String>> = values
                .into_iter()
                .map(|opt| opt.map(|v| v.to_string()))
                .collect();
            Ok(Arc::new(StringArray::from(string_values)))
        }
        RowType::Binary { .. } | RowType::VarBinary { .. } => {
            let decoded: Result<Vec<Option<Vec<u8>>>, ArrowUtilsError> = values
                .into_iter()
                .map(|opt| opt.map(|v| decode_binary_string(v)).transpose())
                .collect();
            let decoded = decoded?;
            let data_capacity: usize = decoded
                .iter()
                .filter_map(|opt| opt.as_ref().map(|buf| buf.len()))
                .sum();
            let mut builder = BinaryBuilder::with_capacity(decoded.len(), data_capacity);
            for opt in decoded {
                if let Some(buf) = opt {
                    builder.append_value(&buf);
                } else {
                    builder.append_null();
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        RowType::Boolean { .. } => {
            let bool_values: Result<Vec<Option<bool>>, _> = values
                .into_iter()
                .map(|opt| {
                    opt.map(|v| match v {
                        "1" | "true" | "True" | "TRUE" => Ok(true),
                        "0" | "false" | "False" | "FALSE" => Ok(false),
                        _ => v.parse::<bool>().context(BoolParsingSnafu {
                            value: v.to_string(),
                        }),
                    })
                    .transpose()
                })
                .collect();
            Ok(Arc::new(BooleanArray::from(bool_values?)))
        }
        RowType::Date { .. } => {
            let int_values: Result<Vec<Option<i32>>, _> = values
                .into_iter()
                .map(|opt| {
                    opt.map(|v| {
                        v.parse::<i32>().context(IntegerParsingSnafu {
                            value: v.to_string(),
                        })
                    })
                    .transpose()
                })
                .collect();
            Ok(Arc::new(Date32Array::from(int_values?)))
        }
        RowType::Time { .. } => {
            let int_values: Result<Vec<Option<i64>>, _> = values
                .into_iter()
                .map(|opt| {
                    opt.map(|v| match v.parse::<i64>() {
                        Ok(ns) => Ok(ns),
                        Err(_) => parse_time_string_to_nanos(&v),
                    })
                    .transpose()
                })
                .collect();
            Ok(Arc::new(Time64NanosecondArray::from(int_values?)))
        }
        RowType::TimestampNtz { precision, .. } => {
            let mut epoch_builder = Int64Builder::with_capacity(values.len());
            let mut fraction_builder = Int32Builder::with_capacity(values.len());

            for opt in values {
                if let Some(v) = opt {
                    let trimmed = v.trim();
                    let (secs, nanos) = if looks_like_decimal_timestamp(trimmed) {
                        parse_numeric_timestamp_to_parts(trimmed, *precision)?
                    } else {
                        parse_timestamp_string_to_parts(trimmed)?
                    };
                    epoch_builder.append_value(secs);
                    fraction_builder.append_value(nanos as i32);
                } else {
                    epoch_builder.append_null();
                    fraction_builder.append_null();
                }
            }

            let struct_array = StructArray::from(vec![
                (
                    Arc::new(Field::new("epoch", DataType::Int64, true)),
                    Arc::new(epoch_builder.finish()) as Arc<dyn Array>,
                ),
                (
                    Arc::new(Field::new("fraction", DataType::Int32, true)),
                    Arc::new(fraction_builder.finish()) as Arc<dyn Array>,
                ),
            ]);

            Ok(Arc::new(struct_array))
        }
        RowType::TimestampLtz { precision, .. } => {
            build_timestamp_struct_array(values, *precision as u8, false)
        }
        RowType::TimestampTz { precision, .. } => {
            build_timestamp_struct_array(values, *precision as u8, true)
        }
        RowType::Variant { .. } | RowType::Object { .. } | RowType::Array { .. } => {
            let string_values: Vec<Option<String>> = values
                .into_iter()
                .map(|opt| opt.map(|v| v.to_string()))
                .collect();
            Ok(Arc::new(StringArray::from(string_values)))
        }
        RowType::Geography { .. } | RowType::Geometry { .. } => {
            let string_values: Vec<Option<String>> = values
                .into_iter()
                .map(|opt| opt.map(|v| v.to_string()))
                .collect();
            Ok(Arc::new(StringArray::from(string_values)))
        }
    }
}

/// Converts a string rowset with RowType metadata to Arrow format
/// Supports TEXT and FIXED (with scale 0) types, converting strings to appropriate Arrow types
/// Assumes rowset and row_types have been validated to have matching column counts
pub fn convert_string_rowset_to_arrow_reader(
    rowset: &[Vec<Option<String>>],
    row_types: &[RowType],
) -> Result<Box<dyn arrow::record_batch::RecordBatchReader + Send>, ArrowUtilsError> {
    // Create Arrow arrays for each column
    let columns: Result<Vec<Arc<dyn Array>>, ArrowUtilsError> = row_types
        .iter()
        .enumerate()
        .map(|(col_idx, row_type)| {
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rust_debug.log")
            {
                let _ = writeln!(
                    file,
                    "convert_string_rowset_to_arrow_reader: column {} row_type {:?}",
                    col_idx + 1,
                    row_type
                );
            }
            let values: Vec<Option<&str>> =
                rowset.iter().map(|row| row[col_idx].as_deref()).collect();
            create_column_array(values, row_type)
        })
        .collect();

    let columns = columns?;

    // Build schema from actual array types and RowType metadata
    // This ensures schema matches the arrays we actually created
    let fields: Vec<Field> = row_types
        .iter()
        .zip(columns.iter())
        .map(|(row_type, array)| {
            let base_field = create_field(row_type);
            // If array type doesn't match expected type, use array's type
            if base_field.data_type() != array.data_type() {
                Field::new(
                    base_field.name(),
                    array.data_type().clone(),
                    base_field.is_nullable(),
                )
                .with_metadata(base_field.metadata().clone())
            } else {
                base_field
            }
        })
        .collect();

    let schema = Arc::new(Schema::new(fields));

    boxed_arrow_reader(schema, columns).context(ArrowSnafu)
}

/// Creates an Arrow Schema from a list of RowType definitions
pub fn create_schema(row_types: &[RowType]) -> Result<Arc<Schema>, ArrowUtilsError> {
    let fields: Vec<Field> = row_types.iter().map(create_field).collect();
    Ok(Arc::new(Schema::new(fields)))
}

/// Heuristic mapping from decimal precision (digits) to signed physical storage type
#[allow(dead_code)]
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

fn parse_time_string_to_nanos(value: &str) -> Result<i64, ArrowUtilsError> {
    if value.trim().is_empty() {
        return Ok(0);
    }
    if let Ok(ns) = value.trim().parse::<i128>() {
        return i64::try_from(ns).map_err(|_| ArrowUtilsError::ValueOutOfRange {
            location: Location::generate(),
        });
    }

    let nanos = parse_decimal_to_nanoseconds(value, false)?;
    i64::try_from(nanos).map_err(|_| ArrowUtilsError::ValueOutOfRange {
        location: Location::generate(),
    })
}

fn parse_timestamp_string_to_nanos(value: &str) -> Result<i64, ArrowUtilsError> {
    let (secs, nanos) = parse_timestamp_string_to_parts(value)?;
    secs.checked_mul(1_000_000_000)
        .and_then(|base| base.checked_add(nanos as i64))
        .ok_or_else(|| ArrowUtilsError::ValueOutOfRange {
            location: Location::generate(),
        })
}

fn parse_timestamp_string_to_parts(value: &str) -> Result<(i64, u32), ArrowUtilsError> {
    if value.trim().is_empty() {
        return Ok((0, 0));
    }

    if !value.contains('.') {
        let secs = value
            .trim()
            .parse::<i64>()
            .map_err(|e| ArrowUtilsError::IntegerParsing {
                value: value.to_string(),
                source: e,
                location: Location::generate(),
            })?;
        return Ok((secs, 0));
    }

    let nanos = parse_decimal_to_nanoseconds(value, false)?;
    split_nanos_parts(nanos)
}

fn looks_like_decimal_timestamp(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    let num_part = trimmed.split_whitespace().next().unwrap_or("");
    if num_part.is_empty() {
        return false;
    }
    let mut chars = num_part.chars();
    if let Some(first) = chars.next() {
        if !(first.is_ascii_digit() || first == '-' || first == '+') {
            return false;
        }
    }
    let mut seen_dot = false;
    chars.all(|c| {
        if c == '.' {
            if seen_dot {
                return false;
            }
            seen_dot = true;
            true
        } else {
            c.is_ascii_digit()
        }
    })
}

fn parse_numeric_timestamp_to_parts(
    value: &str,
    _scale: u8,
) -> Result<(i64, u32), ArrowUtilsError> {
    parse_timestamp_string_to_parts(value)
}

fn split_nanos_parts(total_nanos: i128) -> Result<(i64, u32), ArrowUtilsError> {
    const BILLION: i128 = 1_000_000_000;
    let secs = total_nanos.div_euclid(BILLION);
    let rem = total_nanos.rem_euclid(BILLION);
    let secs_i64 = i64::try_from(secs).map_err(|_| ArrowUtilsError::ValueOutOfRange {
        location: Location::generate(),
    })?;
    tracing::debug!(
        "split_nanos_parts total_nanos={} secs={} nanos={}",
        total_nanos,
        secs_i64,
        rem
    );
    Ok((secs_i64, rem as u32))
}

fn timestamp_struct_fields(logical_type: &str, include_timezone: bool) -> Vec<Field> {
    fn metadata_for(logical_type: &str, scale: &str) -> HashMap<String, String> {
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), logical_type.to_string());
        meta.insert("scale".to_string(), scale.to_string());
        meta.insert("precision".to_string(), "0".to_string());
        meta.insert("byteLength".to_string(), "16".to_string());
        meta.insert("finalType".to_string(), "T".to_string());
        meta.insert("charLength".to_string(), "0".to_string());
        meta
    }

    let epoch_field =
        Field::new("epoch", DataType::Int64, true).with_metadata(metadata_for(logical_type, "9"));

    let fraction_field = Field::new("fraction", DataType::Int32, true)
        .with_metadata(metadata_for(logical_type, "9"));

    let mut fields = vec![epoch_field, fraction_field];

    if include_timezone {
        let tz_field = Field::new("timezone", DataType::Int32, true)
            .with_metadata(metadata_for(logical_type, "0"));
        fields.push(tz_field);
    }

    fields
}

fn build_timestamp_struct_array(
    values: Vec<Option<&str>>,
    precision: u8,
    include_timezone: bool,
) -> Result<Arc<dyn Array>, ArrowUtilsError> {
    let mut epoch_builder = Int64Builder::with_capacity(values.len());
    let mut fraction_builder = Int32Builder::with_capacity(values.len());
    let mut timezone_builder = include_timezone.then(|| Int32Builder::with_capacity(values.len()));

    for opt in values {
        match opt {
            Some(raw) => {
                let (ts_part, tz_part) = if include_timezone {
                    let mut parts = raw.split_whitespace();
                    let timestamp_part = parts.next().unwrap_or("").trim();
                    let tz_part = parts.next().map(str::trim);
                    (timestamp_part, tz_part)
                } else {
                    (raw.trim(), None)
                };

                if ts_part.is_empty() {
                    epoch_builder.append_null();
                    fraction_builder.append_null();
                    if let Some(builder) = timezone_builder.as_mut() {
                        builder.append_null();
                    }
                    continue;
                }

                let (secs, nanos) = if looks_like_decimal_timestamp(ts_part) {
                    parse_numeric_timestamp_to_parts(ts_part, precision)?
                } else {
                    parse_timestamp_string_to_parts(ts_part)?
                };

                epoch_builder.append_value(secs);
                fraction_builder.append_value(nanos as i32);

                if let Some(builder) = timezone_builder.as_mut() {
                    if let Some(tz_str) = tz_part {
                        match tz_str.parse::<i32>() {
                            Ok(tz) => builder.append_value(tz),
                            Err(_) => builder.append_null(),
                        }
                    } else {
                        builder.append_null();
                    }
                }
            }
            None => {
                epoch_builder.append_null();
                fraction_builder.append_null();
                if let Some(builder) = timezone_builder.as_mut() {
                    builder.append_null();
                }
            }
        }
    }

    let mut fields: Vec<(Arc<Field>, Arc<dyn Array>)> = vec![
        (
            Arc::new(Field::new("epoch", DataType::Int64, true)),
            Arc::new(epoch_builder.finish()) as Arc<dyn Array>,
        ),
        (
            Arc::new(Field::new("fraction", DataType::Int32, true)),
            Arc::new(fraction_builder.finish()) as Arc<dyn Array>,
        ),
    ];

    if let Some(mut builder) = timezone_builder {
        fields.push((
            Arc::new(Field::new("timezone", DataType::Int32, true)),
            Arc::new(builder.finish()) as Arc<dyn Array>,
        ));
    }

    Ok(Arc::new(StructArray::from(fields)))
}

fn parse_decimal_to_nanoseconds(
    value: &str,
    treat_inserted_decimal: bool,
) -> Result<i128, ArrowUtilsError> {
    let mut trimmed = value.trim();
    let mut negative = false;
    if let Some(rest) = trimmed.strip_prefix('-') {
        negative = true;
        trimmed = rest;
    } else if let Some(rest) = trimmed.strip_prefix('+') {
        trimmed = rest;
    }

    if trimmed.is_empty() {
        return Ok(0);
    }

    let dot_idx = trimmed
        .find('.')
        .ok_or_else(|| ArrowUtilsError::InvalidDecimal {
            value: value.to_string(),
            location: Location::generate(),
        })?;
    let (int_part, frac_with_dot) = trimmed.split_at(dot_idx);
    let frac_part = &frac_with_dot[1..];

    let has_non_zero_fraction = frac_part.chars().any(|c| c != '0');
    let inserted = treat_inserted_decimal
        && has_non_zero_fraction
        && is_inserted_decimal_case(int_part, frac_part, trimmed);
    if inserted {
        let mut combined = String::with_capacity(int_part.len() + frac_part.len());
        combined.push_str(int_part);
        combined.push_str(frac_part);
        let secs = if combined.is_empty() {
            0
        } else {
            combined
                .parse::<i128>()
                .map_err(|e| ArrowUtilsError::IntegerParsing {
                    value: combined.clone(),
                    source: e,
                    location: Location::generate(),
                })?
        };
        let result = secs * 1_000_000_000i128;
        return Ok(if negative { -result } else { result });
    }

    let secs = if int_part.is_empty() {
        0
    } else {
        int_part
            .parse::<i128>()
            .map_err(|e| ArrowUtilsError::IntegerParsing {
                value: int_part.to_string(),
                source: e,
                location: Location::generate(),
            })?
    };
    let nanos = parse_fractional_to_nanos(frac_part)? as i128;
    let total = if negative {
        -(secs * 1_000_000_000i128) - nanos
    } else {
        secs * 1_000_000_000i128 + nanos
    };
    Ok(total)
}

fn parse_fractional_to_nanos(frac: &str) -> Result<u32, ArrowUtilsError> {
    if frac.is_empty() {
        return Ok(0);
    }
    let digits: String = frac.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return Ok(0);
    }
    let mut normalized = if digits.len() > 9 {
        digits[..9].to_string()
    } else {
        digits
    };
    while normalized.len() < 9 {
        normalized.push('0');
    }
    normalized
        .parse::<u32>()
        .map_err(|e| ArrowUtilsError::IntegerParsing {
            value: frac.to_string(),
            source: e,
            location: Location::generate(),
        })
}

fn is_inserted_decimal_case(int_part: &str, frac_part: &str, unsigned_value: &str) -> bool {
    if int_part.is_empty() || frac_part.is_empty() {
        return false;
    }
    let normalized = int_part.trim_start_matches('0');
    if normalized.is_empty() {
        return false;
    }
    normalized.len() == 1 && decimal_value_less_than_ten(unsigned_value)
}

fn decimal_value_less_than_ten(value: &str) -> bool {
    let mut trimmed = value.trim();
    if trimmed.is_empty() {
        return true;
    }
    if trimmed.starts_with('-') {
        return true;
    }
    if let Some(rest) = trimmed.strip_prefix('+') {
        trimmed = rest;
    }
    let trimmed = trimmed.trim_start_matches('0');
    if trimmed.is_empty() {
        return true;
    }
    if trimmed.starts_with('.') {
        return true;
    }
    let before_dot = trimmed.split('.').next().unwrap_or(trimmed);
    before_dot.len() <= 1
}

fn decode_binary_string(value: &str) -> Result<Vec<u8>, ArrowUtilsError> {
    match BASE64.decode(value) {
        Ok(bytes) => Ok(bytes),
        Err(_) => {
            let trimmed = value.trim();
            if trimmed.is_empty() || trimmed.len() % 2 != 0 {
                return Err(invalid_binary_error(value));
            }

            let mut bytes = Vec::with_capacity(trimmed.len() / 2);
            let mut iter = trimmed.as_bytes().chunks(2);
            while let Some(pair) = iter.next() {
                if pair.len() != 2 {
                    return Err(invalid_binary_error(value));
                }
                let hi = hex_value(pair[0]).ok_or_else(|| invalid_binary_error(value))?;
                let lo = hex_value(pair[1]).ok_or_else(|| invalid_binary_error(value))?;
                bytes.push((hi << 4) | lo);
            }
            Ok(bytes)
        }
    }
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn invalid_binary_error(value: &str) -> ArrowUtilsError {
    ArrowUtilsError::InvalidBinary {
        value: value.to_string(),
        location: Location::generate(),
    }
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
    BoolParsing {
        value: String,
        source: std::str::ParseBoolError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid decimal representation: {value}"))]
    InvalidDecimal {
        value: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Numeric value is out of range for 64-bit precision"))]
    ValueOutOfRange {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid binary literal: {value}"))]
    InvalidBinary {
        value: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Int64Array, StringArray};
    use arrow::record_batch::RecordBatchReader;

    fn wrap_rowset(rows: Vec<Vec<String>>) -> Vec<Vec<Option<String>>> {
        rows.into_iter()
            .map(|row| row.into_iter().map(Some).collect())
            .collect()
    }

    #[test]
    fn test_string_rowset_translation_with_metadata_small() {
        // Build a Snowflake-like rowset
        let rowset = wrap_rowset(vec![
            vec!["alpha.txt".to_string(), "7".to_string()], // SB1
            vec!["beta.md".to_string(), "123".to_string()], // SB2
            vec!["gamma.bin".to_string(), "32767".to_string()], // SB2
            vec!["delta.png".to_string(), "1024".to_string()], // SB2
        ]);

        // Describe columns via RowType
        let row_types = vec![
            RowType::text("col_text", false, 16, 64),
            RowType::fixed("col_fixed", false, 5, 0).unwrap(),
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
        assert_eq!(format!("{:?}", fields[1].data_type()), "Int64");
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
        let rowset = wrap_rowset(vec![
            vec!["alpha/report.csv".to_string(), "7".to_string()], // SB1
            vec!["beta/readme.md".to_string(), "123".to_string()], // SB2
            vec!["gamma/data.bin".to_string(), "32767".to_string()], // SB2
            vec!["delta/image.png".to_string(), "2147483647".to_string()], // SB4
            vec![
                "epsilon/archive.tar.gz".to_string(),
                "9223372036854775807".to_string(), // SB8
            ],
        ]);

        // Describe columns via RowType
        let row_types = vec![
            RowType::text("col_text", false, 64, 256),
            RowType::fixed("col_fixed", false, 19, 0).unwrap(),
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
    fn test_decimal_conversion() {
        let rowset = wrap_rowset(vec![
            vec!["123.45".to_string()],
            vec!["0.01".to_string()],
            vec!["-999.99".to_string()],
        ]);

        let row_types = vec![RowType::fixed("amount", false, 10, 2).unwrap()];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let schema = reader.schema();

        // Should be Decimal128 for scale > 0
        assert_eq!(
            format!("{:?}", schema.field(0).data_type()),
            "Decimal128(10, 2)"
        );

        if let Some(Ok(batch)) = reader.next() {
            let col = batch
                .column(0)
                .as_any()
                .downcast_ref::<Decimal128Array>()
                .unwrap();

            // 123.45 with scale 2 = 12345
            assert_eq!(col.value(0), 12345);
            // 0.01 with scale 2 = 1
            assert_eq!(col.value(1), 1);
            // -999.99 with scale 2 = -99999
            assert_eq!(col.value(2), -99999);
        } else {
            panic!("Expected one record batch");
        }
    }

    #[test]
    fn test_large_integer_fallback_to_string() {
        // Integer larger than i64::MAX
        let rowset = wrap_rowset(vec![
            vec!["12345".to_string()],
            vec!["99999999999999999999999999999999999999".to_string()], // > i64::MAX
        ]);

        let row_types = vec![RowType::fixed("big_num", false, 38, 0).unwrap()];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();
        let schema = reader.schema();

        // Should fall back to String when any value doesn't fit in i64
        assert_eq!(format!("{:?}", schema.field(0).data_type()), "Utf8");

        if let Some(Ok(batch)) = reader.next() {
            let col = batch
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();

            assert_eq!(col.value(0), "12345");
            assert_eq!(col.value(1), "99999999999999999999999999999999999999");
        } else {
            panic!("Expected one record batch");
        }
    }

    #[test]
    fn test_boolean_conversion() {
        let rowset = wrap_rowset(vec![
            vec!["1".to_string()],
            vec!["0".to_string()],
            vec!["1".to_string()],
        ]);

        let row_types = vec![RowType::boolean("flag", false)];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();

        if let Some(Ok(batch)) = reader.next() {
            let col = batch
                .column(0)
                .as_any()
                .downcast_ref::<BooleanArray>()
                .unwrap();

            assert_eq!(col.value(0), true);
            assert_eq!(col.value(1), false);
            assert_eq!(col.value(2), true);
        } else {
            panic!("Expected one record batch");
        }
    }

    #[test]
    fn test_empty_rowset() {
        let rowset: Vec<Vec<Option<String>>> = vec![];
        let row_types = vec![RowType::fixed("col", false, 10, 0).unwrap()];

        let mut reader = convert_string_rowset_to_arrow_reader(&rowset, &row_types).unwrap();

        if let Some(Ok(batch)) = reader.next() {
            assert_eq!(batch.num_rows(), 0);
            assert_eq!(batch.num_columns(), 1);
        } else {
            panic!("Expected one record batch");
        }
    }
}
