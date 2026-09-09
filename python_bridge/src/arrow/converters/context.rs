use arrow::array::ArrayRef;
use arrow::record_batch::RecordBatch;
use pyo3::exceptions::PyNotImplementedError;
use pyo3::prelude::*;

use crate::arrow::batch_converter::BatchConverter;
use crate::arrow::plan::{LogicalPlan, SnowflakeFieldType};

use super::Column;
use super::binary;
use super::boolean;
use super::date;
use super::decfloat;
use super::number;
use super::real;
use super::text;
use super::time;
use super::timestamp_ntz;

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
        array: &ArrayRef,
        field_type: &SnowflakeFieldType,
    ) -> PyResult<Column> {
        match *field_type {
            SnowflakeFieldType::Boolean => boolean::from_column(array, field_type),
            SnowflakeFieldType::Number { scale, .. } => {
                number::from_column(array, field_type, scale)
            }
            SnowflakeFieldType::Real => real::from_column(array, field_type),
            SnowflakeFieldType::Varchar { .. } => text::from_column(array, field_type),
            SnowflakeFieldType::Binary { .. } => binary::from_column(array, field_type),
            SnowflakeFieldType::Decfloat { .. } => decfloat::from_column(array, field_type),
            SnowflakeFieldType::Date => date::from_column(array, field_type),
            SnowflakeFieldType::Time { scale } => time::from_column(array, field_type, scale),
            SnowflakeFieldType::TimestampNtz { scale } => {
                timestamp_ntz::from_column(array, field_type, scale)
            }
            SnowflakeFieldType::TimestampLtz { .. }
            | SnowflakeFieldType::TimestampTz { .. }
            | SnowflakeFieldType::Vector { .. } => Err(PyNotImplementedError::new_err(format!(
                "native Arrow conversion is not implemented for logical type {}",
                field_type.logical_type_name()
            ))),
        }
    }
}
