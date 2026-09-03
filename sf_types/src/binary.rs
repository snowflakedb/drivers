use arrow::array::{Array, GenericByteArray};
use arrow::datatypes::GenericBinaryType;

use crate::error::ReadArrowError;
use crate::traits::{ReadArrowType, SnowflakeType};

pub struct SnowflakeBinary;

impl SnowflakeType for SnowflakeBinary {
    type Representation<'a> = &'a [u8];
}

impl ReadArrowType<GenericByteArray<GenericBinaryType<i32>>> for SnowflakeBinary {
    fn read_arrow_type<'a>(
        &self,
        array: &'a GenericByteArray<GenericBinaryType<i32>>,
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
    use arrow::array::BinaryArray;

    #[test]
    fn should_read_borrowed_bytes() {
        let array = BinaryArray::from(vec![Some(&[0x48, 0x65, 0x6C, 0x6C, 0x6F][..])]);
        assert_eq!(
            SnowflakeBinary.read_arrow_type(&array, 0).unwrap(),
            &[0x48, 0x65, 0x6C, 0x6C, 0x6F]
        );
    }

    #[test]
    fn should_read_empty_bytes_as_present_not_null() {
        let array = BinaryArray::from(vec![Some(&[][..])]);
        assert_eq!(
            SnowflakeBinary.read_arrow_type(&array, 0).unwrap(),
            &[] as &[u8]
        );
    }

    #[test]
    fn should_read_arbitrary_bytes_including_embedded_nul() {
        let array = BinaryArray::from(vec![Some(&[0x00, 0xFF, 0x00, 0xDE, 0xAD][..])]);
        assert_eq!(
            SnowflakeBinary.read_arrow_type(&array, 0).unwrap(),
            &[0x00, 0xFF, 0x00, 0xDE, 0xAD]
        );
    }

    #[test]
    fn should_report_null_cell_as_null_value_error() {
        let array = BinaryArray::from(vec![None::<&[u8]>, Some(&[0x01][..])]);
        let err = SnowflakeBinary.read_arrow_type(&array, 0).unwrap_err();
        assert!(
            matches!(err, ReadArrowError::NullValue { .. }),
            "got {err:?}"
        );
    }
}
