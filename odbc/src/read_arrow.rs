use arrow::{
    array::{Array, ArrowPrimitiveType, GenericByteArray, PrimitiveArray},
    datatypes::{
        ByteArrayType, DataType, Field, Int8Type, Int16Type, Int32Type, Int64Type, Utf8Type,
    },
};
use odbc_sys as sql;

use crate::cdata_types::{CDataType, Double, SBigInt, UBigInt};

#[derive(Debug)]
#[allow(dead_code)]
pub enum ExtractError {
    UnsupportedArrowType(DataType),
    UnsupportedTargetType(CDataType),
    DowncastError,
    ErrorParsingFieldMeta(Box<Field>, String),
    UnsupportedFieldMeta(FieldMeta, DataType),
    ConversionError(String),
}

fn get_value<T: ArrowPrimitiveType>(
    array: &dyn Array,
    row_idx: usize,
) -> Result<T::Native, ExtractError> {
    Ok(array
        .as_any()
        .downcast_ref::<PrimitiveArray<T>>()
        .ok_or(ExtractError::DowncastError)?
        .value(row_idx))
}

fn get_byte_array_value<T: ByteArrayType>(
    array: &dyn Array,
    row_idx: usize,
) -> Result<&T::Native, ExtractError> {
    Ok(array
        .as_any()
        .downcast_ref::<GenericByteArray<T>>()
        .unwrap()
        .value(row_idx))
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum FieldMeta {
    Fixed { scale: u32, precision: u32 },
    Text,
    None,
}

fn get_field_meta(field: &Field) -> Result<FieldMeta, ExtractError> {
    let metadata = field.metadata();
    let default_value = "NONE".to_string();
    let logical_type = metadata.get("logicalType").unwrap_or(&default_value);

    match logical_type.as_str() {
        "FIXED" => {
            let scale = metadata
                .get("scale")
                .ok_or(ExtractError::ErrorParsingFieldMeta(
                    Box::new(field.clone()),
                    "scale not found".to_string(),
                ))?;
            let precision =
                metadata
                    .get("precision")
                    .ok_or(ExtractError::ErrorParsingFieldMeta(
                        Box::new(field.clone()),
                        "precision not found".to_string(),
                    ))?;
            Ok(FieldMeta::Fixed {
                scale: scale.parse::<u32>().map_err(|_| {
                    ExtractError::ErrorParsingFieldMeta(
                        Box::new(field.clone()),
                        "scale not a valid u32".to_string(),
                    )
                })?,
                precision: precision.parse::<u32>().map_err(|_| {
                    ExtractError::ErrorParsingFieldMeta(
                        Box::new(field.clone()),
                        "precision not a valid u32".to_string(),
                    )
                })?,
            })
        }
        "TEXT" => Ok(FieldMeta::Text),
        _ => {
            tracing::warn!("Unknown logicalType: {}", logical_type.as_str());
            Ok(FieldMeta::None)
        }
    }
}

pub trait ReadArrowValue<T>: Sized {
    fn read(self, array: &dyn Array, field: &Field, row_idx: usize) -> Result<(), ExtractError> {
        match array.data_type() {
            DataType::Int16 => self.read_int16(
                &get_field_meta(field)?,
                get_value::<Int16Type>(array, row_idx)?,
            ),
            DataType::Int32 => self.read_int32(
                &get_field_meta(field)?,
                get_value::<Int32Type>(array, row_idx)?,
            ),
            DataType::Int8 => self.read_int8(
                &get_field_meta(field)?,
                get_value::<Int8Type>(array, row_idx)?,
            ),
            DataType::Int64 => self.read_int64(
                &get_field_meta(field)?,
                get_value::<Int64Type>(array, row_idx)?,
            ),
            DataType::Utf8 => self.read_utf8(
                &get_field_meta(field)?,
                get_byte_array_value::<Utf8Type>(array, row_idx)?,
            ),
            _ => Err(ExtractError::UnsupportedArrowType(
                array.data_type().clone(),
            )),
        }
    }
    fn read_int8(self, _field: &FieldMeta, _value: i8) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Int8))
    }
    fn read_int16(self, _field: &FieldMeta, _value: i16) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Int16))
    }
    fn read_int32(self, _field: &FieldMeta, _value: i32) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Int32))
    }
    fn read_int64(self, _field: &FieldMeta, _value: i64) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Int64))
    }
    fn read_utf8(self, _field: &FieldMeta, _value: &str) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Utf8))
    }
}

