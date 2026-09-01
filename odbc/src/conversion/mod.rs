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
#[cfg(test)]
mod getdata_probe_tests;
mod int_fmt;
mod interval;
mod interval_str;
#[cfg(test)]
mod interval_str_tests;
#[cfg(test)]
mod interval_tests;
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
pub(crate) mod timestamp;
#[cfg(test)]
mod timestamp_tests;
mod varchar;
mod vector;
#[cfg(test)]
mod vector_tests;

use crate::api::CDataType;
use arrow::array::{Array, ArrowPrimitiveType, PrimitiveArray};
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
pub use number::{NumericSettings, SF_DEFAULT_VARCHAR_MAX_LEN, TzOffsetFormatCache};

#[cfg(not(windows))]
use crate::api::encoding::is_ascii_locale;
use crate::conversion::error::{
    IncompatibleFieldMetadataSnafu, ReadArrowValueSnafu, UnsupportedArrowDataTypeSnafu,
    WriteOdbcValueSnafu,
};
use crate::conversion::warning::Warnings;

/// Maximum bytes per character for the narrow (ANSI) ODBC API, matching old
/// driver behavior:
/// - Windows: 1 (Windows-1252 single-byte)
/// - Unix: 4 (UTF-8 worst-case)
fn narrow_char_byte_width() -> odbc_sys::Len {
    #[cfg(windows)]
    {
        1
    }
    #[cfg(not(windows))]
    {
        if is_ascii_locale() { 1 } else { 4 }
    }
}

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
    /// inline via `BindingStrides::for_row`. Default impl is per-cell via
    /// [`per_cell_convert_range`]; `Converter<A, T>` overrides it to
    /// downcast once per segment.
    fn convert_arrow_range(
        &self,
        array: &dyn Array,
        arrow_row_range: std::ops::Range<usize>,
        base_binding: &Binding,
        out_row_start: usize,
        strides: BindingStrides,
        outputs: &mut [Result<Warnings, ConversionError>],
    ) {
        per_cell_convert_range(
            self,
            array,
            arrow_row_range,
            base_binding,
            out_row_start,
            strides,
            outputs,
        );
    }
}

/// Shared per-row loop used by both the default `convert_arrow_range`
/// impl and the downcast-failure fallback in `Converter`. Extracted so
/// the row-skip / first-error semantics live in exactly one place.
fn per_cell_convert_range(
    converter: &(impl ColumnConverter + ?Sized),
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
        let binding = match strides.for_row(base_binding, out_row_start + i) {
            Ok(b) => b,
            Err(e) => {
                outputs[i] = Err(e);
                continue;
            }
        };
        match converter.convert_arrow_value(array, batch_idx, &binding, &mut None) {
            Ok(w) => {
                // Warnings are rare; gating the `outputs[i]` index + extend on the
                // empty-warning check is worth ~3% on NUMBER fetches. Kept as a
                // nested `if` (not a `&& let` chain) per review preference — the
                // clippy collapse suggestion would reintroduce the let-chain.
                #[allow(clippy::collapsible_if)]
                if !w.is_empty() {
                    if let Ok(existing) = &mut outputs[i] {
                        existing.extend(w);
                    }
                }
            }
            Err(e) => {
                outputs[i] = Err(e);
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
        self.snowflake_type.validate_value(&value)?;
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
            per_cell_convert_range(
                self,
                array,
                arrow_row_range,
                base_binding,
                out_row_start,
                strides,
                outputs,
            );
            return;
        };

        // Incremental striding: materialize the first row's binding once
        // (handling `bind_offset` and the pathological stride-overflow case),
        // then advance the pointers by a constant per-row stride each cell via
        // `Binding::stepped`, instead of recomputing `row_idx * stride` through
        // three overflow-checked `advance_ptr` calls in `for_row` on every
        // cell. If the very first row already overflows we fall back to the
        // per-cell path so overflow is still reported per row, exactly as
        // before.
        let mut binding = match strides.for_row(base_binding, out_row_start) {
            Ok(b) => b,
            Err(_) => {
                per_cell_convert_range(
                    self,
                    array,
                    arrow_row_range,
                    base_binding,
                    out_row_start,
                    strides,
                    outputs,
                );
                return;
            }
        };
        let (value_stride, indicator_stride) =
            strides.row_step(base_binding.target_type, base_binding.buffer_length);

        for (i, batch_idx) in arrow_row_range.enumerate() {
            if i > 0 {
                binding = binding.stepped(value_stride, indicator_stride);
            }
            if outputs[i].is_err() {
                continue;
            }
            let result = self
                .snowflake_type
                .read_arrow_type(arrow_array, batch_idx)
                .context(ReadArrowValueSnafu)
                .and_then(|value| {
                    self.snowflake_type.validate_value(&value)?;
                    self.snowflake_type
                        .write_odbc_type(value, &binding, &mut None)
                        .context(WriteOdbcValueSnafu)
                });
            match result {
                Ok(w) => {
                    // Warnings are rare; gating the `outputs[i]` index + extend on
                    // the empty-warning check is worth ~3% on NUMBER fetches. Kept
                    // as a nested `if` (not a `&& let` chain) per review preference
                    // — the clippy collapse suggestion would reintroduce it.
                    #[allow(clippy::collapsible_if)]
                    if !w.is_empty() {
                        if let Ok(existing) = &mut outputs[i] {
                            existing.extend(w);
                        }
                    }
                }
                Err(e) => {
                    outputs[i] = Err(e);
                }
            }
        }
    }
}

