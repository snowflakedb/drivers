use arrow::array::{
    Array, BinaryArray, BooleanArray, Date32Array, FixedSizeListArray, StringArray, StructArray,
};
use arrow::buffer::NullBuffer;
use arrow::datatypes::Field;
use chrono::NaiveTime;

use super::column_source::{ColumnId, ColumnSource};
use super::decfloat::format_decfloat;
use super::js_cell::JsCell;
use super::numeric::{FloatColumn, IntColumn, decimal_string};
use super::row_options::{ColumnOutput, RowOptions};
use super::temporal::{SnowflakeDateFormat, TimeColumn, TimeFormat, TimestampColumn};
use super::vector::vector_numbers;
use crate::date_format::{ResolvedTimezone, compiled_format};
use crate::session_params::KnownSessionParameters;
use sf_types::{ReadArrowError, ReadArrowType, SnowflakeTimestampTz};
use snafu::Snafu;
use std::borrow::Cow;

/// Decodes one Arrow column into [`JsCell`]s, one cell at a time.
///
/// Use it in two steps:
/// - [`for_field`](Self::for_field) inspects the column's `logicalType` and
///   picks the matching [`ColumnDecoder`], which holds the typed array. Runs on
///   a worker thread, so anything proportional to the batch size belongs here.
/// - [`read`](Self::read) returns the [`JsCell`] for a given row, or an error
///   naming the column and row when the decoder rejects the cell's value. Runs
///   on the Node.js main thread; the caller converts the cell through
///   [`napi::bindgen_prelude::ToNapiValue`].
///
/// Nulls are resolved here from the column's null bitmap, so a
/// [`ColumnDecoder`] only ever sees non-null cells.
pub(super) struct ColumnReader {
    id: ColumnId,
    nulls: Option<NullBuffer>,
    decoder: ColumnDecoder,
    output: ColumnOutput,
}

#[derive(Snafu, Debug)]
pub(crate) enum RowDecodeError {
    #[snafu(display("{message}"))]
    Schema { message: String },
    #[snafu(display("{column} row {row}: {source}"))]
    InvalidValue {
        column: ColumnId,
        row: usize,
        source: ReadArrowError,
    },
}

enum ColumnDecoder {
    Binary(BinaryArray),
    Boolean(BooleanArray),
    Date(Date32Array, SnowflakeDateFormat),
    Decfloat(StructArray, usize),
    Fixed(IntColumn, u32),
    Interval(IntColumn),
    Real(FloatColumn),
    Text(StringArray),
    Time(TimeColumn, TimeFormat),
    Timestamp(TimestampColumn, SnowflakeDateFormat),
    TimestampTz(StructArray, SnowflakeDateFormat),
    Variant(StringArray),
    Vector(FixedSizeListArray),
}

impl ColumnReader {
    pub(super) fn for_field(
        index: usize,
        field: &Field,
        column: &dyn Array,
        session_params: &KnownSessionParameters,
        row_options: &RowOptions,
    ) -> Result<Self, RowDecodeError> {
        let logical_type =
            field
                .metadata()
                .get("logicalType")
                .ok_or_else(|| RowDecodeError::Schema {
                    message: format!("column {index} is missing logicalType metadata"),
                })?;
        let src = ColumnSource {
            field,
            column,
            id: ColumnId {
                index,
                logical_type: logical_type.clone(),
            },
        };
        let decoder = ColumnDecoder::for_field(logical_type, &src, session_params)
            .map_err(|message| RowDecodeError::Schema { message })?;
        Ok(Self {
            id: src.id,
            nulls: column.nulls().cloned(),
            output: row_options.output_for(logical_type),
            decoder,
        })
    }

    pub(super) fn read(&self, row_index: usize) -> Result<JsCell<'_>, RowDecodeError> {
        if self
            .nulls
            .as_ref()
            .is_some_and(|nulls| nulls.is_null(row_index))
        {
            return Ok(self.output.null_cell());
        }
        self.decoder
            .read(row_index, self.output)
            .map_err(|source| RowDecodeError::InvalidValue {
                column: self.id.clone(),
                row: row_index,
                source,
            })
    }
}

