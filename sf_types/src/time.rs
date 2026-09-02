use arrow::array::{Array, PrimitiveArray};
use arrow::datatypes::ArrowPrimitiveType;
use chrono::NaiveTime;
use snafu::OptionExt;

use crate::clock::split_time_raw;
use crate::error::{InvalidArrowValueSnafu, ReadArrowError};
use crate::traits::{ReadArrowType, SnowflakeType};

/// Snowflake TIME. Decodes Arrow `Int32`/`Int64` as [`NaiveTime`].
///
/// Unlike DATE, the wire integer is not a time of day without `scale`, so the
/// reader carries it (parsed once per column from field metadata).
pub struct SnowflakeTime {
    /// Sub-second scale (0..=9). Public so front ends can fill it from Arrow metadata.
    pub scale: u32,
}

impl SnowflakeType for SnowflakeTime {
    type Representation<'a> = NaiveTime;
}

impl<T: ArrowPrimitiveType> ReadArrowType<PrimitiveArray<T>> for SnowflakeTime
where
    T::Native: Into<i64>,
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
        // One split for the materializer and the bulk-CHAR hot path.
        let raw: i64 = array.value(row_idx).into();
        split_time_raw(raw, self.scale)
            .and_then(|(secs, nanos)| NaiveTime::from_num_seconds_from_midnight_opt(secs, nanos))
            .with_context(|| InvalidArrowValueSnafu {
                reason: format!(
                    "raw TIME value {raw} with scale {} is not a valid time of day",
                    self.scale
                ),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::datatypes::{Int32Type, Int64Type};

    #[test]
    fn should_decode_across_scales() {
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(45_296)]);
        assert_eq!(
            SnowflakeTime { scale: 0 }
                .read_arrow_type(&array, 0)
                .unwrap(),
            NaiveTime::from_hms_opt(12, 34, 56).unwrap()
        );

        let array = PrimitiveArray::<Int64Type>::from(vec![Some(45_296_789)]);
        assert_eq!(
            SnowflakeTime { scale: 3 }
                .read_arrow_type(&array, 0)
                .unwrap(),
            NaiveTime::from_hms_milli_opt(12, 34, 56, 789).unwrap()
        );

        let array = PrimitiveArray::<Int64Type>::from(vec![Some(45_296_123_456_789)]);
        assert_eq!(
            SnowflakeTime { scale: 9 }
                .read_arrow_type(&array, 0)
                .unwrap(),
            NaiveTime::from_hms_nano_opt(12, 34, 56, 123_456_789).unwrap()
        );
    }

    /// TIME columns with scale ≤ 4 arrive as `Int32`; the generic reader must
    /// decode that backing array as readily as `Int64`, including the largest
    /// value an `Int32` TIME can hold (23:59:59.9999 at scale 4).
    #[test]
    fn should_decode_int32_backed_array() {
        let array = PrimitiveArray::<Int32Type>::from(vec![Some(45_296)]);
        assert_eq!(
            SnowflakeTime { scale: 0 }
                .read_arrow_type(&array, 0)
                .unwrap(),
            NaiveTime::from_hms_opt(12, 34, 56).unwrap()
        );

        let array = PrimitiveArray::<Int32Type>::from(vec![Some(863_999_999)]);
        assert_eq!(
            SnowflakeTime { scale: 4 }
                .read_arrow_type(&array, 0)
                .unwrap(),
            NaiveTime::from_hms_nano_opt(23, 59, 59, 999_900_000).unwrap()
        );
    }

    #[test]
    fn should_report_null_cell_as_null_value_error() {
        let array = PrimitiveArray::<Int64Type>::from(vec![None::<i64>, Some(0)]);
        let err = SnowflakeTime { scale: 9 }
            .read_arrow_type(&array, 0)
            .unwrap_err();
        assert!(
            matches!(err, ReadArrowError::NullValue { .. }),
            "got {err:?}"
        );
    }

    /// Inputs the server never sends for TIME, but which the reader must reject
    /// rather than panic on: a negative value, a scale beyond 9, and a
    /// second-of-day at or past the end of the day.
    #[test]
    fn should_report_out_of_range_inputs_as_invalid_value() {
        let negative = PrimitiveArray::<Int64Type>::from(vec![Some(-1)]);
        let bad_scale = PrimitiveArray::<Int64Type>::from(vec![Some(0)]);
        let overflow = PrimitiveArray::<Int64Type>::from(vec![Some(86_400)]);

        for (reader, array) in [
            (SnowflakeTime { scale: 9 }, &negative),
            (SnowflakeTime { scale: 10 }, &bad_scale),
            (SnowflakeTime { scale: 0 }, &overflow),
        ] {
            let err = reader.read_arrow_type(array, 0).unwrap_err();
            assert!(
                matches!(err, ReadArrowError::InvalidArrowValue { .. }),
                "got {err:?}"
            );
        }
    }
}
