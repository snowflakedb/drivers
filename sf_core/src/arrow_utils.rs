use crate::query_types::RowType;
use arrow::array::{
    Array, BinaryArray, BooleanArray, Float64Array, Int8Array, Int16Array, Int32Array, Int64Array,
    StringArray,
};
use arrow::datatypes::{DataType, Date32Type, Field, Int32Type, Int64Type, Schema};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use hex::FromHexError;
use snafu::{Location, ResultExt, Snafu};
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

/// Parses a DECFLOAT string (decimal or scientific notation) into (exponent, mantissa_bytes).
/// The mantissa is normalized by stripping trailing zeros, and the mantissa bytes are
/// minimal big-endian two's complement.
fn parse_decfloat_str(v: &str) -> Result<(i16, Vec<u8>), ArrowUtilsError> {
    if v == "0" || v == "0.0" || v == "-0" || v == "-0.0" {
        return Ok((0_i16, vec![0]));
    }

    // Parse into integer mantissa (i128) and exponent (i16).
    // Handle scientific notation: "1.23e5" or "1.23E-5"
    let lowered = v.to_lowercase();
    let (coeff_str, exp_offset) = match lowered.split_once('e') {
        Some((c, e)) => {
            let exp: i16 = e.parse().context(DecfloatParsingSnafu {
                value: v.to_string(),
            })?;
            (c, exp)
        }
        None => (v, 0_i16),
    };

    let negative = coeff_str.starts_with('-');
    let abs_coeff = coeff_str.trim_start_matches('-');

    // Split on decimal point
    let (int_part, frac_part) = match abs_coeff.split_once('.') {
        Some((i, f)) => (i, f),
        None => (abs_coeff, ""),
    };

    // Combine into integer mantissa: "123.456" -> 123456, with implicit exponent = -3
    let digits = format!("{int_part}{frac_part}");
    let frac_len = frac_part.len() as i16;

    let mut mantissa: i128 = digits.parse().context(DecfloatParsingSnafu {
        value: v.to_string(),
    })?;
    let mut exponent: i16 = exp_offset - frac_len;

    if negative {
        mantissa = -mantissa;
    }

    // Strip trailing zeros from mantissa, adjust exponent (Snowflake normalization)
    if mantissa != 0 {
        while mantissa % 10 == 0 {
            mantissa /= 10;
            exponent += 1;
        }
    }

    // Convert mantissa i128 to minimal big-endian two's complement bytes
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
        // Keep this byte if removing it would change the sign
        let next_sign = bytes[start + 1] & 0x80 != 0;
        if next_sign != is_negative {
            break;
        }
        start += 1;
    }

    bytes[start..].to_vec()
}

