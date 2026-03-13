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
            octet_length_ptr: str_len as *mut sql::Len,
            indicator_ptr: str_len as *mut sql::Len,
            ..Default::default()
        }
    }

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
            octet_length_ptr: str_len as *mut sql::Len,
            indicator_ptr: str_len as *mut sql::Len,
            precision,
            scale,
            datetime_interval_precision: None,
        }
    }

    fn binding_for_binary(buffer: &mut [u8], str_len: &mut sql::Len) -> Binding {
        Binding {
            target_type: CDataType::Binary,
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
    // Integer conversions — macro-based comprehensive tests
    // Pattern from number_tests.rs: value, expected, truncation warning
    // ======================================================================

    macro_rules! real_integer_tests {
        ($($name:ident: $c_type:expr, $rust_type:ty, $input:expr => $expected:expr, truncated=$trunc:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    let sr = make_real();
                    let mut value: $rust_type = 0 as $rust_type;
                    let mut str_len: sql::Len = 0;
                    let binding = binding_for_value($c_type, &mut value, &mut str_len);
                    let warnings = sr.write_odbc_type($input, &binding, &mut None).unwrap();
                    assert_eq!(value, $expected as $rust_type,
                        "value mismatch: expected {}, got {}", $expected, value);
                    assert_eq!(!warnings.is_empty(), $trunc,
                        "truncation warning mismatch: expected truncated={}, got warnings={:?}",
                        $trunc, warnings);
                }
            )*
        };
    }

    real_integer_tests! {
        // SQL_C_SLONG / SQL_C_LONG — basic
        slong_positive:             CDataType::SLong,    i32, 42.0    => 42,    truncated=false;
        slong_negative:             CDataType::SLong,    i32, -42.0   => -42,   truncated=false;
        slong_zero:                 CDataType::SLong,    i32, 0.0     => 0,     truncated=false;
        slong_one:                  CDataType::SLong,    i32, 1.0     => 1,     truncated=false;
        slong_neg_one:              CDataType::SLong,    i32, -1.0    => -1,    truncated=false;
        slong_truncates_frac:       CDataType::SLong,    i32, 42.7    => 42,    truncated=true;
        slong_truncates_neg_frac:   CDataType::SLong,    i32, -42.9   => -42,   truncated=true;
        long_positive:              CDataType::Long,     i32, 42.0    => 42,    truncated=false;

        // SQL_C_SLONG boundaries
        slong_i32_max:              CDataType::SLong,    i32, 2_147_483_647.0   => 2_147_483_647i32,  truncated=false;
        slong_i32_min:              CDataType::SLong,    i32, -2_147_483_648.0  => -2_147_483_648i32, truncated=false;

        // SQL_C_ULONG
        ulong_positive:             CDataType::ULong,    u32, 42.0    => 42u32,  truncated=false;
        ulong_zero:                 CDataType::ULong,    u32, 0.0     => 0u32,   truncated=false;
        ulong_truncates_frac:       CDataType::ULong,    u32, 42.9    => 42u32,  truncated=true;

        // SQL_C_SSHORT / SQL_C_SHORT
        sshort_positive:            CDataType::SShort,   i16, 100.0   => 100i16,  truncated=false;
        sshort_negative:            CDataType::SShort,   i16, -100.0  => -100i16, truncated=false;
        sshort_zero:                CDataType::SShort,   i16, 0.0     => 0i16,    truncated=false;
        sshort_truncates_frac:      CDataType::SShort,   i16, 100.9   => 100i16,  truncated=true;
        short_positive:             CDataType::Short,    i16, 300.0   => 300i16,  truncated=false;

        // SQL_C_SSHORT boundaries
        sshort_i16_max:             CDataType::SShort,   i16, 32767.0    => 32767i16,  truncated=false;
        sshort_i16_min:             CDataType::SShort,   i16, -32768.0   => -32768i16, truncated=false;

        // SQL_C_USHORT
        ushort_positive:            CDataType::UShort,   u16, 300.0   => 300u16,   truncated=false;
        ushort_zero:                CDataType::UShort,   u16, 0.0     => 0u16,     truncated=false;
        ushort_truncates_frac:      CDataType::UShort,   u16, 300.9   => 300u16,   truncated=true;
        ushort_u16_max:             CDataType::UShort,   u16, 65535.0 => 65535u16, truncated=false;

        // SQL_C_STINYINT / SQL_C_TINYINT
        stinyint_positive:          CDataType::STinyInt, i8, 42.0     => 42i8,    truncated=false;
        stinyint_negative:          CDataType::STinyInt, i8, -42.0    => -42i8,   truncated=false;
        stinyint_zero:              CDataType::STinyInt, i8, 0.0      => 0i8,     truncated=false;
        stinyint_truncates_frac:    CDataType::STinyInt, i8, 42.9     => 42i8,    truncated=true;
        tinyint_positive:           CDataType::TinyInt,  i8, 42.0     => 42i8,    truncated=false;

        // SQL_C_STINYINT boundaries
        stinyint_i8_max:            CDataType::STinyInt, i8, 127.0    => 127i8,   truncated=false;
        stinyint_i8_min:            CDataType::STinyInt, i8, -128.0   => -128i8,  truncated=false;

        // SQL_C_UTINYINT
        utinyint_positive:          CDataType::UTinyInt, u8, 42.0     => 42u8,    truncated=false;
        utinyint_zero:              CDataType::UTinyInt, u8, 0.0      => 0u8,     truncated=false;
        utinyint_truncates_frac:    CDataType::UTinyInt, u8, 42.9     => 42u8,    truncated=true;
        utinyint_u8_max:            CDataType::UTinyInt, u8, 255.0    => 255u8,   truncated=false;

        // SQL_C_SBIGINT
        sbigint_positive:           CDataType::SBigInt,  i64, 123456789.0     => 123456789i64,   truncated=false;
        sbigint_negative:           CDataType::SBigInt,  i64, -123456789.0    => -123456789i64,  truncated=false;
        sbigint_zero:               CDataType::SBigInt,  i64, 0.0            => 0i64,           truncated=false;
        sbigint_truncates_frac:     CDataType::SBigInt,  i64, 123456789.9    => 123456789i64,   truncated=true;

        // SQL_C_UBIGINT
        ubigint_positive:           CDataType::UBigInt,  u64, 123456789.0     => 123456789u64,   truncated=false;
        ubigint_zero:               CDataType::UBigInt,  u64, 0.0            => 0u64,           truncated=false;
        ubigint_truncates_frac:     CDataType::UBigInt,  u64, 123456789.9    => 123456789u64,   truncated=true;
    }

    // ======================================================================
    // Integer conversions — overflow error cases (SQLSTATE 22003)
    // ======================================================================

    macro_rules! real_overflow_tests {
        ($($name:ident: $c_type:expr, $rust_type:ty, $input:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    let sr = make_real();
                    let mut value: $rust_type = 0 as $rust_type;
                    let mut str_len: sql::Len = 0;
                    let binding = binding_for_value($c_type, &mut value, &mut str_len);
                    assert!(sr.write_odbc_type($input, &binding, &mut None).is_err());
                }
            )*
        };
    }

    real_overflow_tests! {
        // i32 overflow
        slong_overflow_above:           CDataType::SLong,    i32, 2_147_483_648.0;
        slong_overflow_below:           CDataType::SLong,    i32, -2_147_483_649.0;

        // u32 overflow
        ulong_overflow_above:           CDataType::ULong,    u32, 4_294_967_296.0;
        ulong_overflow_negative:        CDataType::ULong,    u32, -1.0;

        // i16 overflow
        sshort_overflow_above:          CDataType::SShort,   i16, 32768.0;
        sshort_overflow_below:          CDataType::SShort,   i16, -32769.0;

        // u16 overflow
        ushort_overflow_above:          CDataType::UShort,   u16, 65536.0;
        ushort_overflow_negative:       CDataType::UShort,   u16, -1.0;

        // i8 overflow
        stinyint_overflow_above:        CDataType::STinyInt, i8,  128.0;
        stinyint_overflow_below:        CDataType::STinyInt, i8,  -129.0;

        // u8 overflow
        utinyint_overflow_above:        CDataType::UTinyInt, u8,  256.0;
        utinyint_overflow_negative:     CDataType::UTinyInt, u8,  -1.0;

        // i64 overflow
        sbigint_overflow_above:         CDataType::SBigInt,  i64, 1e19;
        sbigint_overflow_below:         CDataType::SBigInt,  i64, -1e19;

        // u64 overflow
        ubigint_overflow_above:         CDataType::UBigInt,  u64, 1e20;
        ubigint_overflow_negative:      CDataType::UBigInt,  u64, -1.0;

        // Fractional value above boundary still overflows
        stinyint_frac_overflow:         CDataType::STinyInt, i8,  128.1;
    }

    // ======================================================================
    // SQL_C_FLOAT — explicit tests
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
    fn real_explicit_float_negative() {
        let sr = make_real();
        let mut value: f32 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Float, &mut value, &mut str_len);

        sr.write_odbc_type(-99.5, &binding, &mut None).unwrap();

        assert!((value - (-99.5f32)).abs() < f32::EPSILON);
    }

    #[test]
    fn real_explicit_float_zero() {
        let sr = make_real();
        let mut value: f32 = 1.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Float, &mut value, &mut str_len);

        sr.write_odbc_type(0.0, &binding, &mut None).unwrap();

        assert_eq!(value, 0.0f32);
    }

    #[test]
    fn real_explicit_float_overflow_positive() {
        let sr = make_real();
        let mut value: f32 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Float, &mut value, &mut str_len);

        assert!(sr.write_odbc_type(1e300, &binding, &mut None).is_err());
    }

    #[test]
    fn real_explicit_float_overflow_negative() {
        let sr = make_real();
        let mut value: f32 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Float, &mut value, &mut str_len);

        assert!(sr.write_odbc_type(-1e300, &binding, &mut None).is_err());
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

        let just_above = (f32::MAX as f64) * (1.0 + f64::EPSILON);
        assert!(sr.write_odbc_type(just_above, &binding, &mut None).is_err());
    }

    // ======================================================================
    // SQL_C_BIT — full spec compliance
    // ======================================================================

    #[test]
    fn real_bit_zero_succeeds() {
        let sr = make_real();
        let mut value: u8 = 99;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        let warnings = sr.write_odbc_type(0.0, &binding, &mut None).unwrap();

        assert_eq!(value, 0);
        assert!(warnings.is_empty());
    }

    #[test]
    fn real_bit_one_succeeds() {
        let sr = make_real();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        let warnings = sr.write_odbc_type(1.0, &binding, &mut None).unwrap();

        assert_eq!(value, 1);
        assert!(warnings.is_empty());
    }

    #[test]
    fn real_bit_fractional_truncates_to_zero() {
        let sr = make_real();
        let mut value: u8 = 99;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        let warnings = sr.write_odbc_type(0.5, &binding, &mut None).unwrap();

        assert_eq!(value, 0);
        assert!(warnings.contains(&Warning::NumericValueTruncated));
    }

    #[test]
    fn real_bit_fractional_truncates_to_one() {
        let sr = make_real();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        let warnings = sr.write_odbc_type(1.5, &binding, &mut None).unwrap();

        assert_eq!(value, 1);
        assert!(warnings.contains(&Warning::NumericValueTruncated));
    }

    #[test]
    fn real_bit_above_range_errors() {
        let sr = make_real();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        assert!(sr.write_odbc_type(5.5, &binding, &mut None).is_err());
    }

    #[test]
    fn real_bit_negative_errors() {
        let sr = make_real();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        assert!(sr.write_odbc_type(-1.5, &binding, &mut None).is_err());
    }

    #[test]
    fn real_bit_large_positive_errors() {
        let sr = make_real();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        assert!(sr.write_odbc_type(100.0, &binding, &mut None).is_err());
    }

    // ======================================================================
    // SQL_C_CHAR — string conversion
    // ======================================================================

    #[test]
    fn real_char_positive() {
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
    fn real_char_negative() {
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
    fn real_char_integer_value() {
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
    fn real_char_zero() {
        let sr = make_real();
        let mut buffer = vec![0u8; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);

        sr.write_odbc_type(0.0, &binding, &mut None).unwrap();

        let expected = b"0";
        assert_eq!(str_len, expected.len() as sql::Len);
        assert_eq!(&buffer[..expected.len()], expected);
    }

    #[test]
    fn real_char_fractional_only_truncation_returns_01004() {
        let sr = make_real();
        let mut buffer = vec![0u8; 4]; // "7.98765" → whole digits "7" (1 char), fits in 4-byte buffer
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);

        let warnings = sr.write_odbc_type(7.98765, &binding, &mut None).unwrap();

        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
    }

    #[test]
    fn real_char_whole_digits_lost_returns_22003() {
        let sr = make_real();
        let mut buffer = vec![0u8; 4]; // "123456.789" → whole digits "123456" (6 chars), doesn't fit in 4-byte buffer
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);

        assert!(sr.write_odbc_type(123456.789, &binding, &mut None).is_err());
    }

    // ======================================================================
    // SQL_C_WCHAR — wide string conversion
    // ======================================================================

    #[test]
    fn real_wchar_positive() {
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

    #[test]
    fn real_wchar_negative() {
        let sr = make_real();
        let mut buffer = vec![0u16; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(CDataType::WChar, &mut buffer, &mut str_len);

        sr.write_odbc_type(-99.5, &binding, &mut None).unwrap();

        let expected: Vec<u16> = "-99.5".encode_utf16().collect();
        assert_eq!(
            str_len,
            (expected.len() * std::mem::size_of::<u16>()) as sql::Len
        );
        assert_eq!(&buffer[..expected.len()], &expected[..]);
    }

    #[test]
    fn real_wchar_zero() {
        let sr = make_real();
        let mut buffer = vec![0u16; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(CDataType::WChar, &mut buffer, &mut str_len);

        sr.write_odbc_type(0.0, &binding, &mut None).unwrap();

        let expected: Vec<u16> = "0".encode_utf16().collect();
        assert_eq!(
            str_len,
            (expected.len() * std::mem::size_of::<u16>()) as sql::Len
        );
        assert_eq!(&buffer[..expected.len()], &expected[..]);
    }

    #[test]
    fn real_wchar_integer() {
        let sr = make_real();
        let mut buffer = vec![0u16; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(CDataType::WChar, &mut buffer, &mut str_len);

        sr.write_odbc_type(42.0, &binding, &mut None).unwrap();

        let expected: Vec<u16> = "42".encode_utf16().collect();
        assert_eq!(
            str_len,
            (expected.len() * std::mem::size_of::<u16>()) as sql::Len
        );
        assert_eq!(&buffer[..expected.len()], &expected[..]);
    }

    #[test]
    fn real_wchar_fractional_only_truncation_returns_01004() {
        let sr = make_real();
        let mut buffer = vec![0u16; 4]; // "7.98765" → whole digits "7" (1 char), fits in 4-wchar buffer
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(CDataType::WChar, &mut buffer, &mut str_len);

        let warnings = sr.write_odbc_type(7.98765, &binding, &mut None).unwrap();

        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
    }

    #[test]
    fn real_wchar_whole_digits_lost_returns_22003() {
        let sr = make_real();
        let mut buffer = vec![0u16; 4]; // "123456.789" → whole digits "123456" (6 chars), doesn't fit in 4-wchar buffer
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(CDataType::WChar, &mut buffer, &mut str_len);

        assert!(sr.write_odbc_type(123456.789, &binding, &mut None).is_err());
    }

    // ======================================================================
    // SQL_C_NUMERIC
    // ======================================================================

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
        assert_eq!(u128::from_le_bytes(value.val), 42);
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
        assert_eq!(u128::from_le_bytes(value.val), 7);
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
        assert_eq!(u128::from_le_bytes(value.val), 0);
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
        assert_eq!(u128::from_le_bytes(value.val), 123);
    }

    #[test]
    fn real_numeric_with_explicit_scale() {
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
        assert_eq!(u128::from_le_bytes(value.val), 1250);
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
        assert_eq!(u128::from_le_bytes(value.val), 1000000);
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

        assert!(sr.write_odbc_type(1e300, &binding, &mut None).is_err());
    }

    #[test]
    fn real_numeric_negative_fractional() {
        let sr = make_real();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_numeric(&mut value, &mut str_len, None, None);

        let warnings = sr.write_odbc_type(-99.9, &binding, &mut None).unwrap();

        assert!(warnings.contains(&Warning::NumericValueTruncated));
        assert_eq!(value.sign, 0);
        assert_eq!(u128::from_le_bytes(value.val), 99);
    }

    #[test]
    fn real_numeric_val_255_single_byte() {
        let sr = make_real();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_numeric(&mut value, &mut str_len, None, None);

        sr.write_odbc_type(255.0, &binding, &mut None).unwrap();

        assert_eq!(value.val[0], 255);
        for i in 1..16 {
            assert_eq!(value.val[i], 0);
        }
    }

    #[test]
    fn real_numeric_val_256_spans_two_bytes() {
        let sr = make_real();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_numeric(&mut value, &mut str_len, None, None);

        sr.write_odbc_type(256.0, &binding, &mut None).unwrap();

        assert_eq!(value.val[0], 0);
        assert_eq!(value.val[1], 1);
        for i in 2..16 {
            assert_eq!(value.val[i], 0);
        }
    }

    // ======================================================================
    // SQL_C_BINARY — new conversion (per ODBC matrix)
    // Writes SQL_NUMERIC_STRUCT into the binary buffer, same pattern as NUMBER.
    // ======================================================================

    #[test]
    fn real_binary_positive_integer() {
        let sr = make_real();
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_binary(&mut buffer, &mut str_len);

        let warnings = sr.write_odbc_type(42.0, &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        assert_eq!(str_len, std::mem::size_of::<sql::Numeric>() as sql::Len);

        let numeric: &sql::Numeric = unsafe { &*(buffer.as_ptr() as *const sql::Numeric) };
        assert_eq!(numeric.sign, 1);
        assert_eq!(u128::from_le_bytes(numeric.val), 42);
    }

    #[test]
    fn real_binary_negative_integer() {
        let sr = make_real();
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_binary(&mut buffer, &mut str_len);

        let warnings = sr.write_odbc_type(-7.0, &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        let numeric: &sql::Numeric = unsafe { &*(buffer.as_ptr() as *const sql::Numeric) };
        assert_eq!(numeric.sign, 0);
        assert_eq!(u128::from_le_bytes(numeric.val), 7);
    }

    #[test]
    fn real_binary_zero() {
        let sr = make_real();
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_binary(&mut buffer, &mut str_len);

        let warnings = sr.write_odbc_type(0.0, &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        let numeric: &sql::Numeric = unsafe { &*(buffer.as_ptr() as *const sql::Numeric) };
        assert_eq!(numeric.sign, 1);
        assert_eq!(u128::from_le_bytes(numeric.val), 0);
    }

    #[test]
    fn real_binary_fractional_truncates() {
        let sr = make_real();
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_binary(&mut buffer, &mut str_len);

        let warnings = sr.write_odbc_type(42.9, &binding, &mut None).unwrap();

        assert!(warnings.contains(&Warning::NumericValueTruncated));
        let numeric: &sql::Numeric = unsafe { &*(buffer.as_ptr() as *const sql::Numeric) };
        assert_eq!(numeric.sign, 1);
        assert_eq!(u128::from_le_bytes(numeric.val), 42);
    }

    #[test]
    fn real_binary_buffer_too_small_returns_error() {
        let sr = make_real();
        let mut buffer = vec![0u8; 4]; // too small for SQL_NUMERIC_STRUCT
        let mut str_len: sql::Len = 0;
        let binding = binding_for_binary(&mut buffer, &mut str_len);

        assert!(sr.write_odbc_type(42.0, &binding, &mut None).is_err());
    }

    #[test]
    fn real_binary_exact_size_succeeds() {
        let sr = make_real();
        let numeric_size = std::mem::size_of::<sql::Numeric>();
        let mut buffer = vec![0u8; numeric_size];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_binary(&mut buffer, &mut str_len);

        let warnings = sr.write_odbc_type(1000000.0, &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        let numeric: &sql::Numeric = unsafe { &*(buffer.as_ptr() as *const sql::Numeric) };
        assert_eq!(numeric.sign, 1);
        assert_eq!(u128::from_le_bytes(numeric.val), 1000000);
    }

    #[test]
    fn real_binary_large_positive_beyond_i64() {
        let sr = make_real();
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_binary(&mut buffer, &mut str_len);

        let large = 1e19_f64;
        let warnings = sr.write_odbc_type(large, &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        let numeric: &sql::Numeric = unsafe { &*(buffer.as_ptr() as *const sql::Numeric) };
        assert_eq!(numeric.sign, 1);
        assert_eq!(u128::from_le_bytes(numeric.val), large as u128);
    }

    #[test]
    fn real_binary_large_negative_beyond_i64() {
        let sr = make_real();
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_binary(&mut buffer, &mut str_len);

        let large_neg = -1e19_f64;
        let warnings = sr.write_odbc_type(large_neg, &binding, &mut None).unwrap();

        assert!(warnings.is_empty());
        let numeric: &sql::Numeric = unsafe { &*(buffer.as_ptr() as *const sql::Numeric) };
        assert_eq!(numeric.sign, 0);
        assert_eq!(u128::from_le_bytes(numeric.val), large_neg.abs() as u128);
    }

    #[test]
    fn real_binary_overflow_returns_error() {
        use crate::conversion::error::WriteOdbcError;
        let sr = make_real();
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_binary(&mut buffer, &mut str_len);

        let result = sr.write_odbc_type(f64::MAX, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::NumericValueOutOfRange { .. }
        ));
    }

    // ======================================================================
    // Multiple row indices
    // ======================================================================

    #[test]
    fn real_reads_from_different_indices() {
        use crate::conversion::ReadArrowType;
        use arrow::array::Float64Array;

        let sr = make_real();
        let array = Float64Array::from(vec![1.1, 2.2, 3.3]);

        for (idx, expected) in [(0, 1.1), (1, 2.2), (2, 3.3)] {
            let value: f64 = sr.read_arrow_type(&array, idx).unwrap();
            assert!((value - expected).abs() < f64::EPSILON);
        }
    }

    // ======================================================================
    // BIT — negative fractional values must error (22003)
    // ======================================================================

    #[test]
    fn real_bit_negative_fraction_errors() {
        use crate::conversion::error::WriteOdbcError;
        let sr = make_real();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        let result = sr.write_odbc_type(-0.5, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::NumericValueOutOfRange { .. }
        ));
    }

    #[test]
    fn real_bit_negative_tiny_fraction_errors() {
        use crate::conversion::error::WriteOdbcError;
        let sr = make_real();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        let result = sr.write_odbc_type(-0.001, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::NumericValueOutOfRange { .. }
        ));
    }

    #[test]
    fn real_bit_nan_errors() {
        use crate::conversion::error::WriteOdbcError;
        let sr = make_real();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        let result = sr.write_odbc_type(f64::NAN, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::NumericValueOutOfRange { .. }
        ));
    }

    #[test]
    fn real_bit_infinity_errors() {
        use crate::conversion::error::WriteOdbcError;
        let sr = make_real();
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);

        let result = sr.write_odbc_type(f64::INFINITY, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::NumericValueOutOfRange { .. }
        ));
    }

    // ======================================================================
    // NaN — must error for all integer types (22003)
    // ======================================================================

    macro_rules! nan_integer_error_tests {
        ($($name:ident: $c_type:expr, $rust_type:ty;)*) => {
            $(
                #[test]
                fn $name() {
                    use crate::conversion::error::WriteOdbcError;
                    let sr = make_real();
                    let mut value: $rust_type = 0;
                    let mut str_len: sql::Len = 0;
                    let binding = binding_for_value($c_type, &mut value, &mut str_len);
                    let result = sr.write_odbc_type(f64::NAN, &binding, &mut None);
                    assert!(matches!(
                        result.unwrap_err(),
                        WriteOdbcError::NumericValueOutOfRange { .. }
                    ));
                }
            )*
        };
    }

    nan_integer_error_tests! {
        nan_to_short_errors:    CDataType::Short,    i16;
        nan_to_ushort_errors:   CDataType::UShort,   u16;
        nan_to_tinyint_errors:  CDataType::TinyInt,  i8;
        nan_to_utinyint_errors: CDataType::UTinyInt,  u8;
        nan_to_long_errors:     CDataType::Long,     i32;
        nan_to_ulong_errors:    CDataType::ULong,    u32;
        nan_to_sbigint_errors:  CDataType::SBigInt,  i64;
        nan_to_ubigint_errors:  CDataType::UBigInt,  u64;
    }

    // ======================================================================
    // NaN — must error for Numeric and Binary (22003)
    // ======================================================================

    #[test]
    fn nan_to_numeric_errors() {
        use crate::conversion::error::WriteOdbcError;
        let sr = make_real();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0u8; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_numeric(&mut value, &mut str_len, None, None);

        let result = sr.write_odbc_type(f64::NAN, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::NumericValueOutOfRange { .. }
        ));
    }

    #[test]
    fn nan_to_binary_errors() {
        use crate::conversion::error::WriteOdbcError;
        let sr = make_real();
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_binary(&mut buffer, &mut str_len);

        let result = sr.write_odbc_type(f64::NAN, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::NumericValueOutOfRange { .. }
        ));
    }

    // ======================================================================
    // Negative zero — Numeric and Binary must produce sign=0 (negative)
    // when magnitude is zero
    // ======================================================================

    #[test]
    fn numeric_negative_fraction_produces_negative_zero() {
        let sr = make_real();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0u8; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_numeric(&mut value, &mut str_len, None, None);

        let warnings = sr.write_odbc_type(-0.5, &binding, &mut None).unwrap();
        assert!(warnings.contains(&Warning::NumericValueTruncated));
        assert_eq!(value.sign, 0, "sign preserves source negativity");
        assert_eq!(u128::from_le_bytes(value.val), 0);
    }

    #[test]
    fn numeric_negative_tiny_fraction_produces_negative_zero() {
        let sr = make_real();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0u8; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = binding_for_numeric(&mut value, &mut str_len, None, None);

        let warnings = sr.write_odbc_type(-0.001, &binding, &mut None).unwrap();
        assert!(warnings.contains(&Warning::NumericValueTruncated));
        assert_eq!(value.sign, 0, "sign preserves source negativity");
        assert_eq!(u128::from_le_bytes(value.val), 0);
    }

    #[test]
    fn binary_negative_fraction_produces_negative_zero() {
        let sr = make_real();
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_binary(&mut buffer, &mut str_len);

        let warnings = sr.write_odbc_type(-0.5, &binding, &mut None).unwrap();
        assert!(warnings.contains(&Warning::NumericValueTruncated));
        let numeric: &sql::Numeric = unsafe { &*(buffer.as_ptr() as *const sql::Numeric) };
        assert_eq!(numeric.sign, 0, "sign preserves source negativity");
        assert_eq!(u128::from_le_bytes(numeric.val), 0);
    }

    #[test]
    fn binary_negative_tiny_fraction_produces_negative_zero() {
        let sr = make_real();
        let mut buffer = vec![0u8; std::mem::size_of::<sql::Numeric>()];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_binary(&mut buffer, &mut str_len);

        let warnings = sr.write_odbc_type(-0.001, &binding, &mut None).unwrap();
        assert!(warnings.contains(&Warning::NumericValueTruncated));
        let numeric: &sql::Numeric = unsafe { &*(buffer.as_ptr() as *const sql::Numeric) };
        assert_eq!(numeric.sign, 0, "sign preserves source negativity");
        assert_eq!(u128::from_le_bytes(numeric.val), 0);
    }

    // ======================================================================
    // Unsupported type
    // ======================================================================

    #[test]
    fn real_unsupported_type_returns_error() {
        let sr = make_real();
        let mut value: i32 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::TypeDate, &mut value, &mut str_len);

        assert!(sr.write_odbc_type(1.0, &binding, &mut None).is_err());
    }
}
