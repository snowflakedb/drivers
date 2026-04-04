use std::sync::Arc;

use arrow::array::{Array, RecordBatch, RecordBatchIterator, RecordBatchReader};
use arrow::datatypes::{
    DataType, Date32Type, Decimal128Type, Field, Fields, Int32Type, Int64Type, Schema,
};
use arrow::error::ArrowError;
use snafu::ResultExt;

use super::ChunkError;
use super::error::*;
use super::prefetch::ParseChunk;
use crate::arrow_utils::{create_field, create_field_with_type};
use crate::query_types::RowType;

#[derive(Clone)]
pub struct JsonChunkParser {
    pub row_types: Vec<RowType>,
}

/// Converts a string rowset with RowType metadata to an Arrow RecordBatchReader.
/// Null values in the rowset are preserved as Arrow nulls.
pub fn convert_string_rowset_to_arrow_reader(
    rowset: &[Vec<Option<String>>],
    row_types: &[RowType],
) -> Result<Box<dyn RecordBatchReader + Send>, ChunkError> {
    let capacity = rowset.len().max(64);
    let mut builders: Vec<ColumnBuilder> = row_types
        .iter()
        .map(|rt| ColumnBuilder::new(rt, capacity))
        .collect();

    for row in rowset {
        for (col_idx, cell) in row.iter().enumerate() {
            match cell {
                None => builders[col_idx].push_null(),
                Some(val) => builders[col_idx]
                    .push_value(val.as_bytes())
                    .context(ChunkReadingSnafu)?,
            }
        }
    }

    let batches = builders_to_batches(builders, row_types).context(ChunkReadingSnafu)?;

    let schema = if let Some(batch) = batches.first() {
        batch.schema()
    } else {
        Arc::new(Schema::new(Fields::empty()))
    };

    Ok(Box::new(RecordBatchIterator::new(
        batches.into_iter().map(Ok::<RecordBatch, ArrowError>),
        schema,
    )))
}

impl ParseChunk for JsonChunkParser {
    fn parse_chunk(&self, data: Vec<u8>) -> Result<Vec<RecordBatch>, ArrowError> {
        let expected_cols = self.row_types.len();
        if expected_cols == 0 || data.is_empty() {
            return Ok(empty_batch_from_row_types(&self.row_types)?
                .into_iter()
                .collect());
        }

        let capacity_hint = data.len() / (expected_cols * 8);
        let capacity_hint = capacity_hint.max(64);

        let mut builders: Vec<ColumnBuilder> = self
            .row_types
            .iter()
            .map(|rt| ColumnBuilder::new(rt, capacity_hint))
            .collect();

        let row_count = parse_json_rows(&data, &mut builders)?;

        if row_count == 0 {
            return Ok(empty_batch_from_row_types(&self.row_types)?
                .into_iter()
                .collect());
        }

        builders_to_batches(builders, &self.row_types)
    }
}

/// Scans JSON chunk bytes line-by-line, dispatching each cell directly to
/// column builders. Each line is a JSON array ending with `,\n`, e.g.
/// `["v1","v2",null],\n`. Uses `sonic_rs::to_array_iter` for SIMD-accelerated
/// lazy JSON parsing.
fn parse_json_rows(data: &[u8], builders: &mut [ColumnBuilder]) -> Result<usize, ArrowError> {
    let mut row_count: usize = 0;

    for line in data.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let line = line.strip_suffix(b",").unwrap_or(line);
        if line.is_empty() {
            continue;
        }

        scan_json_row(line, builders)?;
        row_count += 1;
    }

    Ok(row_count)
}

/// Parses a single JSON row `["v1","v2",null]` from raw bytes using sonic-rs
/// lazy array iteration. Each element is either a JSON string or `null`.
fn scan_json_row(line: &[u8], builders: &mut [ColumnBuilder]) -> Result<(), ArrowError> {
    use sonic_rs::JsonValueTrait;

    let expected_cols = builders.len();
    let mut col_idx = 0;
    for elem in sonic_rs::to_array_iter(line) {
        let value = elem.map_err(|e| ArrowError::ExternalError(Box::new(e)))?;
        if col_idx >= expected_cols {
            return Err(ArrowError::InvalidArgumentError(format!(
                "Row has more columns than expected ({expected_cols})",
            )));
        }
        if value.is_null() {
            builders[col_idx].push_null();
        } else if let Some(s) = value.as_str() {
            builders[col_idx].push_value(s.as_bytes())?;
        } else {
            return Err(ArrowError::InvalidArgumentError(format!(
                "Expected string or null at column {col_idx}",
            )));
        }
        col_idx += 1;
    }

    if col_idx < expected_cols {
        return Err(ArrowError::InvalidArgumentError(format!(
            "Expected {expected_cols} columns but got {col_idx}",
        )));
    }

    Ok(())
}

/// i64 fast-path storage for FIXED columns. Falls back to Vec<Option<i128>> only when
/// a value's digit count exceeds 18 (i.e., won't fit in i64).
pub(super) enum FixedStorage {
    /// All values so far fit in i64. Builder appends directly to Arrow buffer.
    I64 {
        builder: arrow::array::PrimitiveBuilder<Int64Type>,
    },
    /// At least one value exceeded i64 range.
    I128 {
        builder: arrow::array::PrimitiveBuilder<Decimal128Type>,
    },
}

