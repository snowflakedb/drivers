//! Schema plan: Arrow field metadata → [`SnowflakeFieldType`].
//!
//! Copied from the ODBC conversion layer. This module does not depend on the
//! `odbc` crate; shared extraction into a common crate is deferred.
//!
//! Source: `odbc/src/conversion/mod.rs` (`SnowflakeFieldType`, `from_field`,
//! `get_field_metadata`, `timestamp_scale`) and
//! `odbc/src/conversion/vector.rs` (`VectorElementType`). Bind-side SQL type
//! reporting and `NumericSettings` are omitted; missing VARCHAR `charLength`
//! / BINARY `byteLength` use the same defaults ODBC uses.

use arrow::datatypes::{DataType, Field, Schema};

use crate::arrow::error::{
    InvalidMetadataSnafu, MissingLogicalTypeSnafu, MissingMetadataSnafu, PlanError,
    UnsupportedLogicalTypeSnafu,
};

/// Copied from `odbc/src/conversion/number.rs` (`SF_DEFAULT_VARCHAR_MAX_LEN`).
const SF_DEFAULT_VARCHAR_MAX_LEN: u32 = 16_777_216;

/// Copied from `odbc/src/conversion/mod.rs` (`from_field` BINARY arm): fallback
/// when `byteLength` is omitted (8 MB).
const DEFAULT_BINARY_BYTE_LENGTH: u32 = 8_388_608;

/// Copied from `odbc/src/conversion/vector.rs` (`VectorElementType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VectorElementType {
    Int32,
    Float32,
}

/// Parsed Snowflake type from an Arrow field's metadata.
///
/// Copied from `odbc/src/conversion/mod.rs` (`SnowflakeFieldType`). Variants
/// hold the same data the ODBC `Snowflake*` types store; `BOOLEAN` is a ZST
/// there (`SnowflakeBoolean`) so it is a unit variant here until that reader
/// lands. `OBJECT` / `ARRAY` / `VARIANT` are `Varchar` with
/// `is_semi_structured: true`, matching ODBC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnowflakeFieldType {
    Varchar {
        len: u32,
        is_semi_structured: bool,
    },
    Number {
        scale: u32,
        precision: u32,
    },
    Date,
    Time {
        scale: u32,
    },
    TimestampNtz {
        scale: u32,
    },
    TimestampLtz {
        scale: u32,
    },
    TimestampTz {
        scale: u32,
    },
    Boolean,
    Binary {
        len: u32,
    },
    Real,
    Decfloat {
        precision: u32,
    },
    Vector {
        element_type: VectorElementType,
        column_size: u32,
    },
}

