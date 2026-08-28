use crate::error::ReadArrowError;
use crate::traits::{ReadArrowType, SnowflakeType};

/// Wraps a Snowflake type so NULL cells decode to `None` instead of failing.
///
/// Front ends wrap the reader for every column the server declared nullable;
/// a NOT NULL column uses the bare type, so an unexpected NULL surfaces as
/// [`ReadArrowError::NullValue`] rather than being silently absorbed.
pub struct Nullable<T> {
    pub value: T,
}

impl<T: SnowflakeType> SnowflakeType for Nullable<T> {
    type Representation<'a> = Option<T::Representation<'a>>;
}

impl<R, T: SnowflakeType + ReadArrowType<R>> ReadArrowType<R> for Nullable<T> {
    fn read_arrow_type<'a>(
        &self,
        array: &'a R,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        self.value
            .read_arrow_type(array, row_idx)
            .map(Some)
            .or_else(|e| match e {
                ReadArrowError::NullValue { .. } => Ok(None),
                other => Err(other),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::date::SnowflakeDate;
    use arrow::array::PrimitiveArray;
    use arrow::datatypes::Date32Type;

    #[test]
    fn should_absorb_null_cell_as_none() {
        let array = PrimitiveArray::<Date32Type>::from(vec![None, Some(0)]);
        let nullable = Nullable {
            value: SnowflakeDate,
        };
        assert_eq!(nullable.read_arrow_type(&array, 0).unwrap(), None);
        assert_eq!(
            nullable.read_arrow_type(&array, 1).unwrap(),
            Some(chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
        );
    }

    /// Only `NullValue` is absorbed — a genuine decode failure must still
    /// reach the caller, or a malformed batch would read as a column of NULLs.
    #[test]
    fn should_propagate_non_null_read_errors() {
        use arrow::array::BooleanArray;

        struct AlwaysInvalid;

        impl SnowflakeType for AlwaysInvalid {
            type Representation<'a> = ();
        }

        impl ReadArrowType<BooleanArray> for AlwaysInvalid {
            fn read_arrow_type<'a>(
                &self,
                _array: &'a BooleanArray,
                _row_idx: usize,
            ) -> Result<Self::Representation<'a>, ReadArrowError> {
                Err(ReadArrowError::InvalidArrowValue {
                    reason: "malformed".to_string(),
                    location: snafu::location!(),
                })
            }
        }

        let array = BooleanArray::from(vec![Some(true)]);
        let err = Nullable {
            value: AlwaysInvalid,
        }
        .read_arrow_type(&array, 0)
        .unwrap_err();
        assert!(
            matches!(err, ReadArrowError::InvalidArrowValue { .. }),
            "got {err:?}"
        );
    }
}
