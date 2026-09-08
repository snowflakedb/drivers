#[cfg(test)]
mod tests {
    use crate::api::CDataType;
    use crate::api::encoding::{WIDE_CHAR_SIZE, WideChar, encode_wide};
    use crate::conversion::error::{ReadArrowError, WriteOdbcError};
    use crate::conversion::interval_result::{IntervalDayTimeReader, IntervalYearMonthReader};
    use crate::conversion::test_utils::helpers::{
        binding_for_char_buffer, binding_for_interval, binding_for_wchar_buffer, zero_interval,
    };
    use crate::conversion::traits::Binding;
    use crate::conversion::warning::{Warning, Warnings};
    use crate::conversion::{ReadArrowType, WriteODBCType};
    use arrow::array::{Decimal128Array, Int32Array, Int64Array};
    use odbc_sys as sql;

    const NANOS_PER_SECOND: i128 = 1_000_000_000;
    const SECONDS_PER_DAY: i128 = 86_400;

    fn day_time_nanos(days: i128, hours: i128, minutes: i128, seconds: i128) -> i128 {
        ((days * SECONDS_PER_DAY) + (hours * 3_600) + (minutes * 60) + seconds) * NANOS_PER_SECOND
    }

    fn binding_for_fixed<T>(
        target_type: CDataType,
        value: &mut T,
        str_len: &mut sql::Len,
    ) -> Binding {
        Binding {
            target_type,
            target_value_ptr: value as *mut T as sql::Pointer,
            buffer_length: std::mem::size_of::<T>() as sql::Len,
            octet_length_ptr: str_len as *mut sql::Len,
            indicator_ptr: str_len as *mut sql::Len,
            ..Default::default()
        }
    }

