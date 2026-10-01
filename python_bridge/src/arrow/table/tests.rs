use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, BooleanArray, Decimal128Array, DurationNanosecondArray, Float64Array,
    Int8Array, Int16Array, Int32Array, Int64Array, StringArray, StructArray,
    Time32MillisecondArray, Time32SecondArray, Time64MicrosecondArray, TimestampMicrosecondArray,
    TimestampMillisecondArray, TimestampNanosecondArray, TimestampSecondArray,
};
use arrow::buffer::NullBuffer;
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;

use super::{TableConverter, field_needs_conversion};
use crate::arrow::error::PlanError;

fn metadata(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

fn field(name: &str, data_type: DataType, logical: &str) -> Field {
    Field::new(name, data_type, true).with_metadata(metadata(&[("logicalType", logical)]))
}

fn converter() -> TableConverter {
    TableConverter::new(false, false, None)
}

fn boolean_batch(schema_metadata: HashMap<String, String>) -> RecordBatch {
    RecordBatch::try_new(
        Arc::new(Schema::new_with_metadata(
            vec![field("b", DataType::Boolean, "BOOLEAN")],
            schema_metadata,
        )),
        vec![Arc::new(BooleanArray::from(vec![Some(true)]))],
    )
    .unwrap()
}

#[test]
fn convert_batch_passthrough_returns_batch_unchanged() {
    let batch = boolean_batch(metadata(&[("queryId", "abc")]));
    let converted = converter().convert_batch(batch.clone()).unwrap();
    assert_eq!(converted, batch);
}

#[test]
fn convert_batch_decfloat_is_unsupported_snowflake_type() {
    let batch = RecordBatch::try_new(
        Arc::new(Schema::new(vec![field("d", DataType::Utf8, "DECFLOAT")])),
        vec![Arc::new(StringArray::from(vec![Some("1E0")]))],
    )
    .unwrap();
    let err = converter().convert_batch(batch).unwrap_err();
    assert!(matches!(
        err,
        PlanError::UnsupportedSnowflakeType {
            ref logical_type,
            ..
        } if logical_type == "DECFLOAT"
    ));
    assert!(
        err.to_string()
            .contains("[Snowflake Exception] unsupported Snowflake type: DECFLOAT")
    );
}

#[test]
fn convert_schema_passthrough_untouched_logical_types() {
    for (logical, data_type) in [
        ("ARRAY", DataType::Utf8),
        ("MAP", DataType::Utf8),
        ("OBJECT", DataType::Utf8),
        ("VARIANT", DataType::Utf8),
        ("INTERVAL_YEAR_MONTH", DataType::Int32),
    ] {
        let schema = Schema::new_with_metadata(
            vec![field("v", data_type, logical)],
            metadata(&[("queryId", "q")]),
        );
        assert!(!field_needs_conversion(schema.field(0), schema.field(0).data_type()).unwrap());
        let converted = converter().convert_schema(schema.clone()).unwrap();
        assert_eq!(converted, schema);
    }
}

#[test]
fn convert_schema_rejects_nested_arrow_semi_structured_types() {
    let map_entries = Field::new(
        "entries",
        DataType::Struct(
            vec![
                Field::new("key", DataType::Utf8, false),
                Field::new("value", DataType::Int64, true),
            ]
            .into(),
        ),
        false,
    );
    for (logical, data_type) in [
        (
            "ARRAY",
            DataType::List(Arc::new(Field::new("item", DataType::Int64, true))),
        ),
        ("MAP", DataType::Map(Arc::new(map_entries), false)),
        (
            "OBJECT",
            DataType::Struct(vec![Field::new("n", DataType::Int64, true)].into()),
        ),
    ] {
        let schema = Schema::new(vec![field("v", data_type, logical)]);
        let err = converter().convert_schema(schema).unwrap_err();
        assert!(
            matches!(
                err,
                PlanError::NestedArrowType {
                    ref logical_type,
                    ..
                } if logical_type == logical
            ),
            "{logical}: {err:?}"
        );
        assert!(err.to_string().contains("nested Arrow type") && err.to_string().contains(logical));
    }
}

fn time_field(name: &str, data_type: DataType, scale: &str) -> Field {
    Field::new(name, data_type, true)
        .with_metadata(metadata(&[("logicalType", "TIME"), ("scale", scale)]))
}

fn time_batch(data_type: DataType, scale: &str, column: impl Array + 'static) -> RecordBatch {
    RecordBatch::try_new(
        Arc::new(Schema::new_with_metadata(
            vec![time_field("t", data_type, scale)],
            metadata(&[("queryId", "q")]),
        )),
        vec![Arc::new(column)],
    )
    .unwrap()
}

fn assert_rewritten_time(batch: &RecordBatch, data_type: DataType) {
    assert_eq!(batch.schema().metadata(), &metadata(&[("queryId", "q")]));
    assert_eq!(batch.schema().field(0).name(), "t");
    assert!(batch.schema().field(0).is_nullable());
    assert_eq!(batch.schema().field(0).data_type(), &data_type);
    assert!(batch.schema().field(0).metadata().is_empty());
}

#[test]
fn convert_batch_time_scale_0_is_time32_seconds() {
    let converted = converter()
        .convert_batch(time_batch(
            DataType::Int64,
            "0",
            Int64Array::from(vec![Some(45_296), None, Some(0)]),
        ))
        .unwrap();
    assert_rewritten_time(&converted, DataType::Time32(TimeUnit::Second));
    let values = converted
        .column(0)
        .as_any()
        .downcast_ref::<Time32SecondArray>()
        .unwrap();
    assert_eq!(
        values,
        &Time32SecondArray::from(vec![Some(45_296), None, Some(0)])
    );
}

#[test]
fn convert_batch_time_scale_2_multiplies_to_milliseconds() {
    let converted = converter()
        .convert_batch(time_batch(
            DataType::Int64,
            "2",
            Int64Array::from(vec![Some(4_529_612)]),
        ))
        .unwrap();
    let values = converted
        .column(0)
        .as_any()
        .downcast_ref::<Time32MillisecondArray>()
        .unwrap();
    assert_eq!(
        values,
        &Time32MillisecondArray::from(vec![Some(45_296_120)])
    );
}

#[test]
fn convert_batch_time_scale_3_is_time32_milliseconds() {
    let converted = converter()
        .convert_batch(time_batch(
            DataType::Int32,
            "3",
            Int32Array::from(vec![Some(45_296_123)]),
        ))
        .unwrap();
    assert_rewritten_time(&converted, DataType::Time32(TimeUnit::Millisecond));
    let values = converted
        .column(0)
        .as_any()
        .downcast_ref::<Time32MillisecondArray>()
        .unwrap();
    assert_eq!(
        values,
        &Time32MillisecondArray::from(vec![Some(45_296_123)])
    );
}

#[test]
fn convert_batch_time_scale_6_is_time64_microseconds() {
    let converted = converter()
        .convert_batch(time_batch(
            DataType::Int64,
            "6",
            Int64Array::from(vec![Some(45_296_123_456)]),
        ))
        .unwrap();
    assert_rewritten_time(&converted, DataType::Time64(TimeUnit::Microsecond));
    let values = converted
        .column(0)
        .as_any()
        .downcast_ref::<Time64MicrosecondArray>()
        .unwrap();
    assert_eq!(
        values,
        &Time64MicrosecondArray::from(vec![Some(45_296_123_456)])
    );
}

#[test]
fn convert_batch_time_scale_7_divides_to_microseconds() {
    let converted = converter()
        .convert_batch(time_batch(
            DataType::Int64,
            "7",
            Int64Array::from(vec![Some(452_961_234_567)]),
        ))
        .unwrap();
    let values = converted
        .column(0)
        .as_any()
        .downcast_ref::<Time64MicrosecondArray>()
        .unwrap();
    assert_eq!(
        values,
        &Time64MicrosecondArray::from(vec![Some(45_296_123_456)])
    );
}

#[test]
fn convert_batch_time_rejects_scale_outside_0_to_9() {
    for scale in ["-1", "10"] {
        let err = converter()
            .convert_batch(time_batch(
                DataType::Int64,
                scale,
                Int64Array::from(vec![Some(0)]),
            ))
            .unwrap_err();
        assert!(
            matches!(err, PlanError::InvalidScale { .. }),
            "scale={scale} err={err}"
        );
        assert!(
            err.to_string().contains("invalid scale value")
                && err.to_string().contains("TIME")
                && err.to_string().contains("expected 0-9"),
            "scale={scale} err={err}"
        );
    }
}

#[test]
fn convert_batch_time_rejects_missing_scale() {
    let schema = Schema::new(vec![field("t", DataType::Int64, "TIME")]);
    let batch = RecordBatch::try_new(
        Arc::new(schema),
        vec![Arc::new(Int64Array::from(vec![Some(1)]))],
    )
    .unwrap();
    let err = converter().convert_batch(batch).unwrap_err();
    assert!(matches!(err, PlanError::MissingMetadata { .. }));
}

#[test]
fn convert_batch_time_rejects_value_that_does_not_fit_time32() {
    let err = converter()
        .convert_batch(time_batch(
            DataType::Int64,
            "0",
            Int64Array::from(vec![Some(i64::from(i32::MAX) + 1)]),
        ))
        .unwrap_err();
    assert!(matches!(err, PlanError::TimeDoesNotFit { .. }));
    assert!(err.to_string().contains("does not fit in time32"));
}

fn interval_field(name: &str, data_type: DataType) -> Field {
    Field::new(name, data_type, true).with_metadata(metadata(&[
        ("logicalType", "INTERVAL_DAY_TIME"),
        ("scale", "9"),
    ]))
}

fn interval_batch(column: impl Array + 'static) -> RecordBatch {
    let column: ArrayRef = Arc::new(column);
    RecordBatch::try_new(
        Arc::new(Schema::new_with_metadata(
            vec![interval_field("iv", column.data_type().clone())],
            metadata(&[("queryId", "q")]),
        )),
        vec![column],
    )
    .unwrap()
}

fn assert_rewritten_interval(batch: &RecordBatch) {
    assert_eq!(batch.schema().metadata(), &metadata(&[("queryId", "q")]));
    assert_eq!(batch.schema().field(0).name(), "iv");
    assert!(batch.schema().field(0).is_nullable());
    assert_eq!(
        batch.schema().field(0).data_type(),
        &DataType::Duration(TimeUnit::Nanosecond)
    );
    assert!(batch.schema().field(0).metadata().is_empty());
}

#[test]
fn convert_batch_interval_day_time_decimal128_to_duration_ns() {
    let converted = converter()
        .convert_batch(interval_batch(Decimal128Array::from(vec![
            Some(93_784_500_000_000i128),
            None,
            Some(0),
        ])))
        .unwrap();
    assert_rewritten_interval(&converted);
    let values = converted
        .column(0)
        .as_any()
        .downcast_ref::<DurationNanosecondArray>()
        .unwrap();
    assert_eq!(
        values,
        &DurationNanosecondArray::from(vec![Some(93_784_500_000_000), None, Some(0)])
    );
}

#[test]
fn convert_batch_interval_day_time_int64_to_duration_ns() {
    let converted = converter()
        .convert_batch(interval_batch(Int64Array::from(vec![
            Some(i64::MAX),
            Some(i64::MIN),
            Some(-1),
        ])))
        .unwrap();
    let values = converted
        .column(0)
        .as_any()
        .downcast_ref::<DurationNanosecondArray>()
        .unwrap();
    assert_eq!(
        values,
        &DurationNanosecondArray::from(vec![Some(i64::MAX), Some(i64::MIN), Some(-1)])
    );
}

#[test]
fn convert_batch_interval_day_time_rejects_decimal128_outside_i64() {
    let beyond = i64::MAX as i128 + 1;
    let err = converter()
        .convert_batch(interval_batch(Decimal128Array::from(vec![Some(beyond)])))
        .unwrap_err();
    assert!(matches!(
        err,
        PlanError::IntervalOverflow { value, .. } if value == beyond
    ));

    let below = i64::MIN as i128 - 1;
    let err = converter()
        .convert_batch(interval_batch(Decimal128Array::from(vec![Some(below)])))
        .unwrap_err();
    assert!(matches!(
        err,
        PlanError::IntervalOverflow { value, .. } if value == below
    ));
}

const EPOCH_2024: i64 = 1_705_314_600;
const EPOCH_YEAR_9999: i64 = 253_402_300_799;
const I64_MAX_DIV_1E9: i64 = i64::MAX / 1_000_000_000;

fn timestamp_field(logical: &str, data_type: DataType, scale: &str) -> Field {
    Field::new("ts", data_type, true)
        .with_metadata(metadata(&[("logicalType", logical), ("scale", scale)]))
}

fn timestamp_int_batch(logical: &str, scale: &str, values: Vec<Option<i64>>) -> RecordBatch {
    let data_type = DataType::Int64;
    RecordBatch::try_new(
        Arc::new(Schema::new_with_metadata(
            vec![timestamp_field(logical, data_type, scale)],
            metadata(&[("queryId", "q")]),
        )),
        vec![Arc::new(Int64Array::from(values))],
    )
    .unwrap()
}

fn ntz_struct(epochs: Vec<Option<i64>>, fractions: Vec<Option<i32>>) -> StructArray {
    let nulls: Vec<bool> = epochs.iter().map(Option::is_some).collect();
    StructArray::try_new(
        vec![
            Field::new("epoch", DataType::Int64, true),
            Field::new("fraction", DataType::Int32, true),
        ]
        .into(),
        vec![
            Arc::new(Int64Array::from(epochs)),
            Arc::new(Int32Array::from(fractions)),
        ],
        Some(NullBuffer::from(nulls)),
    )
    .unwrap()
}

fn timestamp_struct_batch(
    logical: &str,
    scale: &str,
    extra: &[(&str, &str)],
    array: StructArray,
) -> RecordBatch {
    let mut pairs = vec![("logicalType", logical), ("scale", scale)];
    pairs.extend(extra);
    let field = Field::new("ts", array.data_type().clone(), true).with_metadata(metadata(&pairs));
    RecordBatch::try_new(
        Arc::new(Schema::new_with_metadata(
            vec![field],
            metadata(&[("queryId", "q")]),
        )),
        vec![Arc::new(array)],
    )
    .unwrap()
}

#[test]
fn convert_batch_timestamp_ntz_int64_scale_table() {
    let seconds = converter()
        .convert_batch(timestamp_int_batch(
            "TIMESTAMP_NTZ",
            "0",
            vec![Some(EPOCH_2024), None],
        ))
        .unwrap();
    assert_eq!(
        seconds.schema().field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Second, None)
    );
    assert_eq!(
        seconds
            .column(0)
            .as_any()
            .downcast_ref::<TimestampSecondArray>()
            .unwrap(),
        &TimestampSecondArray::from(vec![Some(EPOCH_2024), None])
    );

    let nanos = converter()
        .convert_batch(timestamp_int_batch(
            "TIMESTAMP_NTZ",
            "9",
            vec![Some(EPOCH_2024 * 1_000_000_000 + 123_456_789)],
        ))
        .unwrap();
    assert_eq!(
        nanos.schema().field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Nanosecond, None)
    );
    assert_eq!(
        nanos
            .column(0)
            .as_any()
            .downcast_ref::<TimestampNanosecondArray>()
            .unwrap(),
        &TimestampNanosecondArray::from(vec![Some(EPOCH_2024 * 1_000_000_000 + 123_456_789)])
    );
}