impl ColumnDecoder {
    fn for_field(
        logical_type: &str,
        src: &ColumnSource,
        params: &KnownSessionParameters,
    ) -> Result<Self, String> {
        Ok(match logical_type {
            "BINARY" => Self::Binary(src.array()?),
            "BOOLEAN" => Self::Boolean(src.array()?),
            "DATE" => Self::Date(
                src.array()?,
                SnowflakeDateFormat {
                    scale: 0,
                    format: params.date_output_format.clone(),
                    timezone: ResolvedTimezone::Utc,
                },
            ),
            "DECFLOAT" => Self::Decfloat(src.array()?, src.metadata("precision")?),
            "FIXED" => Self::Fixed(src.int_column()?, src.metadata("scale")?),
            "INTERVAL_YEAR_MONTH" | "INTERVAL_DAY_TIME" => Self::Interval(src.int_column()?),
            "REAL" => Self::Real(src.float_column()?),
            "TEXT" => Self::Text(src.array()?),
            "TIME" => Self::Time(
                src.time_column()?,
                TimeFormat {
                    scale: src.metadata("scale")?,
                    compiled_format: compiled_format(&params.time_output_format),
                },
            ),
            "TIMESTAMP_LTZ" => Self::Timestamp(
                src.timestamp_column()?,
                SnowflakeDateFormat {
                    scale: src.metadata("scale")?,
                    format: params.timestamp_ltz_output_format.clone(),
                    timezone: ResolvedTimezone::from_name(&params.timezone)?,
                },
            ),
            "TIMESTAMP_NTZ" => Self::Timestamp(
                src.timestamp_column()?,
                SnowflakeDateFormat {
                    scale: src.metadata("scale")?,
                    format: params.timestamp_ntz_output_format.clone(),
                    timezone: ResolvedTimezone::Utc,
                },
            ),
            "TIMESTAMP_TZ" => Self::TimestampTz(
                src.array()?,
                SnowflakeDateFormat {
                    scale: src.metadata("scale")?,
                    format: params.timestamp_tz_output_format.clone(),
                    timezone: ResolvedTimezone::Utc,
                },
            ),
            "VARIANT" | "OBJECT" | "ARRAY" => Self::Variant(src.array()?),
            "VECTOR" => Self::Vector(src.array()?),
            _ => return Err(format!("{} has an unsupported logicalType", src.id)),
        })
    }