impl SnowflakeFieldType {
    /// Backend `logicalType` name this variant was parsed from.
    pub(crate) fn logical_type_name(&self) -> &'static str {
        match self {
            Self::Varchar {
                is_semi_structured: false,
                ..
            } => "TEXT",
            Self::Varchar {
                is_semi_structured: true,
                ..
            } => "VARIANT",
            Self::Number { .. } => "FIXED",
            Self::Date => "DATE",
            Self::Time { .. } => "TIME",
            Self::TimestampNtz { .. } => "TIMESTAMP_NTZ",
            Self::TimestampLtz { .. } => "TIMESTAMP_LTZ",
            Self::TimestampTz { .. } => "TIMESTAMP_TZ",
            Self::Boolean => "BOOLEAN",
            Self::Binary { .. } => "BINARY",
            Self::Real => "REAL",
            Self::Decfloat { .. } => "DECFLOAT",
            Self::Vector { .. } => "VECTOR",
        }
    }

    /// Copied from `odbc/src/conversion/mod.rs` (`SnowflakeFieldType::from_field`).
    pub(crate) fn from_field(field: &Field) -> Result<Self, PlanError> {
        let logical_type = field
            .metadata()
            .get("logicalType")
            .ok_or_else(|| {
                MissingLogicalTypeSnafu {
                    column: field.name().to_string(),
                }
                .build()
            })?
            .as_str();
        match logical_type {
            "TEXT" => {
                let len = match get_field_metadata(field, "charLength") {
                    Ok(len) => len,
                    Err(PlanError::MissingMetadata { .. }) => SF_DEFAULT_VARCHAR_MAX_LEN,
                    Err(e) => return Err(e),
                };
                Ok(Self::Varchar {
                    len,
                    is_semi_structured: false,
                })
            }
            "FIXED" => Ok(Self::Number {
                scale: get_field_metadata(field, "scale")?,
                precision: get_field_metadata(field, "precision")?,
            }),
            "DATE" => Ok(Self::Date),
            "TIME" => Ok(Self::Time {
                scale: get_field_metadata(field, "scale")?,
            }),
            "TIMESTAMP_NTZ" => Ok(Self::TimestampNtz {
                scale: timestamp_scale(field)?,
            }),
            "TIMESTAMP_LTZ" => Ok(Self::TimestampLtz {
                scale: timestamp_scale(field)?,
            }),
            "TIMESTAMP_TZ" => Ok(Self::TimestampTz {
                scale: timestamp_scale(field)?,
            }),
            "BOOLEAN" => Ok(Self::Boolean),
            "BINARY" => {
                let len = match get_field_metadata(field, "byteLength") {
                    Ok(len) => len,
                    Err(PlanError::MissingMetadata { .. }) => DEFAULT_BINARY_BYTE_LENGTH,
                    Err(e) => return Err(e),
                };
                Ok(Self::Binary { len })
            }
            "REAL" => Ok(Self::Real),
            "DECFLOAT" => Ok(Self::Decfloat {
                precision: get_field_metadata(field, "precision")?,
            }),
            "OBJECT" | "ARRAY" | "VARIANT" => {
                let len = match get_field_metadata(field, "charLength") {
                    Ok(len) => len,
                    Err(PlanError::MissingMetadata { .. }) => SF_DEFAULT_VARCHAR_MAX_LEN,
                    Err(e) => return Err(e),
                };
                Ok(Self::Varchar {
                    len,
                    is_semi_structured: true,
                })
            }
            "VECTOR" => {
                let element_type = match field.data_type() {
                    DataType::FixedSizeList(child_field, _) => match child_field.data_type() {
                        DataType::Int32 => VectorElementType::Int32,
                        DataType::Float32 => VectorElementType::Float32,
                        dt => {
                            return UnsupportedLogicalTypeSnafu {
                                logical_type: format!("VECTOR with unsupported child type {dt:?}"),
                                column: field.name().to_string(),
                            }
                            .fail();
                        }
                    },
                    _ => {
                        return UnsupportedLogicalTypeSnafu {
                            logical_type: "VECTOR".to_string(),
                            column: field.name().to_string(),
                        }
                        .fail();
                    }
                };
                let column_size = match get_field_metadata(field, "charLength") {
                    Ok(len) => len,
                    Err(PlanError::MissingMetadata { .. }) => SF_DEFAULT_VARCHAR_MAX_LEN,
                    Err(e) => return Err(e),
                };
                Ok(Self::Vector {
                    element_type,
                    column_size,
                })
            }
            lt => UnsupportedLogicalTypeSnafu {
                logical_type: lt.to_string(),
                column: field.name().to_string(),
            }
            .fail(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LogicalPlan {
    pub(crate) field_types: Vec<SnowflakeFieldType>,
}

impl LogicalPlan {
    pub(crate) fn from_schema(schema: &Schema) -> Result<Self, PlanError> {
        let field_types = schema
            .fields()
            .iter()
            .map(|field| SnowflakeFieldType::from_field(field.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { field_types })
    }
}

/// Copied from `odbc/src/conversion/mod.rs` (`get_field_metadata`).
fn get_field_metadata(field: &Field, key: &str) -> Result<u32, PlanError> {
    let value = field.metadata().get(key).ok_or_else(|| {
        MissingMetadataSnafu {
            key: key.to_string(),
            column: field.name().to_string(),
        }
        .build()
    })?;
    value.parse().map_err(|_| {
        InvalidMetadataSnafu {
            key: key.to_string(),
            value: value.clone(),
        }
        .build()
    })
}

/// Copied from `odbc/src/conversion/mod.rs` (`timestamp_scale`). Tracing
/// warnings are omitted.
fn timestamp_scale(field: &Field) -> Result<u32, PlanError> {
    match get_field_metadata(field, "scale") {
        Ok(scale) if scale > 9 => Ok(9),
        Ok(scale) => Ok(scale),
        Err(PlanError::MissingMetadata { .. }) => Ok(9),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use arrow::datatypes::{DataType, Field, Schema};
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

    use super::*;
    use crate::arrow::error::{MissingLogicalTypeSnafu, PlanError};

    fn field_with_metadata(
        name: &str,
        data_type: DataType,
        metadata: HashMap<String, String>,
    ) -> Field {
        Field::new(name, data_type, true).with_metadata(metadata)
    }

    fn logical_meta(logical_type: &str, extra: &[(&str, &str)]) -> HashMap<String, String> {
        let mut metadata = HashMap::from([("logicalType".to_string(), logical_type.to_string())]);
        for (key, value) in extra {
            metadata.insert((*key).to_string(), (*value).to_string());
        }
        metadata
    }

    #[test]
    fn from_schema_plans_supported_field_types() {
        let schema = Schema::new(vec![
            field_with_metadata(
                "n0",
                DataType::Int64,
                logical_meta("FIXED", &[("scale", "0"), ("precision", "38")]),
            ),
            field_with_metadata("s", DataType::Utf8, logical_meta("TEXT", &[])),
            field_with_metadata("v", DataType::Utf8, logical_meta("VARIANT", &[])),
            field_with_metadata("b", DataType::Boolean, logical_meta("BOOLEAN", &[])),
            field_with_metadata(
                "t",
                DataType::Int64,
                logical_meta("TIME", &[("scale", "3")]),
            ),
            field_with_metadata(
                "ntz",
                DataType::Int64,
                logical_meta("TIMESTAMP_NTZ", &[("scale", "6")]),
            ),
        ]);

        let plan = LogicalPlan::from_schema(&schema).unwrap();
        assert_eq!(
            plan.field_types,
            vec![
                SnowflakeFieldType::Number {
                    scale: 0,
                    precision: 38,
                },
                SnowflakeFieldType::Varchar {
                    len: SF_DEFAULT_VARCHAR_MAX_LEN,
                    is_semi_structured: false,
                },
                SnowflakeFieldType::Varchar {
                    len: SF_DEFAULT_VARCHAR_MAX_LEN,
                    is_semi_structured: true,
                },
                SnowflakeFieldType::Boolean,
                SnowflakeFieldType::Time { scale: 3 },
                SnowflakeFieldType::TimestampNtz { scale: 6 },
            ]
        );
    }

    #[test]
    fn from_schema_errors_on_unknown_logical_type() {
        let schema = Schema::new(vec![field_with_metadata(
            "x",
            DataType::Int64,
            logical_meta("NOT_A_TYPE", &[]),
        )]);
        let err = LogicalPlan::from_schema(&schema).unwrap_err();
        match err {
            PlanError::UnsupportedLogicalType {
                logical_type,
                column,
                ..
            } => {
                assert_eq!(logical_type, "NOT_A_TYPE");
                assert_eq!(column, "x");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn from_schema_errors_on_missing_logical_type() {
        let schema = Schema::new(vec![Field::new("x", DataType::Int64, true)]);
        let err = LogicalPlan::from_schema(&schema).unwrap_err();
        match err {
            PlanError::MissingLogicalType { column, .. } => assert_eq!(column, "x"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn from_schema_errors_on_missing_scale() {
        let schema = Schema::new(vec![field_with_metadata(
            "t",
            DataType::Int64,
            logical_meta("TIME", &[]),
        )]);
        let err = LogicalPlan::from_schema(&schema).unwrap_err();
        match err {
            PlanError::MissingMetadata { key, column, .. } => {
                assert_eq!(key, "scale");
                assert_eq!(column, "t");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn from_schema_errors_on_invalid_scale_metadata() {
        let schema = Schema::new(vec![field_with_metadata(
            "t",
            DataType::Int64,
            logical_meta("TIME", &[("scale", "nope")]),
        )]);
        let err = LogicalPlan::from_schema(&schema).unwrap_err();
        match err {
            PlanError::InvalidMetadata { key, value, .. } => {
                assert_eq!(key, "scale");
                assert_eq!(value, "nope");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn plan_error_maps_to_value_error() {
        Python::initialize();
        let err: PyErr = MissingLogicalTypeSnafu {
            column: "c".to_string(),
        }
        .build()
        .into();
        Python::attach(|py| {
            assert!(err.is_instance_of::<PyValueError>(py));
        });
    }
}