/// Wraps the generic NUMBER converter, intercepting the hot `SQL_C_CHAR`
/// range conversion with the batched [`number::convert_number_char_range`]
/// (read + format + write inlined into one loop) and delegating every other
/// target — and the single-cell `SQLGetData` path — to the generic per-cell
/// converter unchanged. If the batched path declines (stride overflow, or a
/// non-nullable column carrying nulls) it also falls back to the generic path,
/// so behavior is identical in every case.
struct NumberCharConverter<T: ArrowPrimitiveType> {
    inner: Box<dyn ColumnConverter>,
    scale: u32,
    nullable: bool,
    _phantom: std::marker::PhantomData<fn() -> T>,
}

impl<T> ColumnConverter for NumberCharConverter<T>
where
    T: ArrowPrimitiveType + 'static,
    T::Native: Into<i128>,
{
    fn convert_arrow_value(
        &self,
        array: &dyn Array,
        row_idx: usize,
        binding: &Binding,
        get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, ConversionError> {
        self.inner
            .convert_arrow_value(array, row_idx, binding, get_data_offset)
    }

    fn convert_arrow_range(
        &self,
        array: &dyn Array,
        arrow_row_range: std::ops::Range<usize>,
        base_binding: &Binding,
        out_row_start: usize,
        strides: BindingStrides,
        outputs: &mut [Result<Warnings, ConversionError>],
    ) {
        if base_binding.target_type == CDataType::Char
            && let Some(arr) = array.as_any().downcast_ref::<PrimitiveArray<T>>()
            && number::convert_number_char_range(
                self.scale,
                self.nullable,
                arr,
                arrow_row_range.clone(),
                base_binding,
                out_row_start,
                strides,
                outputs,
            )
        {
            return;
        }
        self.inner.convert_arrow_range(
            array,
            arrow_row_range,
            base_binding,
            out_row_start,
            strides,
            outputs,
        );
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

macro_rules! number_char_converter {
    ($arrow_type:ty, $snowflake_type:expr, $nullable:expr) => {{
        let scale = $snowflake_type.scale;
        let inner = make_primitive_data_converter!($arrow_type, $snowflake_type, $nullable)?;
        Ok(Box::new(NumberCharConverter::<$arrow_type> {
            inner,
            scale,
            nullable: $nullable,
            _phantom: std::marker::PhantomData,
        }) as Box<dyn ColumnConverter>)
    }};
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
    Vector(vector::SnowflakeVector),
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
                let len = match get_field_metadata(field, "charLength") {
                    Ok(len) => len,
                    Err(ConversionError::MissingFieldMetadata { .. }) => {
                        u32::try_from(numeric_settings.max_varchar_size)
                            .unwrap_or(SF_DEFAULT_VARCHAR_MAX_LEN as u32)
                    }
                    Err(e) => return Err(e),
                };
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
                tz_offset_format: numeric_settings.tz_offset_format(),
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
            "VECTOR" => {
                // VECTOR columns arrive from sf_core as FixedSizeListArray of Int32 or Float32.
                // Determine the child element type from the Arrow DataType.
                let element_type = match field.data_type() {
                    DataType::FixedSizeList(child_field, _) => match child_field.data_type() {
                        DataType::Int32 => vector::VectorElementType::Int32,
                        DataType::Float32 => vector::VectorElementType::Float32,
                        dt => {
                            return IncompatibleFieldMetadataSnafu {
                                logical_type: format!("VECTOR with unsupported child type {dt:?}"),
                                data_type: field.data_type().clone(),
                            }
                            .fail();
                        }
                    },
                    dt => {
                        return IncompatibleFieldMetadataSnafu {
                            logical_type: "VECTOR".to_string(),
                            data_type: dt.clone(),
                        }
                        .fail();
                    }
                };
                let column_size = match get_field_metadata(field, "charLength") {
                    Ok(len) => len,
                    Err(ConversionError::MissingFieldMetadata { .. }) => {
                        numeric_settings.max_varchar_size.min(u32::MAX as u64) as u32
                    }
                    Err(e) => return Err(e),
                };
                Ok(Self::Vector(vector::SnowflakeVector {
                    element_type,
                    column_size,
                }))
            }
            // Missing logicalType is corrupt metadata.
            "" => IncompatibleFieldMetadataSnafu {
                logical_type: String::new(),
                data_type: field.data_type().clone(),
            }
            .fail(),
            // Snowflake logical types that are not first-class ODBC types are
            // treated as VARCHAR.
            _ => {
                let len = match get_field_metadata(field, "charLength") {
                    Ok(len) => len,
                    Err(ConversionError::MissingFieldMetadata { .. }) => {
                        u32::try_from(numeric_settings.max_varchar_size)
                            .unwrap_or(SF_DEFAULT_VARCHAR_MAX_LEN as u32)
                    }
                    Err(e) => return Err(e),
                };
                Ok(Self::Varchar(varchar::SnowflakeVarchar {
                    len,
                    is_semi_structured: false,
                }))
            }
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
            Self::Vector(t) => t.sql_type(),
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
            Self::Vector(t) => t.column_size(),
        }
    }

    fn precision(&self) -> odbc_sys::ULen {
        match self {
            Self::Binary(_) | Self::Date(_) => 0,
            other => other.column_size(),
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
            Self::Vector(t) => t.decimal_digits(),
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::Varchar(_) => "VARCHAR",
            Self::Number(_) => "DECIMAL",
            Self::Date(_) => "TYPE_DATE",
            Self::Time(_) => "TYPE_TIME",
            Self::TimestampNtz(_) => "TYPE_TIMESTAMP",
            Self::TimestampLtz(_) => "TYPE_TIMESTAMP",
            Self::TimestampTz(_) => "TYPE_TIMESTAMP",
            Self::Boolean(_) => "BIT",
            Self::Binary(_) => "BINARY",
            Self::Real(_) => "DOUBLE",
            Self::Decfloat(_) => "NUMERIC",
            Self::Vector(_) => "VECTOR",
        }
    }

    fn display_size(&self) -> odbc_sys::Len {
        match self {
            Self::Varchar(t) => t.len as odbc_sys::Len,
            Self::Number(_) | Self::Decfloat(_) => 136,
            Self::Date(_) => 10,
            Self::Time(t) => t.column_size() as odbc_sys::Len,
            Self::TimestampNtz(t) => t.column_size() as odbc_sys::Len,
            Self::TimestampLtz(t) => t.column_size() as odbc_sys::Len,
            Self::TimestampTz(t) => t.column_size() as odbc_sys::Len,
            Self::Boolean(_) => 1,
            Self::Binary(t) => 2 * t.len as odbc_sys::Len,
            Self::Real(_) => 24,
            Self::Vector(t) => t.column_size as odbc_sys::Len,
        }
    }

    fn octet_length(&self) -> odbc_sys::Len {
        match self {
            Self::Varchar(t) => (t.len as odbc_sys::Len) * narrow_char_byte_width(),
            Self::Number(_) | Self::Decfloat(_) => 136,
            Self::Date(_) => 6,
            Self::Time(_) => 6,
            Self::TimestampNtz(_) => 16,
            Self::TimestampLtz(_) => 16,
            Self::TimestampTz(_) => 16,
            Self::Boolean(_) => 1,
            Self::Binary(t) => t.len as odbc_sys::Len,
            Self::Real(_) => 8,
            Self::Vector(t) => (t.column_size as odbc_sys::Len) * narrow_char_byte_width(),
        }
    }

    fn num_prec_radix(&self) -> odbc_sys::Len {
        match self {
            Self::Number(_) | Self::Decfloat(_) => 10,
            Self::Real(_) => 2,
            _ => 0,
        }
    }

    fn is_unsigned(&self) -> bool {
        matches!(
            self,
            Self::Varchar(_)
                | Self::Boolean(_)
                | Self::Binary(_)
                | Self::Date(_)
                | Self::Time(_)
                | Self::TimestampNtz(_)
                | Self::TimestampLtz(_)
                | Self::TimestampTz(_)
                | Self::Vector(_)
        )
    }

    fn is_case_sensitive(&self) -> bool {
        matches!(self, Self::Varchar(_))
    }

    fn searchable(&self) -> odbc_sys::Len {
        match self {
            Self::Varchar(_) => 3, // SQL_SEARCHABLE
            _ => 2,                // SQL_ALL_EXCEPT_LIKE
        }
    }

    fn literal_prefix(&self) -> &'static str {
        match self {
            Self::Varchar(_)
            | Self::Date(_)
            | Self::Time(_)
            | Self::TimestampNtz(_)
            | Self::TimestampLtz(_)
            | Self::TimestampTz(_) => "'",
            Self::Binary(_) => "0x",
            _ => "",
        }
    }

    fn literal_suffix(&self) -> &'static str {
        match self {
            Self::Varchar(_)
            | Self::Date(_)
            | Self::Time(_)
            | Self::TimestampNtz(_)
            | Self::TimestampLtz(_)
            | Self::TimestampTz(_) => "'",
            _ => "",
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
            DataType::Int8 => number_char_converter!(Int8Type, snowflake_type, nullable),
            DataType::Int16 => number_char_converter!(Int16Type, snowflake_type, nullable),
            DataType::Int32 => number_char_converter!(Int32Type, snowflake_type, nullable),
            DataType::Int64 => number_char_converter!(Int64Type, snowflake_type, nullable),
            DataType::Decimal128(_, _) => {
                number_char_converter!(Decimal128Type, snowflake_type, nullable)
            }
            dt => UnsupportedArrowDataTypeSnafu {
                data_type: dt.clone(),
            }
            .fail(),
        },
        SnowflakeFieldType::Date(snowflake_type) => {
            make_primitive_data_converter!(Date32Type, snowflake_type, nullable)
        }
        SnowflakeFieldType::Time(snowflake_type) => match field.data_type() {
            DataType::Int32 => {
                make_primitive_data_converter!(Int32Type, snowflake_type, nullable)
            }
            DataType::Int64 => {
                make_primitive_data_converter!(Int64Type, snowflake_type, nullable)
            }
            dt => UnsupportedArrowDataTypeSnafu {
                data_type: dt.clone(),
            }
            .fail(),
        },
        SnowflakeFieldType::TimestampNtz(snowflake_type) => {
            make_timestamp_converter!(snowflake_type, field, nullable)
        }
        SnowflakeFieldType::TimestampLtz(snowflake_type) => {
            make_timestamp_converter!(snowflake_type, field, nullable)
        }
        SnowflakeFieldType::TimestampTz(snowflake_type) => match field.data_type() {
            DataType::Struct(_) => {
                make_converter!(arrow::array::StructArray, snowflake_type, nullable)
            }
            dt => IncompatibleFieldMetadataSnafu {
                logical_type: "TIMESTAMP_TZ".to_string(),
                data_type: dt.clone(),
            }
            .fail(),
        },
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
        SnowflakeFieldType::Vector(snowflake_type) => {
            make_converter!(arrow::array::FixedSizeListArray, snowflake_type, nullable)
        }
    }
}

