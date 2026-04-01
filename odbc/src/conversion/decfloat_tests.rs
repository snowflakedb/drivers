#[cfg(test)]
mod tests {
    use crate::api::CDataType;
    use crate::conversion::WriteODBCType;
    use crate::conversion::decfloat::{
        SnowflakeDecfloat, format_decfloat, i128_from_big_endian_signed,
    };
    use crate::conversion::error::{ReadArrowError, WriteOdbcError};
    use crate::conversion::test_utils::helpers::binding_for_wchar_buffer;
    use crate::conversion::traits::Binding;
    use crate::conversion::warning::Warning;
    use odbc_sys as sql;

    const PRECISION: usize = 38;

    fn decfloat() -> SnowflakeDecfloat {
        SnowflakeDecfloat {
            precision: PRECISION as u32,
        }
    }

    fn binding_for_char_buffer(
        target_type: CDataType,
        buffer: &mut [u8],
        str_len: &mut sql::Len,
    ) -> Binding {
        Binding {
            target_type,
            target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
            buffer_length: buffer.len() as sql::Len,
            octet_length_ptr: str_len as *mut sql::Len,
            indicator_ptr: str_len as *mut sql::Len,
            ..Default::default()
        }
    }

    // ======================================================================
    // i128_from_big_endian_signed conversion
    // ======================================================================

    #[test]
    fn i128_from_big_endian_signed_single_byte_positive() {
        assert_eq!(i128_from_big_endian_signed(&[0x01]).unwrap(), 1);
    }

    #[test]
    fn i128_from_big_endian_signed_single_byte_negative() {
        assert_eq!(i128_from_big_endian_signed(&[0xFF]).unwrap(), -1);
    }

    #[test]
    fn i128_from_big_endian_signed_single_byte_zero() {
        assert_eq!(i128_from_big_endian_signed(&[0x00]).unwrap(), 0);
    }

    #[test]
    fn i128_from_big_endian_signed_empty_bytes() {
        assert_eq!(i128_from_big_endian_signed(&[]).unwrap(), 0);
    }

    #[test]
    fn i128_from_big_endian_signed_single_byte_max_positive() {
        assert_eq!(i128_from_big_endian_signed(&[0x7F]).unwrap(), 127);
    }

    #[test]
    fn i128_from_big_endian_signed_single_byte_min_negative() {
        assert_eq!(i128_from_big_endian_signed(&[0x80]).unwrap(), -128);
    }

    #[test]
    fn i128_from_big_endian_signed_two_bytes() {
        assert_eq!(i128_from_big_endian_signed(&[0x01, 0x00]).unwrap(), 256);
    }

    #[test]
    fn i128_from_big_endian_signed_two_bytes_negative() {
        // 0xFF00 as i16 = -256
        assert_eq!(i128_from_big_endian_signed(&[0xFF, 0x00]).unwrap(), -256);
    }

    #[test]
    fn i128_from_big_endian_signed_large_positive() {
        // 123456789 = 0x075BCD15
        let bytes = 123456789i128.to_be_bytes();
        let trimmed = &bytes[bytes.iter().position(|&b| b != 0).unwrap_or(15)..];
        assert_eq!(i128_from_big_endian_signed(trimmed).unwrap(), 123456789);
    }

    #[test]
    fn i128_from_big_endian_signed_full_16_bytes_positive() {
        let val: i128 = 12345678901234567890;
        let bytes = val.to_be_bytes();
        assert_eq!(i128_from_big_endian_signed(&bytes).unwrap(), val);
    }

    #[test]
    fn i128_from_big_endian_signed_full_16_bytes_negative() {
        let val: i128 = -12345678901234567890;
        let bytes = val.to_be_bytes();
        assert_eq!(i128_from_big_endian_signed(&bytes).unwrap(), val);
    }

    #[test]
    fn i128_from_big_endian_signed_38_digit_significand() {
        let val: i128 = 12345678901234567890123456789012345678;
        let bytes = val.to_be_bytes();
        assert_eq!(i128_from_big_endian_signed(&bytes).unwrap(), val);
    }

