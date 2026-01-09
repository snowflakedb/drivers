use arrow::datatypes::DataType;
use snafu::{Location, Snafu};

use crate::cdata_types::CDataType;

/// Error type for data conversion operations between Arrow, Snowflake, and ODBC types.
#[derive(Snafu, Debug)]
#[snafu(visibility(pub))]
pub enum ConversionError {
    /// The target ODBC type is not supported for the given Snowflake/Arrow source type.
    #[snafu(display("Target ODBC type '{target_type:?}' is not supported for this conversion"))]
    UnsupportedOdbcType {
        target_type: CDataType,
        #[snafu(implicit)]
        location: Location,
    },

    /// The Arrow data type cannot be processed or converted.
    #[snafu(display("Arrow data type '{data_type:?}' is not supported"))]
    UnsupportedArrowDataType {
        data_type: DataType,
        #[snafu(implicit)]
        location: Location,
    },

    /// Failed to downcast an Arrow array to the expected type.
    #[snafu(display("Failed to downcast Arrow array to expected type={expected_type}"))]
    ArrowArrayDowncast {
        expected_type: String,
        #[snafu(implicit)]
        location: Location,
    },

    /// Required field metadata (like scale or precision) is missing.
    #[snafu(display("Required field metadata '{key}' is missing for field '{field_name}'"))]
    MissingFieldMetadata {
        key: String,
        field_name: String,
        #[snafu(implicit)]
        location: Location,
    },

    /// Field metadata exists but has an invalid value.
    #[snafu(display(
        "Field metadata '{key}' for field '{field_name}' has invalid value: {reason}"
    ))]
    InvalidFieldMetadata {
        key: String,
        field_name: String,
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    /// Field metadata logical type is incompatible with the requested operation or data type.
    #[snafu(display(
        "Field metadata logical type '{logical_type}' is incompatible with data type '{data_type:?}'"
    ))]
    IncompatibleFieldMetadata {
        logical_type: String,
        data_type: DataType,
        #[snafu(implicit)]
        location: Location,
    },

    /// Failed to parse a numeric value during conversion.
    #[snafu(display("Failed to parse numeric value: {reason}"))]
    NumericParsing {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse field={field_name} metadata={key}: {reason}"))]
    FieldMetadataParsing {
        field_name: String,
        key: String,
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },
}
