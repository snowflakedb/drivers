use arrow::array::{Array, PrimitiveArray, StructArray};
use arrow::datatypes::{Int32Type, Int64Type};
use chrono::{DateTime, NaiveDateTime};
use snafu::{OptionExt, ensure};

use crate::error::{InvalidArrowValueSnafu, ReadArrowError};
use crate::traits::{ReadArrowType, SnowflakeType};

/// Wire-protocol bias for TIMESTAMP_TZ offset minutes.
///
/// Snowflake stores `signed_offset_minutes + 1440` so the Arrow/JSON token is
/// always non-negative. Subtract this on READ; add it on bind WRITE.
pub const TZ_OFFSET_BIAS_MINUTES: i32 = 1440;

/// Inclusive upper bound on the **biased** raw offset field (`0..=2880`).
///
/// Legacy Node (`convertRawTimestampTz`) asserts this range before subtracting
/// the bias. Values outside it are not legal protocol; they are treated as
/// decode errors rather than producing a nonsensical signed offset.
pub const TZ_OFFSET_MAX_RAW: i32 = TZ_OFFSET_BIAS_MINUTES * 2;

/// UTC instant plus the original observer offset in minutes.
///
/// `utc` is the wall-clock at UTC (`offset_minutes == 0` means the naive
/// datetime is already UTC). `offset_minutes` is the signed offset recovered
/// from the wire (`raw - TZ_OFFSET_BIAS_MINUTES`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TzInstant {
    pub utc: NaiveDateTime,
    pub offset_minutes: i32,
}

impl TzInstant {
    /// Naive datetime at `offset_minutes` (`utc` shifted by the stored offset).
    ///
    /// This is not UTC. Formatters use it for wall-clock digits next to the
    /// offset suffix. Overflow of the naive range falls back to `utc`; legal
    /// Arrow TZ offsets do not hit that path.
    pub fn to_naive_datetime_at_offset(&self) -> NaiveDateTime {
        if self.offset_minutes == 0 {
            return self.utc;
        }
        self.utc
            .checked_add_signed(chrono::Duration::minutes(i64::from(self.offset_minutes)))
            .unwrap_or(self.utc)
    }
}

/// Snowflake TIMESTAMP_TZ Arrow reader (Kind-1: `scale` only).
pub struct SnowflakeTimestampTz {
    pub scale: u32,
}

impl SnowflakeType for SnowflakeTimestampTz {
    type Representation<'a> = TzInstant;
}

impl ReadArrowType<StructArray> for SnowflakeTimestampTz {
    fn read_arrow_type<'a>(
        &self,
        array: &'a StructArray,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        read_struct_timestamp_tz(array, row_idx, self.scale)
    }
}

/// Split a raw scaled epoch into `(epoch_seconds, nanoseconds)`.
///
/// Scale 0 → seconds, 3 → milliseconds, 6 → microseconds; other 0–9 use
/// `10^scale`. Uses `div_euclid` so pre-epoch values floor correctly.
pub fn split_scaled_epoch(raw: i64, scale: u32) -> Result<(i64, u32), ReadArrowError> {
    if scale > 9 {
        return InvalidArrowValueSnafu {
            reason: format!("timestamp scale {scale} exceeds maximum of 9"),
        }
        .fail();
    }
    Ok(match scale {
        0 => (raw, 0u32),
        3 => {
            let secs = raw.div_euclid(1_000);
            let millis = raw.rem_euclid(1_000) as u32;
            (secs, millis * 1_000_000)
        }
        6 => {
            let secs = raw.div_euclid(1_000_000);
            let micros = raw.rem_euclid(1_000_000) as u32;
            (secs, micros * 1_000)
        }
        _ => {
            let divisor = 10i64.pow(scale);
            let secs = raw.div_euclid(divisor);
            let frac = raw.rem_euclid(divisor) as u32;
            let nanos = frac * (1_000_000_000u32 / divisor as u32);
            (secs, nanos)
        }
    })
}