/// The `conciseSqlType` Arrow-metadata value that tags catalog SMALLINT columns
/// as ODBC `SQL_SMALLINT`. Single source of truth for the write side
/// (`catalog::catalog_key_seq_field`) and the read side (`sql_type_from_field`)
/// so the two cannot drift apart.
pub(crate) const SMALLINT_CONCISE_SQL_TYPE: i16 = odbc_sys::SqlDataType::SMALLINT.0;

/// The `conciseSqlType` Arrow-metadata value that tags catalog string columns
/// as ODBC `SQL_WVARCHAR` (−9), matching the reference driver catalog IRD.
pub(crate) const WVARCHAR_CONCISE_SQL_TYPE: i16 = odbc_sys::SqlDataType::EXT_W_VARCHAR.0;

/// The `conciseSqlType` Arrow-metadata value that tags catalog INTEGER columns
/// as ODBC `SQL_INTEGER`.
pub(crate) const INTEGER_CONCISE_SQL_TYPE: i16 = odbc_sys::SqlDataType::INTEGER.0;

/// Map a Snowflake Arrow field to the corresponding SQL data type.
pub fn sql_type_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<odbc_sys::SqlDataType, ConversionError> {
    // Catalog result schemas may override the concise type (e.g. WVARCHAR for
    // string catalog cols, SMALLINT/INTEGER for numeric catalog cols) while
    // keeping a physical Arrow type that is convenient to build. Query-result
    // TEXT fields never set this metadata and keep resolving to SQL_VARCHAR.
    if let Some(code) = field
        .metadata()
        .get("conciseSqlType")
        .and_then(|v| v.parse::<i16>().ok())
    {
        return Ok(odbc_sys::SqlDataType(code));
    }
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.sql_type())
}

