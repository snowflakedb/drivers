use arrow::array::{Int32Array, Int64Array, StructArray};
use chrono::{DateTime, NaiveDateTime, NaiveTime, Utc};
use sf_output_format::datetime::{DateTimeFormat, DateTimeValue, Zone};
use std::borrow::Cow;
use std::sync::Arc;

use sf_types::{ReadArrowError, ReadArrowType, SnowflakeTime, SnowflakeTimestampNtz};

use super::js_cell::JsCell;
use super::row_options::ColumnOutput;
use crate::date_format::{ResolvedTimezone, compiled_format};

pub(super) enum TimeColumn {
    I32(Int32Array),
    I64(Int64Array),
}

impl TimeColumn {
    pub(super) fn get(&self, scale: u32, row_index: usize) -> Result<NaiveTime, ReadArrowError> {
        let reader = SnowflakeTime { scale };
        match self {
            Self::I32(array) => reader.read_arrow_type(array, row_index),
            Self::I64(array) => reader.read_arrow_type(array, row_index),
        }
    }
}

/// A TIMESTAMP_LTZ or TIMESTAMP_NTZ column: a struct of epoch and fraction, or
/// a single scaled epoch integer. Both types share one physical encoding and
/// differ only in the timezone their [`SnowflakeDateFormat`] carries.
pub(super) enum TimestampColumn {
    Struct(StructArray),
    I64(Int64Array),
}

impl TimestampColumn {
    pub(super) fn get(
        &self,
        scale: u32,
        row_index: usize,
    ) -> Result<NaiveDateTime, ReadArrowError> {
        let reader = SnowflakeTimestampNtz { scale };
        match self {
            Self::Struct(array) => reader.read_arrow_type(array, row_index),
            Self::I64(array) => reader.read_arrow_type(array, row_index),
        }
    }
}

/// Renders TIME cells to strings in Rust.
pub(super) struct TimeFormat {
    pub(super) scale: u32,
    pub(super) compiled_format: Arc<DateTimeFormat>,
}

impl TimeFormat {
    /// A TIME has no calendar date and no timezone. The server formats one by
    /// reading `secs_since_midnight` as an instant, which lands on the Unix
    /// epoch at UTC, so date elements in a TIME format render 1970-01-01 and
    /// timezone elements render a zero offset.
    pub(super) fn to_cell(&self, time: NaiveTime) -> JsCell<'_> {
        JsCell::Str(Cow::Owned(self.compiled_format.format(&DateTimeValue {
            local: DateTime::UNIX_EPOCH.naive_utc().date().and_time(time),
            offset_minutes: 0,
            scale: self.scale,
            zone: Zone::Named("GMT"),
        })))
    }
}

/// Builds cells for DATE and TIMESTAMP_* columns: a `SnowflakeDate` the JS side
/// formats, or the formatted string when the column is fetched as a string.
pub(super) struct SnowflakeDateFormat {
    pub(super) scale: u32,
    pub(super) format: String,
    pub(super) timezone: ResolvedTimezone,
}

impl SnowflakeDateFormat {
    /// `offset_minutes` is the cell's own offset (TIMESTAMP_TZ); other types use
    /// the column's timezone.
    pub(super) fn to_cell(
        &self,
        utc: DateTime<Utc>,
        offset_minutes: Option<i32>,
        output: ColumnOutput,
    ) -> JsCell<'_> {
        let timezone = offset_minutes.map_or(self.timezone, ResolvedTimezone::Offset);
        match output {
            ColumnOutput::String { .. } => JsCell::Str(Cow::Owned(timezone.format(
                &compiled_format(&self.format),
                utc,
                self.scale,
            ))),
            ColumnOutput::Native => JsCell::SnowflakeDate {
                epoch_millis: utc.timestamp_millis() as f64,
                nanos: utc.timestamp_subsec_nanos(),
                scale: self.scale,
                timezone,
                format: &self.format,
            },
        }
    }
}