    #[test]
    fn i128_from_big_endian_signed_exceeds_16_bytes_returns_error() {
        let oversized = [0u8; 17];
        let result = i128_from_big_endian_signed(&oversized);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn i128_from_big_endian_signed_much_larger_than_16_bytes_returns_error() {
        let oversized = vec![0xABu8; 32];
        let result = i128_from_big_endian_signed(&oversized);
        assert!(matches!(
            result,
            Err(ReadArrowError::InvalidArrowValue { .. })
        ));
    }

    #[test]
    fn i128_from_big_endian_signed_max_value() {
        let bytes = i128::MAX.to_be_bytes();
        assert_eq!(i128_from_big_endian_signed(&bytes).unwrap(), i128::MAX);
    }

    #[test]
    fn i128_from_big_endian_signed_min_value() {
        let bytes = i128::MIN.to_be_bytes();
        assert_eq!(i128_from_big_endian_signed(&bytes).unwrap(), i128::MIN);
    }

    // ======================================================================
    // format_decfloat — zero
    // ======================================================================

    #[test]
    fn format_zero() {
        assert_eq!(format_decfloat(0, 0, PRECISION), "0");
    }

    #[test]
    fn format_zero_with_exponent() {
        assert_eq!(format_decfloat(0, 100, PRECISION), "0");
    }

    #[test]
    fn format_zero_with_negative_exponent() {
        assert_eq!(format_decfloat(0, -100, PRECISION), "0");
    }

    // ======================================================================
    // format_decfloat — simple integers (exp = 0)
    // ======================================================================

    #[test]
    fn format_simple_positive_integer() {
        assert_eq!(format_decfloat(42, 0, PRECISION), "42");
    }

    #[test]
    fn format_simple_negative_integer() {
        assert_eq!(format_decfloat(-42, 0, PRECISION), "-42");
    }

    #[test]
    fn format_one() {
        assert_eq!(format_decfloat(1, 0, PRECISION), "1");
    }

    // ======================================================================
    // format_decfloat — simple decimals (negative exp within digits)
    // ======================================================================

    #[test]
    fn format_simple_decimal() {
        // 123456 * 10^-3 = 123.456
        assert_eq!(format_decfloat(123456, -3, PRECISION), "123.456");
    }

    #[test]
    fn format_negative_decimal() {
        // -789012 * 10^-3 = -789.012
        assert_eq!(format_decfloat(-789012, -3, PRECISION), "-789.012");
    }

    #[test]
    fn format_decimal_one_point_five() {
        // 15 * 10^-1 = 1.5
        assert_eq!(format_decfloat(15, -1, PRECISION), "1.5");
    }

    #[test]
    fn format_decimal_negative_one_point_five() {
        assert_eq!(format_decfloat(-15, -1, PRECISION), "-1.5");
    }

    #[test]
    fn format_decimal_123_456789() {
        // 123456789 * 10^-6 = 123.456789
        assert_eq!(format_decfloat(123456789, -6, PRECISION), "123.456789");
    }

    #[test]
    fn format_decimal_negative_987_654321() {
        assert_eq!(format_decfloat(-987654321, -6, PRECISION), "-987.654321");
    }

    // ======================================================================
    // format_decfloat — trailing zeros (positive exp, plain form)
    // ======================================================================

    #[test]
    fn format_trailing_zeros_small() {
        // 123 * 10^2 = 12300
        assert_eq!(format_decfloat(123, 2, PRECISION), "12300");
    }

    #[test]
    fn format_trailing_zeros_38_chars() {
        // 123 * 10^35 = 38 chars plain
        let result = format_decfloat(123, 35, PRECISION);
        assert_eq!(result, "12300000000000000000000000000000000000");
        assert_eq!(result.len(), 38);
    }

    #[test]
    fn format_trailing_zeros_exceeds_38_uses_scientific() {
        // 123 * 10^36 = 39 chars plain -> scientific
        assert_eq!(format_decfloat(123, 36, PRECISION), "1.23e38");
    }

    #[test]
    fn format_1e20() {
        // 123 * 10^18 = 123000000000000000000
        assert_eq!(format_decfloat(123, 18, PRECISION), "123000000000000000000");
    }

    // ======================================================================
    // format_decfloat — leading zeros (negative exp >= n, plain form)
    // ======================================================================

    #[test]
    fn format_leading_zeros_small() {
        // 987 * 10^-5 = 0.00987
        assert_eq!(format_decfloat(987, -5, PRECISION), "0.00987");
    }

    #[test]
    fn format_leading_zeros_negative_987e_neg17() {
        // -987 * 10^-17 = -0.00000000000000987 (matches E2E test for '-9.87E-15')
        assert_eq!(
            format_decfloat(-987, -17, PRECISION),
            "-0.00000000000000987"
        );
    }

    #[test]
    fn format_leading_zeros_at_boundary() {
        // 1 * 10^-36: plain length = 2 + 36 = 38 <= 38, use plain
        let result = format_decfloat(1, -36, PRECISION);
        assert_eq!(result, "0.000000000000000000000000000000000001");
        assert_eq!(result.len(), 38);
    }

    #[test]
    fn format_leading_zeros_exceeds_boundary() {
        // 1 * 10^-37: plain length = 2 + 37 = 39 > 38, use scientific
        assert_eq!(format_decfloat(1, -37, PRECISION), "1e-37");
    }

    // ======================================================================
    // format_decfloat — scientific notation
    // ======================================================================

    #[test]
    fn format_scientific_large_positive_exponent() {
        // 1 * 10^16384
        assert_eq!(format_decfloat(1, 16384, PRECISION), "1e16384");
    }

    #[test]
    fn format_scientific_large_negative_exponent() {
        // 1 * 10^-16383
        assert_eq!(format_decfloat(1, -16383, PRECISION), "1e-16383");
    }

    #[test]
    fn format_scientific_with_decimal() {
        // -1234 * 10^7997 = -1.234e8000
        assert_eq!(format_decfloat(-1234, 7997, PRECISION), "-1.234e8000");
    }

    #[test]
    fn format_scientific_positive_small_significand() {
        // 9876 * 10^-8003 = 9.876e-8000
        assert_eq!(format_decfloat(9876, -8003, PRECISION), "9.876e-8000");
    }

    #[test]
    fn format_scientific_38_digit_significand_positive_exp() {
        // 12345678901234567890123456789012345678 * 10^63
        // adjusted_exp = 63 + 38 - 1 = 100
        let sig: i128 = 12345678901234567890123456789012345678;
        let result = format_decfloat(sig, 63, PRECISION);
        assert_eq!(result, "1.2345678901234567890123456789012345678e100");
    }

    #[test]
    fn format_scientific_38_digit_significand_negative_exp() {
        // 12345678901234567890123456789012345678 * 10^-137
        // adjusted_exp = -137 + 38 - 1 = -100
        let sig: i128 = 12345678901234567890123456789012345678;
        let result = format_decfloat(sig, -137, PRECISION);
        assert_eq!(result, "1.2345678901234567890123456789012345678e-100");
    }

    // ======================================================================
    // format_decfloat — trailing zero normalization
    // ======================================================================

    #[test]
    fn format_normalizes_trailing_zeros_in_significand() {
        // 42000 * 10^-3 should normalize to 42 * 10^0 = "42"
        assert_eq!(format_decfloat(42000, -3, PRECISION), "42");
    }

    #[test]
    fn format_normalizes_trailing_zeros_decimal() {
        // 1230 * 10^-2 should normalize to 123 * 10^-1 = "12.3"
        assert_eq!(format_decfloat(1230, -2, PRECISION), "12.3");
    }

    // ======================================================================
    // format_decfloat — E2E test expectations
    // ======================================================================

    #[test]
    fn format_e2e_cast_zero() {
        assert_eq!(format_decfloat(0, 0, PRECISION), "0");
    }

    #[test]
    fn format_e2e_cast_123_456() {
        assert_eq!(format_decfloat(123456, -3, PRECISION), "123.456");
    }

    #[test]
    fn format_e2e_select_42_5() {
        // 425 * 10^-1
        assert_eq!(format_decfloat(425, -1, PRECISION), "42.5");
    }

    // ======================================================================
    // WriteODBCType — Default and Char targets
    // ======================================================================

    #[test]
    fn write_default_produces_char_string() {
        let df = decfloat();
        let mut buffer = vec![0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Default, &mut buffer, &mut str_len);

        let warnings = df
            .write_odbc_type((123456, -3), &binding, &mut None)
            .unwrap();

        assert!(warnings.is_empty());
        let expected = b"123.456";
        assert_eq!(str_len, expected.len() as sql::Len);
        assert_eq!(&buffer[..expected.len()], expected);
        assert_eq!(buffer[expected.len()], 0);
    }

    #[test]
    fn write_char_produces_string() {
        let df = decfloat();
        let mut buffer = vec![0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);

        let warnings = df.write_odbc_type((1, 16384), &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        let expected = b"1e16384";
        assert_eq!(str_len, expected.len() as sql::Len);
        assert_eq!(&buffer[..expected.len()], expected);
        assert_eq!(buffer[expected.len()], 0);
    }

    #[test]
    fn write_char_zero() {
        let df = decfloat();
        let mut buffer = vec![0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);

        let warnings = df.write_odbc_type((0, 0), &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        assert_eq!(str_len, 1);
        assert_eq!(buffer[0], b'0');
        assert_eq!(buffer[1], 0);
    }

    #[test]
    fn write_char_negative() {
        let df = decfloat();
        let mut buffer = vec![0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);

        let warnings = df
            .write_odbc_type((-1234, 7997), &binding, &mut None)
            .unwrap();

        assert!(warnings.is_empty());
        let expected = b"-1.234e8000";
        assert_eq!(str_len, expected.len() as sql::Len);
        assert_eq!(&buffer[..expected.len()], expected);
    }

    #[test]
    fn write_char_38_digit_precision() {
        let df = decfloat();
        let mut buffer = vec![0u8; 128];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);

        let sig: i128 = 12345678901234567890123456789012345678;
        let warnings = df.write_odbc_type((sig, 0), &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        let expected = b"12345678901234567890123456789012345678";
        assert_eq!(str_len, expected.len() as sql::Len);
        assert_eq!(&buffer[..expected.len()], expected);
    }

    // ======================================================================
    // WriteODBCType — extreme exponents to SQL_C_CHAR
    // ======================================================================

    #[test]
    fn write_char_extreme_negative_exponent() {
        let df = decfloat();
        let mut buffer = vec![0u8; 64];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);

        let warnings = df
            .write_odbc_type((1, -16383), &binding, &mut None)
            .unwrap();

        assert!(warnings.is_empty());
        let expected = b"1e-16383";
        assert_eq!(str_len, expected.len() as sql::Len);
        assert_eq!(&buffer[..expected.len()], expected);
    }

    #[test]
    fn write_char_38_digit_with_large_positive_exponent() {
        let df = decfloat();
        let mut buffer = vec![0u8; 128];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);

        let sig: i128 = 12345678901234567890123456789012345678;
        let warnings = df.write_odbc_type((sig, 63), &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        let expected = b"1.2345678901234567890123456789012345678e100";
        assert_eq!(str_len, expected.len() as sql::Len);
        assert_eq!(&buffer[..expected.len()], expected);
    }

    #[test]
    fn write_char_38_digit_with_large_negative_exponent() {
        let df = decfloat();
        let mut buffer = vec![0u8; 128];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);

        let sig: i128 = 12345678901234567890123456789012345678;
        let warnings = df
            .write_odbc_type((sig, -137), &binding, &mut None)
            .unwrap();

        assert!(warnings.is_empty());
        let expected = b"1.2345678901234567890123456789012345678e-100";
        assert_eq!(str_len, expected.len() as sql::Len);
        assert_eq!(&buffer[..expected.len()], expected);
    }

    // ======================================================================
    // Extreme exponents to integer type (regression: overflow panic)
    // ======================================================================

    #[test]
    fn write_long_extreme_negative_exponent_truncates_to_zero() {
        let df = decfloat();
        let mut value: i32 = 99;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Long, &mut value, &mut str_len);

        let warnings = df
            .write_odbc_type((1, -16383), &binding, &mut None)
            .unwrap();

        assert_eq!(value, 0);
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::NumericValueTruncated))
        );
    }

    #[test]
    fn write_long_extreme_negative_exponent_zero_sig() {
        let df = decfloat();
        let mut value: i32 = 99;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Long, &mut value, &mut str_len);

        let warnings = df
            .write_odbc_type((0, -16383), &binding, &mut None)
            .unwrap();

        assert_eq!(value, 0);
        assert!(warnings.is_empty());
    }

    // ======================================================================
    // WriteODBCType — interval types
    // ======================================================================

    fn binding_for_interval(
        target_type: CDataType,
        value: &mut sql::IntervalStruct,
        str_len: &mut sql::Len,
    ) -> Binding {
        Binding {
            target_type,
            target_value_ptr: value as *mut sql::IntervalStruct as sql::Pointer,
            buffer_length: 0,
            octet_length_ptr: str_len as *mut sql::Len,
            indicator_ptr: str_len as *mut sql::Len,
            ..Default::default()
        }
    }

    fn zero_interval() -> sql::IntervalStruct {
        sql::IntervalStruct {
            interval_type: 0,
            interval_sign: 0,
            interval_value: sql::IntervalUnion {
                day_second: sql::DaySecond::default(),
            },
        }
    }

    #[test]
    fn write_interval_year_positive() {
        let df = decfloat();
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalYear, &mut interval, &mut str_len);

        let warnings = df.write_odbc_type((5, 0), &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        assert_eq!(interval.interval_type, sql::Interval::Year as i32);
        assert_eq!(interval.interval_sign, 0);
        assert_eq!(unsafe { interval.interval_value.year_month.year }, 5);
    }

    #[test]
    fn write_interval_year_negative() {
        let df = decfloat();
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalYear, &mut interval, &mut str_len);

        let warnings = df.write_odbc_type((-3, 0), &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        assert_eq!(interval.interval_type, sql::Interval::Year as i32);
        assert_eq!(interval.interval_sign, 1);
        assert_eq!(unsafe { interval.interval_value.year_month.year }, 3);
    }

    #[test]
    fn write_interval_month() {
        let df = decfloat();
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalMonth, &mut interval, &mut str_len);

        let warnings = df.write_odbc_type((10, 0), &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        assert_eq!(interval.interval_type, sql::Interval::Month as i32);
        assert_eq!(unsafe { interval.interval_value.year_month.month }, 10);
    }

    #[test]
    fn write_interval_day() {
        let df = decfloat();
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalDay, &mut interval, &mut str_len);

        let warnings = df.write_odbc_type((15, 0), &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        assert_eq!(interval.interval_type, sql::Interval::Day as i32);
        assert_eq!(unsafe { interval.interval_value.day_second.day }, 15);
    }

    #[test]
    fn write_interval_second_integer() {
        let df = decfloat();
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalSecond, &mut interval, &mut str_len);

        let warnings = df.write_odbc_type((45, 0), &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        assert_eq!(interval.interval_type, sql::Interval::Second as i32);
        assert_eq!(unsafe { interval.interval_value.day_second.second }, 45);
        assert_eq!(unsafe { interval.interval_value.day_second.fraction }, 0);
    }

    #[test]
    fn write_interval_year_fractional_truncation() {
        let df = decfloat();
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalYear, &mut interval, &mut str_len);

        let warnings = df.write_odbc_type((57, -1), &binding, &mut None).unwrap();

        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::NumericValueTruncated))
        );
        assert_eq!(unsafe { interval.interval_value.year_month.year }, 5);
    }

    #[test]
    fn write_interval_second_with_fraction() {
        let df = decfloat();
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalSecond, &mut interval, &mut str_len);

        let warnings = df
            .write_odbc_type((45500, -3), &binding, &mut None)
            .unwrap();

        assert!(warnings.is_empty());
        assert_eq!(unsafe { interval.interval_value.day_second.second }, 45);
        assert_eq!(
            unsafe { interval.interval_value.day_second.fraction },
            500000
        );
    }

    #[test]
    fn write_interval_multi_field_returns_error() {
        let df = decfloat();
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;

        for target_type in [
            CDataType::IntervalYearToMonth,
            CDataType::IntervalDayToHour,
            CDataType::IntervalDayToMinute,
            CDataType::IntervalDayToSecond,
            CDataType::IntervalHourToMinute,
            CDataType::IntervalHourToSecond,
            CDataType::IntervalMinuteToSecond,
        ] {
            let binding = binding_for_interval(target_type, &mut interval, &mut str_len);
            let result = df.write_odbc_type((42, 0), &binding, &mut None);
            assert!(matches!(
                result,
                Err(WriteOdbcError::IntervalFieldOverflow { .. })
            ));
        }
    }

    // ======================================================================
    // WriteODBCType — unsupported types
    // ======================================================================

    #[test]
    fn write_unsupported_date_type_returns_error() {
        let df = decfloat();
        let mut value = [0u8; 32];
        let mut str_len: sql::Len = 0;
        let binding = Binding {
            target_type: CDataType::TypeDate,
            target_value_ptr: value.as_mut_ptr() as sql::Pointer,
            buffer_length: value.len() as sql::Len,
            octet_length_ptr: &mut str_len as *mut sql::Len,
            indicator_ptr: &mut str_len as *mut sql::Len,
            ..Default::default()
        };

        assert!(df.write_odbc_type((123, 0), &binding, &mut None).is_err());
    }

    // ======================================================================
    // WriteODBCType — SQL_C_DOUBLE
    // ======================================================================

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

    #[test]
    fn write_double_integer() {
        let df = decfloat();
        let mut value: f64 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Double, &mut value, &mut str_len);
        let warnings = df.write_odbc_type((42, 0), &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert!((value - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn write_double_fractional() {
        let df = decfloat();
        let mut value: f64 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Double, &mut value, &mut str_len);
        let warnings = df
            .write_odbc_type((123456, -3), &binding, &mut None)
            .unwrap();
        assert!(warnings.is_empty());
        assert!((value - 123.456).abs() < 1e-10);
    }

    #[test]
    fn write_double_negative() {
        let df = decfloat();
        let mut value: f64 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Double, &mut value, &mut str_len);
        let warnings = df.write_odbc_type((-425, -1), &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert!((value - (-42.5)).abs() < f64::EPSILON);
    }

    // ======================================================================
    // WriteODBCType — SQL_C_FLOAT
    // ======================================================================

    #[test]
    fn write_float_integer() {
        let df = decfloat();
        let mut value: f32 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Float, &mut value, &mut str_len);
        let warnings = df.write_odbc_type((42, 0), &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert!((value - 42.0f32).abs() < f32::EPSILON);
    }

    // ======================================================================
    // WriteODBCType — integer types
    // ======================================================================

    #[test]
    fn write_long_integer() {
        let df = decfloat();
        let mut value: i32 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Long, &mut value, &mut str_len);
        let warnings = df.write_odbc_type((42, 0), &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, 42);
    }

    #[test]
    fn write_long_fractional_truncation() {
        let df = decfloat();
        let mut value: i32 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Long, &mut value, &mut str_len);
        let warnings = df
            .write_odbc_type((123456, -3), &binding, &mut None)
            .unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::NumericValueTruncated))
        );
        assert_eq!(value, 123);
    }

    #[test]
    fn write_long_overflow_returns_error() {
        let df = decfloat();
        let mut value: i32 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Long, &mut value, &mut str_len);
        assert!(
            df.write_odbc_type((99999999999999999, 0), &binding, &mut None)
                .is_err()
        );
    }

    #[test]
    fn write_sbigint() {
        let df = decfloat();
        let mut value: i64 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::SBigInt, &mut value, &mut str_len);
        let warnings = df.write_odbc_type((42, 0), &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, 42);
    }

    #[test]
    fn write_ubigint_negative_returns_error() {
        let df = decfloat();
        let mut value: u64 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::UBigInt, &mut value, &mut str_len);
        assert!(df.write_odbc_type((-1, 0), &binding, &mut None).is_err());
    }

    #[test]
    fn write_short() {
        let df = decfloat();
        let mut value: i16 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Short, &mut value, &mut str_len);
        let warnings = df.write_odbc_type((42, 0), &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, 42);
    }

    #[test]
    fn write_tinyint() {
        let df = decfloat();
        let mut value: i8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::TinyInt, &mut value, &mut str_len);
        let warnings = df.write_odbc_type((42, 0), &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, 42);
    }

    #[test]
    fn write_integer_with_positive_exponent() {
        let df = decfloat();
        let mut value: i32 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Long, &mut value, &mut str_len);
        // 42 * 10^2 = 4200
        let warnings = df.write_odbc_type((42, 2), &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, 4200);
    }

    // ======================================================================
    // WriteODBCType — SQL_C_BIT
    // ======================================================================

    #[test]
    fn write_bit_zero() {
        let df = decfloat();
        let mut value: u8 = 0xFF;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Bit, &mut value, &mut str_len);
        let warnings = df.write_odbc_type((0, 0), &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, 0);
    }

    #[test]
    fn write_bit_one() {
        let df = decfloat();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Bit, &mut value, &mut str_len);
        let warnings = df.write_odbc_type((1, 0), &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, 1);
    }

    #[test]
    fn write_bit_out_of_range() {
        let df = decfloat();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Bit, &mut value, &mut str_len);
        assert!(df.write_odbc_type((2, 0), &binding, &mut None).is_err());
    }

    #[test]
    fn write_bit_negative_returns_error() {
        let df = decfloat();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Bit, &mut value, &mut str_len);
        assert!(df.write_odbc_type((-1, 0), &binding, &mut None).is_err());
    }

    #[test]
    fn write_bit_fractional_truncation() {
        let df = decfloat();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Bit, &mut value, &mut str_len);
        // 0.5 = 5 * 10^-1
        let warnings = df.write_odbc_type((5, -1), &binding, &mut None).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::NumericValueTruncated))
        );
        assert_eq!(value, 0);
    }

    // ======================================================================
    // WriteODBCType — SQL_C_NUMERIC
    // ======================================================================

    #[test]
    fn write_numeric_positive_integer() {
        let df = decfloat();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Numeric, &mut value, &mut str_len);
        let warnings = df.write_odbc_type((42, 0), &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.sign, 1);
        assert_eq!(value.val[0], 42);
        assert!(value.val[1..].iter().all(|&b| b == 0));
    }

    #[test]
    fn write_numeric_negative() {
        let df = decfloat();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Numeric, &mut value, &mut str_len);
        let warnings = df.write_odbc_type((-123, 0), &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.sign, 0);
        assert_eq!(value.val[0], 123);
    }

    #[test]
    fn write_numeric_fractional_truncation() {
        let df = decfloat();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Numeric, &mut value, &mut str_len);
        // 123.456 = 123456 * 10^-3, default scale=0 -> truncate to 123
        let warnings = df
            .write_odbc_type((123456, -3), &binding, &mut None)
            .unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::NumericValueTruncated))
        );
        assert_eq!(value.val[0], 123);
    }

    // ======================================================================
    // WriteODBCType — SQL_C_BINARY
    // ======================================================================

    #[test]
    fn write_binary_integer() {
        let df = decfloat();
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = Binding {
            target_type: CDataType::Binary,
            target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
            buffer_length: buffer.len() as sql::Len,
            octet_length_ptr: &mut str_len as *mut sql::Len,
            indicator_ptr: &mut str_len as *mut sql::Len,
            ..Default::default()
        };
        let warnings = df.write_odbc_type((42, 0), &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        let numeric: &sql::Numeric = unsafe { &*(buffer.as_ptr() as *const sql::Numeric) };
        assert_eq!(numeric.sign, 1);
        assert_eq!(numeric.val[0], 42);
    }

    #[test]
    fn write_binary_buffer_too_small_returns_error() {
        let df = decfloat();
        let mut buffer = vec![0u8; 4]; // too small for sql::Numeric
        let mut str_len: sql::Len = 0;
        let binding = Binding {
            target_type: CDataType::Binary,
            target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
            buffer_length: buffer.len() as sql::Len,
            octet_length_ptr: &mut str_len as *mut sql::Len,
            indicator_ptr: &mut str_len as *mut sql::Len,
            ..Default::default()
        };
        assert!(df.write_odbc_type((42, 0), &binding, &mut None).is_err());
    }

    // ======================================================================
    // WriteODBCType — metadata
    // ======================================================================

    #[test]
    fn sql_type_is_numeric() {
        let df = decfloat();
        assert_eq!(df.sql_type(), sql::SqlDataType::NUMERIC);
    }

    #[test]
    fn column_size_is_38() {
        let df = decfloat();
        assert_eq!(df.column_size(), 38);
    }

    #[test]
    fn decimal_digits_is_0() {
        let df = decfloat();
        assert_eq!(df.decimal_digits(), 0);
    }

    // ======================================================================
    // WriteODBCType — buffer truncation (Char)
    // ======================================================================

    #[test]
    fn write_char_truncation_returns_warning() {
        let df = decfloat();
        let mut buffer = vec![0u8; 4];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);

        let warnings = df
            .write_odbc_type((123456, -3), &binding, &mut None)
            .unwrap();

        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
        assert_eq!(str_len, 7); // "123.456".len()
        assert_eq!(&buffer[..3], b"123");
        assert_eq!(buffer[3], 0);
    }

    // ======================================================================
    // WriteODBCType — get_data_offset chunked reading (Char)
    // ======================================================================

    #[test]
    fn write_char_chunked_reading() {
        let df = decfloat();
        // "123.456" = 7 bytes; buffer of 4 allows 3 chars + NUL per chunk
        let mut buffer = vec![0u8; 4];
        let mut str_len: sql::Len = 0;
        let mut offset: Option<usize> = None;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);

        let warnings = df
            .write_odbc_type((123456, -3), &binding, &mut offset)
            .unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
        assert_eq!(&buffer[..3], b"123");
        assert_eq!(buffer[3], 0);
        assert_eq!(str_len, 7);
        assert_eq!(offset, Some(3));

        // Second chunk
        buffer.fill(0);
        str_len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = df
            .write_odbc_type((123456, -3), &binding, &mut offset)
            .unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
        assert_eq!(&buffer[..3], b".45");
        assert_eq!(str_len, 4); // remaining bytes
        assert_eq!(offset, Some(6));

        // Third chunk — final byte
        buffer.fill(0);
        str_len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = df
            .write_odbc_type((123456, -3), &binding, &mut offset)
            .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(buffer[0], b'6');
        assert_eq!(buffer[1], 0);
        assert_eq!(str_len, 1);
        assert_eq!(offset, None);
    }

    // ======================================================================
    // WriteODBCType — SQL_C_WCHAR target type
    // ======================================================================

    #[test]
    fn write_wchar_produces_string() {
        let df = decfloat();
        let mut buffer = vec![0u16; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);

        let warnings = df
            .write_odbc_type((123456, -3), &binding, &mut None)
            .unwrap();

        assert!(warnings.is_empty());
        let expected: Vec<u16> = "123.456".encode_utf16().collect();
        assert_eq!(str_len, (expected.len() * 2) as sql::Len);
        assert_eq!(&buffer[..expected.len()], &expected[..]);
        assert_eq!(buffer[expected.len()], 0);
    }

    #[test]
    fn write_wchar_negative_scientific() {
        let df = decfloat();
        let mut buffer = vec![0u16; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);

        let warnings = df
            .write_odbc_type((-1234, 7997), &binding, &mut None)
            .unwrap();

        assert!(warnings.is_empty());
        let expected: Vec<u16> = "-1.234e8000".encode_utf16().collect();
        assert_eq!(str_len, (expected.len() * 2) as sql::Len);
        assert_eq!(&buffer[..expected.len()], &expected[..]);
    }

    #[test]
    fn write_wchar_zero() {
        let df = decfloat();
        let mut buffer = vec![0u16; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);

        let warnings = df.write_odbc_type((0, 0), &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        let expected: Vec<u16> = "0".encode_utf16().collect();
        assert_eq!(str_len, (expected.len() * 2) as sql::Len);
        assert_eq!(buffer[0], '0' as u16);
        assert_eq!(buffer[1], 0);
    }

    #[test]
    fn write_wchar_truncation_returns_warning() {
        let df = decfloat();
        let mut buffer = vec![0u16; 4];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);

        let warnings = df
            .write_odbc_type((123456, -3), &binding, &mut None)
            .unwrap();

        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
    }

    #[test]
    fn write_wchar_chunked_reading() {
        let df = decfloat();
        // "123.456" = 7 wide chars; buffer of 4 u16 allows 3 chars + NUL per chunk
        let mut buffer = vec![0u16; 4];
        let mut str_len: sql::Len = 0;
        let mut offset: Option<usize> = None;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);

        let warnings = df
            .write_odbc_type((123456, -3), &binding, &mut offset)
            .unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
        let first_chunk: Vec<u16> = "123".encode_utf16().collect();
        assert_eq!(&buffer[..3], &first_chunk[..]);
        assert!(offset.is_some());

        // Second chunk
        buffer.fill(0);
        str_len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = df
            .write_odbc_type((123456, -3), &binding, &mut offset)
            .unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );

        // Final chunk — should complete
        buffer.fill(0);
        str_len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = df
            .write_odbc_type((123456, -3), &binding, &mut offset)
            .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(offset, None);
    }

    // ======================================================================
    // ReadArrowType — round-trip via StructArray
    // ======================================================================

    #[test]
    fn read_arrow_type_basic() {
        use crate::conversion::ReadArrowType;
        use arrow::array::{BinaryArray, Int16Array, StructArray};
        use arrow::datatypes::{DataType, Field};
        use std::sync::Arc;

        let exponent_array = Int16Array::from(vec![Some(-3i16)]);
        let sig_bytes = 123456i128.to_be_bytes();
        let significand_bytes: Vec<Option<&[u8]>> = vec![Some(&sig_bytes[16 - 4..])];
        let significand_array = BinaryArray::from(significand_bytes);

        let fields = vec![
            Field::new("exponent", DataType::Int16, false),
            Field::new("significand", DataType::Binary, false),
        ];
        let struct_array = StructArray::try_new(
            fields.into(),
            vec![Arc::new(exponent_array), Arc::new(significand_array)],
            None,
        )
        .unwrap();

        let df = decfloat();
        let (sig, exp) = df.read_arrow_type(&struct_array, 0).unwrap();
        assert_eq!(sig, 123456);
        assert_eq!(exp, -3);
    }

    #[test]
    fn read_arrow_type_null_row() {
        use crate::conversion::ReadArrowType;
        use arrow::array::{BinaryArray, Int16Array, StructArray};
        use arrow::buffer::NullBuffer;
        use arrow::datatypes::{DataType, Field};
        use std::sync::Arc;

        let exponent_array = Int16Array::from(vec![Some(0i16)]);
        let significand_bytes: Vec<Option<&[u8]>> = vec![Some(&[0x00])];
        let significand_array = BinaryArray::from(significand_bytes);

        let fields = vec![
            Field::new("exponent", DataType::Int16, true),
            Field::new("significand", DataType::Binary, true),
        ];
        let null_buffer = NullBuffer::from(vec![false]);
        let struct_array = StructArray::try_new(
            fields.into(),
            vec![Arc::new(exponent_array), Arc::new(significand_array)],
            Some(null_buffer),
        )
        .unwrap();

        let df = decfloat();
        let result = df.read_arrow_type(&struct_array, 0);
        assert!(result.is_err());
    }

    #[test]
    fn read_arrow_type_negative_significand() {
        use crate::conversion::ReadArrowType;
        use arrow::array::{BinaryArray, Int16Array, StructArray};
        use arrow::datatypes::{DataType, Field};
        use std::sync::Arc;

        let exponent_array = Int16Array::from(vec![Some(0i16)]);
        let neg_val: i128 = -42;
        let be_bytes = neg_val.to_be_bytes();
        // Trim leading 0xFF bytes but keep at least the sign byte
        let start = be_bytes
            .iter()
            .enumerate()
            .position(|(i, &b)| {
                if i == 15 {
                    return true;
                }
                b != 0xFF || be_bytes[i + 1] & 0x80 == 0
            })
            .unwrap_or(0);
        let trimmed = &be_bytes[start..];

        let significand_bytes: Vec<Option<&[u8]>> = vec![Some(trimmed)];
        let significand_array = BinaryArray::from(significand_bytes);

        let fields = vec![
            Field::new("exponent", DataType::Int16, false),
            Field::new("significand", DataType::Binary, false),
        ];
        let struct_array = StructArray::try_new(
            fields.into(),
            vec![Arc::new(exponent_array), Arc::new(significand_array)],
            None,
        )
        .unwrap();

        let df = decfloat();
        let (sig, exp) = df.read_arrow_type(&struct_array, 0).unwrap();
        assert_eq!(sig, -42);
        assert_eq!(exp, 0);
    }

    // ======================================================================
    // Overflow safety — SQL_C_NUMERIC (checked pow/mul)
    // ======================================================================

    #[test]
    fn write_numeric_large_positive_exponent_overflows_returns_error() {
        let df = decfloat();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Numeric, &mut value, &mut str_len);
        // sig=1, exp=100 → adjusted_exp=100 → 10^100 overflows u128
        let result = df.write_odbc_type((1, 100), &binding, &mut None);
        assert!(matches!(
            result,
            Err(WriteOdbcError::NumericValueOutOfRange { .. })
        ));
    }

    #[test]
    fn write_numeric_multiplication_overflow_returns_error() {
        let df = decfloat();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Numeric, &mut value, &mut str_len);
        // 10^38 * 10^1 overflows u128 (u128 max ~ 3.4e38)
        let sig: i128 = 10i128.pow(37);
        let result = df.write_odbc_type((sig, 2), &binding, &mut None);
        assert!(matches!(
            result,
            Err(WriteOdbcError::NumericValueOutOfRange { .. })
        ));
    }

    #[test]
    fn write_numeric_large_negative_adjusted_exp_truncates_to_zero() {
        let df = decfloat();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Numeric, &mut value, &mut str_len);
        // sig=42, exp=-100 → adjusted_exp=-100 → 10^100 overflows u128 → (0, true)
        let warnings = df.write_odbc_type((42, -100), &binding, &mut None).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::NumericValueTruncated))
        );
        assert!(value.val.iter().all(|&b| b == 0));
    }

    #[test]
    fn write_numeric_small_positive_exponent_succeeds() {
        let df = decfloat();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_fixed(CDataType::Numeric, &mut value, &mut str_len);
        // sig=5, exp=2 → adjusted_exp=2 → 5 * 100 = 500, fits fine
        let warnings = df.write_odbc_type((5, 2), &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.sign, 1);
        let stored = u128::from_le_bytes(value.val);
        assert_eq!(stored, 500);
    }

    // ======================================================================
    // Overflow safety — SQL_C_BINARY (clamped value detection)
    // ======================================================================

    #[test]
    fn write_binary_extreme_positive_exponent_returns_error() {
        let df = decfloat();
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = Binding {
            target_type: CDataType::Binary,
            target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
            buffer_length: buffer.len() as sql::Len,
            octet_length_ptr: &mut str_len as *mut sql::Len,
            indicator_ptr: &mut str_len as *mut sql::Len,
            ..Default::default()
        };
        // sig=1, exp=100 → 10^100 overflows i128 → should return 22003
        let result = df.write_odbc_type((1, 100), &binding, &mut None);
        assert!(matches!(
            result,
            Err(WriteOdbcError::NumericValueOutOfRange { .. })
        ));
    }

    #[test]
    fn write_binary_negative_extreme_exponent_returns_error() {
        let df = decfloat();
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = Binding {
            target_type: CDataType::Binary,
            target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
            buffer_length: buffer.len() as sql::Len,
            octet_length_ptr: &mut str_len as *mut sql::Len,
            indicator_ptr: &mut str_len as *mut sql::Len,
            ..Default::default()
        };
        // sig=-1, exp=100 → overflows → should return 22003
        let result = df.write_odbc_type((-1, 100), &binding, &mut None);
        assert!(matches!(
            result,
            Err(WriteOdbcError::NumericValueOutOfRange { .. })
        ));
    }

    #[test]
    fn write_binary_large_sig_with_exponent_returns_error() {
        let df = decfloat();
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = Binding {
            target_type: CDataType::Binary,
            target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
            buffer_length: buffer.len() as sql::Len,
            octet_length_ptr: &mut str_len as *mut sql::Len,
            indicator_ptr: &mut str_len as *mut sql::Len,
            ..Default::default()
        };
        // sig * 10^exp overflows i128 via checked_mul
        let sig: i128 = i128::MAX / 5;
        let result = df.write_odbc_type((sig, 1), &binding, &mut None);
        assert!(matches!(
            result,
            Err(WriteOdbcError::NumericValueOutOfRange { .. })
        ));
    }

    #[test]
    fn write_binary_max_representable_value_succeeds() {
        let df = decfloat();
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = Binding {
            target_type: CDataType::Binary,
            target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
            buffer_length: buffer.len() as sql::Len,
            octet_length_ptr: &mut str_len as *mut sql::Len,
            indicator_ptr: &mut str_len as *mut sql::Len,
            ..Default::default()
        };
        // Large value that fits in i128: sig=99, exp=0
        let warnings = df.write_odbc_type((99, 0), &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        let numeric: &sql::Numeric = unsafe { &*(buffer.as_ptr() as *const sql::Numeric) };
        assert_eq!(numeric.sign, 1);
        assert_eq!(numeric.val[0], 99);
    }
}