/// Returns the verbose SQL data type (SQL_DESC_TYPE) for a field.
/// For date/time types this returns SQL_DATETIME (9) instead of the concise type.
/// For all other types, verbose == concise.
pub fn verbose_sql_type_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<odbc_sys::SqlDataType, ConversionError> {
    let concise = sql_type_from_field(field, numeric_settings)?;
    Ok(match concise {
        odbc_sys::SqlDataType::DATE
        | odbc_sys::SqlDataType::TIME
        | odbc_sys::SqlDataType::TIMESTAMP => odbc_sys::SqlDataType::DATETIME,
        other => other,
    })
}

#[cfg(test)]
mod concise_sql_type_override_tests {
    use super::{
        INTEGER_CONCISE_SQL_TYPE, NumericSettings, SMALLINT_CONCISE_SQL_TYPE,
        WVARCHAR_CONCISE_SQL_TYPE, sql_type_from_field,
    };
    use arrow::datatypes::{DataType, Field};
    use odbc_sys as sql;
    use std::collections::HashMap;

    fn text_field_with_concise(code: i16) -> Field {
        let metadata: HashMap<String, String> = [
            ("logicalType".to_string(), "TEXT".to_string()),
            ("charLength".to_string(), "255".to_string()),
            ("conciseSqlType".to_string(), code.to_string()),
        ]
        .into();
        Field::new("col", DataType::Utf8, true).with_metadata(metadata)
    }

