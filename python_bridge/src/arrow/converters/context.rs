use std::sync::Arc;

use arrow::array::ArrayRef;
use arrow::record_batch::RecordBatch;
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
use super::numpy::NumpyProvider;
use super::real;
use super::text;
use super::time;
use super::timestamp_ltz;
use super::timestamp_ntz;
use super::timestamp_tz;
use super::timezone::TimezoneProvider;
use super::vector;

pub(crate) enum RowShape {
    Tuple,
    Dict { keys: Vec<Py<PyString>> },
}

fn dict_row_shape(py: Python<'_>, schema: &arrow::datatypes::Schema) -> RowShape {
    let keys = schema
        .fields()
        .iter()
        .map(|field| PyString::intern(py, field.name()).unbind())
        .collect();
    RowShape::Dict { keys }
}

pub(crate) struct ConversionContext {
    plan: LogicalPlan,
    row_shape: Arc<RowShape>,
    timezone: Arc<TimezoneProvider>,
    numpy: Arc<NumpyProvider>,
    use_numpy: bool,
}

impl ConversionContext {
    #[cfg(test)]
    pub(crate) fn new(schema: &arrow::datatypes::Schema) -> PyResult<Self> {
        Self::from_schema(schema, RowShape::Tuple, None, false)
    }

    #[cfg(test)]
    pub(crate) fn with_numpy(schema: &arrow::datatypes::Schema) -> PyResult<Self> {
        Self::from_schema(schema, RowShape::Tuple, None, true)
    }

    pub(crate) fn with_session_timezone(
        schema: &arrow::datatypes::Schema,
        session_timezone: Option<String>,
        use_numpy: bool,
    ) -> PyResult<Self> {
        Self::from_schema(schema, RowShape::Tuple, session_timezone, use_numpy)
    }

    pub(crate) fn with_dict_keys(
        py: Python<'_>,
        schema: &arrow::datatypes::Schema,
        session_timezone: Option<String>,
        use_numpy: bool,
    ) -> PyResult<Self> {
        Self::from_schema(
            schema,
            dict_row_shape(py, schema),
            session_timezone,
            use_numpy,
        )
    }

    fn from_schema(
        schema: &arrow::datatypes::Schema,
        row_shape: RowShape,
        session_timezone: Option<String>,
        use_numpy: bool,
    ) -> PyResult<Self> {
        let plan = LogicalPlan::from_schema(schema)?;
        Ok(Self {
            plan,
            row_shape: Arc::new(row_shape),
            timezone: Arc::new(TimezoneProvider::new(session_timezone)),
            numpy: Arc::new(NumpyProvider::new()),
            use_numpy,
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
            SnowflakeFieldType::Number { scale, .. } => number::from_column(
                array,
                field_type,
                scale,
                Arc::clone(&self.numpy),
                self.use_numpy,
            ),
            SnowflakeFieldType::Real => {
                real::from_column(array, field_type, Arc::clone(&self.numpy), self.use_numpy)
            }
            SnowflakeFieldType::Varchar { .. } => text::from_column(array, field_type),
            SnowflakeFieldType::Binary { .. } => binary::from_column(array, field_type),
            SnowflakeFieldType::Decfloat { .. } => {
                decfloat::from_column(array, field_type, Arc::clone(&self.numpy), self.use_numpy)
            }
            SnowflakeFieldType::Date => {
                date::from_column(array, field_type, Arc::clone(&self.numpy), self.use_numpy)
            }
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
            SnowflakeFieldType::Vector { .. } => vector::from_column(array, field_type),
        }
    }
}
