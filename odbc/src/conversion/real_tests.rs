#[cfg(test)]
mod tests {
    use crate::cdata_types::CDataType;
    use crate::conversion::WriteODBCType;
    use crate::conversion::real::SnowflakeReal;
    use crate::conversion::traits::Binding;
    use crate::conversion::warning::Warning;
    use odbc_sys as sql;

    fn binding_for_value<T>(
        target_type: CDataType,
        value: &mut T,
        str_len: &mut sql::Len,
    ) -> Binding {
        Binding {
            target_type,
            target_value_ptr: value as *mut T as sql::Pointer,
            buffer_length: 0,
            octet_length_ptr: str_len as *mut sql::Len,
            indicator_ptr: str_len as *mut sql::Len,
            ..Default::default()
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

    fn binding_for_wchar_buffer(
        target_type: CDataType,
        buffer: &mut [u16],
        str_len: &mut sql::Len,
    ) -> Binding {
        Binding {
            target_type,
            target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
            buffer_length: (buffer.len() * 2) as sql::Len,
            str_len_or_ind_ptr: str_len as *mut sql::Len,
            precision: None,
            scale: None,
        }
    }

    fn make_real() -> SnowflakeReal {
        SnowflakeReal
    }

    // ======================================================================
    // Default (SQL_C_DOUBLE) tests
    // ======================================================================

    #[test]
    fn real_default_writes_positive_f64() {
        let sr = make_real();
        let mut value: f64 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Default, &mut value, &mut str_len);

        sr.write_odbc_type(3.125, &binding, &mut None).unwrap();

        assert!((value - 3.125).abs() < f64::EPSILON);
    }

    #[test]
    fn real_default_writes_negative_f64() {
        let sr = make_real();
        let mut value: f64 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Default, &mut value, &mut str_len);

        sr.write_odbc_type(-99.5, &binding, &mut None).unwrap();

        assert!((value - (-99.5)).abs() < f64::EPSILON);
    }

    #[test]
    fn real_default_writes_zero() {
        let sr = make_real();
        let mut value: f64 = 1.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Default, &mut value, &mut str_len);

        sr.write_odbc_type(0.0, &binding, &mut None).unwrap();

        assert!((value - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn real_default_writes_very_small_value() {
        let sr = make_real();
        let mut value: f64 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Default, &mut value, &mut str_len);

        let input = 1.23e-10;
        sr.write_odbc_type(input, &binding, &mut None).unwrap();

        assert!((value - input).abs() < f64::EPSILON);
    }

    #[test]
    fn real_default_writes_very_large_value() {
        let sr = make_real();
        let mut value: f64 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Default, &mut value, &mut str_len);

        let input = 1.23e+100;
        sr.write_odbc_type(input, &binding, &mut None).unwrap();

        assert!((value - input).abs() < f64::EPSILON);
    }

    // ======================================================================
    // Explicit C types — basic
    // ======================================================================

    #[test]
    fn real_explicit_float_writes_f32() {
        let sr = make_real();
        let mut value: f32 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Float, &mut value, &mut str_len);

        sr.write_odbc_type(3.125, &binding, &mut None).unwrap();