    fn plain_text_field() -> Field {
        let metadata: HashMap<String, String> = [
            ("logicalType".to_string(), "TEXT".to_string()),
            ("charLength".to_string(), "255".to_string()),
        ]
        .into();
        Field::new("col", DataType::Utf8, true).with_metadata(metadata)
    }

    #[test]
    fn concise_sql_type_wvarchar_override() {
        let field = text_field_with_concise(WVARCHAR_CONCISE_SQL_TYPE);
        assert_eq!(
            sql_type_from_field(&field, &NumericSettings::default()).unwrap(),
            sql::SqlDataType::EXT_W_VARCHAR
        );
    }

    #[test]
    fn concise_sql_type_integer_override() {
        let field = text_field_with_concise(INTEGER_CONCISE_SQL_TYPE);
        assert_eq!(
            sql_type_from_field(&field, &NumericSettings::default()).unwrap(),
            sql::SqlDataType::INTEGER
        );
    }

    #[test]
    fn concise_sql_type_smallint_override() {
        let field = text_field_with_concise(SMALLINT_CONCISE_SQL_TYPE);
        assert_eq!(
            sql_type_from_field(&field, &NumericSettings::default()).unwrap(),
            sql::SqlDataType::SMALLINT
        );
    }

    #[test]
    fn plain_text_without_override_stays_varchar() {
        let field = plain_text_field();
        assert_eq!(
            sql_type_from_field(&field, &NumericSettings::default()).unwrap(),
            sql::SqlDataType::VARCHAR
        );
    }
}

#[cfg(test)]
mod unknown_logical_type_tests {
    use super::{NumericSettings, sql_type_from_field};
    use crate::conversion::error::ConversionError;
    use arrow::datatypes::{DataType, Field};
    use odbc_sys as sql;
    use std::collections::HashMap;

    fn field_with_logical_type(logical_type: &str, extra: &[(&str, &str)]) -> Field {
        let md: HashMap<String, String> = extra
            .iter()
            .copied()
            .chain(std::iter::once(("logicalType", logical_type)))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Field::new("col", DataType::Utf8, true).with_metadata(md)
    }

    #[test]
    fn geography_maps_to_sql_varchar() {
        let field = field_with_logical_type("GEOGRAPHY", &[]);
        assert_eq!(
            sql_type_from_field(&field, &NumericSettings::default()).unwrap(),
            sql::SqlDataType::VARCHAR
        );
    }

    #[test]
    fn invented_logical_type_maps_to_sql_varchar() {
        let field = field_with_logical_type("INTERVAL", &[]);
        assert_eq!(
            sql_type_from_field(&field, &NumericSettings::default()).unwrap(),
            sql::SqlDataType::VARCHAR
        );
    }

    #[test]
    fn empty_logical_type_still_errors() {
        let field = Field::new("col", DataType::Utf8, true);
        let err = sql_type_from_field(&field, &NumericSettings::default()).unwrap_err();
        assert!(
            matches!(err, ConversionError::IncompatibleFieldMetadata { .. }),
            "expected IncompatibleFieldMetadata for missing logicalType, got: {err}"
        );
    }

    #[test]
    fn unparseable_char_length_on_unknown_type_still_errors() {
        let field = field_with_logical_type("GEOGRAPHY", &[("charLength", "not_a_number")]);
        let err = sql_type_from_field(&field, &NumericSettings::default()).unwrap_err();
        assert!(
            matches!(err, ConversionError::FieldMetadataParsing { ref key, .. } if key == "charLength"),
            "expected FieldMetadataParsing for charLength, got: {err}"
        );
    }
}

pub fn column_size_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<odbc_sys::ULen, ConversionError> {
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.column_size())
}

pub fn precision_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<odbc_sys::ULen, ConversionError> {
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.precision())
}

pub fn decimal_digits_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<odbc_sys::SmallInt, ConversionError> {
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.decimal_digits())
}

pub fn type_name_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<&'static str, ConversionError> {
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.type_name())
}

pub fn display_size_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<odbc_sys::Len, ConversionError> {
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.display_size())
}

pub fn octet_length_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<odbc_sys::Len, ConversionError> {
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.octet_length())
}

