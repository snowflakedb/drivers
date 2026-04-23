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
#[cfg(test)]
mod converter_tests;
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
mod semi_structured_tests;
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
pub use traits::{
    Binding, BindingStrides, LengthOrNull, ReadArrowType, SnowflakeType, WriteODBCType,
};

pub use error::{
    ArrowArrayDowncastSnafu, ConversionError, FieldMetadataParsingSnafu, MissingFieldMetadataSnafu,
};
pub use number::{NumericSettings, SF_DEFAULT_VARCHAR_MAX_LEN};

use crate::conversion::error::{
    IncompatibleFieldMetadataSnafu, ReadArrowValueSnafu, UnsupportedArrowDataTypeSnafu,
    WriteOdbcValueSnafu,
};
use crate::conversion::warning::Warnings;

/// Per-column converter from Arrow values to ODBC buffers.
///
/// `'static` so it can be cached per column per `RecordBatch`. Two entry
/// points: `convert_arrow_value` for single-cell `SQLGetData`, and
/// `convert_arrow_range` for `SQLFetch` segments (overridden to amortise the
/// Arrow downcast across the whole segment).
///
/// This is the type-erased handle stored in the per-batch cache as
/// `Box<dyn ColumnConverter>`. The single concrete implementation is
/// `Converter<ArrowArrayType, T>`, monomorphised once per Arrow/Snowflake
/// type pair.
pub trait ColumnConverter {
    fn convert_arrow_value(
        &self,
        array: &dyn Array,
        row_idx: usize,
        binding: &Binding,
        get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, ConversionError>;

    /// Convert each row in `arrow_row_range` into `outputs[i]`, skipping
    /// rows that already hold `Err` (preserving row-major "first error
    /// aborts the row" semantics). The per-row `Binding` is materialised
    /// inline via `BindingStrides::for_row`. Default impl is per-cell;
    /// `Converter<A, T>` overrides it to downcast once per segment.
    fn convert_arrow_range(
        &self,
        array: &dyn Array,
        arrow_row_range: std::ops::Range<usize>,
        base_binding: &Binding,
        out_row_start: usize,
        strides: BindingStrides,
        outputs: &mut [Result<Warnings, ConversionError>],
    ) {
        for (i, batch_idx) in arrow_row_range.enumerate() {
            if outputs[i].is_err() {
                continue;
            }
            let binding = strides.for_row(base_binding, out_row_start + i);
            match self.convert_arrow_value(array, batch_idx, &binding, &mut None) {
                Ok(w) => {
                    if let Ok(existing) = &mut outputs[i] {
                        existing.extend(w);
                    }
                }
                Err(e) => {
                    outputs[i] = Err(e);
                }
            }
        }
    }
}

/// Concrete column converter, parameterised over a single Arrow array type
/// `ArrowArrayType` and a single Snowflake logical type `T`. Each
/// `(ArrowArrayType, T)` pair monomorphises into a distinct concrete type;
/// the per-batch cache stores them as `Box<dyn ColumnConverter>`.
struct Converter<ArrowArrayType, T> {
    snowflake_type: T,
    // `fn() -> ArrowArrayType` so the marker imposes no auto-trait bounds.
    _phantom: std::marker::PhantomData<fn() -> ArrowArrayType>,
}

impl<
    ArrowArrayType: Array + 'static,
    T: SnowflakeType + WriteODBCType + ReadArrowType<ArrowArrayType>,
> ColumnConverter for Converter<ArrowArrayType, T>
{
    fn convert_arrow_value(
        &self,
        array: &dyn Array,
        row_idx: usize,
        binding: &Binding,
        get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, ConversionError> {
        let arrow_array = array.as_any().downcast_ref::<ArrowArrayType>().ok_or(
            ArrowArrayDowncastSnafu {
                expected_type: std::any::type_name::<ArrowArrayType>().to_string(),
            }
            .build(),
        )?;
        let value = self
            .snowflake_type
            .read_arrow_type(arrow_array, row_idx)
            .context(ReadArrowValueSnafu)?;
        self.snowflake_type
            .write_odbc_type(value, binding, get_data_offset)
            .context(WriteOdbcValueSnafu)
    }

    /// Downcasts `array` once for the whole segment, then iterates rows
    /// through statically-dispatched `read_arrow_type`/`write_odbc_type`
    /// — no per-cell vtable, closure, or `Any` cost. Falls back to the
    /// per-cell default impl on downcast failure so each row reports the
    /// original error.
    fn convert_arrow_range(
        &self,
        array: &dyn Array,
        arrow_row_range: std::ops::Range<usize>,
        base_binding: &Binding,
        out_row_start: usize,
        strides: BindingStrides,
        outputs: &mut [Result<Warnings, ConversionError>],
    ) {
        let Some(arrow_array) = array.as_any().downcast_ref::<ArrowArrayType>() else {
            for (i, batch_idx) in arrow_row_range.enumerate() {
                if outputs[i].is_err() {
                    continue;
                }
                let binding = strides.for_row(base_binding, out_row_start + i);
                match self.convert_arrow_value(array, batch_idx, &binding, &mut None) {
                    Ok(w) => {
                        if let Ok(existing) = &mut outputs[i] {
                            existing.extend(w);
                        }
                    }
                    Err(e) => {
                        outputs[i] = Err(e);
                    }
                }
            }
            return;
        };

        for (i, batch_idx) in arrow_row_range.enumerate() {
            if outputs[i].is_err() {
                continue;
            }
            let binding = strides.for_row(base_binding, out_row_start + i);
            let result = self
                .snowflake_type
                .read_arrow_type(arrow_array, batch_idx)
                .context(ReadArrowValueSnafu)
                .and_then(|value| {
                    self.snowflake_type
                        .write_odbc_type(value, &binding, &mut None)
                        .context(WriteOdbcValueSnafu)
                });
            match result {
                Ok(w) => {
                    if let Ok(existing) = &mut outputs[i] {
                        existing.extend(w);
                    }
                }
                Err(e) => {
                    outputs[i] = Err(e);
                }
            }
        }
    }
}

macro_rules! make_converter {
    ($arrow_array_type:ty, $snowflake_type:expr, $nullable:expr) => {{
        if $nullable {
            Ok(Box::new(Converter::<$arrow_array_type, _> {
                snowflake_type: nullable::Nullable {
                    value: $snowflake_type,
                },
                _phantom: std::marker::PhantomData,
            }) as Box<dyn ColumnConverter>)
        } else {
            Ok(Box::new(Converter::<$arrow_array_type, _> {
                snowflake_type: $snowflake_type,
                _phantom: std::marker::PhantomData,
            }) as Box<dyn ColumnConverter>)
        }
    }};
}

macro_rules! make_primitive_data_converter {
    ($arrow_type:ty, $snowflake_type:expr, $nullable:expr) => {{
        make_converter!(
            arrow::array::PrimitiveArray<$arrow_type>,
            $snowflake_type,
            $nullable
        )
    }};
}

macro_rules! make_timestamp_converter {
    ($snowflake_type:expr, $field:expr, $nullable:expr) => {
        match $field.data_type() {
            DataType::Struct(_) => {
                make_converter!(arrow::array::StructArray, $snowflake_type, $nullable)
            }
            _ => {
                make_primitive_data_converter!(Int64Type, $snowflake_type, $nullable)
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
                Ok(Self::Varchar(varchar::SnowflakeVarchar {
                    len,
                    is_semi_structured: false,
                }))
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
            "OBJECT" | "ARRAY" | "VARIANT" => {
                let len = match get_field_metadata(field, "charLength") {
                    Ok(len) => len,
                    // charLength is optional; fall back to the server-configured
                    // max VARCHAR size used elsewhere in ODBC metadata reporting.
                    Err(ConversionError::MissingFieldMetadata { .. }) => {
                        numeric_settings.max_varchar_size.min(u32::MAX as u64) as u32
                    }
                    Err(e) => return Err(e),
                };
                Ok(Self::Varchar(varchar::SnowflakeVarchar {
                    len,
                    is_semi_structured: true,
                }))
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

/// Build a converter for a column from its Arrow `Field`.
///
/// The returned `Box<dyn ColumnConverter>` is `'static` and meant to be built
/// once per column per `RecordBatch` (the `SnowflakeFieldType::from_field`
/// work here parses field metadata and is too expensive to do per cell).
pub fn make_converter(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<Box<dyn ColumnConverter>, ConversionError> {
    let field_type = SnowflakeFieldType::from_field(field, numeric_settings)?;
    let nullable = field.is_nullable();
    match field_type {
        SnowflakeFieldType::Varchar(snowflake_type) => {
            make_converter!(
                arrow::array::GenericByteArray<arrow::datatypes::Utf8Type>,
                snowflake_type,
                nullable
            )
        }
        SnowflakeFieldType::Number(snowflake_type) => match field.data_type() {
            DataType::Int8 => {
                make_primitive_data_converter!(Int8Type, snowflake_type, nullable)
            }
            DataType::Int16 => {
                make_primitive_data_converter!(Int16Type, snowflake_type, nullable)
            }
            DataType::Int32 => {
                make_primitive_data_converter!(Int32Type, snowflake_type, nullable)
            }
            DataType::Int64 => {
                make_primitive_data_converter!(Int64Type, snowflake_type, nullable)
            }
            DataType::Decimal128(_, _) => {
                make_primitive_data_converter!(Decimal128Type, snowflake_type, nullable)
            }
            dt => UnsupportedArrowDataTypeSnafu {
                data_type: dt.clone(),
            }
            .fail(),
        },
        SnowflakeFieldType::Date(snowflake_type) => {
            make_primitive_data_converter!(Date32Type, snowflake_type, nullable)
        }
        SnowflakeFieldType::Time(snowflake_type) => {
            make_primitive_data_converter!(Int64Type, snowflake_type, nullable)
        }
        SnowflakeFieldType::TimestampNtz(snowflake_type) => {
            make_timestamp_converter!(snowflake_type, field, nullable)
        }
        SnowflakeFieldType::TimestampLtz(snowflake_type) => {
            make_timestamp_converter!(snowflake_type, field, nullable)
        }
        SnowflakeFieldType::TimestampTz(snowflake_type) => {
            make_timestamp_converter!(snowflake_type, field, nullable)
        }
        SnowflakeFieldType::Boolean(snowflake_type) => {
            make_converter!(arrow::array::BooleanArray, snowflake_type, nullable)
        }
        SnowflakeFieldType::Binary(snowflake_type) => {
            make_converter!(
                arrow::array::GenericByteArray<arrow::datatypes::GenericBinaryType<i32>>,
                snowflake_type,
                nullable
            )
        }
        SnowflakeFieldType::Real(snowflake_type) => {
            make_primitive_data_converter!(Float64Type, snowflake_type, nullable)
        }
        SnowflakeFieldType::Decfloat(snowflake_type) => {
            make_converter!(arrow::array::StructArray, snowflake_type, nullable)
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
