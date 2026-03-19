use std::str;

use arrow::datatypes::DataType;
use error_trace::ErrorTrace;
use odbc_sys as sql;
use snafu::{Location, Snafu};

use crate::{api::CDataType, conversion::parsers::numeric_literal_parser::NumericParsingError};

#[derive(Snafu, Debug, ErrorTrace)]
#[snafu(visibility(pub))]
pub enum ReadArrowError {
    #[snafu(display("Value is null"))]
    NullValue {
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Snafu, Debug, ErrorTrace)]
#[snafu(visibility(pub))]
pub enum WriteOdbcError {
    InvalidValue {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to parse value as numeric: {reason}"))]
    RustParsing {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse value as numeric: {source:?}"))]
    NumericLiteralParsing {
        source: NumericParsingError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Numeric value out of range: {reason}"))]
    NumericValueOutOfRange {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Indicator variable required but not supplied"))]
    IndicatorVariableRequired {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Interval field overflow: {reason}"))]
    IntervalFieldOverflow {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    /// The target ODBC type is not supported for the given Snowflake/Arrow source type.
    #[snafu(display("Target ODBC type '{target_type:?}' is not supported for this conversion"))]
    UnsupportedOdbcType {
        target_type: CDataType,
        #[snafu(implicit)]
        location: Location,
    },

    /// Indicator variable required but not supplied (SQLSTATE 22002).
    /// Returned when data is NULL but StrLen_or_IndPtr is a null pointer.
    #[snafu(display("Indicator variable required but not supplied"))]
    IndicatorRequired {
        #[snafu(implicit)]
        location: Location,
    },
}

/// Error type for data conversion operations between Arrow, Snowflake, and ODBC types.
#[derive(Snafu, Debug, ErrorTrace)]
#[snafu(visibility(pub))]
pub enum ConversionError {
    #[snafu(display("Failed to read arrow value"))]
    ReadArrowValue {
        source: ReadArrowError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to write ODBC value"))]
    WriteOdbcValue {
        source: WriteOdbcError,
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
    #[snafu(display("Failed to parse field={field_name} metadata={key}: {reason}"))]
    FieldMetadataParsing {
        field_name: String,
        key: String,
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Snafu, ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum JsonBindingError {
    #[snafu(display("Parameter bindings must be contiguous and start at 1"))]
    InvalidParameterIndices {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported SQL parameter type: {sql_type:?}"))]
    UnsupportedParameterType {
        sql_type: sql::SqlDataType,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported C data type for JSON binding: {c_type:?}"))]
    UnsupportedCDataType {
        c_type: CDataType,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Null parameter value pointer encountered"))]
    NullPointer {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Parameter value is not valid UTF-8: {source}"))]
    InvalidUtf8 {
        source: str::Utf8Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[cfg(windows)]
    #[snafu(display("Failed to convert ANSI code page string to UTF-8"))]
    AcpConversion {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Wide-character (WChar) parameter is not valid UTF-16"))]
    WCharConversion {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to serialize bindings to JSON: {source}"))]
    Serialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
}