    fn char_of(write: impl FnOnce(&Binding) -> Result<Warnings, WriteOdbcError>) -> String {
        let mut buffer = vec![0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = write(&binding).unwrap();
        assert!(warnings.is_empty());
        let len = str_len as usize;
        String::from_utf8(buffer[..len].to_vec()).unwrap()
    }

    // ======================================================================
    // YEAR_MONTH — decode + ANSI literal to Char
    // ======================================================================

    #[test]
    fn year_month_char_from_int64_positive() {
        let array = Int64Array::from(vec![Some(27)]);
        let reader = IntervalYearMonthReader;
        let value = reader.read_arrow_type(&array, 0).unwrap();
        assert_eq!(
            char_of(|b| reader.write_odbc_type(value, b, &mut None)),
            "2-03"
        );
    }

    #[test]
    fn year_month_char_from_int32_negative() {
        let array = Int32Array::from(vec![Some(-27)]);
        let reader = IntervalYearMonthReader;
        let value = reader.read_arrow_type(&array, 0).unwrap();
        assert_eq!(
            char_of(|b| reader.write_odbc_type(value, b, &mut None)),
            "-2-03"
        );
    }

    #[test]
    fn year_month_char_zero() {
        let array = Int64Array::from(vec![Some(0)]);
        let reader = IntervalYearMonthReader;
        let value = reader.read_arrow_type(&array, 0).unwrap();
        assert_eq!(
            char_of(|b| reader.write_odbc_type(value, b, &mut None)),
            "0-00"
        );
    }

    #[test]
    fn year_month_char_from_decimal128() {
        let array = Decimal128Array::from(vec![Some(1_000_000_000_000i128)]);
        let reader = IntervalYearMonthReader;
        let value = reader.read_arrow_type(&array, 0).unwrap();
        assert_eq!(
            char_of(|b| reader.write_odbc_type(value, b, &mut None)),
            "83333333333-04"
        );
    }

    #[test]
    fn year_month_wchar() {
        let array = Int64Array::from(vec![Some(27)]);
        let reader = IntervalYearMonthReader;
        let value = reader.read_arrow_type(&array, 0).unwrap();

        let mut buffer = vec![0 as WideChar; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        reader.write_odbc_type(value, &binding, &mut None).unwrap();

        let expected = encode_wide("2-03");
        assert_eq!(str_len, (expected.len() * WIDE_CHAR_SIZE) as sql::Len);
        assert_eq!(&buffer[..expected.len()], &expected[..]);
    }

    // ======================================================================
    // DAY_TIME — decode + ANSI literal to Char
    // ======================================================================

    #[test]
    fn day_time_char_scale_zero() {
        let array = Int64Array::from(vec![Some(day_time_nanos(1, 2, 3, 4) as i64)]);
        let reader = IntervalDayTimeReader { scale: 0 };
        let value = reader.read_arrow_type(&array, 0).unwrap();
        assert_eq!(
            char_of(|b| reader.write_odbc_type(value, b, &mut None)),
            "1 02:03:04"
        );
    }

    #[test]
    fn day_time_char_scale_six() {
        let nanos = day_time_nanos(0, 2, 3, 4) + 500_000_000;
        let array = Int64Array::from(vec![Some(nanos as i64)]);
        let reader = IntervalDayTimeReader { scale: 6 };
        let value = reader.read_arrow_type(&array, 0).unwrap();
        assert_eq!(
            char_of(|b| reader.write_odbc_type(value, b, &mut None)),
            "0 02:03:04.500000"
        );
    }

    #[test]
    fn day_time_char_negative() {
        let array = Int64Array::from(vec![Some(-(day_time_nanos(1, 2, 3, 4) as i64))]);
        let reader = IntervalDayTimeReader { scale: 0 };
        let value = reader.read_arrow_type(&array, 0).unwrap();
        assert_eq!(
            char_of(|b| reader.write_odbc_type(value, b, &mut None)),
            "-1 02:03:04"
        );
    }

    #[test]
    fn day_time_char_zero() {
        let array = Int64Array::from(vec![Some(0)]);
        let reader = IntervalDayTimeReader { scale: 0 };
        let value = reader.read_arrow_type(&array, 0).unwrap();
        assert_eq!(
            char_of(|b| reader.write_odbc_type(value, b, &mut None)),
            "0 00:00:00"
        );
    }

    #[test]
    fn day_time_char_from_decimal128_beyond_i64() {
        let beyond = i64::MAX as i128 + NANOS_PER_SECOND;
        let array = Decimal128Array::from(vec![Some(beyond)]);
        let reader = IntervalDayTimeReader { scale: 9 };
        let value = reader.read_arrow_type(&array, 0).unwrap();
        assert_eq!(value, beyond);
    }

    // ======================================================================
    // SQL_C_INTERVAL_* round-trips through varchar_to_interval
    // ======================================================================

    #[test]
    fn year_month_to_interval_year_to_month() {
        let reader = IntervalYearMonthReader;
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding =
            binding_for_interval(CDataType::IntervalYearToMonth, &mut interval, &mut str_len);

        reader.write_odbc_type(27, &binding, &mut None).unwrap();

        assert_eq!(interval.interval_type, sql::Interval::YearToMonth as i32);
        assert_eq!(interval.interval_sign, 0);
        assert_eq!(unsafe { interval.interval_value.year_month.year }, 2);
        assert_eq!(unsafe { interval.interval_value.year_month.month }, 3);
    }

    #[test]
    fn year_month_negative_to_interval_year_to_month() {
        let reader = IntervalYearMonthReader;
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding =
            binding_for_interval(CDataType::IntervalYearToMonth, &mut interval, &mut str_len);

        reader.write_odbc_type(-27, &binding, &mut None).unwrap();

        assert_eq!(interval.interval_sign, 1);
        assert_eq!(unsafe { interval.interval_value.year_month.year }, 2);
        assert_eq!(unsafe { interval.interval_value.year_month.month }, 3);
    }

    #[test]
    fn year_month_to_single_field_interval_year() {
        let reader = IntervalYearMonthReader;
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalYear, &mut interval, &mut str_len);

        reader.write_odbc_type(60, &binding, &mut None).unwrap();

        assert_eq!(interval.interval_type, sql::Interval::Year as i32);
        assert_eq!(unsafe { interval.interval_value.year_month.year }, 5);
    }

    #[test]
    fn day_time_to_interval_day_to_second() {
        let reader = IntervalDayTimeReader { scale: 0 };
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding =
            binding_for_interval(CDataType::IntervalDayToSecond, &mut interval, &mut str_len);

        reader
            .write_odbc_type(day_time_nanos(1, 2, 3, 4), &binding, &mut None)
            .unwrap();

        assert_eq!(interval.interval_type, sql::Interval::DayToSecond as i32);
        assert_eq!(interval.interval_sign, 0);
        assert_eq!(unsafe { interval.interval_value.day_second.day }, 1);
        assert_eq!(unsafe { interval.interval_value.day_second.hour }, 2);
        assert_eq!(unsafe { interval.interval_value.day_second.minute }, 3);
        assert_eq!(unsafe { interval.interval_value.day_second.second }, 4);
        assert_eq!(unsafe { interval.interval_value.day_second.fraction }, 0);
    }

    #[test]
    fn day_time_negative_to_interval_day_to_second() {
        let reader = IntervalDayTimeReader { scale: 0 };
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding =
            binding_for_interval(CDataType::IntervalDayToSecond, &mut interval, &mut str_len);

        reader
            .write_odbc_type(-day_time_nanos(1, 2, 3, 4), &binding, &mut None)
            .unwrap();

        assert_eq!(interval.interval_sign, 1);
        assert_eq!(unsafe { interval.interval_value.day_second.day }, 1);
        assert_eq!(unsafe { interval.interval_value.day_second.second }, 4);
    }

    #[test]
    fn day_time_to_single_field_interval_day() {
        let reader = IntervalDayTimeReader { scale: 0 };
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalDay, &mut interval, &mut str_len);

        reader
            .write_odbc_type(day_time_nanos(3, 0, 0, 0), &binding, &mut None)
            .unwrap();

        assert_eq!(interval.interval_type, sql::Interval::Day as i32);
        assert_eq!(unsafe { interval.interval_value.day_second.day }, 3);
    }

