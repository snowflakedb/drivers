use arrow::array::{Array, PrimitiveArray};
use arrow::datatypes::Date32Type;
use chrono::NaiveDate;
use snafu::OptionExt;

use crate::civil::civil_from_unix_days;
use crate::error::{InvalidArrowValueSnafu, ReadArrowError};
use crate::traits::{ReadArrowType, SnowflakeType};

/// Snowflake DATE.
///
/// The server sends DATE as an Arrow `Date32` — a signed day count relative to
/// the Unix epoch — so decoding is pure epoch-relative arithmetic and needs no
/// column metadata. The result is a bare [`NaiveDate`]; how it is rendered
/// (ODBC's `SQL_DATE_STRUCT`, the Node.js bridge's JavaScript `Date`) is the
/// front end's job, as is any calendar-range policy — ODBC restricts DATE to
/// the SQL `0001..9999` year range, while a JavaScript `Date` has no such
/// limit, so that check lives in the `odbc` crate rather than here.
pub struct SnowflakeDate;

impl SnowflakeType for SnowflakeDate {
    type Representation<'a> = NaiveDate;
}

impl ReadArrowType<PrimitiveArray<Date32Type>> for SnowflakeDate {
    fn read_arrow_type<'a>(
        &self,
        array: &'a PrimitiveArray<Date32Type>,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        if array.is_null(row_idx) {
            return Err(ReadArrowError::NullValue {
                location: snafu::location!(),
            });
        }
        // Materialize through the shared calendar primitive rather than a
        // second, independent day→date conversion: the ODBC bulk-CHAR hot path
        // reads the same broken-down fields straight from `civil_from_unix_days`
        // without building a `NaiveDate`, so keeping this on the same kernel
        // means there is exactly one implementation of the calendar math.
        let days = array.value(row_idx);
        let (year, month, day) = civil_from_unix_days(days);
        NaiveDate::from_ymd_opt(year, month, day).with_context(|| InvalidArrowValueSnafu {
            reason: format!(
                "DATE day offset {days} maps to {year:04}-{month:02}-{day:02}, \
                 outside the representable calendar range"
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Date32` counts days from the Unix epoch, so day 0 must decode to it.
    #[test]
    fn should_read_unix_epoch_at_day_zero() {
        let array = PrimitiveArray::<Date32Type>::from(vec![Some(0)]);
        assert_eq!(
            SnowflakeDate.read_arrow_type(&array, 0).unwrap(),
            NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()
        );
    }

    /// The SQL calendar range boundaries the server can send. `sf_types`
    /// decodes them without opinion; whether they are *acceptable* is a
    /// per-front-end policy (see the `odbc` crate's `ValidateSqlValue`).
    #[test]
    fn should_read_year_1_and_year_9999_boundaries() {
        let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        let year_1 = NaiveDate::from_ymd_opt(1, 1, 1).unwrap();
        let year_9999 = NaiveDate::from_ymd_opt(9999, 12, 31).unwrap();

        let array = PrimitiveArray::<Date32Type>::from(vec![
            Some((year_1 - epoch).num_days() as i32),
            Some((year_9999 - epoch).num_days() as i32),
        ]);
        assert_eq!(SnowflakeDate.read_arrow_type(&array, 0).unwrap(), year_1);
        assert_eq!(SnowflakeDate.read_arrow_type(&array, 1).unwrap(), year_9999);
    }

    #[test]
    fn should_report_null_cell_as_null_value_error() {
        let array = PrimitiveArray::<Date32Type>::from(vec![None, Some(0)]);
        let err = SnowflakeDate.read_arrow_type(&array, 0).unwrap_err();
        assert!(
            matches!(err, ReadArrowError::NullValue { .. }),
            "got {err:?}"
        );
    }

    /// A day offset whose year falls outside chrono's representable range is
    /// surfaced as a decode error rather than panicking. The server never sends
    /// such a value for DATE, but the reader must stay total.
    #[test]
    fn should_report_out_of_range_day_offset_as_invalid_value() {
        let array = PrimitiveArray::<Date32Type>::from(vec![Some(i32::MAX)]);
        let err = SnowflakeDate.read_arrow_type(&array, 0).unwrap_err();
        assert!(
            matches!(err, ReadArrowError::InvalidArrowValue { .. }),
            "got {err:?}"
        );
    }
}
