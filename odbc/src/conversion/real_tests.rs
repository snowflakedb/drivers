#[cfg(test)]
mod tests {
    use crate::cdata_types::CDataType;
    use crate::conversion::WriteODBCType;
    use crate::conversion::real::SnowflakeReal;
    use crate::conversion::traits::Binding;
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

    fn make_real() -> SnowflakeReal {
        SnowflakeReal
    }

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
    // Explicit C types
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

        sr.write_odbc_type(42.7, &binding, &mut None).unwrap();

        assert_eq!(value, 42);
    }

    #[test]
    fn real_explicit_sbigint_writes_i64() {
        let sr = make_real();
        let mut value: i64 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::SBigInt, &mut value, &mut str_len);

        sr.write_odbc_type(123456789.9, &binding, &mut None)
            .unwrap();

        assert_eq!(value, 123456789i64);
    }

    #[test]
    fn real_explicit_sshort_writes_i16() {
        let sr = make_real();
        let mut value: i16 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::SShort, &mut value, &mut str_len);

        sr.write_odbc_type(100.9, &binding, &mut None).unwrap();

        assert_eq!(value, 100);
    }

    #[test]
    fn real_explicit_stinyint_writes_i8() {
        let sr = make_real();
        let mut value: i8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::STinyInt, &mut value, &mut str_len);

        sr.write_odbc_type(42.9, &binding, &mut None).unwrap();

        assert_eq!(value, 42);
    }

    #[test]
    fn real_explicit_bit_writes_one_for_nonzero() {
        let sr = make_real();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        sr.write_odbc_type(5.5, &binding, &mut None).unwrap();

        assert_eq!(value, 1);
    }

    #[test]
    fn real_explicit_bit_writes_zero_for_zero() {
        let sr = make_real();
        let mut value: u8 = 1;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        sr.write_odbc_type(0.0, &binding, &mut None).unwrap();

        assert_eq!(value, 0);
    }

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
