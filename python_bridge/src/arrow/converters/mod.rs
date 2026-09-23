mod binary;
mod boolean;
mod context;
mod date;
mod decfloat;
mod decode;
mod interval;
mod number;
mod numpy;
mod real;
mod text;
mod time;
mod timestamp_ltz;
mod timestamp_ntz;
mod timestamp_tz;
mod timezone;
mod util;
mod vector;

#[cfg(test)]
mod test_util;

use arrow::array::{
    BinaryArray, BooleanArray, Date32Array, FixedSizeListArray, Float64Array, StringArray,
    StructArray,
};
use pyo3::prelude::*;
use sf_types::{
    SnowflakeBinary, SnowflakeBoolean, SnowflakeDate, SnowflakeDecfloat, SnowflakeReal,
    SnowflakeText, SnowflakeTimestampTz, SnowflakeVector,
};

use self::binary::BinaryMaterializer;
use self::boolean::BoolMaterializer;
use self::date::{DateMaterializer, DateNumpyMaterializer};
use self::decfloat::{DecfloatMaterializer, DecfloatNumpyMaterializer};
use self::decode::TypedColumn;
use self::interval::{IntervalDayTimeColumn, IntervalYearMonthColumn};
use self::number::{
    NumberColumn, NumberMaterializer, NumberNumpyFloatMaterializer, NumberNumpyIntMaterializer,
};
use self::real::{RealMaterializer, RealNumpyMaterializer};
use self::text::TextMaterializer;
use self::time::TimeColumn;
use self::timestamp_ltz::TimestampLtzColumn;
use self::timestamp_ntz::TimestampNtzColumn;
use self::timestamp_tz::TimestampTzMaterializer;
use self::vector::VectorMaterializer;

pub(crate) use context::{ConversionContext, RowShape};

pub(crate) enum Column {
    Bool(TypedColumn<BooleanArray, SnowflakeBoolean, BoolMaterializer>),
    Number(NumberColumn<NumberMaterializer>),
    NumberNumpyInt(NumberColumn<NumberNumpyIntMaterializer>),
    NumberNumpyFloat(NumberColumn<NumberNumpyFloatMaterializer>),
    Real(TypedColumn<Float64Array, SnowflakeReal, RealMaterializer>),
    RealNumpy(TypedColumn<Float64Array, SnowflakeReal, RealNumpyMaterializer>),
    Text(TypedColumn<StringArray, SnowflakeText, TextMaterializer>),
    Binary(TypedColumn<BinaryArray, SnowflakeBinary, BinaryMaterializer>),
    Decfloat(TypedColumn<StructArray, SnowflakeDecfloat, DecfloatMaterializer>),
    DecfloatNumpy(TypedColumn<StructArray, SnowflakeDecfloat, DecfloatNumpyMaterializer>),
    Date(TypedColumn<Date32Array, SnowflakeDate, DateMaterializer>),
    DateNumpy(TypedColumn<Date32Array, SnowflakeDate, DateNumpyMaterializer>),
    Time(TimeColumn),
    TimestampNtz(TimestampNtzColumn),
    TimestampLtz(TimestampLtzColumn),
    TimestampTz(TypedColumn<StructArray, SnowflakeTimestampTz, TimestampTzMaterializer>),
    IntervalYearMonth(IntervalYearMonthColumn),
    IntervalDayTime(IntervalDayTimeColumn),
    Vector(TypedColumn<FixedSizeListArray, SnowflakeVector, VectorMaterializer>),
}

impl Column {
    pub(crate) fn to_py<'py>(&self, py: Python<'py>, row: usize) -> PyResult<Bound<'py, PyAny>> {
        match self {
            Self::Bool(column) => column.to_py(py, row),
            Self::Number(column) => column.to_py(py, row),
            Self::NumberNumpyInt(column) => column.to_py(py, row),
            Self::NumberNumpyFloat(column) => column.to_py(py, row),
            Self::Real(column) => column.to_py(py, row),
            Self::RealNumpy(column) => column.to_py(py, row),
            Self::Text(column) => column.to_py(py, row),
            Self::Binary(column) => column.to_py(py, row),
            Self::Decfloat(column) => column.to_py(py, row),
            Self::DecfloatNumpy(column) => column.to_py(py, row),
            Self::Date(column) => column.to_py(py, row),
            Self::DateNumpy(column) => column.to_py(py, row),
            Self::Time(column) => column.to_py(py, row),
            Self::TimestampNtz(column) => column.to_py(py, row),
            Self::TimestampLtz(column) => column.to_py(py, row),
            Self::TimestampTz(column) => column.to_py(py, row),
            Self::IntervalYearMonth(column) => column.to_py(py, row),
            Self::IntervalDayTime(column) => column.to_py(py, row),
            Self::Vector(column) => column.to_py(py, row),
        }
    }
}