#[test]
fn convert_batch_timestamp_ntz_int64_force_microsecond_divides() {
    let raw = EPOCH_2024 * 10_000_000 + 1_234_567;
    let converted = TableConverter::new(false, true, None)
        .convert_batch(timestamp_int_batch("TIMESTAMP_NTZ", "7", vec![Some(raw)]))
        .unwrap();
    assert_eq!(
        converted.schema().field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Microsecond, None)
    );
    assert_eq!(
        converted
            .column(0)
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
            .unwrap(),
        &TimestampMicrosecondArray::from(vec![Some(raw / 10)])
    );
}

#[test]
fn convert_batch_timestamp_ntz_struct_in_range_is_nanoseconds() {
    let converted = converter()
        .convert_batch(timestamp_struct_batch(
            "TIMESTAMP_NTZ",
            "9",
            &[],
            ntz_struct(vec![Some(EPOCH_2024), None], vec![Some(123_456_789), None]),
        ))
        .unwrap();
    assert_eq!(
        converted.schema().field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Nanosecond, None)
    );
    assert!(converted.schema().field(0).metadata().is_empty());
    assert_eq!(
        converted
            .column(0)
            .as_any()
            .downcast_ref::<TimestampNanosecondArray>()
            .unwrap(),
        &TimestampNanosecondArray::from(vec![Some(EPOCH_2024 * 1_000_000_000 + 123_456_789), None])
    );
}