pub(super) fn convert_i64_to_decimal128(
    builder: &mut arrow::array::PrimitiveBuilder<Int64Type>,
) -> arrow::array::PrimitiveBuilder<Decimal128Type> {
    let arr = builder.finish();
    let mut decimal_builder =
        arrow::array::PrimitiveBuilder::<Decimal128Type>::with_capacity(builder.capacity());
    for i in 0..arr.len() {
        if arr.is_null(i) {
            decimal_builder.append_null();
        } else {
            decimal_builder.append_value(arr.value(i) as i128);
        }
    }
    decimal_builder
}

pub(super) enum ColumnBuilder {
    Fixed {
        scale: u32,
        precision: u64,
        storage: FixedStorage,
    },
    Text {
        builder: arrow::array::StringBuilder,
    },
    Boolean {
        builder: arrow::array::BooleanBuilder,
    },
    Real {
        builder: arrow::array::PrimitiveBuilder<arrow::datatypes::Float64Type>,
    },
    Date {
        builder: arrow::array::PrimitiveBuilder<Date32Type>,
    },
    /// TimestampNtz/Ltz with scale <= 7: single Int64 value
    TimestampI64 {
        scale: u64,
        builder: arrow::array::PrimitiveBuilder<Int64Type>,
    },
    /// TimestampNtz/Ltz with scale > 7: struct(epoch, fraction)
    TimestampStruct {
        scale: u64,
        epoch_builder: arrow::array::PrimitiveBuilder<Int64Type>,
        frac_builder: arrow::array::PrimitiveBuilder<Int32Type>,
        nulls: arrow::array::BooleanBufferBuilder,
    },
    /// TimestampTz with scale <= 3: struct(combined_epoch, tz)
    TimestampTz2 {
        scale: u64,
        epoch_builder: arrow::array::PrimitiveBuilder<Int64Type>,
        tz_builder: arrow::array::PrimitiveBuilder<Int32Type>,
        nulls: arrow::array::BooleanBufferBuilder,
    },
    /// TimestampTz with scale > 3: struct(epoch, fraction, tz)
    TimestampTz3 {
        scale: u64,
        epoch_builder: arrow::array::PrimitiveBuilder<Int64Type>,
        frac_builder: arrow::array::PrimitiveBuilder<Int32Type>,
        tz_builder: arrow::array::PrimitiveBuilder<Int32Type>,
        nulls: arrow::array::BooleanBufferBuilder,
    },
    TimeI32 {
        scale: u64,
        builder: arrow::array::PrimitiveBuilder<Int32Type>,
    },
    TimeI64 {
        scale: u64,
        builder: arrow::array::PrimitiveBuilder<Int64Type>,
    },
    Binary {
        builder: arrow::array::BinaryBuilder,
    },
    Decfloat {
        exp_builder: arrow::array::PrimitiveBuilder<arrow::datatypes::Int16Type>,
        mant_builder: arrow::array::BinaryBuilder,
        nulls: arrow::array::BooleanBufferBuilder,
    },
}

impl ColumnBuilder {
    pub(super) fn new(row_type: &RowType, capacity: usize) -> Self {
        match row_type {
            RowType::Fixed {
                scale, precision, ..
            } => ColumnBuilder::Fixed {
                scale: *scale as u32,
                precision: *precision,
                storage: FixedStorage::I64 {
                    builder: arrow::array::PrimitiveBuilder::<Int64Type>::with_capacity(capacity),
                },
            },
            RowType::Text { .. }
            | RowType::Variant { .. }
            | RowType::Object { .. }
            | RowType::Array { .. } => ColumnBuilder::Text {
                builder: arrow::array::StringBuilder::with_capacity(capacity, capacity * 8),
            },
            RowType::Boolean { .. } => ColumnBuilder::Boolean {
                builder: arrow::array::BooleanBuilder::with_capacity(capacity),
            },
            RowType::Real { .. } => ColumnBuilder::Real {
                builder: arrow::array::PrimitiveBuilder::with_capacity(capacity),
            },
            RowType::Date { .. } => ColumnBuilder::Date {
                builder: arrow::array::PrimitiveBuilder::with_capacity(capacity),
            },
            RowType::TimestampNtz { scale, .. } | RowType::TimestampLtz { scale, .. } => {
                if *scale <= 7 {
                    ColumnBuilder::TimestampI64 {
                        scale: *scale,
                        builder: arrow::array::PrimitiveBuilder::with_capacity(capacity),
                    }
                } else {
                    ColumnBuilder::TimestampStruct {
                        scale: *scale,
                        epoch_builder: arrow::array::PrimitiveBuilder::with_capacity(capacity),
                        frac_builder: arrow::array::PrimitiveBuilder::with_capacity(capacity),
                        nulls: arrow::array::BooleanBufferBuilder::new(capacity),
                    }
                }
            }
            RowType::TimestampTz { scale, .. } => {
                if *scale <= 3 {
                    ColumnBuilder::TimestampTz2 {
                        scale: *scale,
                        epoch_builder: arrow::array::PrimitiveBuilder::with_capacity(capacity),
                        tz_builder: arrow::array::PrimitiveBuilder::with_capacity(capacity),
                        nulls: arrow::array::BooleanBufferBuilder::new(capacity),
                    }
                } else {
                    ColumnBuilder::TimestampTz3 {
                        scale: *scale,
                        epoch_builder: arrow::array::PrimitiveBuilder::with_capacity(capacity),
                        frac_builder: arrow::array::PrimitiveBuilder::with_capacity(capacity),
                        tz_builder: arrow::array::PrimitiveBuilder::with_capacity(capacity),
                        nulls: arrow::array::BooleanBufferBuilder::new(capacity),
                    }
                }
            }
            RowType::Time { scale, .. } => {
                if *scale <= 4 {
                    ColumnBuilder::TimeI32 {
                        scale: *scale,
                        builder: arrow::array::PrimitiveBuilder::with_capacity(capacity),
                    }
                } else {
                    ColumnBuilder::TimeI64 {
                        scale: *scale,
                        builder: arrow::array::PrimitiveBuilder::with_capacity(capacity),
                    }
                }
            }
            RowType::Binary { .. } => ColumnBuilder::Binary {
                builder: arrow::array::BinaryBuilder::with_capacity(capacity, capacity * 16),
            },
            RowType::Decfloat { .. } => ColumnBuilder::Decfloat {
                exp_builder: arrow::array::PrimitiveBuilder::with_capacity(capacity),
                mant_builder: arrow::array::BinaryBuilder::with_capacity(capacity, capacity * 8),
                nulls: arrow::array::BooleanBufferBuilder::new(capacity),
            },
        }
    }