    fn read(&self, row: usize, output: ColumnOutput) -> Result<JsCell<'_>, ReadArrowError> {
        Ok(match self {
            Self::Binary(array) => {
                JsCell::Buffer(sf_types::SnowflakeBinary.read_arrow_type(array, row)?)
            }
            Self::Boolean(array) => {
                JsCell::Bool(sf_types::SnowflakeBoolean.read_arrow_type(array, row)?)
            }
            Self::Date(array, format) => {
                let date = sf_types::SnowflakeDate.read_arrow_type(array, row)?;
                format.to_cell(date.and_time(NaiveTime::MIN).and_utc(), None, output)
            }
            Self::Decfloat(array, precision) => {
                let (sig, exp) = sf_types::SnowflakeDecfloat.read_arrow_type(array, row)?;
                JsCell::Str(Cow::Owned(format_decfloat(sig, exp, *precision)))
            }
            Self::Fixed(values, scale) => {
                JsCell::Str(Cow::Owned(decimal_string(values.get(row)?, *scale)))
            }
            Self::Interval(values) => JsCell::Str(Cow::Owned(values.get(row)?.to_string())),
            Self::Real(values) => JsCell::Number(values.get(row)?),
            Self::Text(array) | Self::Variant(array) => JsCell::Str(Cow::Borrowed(
                sf_types::SnowflakeText.read_arrow_type(array, row)?,
            )),
            Self::Time(values, format) => format.to_cell(values.get(format.scale, row)?),
            Self::Timestamp(values, format) => {
                format.to_cell(values.get(format.scale, row)?.and_utc(), None, output)
            }
            Self::TimestampTz(array, format) => {
                let instant = SnowflakeTimestampTz {
                    scale: format.scale,
                }
                .read_arrow_type(array, row)?;
                format.to_cell(instant.utc.and_utc(), Some(instant.offset_minutes), output)
            }
            Self::Vector(array) => JsCell::NumberArray(vector_numbers(array, row)?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::date_format::ResolvedTimezone;
    use crate::rows::row_options::FetchAsStringType;
    use arrow::array::{
        BinaryArray, BooleanArray, Decimal128Array, FixedSizeListArray, Float32Array, Float64Array,
        Int8Array, Int16Array, Int32Array, Int64Array, StringArray, StructArray,
    };
    use arrow::buffer::NullBuffer;
    use arrow::datatypes::{DataType, Date32Type, Field};
    use chrono::{NaiveDate, NaiveTime};
    use std::borrow::Cow;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn field(logical: &str, data_type: DataType, extra: &[(&str, &str)]) -> Field {
        let mut metadata = HashMap::new();
        metadata.insert("logicalType".to_string(), logical.to_string());
        for (key, value) in extra {
            metadata.insert((*key).to_string(), (*value).to_string());
        }
        Field::new("C", data_type, true).with_metadata(metadata)
    }

    fn session_params(time_format: &str) -> Arc<KnownSessionParameters> {
        Arc::new(KnownSessionParameters {
            time_output_format: time_format.to_string(),
            timestamp_tz_output_format: "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            ..KnownSessionParameters::defaults()
        })
    }

    fn reader_with_format(field: &Field, column: &dyn Array, time_format: &str) -> ColumnReader {
        ColumnReader::for_field(
            0,
            field,
            column,
            &session_params(time_format),
            &RowOptions::default(),
        )
        .unwrap()
    }

    fn date_as_string() -> RowOptions {
        RowOptions {
            fetch_as_string: vec![FetchAsStringType::Date],
            represent_null_as_string_null: false,
        }
    }

    fn try_reader_with_options(
        field: &Field,
        column: &dyn Array,
        options: &RowOptions,
    ) -> Result<ColumnReader, String> {
        ColumnReader::for_field(0, field, column, &session_params("HH24:MI:SS"), options)
            .map_err(|err| err.to_string())
    }

    fn try_reader(field: &Field, column: &dyn Array) -> Result<ColumnReader, String> {
        try_reader_with_options(field, column, &RowOptions::default())
    }

    fn string_reader(field: &Field, column: &dyn Array) -> ColumnReader {
        try_reader_with_options(field, column, &date_as_string()).unwrap()
    }

    fn reader(field: &Field, column: &dyn Array) -> ColumnReader {
        reader_with_format(field, column, "HH24:MI:SS")
    }

    fn date_value(date: NaiveDate) -> JsCell<'static> {
        let midnight = date.and_time(NaiveTime::MIN).and_utc();
        JsCell::SnowflakeDate {
            epoch_millis: midnight.timestamp_millis() as f64,
            nanos: 0,
            scale: 0,
            timezone: ResolvedTimezone::Utc,
            format: "YYYY-MM-DD",
        }
    }

    fn str_cell(s: &str) -> JsCell<'_> {
        JsCell::Str(Cow::Borrowed(s))
    }

    #[test]
    fn boolean_reads_true_false_and_null() {
        let field = field("BOOLEAN", DataType::Boolean, &[]);
        let array = BooleanArray::from(vec![Some(true), Some(false), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader.decoder, ColumnDecoder::Boolean(_)),
            "BOOLEAN should route to the Boolean arm"
        );
        assert_eq!(reader.read(0).unwrap(), JsCell::Bool(true));
        assert_eq!(reader.read(1).unwrap(), JsCell::Bool(false));
        assert_eq!(reader.read(2).unwrap(), JsCell::Null);
    }

    #[test]
    fn binary_reads_bytes_and_null() {
        let field = field("BINARY", DataType::Binary, &[]);
        let array = BinaryArray::from(vec![Some(b"\xab\xcd".as_slice()), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader.decoder, ColumnDecoder::Binary(_)),
            "BINARY should route to the Binary arm"
        );
        assert_eq!(reader.read(0).unwrap(), JsCell::Buffer(&[0xab, 0xcd]));
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
    }

    #[test]
    fn date_reads_utc_midnight_and_null() {
        let date = NaiveDate::from_ymd_opt(2016, 1, 21).unwrap();
        let field = field("DATE", DataType::Date32, &[]);
        let array = Date32Array::from(vec![Some(Date32Type::from_naive_date(date)), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader.decoder, ColumnDecoder::Date(_, _)),
            "DATE should route to the Date arm"
        );
        assert_eq!(reader.read(0).unwrap(), date_value(date));
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
    }

    #[test]
    fn date_reads_boundaries_epoch_and_leap_day() {
        let dates = [
            NaiveDate::from_ymd_opt(1, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(1969, 12, 31).unwrap(),
            NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(),
            NaiveDate::from_ymd_opt(9999, 12, 31).unwrap(),
        ];
        let field = field("DATE", DataType::Date32, &[]);
        let array = Date32Array::from(
            dates
                .iter()
                .map(|d| Some(Date32Type::from_naive_date(*d)))
                .collect::<Vec<_>>(),
        );
        let reader = reader(&field, &array);
        for (row, date) in dates.iter().enumerate() {
            assert_eq!(reader.read(row).unwrap(), date_value(*date));
        }
    }

    fn vector_field(child: DataType, dimension: i32) -> Field {
        let child_field = Arc::new(Field::new("item", child, false));
        field(
            "VECTOR",
            DataType::FixedSizeList(child_field, dimension),
            &[],
        )
    }

    fn int_vector_array(rows: &[Option<Vec<i32>>], dimension: i32) -> FixedSizeListArray {
        let child_field = Arc::new(Field::new("item", DataType::Int32, false));
        let flat: Vec<i32> = rows
            .iter()
            .flat_map(|row| row.clone().unwrap_or_else(|| vec![0; dimension as usize]))
            .collect();
        let nulls = NullBuffer::from(rows.iter().map(Option::is_some).collect::<Vec<bool>>());
        FixedSizeListArray::try_new(
            child_field,
            dimension,
            Arc::new(Int32Array::from(flat)),
            Some(nulls),
        )
        .unwrap()
    }

    fn float_vector_array(rows: &[Option<Vec<f32>>], dimension: i32) -> FixedSizeListArray {
        let child_field = Arc::new(Field::new("item", DataType::Float32, false));
        let flat: Vec<f32> = rows
            .iter()
            .flat_map(|row| row.clone().unwrap_or_else(|| vec![0.0; dimension as usize]))
            .collect();
        let nulls = NullBuffer::from(rows.iter().map(Option::is_some).collect::<Vec<bool>>());
        FixedSizeListArray::try_new(
            child_field,
            dimension,
            Arc::new(Float32Array::from(flat)),
            Some(nulls),
        )
        .unwrap()
    }

    #[test]
    fn vector_int_reads_number_array_and_null() {
        let field = vector_field(DataType::Int32, 3);
        let array = int_vector_array(&[Some(vec![1, 2, 3]), None, Some(vec![4, 5, 6])], 3);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader.decoder, ColumnDecoder::Vector(_)),
            "VECTOR should route to the Vector arm"
        );
        assert_eq!(
            reader.read(0).unwrap(),
            JsCell::NumberArray(vec![1.0, 2.0, 3.0])
        );
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
        assert_eq!(
            reader.read(2).unwrap(),
            JsCell::NumberArray(vec![4.0, 5.0, 6.0])
        );
    }