#[test]
fn convert_batch_timestamp_ntz_struct_overflow_aligned_downscales() {
    let converted = converter()
        .convert_batch(timestamp_struct_batch(
            "TIMESTAMP_NTZ",
            "9",
            &[],
            ntz_struct(vec![Some(EPOCH_YEAR_9999)], vec![Some(123_456_000)]),
        ))
        .unwrap();
    assert_eq!(
        converted.schema().field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Microsecond, None)
    );
    assert_eq!(
        converted
            .column(0)
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
            .unwrap(),
        &TimestampMicrosecondArray::from(vec![Some(EPOCH_YEAR_9999 * 1_000_000 + 123_456)])
    );
}

#[test]
fn convert_batch_timestamp_ntz_struct_overflow_leftover_nanos_errors() {
    let err = converter()
        .convert_batch(timestamp_struct_batch(
            "TIMESTAMP_NTZ",
            "9",
            &[],
            ntz_struct(vec![Some(EPOCH_YEAR_9999)], vec![Some(123_456_789)]),
        ))
        .unwrap_err();
    assert!(matches!(err, PlanError::TimestampOverflow { .. }));
    let text = err.to_string();
    assert!(text.contains("253402300799123456789"));
    assert!(text.contains("force_microsecond_precision=True"));
}

