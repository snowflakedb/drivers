use arrow::array::{Array, BinaryArray, Int16Array, StructArray};

use crate::error::{InvalidArrowValueSnafu, ReadArrowError};
use crate::traits::{ReadArrowType, SnowflakeType};

/// Snowflake DECFLOAT.
///
/// The server sends DECFLOAT as an Arrow struct of two children: an `Int16`
/// `exponent` and a big-endian two's-complement `significand` (a `Binary`
/// column). The decoded value is the pair `(significand, exponent)` — a
/// base-10 `significand * 10^exponent` — with no rendering applied. Turning
/// that pair into a decimal or scientific-notation string is presentation
/// policy that differs between front ends, so it stays in each wrapper.
pub struct SnowflakeDecfloat;

impl SnowflakeType for SnowflakeDecfloat {
    type Representation<'a> = (i128, i16);
}

/// Converts a big-endian two's complement byte slice (1–16 bytes) into an i128.
/// The Arrow wire format trims leading bytes, so we sign-extend to 16 bytes
/// before calling `i128::from_be_bytes`. Empty input is treated as zero.
fn i128_from_big_endian_signed(bytes: &[u8]) -> Result<i128, ReadArrowError> {
    if bytes.is_empty() {
        return Ok(0);
    }
    if bytes.len() > 16 {
        return InvalidArrowValueSnafu {
            reason: format!(
                "significand byte length {} exceeds maximum of 16",
                bytes.len()
            ),
        }
        .fail();
    }
    let sign_bytes = if bytes[0] & 0x80 != 0 { 0xFF } else { 0x00 };
    let mut buf = [sign_bytes; 16];
    buf[16 - bytes.len()..].copy_from_slice(bytes);
    Ok(i128::from_be_bytes(buf))
}

#[derive(Debug)]
pub struct DecfloatColumn<'a> {
    exponent: &'a Int16Array,
    significand: &'a BinaryArray,
}

impl<'a> DecfloatColumn<'a> {
    pub fn try_new(array: &'a StructArray) -> Result<Self, ReadArrowError> {
        let exponent = array
            .column_by_name("exponent")
            .ok_or_else(|| ReadArrowError::InvalidArrowValue {
                reason: "DECFLOAT struct missing 'exponent' field".to_string(),
                location: snafu::location!(),
            })?
            .as_any()
            .downcast_ref::<Int16Array>()
            .ok_or_else(|| ReadArrowError::InvalidArrowValue {
                reason: "DECFLOAT 'exponent' field is not Int16".to_string(),
                location: snafu::location!(),
            })?;

        let significand = array
            .column_by_name("significand")
            .ok_or_else(|| ReadArrowError::InvalidArrowValue {
                reason: "DECFLOAT struct missing 'significand' field".to_string(),
                location: snafu::location!(),
            })?
            .as_any()
            .downcast_ref::<BinaryArray>()
            .ok_or_else(|| ReadArrowError::InvalidArrowValue {
                reason: "DECFLOAT 'significand' field is not Binary".to_string(),
                location: snafu::location!(),
            })?;

        Ok(Self {
            exponent,
            significand,
        })
    }

    pub fn value(&self, row_idx: usize) -> Result<(i128, i16), ReadArrowError> {
        let exponent = self.exponent.value(row_idx);
        let significand = i128_from_big_endian_signed(self.significand.value(row_idx))?;
        Ok((significand, exponent))
    }
}

