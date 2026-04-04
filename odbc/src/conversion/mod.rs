// mod readers;
pub mod error;
pub(crate) mod param_binding;
mod parsers;
mod traits;
pub mod warning;

mod binary;
#[cfg(test)]
mod binary_tests;
mod boolean;
#[cfg(test)]
mod boolean_tests;
mod date;
mod decfloat;
#[cfg(test)]
mod decfloat_tests;
mod nullable;
mod number;
#[cfg(test)]
mod number_tests;
mod numeric_helpers;
mod real;
#[cfg(test)]
mod real_tests;
#[cfg(test)]
mod test_utils;
mod time;
#[cfg(test)]
mod time_tests;
mod timestamp;
#[cfg(test)]
mod timestamp_tests;
mod varchar;

use arrow::array::Array;
use arrow::datatypes::{
    DataType, Date32Type, Decimal128Type, Field, Float64Type, Int8Type, Int16Type, Int32Type,
    Int64Type,
};
use snafu::ResultExt;
pub use traits::{Binding, LengthOrNull, ReadArrowType, SnowflakeType, WriteODBCType};

pub use error::{
    ArrowArrayDowncastSnafu, ConversionError, FieldMetadataParsingSnafu, MissingFieldMetadataSnafu,
};
pub use number::{NumericSettings, SF_DEFAULT_VARCHAR_MAX_LEN};

use crate::conversion::error::{
    IncompatibleFieldMetadataSnafu, ReadArrowValueSnafu, UnsupportedArrowDataTypeSnafu,
    WriteOdbcValueSnafu,
};
use crate::conversion::warning::Warnings;

pub trait Converter<'a> {
    fn convert_arrow_value(
        &self,
        row_idx: usize,
        binding: &Binding,
        get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, ConversionError>;
}

struct GenericConverter<'a, ArrowArrayType, T> {
    snowflake_type: T,
    arrow_array: &'a ArrowArrayType,
}