pub fn num_prec_radix_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<odbc_sys::Len, ConversionError> {
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.num_prec_radix())
}

pub fn is_unsigned_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<bool, ConversionError> {
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.is_unsigned())
}

pub fn is_case_sensitive_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<bool, ConversionError> {
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.is_case_sensitive())
}

pub fn searchable_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<odbc_sys::Len, ConversionError> {
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.searchable())
}

pub fn literal_prefix_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<&'static str, ConversionError> {
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.literal_prefix())
}

pub fn literal_suffix_from_field(
    field: &Field,
    numeric_settings: &NumericSettings,
) -> Result<&'static str, ConversionError> {
    SnowflakeFieldType::from_field(field, numeric_settings).map(|ft| ft.literal_suffix())
}

#[cfg(test)]
mod number_char_batch_tests {
    use super::{ColumnConverter, NumericSettings, make_converter};
    use crate::api::CDataType;
    use crate::conversion::error::ConversionError;
    use crate::conversion::traits::{Binding, BindingStrides};
    use crate::conversion::warning::Warnings;
    use arrow::array::{Array, Decimal128Array, Int64Array};
    use arrow::datatypes::{DataType, Field};
    use odbc_sys as sql;
    use std::collections::HashMap;

    fn number_field(scale: &str, nullable: bool) -> Field {
        let mut md = HashMap::new();
        md.insert("logicalType".to_string(), "FIXED".to_string());
        md.insert("scale".to_string(), scale.to_string());
        md.insert("precision".to_string(), "18".to_string());
        Field::new("c", DataType::Int64, nullable).with_metadata(md)
    }

    fn decimal_field(precision: u8, scale: i8, nullable: bool) -> Field {
        let mut md = HashMap::new();
        md.insert("logicalType".to_string(), "FIXED".to_string());
        md.insert("scale".to_string(), (scale as i64).to_string());
        md.insert("precision".to_string(), (precision as i64).to_string());
        Field::new("c", DataType::Decimal128(precision, scale), nullable).with_metadata(md)
    }

    fn base(buf: &mut [u8], inds: &mut [sql::Len], cell: usize) -> Binding {
        Binding {
            target_type: CDataType::Char,
            target_value_ptr: buf.as_mut_ptr() as sql::Pointer,
            buffer_length: cell as sql::Len,
            octet_length_ptr: inds.as_mut_ptr(),
            indicator_ptr: inds.as_mut_ptr(),
            ..Default::default()
        }
    }

    /// Reference per-cell run, identical to `per_cell_convert_range`, using the
    /// wrapper's `convert_arrow_value` (which delegates to the generic converter).
    fn per_cell(
        conv: &dyn ColumnConverter,
        arr: &dyn arrow::array::Array,
        cell: usize,
        buf: &mut [u8],
        inds: &mut [sql::Len],
    ) -> Vec<Result<Warnings, ConversionError>> {
        let n = arr.len();
        let b = base(buf, inds, cell);
        let strides = BindingStrides {
            bind_type: 0,
            bind_offset: 0,
        };
        let mut outs: Vec<Result<Warnings, ConversionError>> =
            (0..n).map(|_| Ok(Vec::new())).collect();
        for (i, slot) in outs.iter_mut().enumerate() {
            if slot.is_err() {
                continue;
            }
            let rb = strides.for_row(&b, i).unwrap();
            match conv.convert_arrow_value(arr, i, &rb, &mut None) {
                Ok(w) => {
                    if !w.is_empty()
                        && let Ok(e) = slot
                    {
                        e.extend(w);
                    }
                }
                Err(e) => *slot = Err(e),
            }
        }
        outs
    }

    fn batched(
        conv: &dyn ColumnConverter,
        arr: &dyn arrow::array::Array,
        cell: usize,
        buf: &mut [u8],
        inds: &mut [sql::Len],
    ) -> Vec<Result<Warnings, ConversionError>> {
        let n = arr.len();
        let b = base(buf, inds, cell);
        let mut outs: Vec<Result<Warnings, ConversionError>> =
            (0..n).map(|_| Ok(Vec::new())).collect();
        conv.convert_arrow_range(
            arr,
            0..n,
            &b,
            0,
            BindingStrides {
                bind_type: 0,
                bind_offset: 0,
            },
            &mut outs,
        );
        outs
    }

    fn scrub(s: &str) -> String {
        // drop snafu `Location { .. }` spans so error comparison ignores capture site
        let mut out = String::new();
        let mut rest = s;
        while let Some(pos) = rest.find("Location {") {
            out.push_str(&rest[..pos]);
            rest = &rest[pos..];
            match rest.find('}') {
                Some(end) => rest = &rest[end + 1..],
                None => break,
            }
        }
        out.push_str(rest);
        out
    }