impl ReadArrowType<StructArray> for SnowflakeDecfloat {
    fn read_arrow_type<'a>(
        &self,
        array: &'a StructArray,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        if array.is_null(row_idx) {
            return Err(ReadArrowError::NullValue {
                location: snafu::location!(),
            });
        }

        DecfloatColumn::try_new(array)?.value(row_idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::buffer::NullBuffer;
    use arrow::datatypes::{DataType, Field};
    use std::sync::Arc;

    #[test]
    fn i128_single_byte_positive() {
        assert_eq!(i128_from_big_endian_signed(&[0x01]).unwrap(), 1);
    }

    #[test]
    fn i128_single_byte_negative() {
        assert_eq!(i128_from_big_endian_signed(&[0xFF]).unwrap(), -1);
    }

    #[test]
    fn i128_single_byte_zero() {
        assert_eq!(i128_from_big_endian_signed(&[0x00]).unwrap(), 0);
    }

    #[test]
    fn i128_empty_bytes_is_zero() {
        assert_eq!(i128_from_big_endian_signed(&[]).unwrap(), 0);
    }

    #[test]
    fn i128_single_byte_max_positive() {
        assert_eq!(i128_from_big_endian_signed(&[0x7F]).unwrap(), 127);
    }

    #[test]
    fn i128_single_byte_min_negative() {
        assert_eq!(i128_from_big_endian_signed(&[0x80]).unwrap(), -128);
    }

    #[test]
    fn i128_two_bytes() {
        assert_eq!(i128_from_big_endian_signed(&[0x01, 0x00]).unwrap(), 256);
    }

    #[test]
    fn i128_two_bytes_negative() {
        assert_eq!(i128_from_big_endian_signed(&[0xFF, 0x00]).unwrap(), -256);
    }

    #[test]
    fn i128_large_positive_trimmed() {
        let bytes = 123456789i128.to_be_bytes();
        let trimmed = &bytes[bytes.iter().position(|&b| b != 0).unwrap_or(15)..];
        assert_eq!(i128_from_big_endian_signed(trimmed).unwrap(), 123456789);
    }

    #[test]
    fn i128_full_16_bytes_positive() {
        let val: i128 = 12345678901234567890;
        assert_eq!(
            i128_from_big_endian_signed(&val.to_be_bytes()).unwrap(),
            val
        );
    }

    #[test]
    fn i128_full_16_bytes_negative() {
        let val: i128 = -12345678901234567890;
        assert_eq!(
            i128_from_big_endian_signed(&val.to_be_bytes()).unwrap(),
            val
        );
    }

    #[test]
    fn i128_38_digit_significand() {
        let val: i128 = 12345678901234567890123456789012345678;
        assert_eq!(
            i128_from_big_endian_signed(&val.to_be_bytes()).unwrap(),
            val
        );
    }

    #[test]
    fn i128_max_value() {
        assert_eq!(
            i128_from_big_endian_signed(&i128::MAX.to_be_bytes()).unwrap(),
            i128::MAX
        );
    }

    #[test]
    fn i128_min_value() {
        assert_eq!(
            i128_from_big_endian_signed(&i128::MIN.to_be_bytes()).unwrap(),
            i128::MIN
        );
    }

    #[test]
    fn i128_exceeds_16_bytes_is_invalid() {
        let err = i128_from_big_endian_signed(&[0u8; 17]).unwrap_err();
        assert!(
            matches!(err, ReadArrowError::InvalidArrowValue { .. }),
            "got {err:?}"
        );
    }

    fn decfloat_struct(
        exponents: Vec<Option<i16>>,
        significands: Vec<Option<&[u8]>>,
    ) -> StructArray {
        let nulls: Vec<bool> = exponents.iter().map(Option::is_some).collect();
        let fields = vec![
            Field::new("exponent", DataType::Int16, true),
            Field::new("significand", DataType::Binary, true),
        ];
        StructArray::try_new(
            fields.into(),
            vec![
                Arc::new(Int16Array::from(exponents)),
                Arc::new(BinaryArray::from(significands)),
            ],
            Some(NullBuffer::from(nulls)),
        )
        .unwrap()
    }

    #[test]
    fn should_read_significand_and_exponent() {
        let array = decfloat_struct(vec![Some(-3)], vec![Some(&[0x01, 0xe2, 0x40])]);
        assert_eq!(
            SnowflakeDecfloat.read_arrow_type(&array, 0).unwrap(),
            (123456, -3)
        );
    }

    #[test]
    fn should_read_empty_significand_as_zero() {
        let array = decfloat_struct(vec![Some(5)], vec![Some(&[])]);
        assert_eq!(
            SnowflakeDecfloat.read_arrow_type(&array, 0).unwrap(),
            (0, 5)
        );
    }

    #[test]
    fn should_report_null_row_as_null_value_error() {
        let array = decfloat_struct(vec![None, Some(0)], vec![None, Some(&[0x01])]);
        let err = SnowflakeDecfloat.read_arrow_type(&array, 0).unwrap_err();
        assert!(
            matches!(err, ReadArrowError::NullValue { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn should_report_missing_significand_field_as_invalid() {
        let fields = vec![Field::new("exponent", DataType::Int16, true)];
        let array = StructArray::try_new(
            fields.into(),
            vec![Arc::new(Int16Array::from(vec![Some(0)])) as _],
            None,
        )
        .unwrap();
        let err = SnowflakeDecfloat.read_arrow_type(&array, 0).unwrap_err();
        assert!(
            matches!(err, ReadArrowError::InvalidArrowValue { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn decfloat_column_reads_every_row_after_a_single_try_new() {
        let array = decfloat_struct(
            vec![Some(-3), Some(5), None],
            vec![Some(&[0x01, 0xe2, 0x40]), Some(&[]), None],
        );
        let column = DecfloatColumn::try_new(&array).unwrap();
        assert_eq!(column.value(0).unwrap(), (123456, -3));
        assert_eq!(column.value(1).unwrap(), (0, 5));
    }

    #[test]
    fn decfloat_column_try_new_reports_missing_exponent_field_as_invalid() {
        let fields = vec![Field::new("significand", DataType::Binary, true)];
        let array = StructArray::try_new(
            fields.into(),
            vec![Arc::new(BinaryArray::from(vec![Some(&[0x01][..])])) as _],
            None,
        )
        .unwrap();
        let err = DecfloatColumn::try_new(&array).unwrap_err();
        assert!(
            matches!(err, ReadArrowError::InvalidArrowValue { .. }),
            "got {err:?}"
        );
    }
}
