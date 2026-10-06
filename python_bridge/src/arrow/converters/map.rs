use std::sync::Arc;

use arrow::array::{Array, ArrayRef, MapArray};
use arrow::datatypes::{DataType, FieldRef};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::Column;
use super::context::ConversionContext;
use super::util::{downcast_column, py_none};
use crate::arrow::error::{InvalidMapSchemaSnafu, PlanError};
use crate::arrow::plan::SnowflakeFieldType;

pub(crate) struct MapColumn {
    map: MapArray,
    // MapColumn is a column so this has to be boxed to not have a circular reference
    key: Box<Column>,
    value: Box<Column>,
}

impl MapColumn {
    pub(super) fn to_py<'py>(&self, py: Python<'py>, row: usize) -> PyResult<Bound<'py, PyAny>> {
        if self.map.is_null(row) {
            return Ok(py_none(py));
        }
        let offsets = self.map.offsets();
        let start = map_offset(offsets, row)?;
        let end = map_offset(offsets, row + 1)?;
        if start > end || end > self.map.entries().len() {
            return Err(PyValueError::new_err(
                "[Snowflake Exception] map offsets are out of range",
            ));
        }
        let dict = PyDict::new(py);
        for index in start..end {
            dict.set_item(self.key.to_py(py, index)?, self.value.to_py(py, index)?)?;
        }
        Ok(dict.into_any())
    }
}

pub(super) fn from_column(context: &ConversionContext, array: &ArrayRef) -> PyResult<Column> {
    let map = downcast_column::<MapArray>(array, &SnowflakeFieldType::ArrowMap)?;
    let [key_field, value_field] = entry_fields(map.data_type())?;
    let key_type = SnowflakeFieldType::from_field(key_field.as_ref())?;
    let value_type = SnowflakeFieldType::from_field(value_field.as_ref())?;
    let key = context.converter_from_column(map.keys(), &key_type)?;
    let value = context.converter_from_column(map.values(), &value_type)?;
    Ok(Column::Map(MapColumn {
        map,
        key: Box::new(key),
        value: Box::new(value),
    }))
}

fn entry_fields(data_type: &DataType) -> Result<[FieldRef; 2], PlanError> {
    let DataType::Map(entries, _) = data_type else {
        return InvalidMapSchemaSnafu { count: 0usize }.fail();
    };
    let DataType::Struct(fields) = entries.data_type() else {
        return InvalidMapSchemaSnafu { count: 0usize }.fail();
    };
    let [key_field, value_field] = fields.as_ref() else {
        return InvalidMapSchemaSnafu {
            count: fields.len(),
        }
        .fail();
    };
    Ok([Arc::clone(key_field), Arc::clone(value_field)])
}

fn map_offset(offsets: &[i32], index: usize) -> PyResult<usize> {
    let raw = offsets.get(index).copied().ok_or_else(|| {
        PyValueError::new_err(format!(
            "[Snowflake Exception] map offset index {index} is out of range"
        ))
    })?;
    usize::try_from(raw).map_err(|_| {
        PyValueError::new_err(format!(
            "[Snowflake Exception] map offset {raw} is negative"
        ))
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use arrow::array::{Decimal128Array, MapArray, StringArray, StructArray};
    use arrow::buffer::{NullBuffer, OffsetBuffer, ScalarBuffer};
    use arrow::datatypes::{DataType, Field, Schema};
    use pyo3::prelude::*;
    use pyo3::types::PyDict;
    use snafu::ResultExt;

    use super::*;
    use crate::arrow::converters::test_util::{assert_py_int, assert_py_none, assert_py_str};
    use crate::arrow::error::ColumnConvertSnafu;

    fn meta(logical_type: &str, extra: &[(&str, &str)]) -> HashMap<String, String> {
        let mut metadata = HashMap::from([("logicalType".to_string(), logical_type.to_string())]);
        for (key, value) in extra {
            metadata.insert((*key).to_string(), (*value).to_string());
        }
        metadata
    }

    fn varchar_int_map(values: Vec<Option<i128>>, nulls: Option<Vec<bool>>) -> ArrayRef {
        let key_field = Field::new("key", DataType::Utf8, false).with_metadata(meta("TEXT", &[]));
        let value_field = Field::new("value", DataType::Decimal128(38, 0), true)
            .with_metadata(meta("FIXED", &[("scale", "0"), ("precision", "38")]));
        let keys = Arc::new(StringArray::from(
            values
                .iter()
                .enumerate()
                .map(|(index, value)| value.map(|_| format!("{}", (b'a' + index as u8) as char)))
                .collect::<Vec<_>>(),
        ));
        let decimals = Arc::new(
            Decimal128Array::from(values)
                .with_precision_and_scale(38, 0)
                .unwrap(),
        );
        let entries = StructArray::try_new(
            vec![key_field.clone(), value_field.clone()].into(),
            vec![keys, decimals],
            None,
        )
        .unwrap();
        let mut offsets = Vec::with_capacity(entries.len() + 1);
        offsets.push(0i32);
        for index in 0..entries.len() {
            offsets.push((index + 1) as i32);
        }
        let nulls = nulls.map(NullBuffer::from);
        let entries_field = Arc::new(Field::new(
            "entries",
            DataType::Struct(vec![key_field, value_field].into()),
            false,
        ));
        Arc::new(
            MapArray::try_new(
                entries_field,
                OffsetBuffer::new(ScalarBuffer::from(offsets)),
                entries,
                nulls,
                false,
            )
            .context(ColumnConvertSnafu)
            .unwrap(),
        )
    }

    #[test]
    fn map_row_is_dict_and_null_is_none() {
        Python::initialize();
        let context = ConversionContext::new(&Schema::empty()).unwrap();
        let array = varchar_int_map(vec![Some(1), Some(2)], Some(vec![true, false]));
        let column = context
            .converter_from_column(&array, &SnowflakeFieldType::ArrowMap)
            .unwrap();

        Python::attach(|py| {
            let dict = column.to_py(py, 0).unwrap();
            let dict = dict.cast::<PyDict>().unwrap();
            assert_eq!(dict.len(), 1);
            assert_py_int(&dict.get_item("a").unwrap().unwrap(), 1);
            assert_py_none(&column.to_py(py, 1).unwrap());
        });
    }

    #[test]
    fn utf8_map_stays_json_string() {
        Python::initialize();
        let field = Field::new("m", DataType::Utf8, true).with_metadata(meta("MAP", &[]));
        let field_type = SnowflakeFieldType::from_field(&field).unwrap();
        let context = ConversionContext::new(&Schema::empty()).unwrap();
        let array: ArrayRef = Arc::new(StringArray::from(vec![Some("{\"a\": 1}")]));
        let column = context.converter_from_column(&array, &field_type).unwrap();

        Python::attach(|py| {
            assert_py_str(&column.to_py(py, 0).unwrap(), "{\"a\": 1}");
        });
    }
}