    fn assert_equiv(field: &Field, arr: &dyn arrow::array::Array, cell: usize) {
        let conv = make_converter(field, &NumericSettings::default()).unwrap();
        let n = arr.len();
        let (mut bp, mut ip) = (vec![0xAAu8; n * cell], vec![-9 as sql::Len; n]);
        let (mut bb, mut ib) = (vec![0xAAu8; n * cell], vec![-9 as sql::Len; n]);
        let op = per_cell(conv.as_ref(), arr, cell, &mut bp, &mut ip);
        let ob = batched(conv.as_ref(), arr, cell, &mut bb, &mut ib);
        assert_eq!(bp, bb, "value buffers differ");
        assert_eq!(ip, ib, "indicators differ");
        for (i, (a, b)) in op.iter().zip(ob.iter()).enumerate() {
            assert_eq!(
                scrub(&format!("{a:?}")),
                scrub(&format!("{b:?}")),
                "output {i}"
            );
        }
    }

    #[test]
    fn batched_matches_per_cell_nullable_with_nulls_negatives_and_truncation() {
        let arr = Int64Array::from(vec![
            Some(0),
            Some(101),
            Some(-4599),
            Some(12345),
            None,
            Some(99_999_999_999), // 12 chars formatted -> truncates in an 8-byte cell -> 22003
            Some(7),
        ]);
        assert_equiv(&number_field("2", true), &arr, 8);
        // wide cell: no truncation, exercises the pure fast path
        assert_equiv(&number_field("2", true), &arr, 64);
        // scale 0
        assert_equiv(&number_field("0", true), &arr, 64);
    }

    #[test]
    fn batched_matches_per_cell_non_nullable_no_nulls() {
        let arr = Int64Array::from(vec![0, 5, -12, 6789, 250]);
        assert_equiv(&number_field("2", false), &arr, 64);
    }

    #[test]
    fn batched_matches_per_cell_decimal128() {
        // Decimal128 is a real Snowflake NUMBER representation and is also wired
        // through the batched converter (Decimal128Type). Verify byte-identity
        // to the generic path, including the truncation/overflow branch.
        let arr = Decimal128Array::from(vec![
            Some(0i128),
            Some(101),
            Some(-4599),
            Some(1_234_567_890_123_456i128),
            None,
            Some(7),
        ])
        .with_precision_and_scale(38, 2)
        .unwrap();
        assert_equiv(&decimal_field(38, 2, true), &arr, 64);
        // small cell -> exercises truncation + whole-digits 22003 error path
        assert_equiv(&decimal_field(38, 2, true), &arr, 8);
    }

    /// Independent oracle: assert the batched path writes the exact bytes,
    /// NUL terminator, and indicators for known inputs — not just parity with
    /// the per-cell path. An equivalence-only test would pass even if a bug
    /// shared by both paths produced wrong-but-identical output.
    #[test]
    fn batched_writes_expected_text_indicators_and_nul() {
        let arr = Int64Array::from(vec![Some(0), Some(101), Some(-4599), None, Some(250)]);
        let conv = make_converter(&number_field("2", true), &NumericSettings::default()).unwrap();
        let cell = 16;
        let n = arr.len();
        let (mut buf, mut inds) = (vec![0xAAu8; n * cell], vec![-9 as sql::Len; n]);
        let outs = batched(conv.as_ref(), &arr, cell, &mut buf, &mut inds);

        // scale 2: value/100 with two fractional digits.
        let expected = ["0.00", "1.01", "-45.99", "", "2.50"];
        for (i, exp) in expected.iter().enumerate() {
            if arr.is_null(i) {
                assert_eq!(inds[i], -1, "row {i}: null cell must set SQL_NULL_DATA");
                continue;
            }
            let cell_bytes = &buf[i * cell..(i + 1) * cell];
            assert_eq!(&cell_bytes[..exp.len()], exp.as_bytes(), "row {i} text");
            assert_eq!(cell_bytes[exp.len()], 0, "row {i} NUL terminator");
            assert_eq!(inds[i], exp.len() as sql::Len, "row {i} indicator");
            assert!(
                outs[i].as_ref().unwrap().is_empty(),
                "row {i} unexpected warnings"
            );
        }
    }

    /// A non-nullable column that unexpectedly carries a null must be declined
    /// (return `false`) so the caller falls back to the generic per-cell path,
    /// rather than the batched path inventing a null indicator.
    #[test]
    fn batched_declines_non_nullable_column_carrying_nulls() {
        use arrow::datatypes::Int64Type;
        let arr = Int64Array::from(vec![Some(1), None, Some(3)]);
        let cell = 16;
        let n = arr.len();
        let (mut buf, mut inds) = (vec![0u8; n * cell], vec![0 as sql::Len; n]);
        let b = base(&mut buf, &mut inds, cell);
        let mut outs: Vec<Result<Warnings, ConversionError>> =
            (0..n).map(|_| Ok(Vec::new())).collect();
        let took = crate::conversion::number::convert_number_char_range::<Int64Type>(
            2,
            false, // non-nullable
            &arr,
            0..n,
            &b,
            0,
            BindingStrides {
                bind_type: 0,
                bind_offset: 0,
            },
            &mut outs,
        );
        assert!(
            !took,
            "must decline so the caller falls back to the generic path"
        );
    }

