use arrow::array::{Array, Float64Array};

use crate::error::ReadArrowError;
use crate::traits::{ReadArrowType, SnowflakeType};

/// Snowflake REAL (the FLOAT / DOUBLE / REAL logical type).
///
/// The server sends REAL as an Arrow `Float64`, so the decode is the identity
/// on the stored `f64` and needs no column metadata. `NaN` and `±Infinity` are
/// values a Snowflake FLOAT column can hold, so the reader passes them through
/// unchanged rather than normalizing or rejecting them; how a front end renders
/// or coerces them (ODBC's `"INFINITY"` char form, the Node.js bridge's numeric
/// cell) is the front end's job.
pub struct SnowflakeReal;

impl SnowflakeType for SnowflakeReal {
    type Representation<'a> = f64;
}

impl ReadArrowType<Float64Array> for SnowflakeReal {
    fn read_arrow_type<'a>(
        &self,
        array: &'a Float64Array,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        if array.is_null(row_idx) {
            return Err(ReadArrowError::NullValue {
                location: snafu::location!(),
            });
        }
        Ok(array.value(row_idx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_finite_value() {
        let array = Float64Array::from(vec![Some(12345.6789)]);
        assert_eq!(
            SnowflakeReal.read_arrow_type(&array, 0).unwrap(),
            12345.6789
        );
    }

    /// `NaN` and `±Infinity` are legal FLOAT cell values; the reader must return
    /// them as-is so a front end can apply its own non-finite convention.
    #[test]
    fn should_preserve_non_finite_values() {
        let array = Float64Array::from(vec![
            Some(f64::NAN),
            Some(f64::INFINITY),
            Some(f64::NEG_INFINITY),
        ]);
        assert!(SnowflakeReal.read_arrow_type(&array, 0).unwrap().is_nan());
        assert_eq!(
            SnowflakeReal.read_arrow_type(&array, 1).unwrap(),
            f64::INFINITY
        );
        assert_eq!(
            SnowflakeReal.read_arrow_type(&array, 2).unwrap(),
            f64::NEG_INFINITY
        );
    }

    /// Negative zero is distinct from positive zero in its sign bit; the decode
    /// keeps the bit pattern rather than collapsing `-0.0` to `0.0`.
    #[test]
    fn should_preserve_negative_zero_sign() {
        let array = Float64Array::from(vec![Some(-0.0)]);
        assert!(
            SnowflakeReal
                .read_arrow_type(&array, 0)
                .unwrap()
                .is_sign_negative()
        );
    }

    #[test]
    fn should_report_null_cell_as_null_value_error() {
        let array = Float64Array::from(vec![None, Some(1.0)]);
        let err = SnowflakeReal.read_arrow_type(&array, 0).unwrap_err();
        assert!(
            matches!(err, ReadArrowError::NullValue { .. }),
            "got {err:?}"
        );
    }
}