    // ======================================================================
    // Cross-family interval target → 07006
    // ======================================================================

    #[test]
    fn year_month_to_day_time_target_is_unsupported() {
        let reader = IntervalYearMonthReader;
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalDay, &mut interval, &mut str_len);

        let err = reader.write_odbc_type(27, &binding, &mut None).unwrap_err();
        assert!(matches!(err, WriteOdbcError::UnsupportedOdbcType { .. }));
    }

    #[test]
    fn day_time_to_year_month_target_is_unsupported() {
        let reader = IntervalDayTimeReader { scale: 0 };
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalYear, &mut interval, &mut str_len);

        let err = reader
            .write_odbc_type(day_time_nanos(1, 0, 0, 0), &binding, &mut None)
            .unwrap_err();
        assert!(matches!(err, WriteOdbcError::UnsupportedOdbcType { .. }));
    }

    // ======================================================================
    // YEAR_MONTH → scalar numeric = total months (integral, no truncation)
    // ======================================================================

    #[test]
    fn year_month_to_sbigint_is_total_months() {
        let reader = IntervalYearMonthReader;
        let mut value: i64 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::SBigInt, &mut value, &mut str_len);

        let warnings = reader.write_odbc_type(27, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, 27);
    }

    #[test]
    fn year_month_negative_to_long() {
        let reader = IntervalYearMonthReader;
        let mut value: i32 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Long, &mut value, &mut str_len);

        let warnings = reader.write_odbc_type(-27, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, -27);
    }

    #[test]
    fn year_month_to_short_out_of_range_is_22003() {
        let reader = IntervalYearMonthReader;
        let mut value: i16 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Short, &mut value, &mut str_len);

        let err = reader
            .write_odbc_type(i16::MAX as i128 + 1, &binding, &mut None)
            .unwrap_err();
        assert!(matches!(err, WriteOdbcError::NumericValueOutOfRange { .. }));
    }

    #[test]
    fn year_month_to_bit_out_of_range_is_22003() {
        let reader = IntervalYearMonthReader;
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Bit, &mut value, &mut str_len);

        let err = reader.write_odbc_type(27, &binding, &mut None).unwrap_err();
        assert!(matches!(err, WriteOdbcError::NumericValueOutOfRange { .. }));
    }

    #[test]
    fn year_month_to_numeric() {
        let reader = IntervalYearMonthReader;
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Numeric, &mut value, &mut str_len);

        let warnings = reader.write_odbc_type(-27, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.sign, 0);
        assert_eq!(value.val[0], 27);
        assert!(value.val[1..].iter().all(|&b| b == 0));
    }

    #[test]
    fn year_month_to_binary() {
        let reader = IntervalYearMonthReader;
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Binary, &mut buffer, &mut str_len);

        let warnings = reader.write_odbc_type(27, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        let numeric: &sql::Numeric = unsafe { &*(buffer.as_ptr() as *const sql::Numeric) };
        assert_eq!(numeric.sign, 1);
        assert_eq!(numeric.val[0], 27);
    }

    // ======================================================================
    // DAY_TIME → scalar numeric = total whole seconds; 01S07 when nanos lost
    // ======================================================================

    #[test]
    fn day_time_to_sbigint_is_whole_seconds() {
        let reader = IntervalDayTimeReader { scale: 0 };
        let mut value: i64 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::SBigInt, &mut value, &mut str_len);

        let warnings = reader
            .write_odbc_type(day_time_nanos(1, 2, 3, 4), &binding, &mut None)
            .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, 93_784);
    }

    #[test]
    fn day_time_fractional_seconds_truncate_with_warning() {
        let reader = IntervalDayTimeReader { scale: 9 };
        let mut value: i64 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::SBigInt, &mut value, &mut str_len);

        let nanos = day_time_nanos(0, 0, 0, 5) + 500_000_000;
        let warnings = reader.write_odbc_type(nanos, &binding, &mut None).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::NumericValueTruncated))
        );
        assert_eq!(value, 5);
    }

    #[test]
    fn day_time_negative_to_sbigint() {
        let reader = IntervalDayTimeReader { scale: 0 };
        let mut value: i64 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::SBigInt, &mut value, &mut str_len);

        let warnings = reader
            .write_odbc_type(-day_time_nanos(1, 2, 3, 4), &binding, &mut None)
            .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, -93_784);
    }

    #[test]
    fn day_time_to_numeric() {
        let reader = IntervalDayTimeReader { scale: 0 };
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Numeric, &mut value, &mut str_len);

        let warnings = reader
            .write_odbc_type(day_time_nanos(0, 0, 0, 42), &binding, &mut None)
            .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.sign, 1);
        assert_eq!(value.val[0], 42);
    }

    // ======================================================================
    // NULL cells surface ReadArrowError::NullValue
    // ======================================================================

    #[test]
    fn year_month_null_is_null_value_error() {
        let array = Int64Array::from(vec![None, Some(1)]);
        let reader = IntervalYearMonthReader;
        let err = reader.read_arrow_type(&array, 0).unwrap_err();
        assert!(matches!(err, ReadArrowError::NullValue { .. }));
    }

    #[test]
    fn day_time_null_is_null_value_error() {
        let array = Int64Array::from(vec![None, Some(1)]);
        let reader = IntervalDayTimeReader { scale: 0 };
        let err = reader.read_arrow_type(&array, 0).unwrap_err();
        assert!(matches!(err, ReadArrowError::NullValue { .. }));
    }

    // ======================================================================
    // Metadata
    // ======================================================================

    #[test]
    fn year_month_metadata() {
        let reader = IntervalYearMonthReader;
        assert_eq!(reader.sql_type(), sql::SqlDataType(107));
        assert_eq!(reader.column_size(), 5);
        assert_eq!(reader.decimal_digits(), 0);
    }

    #[test]
    fn day_time_metadata_scale_zero() {
        let reader = IntervalDayTimeReader { scale: 0 };
        assert_eq!(reader.sql_type(), sql::SqlDataType(110));
        assert_eq!(reader.column_size(), 11);
        assert_eq!(reader.decimal_digits(), 0);
    }

    #[test]
    fn day_time_metadata_scale_six() {
        let reader = IntervalDayTimeReader { scale: 6 };
        assert_eq!(reader.column_size(), 18);
        assert_eq!(reader.decimal_digits(), 6);
    }
}