    #[test]
    fn vector_float_reads_number_array_and_preserves_smallest_normal() {
        let field = vector_field(DataType::Float32, 4);
        let array = float_vector_array(&[Some(vec![1.5, -3.5, 0.0, f32::MIN_POSITIVE])], 4);
        let reader = reader(&field, &array);
        assert_eq!(
            reader.read(0).unwrap(),
            JsCell::NumberArray(vec![1.5, -3.5, 0.0, 1.1754944e-38])
        );
    }

    #[test]
    fn vector_float_reads_inexact_decimals_as_their_js_literals() {
        let field = vector_field(DataType::Float32, 5);
        let array = float_vector_array(&[Some(vec![1.8, -3.4, 6.7, 0.0, 2.3])], 5);
        let reader = reader(&field, &array);
        assert_eq!(
            reader.read(0).unwrap(),
            JsCell::NumberArray(vec![1.8, -3.4, 6.7, 0.0, 2.3])
        );
    }

    #[test]
    fn vector_float_preserves_negative_zero_sign() {
        let field = vector_field(DataType::Float32, 1);
        let array = float_vector_array(&[Some(vec![-0.0])], 1);
        match reader(&field, &array).read(0).unwrap() {
            JsCell::NumberArray(values) => assert!(values[0].is_sign_negative()),
            other => panic!("expected NumberArray, got {other:?}"),
        }
    }

