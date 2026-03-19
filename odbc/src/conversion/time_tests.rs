#[cfg(test)]
mod tests {
    use crate::api::CDataType;
    use crate::conversion::ReadArrowType;
    use crate::conversion::WriteODBCType;
    use crate::conversion::error::ReadArrowError;
    use crate::conversion::test_utils::helpers::{
        binding_for_char_buffer, binding_for_value, binding_for_wchar_buffer,
    };
    use crate::conversion::time::SnowflakeTime;
    use arrow::array::PrimitiveArray;
    use arrow::datatypes::Int64Type;
    use chrono::NaiveTime;
    use odbc_sys as sql;

    fn time(scale: u32) -> SnowflakeTime {
        SnowflakeTime { scale }
    }

    // ========================================================================
    // ReadArrowType — scale handling
    // ========================================================================

    #[test]
    fn read_arrow_scale_0_whole_seconds() {
        let sn = time(0);
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(45296)]); // 12:34:56
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert_eq!(value, NaiveTime::from_hms_opt(12, 34, 56).unwrap());
    }

    #[test]
    fn read_arrow_scale_3_milliseconds() {
        let sn = time(3);
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(45_296_789)]); // 12:34:56.789
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert_eq!(
            value,
            NaiveTime::from_hms_milli_opt(12, 34, 56, 789).unwrap()
        );
    }

    #[test]
    fn read_arrow_scale_9_nanoseconds() {
        let sn = time(9);
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(45_296_123_456_789)]); // 12:34:56.123456789
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert_eq!(
            value,
            NaiveTime::from_hms_nano_opt(12, 34, 56, 123_456_789).unwrap()
        );
    }

    #[test]
    fn read_arrow_midnight() {
        let sn = time(9);
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(0)]);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert_eq!(value, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }

    // ========================================================================
    // ReadArrowType — null and error handling
    // ========================================================================

    #[test]
    fn read_arrow_null_returns_null_error() {
        let sn = time(9);
        let array = PrimitiveArray::<Int64Type>::from(vec![None::<i64>]);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(result, Err(ReadArrowError::NullValue { .. })));
    }

    #[test]
    fn read_arrow_negative_value_returns_invalid() {
        let sn = time(9);
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(-1)]);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn read_arrow_scale_over_9_returns_invalid() {
        let sn = time(10);
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(0)]);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn read_arrow_overflow_secs_returns_invalid() {
        let sn = time(0);
        // 86400 seconds = 24:00:00, out of valid range 0..86399
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(86_400)]);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn read_arrow_huge_value_returns_invalid() {
        let sn = time(9);
        // Value exceeding 24 hours in nanos — would wrap u32 without validation
        let array = PrimitiveArray::<Int64Type>::from(vec![Some(100_000_000_000_000_000)]);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    // ========================================================================
    // WriteODBCType — SQL_C_TYPE_TIME struct
    // ========================================================================

    #[test]
    fn write_time_struct() {
        let sn = time(9);
        let mut value = sql::Time {
            hour: 0,
            minute: 0,
            second: 0,
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::TypeTime, &mut value, &mut str_len);
        let input = NaiveTime::from_hms_opt(12, 34, 56).unwrap();
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.hour, 12);
        assert_eq!(value.minute, 34);
        assert_eq!(value.second, 56);
    }

    #[test]
    fn write_time_struct_default_type() {
        let sn = time(0);
        let mut value = sql::Time {
            hour: 0,
            minute: 0,
            second: 0,
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Default, &mut value, &mut str_len);
        let input = NaiveTime::from_hms_opt(23, 59, 59).unwrap();
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.hour, 23);
        assert_eq!(value.minute, 59);
        assert_eq!(value.second, 59);
    }

    // ========================================================================
    // WriteODBCType — SQL_C_CHAR
    // ========================================================================

    #[test]
    fn write_char() {
        let sn = time(0);
        let mut buffer = vec![0u8; 16];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let input = NaiveTime::from_hms_opt(9, 5, 3).unwrap();
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 8);
        assert_eq!(&buffer[..8], b"09:05:03");
    }

    #[test]
    fn write_char_buffer_too_small_truncates() {
        use crate::conversion::warning::Warning;
        let sn = time(0);
        let mut buffer = vec![0u8; 4];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let input = NaiveTime::from_hms_opt(12, 34, 56).unwrap();
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
    }

    // TODO: these tests document current behavior where fractional seconds
    // are dropped in string output. Once fractional-second formatting is
    // implemented, update these to verify the fractional part is included.
    #[test]
    fn write_char_scale_3_drops_fractional() {
        let sn = time(3);
        let mut buffer = vec![0u8; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let input = NaiveTime::from_hms_milli_opt(12, 34, 56, 789).unwrap();
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 8);
        assert_eq!(&buffer[..8], b"12:34:56");
    }

    #[test]
    fn write_wchar_scale_9_drops_fractional() {
        let sn = time(9);
        let mut buffer = vec![0u16; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let input = NaiveTime::from_hms_nano_opt(12, 34, 56, 123_456_789).unwrap();
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        let expected: Vec<u16> = "12:34:56".encode_utf16().collect();
        assert_eq!(&buffer[..expected.len()], &expected[..]);
    }

    // ========================================================================
    // WriteODBCType — SQL_C_WCHAR
    // ========================================================================

    #[test]
    fn write_wchar() {
        let sn = time(0);
        let mut buffer = vec![0u16; 16];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let input = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 16); // 8 UTF-16 code units * 2 bytes
        let expected: Vec<u16> = "12:00:00".encode_utf16().collect();
        assert_eq!(&buffer[..expected.len()], &expected[..]);
    }

    // ========================================================================
    // WriteODBCType — unsupported type
    // ========================================================================

    #[test]
    fn unsupported_type_returns_error() {
        let sn = time(0);
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::TypeDate, &mut value, &mut str_len);
        let input = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        assert!(sn.write_odbc_type(input, &binding, &mut None).is_err());
    }

    // ========================================================================
    // Metadata
    // ========================================================================

    #[test]
    fn sql_type_is_time() {
        let sn = time(0);
        assert_eq!(sn.sql_type(), sql::SqlDataType::TIME);
    }

    #[test]
    fn column_size_scale_0() {
        let sn = time(0);
        assert_eq!(sn.column_size(), 8);
    }

    #[test]
    fn column_size_scale_3() {
        let sn = time(3);
        assert_eq!(sn.column_size(), 12); // 9 + 3
    }

    #[test]
    fn column_size_scale_9() {
        let sn = time(9);
        assert_eq!(sn.column_size(), 18); // 9 + 9
    }

    #[test]
    fn decimal_digits_matches_scale() {
        for scale in 0..=9 {
            let sn = time(scale);
            assert_eq!(sn.decimal_digits(), scale as sql::SmallInt);
        }
    }
}