impl<V: WriteValue<UBigInt>> ReadArrowValue<UBigInt> for V {
    fn read_int8(self, field: &FieldMeta, value: i8) -> Result<(), ExtractError> {
        read_u64(self, field, value as u64)
    }
    fn read_int16(self, field: &FieldMeta, value: i16) -> Result<(), ExtractError> {
        read_u64(self, field, value as u64)
    }
    fn read_int32(self, field: &FieldMeta, value: i32) -> Result<(), ExtractError> {
        read_u64(self, field, value as u64)
    }
    fn read_int64(self, field: &FieldMeta, value: i64) -> Result<(), ExtractError> {
        read_u64(self, field, value as u64)
    }
}

pub trait WriteValue<T> {
    fn write(&self, value: T);
}

pub struct Contramap<V, T, U>
where
    V: WriteValue<T>,
{
    pub value: V,
    pub f: fn(U) -> T,
}

impl<V, T, U> Contramap<V, T, U>
where
    V: WriteValue<T>,
{
    pub fn new(value: V, f: fn(U) -> T) -> Self {
        Self { value, f }
    }
}

impl<V, T, U> WriteValue<U> for Contramap<V, T, U>
where
    V: WriteValue<T>,
{
    fn write(&self, value: U) {
        self.value.write((self.f)(value))
    }
}

pub struct Value<T> {
    pub value: *mut T,
}

impl<T> WriteValue<T> for Value<T> {
    fn write(&self, value: T) {
        unsafe { std::ptr::write(self.value, value) };
    }
}

impl<T> Value<T> {
    pub fn new(value: *mut T) -> Self {
        Self { value }
    }
    pub fn contramap<U>(self, f: fn(U) -> T) -> Contramap<Self, T, U> {
        Contramap::new(self, f)
    }
}

pub struct Buffer<T> {
    pub data: *mut T,
    pub len: usize,
    pub str_len_or_ind: *mut sql::Len,
}

impl WriteValue<&str> for Buffer<sql::Char> {
    fn write(&self, value: &str) {
        if !self.str_len_or_ind.is_null() {
            unsafe { std::ptr::write(self.str_len_or_ind, value.len() as sql::Len) };
        }
        unsafe {
            std::ptr::copy_nonoverlapping(
                value.as_ptr() as *const sql::Char,
                self.data,
                std::cmp::min(self.len, value.len()),
            )
        };
    }
}

impl<T> Buffer<T> {
    pub fn new(data: *mut T, len: usize, str_len_or_ind: *mut sql::Len) -> Self {
        Self {
            data,
            len,
            str_len_or_ind,
        }
    }
}

fn decimal_to_string(value: i64, scale: u32) -> String {
    if scale == 0 {
        return value.to_string();
    }

    let scale_dec = 10_i64.pow(scale);
    let whole = value / scale_dec;
    let decimal = value % scale_dec;
    format!(
        "{}.{:0width$}",
        whole,
        decimal.abs(),
        width = scale as usize
    )
}

fn drop_decimal_digits_i64(value: i64, scale: u32) -> i64 {
    if scale == 0 {
        return value;
    }
    let scale_dec = 10_i64.pow(scale);
    value / scale_dec
}

fn drop_decimal_digits_u64(value: u64, scale: u32) -> u64 {
    if scale == 0 {
        return value;
    }
    let scale_dec = 10_u64.pow(scale);
    value / scale_dec
}

impl ReadArrowValue<&str> for Buffer<sql::Char> {
    fn read_utf8(self, _field: &FieldMeta, value: &str) -> Result<(), ExtractError> {
        self.write(value);
        Ok(())
    }

    fn read_int8(self, field: &FieldMeta, value: i8) -> Result<(), ExtractError> {
        self.read_int64(field, value as i64)
    }