    /// A first-row binding-pointer overflow must be declined so overflow is
    /// still reported per row by the generic path, exactly as before.
    #[test]
    fn batched_declines_on_first_row_stride_overflow() {
        use arrow::datatypes::Int64Type;
        let arr = Int64Array::from(vec![Some(1), Some(2)]);
        let cell = 16;
        let n = arr.len();
        let (mut buf, mut inds) = (vec![0u8; n * cell], vec![0 as sql::Len; n]);
        let b = base(&mut buf, &mut inds, cell);
        let mut outs: Vec<Result<Warnings, ConversionError>> =
            (0..n).map(|_| Ok(Vec::new())).collect();
        // Row-wise stride of usize::MAX with a non-zero start row overflows the
        // very first `for_row` pointer computation.
        let took = crate::conversion::number::convert_number_char_range::<Int64Type>(
            2,
            true,
            &arr,
            0..n,
            &b,
            2, // out_row_start: 2 * usize::MAX overflows
            BindingStrides {
                bind_type: usize::MAX,
                bind_offset: 0,
            },
            &mut outs,
        );
        assert!(!took, "first-row stride overflow must decline");
    }

    /// `out_row_start != 0` (a mid-block sub-range) must offset where each row
    /// lands: range row `k` writes to bound slot `out_row_start + k`, leaving
    /// earlier slots untouched.
    #[test]
    fn batched_honors_out_row_start_offset() {
        use arrow::datatypes::Int64Type;
        let arr = Int64Array::from(vec![Some(11), Some(22)]);
        let cell = 8;
        let slots = 4;
        let (mut buf, mut inds) = (vec![0xAAu8; slots * cell], vec![-9 as sql::Len; slots]);
        let b = base(&mut buf, &mut inds, cell);
        let n = arr.len();
        let mut outs: Vec<Result<Warnings, ConversionError>> =
            (0..n).map(|_| Ok(Vec::new())).collect();
        let took = crate::conversion::number::convert_number_char_range::<Int64Type>(
            0, // scale 0
            true,
            &arr,
            0..n,
            &b,
            2, // out_row_start
            BindingStrides {
                bind_type: 0,
                bind_offset: 0,
            },
            &mut outs,
        );
        assert!(took);
        // Rows landed at slots 2 and 3.
        assert_eq!(&buf[2 * cell..2 * cell + 2], b"11");
        assert_eq!(&buf[3 * cell..3 * cell + 2], b"22");
        assert_eq!(inds[2], 2);
        assert_eq!(inds[3], 2);
        // Slots before out_row_start are untouched.
        assert!(
            buf[0..2 * cell].iter().all(|&x| x == 0xAA),
            "slots before out_row_start must be untouched"
        );
    }

    /// A row already carrying an error (as an earlier column in the same
    /// segment would leave it) must be skipped, not overwritten — the batched
    /// path preserves the "first error aborts the row" contract.
    #[test]
    fn batched_skips_rows_already_in_error() {
        use arrow::datatypes::Int64Type;
        let arr = Int64Array::from(vec![Some(1), Some(2), Some(3)]);
        let cell = 16;
        let n = arr.len();
        let (mut buf, mut inds) = (vec![0xAAu8; n * cell], vec![-9 as sql::Len; n]);
        let b = base(&mut buf, &mut inds, cell);
        let mut outs: Vec<Result<Warnings, ConversionError>> =
            (0..n).map(|_| Ok(Vec::new())).collect();
        // Pre-seed row 1 with an error, as an earlier column would.
        outs[1] = Err(crate::conversion::error::MissingFieldMetadataSnafu {
            key: "x".to_string(),
            field_name: "c".to_string(),
        }
        .build());

        let took = crate::conversion::number::convert_number_char_range::<Int64Type>(
            0,
            true,
            &arr,
            0..n,
            &b,
            0,
            BindingStrides {
                bind_type: 0,
                bind_offset: 0,
            },
            &mut outs,
        );
        assert!(took);
        assert!(outs[1].is_err(), "pre-existing error must be preserved");
        // The errored row's buffer slot is left untouched, neighbors converted.
        assert!(
            buf[cell..2 * cell].iter().all(|&x| x == 0xAA),
            "errored row buffer must be untouched"
        );
        assert_eq!(&buf[0..1], b"1");
        assert_eq!(&buf[2 * cell..2 * cell + 1], b"3");
    }
}