    pub(super) fn push_null(&mut self) {
        match self {
            ColumnBuilder::Fixed { storage, .. } => match storage {
                FixedStorage::I64 { builder } => builder.append_null(),
                FixedStorage::I128 { builder } => builder.append_null(),
            },
            ColumnBuilder::Text { builder, .. } => builder.append_null(),
            ColumnBuilder::Boolean { builder } => builder.append_null(),
            ColumnBuilder::Real { builder } => builder.append_null(),
            ColumnBuilder::Date { builder } => builder.append_null(),
            ColumnBuilder::TimestampI64 { builder, .. } => builder.append_null(),
            ColumnBuilder::TimestampStruct {
                epoch_builder,
                frac_builder,
                nulls,
                ..
            } => {
                epoch_builder.append_value(0);
                frac_builder.append_value(0);
                nulls.append(false);
            }
            ColumnBuilder::TimestampTz2 {
                epoch_builder,
                tz_builder,
                nulls,
                ..
            } => {
                epoch_builder.append_value(0);
                tz_builder.append_value(0);
                nulls.append(false);
            }
            ColumnBuilder::TimestampTz3 {
                epoch_builder,
                frac_builder,
                tz_builder,
                nulls,
                ..
            } => {
                epoch_builder.append_value(0);
                frac_builder.append_value(0);
                tz_builder.append_value(0);
                nulls.append(false);
            }
            ColumnBuilder::TimeI32 { builder, .. } => builder.append_null(),
            ColumnBuilder::TimeI64 { builder, .. } => builder.append_null(),
            ColumnBuilder::Binary { builder } => builder.append_null(),
            ColumnBuilder::Decfloat {
                exp_builder,
                mant_builder,
                nulls,
            } => {
                exp_builder.append_value(0);
                mant_builder.append_value(&[] as &[u8]);
                nulls.append(false);
            }
        }
    }

