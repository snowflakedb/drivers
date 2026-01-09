// mod readers;
mod error;
mod traits;

mod number;
mod varchar;

use arrow::array::Array;
use arrow::datatypes::{
    DataType, Decimal128Type, Field, Int8Type, Int16Type, Int32Type, Int64Type,
};
pub use traits::{Binding, ReadArrowType, SnowflakeType, WriteODBCType};

pub use error::{
    ArrowArrayDowncastSnafu, ConversionError, FieldMetadataParsingSnafu, MissingFieldMetadataSnafu,
};

use crate::conversion::error::{IncompatibleFieldMetadataSnafu, UnsupportedArrowDataTypeSnafu};

pub trait Converter<'a> {
    fn convert_arrow_value(&self, row_idx: usize, binding: &Binding)
    -> Result<(), ConversionError>;
}

struct GenericConverter<'a, ArrowArrayType, T> {
    snowflake_type: T,
    arrow_array: &'a ArrowArrayType,
}

impl<'a, ArrowArrayType, T: SnowflakeType + WriteODBCType + ReadArrowType<ArrowArrayType>>
    Converter<'a> for GenericConverter<'a, ArrowArrayType, T>
{
    fn convert_arrow_value(
        &self,
        row_idx: usize,
        binding: &Binding,
    ) -> Result<(), ConversionError> {
        let value = self
            .snowflake_type
            .read_arrow_type(self.arrow_array, row_idx)?;
        self.snowflake_type.write_odbc_type(value, binding)
    }
}

macro_rules! make_converter {
    ($arrow_array_type:ty, $snowflake_type:expr, $arrow_array:expr) => {{
        let arrow_array = $arrow_array
            .as_any()
            .downcast_ref::<$arrow_array_type>()
            .ok_or(
                ArrowArrayDowncastSnafu {
                    expected_type: stringify!($arrow_array_type).to_string(),
                }
                .build(),
            )?;
        Ok(Box::new(GenericConverter {
            snowflake_type: $snowflake_type,
            arrow_array,
        }))
    }};
}

macro_rules! make_number_converter {
    ($arrow_type:ty, $snowflake_type:expr, $arrow_array:expr) => {{
        make_converter!(
            arrow::array::PrimitiveArray<$arrow_type>,
            $snowflake_type,
            $arrow_array
        )
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

pub fn make_converter<'a>(
    field: &Field,
    arrow_array: &'a dyn Array,
) -> Result<Box<dyn Converter<'a> + 'a>, ConversionError> {
    let logical_type = field
        .metadata()
        .get("logicalType")
        .map(|s| s.as_str())
        .unwrap_or("");
    match (logical_type, field.data_type()) {
        ("TEXT", DataType::Utf8) => {
            let len = get_field_metadata(field, "charLength")?;
            let snowflake_type = varchar::SnowflakeVarchar { len };
            make_converter!(
                arrow::array::GenericByteArray<arrow::datatypes::Utf8Type>,
                snowflake_type,
                arrow_array
            )
        }
        ("FIXED", _) => {
            let scale = get_field_metadata(field, "scale")?;
            let precision = get_field_metadata(field, "precision")?;
            let snowflake_type = number::SnowflakeNumber { scale, precision };
            match field.data_type() {
                DataType::Int8 => make_number_converter!(Int8Type, snowflake_type, arrow_array),
                DataType::Int16 => make_number_converter!(Int16Type, snowflake_type, arrow_array),
                DataType::Int32 => make_number_converter!(Int32Type, snowflake_type, arrow_array),
                DataType::Int64 => make_number_converter!(Int64Type, snowflake_type, arrow_array),
                DataType::Decimal128(_, _) => {
                    make_number_converter!(Decimal128Type, snowflake_type, arrow_array)
                }
                dt => UnsupportedArrowDataTypeSnafu {
                    data_type: dt.clone(),
                }
                .fail(),
            }
        }
        (lt, dt) => IncompatibleFieldMetadataSnafu {
            logical_type: lt.to_string(),
            data_type: dt.clone(),
        }
        .fail(),
    }
}