    #[test]
    fn vector_float_passes_non_finite_as_native_numbers() {
        let field = vector_field(DataType::Float32, 3);
        let array =
            float_vector_array(&[Some(vec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY])], 3);
        match reader(&field, &array).read(0).unwrap() {
            JsCell::NumberArray(values) => {
                assert!(values[0].is_nan());
                assert_eq!(values[1], f64::INFINITY);
                assert_eq!(values[2], f64::NEG_INFINITY);
            }
            other => panic!("expected NumberArray, got {other:?}"),
        }
    }

    #[test]
    fn vector_rejects_unsupported_child_type() {
        let field = vector_field(DataType::Int64, 3);
        let child_field = Arc::new(Field::new("item", DataType::Int64, false));
        let array = FixedSizeListArray::try_new(
            child_field,
            3,
            Arc::new(Int64Array::from(vec![1i64, 2, 3])),
            None,
        )
        .unwrap();
        let err = reader(&field, &array).read(0).unwrap_err().to_string();
        assert!(
            err.contains("VECTOR column 0 row 0") && err.contains("is not Int32 or Float32"),
            "expected unsupported-child-type error, got: {err}"
        );
    }

    fn time_field(scale: &str) -> Field {
        field("TIME", DataType::Int32, &[("scale", scale)])
    }

    #[test]
    fn time_i32_reads_formatted_string_and_null() {
        // 10:30:00.123 at scale 3: secs=37_800, frac=123.
        let field = time_field("3");
        let array = Int32Array::from(vec![Some(37_800_123), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader.decoder, ColumnDecoder::Time(TimeColumn::I32(_), _)),
            "Int32 TIME should route to the Time arm on an I32 TimeColumn"
        );
        assert_eq!(reader.read(0).unwrap(), str_cell("10:30:00"));
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
    }

    #[test]
    fn time_i64_reads_full_precision_string() {
        let field = field("TIME", DataType::Int64, &[("scale", "9")]);
        let raw: i64 = 37_800 * 1_000_000_000 + 123_456_789;
        let array = Int64Array::from(vec![Some(raw)]);
        let reader = reader_with_format(&field, &array, "HH24:MI:SS.FF9");
        assert!(
            matches!(reader.decoder, ColumnDecoder::Time(TimeColumn::I64(_), _)),
            "Int64 TIME should route to the Time arm on an I64 TimeColumn"
        );
        assert_eq!(reader.read(0).unwrap(), str_cell("10:30:00.123456789"));
    }

    #[test]
    fn time_renders_date_elements_as_the_unix_epoch_and_a_zero_offset() {
        let field = time_field("0");
        let array = Int32Array::from(vec![Some(37_800)]);
        let reader = reader_with_format(&field, &array, "YYYY-MM-DD DY HH24:MI:SS TZH:TZM");
        assert_eq!(
            reader.read(0).unwrap(),
            str_cell("1970-01-01 Thu 10:30:00 Z")
        );
    }

    #[test]
    fn time_fraction_width_follows_the_format_not_the_column_scale() {
        // The stored value carries no fractional digits, but FF3 still asks
        // for three, so three zeros are printed — and a bare FF, which takes
        // its width from the scale, prints none and leaves the '.' behind.
        let field = time_field("0");
        let array = Int32Array::from(vec![Some(37_800)]);
        assert_eq!(
            reader_with_format(&field, &array, "HH24:MI:SS.FF3")
                .read(0)
                .unwrap(),
            str_cell("10:30:00.000")
        );
        assert_eq!(
            reader_with_format(&field, &array, "HH24:MI:SS.FF")
                .read(0)
                .unwrap(),
            str_cell("10:30:00.")
        );
    }

    #[test]
    fn time_read_rejects_out_of_range_values_and_names_the_row() {
        let field = time_field("0");
        let array = Int32Array::from(vec![Some(-1), Some(43_200), Some(86_400)]);
        let reader = reader(&field, &array);
        let err = reader.read(0).unwrap_err().to_string();
        assert!(err.contains("row 0") && err.contains("-1"), "got: {err}");
        assert_eq!(reader.read(1).unwrap(), str_cell("12:00:00"));
        let err = reader.read(2).unwrap_err().to_string();
        assert!(err.contains("row 2") && err.contains("86400"), "got: {err}");
    }

    #[test]
    fn for_field_rejects_missing_scale_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("logicalType".to_string(), "TIME".to_string());
        let field = Field::new("T", DataType::Int32, true).with_metadata(metadata);
        let array = Int32Array::from(vec![Some(0)]);
        let err = try_reader(&field, &array).err().unwrap();
        assert!(
            err.contains("missing scale metadata"),
            "error should name the missing scale metadata, got: {err}"
        );
    }

    #[test]
    fn for_field_rejects_unsupported_physical_type_for_time() {
        let field = time_field("0");
        let array = BooleanArray::from(vec![true]);
        let err = try_reader(&field, &array).err().unwrap();
        assert!(
            err.contains("TIME column 0 has unsupported Arrow type"),
            "error should name the unsupported type, got: {err}"
        );
    }

    #[test]
    fn text_reads_string_and_null() {
        let field = field("TEXT", DataType::Utf8, &[]);
        let array = StringArray::from(vec![Some("hello"), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader.decoder, ColumnDecoder::Text(_)),
            "TEXT should route to the Text arm"
        );
        assert_eq!(reader.read(0).unwrap(), str_cell("hello"));
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
    }

    #[test]
    fn variant_reads_json_text_and_null() {
        let field = field("VARIANT", DataType::Utf8, &[]);
        let array = StringArray::from(vec![Some("{\"a\":1}"), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader.decoder, ColumnDecoder::Variant(_)),
            "VARIANT should route to the Variant arm"
        );
        assert_eq!(reader.read(0).unwrap(), str_cell("{\"a\":1}"));
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
    }

    #[test]
    fn object_and_array_logical_types_route_to_variant() {
        let array = StringArray::from(vec![Some("{}")]);
        assert!(
            matches!(
                reader(&field("OBJECT", DataType::Utf8, &[]), &array).decoder,
                ColumnDecoder::Variant(_)
            ),
            "OBJECT should share the Variant arm"
        );
        assert!(
            matches!(
                reader(&field("ARRAY", DataType::Utf8, &[]), &array).decoder,
                ColumnDecoder::Variant(_)
            ),
            "ARRAY should share the Variant arm"
        );
    }

    #[test]
    fn fixed_int_reads_decimal_strings_and_null() {
        let field = field("FIXED", DataType::Int64, &[("scale", "0")]);
        let array = Int64Array::from(vec![Some(42), Some(-1), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader.decoder, ColumnDecoder::Fixed(IntColumn::I64(_), _)),
            "Int64 FIXED should route to the Fixed arm on an I64 IntColumn"
        );
        assert_eq!(reader.read(0).unwrap(), str_cell("42"));
        assert_eq!(reader.read(1).unwrap(), str_cell("-1"));
        assert_eq!(reader.read(2).unwrap(), JsCell::Null);
    }

    #[test]
    fn fixed_int_reads_scaled_and_leading_zero_fraction() {
        let scaled = field("FIXED", DataType::Int64, &[("scale", "2")]);
        let array = Int64Array::from(vec![Some(123)]);
        assert_eq!(reader(&scaled, &array).read(0).unwrap(), str_cell("1.23"));

        let leading_zeros = field("FIXED", DataType::Int64, &[("scale", "5")]);
        let array = Int64Array::from(vec![Some(12)]);
        assert_eq!(
            reader(&leading_zeros, &array).read(0).unwrap(),
            str_cell("0.00012")
        );
    }

    #[test]
    fn fixed_narrow_integers_keep_physical_type_and_read_string() {
        let i8_field = field("FIXED", DataType::Int8, &[("scale", "0")]);
        let i8 = Int8Array::from(vec![Some(42i8), None]);
        let i8_reader = reader(&i8_field, &i8);
        assert!(
            matches!(i8_reader.decoder, ColumnDecoder::Fixed(IntColumn::I8(_), _)),
            "Int8 FIXED should keep an I8 IntColumn, not widen to Int64"
        );
        assert_eq!(i8_reader.read(0).unwrap(), str_cell("42"));
        assert_eq!(i8_reader.read(1).unwrap(), JsCell::Null);

        let i16_field = field("FIXED", DataType::Int16, &[("scale", "2")]);
        let i16 = Int16Array::from(vec![Some(123i16)]);
        let i16_reader = reader(&i16_field, &i16);
        assert!(
            matches!(
                i16_reader.decoder,
                ColumnDecoder::Fixed(IntColumn::I16(_), _)
            ),
            "Int16 FIXED should keep an I16 IntColumn, not widen to Int64"
        );
        assert_eq!(i16_reader.read(0).unwrap(), str_cell("1.23"));

        let i32_field = field("FIXED", DataType::Int32, &[("scale", "0")]);
        let i32 = Int32Array::from(vec![Some(-1)]);
        let i32_reader = reader(&i32_field, &i32);
        assert!(
            matches!(
                i32_reader.decoder,
                ColumnDecoder::Fixed(IntColumn::I32(_), _)
            ),
            "Int32 FIXED should keep an I32 IntColumn, not widen to Int64"
        );
        assert_eq!(i32_reader.read(0).unwrap(), str_cell("-1"));
    }

    #[test]
    fn fixed_decimal128_reads_scaled_string_and_null() {
        let field = field("FIXED", DataType::Decimal128(38, 2), &[("scale", "2")]);
        let array = Decimal128Array::from(vec![Some(12345i128), None])
            .with_precision_and_scale(38, 2)
            .unwrap();
        let reader = reader(&field, &array);
        assert!(
            matches!(
                reader.decoder,
                ColumnDecoder::Fixed(IntColumn::Decimal(_), _)
            ),
            "Decimal128 FIXED should route to the Fixed arm on a Decimal IntColumn"
        );
        assert_eq!(reader.read(0).unwrap(), str_cell("123.45"));
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
    }

    #[test]
    fn real_reads_float_and_null() {
        let field = field("REAL", DataType::Float64, &[]);
        let array = Float64Array::from(vec![Some(1.5), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader.decoder, ColumnDecoder::Real(FloatColumn::F64(_))),
            "Float64 REAL should route to the Real arm on an F64 FloatColumn"
        );
        assert_eq!(reader.read(0).unwrap(), JsCell::Number(1.5));
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
    }

    #[test]
    fn real_float32_keeps_physical_type_and_reads_float() {
        let field = field("REAL", DataType::Float32, &[]);
        let array = Float32Array::from(vec![Some(1.5f32), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader.decoder, ColumnDecoder::Real(FloatColumn::F32(_))),
            "Float32 REAL should keep an F32 FloatColumn, not widen to Float64"
        );
        assert_eq!(reader.read(0).unwrap(), JsCell::Number(1.5));
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
    }

    fn decfloat_struct(
        exponents: Vec<Option<i16>>,
        significands: Vec<Option<&[u8]>>,
    ) -> StructArray {
        let nulls: Vec<bool> = exponents.iter().map(Option::is_some).collect();
        let fields = vec![
            Field::new("exponent", DataType::Int16, true),
            Field::new("significand", DataType::Binary, true),
        ];
        StructArray::try_new(
            fields.into(),
            vec![
                Arc::new(Int16Array::from(exponents)),
                Arc::new(BinaryArray::from(significands)),
            ],
            Some(NullBuffer::from(nulls)),
        )
        .unwrap()
    }

    #[test]
    fn decfloat_reads_formatted_string_and_null() {
        // 123456 * 10^-3 → "123.456" at precision 38 (plain notation).
        let array = decfloat_struct(vec![Some(-3), None], vec![Some(&[0x01, 0xe2, 0x40]), None]);
        let field = field(
            "DECFLOAT",
            array.data_type().clone(),
            &[("precision", "38")],
        );
        let reader = reader(&field, &array);
        assert!(
            matches!(reader.decoder, ColumnDecoder::Decfloat(_, _)),
            "DECFLOAT should route to the Decfloat arm"
        );
        assert_eq!(reader.read(0).unwrap(), str_cell("123.456"));
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
    }

    #[test]
    fn interval_year_month_int64_reads_raw_month_count_and_null() {
        let field = field("INTERVAL_YEAR_MONTH", DataType::Int64, &[]);
        let array = Int64Array::from(vec![Some(27), Some(-27), Some(0), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader.decoder, ColumnDecoder::Interval(IntColumn::I64(_))),
            "INTERVAL_YEAR_MONTH should route to the Interval arm on an I64 IntColumn"
        );
        assert_eq!(reader.read(0).unwrap(), str_cell("27"));
        assert_eq!(reader.read(1).unwrap(), str_cell("-27"));
        assert_eq!(reader.read(2).unwrap(), str_cell("0"));
        assert_eq!(reader.read(3).unwrap(), JsCell::Null);
    }

    #[test]
    fn interval_narrow_integers_keep_physical_type() {
        let i8_field = field("INTERVAL_YEAR_MONTH", DataType::Int8, &[]);
        let i8 = Int8Array::from(vec![Some(14i8)]);
        let i8_reader = reader(&i8_field, &i8);
        assert!(
            matches!(i8_reader.decoder, ColumnDecoder::Interval(IntColumn::I8(_))),
            "Int8 INTERVAL_YEAR_MONTH should keep an I8 IntColumn, not widen to Int64"
        );
        assert_eq!(i8_reader.read(0).unwrap(), str_cell("14"));

        let i16_field = field("INTERVAL_YEAR_MONTH", DataType::Int16, &[]);
        let i16 = Int16Array::from(vec![Some(13i16)]);
        let i16_reader = reader(&i16_field, &i16);
        assert!(
            matches!(
                i16_reader.decoder,
                ColumnDecoder::Interval(IntColumn::I16(_))
            ),
            "Int16 INTERVAL_YEAR_MONTH should keep an I16 IntColumn, not widen to Int64"
        );
        assert_eq!(i16_reader.read(0).unwrap(), str_cell("13"));

        let i32_field = field("INTERVAL_YEAR_MONTH", DataType::Int32, &[]);
        let i32 = Int32Array::from(vec![Some(13)]);
        let i32_reader = reader(&i32_field, &i32);
        assert!(
            matches!(
                i32_reader.decoder,
                ColumnDecoder::Interval(IntColumn::I32(_))
            ),
            "Int32 INTERVAL_YEAR_MONTH should keep an I32 IntColumn, not widen to Int64"
        );
        assert_eq!(i32_reader.read(0).unwrap(), str_cell("13"));

        let dt32_field = field("INTERVAL_DAY_TIME", DataType::Int32, &[]);
        let dt32 = Int32Array::from(vec![Some(2_000_000_000)]);
        let dt32_reader = reader(&dt32_field, &dt32);
        assert!(
            matches!(
                dt32_reader.decoder,
                ColumnDecoder::Interval(IntColumn::I32(_))
            ),
            "Int32 INTERVAL_DAY_TIME should keep an I32 IntColumn, not widen to Int64"
        );
        assert_eq!(dt32_reader.read(0).unwrap(), str_cell("2000000000"));
    }

    #[test]
    fn interval_year_month_decimal128() {
        let field = field("INTERVAL_YEAR_MONTH", DataType::Decimal128(38, 0), &[]);
        let array = Decimal128Array::from(vec![Some(25i128)])
            .with_precision_and_scale(38, 0)
            .unwrap();
        assert_eq!(reader(&field, &array).read(0).unwrap(), str_cell("25"));
    }

    #[test]
    fn interval_day_time_int64_reads_raw_nanos_and_null() {
        let field = field("INTERVAL_DAY_TIME", DataType::Int64, &[]);
        let nanos: i64 = (86_400 + 2 * 3_600 + 3 * 60 + 4) * 1_000_000_000 + 500_000_000;
        let array = Int64Array::from(vec![Some(nanos), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader.decoder, ColumnDecoder::Interval(IntColumn::I64(_))),
            "INTERVAL_DAY_TIME should route to the Interval arm on an I64 IntColumn"
        );
        assert_eq!(reader.read(0).unwrap(), str_cell("93784500000000"));
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
    }

    #[test]
    fn interval_day_time_decimal128_beyond_i64() {
        let field = field("INTERVAL_DAY_TIME", DataType::Decimal128(38, 0), &[]);
        let beyond = i64::MAX as i128 + 1;
        let array = Decimal128Array::from(vec![Some(beyond)])
            .with_precision_and_scale(38, 0)
            .unwrap();
        assert_eq!(
            reader(&field, &array).read(0).unwrap(),
            str_cell("9223372036854775808")
        );
    }

    fn tz_2col(scaled_epoch: i64, offset_minutes: i32) -> StructArray {
        use arrow::array::ArrayRef;
        let epoch_col: ArrayRef = Arc::new(Int64Array::from(vec![Some(scaled_epoch)]));
        let tz_col: ArrayRef = Arc::new(Int32Array::from(vec![Some(
            offset_minutes + sf_types::TZ_OFFSET_BIAS_MINUTES,
        )]));
        StructArray::from(vec![
            (
                Arc::new(Field::new("epoch", DataType::Int64, false)),
                epoch_col,
            ),
            (
                Arc::new(Field::new("tz_offset", DataType::Int32, false)),
                tz_col,
            ),
        ])
    }

    #[test]
    fn timestamp_tz_reads_parts_and_null() {
        use arrow::array::ArrayRef;

        let valid = tz_2col(1_453_386_764, -480);
        let epoch_col: ArrayRef = Arc::new(Int64Array::from(vec![Some(1_453_386_764), None]));
        let tz_col: ArrayRef = Arc::new(Int32Array::from(vec![
            Some(-480 + sf_types::TZ_OFFSET_BIAS_MINUTES),
            None,
        ]));
        let array = StructArray::new(
            valid.fields().clone(),
            vec![epoch_col, tz_col],
            Some(vec![true, false].into()),
        );
        let field = field("TIMESTAMP_TZ", array.data_type().clone(), &[("scale", "0")]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader.decoder, ColumnDecoder::TimestampTz(_, _)),
            "TIMESTAMP_TZ should route to the TimestampTz arm"
        );
        assert_eq!(
            reader.read(0).unwrap(),
            JsCell::SnowflakeDate {
                epoch_millis: 1_453_386_764_000.0,
                nanos: 0,
                scale: 0,
                timezone: ResolvedTimezone::Offset(-480),
                format: "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM",
            }
        );
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
    }

    #[test]
    fn timestamp_ltz_reads_int64_parts_and_null() {
        let field = field("TIMESTAMP_LTZ", DataType::Int64, &[("scale", "3")]);
        let array = Int64Array::from(vec![Some(-1_500), None]);
        let reader = reader(&field, &array);
        assert!(matches!(
            reader.decoder,
            ColumnDecoder::Timestamp(TimestampColumn::I64(_), _)
        ));
        assert_eq!(
            reader.read(0).unwrap(),
            JsCell::SnowflakeDate {
                epoch_millis: -1_500.0,
                nanos: 500_000_000,
                scale: 3,
                timezone: ResolvedTimezone::Named(chrono_tz::Tz::America__Los_Angeles),
                format: "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM",
            }
        );
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
    }

    #[test]
    fn timestamp_ntz_reads_int64_parts_and_null() {
        let field = field("TIMESTAMP_NTZ", DataType::Int64, &[("scale", "3")]);
        let array = Int64Array::from(vec![Some(-1_500), None]);
        let reader = reader(&field, &array);
        assert!(matches!(
            reader.decoder,
            ColumnDecoder::Timestamp(TimestampColumn::I64(_), _)
        ));
        assert_eq!(
            reader.read(0).unwrap(),
            JsCell::SnowflakeDate {
                epoch_millis: -1_500.0,
                nanos: 500_000_000,
                scale: 3,
                timezone: ResolvedTimezone::Utc,
                format: "YYYY-MM-DD HH24:MI:SS.FF3",
            }
        );
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
    }

    #[test]
    fn date_as_string_formats_with_date_output_format() {
        let date = NaiveDate::from_ymd_opt(2016, 1, 21).unwrap();
        let field = field("DATE", DataType::Date32, &[]);
        let array = Date32Array::from(vec![Some(Date32Type::from_naive_date(date)), None]);
        let reader = string_reader(&field, &array);
        assert_eq!(reader.read(0).unwrap(), str_cell("2016-01-21"));
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
    }

    #[test]
    fn date_as_string_represents_null_as_string_null_when_requested() {
        let field = field("DATE", DataType::Date32, &[]);
        let array = Date32Array::from(vec![None]);
        let options = RowOptions {
            represent_null_as_string_null: true,
            ..date_as_string()
        };
        let reader = try_reader_with_options(&field, &array, &options).unwrap();
        assert_eq!(reader.read(0).unwrap(), str_cell("NULL"));
    }

    #[test]
    fn time_ignores_date_fetch_as_string() {
        let field = time_field("0");
        let array = Int32Array::from(vec![None]);
        let options = RowOptions {
            represent_null_as_string_null: true,
            ..date_as_string()
        };
        let reader = try_reader_with_options(&field, &array, &options).unwrap();
        assert_eq!(reader.read(0).unwrap(), JsCell::Null);
    }

    #[test]
    fn timestamp_tz_as_string_formats_in_the_cell_offset() {
        let array = tz_2col(1_453_386_764, -480);
        let field = field("TIMESTAMP_TZ", array.data_type().clone(), &[("scale", "0")]);
        assert_eq!(
            string_reader(&field, &array).read(0).unwrap(),
            str_cell("2016-01-21 06:32:44.000 -0800")
        );
    }

    #[test]
    fn timestamp_ltz_as_string_formats_in_the_session_timezone() {
        let field = field("TIMESTAMP_LTZ", DataType::Int64, &[("scale", "3")]);
        let array = Int64Array::from(vec![Some(-1_500), None]);
        let reader = string_reader(&field, &array);
        assert_eq!(
            reader.read(0).unwrap(),
            str_cell("1969-12-31 15:59:58.500 -0800")
        );
        assert_eq!(reader.read(1).unwrap(), JsCell::Null);
    }

    #[test]
    fn timestamp_ntz_as_string_formats_in_utc() {
        let field = field("TIMESTAMP_NTZ", DataType::Int64, &[("scale", "3")]);
        let array = Int64Array::from(vec![Some(-1_500)]);
        assert_eq!(
            string_reader(&field, &array).read(0).unwrap(),
            str_cell("1969-12-31 23:59:58.500")
        );
    }

    #[test]
    fn timestamp_ltz_rejects_unknown_session_timezone() {
        let field = field("TIMESTAMP_LTZ", DataType::Int64, &[("scale", "3")]);
        let array = Int64Array::from(vec![Some(0)]);
        let params = KnownSessionParameters {
            timezone: "Not/AZone".to_string(),
            ..KnownSessionParameters::defaults()
        };
        let err = ColumnReader::for_field(0, &field, &array, &params, &RowOptions::default())
            .err()
            .unwrap()
            .to_string();
        assert_eq!(err, "unknown timezone: Not/AZone");
    }

    #[test]
    fn timestamp_tz_rejects_flat_int64_physical_type() {
        let field = field("TIMESTAMP_TZ", DataType::Int64, &[("scale", "9")]);
        let array = Int64Array::from(vec![Some(0i64)]);
        let err = try_reader(&field, &array).err().unwrap();
        assert!(
            err.contains("TIMESTAMP_TZ column 0 has unsupported Arrow type"),
            "got: {err}"
        );
    }
}
