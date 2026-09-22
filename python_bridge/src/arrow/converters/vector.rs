use arrow::array::{ArrayRef, FixedSizeListArray};
use pyo3::prelude::*;
use pyo3::types::PyList;
use sf_types::{SnowflakeVector, VectorCell};

use super::Column;
use super::decode::{PyMaterializer, TypedColumn};
use crate::arrow::converters::util::downcast_column;
use crate::arrow::plan::SnowflakeFieldType;

pub(super) fn from_column(array: &ArrayRef, field_type: &SnowflakeFieldType) -> PyResult<Column> {
    downcast_column::<FixedSizeListArray>(array, field_type)
        .map(|array| Column::Vector(TypedColumn::new(array, SnowflakeVector, VectorMaterializer)))
}

pub(crate) struct VectorMaterializer;

impl PyMaterializer<SnowflakeVector> for VectorMaterializer {
    fn materialize<'py>(
        &self,
        py: Python<'py>,
        value: VectorCell<'_>,
    ) -> PyResult<Bound<'py, PyAny>> {
        match value {
            VectorCell::Int32(values) => Ok(PyList::new(py, values)?.into_any()),
            VectorCell::Float32(values) => {
                Ok(PyList::new(py, values.iter().copied().map(f64::from))?.into_any())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{
        Array, ArrayRef, Date32Array, FixedSizeListArray, Float32Array, Int32Array, Int64Array,
    };
    use arrow::buffer::NullBuffer;
    use arrow::datatypes::{DataType, Field, Schema};
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;
    use pyo3::types::{PyFloat, PyInt, PyList};

    use crate::arrow::converters::ConversionContext;
    use crate::arrow::converters::test_util::assert_py_none;
    use crate::arrow::plan::{SnowflakeFieldType, VectorElementType};

    fn vector(element_type: VectorElementType) -> SnowflakeFieldType {
        SnowflakeFieldType::Vector {
            element_type,
            column_size: 16_777_216,
        }
    }

    fn int_vector(flat: Vec<Option<i32>>, dimension: i32, nulls: Option<Vec<bool>>) -> ArrayRef {
        let child = Arc::new(Field::new("item", DataType::Int32, true));
        let values = Arc::new(Int32Array::from(flat)) as Arc<dyn Array>;
        let nulls = nulls.map(NullBuffer::from);
        Arc::new(FixedSizeListArray::try_new(child, dimension, values, nulls).unwrap())
    }

    fn float_vector(flat: Vec<Option<f32>>, dimension: i32, nulls: Option<Vec<bool>>) -> ArrayRef {
        let child = Arc::new(Field::new("item", DataType::Float32, true));
        let values = Arc::new(Float32Array::from(flat)) as Arc<dyn Array>;
        let nulls = nulls.map(NullBuffer::from);
        Arc::new(FixedSizeListArray::try_new(child, dimension, values, nulls).unwrap())
    }

    #[test]
    fn int32_converts_to_python_int_list_with_nulls() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array = int_vector(
            vec![
                Some(1),
                Some(2),
                Some(3),
                None,
                None,
                None,
                Some(-5),
                Some(0),
                Some(40),
            ],
            3,
            Some(vec![true, false, true]),
        );
        let column = ctx
            .converter_from_column(&array, &vector(VectorElementType::Int32))
            .unwrap();

        Python::attach(|py| {
            let first = column.to_py(py, 0).unwrap();
            let list = first.cast::<PyList>().unwrap();
            assert_eq!(list.len(), 3);
            assert!(list.get_item(0).unwrap().is_instance_of::<PyInt>());
            assert_eq!(list.extract::<Vec<i32>>().unwrap(), vec![1, 2, 3]);
            assert_py_none(&column.to_py(py, 1).unwrap());
            assert_eq!(
                column.to_py(py, 2).unwrap().extract::<Vec<i32>>().unwrap(),
                vec![-5, 0, 40]
            );
        });
    }

    #[test]
    fn float32_converts_to_widened_python_float_list_with_nulls() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array = float_vector(
            vec![Some(1.5), Some(-3.5), Some(0.0), None, None, None],
            3,
            Some(vec![true, false]),
        );
        let column = ctx
            .converter_from_column(&array, &vector(VectorElementType::Float32))
            .unwrap();

        Python::attach(|py| {
            let first = column.to_py(py, 0).unwrap();
            let list = first.cast::<PyList>().unwrap();
            assert_eq!(list.len(), 3);
            assert!(list.get_item(0).unwrap().is_instance_of::<PyFloat>());
            assert!(!list.get_item(0).unwrap().is_instance_of::<PyInt>());
            assert_eq!(
                list.extract::<Vec<f64>>().unwrap(),
                vec![1.5_f64, -3.5, 0.0]
            );
            assert_py_none(&column.to_py(py, 1).unwrap());
        });
    }

    #[test]
    fn converts_dimension_one_int_and_float() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let ints = ctx
            .converter_from_column(
                &int_vector(vec![Some(i32::MIN)], 1, None),
                &vector(VectorElementType::Int32),
            )
            .unwrap();
        let floats = ctx
            .converter_from_column(
                &float_vector(vec![Some(-0.5)], 1, None),
                &vector(VectorElementType::Float32),
            )
            .unwrap();

        Python::attach(|py| {
            let int_list = ints.to_py(py, 0).unwrap();
            assert!(
                int_list
                    .cast::<PyList>()
                    .unwrap()
                    .get_item(0)
                    .unwrap()
                    .is_instance_of::<PyInt>()
            );
            assert_eq!(int_list.extract::<Vec<i32>>().unwrap(), vec![i32::MIN]);

            let float_list = floats.to_py(py, 0).unwrap();
            assert!(
                float_list
                    .cast::<PyList>()
                    .unwrap()
                    .get_item(0)
                    .unwrap()
                    .is_instance_of::<PyFloat>()
            );
            assert_eq!(float_list.extract::<Vec<f64>>().unwrap(), vec![-0.5_f64]);
        });
    }

    #[test]
    fn converts_max_dimension_int_and_float() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let dimension = 4096_i32;
        let int_values: Vec<i32> = (0..dimension).collect();
        let float_values: Vec<f32> = (0..dimension).map(|i| i as f32).collect();
        let ints = ctx
            .converter_from_column(
                &int_vector(
                    int_values.iter().copied().map(Some).collect(),
                    dimension,
                    None,
                ),
                &vector(VectorElementType::Int32),
            )
            .unwrap();
        let floats = ctx
            .converter_from_column(
                &float_vector(
                    float_values.iter().copied().map(Some).collect(),
                    dimension,
                    None,
                ),
                &vector(VectorElementType::Float32),
            )
            .unwrap();

        Python::attach(|py| {
            let int_list = ints.to_py(py, 0).unwrap();
            assert_eq!(int_list.cast::<PyList>().unwrap().len(), dimension as usize);
            assert_eq!(int_list.extract::<Vec<i32>>().unwrap(), int_values);

            let float_list = floats.to_py(py, 0).unwrap();
            assert_eq!(
                float_list.cast::<PyList>().unwrap().len(),
                dimension as usize
            );
            let expected: Vec<f64> = float_values.iter().copied().map(f64::from).collect();
            assert_eq!(float_list.extract::<Vec<f64>>().unwrap(), expected);
        });
    }

    #[test]
    fn rejects_physical_mismatch_for_vector() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(Date32Array::from(vec![Some(0)]));
        let err = match ctx.converter_from_column(&array, &vector(VectorElementType::Int32)) {
            Ok(_) => panic!("expected physical mismatch for Date32 VECTOR"),
            Err(err) => err,
        };
        Python::attach(|py| {
            assert!(
                err.is_instance_of::<PyValueError>(py),
                "expected PyValueError, got {err}"
            );
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(
                text.contains("logical/physical type mismatch") && text.contains("VECTOR"),
                "got {text}"
            );
        });
    }

    #[test]
    fn rejects_unsupported_child_type() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let child = Arc::new(Field::new("item", DataType::Int64, false));
        let values = Arc::new(Int64Array::from(vec![1i64, 2, 3])) as Arc<dyn Array>;
        let array: ArrayRef =
            Arc::new(FixedSizeListArray::try_new(child, 3, values, None).unwrap());
        let column = ctx
            .converter_from_column(&array, &vector(VectorElementType::Int32))
            .unwrap();

        Python::attach(|py| {
            let err = column.to_py(py, 0).unwrap_err();
            assert!(
                err.is_instance_of::<PyValueError>(py),
                "expected PyValueError, got {err}"
            );
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(
                text.contains("VECTOR child type Int64 is not Int32 or Float32"),
                "got {text}"
            );
        });
    }
}
