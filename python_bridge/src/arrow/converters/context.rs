use std::sync::Arc;

use arrow::array::ArrayRef;
use arrow::record_batch::RecordBatch;
use pyo3::exceptions::PyNotImplementedError;
use pyo3::prelude::*;
use pyo3::types::PyString;

use crate::arrow::batch_converter::BatchConverter;
use crate::arrow::plan::{LogicalPlan, SnowflakeFieldType};

use super::Column;
use super::binary;
use super::boolean;
use super::date;
use super::decfloat;
use super::interval;
use super::number;
use super::real;
use super::text;
use super::time;
use super::timestamp_ltz;
use super::timestamp_ntz;
use super::timestamp_tz;
use super::timezone::TimezoneProvider;

pub(crate) enum RowShape {
    Tuple,
    Dict { keys: Vec<Py<PyString>> },
}

pub(crate) struct ConversionContext {
    plan: LogicalPlan,
    row_shape: Arc<RowShape>,
    timezone: Arc<TimezoneProvider>,
}

impl ConversionContext {
    #[cfg(test)]
    pub(crate) fn new(schema: &arrow::datatypes::Schema) -> PyResult<Self> {
        Self::from_schema(schema, RowShape::Tuple, None)
    }

    pub(crate) fn with_session_timezone(
        schema: &arrow::datatypes::Schema,
        session_timezone: Option<String>,
    ) -> PyResult<Self> {
        Self::from_schema(schema, RowShape::Tuple, session_timezone)
    }

    pub(crate) fn with_dict_keys(
        py: Python<'_>,
        schema: &arrow::datatypes::Schema,
        session_timezone: Option<String>,
    ) -> PyResult<Self> {
        let keys = schema
            .fields()
            .iter()
            .map(|field| PyString::intern(py, field.name()).unbind())
            .collect();
        Self::from_schema(schema, RowShape::Dict { keys }, session_timezone)
    }

    fn from_schema(
        schema: &arrow::datatypes::Schema,
        row_shape: RowShape,
        session_timezone: Option<String>,
    ) -> PyResult<Self> {
        let plan = LogicalPlan::from_schema(schema)?;
        Ok(Self {
            plan,
            row_shape: Arc::new(row_shape),
            timezone: Arc::new(TimezoneProvider::new(session_timezone)),
        })
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
        Ok(BatchConverter::new(
            columns,
            row_count,
            Arc::clone(&self.row_shape),
        ))
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
            SnowflakeFieldType::TimestampLtz { scale } => {
                timestamp_ltz::from_column(array, field_type, scale, Arc::clone(&self.timezone))
            }
            SnowflakeFieldType::TimestampTz { scale } => {
                timestamp_tz::from_column(array, field_type, scale)
            }
            SnowflakeFieldType::IntervalYearMonth { scale } => {
                interval::year_month_from_column(array, field_type, scale)
            }
            SnowflakeFieldType::IntervalDayTime => {
                interval::day_time_from_column(array, field_type)
            }
            SnowflakeFieldType::Vector { .. } => Err(PyNotImplementedError::new_err(format!(
                "native Arrow conversion is not implemented for logical type {}",
                field_type.logical_type_name()
            ))),
        }
    }
}