/// 2-child `{epoch: Int64, fraction: Int32}` timestamp (NTZ/LTZ and TZ 3-col
/// epoch/fraction prefix). Layout is selected by callers; this does not
/// interpret a timezone child.
pub fn read_struct_timestamp(
    array: &StructArray,
    row_idx: usize,
) -> Result<NaiveDateTime, ReadArrowError> {
    if array.is_null(row_idx) {
        return Err(ReadArrowError::NullValue {
            location: snafu::location!(),
        });
    }

    if array.num_columns() < 2 {
        return InvalidArrowValueSnafu {
            reason: format!(
                "timestamp struct has {} column(s), expected at least 2",
                array.num_columns()
            ),
        }
        .fail();
    }

    let epoch_array = array
        .column(0)
        .as_any()
        .downcast_ref::<PrimitiveArray<Int64Type>>()
        .with_context(|| InvalidArrowValueSnafu {
            reason: "timestamp struct column 0 is not Int64".to_string(),
        })?;
    let fraction_array = array
        .column(1)
        .as_any()
        .downcast_ref::<PrimitiveArray<Int32Type>>()
        .with_context(|| InvalidArrowValueSnafu {
            reason: "timestamp struct column 1 is not Int32".to_string(),
        })?;

    let epoch_seconds = epoch_array.value(row_idx);
    let fraction_nanos = fraction_array.value(row_idx);

    if !(0..1_000_000_000).contains(&fraction_nanos) {
        return InvalidArrowValueSnafu {
            reason: format!(
                "fraction_nanos={fraction_nanos} is out of valid range [0, 1_000_000_000)"
            ),
        }
        .fail();
    }

    DateTime::from_timestamp(epoch_seconds, fraction_nanos as u32)
        .map(|dt| dt.naive_utc())
        .with_context(|| InvalidArrowValueSnafu {
            reason: format!(
                "epoch_seconds={epoch_seconds}, fraction_nanos={fraction_nanos} is out of range"
            ),
        })
}

/// Flat Int64 epoch in units of `10^-scale` seconds.
pub fn read_scaled_timestamp(
    array: &PrimitiveArray<Int64Type>,
    row_idx: usize,
    scale: u32,
) -> Result<NaiveDateTime, ReadArrowError> {
    if array.is_null(row_idx) {
        return Err(ReadArrowError::NullValue {
            location: snafu::location!(),
        });
    }

    let raw = array.value(row_idx);
    let (epoch_seconds, nanos) = split_scaled_epoch(raw, scale)?;

    DateTime::from_timestamp(epoch_seconds, nanos)
        .map(|dt| dt.naive_utc())
        .with_context(|| InvalidArrowValueSnafu {
            reason: format!(
                "scaled epoch raw={raw}, scale={scale} produced out-of-range timestamp"
            ),
        })
}