    pub(super) fn push_value(&mut self, cell: &[u8]) -> Result<(), ArrowError> {
        match self {
            ColumnBuilder::Fixed { scale, storage, .. } => match storage {
                // Start with i64 for compactness. On the first value that overflows i64,
                // promote the entire column to Decimal128 and stay there for all
                // subsequent values in this chunk.
                FixedStorage::I64 { builder } => {
                    if cell.len() <= 18 {
                        let v = parse_i64_fixed_unchecked(cell, *scale);
                        builder.append_value(v);
                    } else {
                        let v128 = parse_i128_from_bytes(cell, *scale)?;
                        if let Ok(v) = i64::try_from(v128) {
                            builder.append_value(v);
                        } else {
                            let mut decimal_builder = convert_i64_to_decimal128(builder);
                            decimal_builder.append_value(v128);
                            *storage = FixedStorage::I128 {
                                builder: decimal_builder,
                            };
                        }
                    }
                }
                FixedStorage::I128 { builder } => {
                    let v = parse_i128_from_bytes(cell, *scale)?;
                    builder.append_value(v);
                }
            },
            ColumnBuilder::Text { builder, .. } => {
                let s = std::str::from_utf8(cell)
                    .map_err(|e| ArrowError::ExternalError(Box::new(e)))?;
                builder.append_value(s);
            }
            ColumnBuilder::Boolean { builder } => {
                let v = match cell {
                    b"true" | b"TRUE" | b"True" | b"1" => true,
                    b"false" | b"FALSE" | b"False" | b"0" => false,
                    _ => {
                        let s = String::from_utf8_lossy(cell);
                        if s.eq_ignore_ascii_case("true") {
                            true
                        } else if s.eq_ignore_ascii_case("false") {
                            false
                        } else {
                            return Err(ArrowError::InvalidArgumentError(format!(
                                "Cannot parse boolean: {s}",
                            )));
                        }
                    }
                };
                builder.append_value(v);
            }
            ColumnBuilder::Real { builder } => {
                let s = std::str::from_utf8(cell)
                    .map_err(|e| ArrowError::ExternalError(Box::new(e)))?;
                let v: f64 = s.parse().map_err(|e: std::num::ParseFloatError| {
                    ArrowError::ExternalError(Box::new(e))
                })?;
                builder.append_value(v);
            }
            ColumnBuilder::Date { builder } => {
                let v = parse_i32_from_bytes(cell)?;
                builder.append_value(v);
            }
            ColumnBuilder::TimestampI64 { scale, builder } => {
                let (epoch, frac) = parse_epoch_fraction(cell, *scale)?;
                builder.append_value(epoch * 10i64.pow(*scale as u32) + frac as i64);
            }
            ColumnBuilder::TimestampStruct {
                scale,
                epoch_builder,
                frac_builder,
                nulls,
            } => {
                let (epoch, frac) = parse_epoch_fraction(cell, *scale)?;
                epoch_builder.append_value(epoch);
                frac_builder.append_value(frac * 10i32.pow((9 - *scale) as u32));
                nulls.append(true);
            }
            ColumnBuilder::TimestampTz2 {
                scale,
                epoch_builder,
                tz_builder,
                nulls,
            } => {
                let (epoch_part, tz) = split_tz(cell)?;
                let (epoch, frac) = parse_epoch_fraction(epoch_part, *scale)?;
                epoch_builder.append_value(epoch * 10i64.pow(*scale as u32) + frac as i64);
                tz_builder.append_value(tz);
                nulls.append(true);
            }
            ColumnBuilder::TimestampTz3 {
                scale,
                epoch_builder,
                frac_builder,
                tz_builder,
                nulls,
            } => {
                let (epoch_part, tz) = split_tz(cell)?;
                let (epoch, frac) = parse_epoch_fraction(epoch_part, *scale)?;
                epoch_builder.append_value(epoch);
                frac_builder.append_value(frac * 10i32.pow((9 - *scale) as u32));
                tz_builder.append_value(tz);
                nulls.append(true);
            }
            ColumnBuilder::TimeI32 { scale, builder } => {
                let (secs, frac) = parse_epoch_fraction(cell, *scale)?;
                builder.append_value(secs as i32 * 10i32.pow(*scale as u32) + frac);
            }
            ColumnBuilder::TimeI64 { scale, builder } => {
                let (secs, frac) = parse_epoch_fraction(cell, *scale)?;
                builder.append_value(secs * 10i64.pow(*scale as u32) + frac as i64);
            }
            ColumnBuilder::Binary { builder } => {
                let s = std::str::from_utf8(cell)
                    .map_err(|e| ArrowError::ExternalError(Box::new(e)))?;
                let v = hex::decode(s).map_err(|e| ArrowError::ExternalError(Box::new(e)))?;
                builder.append_value(v);
            }
            ColumnBuilder::Decfloat {
                exp_builder,
                mant_builder,
                nulls,
            } => {
                let s = std::str::from_utf8(cell)
                    .map_err(|e| ArrowError::ExternalError(Box::new(e)))?;
                let (exp, mantissa) = parse_decfloat_str(s)?;
                exp_builder.append_value(exp);
                mant_builder.append_value(mantissa);
                nulls.append(true);
            }
        }
        Ok(())
    }