    fn read_int16(self, field: &FieldMeta, value: i16) -> Result<(), ExtractError> {
        self.read_int64(field, value as i64)
    }

    fn read_int32(self, field: &FieldMeta, value: i32) -> Result<(), ExtractError> {
        self.read_int64(field, value as i64)
    }

    fn read_int64(self, field: &FieldMeta, value: i64) -> Result<(), ExtractError> {
        if let FieldMeta::Fixed { scale, .. } = field {
            self.read_utf8(field, decimal_to_string(value, *scale).as_str())
        } else {
            Err(ExtractError::UnsupportedFieldMeta(
                field.clone(),
                DataType::Int64,
            ))
        }
    }
}

fn read_u64<V: WriteValue<UBigInt>>(
    sink: V,
    field: &FieldMeta,
    value: u64,
) -> Result<(), ExtractError> {
    if let FieldMeta::Fixed { scale, .. } = field {
        sink.write(drop_decimal_digits_u64(value, *scale) as UBigInt);
        Ok(())
    } else {
        Err(ExtractError::UnsupportedFieldMeta(
            field.clone(),
            DataType::Int64,
        ))
    }
}

fn read_i64<V: WriteValue<SBigInt>>(
    sink: V,
    field: &FieldMeta,
    value: i64,
) -> Result<(), ExtractError> {
    if let FieldMeta::Fixed { scale, .. } = field {
        sink.write(drop_decimal_digits_i64(value, *scale) as SBigInt);
        Ok(())
    } else {
        Err(ExtractError::UnsupportedFieldMeta(
            field.clone(),
            DataType::Int64,
        ))
    }
}

fn read_f64<V: WriteValue<Double>>(
    sink: V,
    field: &FieldMeta,
    value: i64,
) -> Result<(), ExtractError> {
    if let FieldMeta::Fixed { scale, .. } = field {
        // TODO: Don't parse to string, parse to f64 directly
        let value = decimal_to_string(value, *scale);
        sink.write(
            value
                .parse::<Double>()
                .map_err(|_| ExtractError::ConversionError("value not a valid f64".to_string()))?,
        );
        Ok(())
    } else {
        Err(ExtractError::UnsupportedFieldMeta(
            field.clone(),
            DataType::Int64,
        ))
    }
}

impl<V: WriteValue<SBigInt>> ReadArrowValue<SBigInt> for V {
    fn read_int8(self, _field: &FieldMeta, value: i8) -> Result<(), ExtractError> {
        read_i64(self, _field, value as i64)
    }
    fn read_int16(self, _field: &FieldMeta, value: i16) -> Result<(), ExtractError> {
        read_i64(self, _field, value as i64)
    }
    fn read_int32(self, _field: &FieldMeta, value: i32) -> Result<(), ExtractError> {
        read_i64(self, _field, value as i64)
    }
    fn read_int64(self, field: &FieldMeta, value: i64) -> Result<(), ExtractError> {
        read_i64(self, field, value)
    }
}

impl<V: WriteValue<Double>> ReadArrowValue<Double> for V {
    fn read_int8(self, _field: &FieldMeta, value: i8) -> Result<(), ExtractError> {
        read_f64(self, _field, value as i64)?;
        Ok(())
    }
    fn read_int16(self, _field: &FieldMeta, value: i16) -> Result<(), ExtractError> {
        read_f64(self, _field, value as i64)?;
        Ok(())
    }
    fn read_int32(self, _field: &FieldMeta, value: i32) -> Result<(), ExtractError> {
        read_f64(self, _field, value as i64)?;
        Ok(())
    }
    fn read_int64(self, field: &FieldMeta, value: i64) -> Result<(), ExtractError> {
        read_f64(self, field, value)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decimal_to_string() {
        assert_eq!(decimal_to_string(12345, 2), "123.45");
        assert_eq!(decimal_to_string(-12345, 2), "-123.45");
        assert_eq!(decimal_to_string(12345, 3), "12.345");
        assert_eq!(decimal_to_string(1000, 3), "1.000");
        assert_eq!(decimal_to_string(0, 2), "0.00");
        assert_eq!(decimal_to_string(-12304, 2), "-123.04");
    }
}