#[test]
fn convert_batch_timestamp_ntz_struct_product_overflows_when_epoch_is_in_scan_range() {
    let err = converter()
        .convert_batch(timestamp_struct_batch(
            "TIMESTAMP_NTZ",
            "9",
            &[],
            ntz_struct(vec![Some(I64_MAX_DIV_1E9)], vec![Some(854_775_808)]),
        ))
        .unwrap_err();
    assert!(matches!(err, PlanError::TimestampOverflow { .. }));
}

#[test]
fn convert_batch_timestamp_ntz_struct_force_microsecond_truncates() {
    let converted = TableConverter::new(false, true, None)
        .convert_batch(timestamp_struct_batch(
            "TIMESTAMP_NTZ",
            "9",
            &[],
            ntz_struct(vec![Some(EPOCH_YEAR_9999)], vec![Some(123_456_789)]),
        ))
        .unwrap();
    assert_eq!(
        converted.schema().field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Microsecond, None)
    );
    assert_eq!(
        converted
            .column(0)
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
            .unwrap(),
        &TimestampMicrosecondArray::from(vec![Some(EPOCH_YEAR_9999 * 1_000_000 + 123_456)])
    );
}

#[test]
fn convert_batch_timestamp_ltz_writes_session_timezone() {
    let converted = TableConverter::new(false, false, Some("America/New_York".into()))
        .convert_batch(timestamp_int_batch(
            "TIMESTAMP_LTZ",
            "0",
            vec![Some(EPOCH_2024)],
        ))
        .unwrap();
    assert_eq!(
        converted.schema().field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Second, Some("America/New_York".into()))
    );
}

