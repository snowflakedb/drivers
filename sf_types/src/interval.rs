//! Decode-only readers for Snowflake INTERVAL results.
//!
//! The server encodes an interval as a single signed integer, not a broken-down
//! struct: `INTERVAL_YEAR_MONTH` arrives as a total month count and
//! `INTERVAL_DAY_TIME` as a total nanosecond count. Either can be physically
//! `Int32`, `Int64`, or `Decimal128` depending on magnitude, so both readers
//! decode through one generic `PrimitiveArray<T>` impl. Decomposition into
//! years/months or days/hours/minutes/seconds and any textual rendering is
//! per-front-end presentation and stays in the driver crates; the column's
//! `scale` metadata governs only that rendering, never the stored unit.

use arrow::array::{Array, PrimitiveArray};
use arrow::datatypes::ArrowPrimitiveType;

use crate::error::ReadArrowError;
use crate::traits::{ReadArrowType, SnowflakeType};

/// Snowflake INTERVAL YEAR TO MONTH, decoded as a signed total month count.
pub struct SnowflakeIntervalYearMonth;

impl SnowflakeType for SnowflakeIntervalYearMonth {
    type Representation<'a> = i128;
}

impl<T: ArrowPrimitiveType> ReadArrowType<PrimitiveArray<T>> for SnowflakeIntervalYearMonth
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

/// Snowflake INTERVAL DAY TO SECOND, decoded as a signed total nanosecond count.
pub struct SnowflakeIntervalDayTime;

impl SnowflakeType for SnowflakeIntervalDayTime {
    type Representation<'a> = i128;
}

impl<T: ArrowPrimitiveType> ReadArrowType<PrimitiveArray<T>> for SnowflakeIntervalDayTime
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
    use arrow::array::{Decimal128Array, Int32Array, Int64Array};

    #[test]
    fn year_month_reads_int64() {
        let array = Int64Array::from(vec![Some(27)]);
        assert_eq!(
            SnowflakeIntervalYearMonth
                .read_arrow_type(&array, 0)
                .unwrap(),
            27
        );
    }

    #[test]
    fn year_month_reads_int32() {
        let array = Int32Array::from(vec![Some(-15)]);
        assert_eq!(
            SnowflakeIntervalYearMonth
                .read_arrow_type(&array, 0)
                .unwrap(),
            -15
        );
    }

    #[test]
    fn year_month_reads_decimal128() {
        let array = Decimal128Array::from(vec![Some(1_000_000_000_000i128)]);
        assert_eq!(
            SnowflakeIntervalYearMonth
                .read_arrow_type(&array, 0)
                .unwrap(),
            1_000_000_000_000
        );
    }

    #[test]
    fn year_month_reads_zero() {
        let array = Int64Array::from(vec![Some(0)]);
        assert_eq!(
            SnowflakeIntervalYearMonth
                .read_arrow_type(&array, 0)
                .unwrap(),
            0
        );
    }

    #[test]
    fn year_month_null_is_null_value_error() {
        let array = Int64Array::from(vec![None, Some(1)]);
        let err = SnowflakeIntervalYearMonth
            .read_arrow_type(&array, 0)
            .unwrap_err();
        assert!(
            matches!(err, ReadArrowError::NullValue { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn day_time_reads_int64() {
        let array = Int64Array::from(vec![Some(93_784_500_000_000)]);
        assert_eq!(
            SnowflakeIntervalDayTime.read_arrow_type(&array, 0).unwrap(),
            93_784_500_000_000
        );
    }

    #[test]
    fn day_time_reads_negative_int64() {
        let array = Int64Array::from(vec![Some(-1)]);
        assert_eq!(
            SnowflakeIntervalDayTime.read_arrow_type(&array, 0).unwrap(),
            -1
        );
    }

    #[test]
    fn day_time_reads_i64_boundaries() {
        let array = Int64Array::from(vec![Some(i64::MAX), Some(i64::MIN)]);
        assert_eq!(
            SnowflakeIntervalDayTime.read_arrow_type(&array, 0).unwrap(),
            i64::MAX as i128
        );
        assert_eq!(
            SnowflakeIntervalDayTime.read_arrow_type(&array, 1).unwrap(),
            i64::MIN as i128
        );
    }

    #[test]
    fn day_time_reads_decimal128_beyond_i64() {
        let beyond = i64::MAX as i128 + 1;
        let array = Decimal128Array::from(vec![Some(beyond)]);
        assert_eq!(
            SnowflakeIntervalDayTime.read_arrow_type(&array, 0).unwrap(),
            beyond
        );
    }

    #[test]
    fn day_time_reads_zero() {
        let array = Int64Array::from(vec![Some(0)]);
        assert_eq!(
            SnowflakeIntervalDayTime.read_arrow_type(&array, 0).unwrap(),
            0
        );
    }

    #[test]
    fn day_time_null_is_null_value_error() {
        let array = Int64Array::from(vec![None, Some(1)]);
        let err = SnowflakeIntervalDayTime
            .read_arrow_type(&array, 0)
            .unwrap_err();
        assert!(
            matches!(err, ReadArrowError::NullValue { .. }),
            "got {err:?}"
        );
    }
}
