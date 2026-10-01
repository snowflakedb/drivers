use std::sync::Arc;

use arrow::array::ArrayRef;
use arrow::datatypes::{DataType, Field, FieldRef, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use snafu::ResultExt;

use crate::arrow::converters::TimezoneProvider;
use crate::arrow::error::{
    ColumnConvertSnafu, InvalidMetadataSnafu, InvalidScaleSnafu, PlanError,
    UnsupportedSnowflakeTypeSnafu,
};
use crate::arrow::plan::reject_nested_arrow_type;

mod fixed;
mod int_values;
mod interval;
mod time;
mod timestamp;
mod utils;

pub(crate) struct TableConverter {
    number_to_decimal: bool,
    force_microsecond_precision: bool,
    timezone: TimezoneProvider,
}

impl TableConverter {
    pub(crate) fn new(
        number_to_decimal: bool,
        force_microsecond_precision: bool,
        timezone: Option<String>,
    ) -> Self {
        Self {
            number_to_decimal,
            force_microsecond_precision,
            timezone: TimezoneProvider::new(timezone),
        }
    }

    pub(crate) fn convert_batch(&self, batch: RecordBatch) -> Result<RecordBatch, PlanError> {
        if !self.needs_conversion(batch.schema_ref())? {
            return Ok(batch);
        }

        let (schema, columns, _) = batch.into_parts();
        let mut converted_columns = Vec::with_capacity(columns.len());
        for (field, column) in schema.fields().iter().zip(columns) {
            converted_columns.push(self.convert_column(field.as_ref(), column)?);
        }
        let types: Vec<DataType> = converted_columns
            .iter()
            .map(|column| column.data_type().clone())
            .collect();
        let schema = self.convert_fields(Arc::unwrap_or_clone(schema), Some(&types))?;
        RecordBatch::try_new(Arc::new(schema), converted_columns).context(ColumnConvertSnafu)
    }

    pub(crate) fn convert_schema(&self, schema: Schema) -> Result<Schema, PlanError> {
        self.convert_fields(schema, None)
    }

    fn convert_fields(
        &self,
        schema: Schema,
        column_types: Option<&[DataType]>,
    ) -> Result<Schema, PlanError> {
        let Schema {
            fields, metadata, ..
        } = schema;
        let mut converted = Vec::with_capacity(fields.len());
        for (index, field) in fields.iter().enumerate() {
            let data_type = match column_types {
                Some(types) => types[index].clone(),
                None => self.output_type(field.as_ref())?,
            };
            converted.push(converted_field(field.clone(), data_type));
        }
        Ok(Schema::new_with_metadata(converted, metadata))
    }

    fn needs_conversion(&self, schema: &Schema) -> Result<bool, PlanError> {
        for field in schema.fields() {
            if field_needs_conversion(field, field.data_type())? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn output_type(&self, field: &Field) -> Result<DataType, PlanError> {
        match logical_type(field) {
            Some("FIXED") => {
                if field_needs_conversion(field, field.data_type())? {
                    if self.number_to_decimal {
                        let scale = utils::scale_or(field, 0)?;
                        let scale = i8::try_from(scale).map_err(|_| {
                            InvalidMetadataSnafu {
                                key: "scale".to_string(),
                                value: scale.to_string(),
                            }
                            .build()
                        })?;
                        Ok(DataType::Decimal128(38, scale))
                    } else {
                        Ok(DataType::Float64)
                    }
                } else {
                    Ok(field.data_type().clone())
                }
            }
            Some("TIME") => {
                let scale = utils::scale(field)?;
                if !(0..=9).contains(&scale) {
                    return InvalidScaleSnafu {
                        scale,
                        logical_type: "TIME",
                    }
                    .fail();
                }
                Ok(time::data_type(scale))
            }
            Some("INTERVAL_DAY_TIME") => Ok(DataType::Duration(TimeUnit::Nanosecond)),
            Some("TIMESTAMP_NTZ") => timestamp::schema_type(
                field,
                "TIMESTAMP_NTZ",
                None,
                self.force_microsecond_precision,
            ),
            Some("TIMESTAMP_LTZ") => timestamp::schema_type(
                field,
                "TIMESTAMP_LTZ",
                self.timezone.name(),
                self.force_microsecond_precision,
            ),
            Some("TIMESTAMP_TZ") => timestamp::schema_type(
                field,
                "TIMESTAMP_TZ",
                self.timezone.name(),
                self.force_microsecond_precision,
            ),
            Some("DECFLOAT") => UnsupportedSnowflakeTypeSnafu {
                logical_type: "DECFLOAT",
            }
            .fail(),
            Some(logical @ ("ARRAY" | "MAP" | "OBJECT" | "VARIANT")) => {
                reject_nested_arrow_type(logical, field.data_type())?;
                Ok(field.data_type().clone())
            }
            _ => Ok(field.data_type().clone()),
        }
    }

    fn convert_column(&self, field: &Field, column: ArrayRef) -> Result<ArrayRef, PlanError> {
        match logical_type(field) {
            Some("FIXED") => fixed::convert(field, column, self.number_to_decimal),
            Some("TIME") => time::convert(field, column),
            Some("INTERVAL_DAY_TIME") => interval::convert(field, column),
            Some("TIMESTAMP_NTZ") => {
                timestamp::convert_ntz(field, column, self.force_microsecond_precision)
            }
            Some("TIMESTAMP_LTZ") => timestamp::convert_ltz(
                field,
                column,
                self.force_microsecond_precision,
                self.timezone.name(),
            ),
            Some("TIMESTAMP_TZ") => timestamp::convert_tz(
                field,
                column,
                self.force_microsecond_precision,
                self.timezone.name(),
            ),
            Some("DECFLOAT") => UnsupportedSnowflakeTypeSnafu {
                logical_type: "DECFLOAT",
            }
            .fail(),
            Some(logical @ ("ARRAY" | "MAP" | "OBJECT" | "VARIANT")) => {
                reject_nested_arrow_type(logical, column.data_type())?;
                Ok(column)
            }
            _ => Ok(column),
        }
    }
}

fn logical_type(field: &Field) -> Option<&str> {
    field.metadata().get("logicalType").map(String::as_str)
}

fn field_needs_conversion(field: &Field, data_type: &DataType) -> Result<bool, PlanError> {
    match logical_type(field) {
        Some("FIXED") => {
            Ok(utils::scale_or(field, 0)? > 0 && !matches!(data_type, DataType::Decimal128(_, _)))
        }
        Some("TIME")
        | Some("INTERVAL_DAY_TIME")
        | Some("TIMESTAMP_NTZ")
        | Some("TIMESTAMP_LTZ")
        | Some("TIMESTAMP_TZ")
        | Some("DECFLOAT") => Ok(true),
        Some(logical @ ("ARRAY" | "MAP" | "OBJECT" | "VARIANT")) => {
            reject_nested_arrow_type(logical, data_type)?;
            Ok(false)
        }
        _ => Ok(false),
    }
}

// Passthrough keeps the wire Field (and its metadata). A type change builds a
// new Field with only name and nullability — Cython drops logicalType/scale
// on rewritten columns.
fn converted_field(field: FieldRef, data_type: DataType) -> Field {
    if data_type == *field.data_type() {
        Arc::unwrap_or_clone(field)
    } else {
        Field::new(field.name(), data_type, field.is_nullable())
    }
}

#[cfg(test)]
mod tests;