#[test]
fn convert_batch_timestamp_ltz_struct_writes_session_timezone() {
    let converted = TableConverter::new(false, false, Some("America/New_York".into()))
        .convert_batch(timestamp_struct_batch(
            "TIMESTAMP_LTZ",
            "9",
            &[],
            ntz_struct(vec![Some(EPOCH_2024)], vec![Some(123_456_789)]),
        ))
        .unwrap();
    assert_eq!(
        converted.schema().field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Nanosecond, Some("America/New_York".into()))
    );
    assert_eq!(
        converted
            .column(0)
            .as_any()
            .downcast_ref::<TimestampNanosecondArray>()
            .unwrap(),
        &TimestampNanosecondArray::from(vec![Some(EPOCH_2024 * 1_000_000_000 + 123_456_789)])
            .with_timezone("America/New_York".to_string())
    );
}

#[test]
fn convert_batch_timestamp_ntz_int64_scale_3_is_milliseconds() {
    let converted = converter()
        .convert_batch(timestamp_int_batch(
            "TIMESTAMP_NTZ",
            "3",
            vec![Some(EPOCH_2024 * 1_000 + 123)],
        ))
        .unwrap();
    assert_eq!(
        converted.schema().field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Millisecond, None)
    );
    assert_eq!(
        converted
            .column(0)
            .as_any()
            .downcast_ref::<TimestampMillisecondArray>()
            .unwrap(),
        &TimestampMillisecondArray::from(vec![Some(EPOCH_2024 * 1_000 + 123)])
    );
}

#[test]
fn convert_batch_timestamp_ntz_int64_scale_6_is_microseconds() {
    let converted = converter()
        .convert_batch(timestamp_int_batch(
            "TIMESTAMP_NTZ",
            "6",
            vec![Some(EPOCH_2024 * 1_000_000 + 123_456)],
        ))
        .unwrap();
    assert_eq!(
        converted.schema().field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Microsecond, None)
    );
    assert_eq!(
        converted
            .column(0)
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
            .unwrap(),
        &TimestampMicrosecondArray::from(vec![Some(EPOCH_2024 * 1_000_000 + 123_456)])
    );
}

#[test]
fn convert_batch_timestamp_ntz_rejects_non_integer_or_struct() {
    let batch = RecordBatch::try_new(
        Arc::new(Schema::new(vec![timestamp_field(
            "TIMESTAMP_NTZ",
            DataType::Utf8,
            "0",
        )])),
        vec![Arc::new(StringArray::from(vec![Some("x")]))],
    )
    .unwrap();
    let err = converter().convert_batch(batch).unwrap_err();
    assert!(matches!(err, PlanError::ColumnConvert { .. }));
}

#[test]
fn convert_batch_timestamp_tz_rejects_integer_physical_type() {
    let err = converter()
        .convert_batch(timestamp_int_batch("TIMESTAMP_TZ", "0", vec![Some(0)]))
        .unwrap_err();
    assert!(matches!(err, PlanError::ColumnConvert { .. }));
}

#[test]
fn convert_batch_timestamp_tz_omitted_byte_length_is_16() {
    let converted = TableConverter::new(false, false, Some("UTC".into()))
        .convert_batch(timestamp_struct_batch(
            "TIMESTAMP_TZ",
            "9",
            &[],
            ntz_struct(vec![Some(EPOCH_2024)], vec![Some(123_456_789)]),
        ))
        .unwrap();
    assert_eq!(
        converted.schema().field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Nanosecond, Some("UTC".into()))
    );
}

