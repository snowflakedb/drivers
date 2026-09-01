#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use arrow::array::{ArrayRef, PrimitiveArray, StructArray};
    use arrow::datatypes::{DataType, Field as ArrowField, Int32Type, Int64Type};
    use chrono::NaiveDateTime;
    use odbc_sys as sql;

    use crate::api::CDataType;
    use crate::conversion::error::{ConversionError, ReadArrowError};
    use crate::conversion::test_utils::helpers::{binding_for_char_buffer, binding_for_value};
    use crate::conversion::timestamp::{
        SnowflakeTimestampLtz, SnowflakeTimestampNtz, SnowflakeTimestampTz,
    };
    use crate::conversion::warning::Warning;
    use crate::conversion::{
        NumericSettings, ReadArrowType, SnowflakeType, WriteODBCType, decimal_digits_from_field,
        make_converter, sql_type_from_field,
    };

    fn ntz(scale: u32) -> SnowflakeTimestampNtz {
        SnowflakeTimestampNtz { scale }
    }

    fn ltz(scale: u32) -> SnowflakeTimestampLtz {
        SnowflakeTimestampLtz { scale }
    }

    fn tz(scale: u32) -> SnowflakeTimestampTz {
        SnowflakeTimestampTz {
            scale,
            tz_offset_format: None,
        }
    }

    fn tz_with_format(
        scale: u32,
        tz_offset_format: crate::conversion::timestamp::TzOffsetFormat,
    ) -> SnowflakeTimestampTz {
        SnowflakeTimestampTz {
            scale,
            tz_offset_format: Some(tz_offset_format),
        }
    }

    fn make_struct_array(epoch: i64, fraction: i32) -> StructArray {
        let epoch_col: ArrayRef = Arc::new(PrimitiveArray::<Int64Type>::from(vec![Some(epoch)]));
        let frac_col: ArrayRef = Arc::new(PrimitiveArray::<Int32Type>::from(vec![Some(fraction)]));
        StructArray::from(vec![
            (
                Arc::new(ArrowField::new("epoch", DataType::Int64, false)),
                epoch_col,
            ),
            (
                Arc::new(ArrowField::new("fraction", DataType::Int32, false)),
                frac_col,
            ),
        ])
    }

    fn make_null_struct_array() -> StructArray {
        let epoch_col: ArrayRef = Arc::new(PrimitiveArray::<Int64Type>::from(vec![None::<i64>]));
        let frac_col: ArrayRef = Arc::new(PrimitiveArray::<Int32Type>::from(vec![None::<i32>]));
        let fields = vec![
            Arc::new(ArrowField::new("epoch", DataType::Int64, true)),
            Arc::new(ArrowField::new("fraction", DataType::Int32, true)),
        ];
        StructArray::new(
            fields.into(),
            vec![epoch_col, frac_col],
            Some(vec![false].into()),
        )
    }

    fn timestamp_field_with_metadata(metadata: HashMap<String, String>) -> ArrowField {
        ArrowField::new("ts", DataType::Int64, true).with_metadata(metadata)
    }

    fn settings() -> NumericSettings {
        NumericSettings::default()
    }

    #[test]
    fn read_scaled_scale_0_returns_seconds() {
        let sn = ntz(0);
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(1_700_000_000)]);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert_eq!(
            value,
            NaiveDateTime::parse_from_str("2023-11-14 22:13:20", "%Y-%m-%d %H:%M:%S").unwrap()
        );
    }

    #[test]
    fn read_scaled_scale_3_returns_milliseconds() {
        let sn = ntz(3);
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(1_700_000_000_123)]);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert_eq!(value.and_utc().timestamp_millis(), 1_700_000_000_123);
    }

    #[test]
    fn read_scaled_scale_6_returns_microseconds() {
        let sn = ntz(6);
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(1_700_000_000_123_456)]);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert_eq!(value.and_utc().timestamp_micros(), 1_700_000_000_123_456);
    }

    #[test]
    fn read_scaled_scale_9_returns_nanoseconds() {
        let sn = ntz(9);
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(1_700_000_000_123_456_789)]);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert_eq!(
            value.and_utc().timestamp_nanos_opt(),
            Some(1_700_000_000_123_456_789)
        );
    }

    #[test]
    fn read_scaled_scale_10_returns_invalid() {
        let sn = ntz(10);
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(0)]);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn read_scaled_scale_18_returns_invalid() {
        let sn = ntz(18);
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(0)]);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn read_scaled_scale_u32_max_returns_invalid() {
        let sn = ntz(u32::MAX);
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(0)]);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn read_scaled_ltz_scale_over_9_returns_invalid() {
        let sn = ltz(10);
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(0)]);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn should_reject_timestamp_tz_flat_int64_converter() {
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), "TIMESTAMP_TZ".to_string());
        meta.insert("scale".to_string(), "9".to_string());
        let field = timestamp_field_with_metadata(meta);
        let err = match make_converter(&field, &settings()) {
            Err(e) => e,
            Ok(_) => panic!("expected converter construction to fail for Int64 TIMESTAMP_TZ"),
        };
        assert!(
            matches!(
                err,
                ConversionError::IncompatibleFieldMetadata {
                    ref logical_type,
                    data_type: DataType::Int64,
                    ..
                } if logical_type == "TIMESTAMP_TZ"
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn read_scaled_null_returns_null_error() {
        let sn = ntz(9);
        let array = PrimitiveArray::<Int64Type>::from(vec![None::<i64>]);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(result, Err(ReadArrowError::NullValue { .. })));
    }

    #[test]
    fn read_struct_valid_fraction() {
        let sn = ntz(9);
        let array = make_struct_array(1_700_000_000, 500_000_000);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert_eq!(value.and_utc().timestamp(), 1_700_000_000);
        assert_eq!(value.and_utc().timestamp_subsec_nanos(), 500_000_000);
    }

    #[test]
    fn read_struct_zero_fraction() {
        let sn = ntz(9);
        let array = make_struct_array(1_700_000_000, 0);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert_eq!(value.and_utc().timestamp_subsec_nanos(), 0);
    }

    #[test]
    fn read_struct_max_valid_fraction() {
        let sn = ntz(9);
        let array = make_struct_array(0, 999_999_999);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert_eq!(value.and_utc().timestamp_subsec_nanos(), 999_999_999);
    }

    #[test]
    fn read_struct_negative_fraction_returns_invalid() {
        let sn = ntz(9);
        let array = make_struct_array(1_700_000_000, -1);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn read_struct_large_negative_fraction_returns_invalid() {
        let sn = ntz(9);
        let array = make_struct_array(0, i32::MIN);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn read_struct_fraction_at_boundary_returns_invalid() {
        let sn = ntz(9);
        let array = make_struct_array(0, 1_000_000_000);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn read_struct_fraction_above_boundary_returns_invalid() {
        let sn = ntz(9);
        let array = make_struct_array(0, i32::MAX);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn read_struct_null_returns_null_error() {
        let sn = ntz(9);
        let array = make_null_struct_array();
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(result, Err(ReadArrowError::NullValue { .. })));
    }

    #[test]
    fn read_struct_ltz_negative_fraction_returns_invalid() {
        let sn = ltz(9);
        let array = make_struct_array(0, -100);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn timestamp_scale_valid_returns_scale() {
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), "TIMESTAMP_NTZ".to_string());
        meta.insert("scale".to_string(), "3".to_string());
        let field = timestamp_field_with_metadata(meta);
        let digits = decimal_digits_from_field(&field, &settings()).unwrap();
        assert_eq!(digits, 3);
    }

    #[test]
    fn timestamp_scale_zero_returns_zero() {
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), "TIMESTAMP_NTZ".to_string());
        meta.insert("scale".to_string(), "0".to_string());
        let field = timestamp_field_with_metadata(meta);
        let digits = decimal_digits_from_field(&field, &settings()).unwrap();
        assert_eq!(digits, 0);
    }

    #[test]
    fn timestamp_scale_9_returns_9() {
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), "TIMESTAMP_NTZ".to_string());
        meta.insert("scale".to_string(), "9".to_string());
        let field = timestamp_field_with_metadata(meta);
        let digits = decimal_digits_from_field(&field, &settings()).unwrap();
        assert_eq!(digits, 9);
    }

    #[test]
    fn timestamp_scale_over_9_caps_to_9() {
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), "TIMESTAMP_NTZ".to_string());
        meta.insert("scale".to_string(), "12".to_string());
        let field = timestamp_field_with_metadata(meta);
        let digits = decimal_digits_from_field(&field, &settings()).unwrap();
        assert_eq!(digits, 9);
    }

    #[test]
    fn timestamp_scale_missing_defaults_to_9() {
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), "TIMESTAMP_NTZ".to_string());
        let field = timestamp_field_with_metadata(meta);
        let digits = decimal_digits_from_field(&field, &settings()).unwrap();
        assert_eq!(digits, 9);
    }

    #[test]
    fn timestamp_scale_unparseable_returns_error() {
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), "TIMESTAMP_NTZ".to_string());
        meta.insert("scale".to_string(), "abc".to_string());
        let field = timestamp_field_with_metadata(meta);
        let result = decimal_digits_from_field(&field, &settings());
        assert!(matches!(
            result,
            Err(ConversionError::FieldMetadataParsing { .. })
        ));
    }

    #[test]
    fn timestamp_scale_negative_string_returns_error() {
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), "TIMESTAMP_NTZ".to_string());
        meta.insert("scale".to_string(), "-1".to_string());
        let field = timestamp_field_with_metadata(meta);
        let result = decimal_digits_from_field(&field, &settings());
        assert!(matches!(
            result,
            Err(ConversionError::FieldMetadataParsing { .. })
        ));
    }

    #[test]
    fn timestamp_scale_ltz_missing_defaults_to_9() {
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), "TIMESTAMP_LTZ".to_string());
        let field = timestamp_field_with_metadata(meta);
        let digits = decimal_digits_from_field(&field, &settings()).unwrap();
        assert_eq!(digits, 9);
    }

    #[test]
    fn timestamp_scale_tz_over_9_caps_to_9() {
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), "TIMESTAMP_TZ".to_string());
        meta.insert("scale".to_string(), "15".to_string());
        let field = timestamp_field_with_metadata(meta);
        let digits = decimal_digits_from_field(&field, &settings()).unwrap();
        assert_eq!(digits, 9);
    }

    #[test]
    fn timestamp_scale_ltz_unparseable_returns_error() {
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), "TIMESTAMP_LTZ".to_string());
        meta.insert("scale".to_string(), "not_a_number".to_string());
        let field = timestamp_field_with_metadata(meta);
        let result = sql_type_from_field(&field, &settings());
        assert!(matches!(
            result,
            Err(ConversionError::FieldMetadataParsing { .. })
        ));
    }

    #[test]
    fn write_ntz_timestamp_struct() {
        let sn = ntz(9);
        let mut value = sql::Timestamp {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            fraction: 0,
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::TypeTimestamp, &mut value, &mut str_len);
        let input =
            NaiveDateTime::parse_from_str("2023-06-15 10:30:45", "%Y-%m-%d %H:%M:%S").unwrap();
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.year, 2023);
        assert_eq!(value.month, 6);
        assert_eq!(value.day, 15);
        assert_eq!(value.hour, 10);
        assert_eq!(value.minute, 30);
        assert_eq!(value.second, 45);
    }

    #[test]
    fn write_to_date_truncates_time_component() {
        let sn = ntz(9);
        let mut value = sql::Date {
            year: 0,
            month: 0,
            day: 0,
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::TypeDate, &mut value, &mut str_len);
        let input =
            NaiveDateTime::parse_from_str("2023-06-15 10:30:45", "%Y-%m-%d %H:%M:%S").unwrap();
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::NumericValueTruncated))
        );
        assert_eq!(value.year, 2023);
        assert_eq!(value.month, 6);
        assert_eq!(value.day, 15);
    }

    #[test]
    fn write_char_full_timestamp() {
        let sn = ntz(9);
        let mut buffer = vec![0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let input =
            NaiveDateTime::parse_from_str("2023-06-15 10:30:45", "%Y-%m-%d %H:%M:%S").unwrap();
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 19);
        assert_eq!(&buffer[..19], b"2023-06-15 10:30:45");
    }

    /// Build a 2-column TIMESTAMP_TZ Arrow struct for WRITE/policy tests that
    /// still need a decoded `TzInstant`. Arrow READ coverage lives in `sf_types`.
    fn make_tz_struct_array_2col(scaled_epoch: i64, offset_minutes: i32) -> StructArray {
        let epoch_col: ArrayRef =
            Arc::new(PrimitiveArray::<Int64Type>::from(vec![Some(scaled_epoch)]));
        let tz_col: ArrayRef = Arc::new(PrimitiveArray::<Int32Type>::from(vec![Some(
            offset_minutes + 1440,
        )]));
        StructArray::from(vec![
            (
                Arc::new(ArrowField::new("epoch", DataType::Int64, false)),
                epoch_col,
            ),
            (
                Arc::new(ArrowField::new("tz_offset", DataType::Int32, false)),
                tz_col,
            ),
        ])
    }

    #[test]
    fn sql_type_returns_standard_timestamp_for_all_variants() {
        // Per the MS ODBC spec, `SQLDescribeCol` reports the standard
        // `SQL_TYPE_TIMESTAMP` (93) for all three variants (matches legacy
        // 3.16.0). Applications distinguish NTZ/LTZ/TZ via
        // `SQLColAttribute(SQL_DESC_TYPE_NAME)`.
        assert_eq!(ntz(0).sql_type(), sql::SqlDataType::TIMESTAMP);
        assert_eq!(ltz(3).sql_type(), sql::SqlDataType::TIMESTAMP);
        assert_eq!(tz(9).sql_type(), sql::SqlDataType::TIMESTAMP);
    }

    #[test]
    fn column_size_ntz_scale_0_is_19() {
        assert_eq!(ntz(0).column_size(), 19);
    }

    #[test]
    fn column_size_ntz_scale_9_is_29() {
        assert_eq!(ntz(9).column_size(), 29); // 20 + 9
    }

    #[test]
    fn column_size_ltz_matches_ntz() {
        // LTZ has the same wall-clock string layout as NTZ on the wire (no
        // offset suffix), so it shares NTZ's scale-aware column size.
        assert_eq!(ltz(0).column_size(), 19);
        assert_eq!(ltz(9).column_size(), 29);
    }

    #[test]
    fn column_size_tz_matches_ntz() {
        // TZ shares NTZ's scale-aware column size: the default fetch path
        // drops the offset and renders the bare timestamp, matching legacy
        // and the ODBC "datetime with timezone -> datetime without timezone"
        // rule.
        assert_eq!(tz(0).column_size(), 19);
        assert_eq!(tz(3).column_size(), 23);
        assert_eq!(tz(9).column_size(), 29);
    }

    #[test]
    fn decimal_digits_matches_scale() {
        for scale in 0..=9 {
            let sn = ntz(scale);
            assert_eq!(sn.decimal_digits(), scale as sql::SmallInt);
        }
    }

    /// LTZ wire encoding must produce a **bare** wall-clock literal string
    /// with no timezone offset suffix. The wire `type` is `TEXT` (see
    /// `SnowflakeTimestampLtz::sf_type`), and the Snowflake server uses the
    /// session timezone to interpret the wall-clock string when coercing
    /// into a TIMESTAMP_LTZ column. Mirrors the legacy 3.16.0 driver's
    /// JSON-bind path in `SFQueryExecutor.cpp`.
    #[test]
    fn ltz_write_wire_emits_bare_wall_clock_literal() {
        use crate::conversion::traits::WriteWire;
        use chrono::NaiveDate;
        let dt = NaiveDate::from_ymd_opt(2024, 3, 15)
            .and_then(|d| d.and_hms_nano_opt(14, 30, 45, 123_456_789))
            .expect("constant inputs");
        let v = ltz(9).write_wire(dt).expect("write_wire");
        assert_eq!(v, "2024-03-15 14:30:45.123456789");
    }

    /// Whole-second LTZ values must omit the fractional part entirely
    /// (matching legacy 3.16.0 and the existing fetch-side
    /// `format_timestamp_string_into` behaviour). Round-tripping a stored
    /// instant must not gain a `.000000000` suffix the application never
    /// emitted. No offset suffix either — LTZ binds are bare wall-clock.
    #[test]
    fn ltz_write_wire_omits_zero_nanos_no_offset() {
        use crate::conversion::traits::WriteWire;
        use chrono::NaiveDate;
        let dt = NaiveDate::from_ymd_opt(2024, 3, 15)
            .and_then(|d| d.and_hms_opt(14, 30, 45))
            .expect("constant inputs");
        let v = ltz(9).write_wire(dt).expect("write_wire");
        assert_eq!(v, "2024-03-15 14:30:45");
    }

    /// NTZ binds emit the same bare wall-clock literal string as LTZ (wire
    /// `type=TEXT`), so the server attaches the session `TIMEZONE` offset.
    #[test]
    fn ntz_write_wire_emits_bare_wall_clock_literal() {
        use crate::conversion::traits::WriteWire;
        use chrono::NaiveDate;
        let dt = NaiveDate::from_ymd_opt(2024, 3, 15)
            .and_then(|d| d.and_hms_nano_opt(14, 30, 45, 123_456_789))
            .expect("constant inputs");
        let v = ntz(9).write_wire(dt).expect("write_wire");
        assert_eq!(v, "2024-03-15 14:30:45.123456789");
    }
    // ---- TZ-specific write/read/wire round-trip tests ------------------------
    //
    // The helpers under test live in timestamp.rs; we exercise them here through
    // the public `WriteODBCType` / `ReadODBC` / `WriteWire` traits to keep the
    // tests resilient against private function renames.

    use crate::api::ParameterBinding;
    use crate::conversion::traits::{ReadODBC, WriteWire};

    fn make_naive(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f"))
            .expect("valid timestamp literal in test")
    }

    fn tz_value(utc: &str, offset_minutes: i32) -> crate::conversion::timestamp::TzInstant {
        crate::conversion::timestamp::TzInstant {
            utc: make_naive(utc),
            offset_minutes,
        }
    }

    #[test]
    fn write_tz_char_drops_offset_and_renders_utc() {
        // TZ -> SQL_C_CHAR drops the offset suffix on the fetch path: the
        // legacy 3.16.0 driver only emits `+/-HH:MM` when
        // TIMESTAMP_TZ_OUTPUT_FORMAT explicitly contains TZH/TZM tokens, and
        // the ODBC spec says "datetime with timezone -> datetime without
        // timezone" should anchor on UTC. Positive offset case.
        let sn = tz(9);
        let mut buffer = [0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = sn
            .write_odbc_type(tz_value("2024-01-15 09:00:45", 330), &binding, &mut None)
            .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(&buffer[..str_len as usize], b"2024-01-15 09:00:45");
    }

    #[test]
    fn write_tz_char_negative_offset_still_renders_utc() {
        let sn = tz(9);
        let mut buffer = [0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        sn.write_odbc_type(tz_value("2024-01-15 22:30:45", -480), &binding, &mut None)
            .unwrap();
        assert_eq!(&buffer[..str_len as usize], b"2024-01-15 22:30:45");
    }

    #[test]
    fn write_tz_char_zero_offset_renders_utc() {
        let sn = tz(9);
        let mut buffer = [0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        sn.write_odbc_type(tz_value("2024-01-15 14:30:45", 0), &binding, &mut None)
            .unwrap();
        assert_eq!(&buffer[..str_len as usize], b"2024-01-15 14:30:45");
    }

    #[test]
    fn write_tz_type_timestamp_returns_utc_ignoring_offset() {
        // `SQL_C_TYPE_TIMESTAMP` has no offset field; the spec requires returning
        // the UTC wall-clock. The `+05:30` info is intentionally dropped.
        let sn = tz(9);
        let mut value = sql::Timestamp::default();
        let mut str_len: sql::Len = 0;
        let binding =
            binding_for_value::<sql::Timestamp>(CDataType::TypeTimestamp, &mut value, &mut str_len);
        sn.write_odbc_type(tz_value("2024-01-15 09:00:45", 330), &binding, &mut None)
            .unwrap();
        assert_eq!(value.year, 2024);
        assert_eq!(value.hour, 9);
        assert_eq!(value.minute, 0);
        assert_eq!(value.second, 45);
    }

    /// Build a `ParameterBinding` over a borrowed byte buffer + indicator slot,
    /// suitable for exercising the `read_odbc` path on string inputs.
    fn tz_param_binding(s: &mut [u8], str_len: &mut sql::Len) -> ParameterBinding {
        // Restack note: the NTZ/LTZ/TZ vendor-code normalisation in
        // `bind_parameter` (PR #1004) flattens the on-record
        // `sql_data_type` to the standard `SQL_TYPE_TIMESTAMP` and
        // stashes the actual subtype on `sf_subtype`. Mirror that
        // contract in the helper so the readback path under test sees
        // the same shape it would in production.
        ParameterBinding {
            sql_data_type: sql::SqlDataType::TIMESTAMP,
            value_type: CDataType::Char,
            parameter_value_ptr: s.as_mut_ptr() as sql::Pointer,
            buffer_length: s.len() as sql::Len,
            str_len_or_ind_ptr: str_len as *mut sql::Len,
            sf_subtype: Some(crate::api::TimestampSubtype::Tz),
        }
    }

    #[test]
    fn read_tz_char_parses_offset_suffix() {
        // `read_odbc` must recover the offset from the trailing `+/-HH:MM` so
        // the JSON binding can re-emit it on the wire.
        let sn = tz(9);
        let mut s = b"2024-01-15 14:30:45 +05:30".to_vec();
        let mut str_len: sql::Len = s.len() as sql::Len;
        let binding = tz_param_binding(&mut s, &mut str_len);
        let value = sn.read_odbc(&binding).unwrap();
        assert_eq!(value.utc, make_naive("2024-01-15 09:00:45"));
        assert_eq!(value.offset_minutes, 330);
    }

    #[test]
    fn read_tz_char_negative_offset() {
        let sn = tz(9);
        let mut s = b"2024-01-15 14:30:45 -08:00".to_vec();
        let mut str_len: sql::Len = s.len() as sql::Len;
        let binding = tz_param_binding(&mut s, &mut str_len);
        let value = sn.read_odbc(&binding).unwrap();
        assert_eq!(value.utc, make_naive("2024-01-15 22:30:45"));
        assert_eq!(value.offset_minutes, -480);
    }

    #[test]
    fn read_tz_char_without_offset_falls_back_to_utc() {
        // Spec-grey-area: bare timestamps with no offset are treated as UTC
        // (matches legacy Python connector for naive datetimes bound to TZ).
        let sn = tz(9);
        let mut s = b"2024-01-15 14:30:45".to_vec();
        let mut str_len: sql::Len = s.len() as sql::Len;
        let binding = tz_param_binding(&mut s, &mut str_len);
        let value = sn.read_odbc(&binding).unwrap();
        assert_eq!(value.utc, make_naive("2024-01-15 14:30:45"));
        assert_eq!(value.offset_minutes, 0);
    }

    #[test]
    fn read_tz_char_unparseable_returns_error() {
        // Genuinely garbage input must surface an error so the bind fails
        // visibly rather than silently storing the Unix epoch.
        let sn = tz(9);
        let mut s = b"not a timestamp".to_vec();
        let mut str_len: sql::Len = s.len() as sql::Len;
        let binding = tz_param_binding(&mut s, &mut str_len);
        assert!(sn.read_odbc(&binding).is_err());
    }

    #[test]
    fn write_tz_wire_emits_two_token_format() {
        // Wire format: `<epoch_nanos> <offset_minutes + 1440>`. Server
        // subtracts the bias to recover the signed offset; legacy Python and
        // ODBC drivers use the same encoding.
        let sn = tz(9);
        let wire = sn.write_wire(tz_value("2024-01-15 09:00:45", 330)).unwrap();
        // 2024-01-15 09:00:45 UTC -> epoch_nanos = 1705309245000000000
        // offset 330 + 1440 = 1770
        assert_eq!(wire, "1705309245000000000 1770");
    }

    #[test]
    fn write_tz_wire_negative_offset_stays_positive_on_wire() {
        // -480 + 1440 = 960; the bias guarantees the second token is always
        // non-negative for any legal offset.
        let sn = tz(9);
        let wire = sn
            .write_wire(tz_value("2024-01-15 22:30:45", -480))
            .unwrap();
        let parts: Vec<&str> = wire.split(' ').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1], "960");
    }

    #[test]
    fn write_tz_wire_zero_offset_emits_bias() {
        // Naive bind path (offset = 0 -> wire token = 1440).
        let sn = tz(9);
        let wire = sn.write_wire(tz_value("2024-01-15 14:30:45", 0)).unwrap();
        assert!(
            wire.ends_with(" 1440"),
            "expected bias-only suffix, got {wire}"
        );
    }

    // ---- TZ -> CHAR/WCHAR with `tz_offset_format` set --------------------
    //
    // These tests cover the new fetch-side behaviour gated on the session
    // having `TIMESTAMP_TZ_OUTPUT_FORMAT` set to a value containing a
    // TZH/TZM/TZHTZM token. They mirror the legacy 3.16.0 driver's output
    // for the same format strings so a Tableau / Excel migration that
    // explicitly opted into TZ-aware output keeps seeing offsets.

    use crate::conversion::timestamp::{TzOffsetFormat, parse_tz_offset_format};

    #[test]
    fn parse_tz_offset_format_picks_longest_token() {
        // `TZH:TZM` and `TZHTZM` both contain `TZH`; the parser must
        // prefer the longest match so the colon variant doesn't bleed
        // into the no-colon variant for a user who configured the more
        // verbose format.
        assert_eq!(
            parse_tz_offset_format("YYYY-MM-DD HH24:MI:SS.FF TZH:TZM"),
            Some(TzOffsetFormat::Colon)
        );
        assert_eq!(
            parse_tz_offset_format("YYYY-MM-DD HH24:MI:SS.FF TZHTZM"),
            Some(TzOffsetFormat::NoColon)
        );
        assert_eq!(
            parse_tz_offset_format("YYYY-MM-DD HH24:MI:SS TZH"),
            Some(TzOffsetFormat::HourOnly)
        );
        // Case-insensitive — Snowflake's format grammar is too.
        assert_eq!(
            parse_tz_offset_format("yyyy-mm-dd hh24:mi:ss tzhtzm"),
            Some(TzOffsetFormat::NoColon)
        );
    }

    #[test]
    fn parse_tz_offset_format_returns_none_for_no_token() {
        assert_eq!(parse_tz_offset_format(""), None);
        assert_eq!(parse_tz_offset_format("YYYY-MM-DD HH24:MI:SS.FF"), None);
        // A bare "TZ" or stray Z must not match — only the documented
        // TZH/TZM/TZHTZM tokens are honoured.
        assert_eq!(parse_tz_offset_format("YYYY-MM-DD HH24:MI:SS Z"), None);
        assert_eq!(parse_tz_offset_format("any string with TZ but no H"), None);
    }

    /// Pin the substring-vs.-token boundary fix from PR #1068 review on
    /// `timestamp.rs:70`. The previous implementation used
    /// `String::contains`, which false-fired on every input below.
    /// Toggling wire-format bytes on a literal substring is a
    /// correctness bug: a customer who wrote a comment containing the
    /// letters `TZH` would silently get an offset suffix on every TZ
    /// fetch.
    #[test]
    fn parse_tz_offset_format_ignores_double_quoted_literals() {
        // Snowflake `"..."` literal text must not activate the
        // tokenizer. The literal is stripped before matching so even an
        // exact `TZH` between quotes is invisible to the match.
        assert_eq!(
            parse_tz_offset_format("\"comment with TZH\" YYYY-MM-DD"),
            None
        );
        assert_eq!(
            parse_tz_offset_format("\"server-side TZH note: \" YYYY-MM-DD HH24:MI:SS"),
            None,
            "TZH inside a double-quoted literal must not activate HourOnly"
        );
        // But a real TZH token *outside* the literal still wins.
        assert_eq!(
            parse_tz_offset_format("\"comment\" YYYY-MM-DD TZH"),
            Some(TzOffsetFormat::HourOnly)
        );
    }

    #[test]
    fn parse_tz_offset_format_rejects_alphanumeric_substrings() {
        // Alphanumeric tokens longer than the documented variants must
        // not match the bare `TZH` arm. Whole-token equality is the
        // only safe rule: `TZHACK` could plausibly be a Snowflake
        // pre-release token in the future, and silently rendering it as
        // `+HH` would be a wire-format regression.
        assert_eq!(parse_tz_offset_format("TZHACK"), None);
        assert_eq!(parse_tz_offset_format("TZHELP"), None);
        assert_eq!(parse_tz_offset_format("YYYY-MM-DD HH24:MI:SS TZHIRD"), None);
        assert_eq!(parse_tz_offset_format("literal_TZH_marker"), None);
        // Underscore is treated as part of the surrounding token, so
        // `_TZH_` is one identifier-shaped token (not three). This
        // matches the spirit of Snowflake format strings, where `_` is
        // a literal char rather than a token separator. A future
        // tokenizer tweak that flips `_` back to a separator would
        // re-introduce the false-positive class.
        assert_eq!(parse_tz_offset_format("prefix_TZH_suffix"), None);
    }

    #[test]
    fn parse_tz_offset_format_picks_longest_token_when_mixed() {
        // `TZH:TZM TZHTZM` mixes both colon and no-colon variants. The
        // colon variant is matched first and wins, mirroring the
        // longest-match-wins rule documented on `parse_tz_offset_format`.
        assert_eq!(
            parse_tz_offset_format("TZH:TZM TZHTZM"),
            Some(TzOffsetFormat::Colon)
        );
    }

    #[test]
    fn parse_tz_offset_format_colon_check_is_boundary_anchored() {
        // `XTZH:TZMX` is a hypothetical user format that contains the
        // colon-token sequence as a literal substring of two longer
        // alphanumeric tokens. Per the boundary rule it must not match,
        // because neither `TZH` nor `TZM` is a whole token.
        assert_eq!(parse_tz_offset_format("XTZH:TZMX"), None);
    }

    #[test]
    fn parse_tz_offset_format_unrecognised_tz_tokens_return_none() {
        // Snowflake additionally accepts `TZHM` (compact 4-char) and
        // bare `TZM`. The driver doesn't currently render these, so
        // they fall through to bare UTC. The implementation emits a
        // `tracing::warn!` for the fall-through (verified by reading
        // the source -- testing the warning macro itself is a separate
        // concern handled by tracing's own subscriber tests).
        assert_eq!(parse_tz_offset_format("YYYY-MM-DD HH24:MI:SS TZHM"), None);
        assert_eq!(parse_tz_offset_format("YYYY-MM-DD HH24:MI:SS TZM"), None);
    }

    #[test]
    fn write_tz_char_with_colon_format_emits_local_wall_clock_and_offset() {
        // 09:00:45 UTC with offset +05:30 = 14:30:45 local. The format is
        // `+HH:MM` (the verbose Snowflake "TZH:TZM" token).
        let sn = tz_with_format(9, TzOffsetFormat::Colon);
        let mut buffer = [0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        sn.write_odbc_type(tz_value("2024-01-15 09:00:45", 330), &binding, &mut None)
            .unwrap();
        assert_eq!(&buffer[..str_len as usize], b"2024-01-15 14:30:45 +05:30");
    }

    #[test]
    fn write_tz_char_with_no_colon_format_emits_compact_offset() {
        // Same instant, `TZHTZM` token -> `+0530` (no colon).
        let sn = tz_with_format(9, TzOffsetFormat::NoColon);
        let mut buffer = [0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        sn.write_odbc_type(tz_value("2024-01-15 09:00:45", 330), &binding, &mut None)
            .unwrap();
        assert_eq!(&buffer[..str_len as usize], b"2024-01-15 14:30:45 +0530");
    }

    #[test]
    fn write_tz_char_negative_offset_renders_with_minus_sign() {
        // 22:30:45 UTC with offset -08:00 = 14:30:45 local in Pacific.
        let sn = tz_with_format(9, TzOffsetFormat::Colon);
        let mut buffer = [0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        sn.write_odbc_type(tz_value("2024-01-15 22:30:45", -480), &binding, &mut None)
            .unwrap();
        assert_eq!(&buffer[..str_len as usize], b"2024-01-15 14:30:45 -08:00");
    }

    #[test]
    fn write_tz_char_zero_offset_renders_plus_zero_zero() {
        // UTC instant + offset 0 -> `+00:00` (not `-00:00`).
        let sn = tz_with_format(9, TzOffsetFormat::Colon);
        let mut buffer = [0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        sn.write_odbc_type(tz_value("2024-01-15 14:30:45", 0), &binding, &mut None)
            .unwrap();
        assert_eq!(&buffer[..str_len as usize], b"2024-01-15 14:30:45 +00:00");
    }

    #[test]
    fn write_tz_char_hour_only_falls_back_to_full_for_subhour_offset() {
        // The `TZH` token is hour-only when the offset has no minute
        // component, but +05:30 has 30 minutes — silently truncating to
        // `+05` would describe a different instant, so we fall back to
        // the full `+HH:MM` form (matches what the Snowflake server does
        // for the same token).
        let sn = tz_with_format(9, TzOffsetFormat::HourOnly);
        let mut buffer = [0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        sn.write_odbc_type(tz_value("2024-01-15 09:00:45", 330), &binding, &mut None)
            .unwrap();
        assert_eq!(&buffer[..str_len as usize], b"2024-01-15 14:30:45 +05:30");
    }

    #[test]
    fn write_tz_char_hour_only_emits_short_form_for_whole_hour_offset() {
        // Whole-hour offset with `TZH` token -> `+08` (no colon, no minutes).
        let sn = tz_with_format(9, TzOffsetFormat::HourOnly);
        let mut buffer = [0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        sn.write_odbc_type(tz_value("2024-01-15 06:30:00", 480), &binding, &mut None)
            .unwrap();
        assert_eq!(&buffer[..str_len as usize], b"2024-01-15 14:30:00 +08");
    }

    #[test]
    fn write_tz_wchar_with_colon_format_emits_utf16_with_offset() {
        // SQL_C_WCHAR path goes through the same formatter; only the
        // string-write helper differs. Verify the offset reaches WCHAR.
        let sn = tz_with_format(9, TzOffsetFormat::Colon);
        let mut buffer = [0u8; 128];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::WChar, &mut buffer, &mut str_len);
        sn.write_odbc_type(tz_value("2024-01-15 09:00:45", 330), &binding, &mut None)
            .unwrap();
        // UTF-16 LE: each ASCII char occupies 2 bytes, low byte first.
        let utf16: Vec<u16> = buffer[..str_len as usize]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let decoded = String::from_utf16(&utf16).unwrap();
        assert_eq!(decoded, "2024-01-15 14:30:45 +05:30");
    }

    #[test]
    fn write_tz_type_timestamp_with_format_still_returns_utc_ignoring_offset() {
        // The `tz_offset_format` toggle is CHAR/WCHAR-only. SQL_C_TYPE_TIMESTAMP
        // still has no offset field and the spec still requires UTC.
        let sn = tz_with_format(9, TzOffsetFormat::Colon);
        let mut value = sql::Timestamp::default();
        let mut str_len: sql::Len = 0;
        let binding =
            binding_for_value::<sql::Timestamp>(CDataType::TypeTimestamp, &mut value, &mut str_len);
        sn.write_odbc_type(tz_value("2024-01-15 09:00:45", 330), &binding, &mut None)
            .unwrap();
        assert_eq!(value.year, 2024);
        assert_eq!(value.hour, 9);
        assert_eq!(value.minute, 0);
        assert_eq!(value.second, 45);
    }

    #[test]
    fn write_tz_char_with_format_buffer_too_small_truncates_with_warning() {
        // A 25-byte buffer cannot hold `YYYY-MM-DD HH:MM:SS +HH:MM` (26
        // chars). Per ODBC spec for `SQLGetData` / `SQLFetch` the driver
        // must NOT pre-emptively reject with 22003 ("Numeric value out
        // of range") -- that's reserved for numeric overflow. The
        // correct behaviour is 01004 ("String data, right truncation")
        // with `SQL_SUCCESS_WITH_INFO`: write `buffer_length-1` bytes,
        // NUL-terminate, and set the indicator to the full untruncated
        // length so the application can resize and reissue. See PR
        // #1068 review on `timestamp.rs:993`.
        //
        // We assert with `matches!` against the exact `Warning` variant
        // rather than the previous `format!("{err:?}").contains(...)`,
        // which silently breaks on any rename. The test name and
        // assertion together pin the spec contract: undersized CHAR
        // buffer -> `StringDataTruncated` warning -> 01004 SQLSTATE on
        // the outer `SQLGetData` call.
        use crate::conversion::warning::Warning;
        let sn = tz_with_format(0, TzOffsetFormat::Colon);
        let mut buffer = [0u8; 25];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = sn
            .write_odbc_type(tz_value("2024-01-15 09:00:45", 330), &binding, &mut None)
            .expect("undersized buffer must succeed-with-info, not fail");
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated)),
            "expected StringDataTruncated warning on undersized CHAR buffer, got {warnings:?}"
        );
        // Indicator must report the full untruncated length (26) so the
        // application knows how big to resize.
        assert_eq!(str_len, 26);
        // And the buffer must be NUL-terminated at position
        // `buffer_length - 1` per the ODBC C-string contract; the first
        // 24 bytes must be the prefix of the rendered value.
        assert_eq!(buffer[24], 0);
        // The renderer applies the +330-minute offset to the UTC
        // instant, so the rendered local wall-clock is 14:30:45, not
        // the 09:00:45 UTC the test passed in. The 25-byte buffer fits
        // 24 visible bytes + NUL, hence the truncation point lands
        // mid-`+05:30`.
        assert_eq!(&buffer[..24], b"2024-01-15 14:30:45 +05:");
    }

    #[test]
    fn column_size_tz_with_offset_format_adds_seven() {
        // `+ +HH:MM` worst-case = 7 chars. Verify the descriptor reports
        // it so apps that size buffers from `column_size` are safe.
        for scale in [0u32, 3, 6, 9] {
            let base = if scale == 0 {
                19
            } else {
                20 + scale as sql::ULen
            };
            let sn = tz_with_format(scale, TzOffsetFormat::Colon);
            assert_eq!(sn.column_size(), base + 7);
        }
    }

    // =========================================================================
    // Year-range guard: timestamps decoded from the wire must fall within SQL
    // TIMESTAMP's 0001..9999 range.
    // =========================================================================

    fn epoch_secs(year: i32, month: u32, day: u32, h: u32, m: u32, s: u32) -> i64 {
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(h, m, s)
            .unwrap()
            .and_utc()
            .timestamp()
    }

    #[test]
    fn ntz_year_10000_rejected() {
        // Decode succeeds (chrono can hold year 10000); the SQL-range
        // policy check in validate_value rejects it.
        let sn = ntz(0);
        let array =
            PrimitiveArray::<Int64Type>::from(vec![Some(epoch_secs(9999, 12, 31, 23, 59, 59) + 1)]);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        let err = sn.validate_value(&value).unwrap_err();
        assert!(
            matches!(err, ConversionError::DatetimeOutOfSqlRange { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn tz_year_10000_rejected_struct_form() {
        // Same split as ntz: decode produces a TzInstant; validate_value
        // checks the embedded UTC year against SQL TIMESTAMP's range.
        let sn = tz(0);
        let array = make_tz_struct_array_2col(epoch_secs(9999, 12, 31, 23, 59, 59) + 1, 0);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        let err = sn.validate_value(&value).unwrap_err();
        assert!(
            matches!(err, ConversionError::DatetimeOutOfSqlRange { .. }),
            "got {err:?}"
        );
    }
}
