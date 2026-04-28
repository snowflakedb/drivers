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

    #[snafu(display("Invalid Arrow value: {reason}"))]
    InvalidArrowValue {
        reason: String,
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

    #[snafu(display("Numeric value out of range: {reason}"))]
    NumericMagnitudeOverflow {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Binding value out of range: {reason}"))]
    BindingNumericOutOfRange {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    /// Maps to SQLSTATE 22007 ("Invalid datetime format"). Use this when a
    /// SQL_DATE_STRUCT / SQL_TIME_STRUCT / SQL_TIMESTAMP_STRUCT bound to a
    /// temporal SQL target contains field values that don't form a valid
    /// date/time (e.g. month = 13, hour = 25). Per ODBC Appendix D ("C to
    /// SQL: Date / Time / Timestamp"), the spec-mandated SQLSTATE for
    /// "Data value does not contain a valid date/time" is 22007 — distinct
    /// from 22003 (numeric out of range) and 07006 (restricted data type
    /// attribute violation, i.e. unsupported conversion).
    #[snafu(display("Invalid datetime value: {reason}"))]
    InvalidDatetimeValue {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    /// Maps to SQLSTATE 22008 ("Datetime field overflow"). Use this when a
    /// SQL_C_TYPE_TIMESTAMP source is bound to a SQL_TYPE_DATE or
    /// SQL_TYPE_TIME target and the discarded portion is non-zero. Per ODBC
    /// Appendix D ("Converting Data from C to SQL Data Types"):
    ///
    ///   - TIMESTAMP → DATE: 22008 if the time portion of the timestamp is
    ///     nonzero (any of hour / minute / second / fraction).
    ///   - TIMESTAMP → TIME: 22008 if the fractional seconds portion is
    ///     nonzero.
    ///
    /// This is distinct from 22007 (struct field outside the legal range,
    /// e.g. month=13), 22003 (numeric magnitude overflow), and 07006
    /// (unsupported conversion).
    #[snafu(display("Datetime field overflow: {reason}"))]
    DatetimeFieldOverflow {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid boolean value: {value}"))]
    InvalidBooleanValue {
        value: String,
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
