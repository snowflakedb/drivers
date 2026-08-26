use arrow::array::ArrayRef;
use arrow::record_batch::RecordBatch;
use pyo3::exceptions::PyNotImplementedError;
use pyo3::prelude::*;

use crate::arrow::batch_converter::BatchConverter;
use crate::arrow::plan::{LogicalPlan, SnowflakeFieldType};

use super::Column;

pub(crate) struct ConversionContext {
    plan: LogicalPlan,
}

impl ConversionContext {
    pub(crate) fn new(schema: &arrow::datatypes::Schema) -> PyResult<Self> {
        let plan = LogicalPlan::from_schema(schema)?;
        Ok(Self { plan })
    }

    pub(crate) fn batch_converter(&self, batch: RecordBatch) -> PyResult<BatchConverter> {
        debug_assert_eq!(batch.num_columns(), self.plan.field_types.len());
        let row_count = batch.num_rows();
        let columns = batch
            .columns()
            .iter()
            .zip(self.plan.field_types.iter())
            .map(|(array, field_type)| self.converter_from_column(array, field_type))
            .collect::<PyResult<Vec<_>>>()?;
        Ok(BatchConverter::new(columns, row_count))
    }

    pub(crate) fn converter_from_column(
        &self,
        _array: &ArrayRef,
        field_type: &SnowflakeFieldType,
    ) -> PyResult<Column> {
        match *field_type {
            SnowflakeFieldType::Varchar { .. }
            | SnowflakeFieldType::Number { .. }
            | SnowflakeFieldType::Date
            | SnowflakeFieldType::Time { .. }
            | SnowflakeFieldType::TimestampNtz { .. }
            | SnowflakeFieldType::TimestampLtz { .. }
            | SnowflakeFieldType::TimestampTz { .. }
            | SnowflakeFieldType::Boolean
            | SnowflakeFieldType::Binary { .. }
            | SnowflakeFieldType::Real
            | SnowflakeFieldType::Decfloat { .. }
            | SnowflakeFieldType::Vector { .. } => Err(PyNotImplementedError::new_err(format!(
                "native Arrow conversion is not implemented for logical type {}",
                field_type.logical_type_name()
            ))),
        }
    }
}