/// Creates an Arrow array from column values and data type
fn create_column_array(
    values: Vec<&str>,
    row_type: &RowType,
) -> Result<(Field, Arc<dyn Array>), ArrowUtilsError> {
    match row_type {
        RowType::Text { .. } => Ok((create_field(row_type)?, Arc::new(StringArray::from(values)))),
        RowType::Fixed {
            scale, precision, ..
        } => {
            let decimal_values: Result<Vec<i128>, ArrowUtilsError> = values
                .into_iter()
                .map(|v| parse_decimal_str(v, *scale as u32))
                .collect();

            let decimal_values = decimal_values?;
            if decimal_values.is_empty() {
                return Ok((
                    create_field_with_type(row_type, Some(DataType::Int64))?, // TODO is it correct? We have to assume something, but it probably doesn't matter.
                    Arc::new(Int64Array::new_null(0)),
                ));
            }
            let min_value: i128 = decimal_values.iter().min().copied().unwrap();
            let max_value: i128 = decimal_values.iter().max().copied().unwrap();

            if min_value >= i8::MIN as i128 && max_value <= i8::MAX as i128 {
                let int8_values: Vec<i8> = decimal_values.into_iter().map(|v| v as i8).collect();
                Ok((
                    create_field_with_type(row_type, Some(DataType::Int8))?,
                    Arc::new(Int8Array::from(int8_values)),
                ))
            } else if min_value >= i16::MIN as i128 && max_value <= i16::MAX as i128 {
                let int16_values: Vec<i16> = decimal_values.into_iter().map(|v| v as i16).collect();
                Ok((
                    create_field_with_type(row_type, Some(DataType::Int16))?,
                    Arc::new(arrow::array::Int16Array::from(int16_values)),
                ))
            } else if min_value >= i32::MIN as i128 && max_value <= i32::MAX as i128 {
                let int32_values: Vec<i32> = decimal_values.into_iter().map(|v| v as i32).collect();
                Ok((
                    create_field_with_type(row_type, Some(DataType::Int32))?,
                    Arc::new(arrow::array::Int32Array::from(int32_values)),
                ))
            } else if min_value >= i64::MIN as i128 && max_value <= i64::MAX as i128 {
                let int64_values: Vec<i64> = decimal_values.into_iter().map(|v| v as i64).collect();
                Ok((
                    create_field_with_type(row_type, Some(DataType::Int64))?,
                    Arc::new(Int64Array::from(int64_values)),
                ))
            } else {
                Ok((
                    create_field_with_type(
                        row_type,
                        Some(DataType::Decimal128(*precision as u8, *scale as i8)),
                    )?,
                    Arc::new(
                        arrow::array::Decimal128Array::from(decimal_values)
                            .with_precision_and_scale(*precision as u8, *scale as i8)
                            .context(ArrowSnafu {})?,
                    ),
                ))
            }
        }
        RowType::Boolean { .. } => {
            let bool_values: Result<Vec<bool>, ArrowUtilsError> = values
                .into_iter()
                .map(|v| match v {
                    "true" => Ok(true),
                    "false" => Ok(false),
                    other => BooleanParsingSnafu {
                        value: other.to_string(),
                    }
                    .fail(),
                })
                .collect();
            Ok((
                create_field(row_type)?,
                Arc::new(BooleanArray::from(bool_values?)),
            ))
        }
        RowType::Real { .. } => {
            let float_values: Result<Vec<f64>, ArrowUtilsError> = values
                .into_iter()
                .map(|v| {
                    v.parse::<f64>().context(FloatParsingSnafu {
                        value: v.to_string(),
                    })
                })
                .collect();
            Ok((
                create_field(row_type)?,
                Arc::new(Float64Array::from(float_values?)),
            ))
        }
        RowType::Date { .. } => {
            let day_values: Result<Vec<i32>, ArrowUtilsError> = values
                .into_iter()
                .map(|v| {
                    v.parse::<i32>().context(IntegerParsingSnafu {
                        value: v.to_string(),
                    })
                })
                .collect();
            Ok((
                create_field(row_type)?,
                Arc::new(arrow::array::PrimitiveArray::<Date32Type>::from(
                    day_values?,
                )),
            ))
        }
        RowType::TimestampNtz { scale, .. } | RowType::TimestampLtz { scale, .. } => {
            let all_values: Result<Vec<(i64, i32)>, ArrowUtilsError> = values
                .into_iter()
                .map(|v| (v, v.split_once(".")))
                .map(|(orig, split)| match split {
                    None => (orig, None),
                    Some((epoch, fraction)) => (epoch, Some(fraction)),
                })
                .map(|(epoch, fraction)| {
                    let epoch: i64 = epoch.parse().context(IntegerParsingSnafu {
                        value: epoch.to_string(),
                    })?;
                    let fraction: i32 = match fraction {
                        None => Ok(0),
                        Some(f) => {
                            let filled_with_zeros =
                                format!("{:0<width$}", f, width = *scale as usize);
                            let parsed_fraction =
                                filled_with_zeros
                                    .parse::<i32>()
                                    .context(IntegerParsingSnafu {
                                        value: f.to_string(),
                                    })?;
                            Ok(parsed_fraction)
                        }
                    }?;
                    Ok((epoch, fraction))
                })
                .collect();
            let all_values = all_values?;
            let (epoch_values, fraction_values): (Vec<i64>, Vec<i32>) =
                all_values.into_iter().unzip();

            let field = create_field(row_type)?;
            match field.data_type() {
                DataType::Int64 => {
                    let normalized_epoch_values: Vec<i64> = epoch_values
                        .iter()
                        .zip(fraction_values.iter())
                        .map(|(epoch, fraction)| {
                            epoch * 10i64.pow(*scale as u32) + *fraction as i64
                        })
                        .collect();
                    Ok((field, Arc::new(Int64Array::from(normalized_epoch_values))))
                }
                DataType::Struct(fields) => {
                    let normalized_fraction_values: Vec<i32> = fraction_values
                        .iter()
                        .map(|f| f * 10i32.pow((9 - *scale) as u32))
                        .collect();
                    let epoch_array: Arc<dyn Array> =
                        Arc::new(arrow::array::PrimitiveArray::<Int64Type>::from(
                            epoch_values,
                        ));
                    let fraction_array: Arc<dyn Array> =
                        Arc::new(arrow::array::PrimitiveArray::<Int32Type>::from(
                            normalized_fraction_values,
                        ));
                    let values = vec![
                        (fields[0].clone(), epoch_array),
                        (fields[1].clone(), fraction_array),
                    ];
                    Ok((field, Arc::new(arrow::array::StructArray::from(values))))
                }
                _ => UnsupportedDataTypeSnafu {
                    data_type: format!("{:?}", field.data_type()),
                    row_type: "TIMESTAMP_NTZ".to_string(),
                }
                .fail(),
            }
        }
        RowType::TimestampTz { scale, .. } => {
            #[allow(clippy::type_complexity)]
            let all_values: Result<Vec<((i64, i32), i32)>, ArrowUtilsError> = values
                .into_iter()
                .map(|v| {
                    let (epoch_part, tz_part) = v.split_once(' ').unwrap_or((v, ""));
                    let (epoch_str, fraction_str) =
                        epoch_part.split_once('.').unwrap_or((epoch_part, ""));
                    (epoch_str, fraction_str, tz_part)
                })
                .map(|(epoch_str, fraction_str, tz_part)| {
                    let epoch: i64 = epoch_str.parse().context(IntegerParsingSnafu {
                        value: epoch_str.to_string(),
                    })?;
                    let fraction: i32 = if fraction_str.is_empty() {
                        0
                    } else {
                        let filled_with_zeros =
                            format!("{:0<width$}", fraction_str, width = *scale as usize);
                        filled_with_zeros
                            .parse::<i32>()
                            .context(IntegerParsingSnafu {
                                value: fraction_str.to_string(),
                            })?
                    };
                    let tz = if tz_part.is_empty() {
                        // Snowflake uses zone offset [-24h, +24h], which means
                        // [-1440,1440] in minutes. But it is aligned to be zero based
                        // which means the values scope is [0, 2880].
                        // This value means actually zone offset 0.
                        1440
                    } else {
                        tz_part.parse::<i32>().context(IntegerParsingSnafu {
                            value: tz_part.to_string(),
                        })?
                    };
                    // wrapping first two values in a tuple makes unzipping later easier, but it's heavier
                    // potentially could be replaced with returning three values and repackaging to three collections manually
                    Ok(((epoch, fraction), tz))
                })
                .collect();
            let all_values = all_values?;
            let (time_part_values, tz_values): (Vec<(i64, i32)>, Vec<i32>) =
                all_values.into_iter().unzip();
            let (epoch_values, fraction_values): (Vec<i64>, Vec<i32>) =
                time_part_values.into_iter().unzip();

            let field = create_field(row_type)?;
            match field.data_type() {
                DataType::Struct(fields) if fields.len() == 2 => {
                    let normalized_epoch_values: Vec<i64> = epoch_values
                        .iter()
                        .zip(fraction_values.iter())
                        .map(|(epoch, fraction)| {
                            epoch * 10i64.pow(*scale as u32) + *fraction as i64
                        })
                        .collect();
                    let epoch_array: Arc<dyn Array> =
                        Arc::new(arrow::array::PrimitiveArray::<Int64Type>::from(
                            normalized_epoch_values,
                        ));
                    let tz_array: Arc<dyn Array> = Arc::new(Int32Array::from(tz_values));
                    let values = vec![
                        (fields[0].clone(), epoch_array),
                        (fields[1].clone(), tz_array),
                    ];
                    Ok((field, Arc::new(arrow::array::StructArray::from(values))))
                }
                DataType::Struct(fields) if fields.len() == 3 => {
                    let normalized_fraction_values: Vec<i32> = fraction_values
                        .iter()
                        .map(|f| f * 10i32.pow((9 - *scale) as u32))
                        .collect();
                    let epoch_array: Arc<dyn Array> =
                        Arc::new(arrow::array::PrimitiveArray::<Int64Type>::from(
                            epoch_values,
                        ));
                    let fraction_array: Arc<dyn Array> =
                        Arc::new(arrow::array::PrimitiveArray::<Int32Type>::from(
                            normalized_fraction_values,
                        ));
                    let tz_array: Arc<dyn Array> = Arc::new(Int32Array::from(tz_values));
                    let values = vec![
                        (fields[0].clone(), epoch_array),
                        (fields[1].clone(), fraction_array),
                        (fields[2].clone(), tz_array),
                    ];
                    Ok((field, Arc::new(arrow::array::StructArray::from(values))))
                }
                _ => UnsupportedDataTypeSnafu {
                    data_type: format!("{:?}", field.data_type()),
                    row_type: "TIMESTAMP_TZ".to_string(),
                }
                .fail(),
            }
        }
        RowType::Time { scale, .. } => {
            let field = create_field(row_type)?;
            match field.data_type() {
                DataType::Int32 => {
                    let normalized: Result<Vec<i32>, ArrowUtilsError> = values
                        .into_iter()
                        .map(|v| {
                            let (seconds_str, fraction_str) = v.split_once('.').unwrap_or((v, ""));
                            let seconds: i32 =
                                seconds_str.parse().context(IntegerParsingSnafu {
                                    value: v.to_string(),
                                })?;
                            let fraction: i32 = if fraction_str.is_empty() {
                                0
                            } else {
                                let filled =
                                    format!("{:0<width$}", fraction_str, width = *scale as usize);
                                filled.parse::<i32>().context(IntegerParsingSnafu {
                                    value: v.to_string(),
                                })?
                            };
                            Ok(seconds * 10i32.pow(*scale as u32) + fraction)
                        })
                        .collect();
                    Ok((field, Arc::new(Int32Array::from(normalized?))))
                }
                DataType::Int64 => {
                    let normalized: Result<Vec<i64>, ArrowUtilsError> = values
                        .into_iter()
                        .map(|v| {
                            let scale1 = *scale;
                            let (seconds_str, fraction_str) = v.split_once('.').unwrap_or((v, ""));
                            let seconds: i64 =
                                seconds_str.parse().context(IntegerParsingSnafu {
                                    value: v.to_string(),
                                })?;
                            let fraction: i64 = if fraction_str.is_empty() {
                                0
                            } else {
                                let filled =
                                    format!("{:0<width$}", fraction_str, width = scale1 as usize);
                                filled.parse::<i64>().context(IntegerParsingSnafu {
                                    value: v.to_string(),
                                })?
                            };
                            Ok(seconds * 10i64.pow(scale1 as u32) + fraction)
                        })
                        .collect();
                    Ok((field, Arc::new(Int64Array::from(normalized?))))
                }
                _ => UnsupportedDataTypeSnafu {
                    data_type: format!("{:?}", field.data_type()),
                    row_type: "TIME".to_string(),
                }
                .fail(),
            }
        }
        RowType::Binary { .. } => {
            let binary_values: Result<Vec<Vec<u8>>, ArrowUtilsError> = values
                .into_iter()
                .map(|v| hex::decode(v).context(BinaryParsingSnafu {}))
                .collect();
            let binary_values = binary_values?;
            let refs: Vec<&[u8]> = binary_values.iter().map(|v| v.as_slice()).collect();
            Ok((create_field(row_type)?, Arc::new(BinaryArray::from(refs))))
        }
        RowType::Decfloat { .. } => {
            let parsed: Result<Vec<(i16, Vec<u8>)>, ArrowUtilsError> =
                values.into_iter().map(parse_decfloat_str).collect();
            let parsed = parsed?;
            let (exponents, mantissas): (Vec<i16>, Vec<Vec<u8>>) = parsed.into_iter().unzip();
            let mantissa_refs: Vec<&[u8]> = mantissas.iter().map(|v| v.as_slice()).collect();

            let field = create_field(row_type)?;
            match field.data_type() {
                DataType::Struct(fields) => {
                    let exponent_array: Arc<dyn Array> = Arc::new(Int16Array::from(exponents));
                    let mantissa_array: Arc<dyn Array> = Arc::new(BinaryArray::from(mantissa_refs));
                    let values = vec![
                        (fields[0].clone(), exponent_array),
                        (fields[1].clone(), mantissa_array),
                    ];
                    Ok((field, Arc::new(arrow::array::StructArray::from(values))))
                }
                _ => UnsupportedDataTypeSnafu {
                    data_type: format!("{:?}", field.data_type()),
                    row_type: "DECFLOAT".to_string(),
                }
                .fail(),
            }
        }
    }
}

/// Converts a string rowset with RowType metadata to Arrow format
/// Supports TEXT and FIXED (with scale 0) types, converting strings to appropriate Arrow types
/// Assumes rowset and row_types have been validated to have matching column counts
pub fn convert_string_rowset_to_arrow_reader(
    rowset: &[Vec<String>],
    row_types: &[RowType],
) -> Result<Box<dyn arrow::record_batch::RecordBatchReader + Send>, ArrowUtilsError> {
    // Create Arrow arrays for each column
    #[allow(clippy::type_complexity)]
    let schema_and_columns: Result<Vec<(Field, Arc<dyn Array>)>, ArrowUtilsError> = row_types
        .iter()
        .enumerate()
        .map(|(col_idx, row_type)| {
            let values: Vec<&str> = rowset.iter().map(|row| row[col_idx].as_str()).collect();
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
    use arrow::array::{Int16Array, Int64Array, StringArray};
    use arrow::record_batch::RecordBatchReader;

    #[test]
    fn test_string_rowset_translation_with_metadata_small() {
        // Build a Snowflake-like rowset
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
        // Build a Snowflake-like rowset
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
}