#[test]
fn convert_batch_timestamp_tz_byte_length_16_matches_struct() {
    let array = StructArray::try_new(
        vec![
            Field::new("epoch", DataType::Int64, true),
            Field::new("fraction", DataType::Int32, true),
            Field::new("timezone", DataType::Int32, true),
        ]
        .into(),
        vec![
            Arc::new(Int64Array::from(vec![Some(EPOCH_2024)])),
            Arc::new(Int32Array::from(vec![Some(123_456_789)])),
            Arc::new(Int32Array::from(vec![Some(1440)])),
        ],
        None,
    )
    .unwrap();
    let converted = TableConverter::new(false, false, Some("UTC".into()))
        .convert_batch(timestamp_struct_batch(
            "TIMESTAMP_TZ",
            "9",
            &[("byteLength", "16")],
            array,
        ))
        .unwrap();
    assert_eq!(
        converted.schema().field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Nanosecond, Some("UTC".into()))
    );
    assert_eq!(
        converted
            .column(0)
            .as_any()
            .downcast_ref::<TimestampNanosecondArray>()
            .unwrap(),
        &TimestampNanosecondArray::from(vec![Some(EPOCH_2024 * 1_000_000_000 + 123_456_789)])
            .with_timezone("UTC".to_string())
    );
}

#[test]
fn convert_batch_timestamp_tz_byte_length_8_uses_epoch_only() {
    let array = StructArray::try_new(
        vec![
            Field::new("epoch", DataType::Int64, true),
            Field::new("timezone", DataType::Int32, true),
        ]
        .into(),
        vec![
            Arc::new(Int64Array::from(vec![Some(EPOCH_2024)])),
            Arc::new(Int32Array::from(vec![Some(1440)])),
        ],
        None,
    )
    .unwrap();
    let converted = converter()
        .convert_batch(timestamp_struct_batch(
            "TIMESTAMP_TZ",
            "0",
            &[("byteLength", "8")],
            array,
        ))
        .unwrap();
    assert_eq!(
        converted.schema().field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Second, None)
    );
}

fn tz8_struct(epochs: Vec<Option<i64>>) -> StructArray {
    StructArray::try_new(
        vec![
            Field::new("epoch", DataType::Int64, true),
            Field::new("timezone", DataType::Int32, true),
        ]
        .into(),
        vec![
            Arc::new(Int64Array::from(epochs)),
            Arc::new(Int32Array::from(vec![Some(1440)])),
        ],
        None,
    )
    .unwrap()
}

#[test]
fn convert_batch_timestamp_tz_byte_length_8_scale_7_scales_like_int64() {
    let raw = EPOCH_2024 * 10_000_000 + 1_234_567;
    let converted = converter()
        .convert_batch(timestamp_struct_batch(
            "TIMESTAMP_TZ",
            "7",
            &[("byteLength", "8")],
            tz8_struct(vec![Some(raw)]),
        ))
        .unwrap();
    assert_eq!(
        converted.schema().field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Nanosecond, None)
    );
    assert_eq!(
        converted
            .column(0)
            .as_any()
            .downcast_ref::<TimestampNanosecondArray>()
            .unwrap(),
        &TimestampNanosecondArray::from(vec![Some(raw * 100)])
    );

    let micros = TableConverter::new(false, true, None)
        .convert_batch(timestamp_struct_batch(
            "TIMESTAMP_TZ",
            "7",
            &[("byteLength", "8")],
            tz8_struct(vec![Some(raw)]),
        ))
        .unwrap();
    assert_eq!(
        micros.schema().field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Microsecond, None)
    );
    assert_eq!(
        micros
            .column(0)
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
            .unwrap(),
        &TimestampMicrosecondArray::from(vec![Some(raw / 10)])
    );
}

#[test]
fn convert_batch_timestamp_tz_rejects_unknown_byte_length() {
    let array = ntz_struct(vec![Some(EPOCH_2024)], vec![Some(0)]);
    let err = converter()
        .convert_batch(timestamp_struct_batch(
            "TIMESTAMP_TZ",
            "9",
            &[("byteLength", "4")],
            array,
        ))
        .unwrap_err();
    assert!(matches!(err, PlanError::UnknownByteLength { .. }));
}