/// TIMESTAMP_TZ struct: 2 columns `{scaled_epoch, timezone}` or 3 columns
/// `{epoch, fraction, timezone}`. Timezone is always the last child, biased
/// by [`TZ_OFFSET_BIAS_MINUTES`]. Dispatch uses **column count**, not scale
/// (the server's scale≤3 → 2-col / scale>3 → 3-col pivot is an invariant of
/// the producer, not something this reader should re-derive).
fn read_struct_timestamp_tz(
    array: &StructArray,
    row_idx: usize,
    scale: u32,
) -> Result<TzInstant, ReadArrowError> {
    if array.is_null(row_idx) {
        return Err(ReadArrowError::NullValue {
            location: snafu::location!(),
        });
    }

    let num_columns = array.num_columns();
    let utc = if num_columns == 3 {
        read_struct_timestamp(array, row_idx)?
    } else if num_columns == 2 {
        let epoch_array = array
            .column(0)
            .as_any()
            .downcast_ref::<PrimitiveArray<Int64Type>>()
            .with_context(|| InvalidArrowValueSnafu {
                reason: "TIMESTAMP_TZ struct column 0 is not Int64".to_string(),
            })?;

        let raw = epoch_array.value(row_idx);
        let (epoch_seconds, nanos) = split_scaled_epoch(raw, scale)?;

        DateTime::from_timestamp(epoch_seconds, nanos)
            .map(|dt| dt.naive_utc())
            .with_context(|| InvalidArrowValueSnafu {
                reason: format!(
                    "TZ scaled epoch raw={raw}, scale={scale} produced out-of-range timestamp"
                ),
            })?
    } else {
        return InvalidArrowValueSnafu {
            reason: format!("TIMESTAMP_TZ struct has {num_columns} columns, expected 2 or 3"),
        }
        .fail();
    };

    let offset_col_idx = num_columns - 1;
    let offset_array = array
        .column(offset_col_idx)
        .as_any()
        .downcast_ref::<PrimitiveArray<Int32Type>>()
        .with_context(|| InvalidArrowValueSnafu {
            reason: format!(
                "TIMESTAMP_TZ struct column {offset_col_idx} is not Int32 (expected tz_offset_min)"
            ),
        })?;
    let raw_offset = offset_array.value(row_idx);
    ensure!(
        (0..=TZ_OFFSET_MAX_RAW).contains(&raw_offset),
        InvalidArrowValueSnafu {
            reason: format!(
                "TIMESTAMP_TZ offset {raw_offset} is outside the valid biased range 0..={TZ_OFFSET_MAX_RAW}"
            ),
        }
    );
    let offset_minutes = raw_offset - TZ_OFFSET_BIAS_MINUTES;

    Ok(TzInstant {
        utc,
        offset_minutes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::ArrayRef;
    use arrow::datatypes::{DataType, Field as ArrowField};
    use std::sync::Arc;

    fn make_tz_struct_array_3col(epoch: i64, fraction: i32, offset_minutes: i32) -> StructArray {
        let epoch_col: ArrayRef = Arc::new(PrimitiveArray::<Int64Type>::from(vec![Some(epoch)]));
        let frac_col: ArrayRef = Arc::new(PrimitiveArray::<Int32Type>::from(vec![Some(fraction)]));
        let tz_col: ArrayRef = Arc::new(PrimitiveArray::<Int32Type>::from(vec![Some(
            offset_minutes + TZ_OFFSET_BIAS_MINUTES,
        )]));
        StructArray::from(vec![
            (
                Arc::new(ArrowField::new("epoch", DataType::Int64, false)),
                epoch_col,
            ),
            (
                Arc::new(ArrowField::new("fraction", DataType::Int32, false)),
                frac_col,
            ),
            (
                Arc::new(ArrowField::new("tz_offset", DataType::Int32, false)),
                tz_col,
            ),
        ])
    }

    fn make_tz_struct_array_2col(scaled_epoch: i64, offset_minutes: i32) -> StructArray {
        let epoch_col: ArrayRef =
            Arc::new(PrimitiveArray::<Int64Type>::from(vec![Some(scaled_epoch)]));
        let tz_col: ArrayRef = Arc::new(PrimitiveArray::<Int32Type>::from(vec![Some(
            offset_minutes + TZ_OFFSET_BIAS_MINUTES,
        )]));
        StructArray::from(vec![
            (
                Arc::new(ArrowField::new("epoch", DataType::Int64, false)),
                epoch_col,
            ),
            (
                Arc::new(ArrowField::new("tz_offset", DataType::Int32, false)),
                tz_col,
            ),
        ])
    }

    fn make_single_col_struct_array(epoch: i64) -> StructArray {
        let epoch_col: ArrayRef = Arc::new(PrimitiveArray::<Int64Type>::from(vec![Some(epoch)]));
        StructArray::from(vec![(
            Arc::new(ArrowField::new("epoch", DataType::Int64, false)),
            epoch_col,
        )])
    }

    fn make_raw_offset_2col(scaled_epoch: i64, raw_offset: i32) -> StructArray {
        let epoch_col: ArrayRef =
            Arc::new(PrimitiveArray::<Int64Type>::from(vec![Some(scaled_epoch)]));
        let tz_col: ArrayRef = Arc::new(PrimitiveArray::<Int32Type>::from(vec![Some(raw_offset)]));
        StructArray::from(vec![
            (
                Arc::new(ArrowField::new("epoch", DataType::Int64, false)),
                epoch_col,
            ),
            (
                Arc::new(ArrowField::new("tz_offset", DataType::Int32, false)),
                tz_col,
            ),
        ])
    }

    #[test]
    fn should_read_3col_struct_utc_and_zero_offset() {
        let array = make_tz_struct_array_3col(1_700_000_000, 0, 0);
        let value = SnowflakeTimestampTz { scale: 9 }
            .read_arrow_type(&array, 0)
            .unwrap();
        assert_eq!(value.utc.and_utc().timestamp(), 1_700_000_000);
        assert_eq!(value.offset_minutes, 0);
    }

    #[test]
    fn should_read_3col_struct_positive_offset() {
        let array = make_tz_struct_array_3col(1_700_000_000, 0, 330);
        let value = SnowflakeTimestampTz { scale: 9 }
            .read_arrow_type(&array, 0)
            .unwrap();
        assert_eq!(value.offset_minutes, 330);
    }

    #[test]
    fn should_read_3col_struct_negative_offset_after_bias_removal() {
        let array = make_tz_struct_array_3col(1_700_000_000, 0, -480);
        let value = SnowflakeTimestampTz { scale: 9 }
            .read_arrow_type(&array, 0)
            .unwrap();
        assert_eq!(value.offset_minutes, -480);
    }

    #[test]
    fn should_apply_tz_offset_bias_1440_on_read() {
        // Wire 1770 → +330 minutes (+05:30).
        let array = make_raw_offset_2col(1_700_000_000, 1770);
        let value = SnowflakeTimestampTz { scale: 0 }
            .read_arrow_type(&array, 0)
            .unwrap();
        assert_eq!(value.offset_minutes, 330);
    }

    #[test]
    fn should_read_2col_scaled_struct() {
        let array = make_tz_struct_array_2col(1_700_000_000, 0);
        let value = SnowflakeTimestampTz { scale: 0 }
            .read_arrow_type(&array, 0)
            .unwrap();
        assert_eq!(value.utc.and_utc().timestamp(), 1_700_000_000);
        assert_eq!(value.offset_minutes, 0);
    }

    #[test]
    fn should_read_2col_struct_offset() {
        let array = make_tz_struct_array_2col(1_700_000_000, 330);
        let value = SnowflakeTimestampTz { scale: 0 }
            .read_arrow_type(&array, 0)
            .unwrap();
        assert_eq!(value.offset_minutes, 330);
    }

    #[test]
    fn should_reject_1col_struct() {
        let array = make_single_col_struct_array(1_700_000_000);
        let err = SnowflakeTimestampTz { scale: 0 }
            .read_arrow_type(&array, 0)
            .unwrap_err();
        assert!(matches!(err, ReadArrowError::InvalidArrowValue { .. }));
    }

    #[test]
    fn should_reject_4col_struct() {
        let epoch_col: ArrayRef = Arc::new(PrimitiveArray::<Int64Type>::from(vec![Some(0i64)]));
        let frac_col: ArrayRef = Arc::new(PrimitiveArray::<Int32Type>::from(vec![Some(0i32)]));
        let tz_col: ArrayRef = Arc::new(PrimitiveArray::<Int32Type>::from(vec![Some(
            TZ_OFFSET_BIAS_MINUTES,
        )]));
        let extra_col: ArrayRef = Arc::new(PrimitiveArray::<Int32Type>::from(vec![Some(0i32)]));
        let array = StructArray::from(vec![
            (
                Arc::new(ArrowField::new("epoch", DataType::Int64, false)),
                epoch_col,
            ),
            (
                Arc::new(ArrowField::new("fraction", DataType::Int32, false)),
                frac_col,
            ),
            (
                Arc::new(ArrowField::new("tz_offset", DataType::Int32, false)),
                tz_col,
            ),
            (
                Arc::new(ArrowField::new("extra", DataType::Int32, false)),
                extra_col,
            ),
        ]);
        let err = SnowflakeTimestampTz { scale: 9 }
            .read_arrow_type(&array, 0)
            .unwrap_err();
        assert!(matches!(err, ReadArrowError::InvalidArrowValue { .. }));
    }

    #[test]
    fn should_report_null_struct_as_null_value_error() {
        let array = StructArray::new(
            make_tz_struct_array_2col(0, 0).fields().clone(),
            make_tz_struct_array_2col(0, 0).columns().to_vec(),
            Some(vec![false].into()),
        );
        let err = SnowflakeTimestampTz { scale: 0 }
            .read_arrow_type(&array, 0)
            .unwrap_err();
        assert!(matches!(err, ReadArrowError::NullValue { .. }));
    }

    #[test]
    fn should_reject_biased_offset_below_zero() {
        let array = make_raw_offset_2col(0, -1);
        let err = SnowflakeTimestampTz { scale: 0 }
            .read_arrow_type(&array, 0)
            .unwrap_err();
        assert!(
            matches!(err, ReadArrowError::InvalidArrowValue { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn should_reject_biased_offset_above_2880() {
        let array = make_raw_offset_2col(0, TZ_OFFSET_MAX_RAW + 1);
        let err = SnowflakeTimestampTz { scale: 0 }
            .read_arrow_type(&array, 0)
            .unwrap_err();
        assert!(
            matches!(err, ReadArrowError::InvalidArrowValue { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn should_accept_biased_offset_boundaries() {
        let min = SnowflakeTimestampTz { scale: 0 }
            .read_arrow_type(&make_raw_offset_2col(0, 0), 0)
            .unwrap();
        assert_eq!(min.offset_minutes, -TZ_OFFSET_BIAS_MINUTES);
        let max = SnowflakeTimestampTz { scale: 0 }
            .read_arrow_type(&make_raw_offset_2col(0, TZ_OFFSET_MAX_RAW), 0)
            .unwrap();
        assert_eq!(max.offset_minutes, TZ_OFFSET_BIAS_MINUTES);
    }

    #[test]
    fn should_floor_negative_2col_scaled_epoch() {
        // -500 at scale 3 is -0.5s; truncating `/` would yield a negative nanos.
        let array = make_tz_struct_array_2col(-500, 0);
        let value = SnowflakeTimestampTz { scale: 3 }
            .read_arrow_type(&array, 0)
            .unwrap();
        assert_eq!(value.utc.and_utc().timestamp(), -1);
        assert_eq!(value.utc.and_utc().timestamp_subsec_nanos(), 500_000_000);
    }
}
