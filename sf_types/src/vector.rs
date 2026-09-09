use arrow::array::{Array, FixedSizeListArray, Float32Array, Int32Array};
use arrow::datatypes::DataType;

use crate::error::{InvalidArrowValueSnafu, ReadArrowError};
use crate::traits::{ReadArrowType, SnowflakeType};

/// One non-null VECTOR cell: a borrowed slice of the Arrow child buffer.
#[derive(Debug)]
pub enum VectorCell<'a> {
    Int32(&'a [i32]),
    Float32(&'a [f32]),
}

/// Snowflake VECTOR.
///
/// The server sends VECTOR as an Arrow `FixedSizeList` of `Int32`
/// (`VECTOR(INT)`) or `Float32` (`VECTOR(FLOAT)`). Decoding borrows the child
/// buffer as a [`VectorCell`]; any other child type fails closed. How a front
/// end surfaces that slice (ODBC's compact JSON string, the Node.js bridge's
/// `number[]`) stays in the driver crate. Python still decodes VECTOR in
/// `FixedSizeListConverter.cpp` rather than through this type, with the same
/// Int32/Float32 fail-closed contract.
pub struct SnowflakeVector;

impl SnowflakeType for SnowflakeVector {
    type Representation<'a> = VectorCell<'a>;
}

impl ReadArrowType<FixedSizeListArray> for SnowflakeVector {
    fn read_arrow_type<'a>(
        &self,
        array: &'a FixedSizeListArray,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        if array.is_null(row_idx) {
            return Err(ReadArrowError::NullValue {
                location: snafu::location!(),
            });
        }
        let len = array.value_length() as usize;
        let start = array.value_offset(row_idx) as usize;
        let child = array.values();
        match child.data_type() {
            DataType::Int32 => {
                let ints = child.as_any().downcast_ref::<Int32Array>().ok_or_else(|| {
                    InvalidArrowValueSnafu {
                        reason: format!(
                            "VECTOR child declared Int32 but array is {:?}",
                            child.data_type()
                        ),
                    }
                    .build()
                })?;
                let base = ints.offset() + start;
                Ok(VectorCell::Int32(&ints.values()[base..base + len]))
            }
            DataType::Float32 => {
                let floats = child
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .ok_or_else(|| {
                        InvalidArrowValueSnafu {
                            reason: format!(
                                "VECTOR child declared Float32 but array is {:?}",
                                child.data_type()
                            ),
                        }
                        .build()
                    })?;
                let base = floats.offset() + start;
                Ok(VectorCell::Float32(&floats.values()[base..base + len]))
            }
            other => Err(InvalidArrowValueSnafu {
                reason: format!("VECTOR child type {other:?} is not Int32 or Float32"),
            }
            .build()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::datatypes::Field;
    use std::sync::Arc;

    fn int_list(flat: Vec<i32>, dimension: i32) -> FixedSizeListArray {
        let child = Arc::new(Field::new("item", DataType::Int32, false));
        let values = Arc::new(Int32Array::from(flat)) as Arc<dyn Array>;
        FixedSizeListArray::try_new(child, dimension, values, None).unwrap()
    }

    fn float_list(flat: Vec<f32>, dimension: i32) -> FixedSizeListArray {
        let child = Arc::new(Field::new("item", DataType::Float32, false));
        let values = Arc::new(Float32Array::from(flat)) as Arc<dyn Array>;
        FixedSizeListArray::try_new(child, dimension, values, None).unwrap()
    }

    #[test]
    fn should_read_int_row_as_borrowed_slice() {
        let array = int_list(vec![1, 2, 3, 4, 5, 6], 3);
        match SnowflakeVector.read_arrow_type(&array, 1).unwrap() {
            VectorCell::Int32(slice) => assert_eq!(slice, &[4, 5, 6]),
            VectorCell::Float32(_) => panic!("expected Int32 cell"),
        }
    }

    #[test]
    fn should_read_float_row_as_borrowed_slice() {
        let array = float_list(vec![1.5, -3.5, 0.0, 2.0, 4.0, 8.0], 3);
        match SnowflakeVector.read_arrow_type(&array, 0).unwrap() {
            VectorCell::Float32(slice) => assert_eq!(slice, &[1.5, -3.5, 0.0]),
            VectorCell::Int32(_) => panic!("expected Float32 cell"),
        }
    }

    #[test]
    fn should_apply_parent_offset_when_reading_sliced_array() {
        let array = int_list(vec![1, 2, 3, 4, 5, 6, 7, 8, 9], 3);
        let sliced = array.slice(1, 2);
        match SnowflakeVector.read_arrow_type(&sliced, 0).unwrap() {
            VectorCell::Int32(slice) => assert_eq!(slice, &[4, 5, 6]),
            VectorCell::Float32(_) => panic!("expected Int32 cell"),
        }
        match SnowflakeVector.read_arrow_type(&sliced, 1).unwrap() {
            VectorCell::Int32(slice) => assert_eq!(slice, &[7, 8, 9]),
            VectorCell::Float32(_) => panic!("expected Int32 cell"),
        }
    }

    #[test]
    fn should_report_null_cell_as_null_value_error() {
        let child = Arc::new(Field::new("item", DataType::Int32, false));
        let values = Arc::new(Int32Array::from(vec![0, 0, 0, 4, 5, 6])) as Arc<dyn Array>;
        let nulls = arrow::buffer::NullBuffer::from(vec![false, true]);
        let array = FixedSizeListArray::try_new(child, 3, values, Some(nulls)).unwrap();
        let err = SnowflakeVector.read_arrow_type(&array, 0).unwrap_err();
        assert!(
            matches!(err, ReadArrowError::NullValue { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn should_reject_unsupported_child_type() {
        let child = Arc::new(Field::new("item", DataType::Int64, false));
        let values = Arc::new(arrow::array::Int64Array::from(vec![1i64, 2, 3])) as Arc<dyn Array>;
        let array = FixedSizeListArray::try_new(child, 3, values, None).unwrap();
        let err = SnowflakeVector.read_arrow_type(&array, 0).unwrap_err();
        assert!(
            matches!(err, ReadArrowError::InvalidArrowValue { .. }),
            "got {err:?}"
        );
    }
}