        assert!((value - 3.125f32).abs() < f32::EPSILON);
    }

    #[test]
    fn real_explicit_slong_writes_i32() {
        let sr = make_real();
        let mut value: i32 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::SLong, &mut value, &mut str_len);

        let warnings = sr.write_odbc_type(42.7, &binding, &mut None).unwrap();

        assert_eq!(value, 42);
        assert!(warnings.contains(&Warning::NumericValueTruncated));
    }

    #[test]
    fn real_explicit_sbigint_writes_i64() {
        let sr = make_real();
        let mut value: i64 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::SBigInt, &mut value, &mut str_len);

        let warnings = sr
            .write_odbc_type(123456789.9, &binding, &mut None)
            .unwrap();

        assert_eq!(value, 123456789i64);
        assert!(warnings.contains(&Warning::NumericValueTruncated));
    }

    #[test]
    fn real_explicit_sshort_writes_i16() {
        let sr = make_real();
        let mut value: i16 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::SShort, &mut value, &mut str_len);

        let warnings = sr.write_odbc_type(100.9, &binding, &mut None).unwrap();

        assert_eq!(value, 100);
        assert!(warnings.contains(&Warning::NumericValueTruncated));
    }

    #[test]
    fn real_explicit_stinyint_writes_i8() {
        let sr = make_real();
        let mut value: i8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::STinyInt, &mut value, &mut str_len);

        let warnings = sr.write_odbc_type(42.9, &binding, &mut None).unwrap();

        assert_eq!(value, 42);
        assert!(warnings.contains(&Warning::NumericValueTruncated));
    }

    #[test]
    fn real_explicit_float_overflow_positive() {
        let sr = make_real();
        let mut value: f32 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Float, &mut value, &mut str_len);

        let result = sr.write_odbc_type(1e300, &binding, &mut None);
        assert!(result.is_err());
    }

    #[test]
    fn real_explicit_float_overflow_negative() {
        let sr = make_real();
        let mut value: f32 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Float, &mut value, &mut str_len);

        let result = sr.write_odbc_type(-1e300, &binding, &mut None);
        assert!(result.is_err());
    }

    #[test]
    fn real_explicit_float_max_succeeds() {
        let sr = make_real();
        let mut value: f32 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Float, &mut value, &mut str_len);

        let warnings = sr
            .write_odbc_type(f32::MAX as f64, &binding, &mut None)
            .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, f32::MAX);
    }

    #[test]
    fn real_explicit_float_just_above_max_fails() {
        let sr = make_real();
        let mut value: f32 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Float, &mut value, &mut str_len);

        // f32::MAX as f64 is 3.4028234663852886e+38; a value just above it
        // is outside the f32 range per the ODBC spec even though IEEE 754
        // rounding would round it back to FLT_MAX.
        let just_above = (f32::MAX as f64) * (1.0 + f64::EPSILON);
        assert!(sr.write_odbc_type(just_above, &binding, &mut None).is_err());
    }

    // ======================================================================
    // Exact integer values — no truncation warning
    // ======================================================================

    #[test]
    fn real_exact_integer_no_warning() {
        let sr = make_real();
        let mut value: i32 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::SLong, &mut value, &mut str_len);

        let warnings = sr.write_odbc_type(42.0, &binding, &mut None).unwrap();

        assert_eq!(value, 42);
        assert!(warnings.is_empty());
    }

    // ======================================================================
    // SQL_C_BIT — full spec compliance
    // ======================================================================

    #[test]
    fn real_explicit_bit_zero_succeeds() {
        let sr = make_real();
        let mut value: u8 = 99;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        let warnings = sr.write_odbc_type(0.0, &binding, &mut None).unwrap();

        assert_eq!(value, 0);
        assert!(warnings.is_empty());
    }

    #[test]
    fn real_explicit_bit_one_succeeds() {
        let sr = make_real();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        let warnings = sr.write_odbc_type(1.0, &binding, &mut None).unwrap();

        assert_eq!(value, 1);
        assert!(warnings.is_empty());
    }

    #[test]
    fn real_explicit_bit_fractional_truncates() {
        let sr = make_real();
        let mut value: u8 = 99;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        let warnings = sr.write_odbc_type(0.5, &binding, &mut None).unwrap();

        assert_eq!(value, 0);
        assert!(warnings.contains(&Warning::NumericValueTruncated));
    }

    #[test]
    fn real_explicit_bit_above_range_errors() {
        let sr = make_real();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        let result = sr.write_odbc_type(5.5, &binding, &mut None);
        assert!(result.is_err());
    }

    #[test]
    fn real_explicit_bit_negative_errors() {
        let sr = make_real();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        let result = sr.write_odbc_type(-1.5, &binding, &mut None);
        assert!(result.is_err());
    }

    // ======================================================================
    // Overflow errors (22003)
    // ======================================================================

    #[test]
    fn real_overflow_stinyint() {
        let sr = make_real();
        let mut value: i8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::STinyInt, &mut value, &mut str_len);

        assert!(sr.write_odbc_type(128.0, &binding, &mut None).is_err());
        assert!(sr.write_odbc_type(-129.0, &binding, &mut None).is_err());
    }

    #[test]
    fn real_overflow_utinyint_negative() {
        let sr = make_real();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::UTinyInt, &mut value, &mut str_len);

        assert!(sr.write_odbc_type(-1.0, &binding, &mut None).is_err());
        assert!(sr.write_odbc_type(256.0, &binding, &mut None).is_err());
    }

    #[test]
    fn real_overflow_sshort() {
        let sr = make_real();
        let mut value: i16 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::SShort, &mut value, &mut str_len);

        assert!(sr.write_odbc_type(32768.0, &binding, &mut None).is_err());
    }

    #[test]
    fn real_overflow_ulong_negative() {
        let sr = make_real();
        let mut value: u32 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::ULong, &mut value, &mut str_len);

        assert!(sr.write_odbc_type(-1.0, &binding, &mut None).is_err());
    }

    // ======================================================================
    // SQL_C_CHAR
    // ======================================================================

    #[test]
    fn real_explicit_char_writes_string() {
        let sr = make_real();
        let mut buffer = vec![0u8; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);

        sr.write_odbc_type(3.125, &binding, &mut None).unwrap();

        let expected = b"3.125";
        assert_eq!(str_len, expected.len() as sql::Len);
        assert_eq!(&buffer[..expected.len()], expected);
        assert_eq!(buffer[expected.len()], 0);
    }

    #[test]
    fn real_explicit_char_writes_negative_value() {
        let sr = make_real();
        let mut buffer = vec![0u8; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);

        sr.write_odbc_type(-99.5, &binding, &mut None).unwrap();

        let expected = b"-99.5";
        assert_eq!(str_len, expected.len() as sql::Len);
        assert_eq!(&buffer[..expected.len()], expected);
        assert_eq!(buffer[expected.len()], 0);
    }

    #[test]
    fn real_explicit_char_writes_integer_value() {
        let sr = make_real();
        let mut buffer = vec![0u8; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);

        sr.write_odbc_type(42.0, &binding, &mut None).unwrap();

        let expected = b"42";
        assert_eq!(str_len, expected.len() as sql::Len);
        assert_eq!(&buffer[..expected.len()], expected);
        assert_eq!(buffer[expected.len()], 0);
    }

    // ======================================================================
    // SQL_C_WCHAR
    // ======================================================================

    #[test]
    fn real_explicit_wchar_writes_string() {
        let sr = make_real();
        let mut buffer = vec![0u16; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(CDataType::WChar, &mut buffer, &mut str_len);

        sr.write_odbc_type(3.125, &binding, &mut None).unwrap();

        let expected: Vec<u16> = "3.125".encode_utf16().collect();
        assert_eq!(
            str_len,
            (expected.len() * std::mem::size_of::<u16>()) as sql::Len
        );
        assert_eq!(&buffer[..expected.len()], &expected[..]);
    }

    // ======================================================================
    // SQL_C_NUMERIC
    // ======================================================================

    fn binding_for_numeric(
        value: &mut sql::Numeric,
        str_len: &mut sql::Len,
        precision: Option<i16>,
        scale: Option<i16>,
    ) -> Binding {
        Binding {
            target_type: CDataType::Numeric,
            target_value_ptr: value as *mut sql::Numeric as sql::Pointer,
            buffer_length: 0,
            str_len_or_ind_ptr: str_len as *mut sql::Len,
            precision,
            scale,
        }
    }

    #[test]
    fn real_numeric_positive_integer() {
        let sr = make_real();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_numeric(&mut value, &mut str_len, None, None);

        let warnings = sr.write_odbc_type(42.0, &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        assert_eq!(value.sign, 1);
        assert_eq!(value.val[0], 42);
        for i in 1..16 {
            assert_eq!(value.val[i], 0);
        }
    }

    #[test]
    fn real_numeric_negative_integer() {
        let sr = make_real();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_numeric(&mut value, &mut str_len, None, None);

        let warnings = sr.write_odbc_type(-7.0, &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        assert_eq!(value.sign, 0);
        assert_eq!(value.val[0], 7);
        for i in 1..16 {
            assert_eq!(value.val[i], 0);
        }
    }

    #[test]
    fn real_numeric_zero() {
        let sr = make_real();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_numeric(&mut value, &mut str_len, None, None);

        let warnings = sr.write_odbc_type(0.0, &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        assert_eq!(value.sign, 1);
        for i in 0..16 {
            assert_eq!(value.val[i], 0);
        }
    }

    #[test]
    fn real_numeric_fractional_truncates_with_scale_zero() {
        let sr = make_real();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_numeric(&mut value, &mut str_len, None, None);

        let warnings = sr.write_odbc_type(123.456, &binding, &mut None).unwrap();

        assert!(warnings.contains(&Warning::NumericValueTruncated));
        assert_eq!(value.sign, 1);
        assert_eq!(value.val[0], 123);
        for i in 1..16 {
            assert_eq!(value.val[i], 0);
        }
    }

    #[test]
    fn real_numeric_with_scale() {
        let sr = make_real();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_numeric(&mut value, &mut str_len, Some(10), Some(2));

        let warnings = sr.write_odbc_type(12.5, &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        assert_eq!(value.sign, 1);
        assert_eq!(value.scale, 2);
        let stored = u128::from_le_bytes(value.val);
        assert_eq!(stored, 1250);
    }

    #[test]
    fn real_numeric_large_value() {
        let sr = make_real();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_numeric(&mut value, &mut str_len, None, None);

        let warnings = sr.write_odbc_type(1000000.0, &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        assert_eq!(value.sign, 1);
        let stored = u128::from_le_bytes(value.val);
        assert_eq!(stored, 1000000);
    }

    #[test]
    fn real_numeric_overflow_returns_error() {
        let sr = make_real();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_numeric(&mut value, &mut str_len, None, None);

        let result = sr.write_odbc_type(1e300, &binding, &mut None);
        assert!(result.is_err());
    }

    // ======================================================================
    // Unsupported type
    // ======================================================================

    #[test]
    fn real_unsupported_type_returns_error() {
        let sr = make_real();
        let mut value: i32 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Binary, &mut value, &mut str_len);

        let result = sr.write_odbc_type(1.0, &binding, &mut None);

        assert!(result.is_err());
    }
}