    pub(super) fn into_field_and_array(
        self,
        row_type: &RowType,
    ) -> Result<(Field, Arc<dyn Array>), ArrowError> {
        let err = |e: crate::arrow_utils::ArrowUtilsError| ArrowError::ExternalError(Box::new(e));
        match self {
            ColumnBuilder::Fixed {
                storage,
                precision,
                scale,
            } => match storage {
                FixedStorage::I64 { mut builder, .. } => {
                    let arr = builder.finish();
                    Ok((
                        create_field_with_type(row_type, Some(DataType::Int64)).map_err(err)?,
                        Arc::new(arr),
                    ))
                }
                FixedStorage::I128 { mut builder } => {
                    let arr = builder
                        .finish()
                        .with_precision_and_scale(precision as u8, scale as i8)
                        .map_err(|e| ArrowError::ExternalError(Box::new(e)))?;
                    Ok((
                        create_field_with_type(
                            row_type,
                            Some(DataType::Decimal128(precision as u8, scale as i8)),
                        )
                        .map_err(err)?,
                        Arc::new(arr),
                    ))
                }
            },
            ColumnBuilder::Text { mut builder, .. } => Ok((
                create_field(row_type).map_err(err)?,
                Arc::new(builder.finish()),
            )),
            ColumnBuilder::Boolean { mut builder } => Ok((
                create_field(row_type).map_err(err)?,
                Arc::new(builder.finish()),
            )),
            ColumnBuilder::Real { mut builder } => Ok((
                create_field(row_type).map_err(err)?,
                Arc::new(builder.finish()),
            )),
            ColumnBuilder::Date { mut builder } => Ok((
                create_field(row_type).map_err(err)?,
                Arc::new(builder.finish()),
            )),
            ColumnBuilder::TimestampI64 { mut builder, .. } => Ok((
                create_field(row_type).map_err(err)?,
                Arc::new(builder.finish()),
            )),
            ColumnBuilder::TimestampStruct {
                mut epoch_builder,
                mut frac_builder,
                mut nulls,
                ..
            } => {
                let field = create_field(row_type).map_err(err)?;
                let struct_fields = match field.data_type() {
                    DataType::Struct(f) => f.clone(),
                    dt => {
                        return Err(ArrowError::InvalidArgumentError(format!(
                            "Unsupported data type {dt:?} for timestamp struct"
                        )));
                    }
                };
                let epoch_arr: Arc<dyn Array> = Arc::new(epoch_builder.finish());
                let frac_arr: Arc<dyn Array> = Arc::new(frac_builder.finish());
                let null_buf = arrow::buffer::NullBuffer::new(nulls.finish());
                let struct_arr = arrow::array::StructArray::new(
                    struct_fields,
                    vec![epoch_arr, frac_arr],
                    Some(null_buf),
                );
                Ok((field, Arc::new(struct_arr)))
            }
            ColumnBuilder::TimestampTz2 {
                mut epoch_builder,
                mut tz_builder,
                mut nulls,
                ..
            } => {
                let field = create_field(row_type).map_err(err)?;
                let struct_fields = match field.data_type() {
                    DataType::Struct(f) => f.clone(),
                    dt => {
                        return Err(ArrowError::InvalidArgumentError(format!(
                            "Unsupported data type {dt:?} for timestamp_tz"
                        )));
                    }
                };
                let epoch_arr: Arc<dyn Array> = Arc::new(epoch_builder.finish());
                let tz_arr: Arc<dyn Array> = Arc::new(tz_builder.finish());
                let null_buf = arrow::buffer::NullBuffer::new(nulls.finish());
                let struct_arr = arrow::array::StructArray::new(
                    struct_fields,
                    vec![epoch_arr, tz_arr],
                    Some(null_buf),
                );
                Ok((field, Arc::new(struct_arr)))
            }
            ColumnBuilder::TimestampTz3 {
                mut epoch_builder,
                mut frac_builder,
                mut tz_builder,
                mut nulls,
                ..
            } => {
                let field = create_field(row_type).map_err(err)?;
                let struct_fields = match field.data_type() {
                    DataType::Struct(f) => f.clone(),
                    dt => {
                        return Err(ArrowError::InvalidArgumentError(format!(
                            "Unsupported data type {dt:?} for timestamp_tz"
                        )));
                    }
                };
                let epoch_arr: Arc<dyn Array> = Arc::new(epoch_builder.finish());
                let frac_arr: Arc<dyn Array> = Arc::new(frac_builder.finish());
                let tz_arr: Arc<dyn Array> = Arc::new(tz_builder.finish());
                let null_buf = arrow::buffer::NullBuffer::new(nulls.finish());
                let struct_arr = arrow::array::StructArray::new(
                    struct_fields,
                    vec![epoch_arr, frac_arr, tz_arr],
                    Some(null_buf),
                );
                Ok((field, Arc::new(struct_arr)))
            }
            ColumnBuilder::TimeI32 { mut builder, .. } => Ok((
                create_field(row_type).map_err(err)?,
                Arc::new(builder.finish()),
            )),
            ColumnBuilder::TimeI64 { mut builder, .. } => Ok((
                create_field(row_type).map_err(err)?,
                Arc::new(builder.finish()),
            )),
            ColumnBuilder::Binary { mut builder } => Ok((
                create_field(row_type).map_err(err)?,
                Arc::new(builder.finish()),
            )),
            ColumnBuilder::Decfloat {
                mut exp_builder,
                mut mant_builder,
                mut nulls,
            } => {
                let field = create_field(row_type).map_err(err)?;
                let struct_fields = match field.data_type() {
                    DataType::Struct(f) => f.clone(),
                    dt => {
                        return Err(ArrowError::InvalidArgumentError(format!(
                            "Unsupported data type {dt:?} for decfloat"
                        )));
                    }
                };
                let exp_arr: Arc<dyn Array> = Arc::new(exp_builder.finish());
                let mant_arr: Arc<dyn Array> = Arc::new(mant_builder.finish());
                let null_buf = arrow::buffer::NullBuffer::new(nulls.finish());
                let struct_arr = arrow::array::StructArray::new(
                    struct_fields,
                    vec![exp_arr, mant_arr],
                    Some(null_buf),
                );
                Ok((field, Arc::new(struct_arr)))
            }
        }
    }
}

