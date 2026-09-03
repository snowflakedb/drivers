use arrow::array::{Array, GenericByteArray};
use arrow::datatypes::Utf8Type;

use crate::error::ReadArrowError;
use crate::traits::{ReadArrowType, SnowflakeType};

/// Snowflake TEXT.
///
/// The server sends TEXT as an Arrow `Utf8` array, so decoding is a bounds-
/// checked borrow of the already-validated UTF-8 bytes — no column metadata,
/// no calendar or clock math. The result is a `&str` borrowed from the array;
/// how it is surfaced (ODBC's `SnowflakeVarchar` also handles the string →
/// numeric / date / interval bind coercions and owns the `is_semi_structured`
/// flag; the Node.js bridge wraps it in a `Cow` for its `JsCell`) is the front
/// end's job and stays in the driver crates.
pub struct SnowflakeText;

impl SnowflakeType for SnowflakeText {
    type Representation<'a> = &'a str;
}

impl ReadArrowType<GenericByteArray<Utf8Type>> for SnowflakeText {
    fn read_arrow_type<'a>(
        &self,
        array: &'a GenericByteArray<Utf8Type>,
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
    use arrow::array::StringArray;

    #[test]
    fn should_read_borrowed_str() {
        let array = StringArray::from(vec![Some("hello")]);
        assert_eq!(SnowflakeText.read_arrow_type(&array, 0).unwrap(), "hello");
    }

    /// The empty string is a value, distinct from NULL: it decodes to `""`
    /// rather than a `NullValue` error.
    #[test]
    fn should_read_empty_string_as_value_not_null() {
        let array = StringArray::from(vec![Some("")]);
        assert_eq!(SnowflakeText.read_arrow_type(&array, 0).unwrap(), "");
    }

    /// The borrow is the raw UTF-8 the server sent, so multi-byte scalars and
    /// embedded NULs survive the decode untouched.
    #[test]
    fn should_preserve_multibyte_and_embedded_nul() {
        let array = StringArray::from(vec![Some("naïve — 日本語\0tail")]);
        assert_eq!(
            SnowflakeText.read_arrow_type(&array, 0).unwrap(),
            "naïve — 日本語\0tail"
        );
    }

    #[test]
    fn should_report_null_cell_as_null_value_error() {
        let array = StringArray::from(vec![None, Some("x")]);
        let err = SnowflakeText.read_arrow_type(&array, 0).unwrap_err();
        assert!(
            matches!(err, ReadArrowError::NullValue { .. }),
            "got {err:?}"
        );
    }
}