#[test]
fn convert_batch_timestamp_rejects_scale_outside_0_to_9() {
    for logical in ["TIMESTAMP_NTZ", "TIMESTAMP_LTZ"] {
        let err = converter()
            .convert_batch(timestamp_int_batch(logical, "10", vec![Some(0)]))
            .unwrap_err();
        assert!(matches!(err, PlanError::InvalidScale { .. }));
        assert!(err.to_string().contains(logical));
    }
    let err = converter()
        .convert_batch(timestamp_struct_batch(
            "TIMESTAMP_TZ",
            "10",
            &[],
            ntz_struct(vec![Some(0)], vec![Some(0)]),
        ))
        .unwrap_err();
    assert!(matches!(err, PlanError::InvalidScale { .. }));
    assert!(err.to_string().contains("TIMESTAMP_TZ"));
}

#[test]
fn convert_schema_timestamp_tz_defaults_to_nanoseconds_with_session_timezone() {
    let schema = Schema::new_with_metadata(
        vec![timestamp_field(
            "TIMESTAMP_TZ",
            DataType::Struct(
                vec![
                    Field::new("epoch", DataType::Int64, true),
                    Field::new("fraction", DataType::Int32, true),
                    Field::new("timezone", DataType::Int32, true),
                ]
                .into(),
            ),
            "9",
        )],
        metadata(&[("queryId", "q")]),
    );
    let converted = TableConverter::new(false, false, Some("UTC".into()))
        .convert_schema(schema)
        .unwrap();
    assert_eq!(converted.metadata(), &metadata(&[("queryId", "q")]));
    assert_eq!(
        converted.field(0).data_type(),
        &DataType::Timestamp(TimeUnit::Nanosecond, Some("UTC".into()))
    );
}

#[test]
fn field_needs_conversion_for_scaled_fixed_but_not_decimal128() {
    let scaled = Field::new("n", DataType::Int64, true)
        .with_metadata(metadata(&[("logicalType", "FIXED"), ("scale", "2")]));
    assert!(field_needs_conversion(&scaled, scaled.data_type()).unwrap());

    let decimal = Field::new("n", DataType::Decimal128(38, 2), true)
        .with_metadata(metadata(&[("logicalType", "FIXED"), ("scale", "2")]));
    assert!(!field_needs_conversion(&decimal, decimal.data_type()).unwrap());

    let unscaled = Field::new("n", DataType::Int64, true)
        .with_metadata(metadata(&[("logicalType", "FIXED"), ("scale", "0")]));
    assert!(!field_needs_conversion(&unscaled, unscaled.data_type()).unwrap());
}

#[test]
fn field_needs_conversion_rejects_invalid_fixed_scale() {
    let field = Field::new("n", DataType::Int64, true)
        .with_metadata(metadata(&[("logicalType", "FIXED"), ("scale", "x")]));
    let err = field_needs_conversion(&field, field.data_type()).unwrap_err();
    assert!(matches!(err, PlanError::InvalidMetadata { .. }));
}

fn fixed_field(name: &str, data_type: DataType, scale: &str) -> Field {
    Field::new(name, data_type, true)
        .with_metadata(metadata(&[("logicalType", "FIXED"), ("scale", scale)]))
}

fn fixed_batch(data_type: DataType, scale: &str, column: impl Array + 'static) -> RecordBatch {
    RecordBatch::try_new(
        Arc::new(Schema::new_with_metadata(
            vec![fixed_field("n", data_type, scale)],
            metadata(&[("queryId", "q")]),
        )),
        vec![Arc::new(column)],
    )
    .unwrap()
}

fn assert_rewritten_fixed(batch: &RecordBatch, data_type: DataType) {
    assert_eq!(batch.schema().metadata(), &metadata(&[("queryId", "q")]));
    assert_eq!(batch.schema().field(0).name(), "n");
    assert!(batch.schema().field(0).is_nullable());
    assert_eq!(batch.schema().field(0).data_type(), &data_type);
    assert!(batch.schema().field(0).metadata().is_empty());
}

#[test]
fn convert_batch_fixed_scale_2_to_float64() {
    let converted = converter()
        .convert_batch(fixed_batch(
            DataType::Int64,
            "2",
            Int64Array::from(vec![Some(123), None, Some(-50), Some(0)]),
        ))
        .unwrap();
    assert_rewritten_fixed(&converted, DataType::Float64);
    let values = converted
        .column(0)
        .as_any()
        .downcast_ref::<Float64Array>()
        .unwrap();
    assert_eq!(
        values,
        &Float64Array::from(vec![Some(1.23), None, Some(-0.5), Some(0.0)])
    );
}

