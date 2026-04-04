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
        NumericSettings, ReadArrowType, WriteODBCType, decimal_digits_from_field,
        sql_type_from_field,
    };

    fn ntz(scale: u32) -> SnowflakeTimestampNtz {
        SnowflakeTimestampNtz { scale }
    }

    fn ltz(scale: u32) -> SnowflakeTimestampLtz {
        SnowflakeTimestampLtz { scale }
    }

    fn tz(scale: u32) -> SnowflakeTimestampTz {
        SnowflakeTimestampTz { scale }
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
    fn read_scaled_tz_scale_over_9_returns_invalid() {
        let sn = tz(15);
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(0)]);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
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

    fn make_tz_struct_array_3col(epoch: i64, fraction: i32, tz_offset: i32) -> StructArray {
        let epoch_col: ArrayRef = Arc::new(PrimitiveArray::<Int64Type>::from(vec![Some(epoch)]));
        let frac_col: ArrayRef = Arc::new(PrimitiveArray::<Int32Type>::from(vec![Some(fraction)]));
        let tz_col: ArrayRef = Arc::new(PrimitiveArray::<Int32Type>::from(vec![Some(tz_offset)]));
        StructArray::from(vec![
            (
                Arc::new(ArrowField::new("epoch", DataType::Int64, false)),
                epoch_col,
            ),
            (
                Arc::new(ArrowField::new("fraction", DataType::Int32, false)),
                frac_col,
            ),
            (
                Arc::new(ArrowField::new("tz_offset", DataType::Int32, false)),
                tz_col,
            ),
        ])
    }

    fn make_single_col_struct_array(epoch: i64) -> StructArray {
        let epoch_col: ArrayRef = Arc::new(PrimitiveArray::<Int64Type>::from(vec![Some(epoch)]));
        StructArray::from(vec![(
            Arc::new(ArrowField::new("epoch", DataType::Int64, false)),
            epoch_col,
        )])
    }

    #[test]
    fn read_tz_3col_struct_valid() {
        let sn = tz(9);
        let array = make_tz_struct_array_3col(1_700_000_000, 0, 0);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert_eq!(value.and_utc().timestamp(), 1_700_000_000);
    }

    #[test]
    fn read_tz_2col_struct_valid() {
        let sn = tz(0);
        let array = make_struct_array(1_700_000_000, 0);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert_eq!(value.and_utc().timestamp(), 1_700_000_000);
    }

    #[test]
    fn read_tz_1col_struct_returns_invalid() {
        let sn = tz(0);
        let array = make_single_col_struct_array(1_700_000_000);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn read_tz_4col_struct_returns_invalid() {
        let sn = tz(9);
        let epoch_col: ArrayRef = Arc::new(PrimitiveArray::<Int64Type>::from(vec![Some(0i64)]));
        let frac_col: ArrayRef = Arc::new(PrimitiveArray::<Int32Type>::from(vec![Some(0i32)]));
        let tz_col: ArrayRef = Arc::new(PrimitiveArray::<Int32Type>::from(vec![Some(0i32)]));
        let extra_col: ArrayRef = Arc::new(PrimitiveArray::<Int32Type>::from(vec![Some(0i32)]));
        let array = StructArray::from(vec![
            (
                Arc::new(ArrowField::new("epoch", DataType::Int64, false)),
                epoch_col,
            ),
            (
                Arc::new(ArrowField::new("fraction", DataType::Int32, false)),
                frac_col,
            ),
            (
                Arc::new(ArrowField::new("tz_offset", DataType::Int32, false)),
                tz_col,
            ),
            (
                Arc::new(ArrowField::new("extra", DataType::Int32, false)),
                extra_col,
            ),
        ]);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn sql_type_is_timestamp_for_all_variants() {
        assert_eq!(ntz(0).sql_type(), sql::SqlDataType::TIMESTAMP);
        assert_eq!(ltz(3).sql_type(), sql::SqlDataType::TIMESTAMP);
        assert_eq!(tz(9).sql_type(), sql::SqlDataType::TIMESTAMP);
    }

    #[test]
    fn column_size_scale_0_is_19() {
        assert_eq!(ntz(0).column_size(), 19);
    }

    #[test]
    fn column_size_scale_9_is_29() {
        assert_eq!(ntz(9).column_size(), 29); // 20 + 9
    }

    #[test]
    fn decimal_digits_matches_scale() {
        for scale in 0..=9 {
            let sn = ntz(scale);
            assert_eq!(sn.decimal_digits(), scale as sql::SmallInt);
        }
    }
}
