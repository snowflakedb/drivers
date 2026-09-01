use arrow::array::{Array, BooleanArray};

use crate::error::ReadArrowError;
use crate::traits::{ReadArrowType, SnowflakeType};

/// Snowflake BOOLEAN. Decodes an Arrow `BooleanArray` cell as a [`bool`].
pub struct SnowflakeBoolean;

impl SnowflakeType for SnowflakeBoolean {
    type Representation<'a> = bool;
}

impl ReadArrowType<BooleanArray> for SnowflakeBoolean {
    fn read_arrow_type<'a>(
        &self,
        array: &'a BooleanArray,
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
    fn should_read_true() {
        let array = BooleanArray::from(vec![Some(true)]);
        assert!(SnowflakeBoolean.read_arrow_type(&array, 0).unwrap());
    }

    #[test]
    fn should_read_false() {
        let array = BooleanArray::from(vec![Some(false)]);
        assert!(!SnowflakeBoolean.read_arrow_type(&array, 0).unwrap());
    }

    #[test]
    fn should_report_null_cell_as_null_value_error() {
        let array = BooleanArray::from(vec![None::<bool>]);
        let err = SnowflakeBoolean.read_arrow_type(&array, 0).unwrap_err();
        assert!(
            matches!(err, ReadArrowError::NullValue { .. }),
            "got {err:?}"
        );
    }
}