#[test]
fn convert_batch_fixed_scale_8_divides() {
    let converted = converter()
        .convert_batch(fixed_batch(
            DataType::Int64,
            "8",
            Int64Array::from(vec![
                Some(1),
                Some(100_000_000),
                Some(9_007_199_254_740_993),
            ]),
        ))
        .unwrap();
    let values = converted
        .column(0)
        .as_any()
        .downcast_ref::<Float64Array>()
        .unwrap();
    assert_eq!(
        values,
        &Float64Array::from(vec![
            Some(crate::arrow::scaled_f64::scaled_f64(1, -8)),
            Some(crate::arrow::scaled_f64::scaled_f64(100_000_000, -8)),
            Some(crate::arrow::scaled_f64::scaled_f64(
                9_007_199_254_740_993,
                -8
            )),
        ])
    );
}

#[test]
fn convert_batch_fixed_scale_9_uses_scaled_f64() {
    let converted = converter()
        .convert_batch(fixed_batch(
            DataType::Int64,
            "9",
            Int64Array::from(vec![
                Some(1),
                Some(-1),
                Some(1_234_567_890),
                Some(9_007_199_254_740_995),
            ]),
        ))
        .unwrap();
    let values = converted
        .column(0)
        .as_any()
        .downcast_ref::<Float64Array>()
        .unwrap();
    assert_eq!(
        values,
        &Float64Array::from(vec![
            Some(crate::arrow::scaled_f64::scaled_f64(1, -9)),
            Some(crate::arrow::scaled_f64::scaled_f64(-1, -9)),
            Some(crate::arrow::scaled_f64::scaled_f64(1_234_567_890, -9)),
            Some(crate::arrow::scaled_f64::scaled_f64(
                9_007_199_254_740_995,
                -9
            )),
        ])
    );
}

#[test]
fn convert_batch_fixed_integer_widths() {
    let cases: [RecordBatch; 3] = [
        converter()
            .convert_batch(fixed_batch(
                DataType::Int8,
                "2",
                Int8Array::from(vec![Some(100)]),
            ))
            .unwrap(),
        converter()
            .convert_batch(fixed_batch(
                DataType::Int16,
                "2",
                Int16Array::from(vec![Some(1234)]),
            ))
            .unwrap(),
        converter()
            .convert_batch(fixed_batch(
                DataType::Int32,
                "2",
                Int32Array::from(vec![Some(12_345)]),
            ))
            .unwrap(),
    ];
    let expected = [1.0, 12.34, 123.45];
    for (batch, want) in cases.into_iter().zip(expected) {
        let values = batch
            .column(0)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap();
        assert_eq!(values, &Float64Array::from(vec![Some(want)]));
    }
}

#[test]
fn convert_batch_fixed_number_to_decimal() {
    let converted = TableConverter::new(true, false, None)
        .convert_batch(fixed_batch(
            DataType::Int64,
            "2",
            Int64Array::from(vec![Some(123), None, Some(-50)]),
        ))
        .unwrap();
    assert_rewritten_fixed(&converted, DataType::Decimal128(38, 2));
    let values = converted
        .column(0)
        .as_any()
        .downcast_ref::<Decimal128Array>()
        .unwrap();
    assert_eq!(
        values,
        &Decimal128Array::from(vec![Some(123i128), None, Some(-50)])
            .with_precision_and_scale(38, 2)
            .unwrap()
    );
}

#[test]
fn convert_batch_fixed_scale_0_passthrough() {
    let batch = fixed_batch(DataType::Int64, "0", Int64Array::from(vec![Some(123)]));
    let converted = converter().convert_batch(batch.clone()).unwrap();
    assert_eq!(converted.schema().as_ref(), batch.schema().as_ref());
    assert_eq!(converted.column(0).as_ref(), batch.column(0).as_ref());
}

#[test]
fn convert_batch_fixed_rejects_non_integer_physical_type() {
    let err = converter()
        .convert_batch(fixed_batch(
            DataType::Utf8,
            "2",
            StringArray::from(vec![Some("1")]),
        ))
        .unwrap_err();
    assert!(matches!(err, PlanError::ColumnConvert { .. }));
}

#[test]
fn convert_batch_fixed_leaves_boolean_column_and_its_metadata() {
    let schema = Schema::new_with_metadata(
        vec![
            field("b", DataType::Boolean, "BOOLEAN"),
            fixed_field("n", DataType::Int64, "2"),
        ],
        metadata(&[("queryId", "q")]),
    );
    let converted = converter()
        .convert_batch(
            RecordBatch::try_new(
                Arc::new(schema),
                vec![
                    Arc::new(BooleanArray::from(vec![Some(true)])),
                    Arc::new(Int64Array::from(vec![Some(123)])),
                ],
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(converted.schema().field(0).data_type(), &DataType::Boolean);
    assert_eq!(
        converted.schema().field(0).metadata(),
        &metadata(&[("logicalType", "BOOLEAN")])
    );
    assert_eq!(converted.schema().field(1).data_type(), &DataType::Float64);
    assert!(converted.schema().field(1).metadata().is_empty());
}
