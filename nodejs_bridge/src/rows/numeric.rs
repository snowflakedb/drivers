use arrow::array::{
    Decimal128Array, Float32Array, Float64Array, Int8Array, Int16Array, Int32Array, Int64Array,
};
use sf_types::{ReadArrowError, ReadArrowType, SnowflakeFixed, SnowflakeReal};

/// A Snowflake integer column kept on the physical Arrow width the server sent,
/// widened to `i128` only when a cell is read. The FIXED mantissa,
/// `INTERVAL_YEAR_MONTH` month count, and `INTERVAL_DAY_TIME` nanosecond count
/// share this representation: all three arrive as a single signed integer whose
/// width varies with magnitude.
pub(super) enum IntColumn {
    I8(Int8Array),
    I16(Int16Array),
    I32(Int32Array),
    I64(Int64Array),
    Decimal(Decimal128Array),
}

impl IntColumn {
    pub(super) fn get(&self, row_index: usize) -> Result<i128, ReadArrowError> {
        match self {
            Self::I8(array) => SnowflakeFixed.read_arrow_type(array, row_index),
            Self::I16(array) => SnowflakeFixed.read_arrow_type(array, row_index),
            Self::I32(array) => SnowflakeFixed.read_arrow_type(array, row_index),
            Self::I64(array) => SnowflakeFixed.read_arrow_type(array, row_index),
            Self::Decimal(array) => SnowflakeFixed.read_arrow_type(array, row_index),
        }
    }
}

pub(super) enum FloatColumn {
    F32(Float32Array),
    F64(Float64Array),
}

impl FloatColumn {
    pub(super) fn get(&self, row_index: usize) -> Result<f64, ReadArrowError> {
        match self {
            Self::F32(array) => SnowflakeReal.read_arrow_type(array, row_index),
            Self::F64(array) => SnowflakeReal.read_arrow_type(array, row_index),
        }
    }
}

pub(super) fn real_string(value: f64) -> String {
    if value.is_infinite() {
        return if value > 0.0 { "inf" } else { "-inf" }.to_owned();
    }
    ryu_js::Buffer::new().format(value).to_owned()
}

/// Renders `unscaled x 10^-scale` as an exact decimal string — every digit the
/// server sent, no rounding.
///
/// FIXED type is handed to JS in this form rather than as an `f64` so the numeric
/// policy lives on the JS side: `Number()` there reproduces the old driver
/// exactly (it applied `Number()` to the server's decimal text)
///
/// Same three-case shape as ODBC's `format_decimal_into`
/// (`odbc/src/conversion/number.rs`), which writes into a fixed-size buffer for
/// zero-allocation formatting; this version allocates a `String` instead since
/// napi calls aren't as allocation-sensitive as ODBC's `SQLGetData` hot path.
pub(super) fn decimal_string(unscaled: i128, scale: u32) -> String {
    let sign = if unscaled.is_negative() { "-" } else { "" };
    let digits = unscaled.unsigned_abs().to_string();
    let scale = scale as usize;
    if scale == 0 {
        return format!("{sign}{digits}");
    }
    if let Some(split) = digits.len().checked_sub(scale).filter(|split| *split > 0) {
        let (int_part, frac_part) = digits.split_at(split);
        format!("{sign}{int_part}.{frac_part}")
    } else {
        format!("{sign}0.{digits:0>scale$}")
    }
}