pub(super) fn builders_to_batches(
    builders: Vec<ColumnBuilder>,
    row_types: &[RowType],
) -> Result<Vec<RecordBatch>, ArrowError> {
    let (fields, arrays): (Vec<_>, Vec<_>) = builders
        .into_iter()
        .zip(row_types.iter())
        .map(|(builder, row_type)| builder.into_field_and_array(row_type))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .unzip();

    let schema = Arc::new(Schema::new(fields));
    let batch = RecordBatch::try_new(schema, arrays)?;
    Ok(vec![batch])
}

fn empty_batch_from_row_types(row_types: &[RowType]) -> Result<Vec<RecordBatch>, ArrowError> {
    let err = |e: crate::arrow_utils::ArrowUtilsError| ArrowError::ExternalError(Box::new(e));
    let fields: Vec<Field> = row_types
        .iter()
        .map(|rt| create_field(rt).map_err(err))
        .collect::<Result<Vec<_>, _>>()?;
    let schema = Arc::new(Schema::new(fields));
    let batch = RecordBatch::new_empty(schema);
    Ok(vec![batch])
}

/// Parses a DECFLOAT string (decimal or scientific notation) into (exponent, mantissa_bytes).
/// The mantissa is normalized by stripping trailing zeros, and the mantissa bytes are
/// minimal big-endian two's complement.
fn parse_decfloat_str(v: &str) -> Result<(i16, Vec<u8>), ArrowError> {
    if v == "0" || v == "0.0" || v == "-0" || v == "-0.0" {
        return Ok((0_i16, vec![0]));
    }

    let lowered = v.to_lowercase();
    let (coeff_str, exp_offset) = match lowered.split_once('e') {
        Some((c, e)) => {
            let exp: i16 = e
                .parse()
                .map_err(|e: std::num::ParseIntError| ArrowError::ExternalError(Box::new(e)))?;
            (c, exp)
        }
        None => (v, 0_i16),
    };

    let negative = coeff_str.starts_with('-');
    let abs_coeff = coeff_str.trim_start_matches('-');

    let (int_part, frac_part) = match abs_coeff.split_once('.') {
        Some((i, f)) => (i, f),
        None => (abs_coeff, ""),
    };

    let digits = format!("{int_part}{frac_part}");
    let frac_len = frac_part.len() as i16;

    let mut mantissa: i128 = digits
        .parse()
        .map_err(|e: std::num::ParseIntError| ArrowError::ExternalError(Box::new(e)))?;
    let mut exponent: i16 = exp_offset - frac_len;

    if negative {
        mantissa = -mantissa;
    }

    if mantissa != 0 {
        while mantissa % 10 == 0 {
            mantissa /= 10;
            exponent += 1;
        }
    }

    let bytes = i128::to_be_bytes(mantissa);
    let mantissa_bytes = minimal_twos_complement(&bytes);

    Ok((exponent, mantissa_bytes))
}

/// Strips leading redundant bytes from a big-endian two's complement representation,
/// keeping the sign bit intact.
fn minimal_twos_complement(bytes: &[u8]) -> Vec<u8> {
    if bytes.is_empty() {
        return vec![0];
    }

    let is_negative = bytes[0] & 0x80 != 0;
    let pad_byte: u8 = if is_negative { 0xFF } else { 0x00 };

    let mut start = 0;
    while start < bytes.len() - 1 && bytes[start] == pad_byte {
        let next_sign = bytes[start + 1] & 0x80 != 0;
        if next_sign != is_negative {
            break;
        }
        start += 1;
    }

    bytes[start..].to_vec()
}

/// Fast i64 parser for FIXED values. Caller guarantees the value fits in i64
/// (cell.len() <= 18, so at most 18 digit bytes).
#[inline]
fn parse_i64_fixed_unchecked(bytes: &[u8], scale: u32) -> i64 {
    if scale == 0 {
        let (negative, digits) = if !bytes.is_empty() && bytes[0] == b'-' {
            (true, &bytes[1..])
        } else {
            (false, bytes)
        };
        let mut result: i64 = 0;
        for &b in digits {
            result = result * 10 + (b - b'0') as i64;
        }
        if negative { -result } else { result }
    } else {
        let dot_pos = memchr::memchr(b'.', bytes);
        let (int_part, frac_part) = match dot_pos {
            Some(pos) => (&bytes[..pos], &bytes[pos + 1..]),
            None => (bytes, &[] as &[u8]),
        };
        let negative = !int_part.is_empty() && int_part[0] == b'-';
        let digits = if negative { &int_part[1..] } else { int_part };

        let mut abs_int: i64 = 0;
        for &b in digits {
            abs_int = abs_int * 10 + (b - b'0') as i64;
        }

        let frac_scaled: i64 = if frac_part.is_empty() {
            0
        } else {
            let scale_usize = scale as usize;
            let mut frac: i64 = 0;
            let take = frac_part.len().min(scale_usize);
            for &b in &frac_part[..take] {
                frac = frac * 10 + (b - b'0') as i64;
            }
            if frac_part.len() < scale_usize {
                for _ in 0..(scale_usize - frac_part.len()) {
                    frac *= 10;
                }
            }
            frac
        };

        let unscaled = abs_int * 10i64.pow(scale) + frac_scaled;
        if negative { -unscaled } else { unscaled }
    }
}