impl<'a, ArrowArrayType: Array, T: SnowflakeType + WriteODBCType + ReadArrowType<ArrowArrayType>>
    Converter<'a> for GenericConverter<'a, ArrowArrayType, T>
{
    fn convert_arrow_value(
        &self,
        row_idx: usize,
        binding: &Binding,
        get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, ConversionError> {
        tracing::debug!(
            "convert_arrow_value: row_idx={}, binding={:?}",
            row_idx,
            binding
        );
        let value = self
            .snowflake_type
            .read_arrow_type(self.arrow_array, row_idx)
            .context(ReadArrowValueSnafu)?;
        self.snowflake_type
            .write_odbc_type(value, binding, get_data_offset)
            .context(WriteOdbcValueSnafu)
    }
}

macro_rules! make_converter {
    ($arrow_array_type:ty, $snowflake_type:expr, $arrow_array:expr, $nullable:expr) => {{
        let arrow_array = $arrow_array
            .as_any()
            .downcast_ref::<$arrow_array_type>()
            .ok_or(
                ArrowArrayDowncastSnafu {
                    expected_type: stringify!($arrow_array_type).to_string(),
                }
                .build(),
            )?;
        if $nullable {
            Ok(Box::new(GenericConverter {
                snowflake_type: nullable::Nullable {
                    value: $snowflake_type,
                },
                arrow_array,
            }))
        } else {
            Ok(Box::new(GenericConverter {
                snowflake_type: $snowflake_type,
                arrow_array,
            }))
        }
    }};
}

macro_rules! make_primitive_data_converter {
    ($arrow_type:ty, $snowflake_type:expr, $arrow_array:expr, $nullable:expr) => {{
        make_converter!(
            arrow::array::PrimitiveArray<$arrow_type>,
            $snowflake_type,
            $arrow_array,
            $nullable
        )
    }};
}

macro_rules! make_timestamp_converter {
    ($snowflake_type:expr, $field:expr, $arrow_array:expr, $nullable:expr) => {
        match $field.data_type() {
            DataType::Struct(_) => {
                make_converter!(
                    arrow::array::StructArray,
                    $snowflake_type,
                    $arrow_array,
                    $nullable
                )
            }
            _ => {
                make_primitive_data_converter!(Int64Type, $snowflake_type, $arrow_array, $nullable)
            }
        }
    };
}

fn get_field_metadata(field: &Field, key: &str) -> Result<u32, ConversionError> {
    let metadata = field.metadata().get(key).ok_or(
        MissingFieldMetadataSnafu {
            key: key.to_string(),
            field_name: field.name().to_string(),
        }
        .build(),
    )?;
    let parsed = metadata.parse::<u32>().map_err(|e| {
        FieldMetadataParsingSnafu {
            field_name: field.name().to_string(),
            key: key.to_string(),
            reason: e.to_string(),
        }
        .build()
    })?;
    Ok(parsed)
}

fn timestamp_scale(field: &Field) -> Result<u32, ConversionError> {
    match get_field_metadata(field, "scale") {
        Ok(scale) if scale > 9 => {
            tracing::warn!(
                field_name = field.name().as_str(),
                scale,
                "Timestamp scale exceeds maximum of 9, capping to 9"
            );
            Ok(9)
        }
        Ok(scale) => Ok(scale),
        Err(ConversionError::MissingFieldMetadata { .. }) => {
            tracing::warn!(
                field_name = field.name().as_str(),
                "Missing 'scale' metadata for timestamp field, defaulting to 9"
            );
            Ok(9)
        }
        Err(e) => Err(e),
    }
}

/// Parsed Snowflake type from an Arrow field's metadata.
enum SnowflakeFieldType {
    Varchar(varchar::SnowflakeVarchar),
    Number(number::SnowflakeNumber),
    Date(date::SnowflakeDate),
    Time(time::SnowflakeTime),
    TimestampNtz(timestamp::SnowflakeTimestampNtz),
    TimestampLtz(timestamp::SnowflakeTimestampLtz),
    TimestampTz(timestamp::SnowflakeTimestampTz),
    Boolean(boolean::SnowflakeBoolean),
    Binary(binary::SnowflakeBinary),
    Real(real::SnowflakeReal),
    Decfloat(decfloat::SnowflakeDecfloat),
}

impl SnowflakeFieldType {
    fn from_field(
        field: &Field,
        numeric_settings: &NumericSettings,
    ) -> Result<Self, ConversionError> {
        let logical_type = field
            .metadata()
            .get("logicalType")
            .map(|s| s.as_str())
            .unwrap_or("");
        match logical_type {
            "TEXT" => {
                let len = get_field_metadata(field, "charLength")?;
                Ok(Self::Varchar(varchar::SnowflakeVarchar { len }))
            }
            "FIXED" => {
                let scale = get_field_metadata(field, "scale")?;
                let precision = get_field_metadata(field, "precision")?;
                let sql_type = number::NumericSqlType::from_scale_and_precision(
                    scale,
                    precision,
                    numeric_settings,
                );
                Ok(Self::Number(number::SnowflakeNumber {
                    scale,
                    precision,
                    sql_type,
                }))
            }
            "DATE" => Ok(Self::Date(date::SnowflakeDate)),
            "TIME" => {
                let scale = get_field_metadata(field, "scale")?;
                Ok(Self::Time(time::SnowflakeTime { scale }))
            }
            "TIMESTAMP_NTZ" => Ok(Self::TimestampNtz(timestamp::SnowflakeTimestampNtz {
                scale: timestamp_scale(field)?,
            })),
            "TIMESTAMP_LTZ" => Ok(Self::TimestampLtz(timestamp::SnowflakeTimestampLtz {
                scale: timestamp_scale(field)?,
            })),
            "TIMESTAMP_TZ" => Ok(Self::TimestampTz(timestamp::SnowflakeTimestampTz {
                scale: timestamp_scale(field)?,
            })),
            "BOOLEAN" => Ok(Self::Boolean(boolean::SnowflakeBoolean)),
            "BINARY" => {
                let len = match get_field_metadata(field, "byteLength") {
                    Ok(len) => len,
                    // byteLength is optional; default to Snowflake's max (8 MB).
                    Err(ConversionError::MissingFieldMetadata { .. }) => 8_388_608,
                    Err(e) => return Err(e),
                };
                Ok(Self::Binary(binary::SnowflakeBinary { len }))
            }
            "REAL" => Ok(Self::Real(real::SnowflakeReal)),
            "DECFLOAT" => {
                let precision = get_field_metadata(field, "precision")?;
                Ok(Self::Decfloat(decfloat::SnowflakeDecfloat { precision }))
            }
            lt => IncompatibleFieldMetadataSnafu {
                logical_type: lt.to_string(),
                data_type: field.data_type().clone(),
            }
            .fail(),
        }
    }

    fn sql_type(&self) -> odbc_sys::SqlDataType {
        match self {
            Self::Varchar(t) => t.sql_type(),
            Self::Number(t) => t.sql_type(),
            Self::Date(t) => t.sql_type(),
            Self::Time(t) => t.sql_type(),
            Self::TimestampNtz(t) => t.sql_type(),
            Self::TimestampLtz(t) => t.sql_type(),
            Self::TimestampTz(t) => t.sql_type(),
            Self::Boolean(t) => t.sql_type(),
            Self::Binary(t) => t.sql_type(),
            Self::Real(t) => t.sql_type(),
            Self::Decfloat(t) => t.sql_type(),
        }
    }

    fn column_size(&self) -> odbc_sys::ULen {
        match self {
            Self::Varchar(t) => t.column_size(),
            Self::Number(t) => t.column_size(),
            Self::Date(t) => t.column_size(),
            Self::Time(t) => t.column_size(),
            Self::TimestampNtz(t) => t.column_size(),
            Self::TimestampLtz(t) => t.column_size(),
            Self::TimestampTz(t) => t.column_size(),
            Self::Boolean(t) => t.column_size(),
            Self::Binary(t) => t.column_size(),
            Self::Real(t) => t.column_size(),
            Self::Decfloat(t) => t.column_size(),
        }
    }

    fn decimal_digits(&self) -> odbc_sys::SmallInt {
        match self {
            Self::Varchar(t) => t.decimal_digits(),
            Self::Number(t) => t.decimal_digits(),
            Self::Date(t) => t.decimal_digits(),
            Self::Time(t) => t.decimal_digits(),
            Self::TimestampNtz(t) => t.decimal_digits(),
            Self::TimestampLtz(t) => t.decimal_digits(),
            Self::TimestampTz(t) => t.decimal_digits(),
            Self::Boolean(t) => t.decimal_digits(),
            Self::Binary(t) => t.decimal_digits(),
            Self::Real(t) => t.decimal_digits(),
            Self::Decfloat(t) => t.decimal_digits(),
        }
    }
}

pub fn make_converter<'a>(
    field: &Field,
    arrow_array: &'a dyn Array,
    numeric_settings: &NumericSettings,
) -> Result<Box<dyn Converter<'a> + 'a>, ConversionError> {
    let field_type = SnowflakeFieldType::from_field(field, numeric_settings)?;
    let nullable = field.is_nullable();
    match field_type {
        SnowflakeFieldType::Varchar(snowflake_type) => {
            make_converter!(
                arrow::array::GenericByteArray<arrow::datatypes::Utf8Type>,
                snowflake_type,
                arrow_array,
                nullable
            )
        }
        SnowflakeFieldType::Number(snowflake_type) => match field.data_type() {
            DataType::Int8 => {
                make_primitive_data_converter!(Int8Type, snowflake_type, arrow_array, nullable)
            }
            DataType::Int16 => {
                make_primitive_data_converter!(Int16Type, snowflake_type, arrow_array, nullable)
            }
            DataType::Int32 => {
                make_primitive_data_converter!(Int32Type, snowflake_type, arrow_array, nullable)
            }
            DataType::Int64 => {
                make_primitive_data_converter!(Int64Type, snowflake_type, arrow_array, nullable)
            }
            DataType::Decimal128(_, _) => {
                make_primitive_data_converter!(
                    Decimal128Type,
                    snowflake_type,
                    arrow_array,
                    nullable
                )
            }
            dt => UnsupportedArrowDataTypeSnafu {
                data_type: dt.clone(),
            }
            .fail(),
        },
        SnowflakeFieldType::Date(snowflake_type) => {
            make_primitive_data_converter!(Date32Type, snowflake_type, arrow_array, nullable)
        }
        SnowflakeFieldType::Time(snowflake_type) => {
            make_primitive_data_converter!(Int64Type, snowflake_type, arrow_array, nullable)
        }
        SnowflakeFieldType::TimestampNtz(snowflake_type) => {
            make_timestamp_converter!(snowflake_type, field, arrow_array, nullable)
        }
        SnowflakeFieldType::TimestampLtz(snowflake_type) => {
            make_timestamp_converter!(snowflake_type, field, arrow_array, nullable)
        }
        SnowflakeFieldType::TimestampTz(snowflake_type) => {
            make_timestamp_converter!(snowflake_type, field, arrow_array, nullable)
        }
        SnowflakeFieldType::Boolean(snowflake_type) => {
            make_converter!(
                arrow::array::BooleanArray,
                snowflake_type,
                arrow_array,
                nullable
            )
        }
        SnowflakeFieldType::Binary(snowflake_type) => {
            make_converter!(
                arrow::array::GenericByteArray<arrow::datatypes::GenericBinaryType<i32>>,
                snowflake_type,
                arrow_array,
                nullable
            )
        }
        SnowflakeFieldType::Real(snowflake_type) => {
            make_primitive_data_converter!(Float64Type, snowflake_type, arrow_array, nullable)
        }
        SnowflakeFieldType::Decfloat(snowflake_type) => {
            make_converter!(
                arrow::array::StructArray,
                snowflake_type,
                arrow_array,
                nullable
            )
        }
    }
}

/// Map a Snowflake Arrow field to the corresponding SQL data type.
pub fn sql_type_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<odbc_sys::SqlDataType, ConversionError> {
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.sql_type())
}

pub fn column_size_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<odbc_sys::ULen, ConversionError> {
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.column_size())
}

pub fn decimal_digits_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<odbc_sys::SmallInt, ConversionError> {
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.decimal_digits())
}
