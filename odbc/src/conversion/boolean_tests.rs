#[cfg(test)]
mod tests {
    use crate::api::CDataType;
    use crate::conversion::WriteODBCType;
    use crate::conversion::boolean::SnowflakeBoolean;
    use crate::conversion::test_utils::helpers::{
        binding_for_char_buffer, binding_for_interval_with_precision, binding_for_value,
        binding_for_wchar_buffer, zero_interval,
    };
    use odbc_sys as sql;

    // ========================================================================
    // ReadArrowType — reading from BooleanArray
    // ========================================================================

    #[test]
    fn read_arrow_true() {
        use crate::conversion::ReadArrowType;
        use arrow::array::BooleanArray;
        let sn = SnowflakeBoolean;
        let array = BooleanArray::from(vec![Some(true)]);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert!(value);
    }

    #[test]
    fn read_arrow_false() {
        use crate::conversion::ReadArrowType;
        use arrow::array::BooleanArray;
        let sn = SnowflakeBoolean;
        let array = BooleanArray::from(vec![Some(false)]);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert!(!value);
    }

    #[test]
    fn read_arrow_null_returns_error() {
        use crate::conversion::ReadArrowType;
        use crate::conversion::error::ReadArrowError;
        use arrow::array::BooleanArray;
        let sn = SnowflakeBoolean;
        let array = BooleanArray::from(vec![None::<bool>]);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(result, Err(ReadArrowError::NullValue { .. })));
    }

    // ========================================================================
    // SQL_C_BIT (default)
    // ========================================================================

    #[test]
    fn bit_true() {
        let sn = SnowflakeBoolean;
        let mut value: u8 = 0xFF;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, 1);
        assert_eq!(str_len, size_of::<u8>() as sql::Len);
    }

    #[test]
    fn bit_false() {
        let sn = SnowflakeBoolean;
        let mut value: u8 = 0xFF;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(false, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, 0);
        assert_eq!(str_len, size_of::<u8>() as sql::Len);
    }

    #[test]
    fn default_true_maps_to_bit() {
        let sn = SnowflakeBoolean;
        let mut value: u8 = 0xFF;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Default, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, 1);
        assert_eq!(str_len, size_of::<u8>() as sql::Len);
    }

    #[test]
    fn default_false_maps_to_bit() {
        let sn = SnowflakeBoolean;
        let mut value: u8 = 0xFF;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Default, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(false, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, 0);
        assert_eq!(str_len, size_of::<u8>() as sql::Len);
    }

    // ========================================================================
    // Integer type conversions
    // ========================================================================

    macro_rules! integer_tests {
        ($($name_true:ident, $name_false:ident: $c_type:expr, $rust_type:ty;)*) => {
            $(
                #[test]
                fn $name_true() {
                    let sn = SnowflakeBoolean;
                    let mut value: $rust_type = 99 as $rust_type;
                    let mut str_len: sql::Len = 0;
                    let binding = binding_for_value($c_type, &mut value, &mut str_len);
                    let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
                    assert!(warnings.is_empty());
                    assert_eq!(value, 1 as $rust_type);
                    assert_eq!(str_len, std::mem::size_of::<$rust_type>() as sql::Len);
                }

                #[test]
                fn $name_false() {
                    let sn = SnowflakeBoolean;
                    let mut value: $rust_type = 99 as $rust_type;
                    let mut str_len: sql::Len = 0;
                    let binding = binding_for_value($c_type, &mut value, &mut str_len);
                    let warnings = sn.write_odbc_type(false, &binding, &mut None).unwrap();
                    assert!(warnings.is_empty());
                    assert_eq!(value, 0 as $rust_type);
                }
            )*
        };
    }

    integer_tests! {
        slong_true,    slong_false:    CDataType::SLong,    i32;
        long_true,     long_false:     CDataType::Long,     i32;
        ulong_true,    ulong_false:    CDataType::ULong,    u32;
        sshort_true,   sshort_false:   CDataType::SShort,   i16;
        short_true,    short_false:    CDataType::Short,     i16;
        ushort_true,   ushort_false:   CDataType::UShort,   u16;
        stinyint_true, stinyint_false: CDataType::STinyInt, i8;
        tinyint_true,  tinyint_false:  CDataType::TinyInt,  i8;
        utinyint_true, utinyint_false: CDataType::UTinyInt, u8;
        sbigint_true,  sbigint_false:  CDataType::SBigInt,  i64;
        ubigint_true,  ubigint_false:  CDataType::UBigInt,  u64;
    }

    // ========================================================================
    // Float / Double conversions
    // ========================================================================

    #[test]
    fn double_true() {
        let sn = SnowflakeBoolean;
        let mut value: f64 = -1.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Double, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert!((value - 1.0).abs() < f64::EPSILON);
        assert_eq!(str_len, size_of::<f64>() as sql::Len);
    }

    #[test]
    fn double_false() {
        let sn = SnowflakeBoolean;
        let mut value: f64 = -1.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Double, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(false, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert!(value.abs() < f64::EPSILON);
        assert_eq!(str_len, size_of::<f64>() as sql::Len);
    }

    #[test]
    fn float_true() {
        let sn = SnowflakeBoolean;
        let mut value: f32 = -1.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Float, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert!((value - 1.0).abs() < f32::EPSILON);
        assert_eq!(str_len, size_of::<f32>() as sql::Len);
    }

    #[test]
    fn float_false() {
        let sn = SnowflakeBoolean;
        let mut value: f32 = -1.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Float, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(false, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert!(value.abs() < f32::EPSILON);
        assert_eq!(str_len, size_of::<f32>() as sql::Len);
    }

    // ========================================================================
    // SQL_C_CHAR conversions — "0" / "1"
    // ========================================================================

    #[test]
    fn char_true() {
        let sn = SnowflakeBoolean;
        let mut buffer = vec![0u8; 16];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 1);
        assert_eq!(buffer[0], b'1');
        assert_eq!(buffer[1], 0);
    }

    #[test]
    fn char_false() {
        let sn = SnowflakeBoolean;
        let mut buffer = vec![0u8; 16];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(false, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 1);
        assert_eq!(buffer[0], b'0');
        assert_eq!(buffer[1], 0);
    }

    #[test]
    fn char_buffer_size_2_fits_exactly() {
        let sn = SnowflakeBoolean;
        let mut buffer = vec![0u8; 2];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 1);
        assert_eq!(buffer[0], b'1');
        assert_eq!(buffer[1], 0);
    }

    #[test]
    fn char_buffer_size_1_truncates() {
        use crate::conversion::warning::Warning;
        let sn = SnowflakeBoolean;
        let mut buffer = vec![0u8; 1];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
    }

    // ========================================================================
    // SQL_C_WCHAR conversions — u"0" / u"1"
    // ========================================================================

    #[test]
    fn wchar_true() {
        let sn = SnowflakeBoolean;
        let mut buffer = vec![0u16; 16];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 2);
        assert_eq!(buffer[0], '1' as u16);
        assert_eq!(buffer[1], 0);
    }

    #[test]
    fn wchar_false() {
        let sn = SnowflakeBoolean;
        let mut buffer = vec![0u16; 16];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(false, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 2);
        assert_eq!(buffer[0], '0' as u16);
        assert_eq!(buffer[1], 0);
    }

    #[test]
    fn wchar_buffer_too_small_truncates() {
        use crate::conversion::warning::Warning;
        let sn = SnowflakeBoolean;
        let mut buffer = vec![0u16; 1];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
    }

    // ========================================================================
    // SQL_C_NUMERIC
    // ========================================================================

    #[test]
    fn numeric_true() {
        let sn = SnowflakeBoolean;
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0u8; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Numeric, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.precision, 1);
        assert_eq!(value.scale, 0);
        assert_eq!(value.sign, 1);
        assert_eq!(u128::from_le_bytes(value.val), 1);
        assert_eq!(str_len, size_of::<sql::Numeric>() as sql::Len);
    }

    #[test]
    fn numeric_false() {
        let sn = SnowflakeBoolean;
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0u8; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Numeric, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(false, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.precision, 1);
        assert_eq!(value.scale, 0);
        assert_eq!(value.sign, 1);
        assert_eq!(u128::from_le_bytes(value.val), 0);
    }

    #[test]
    fn numeric_uses_binding_precision_and_scale() {
        let sn = SnowflakeBoolean;
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0u8; 16],
        };
        let mut str_len: sql::Len = 0;
        let mut binding = binding_for_value(CDataType::Numeric, &mut value, &mut str_len);
        binding.precision = Some(10);
        binding.scale = Some(2);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.precision, 10);
        assert_eq!(value.scale, 2);
        assert_eq!(value.sign, 1);
        assert_eq!(u128::from_le_bytes(value.val), 1);
    }

    // ========================================================================
    // SQL_C_BINARY — raw byte 0x00 / 0x01
    // ========================================================================

    #[test]
    fn binary_true() {
        let sn = SnowflakeBoolean;
        let mut buffer = vec![0xFFu8; 16];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Binary, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 1);
        assert_eq!(buffer[0], 1);
    }

    #[test]
    fn binary_false() {
        let sn = SnowflakeBoolean;
        let mut buffer = vec![0xFFu8; 16];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Binary, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(false, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 1);
        assert_eq!(buffer[0], 0);
    }

    // ========================================================================
    // Interval type conversions
    // ========================================================================

    #[test]
    fn interval_year_true() {
        let sn = SnowflakeBoolean;
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::IntervalYear, &mut interval, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(interval.interval_type, sql::Interval::Year as i32);
        assert_eq!(interval.interval_sign, 0);
        assert_eq!(unsafe { interval.interval_value.year_month.year }, 1);
    }

    #[test]
    fn interval_year_false() {
        let sn = SnowflakeBoolean;
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::IntervalYear, &mut interval, &mut str_len);
        let warnings = sn.write_odbc_type(false, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(interval.interval_type, sql::Interval::Year as i32);
        assert_eq!(interval.interval_sign, 0);
        assert_eq!(unsafe { interval.interval_value.year_month.year }, 0);
    }

    #[test]
    fn interval_month_true() {
        let sn = SnowflakeBoolean;
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::IntervalMonth, &mut interval, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(interval.interval_type, sql::Interval::Month as i32);
        assert_eq!(unsafe { interval.interval_value.year_month.month }, 1);
    }

    #[test]
    fn interval_day_true() {
        let sn = SnowflakeBoolean;
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::IntervalDay, &mut interval, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(interval.interval_type, sql::Interval::Day as i32);
        assert_eq!(unsafe { interval.interval_value.day_second.day }, 1);
    }

    #[test]
    fn interval_hour_true() {
        let sn = SnowflakeBoolean;
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::IntervalHour, &mut interval, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(interval.interval_type, sql::Interval::Hour as i32);
        assert_eq!(unsafe { interval.interval_value.day_second.hour }, 1);
    }

    #[test]
    fn interval_minute_true() {
        let sn = SnowflakeBoolean;
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::IntervalMinute, &mut interval, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(interval.interval_type, sql::Interval::Minute as i32);
        assert_eq!(unsafe { interval.interval_value.day_second.minute }, 1);
    }

    #[test]
    fn interval_second_true() {
        let sn = SnowflakeBoolean;
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::IntervalSecond, &mut interval, &mut str_len);
        let warnings = sn.write_odbc_type(true, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(interval.interval_type, sql::Interval::Second as i32);
        assert_eq!(interval.interval_sign, 0);
        assert_eq!(unsafe { interval.interval_value.day_second.second }, 1);
        assert_eq!(unsafe { interval.interval_value.day_second.fraction }, 0);
    }

    #[test]
    fn interval_second_false() {
        let sn = SnowflakeBoolean;
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::IntervalSecond, &mut interval, &mut str_len);
        let warnings = sn.write_odbc_type(false, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(interval.interval_type, sql::Interval::Second as i32);
        assert_eq!(interval.interval_sign, 0);
        assert_eq!(unsafe { interval.interval_value.day_second.second }, 0);
        assert_eq!(unsafe { interval.interval_value.day_second.fraction }, 0);
    }

    #[test]
    fn interval_second_true_rejected_by_leading_precision_zero() {
        use crate::conversion::error::WriteOdbcError;
        let sn = SnowflakeBoolean;
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval_with_precision(
            CDataType::IntervalSecond,
            &mut interval,
            &mut str_len,
            0,
        );
        let result = sn.write_odbc_type(true, &binding, &mut None);
        assert!(
            matches!(result, Err(WriteOdbcError::IntervalFieldOverflow { .. })),
            "TRUE should overflow with leading precision 0"
        );
    }

    #[test]
    fn interval_second_false_accepted_by_leading_precision_zero() {
        let sn = SnowflakeBoolean;
        let mut interval = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval_with_precision(
            CDataType::IntervalSecond,
            &mut interval,
            &mut str_len,
            0,
        );
        let warnings = sn.write_odbc_type(false, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(unsafe { interval.interval_value.day_second.second }, 0);
    }

    #[test]
    fn interval_multi_field_returns_error() {
        use crate::conversion::error::WriteOdbcError;
        let sn = SnowflakeBoolean;
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
            let binding = binding_for_value(target_type, &mut interval, &mut str_len);
            let result = sn.write_odbc_type(true, &binding, &mut None);
            assert!(
                matches!(result, Err(WriteOdbcError::IntervalFieldOverflow { .. })),
                "Multi-field interval {target_type:?} should fail"
            );
        }
    }

    // ========================================================================
    // Unsupported target type returns error
    // ========================================================================

    #[test]
    fn unsupported_type_returns_error() {
        let sn = SnowflakeBoolean;
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::TypeDate, &mut value, &mut str_len);
        assert!(sn.write_odbc_type(true, &binding, &mut None).is_err());
    }

    // ========================================================================
    // Metadata
    // ========================================================================

    #[test]
    fn sql_type_is_ext_bit() {
        let sn = SnowflakeBoolean;
        assert_eq!(sn.sql_type(), sql::SqlDataType::EXT_BIT);
    }

    #[test]
    fn column_size_is_1() {
        let sn = SnowflakeBoolean;
        assert_eq!(sn.column_size(), 1);
    }

    #[test]
    fn decimal_digits_is_0() {
        let sn = SnowflakeBoolean;
        assert_eq!(sn.decimal_digits(), 0);
    }
}