#[inline]
fn parse_i128_from_bytes(bytes: &[u8], scale: u32) -> Result<i128, ArrowError> {
    if scale == 0 {
        return parse_int_bytes::<i128>(bytes);
    }
    let (int_part, frac_part) = match memchr::memchr(b'.', bytes) {
        Some(pos) => (&bytes[..pos], &bytes[pos + 1..]),
        None => (bytes, &[] as &[u8]),
    };
    let negative = !int_part.is_empty() && int_part[0] == b'-';
    let digits = if negative { &int_part[1..] } else { int_part };
    let abs_int: i128 = parse_uint_bytes(digits)?;

    let frac_scaled: i128 = if frac_part.is_empty() {
        0
    } else {
        let scale_usize = scale as usize;
        if frac_part.len() >= scale_usize {
            parse_uint_bytes(&frac_part[..scale_usize])?
        } else {
            let base: i128 = parse_uint_bytes(frac_part)?;
            base * 10i128.pow((scale_usize - frac_part.len()) as u32)
        }
    };

    let unscaled = abs_int * 10i128.pow(scale) + frac_scaled;
    Ok(if negative { -unscaled } else { unscaled })
}

#[inline]
fn parse_int_bytes<T>(bytes: &[u8]) -> Result<T, ArrowError>
where
    T: From<u8>
        + std::ops::Mul<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Neg<Output = T>
        + Copy,
{
    if bytes.is_empty() {
        return Err(ArrowError::InvalidArgumentError(
            "empty numeric value".into(),
        ));
    }
    let (negative, digits) = if bytes[0] == b'-' {
        (true, &bytes[1..])
    } else {
        (false, bytes)
    };
    let v: T = parse_uint_bytes(digits)?;
    Ok(if negative { -v } else { v })
}

#[inline]
fn parse_uint_bytes<T>(bytes: &[u8]) -> Result<T, ArrowError>
where
    T: From<u8> + std::ops::Mul<Output = T> + std::ops::Add<Output = T> + Copy,
{
    let ten = T::from(10u8);
    let mut result = T::from(0u8);
    for &b in bytes {
        if !b.is_ascii_digit() {
            return Err(ArrowError::InvalidArgumentError(format!(
                "invalid digit in numeric value: {:?}",
                String::from_utf8_lossy(bytes),
            )));
        }
        result = result * ten + T::from(b - b'0');
    }
    Ok(result)
}

#[inline]
fn parse_i32_from_bytes(bytes: &[u8]) -> Result<i32, ArrowError> {
    parse_int_bytes::<i32>(bytes)
}

#[inline]
fn split_tz(cell: &[u8]) -> Result<(&[u8], i32), ArrowError> {
    match memchr::memchr(b' ', cell) {
        Some(pos) => {
            let tz = parse_i32_from_bytes(&cell[pos + 1..])?;
            Ok((&cell[..pos], tz))
        }
        None => Ok((cell, 1440)),
    }
}

