use arrow::array::{Array, ArrowPrimitiveType, PrimitiveArray};

use crate::error::ReadArrowError;
use crate::traits::{ReadArrowType, SnowflakeType};

/// Snowflake FIXED (NUMBER / DECIMAL, and the integer types that map onto it).
///
/// The server sends FIXED as an Arrow integer (`Int8`..`Int64`) or a
/// `Decimal128`, always carrying the *unscaled* mantissa: the decoded value is
/// the raw two's-complement integer, and the column's `scale` — where the
/// decimal point sits — is applied by the front end at render time, not here.
/// So the reader needs no column metadata; every physical width that fits in
/// `i128` decodes through one generic impl to an `i128` mantissa. Whether that
/// mantissa is then rendered as a decimal string, reported as `SQL_BIGINT`, or
/// widened for a JavaScript number is the front end's decision.
pub struct SnowflakeFixed;

impl SnowflakeType for SnowflakeFixed {
    type Representation<'a> = i128;
}

impl<T: ArrowPrimitiveType> ReadArrowType<PrimitiveArray<T>> for SnowflakeFixed
where
    T::Native: Into<i128>,
{
    fn read_arrow_type<'a>(
        &self,
        array: &'a PrimitiveArray<T>,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        if array.is_null(row_idx) {
            return Err(ReadArrowError::NullValue {
                location: snafu::location!(),
            });
        }
        Ok(array.value(row_idx).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Decimal128Array, Int8Array, Int16Array, Int32Array, Int64Array};

    #[test]
    fn should_read_int64_mantissa() {
        let array = Int64Array::from(vec![Some(9_223_372_036_854_775_807)]);
        assert_eq!(
            SnowflakeFixed.read_arrow_type(&array, 0).unwrap(),
            9_223_372_036_854_775_807i128
        );
    }

    #[test]
    fn should_read_narrower_integer_widths() {
        let i8s = Int8Array::from(vec![Some(-128)]);
        let i16s = Int16Array::from(vec![Some(32_767)]);
        let i32s = Int32Array::from(vec![Some(-2_000_000_000)]);
        assert_eq!(SnowflakeFixed.read_arrow_type(&i8s, 0).unwrap(), -128i128);
        assert_eq!(
            SnowflakeFixed.read_arrow_type(&i16s, 0).unwrap(),
            32_767i128
        );
        assert_eq!(
            SnowflakeFixed.read_arrow_type(&i32s, 0).unwrap(),
            -2_000_000_000i128
        );
    }

    /// The decode returns the raw stored mantissa and ignores the Arrow array's
    /// own declared scale — scale is applied later, by the front end, from the
    /// Snowflake column metadata. A `Decimal128` holding `12345` decodes to
    /// `12345`, not `12.345`, regardless of the array's precision/scale.
    #[test]
    fn should_read_decimal128_as_unscaled_mantissa() {
        let array = Decimal128Array::from(vec![Some(12_345i128)])
            .with_precision_and_scale(38, 3)
            .unwrap();
        assert_eq!(
            SnowflakeFixed.read_arrow_type(&array, 0).unwrap(),
            12_345i128
        );
    }

    #[test]
    fn should_read_full_i128_range_decimal() {
        let array = Decimal128Array::from(vec![Some(i128::MIN), Some(i128::MAX)]);
        assert_eq!(
            SnowflakeFixed.read_arrow_type(&array, 0).unwrap(),
            i128::MIN
        );
        assert_eq!(
            SnowflakeFixed.read_arrow_type(&array, 1).unwrap(),
            i128::MAX
        );
    }

    #[test]
    fn should_report_null_cell_as_null_value_error() {
        let array = Int64Array::from(vec![None, Some(1)]);
        let err = SnowflakeFixed.read_arrow_type(&array, 0).unwrap_err();
        assert!(
            matches!(err, ReadArrowError::NullValue { .. }),
            "got {err:?}"
        );
    }
}