fn parse_epoch_fraction(bytes: &[u8], scale: u64) -> Result<(i64, i32), ArrowError> {
    let (epoch_part, frac_part) = match memchr::memchr(b'.', bytes) {
        Some(pos) => (&bytes[..pos], &bytes[pos + 1..]),
        None => (bytes, &[] as &[u8]),
    };
    let epoch: i64 = parse_int_bytes(epoch_part)?;
    let fraction: i32 = if frac_part.is_empty() {
        0
    } else {
        let scale_usize = scale as usize;
        if frac_part.len() >= scale_usize {
            parse_uint_bytes(&frac_part[..scale_usize])?
        } else {
            let base: i32 = parse_uint_bytes(frac_part)?;
            base * 10i32.pow((scale_usize - frac_part.len()) as u32)
        }
    };
    Ok((epoch, fraction))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Array, BooleanArray, Float64Array, Int32Array, Int64Array, StringArray};

    fn parse(row_types: Vec<RowType>, data: &[u8]) -> Vec<RecordBatch> {
        let parser = JsonChunkParser { row_types };
        parser
            .parse_chunk(data.to_vec())
            .expect("parse_chunk failed")
    }

    #[test]
    fn fixed_integers_scale0() {
        let rt = vec![RowType::fixed("n", true, 10, 0)];
        let data = b"[\"42\"],\n[\"-7\"],\n[\"0\"],\n[null],\n";
        let batches = parse(rt, data);
        assert_eq!(batches.len(), 1);
        let col = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        assert_eq!(col.len(), 4);
        assert_eq!(col.value(0), 42);
        assert_eq!(col.value(1), -7);
        assert_eq!(col.value(2), 0);
        assert!(col.is_null(3));
    }

    #[test]
    fn fixed_decimals_scale2() {
        let rt = vec![RowType::fixed("d", true, 10, 2)];
        let data = b"[\"3.14\"],\n[\"-0.01\"],\n[null],\n";
        let batches = parse(rt, data);
        let col = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        assert_eq!(col.value(0), 314);
        assert_eq!(col.value(1), -1);
        assert!(col.is_null(2));
    }

    #[test]
    fn fixed_i64_to_i128_promotion() {
        let rt = vec![RowType::fixed("big", true, 38, 0)];
        let data = b"[\"5\"],\n[\"99999999999999999999\"],\n[null],\n";
        let batches = parse(rt, data);
        let col = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<arrow::array::Decimal128Array>()
            .unwrap();
        assert_eq!(col.len(), 3);
        assert_eq!(col.value(0), 5);
        assert_eq!(col.value(1), 99999999999999999999i128);
        assert!(col.is_null(2));
    }

    #[test]
    fn text_values_and_nulls() {
        let rt = vec![RowType::text("t", true, 64, 256)];
        let data = b"[\"hello\"],\n[null],\n[\"world\"],\n";
        let batches = parse(rt, data);
        let col = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(col.value(0), "hello");
        assert!(col.is_null(1));
        assert_eq!(col.value(2), "world");
    }

    #[test]
    fn boolean_values() {
        let rt = vec![RowType::boolean("b", true)];
        let data = b"[\"true\"],\n[\"false\"],\n[\"1\"],\n[\"0\"],\n[null],\n";
        let batches = parse(rt, data);
        let col = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<BooleanArray>()
            .unwrap();
        assert!(col.value(0));
        assert!(!col.value(1));
        assert!(col.value(2));
        assert!(!col.value(3));
        assert!(col.is_null(4));
    }

    #[test]
    fn real_values() {
        let rt = vec![RowType::real("r", true)];
        let data = b"[\"1.5\"],\n[\"-2.25\"],\n[null],\n";
        let batches = parse(rt, data);
        let col = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap();
        assert!((col.value(0) - 1.5).abs() < f64::EPSILON);
        assert!((col.value(1) + 2.25).abs() < f64::EPSILON);
        assert!(col.is_null(2));
    }

    #[test]
    fn date_values() {
        let rt = vec![RowType::date("d", true)];
        let data = b"[\"19000\"],\n[null],\n[\"0\"],\n";
        let batches = parse(rt, data);
        let col = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<arrow::array::PrimitiveArray<arrow::datatypes::Date32Type>>()
            .unwrap();
        assert_eq!(col.value(0), 19000);
        assert!(col.is_null(1));
        assert_eq!(col.value(2), 0);
    }

    #[test]
    fn timestamp_i64_scale3() {
        let rt = vec![RowType::timestamp_ntz("ts", true, 3)];
        let data = b"[\"1609459200.123\"],\n[null],\n";
        let batches = parse(rt, data);
        let col = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        assert_eq!(col.value(0), 1609459200 * 1000 + 123);
        assert!(col.is_null(1));
    }

    #[test]
    fn timestamp_tz2_scale3() {
        let rt = vec![RowType::timestamp_tz("ts", true, 3)];
        let data = b"[\"1609459200.123 1500\"],\n[null],\n";
        let batches = parse(rt, data);
        let col = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<arrow::array::StructArray>()
            .unwrap();
        assert_eq!(col.len(), 2);
        assert!(!col.is_null(0));
        assert!(col.is_null(1));
        let epoch = col.column(0).as_any().downcast_ref::<Int64Array>().unwrap();
        assert_eq!(epoch.value(0), 1609459200 * 1000 + 123);
        let tz = col.column(1).as_any().downcast_ref::<Int32Array>().unwrap();
        assert_eq!(tz.value(0), 1500);
    }

    #[test]
    fn empty_data_returns_empty_batch() {
        let rt = vec![RowType::text("c", false, 16, 64)];
        let batches = parse(rt, b"");
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].num_rows(), 0);
    }

    #[test]
    fn column_count_mismatch_returns_error() {
        let rt = vec![
            RowType::text("a", false, 16, 64),
            RowType::text("b", false, 16, 64),
        ];
        let parser = JsonChunkParser { row_types: rt };
        let result = parser.parse_chunk(b"[\"only_one\"],\n".to_vec());
        assert!(result.is_err());
    }

    #[test]
    fn multi_row_multi_column() {
        let rt = vec![
            RowType::text("name", false, 64, 256),
            RowType::fixed("age", true, 5, 0),
        ];
        let data = b"[\"alice\",\"30\"],\n[\"bob\",null],\n[\"carol\",\"25\"],\n";
        let batches = parse(rt, data);
        assert_eq!(batches[0].num_rows(), 3);
        let names = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(names.value(0), "alice");
        assert_eq!(names.value(1), "bob");
        assert_eq!(names.value(2), "carol");
        let ages = batches[0]
            .column(1)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        assert_eq!(ages.value(0), 30);
        assert!(ages.is_null(1));
        assert_eq!(ages.value(2), 25);
    }

    #[test]
    fn binary_hex_values() {
        let rt = vec![RowType::binary("bin", true, 16, 16)];
        let data = b"[\"48656C6C6F\"],\n[null],\n";
        let batches = parse(rt, data);
        let col = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<arrow::array::BinaryArray>()
            .unwrap();
        assert_eq!(col.value(0), b"Hello");
        assert!(col.is_null(1));
    }
}
