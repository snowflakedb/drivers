#[cfg(test)]
mod tests {
    use crate::cdata_types::CDataType;
    use crate::conversion::WriteODBCType;
    use crate::conversion::number::{NumericSqlType, SnowflakeNumber};
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
            str_len_or_ind_ptr: str_len as *mut sql::Len,
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
            str_len_or_ind_ptr: str_len as *mut sql::Len,
        }
    }

    fn make_decimal(scale: u32, precision: u32) -> SnowflakeNumber {
        SnowflakeNumber {
            scale,
            precision,
            sql_type: NumericSqlType::Decimal,
        }
    }

    #[test]
    fn decimal_default_c_type_is_char() {
        assert_eq!(NumericSqlType::Decimal.default_c_type(), CDataType::Char);
    }

    // ========================================================================
    // Integer conversions — success cases
    // (SQL_C_LONG, SQL_C_SHORT, SQL_C_TINYINT, SQL_C_SBIGINT, etc.)
    // Per ODBC spec: exact conversion → no warning; fractional truncation → 01S07
    // ========================================================================

    macro_rules! integer_conversion_tests {
        ($($name:ident: $c_type:expr, $rust_type:ty, $scale:expr, $precision:expr, $input:expr => $expected:expr, truncated=$trunc:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    let sn = make_decimal($scale, $precision);
                    let mut value: $rust_type = 0 as $rust_type;
                    let mut str_len: sql::Len = 0;
                    let binding = binding_for_value($c_type, &mut value, &mut str_len);
                    let warnings = sn.write_odbc_type($input, &binding).unwrap();
                    assert_eq!(value, $expected as $rust_type);
                    assert_eq!(!warnings.is_empty(), $trunc,
                        "truncation warning mismatch: expected truncated={}, got warnings={:?}",
                        $trunc, warnings);
                }
            )*
        };
    }

    integer_conversion_tests! {
        // Basic values — no truncation
        slong_integer:                          CDataType::SLong,    i32, 0,  10, 42i128                         => 42,                            truncated=false;
        sbigint_integer:                        CDataType::SBigInt,  i64, 0,  10, 123456789i128                  => 123456789,                     truncated=false;
        short_integer:                          CDataType::Short,    i16, 0,  5,  300i128                        => 300,                           truncated=false;
        tinyint_integer:                        CDataType::TinyInt,  i8,  0,  3,  123i128                        => 123,                           truncated=false;

        // Zero across all types
        slong_zero:                             CDataType::SLong,    i32, 0,  10, 0i128                          => 0,                             truncated=false;
        sbigint_zero:                           CDataType::SBigInt,  i64, 0,  10, 0i128                          => 0,                             truncated=false;
        short_zero:                             CDataType::Short,    i16, 0,  5,  0i128                          => 0,                             truncated=false;
        tinyint_zero:                           CDataType::TinyInt,  i8,  0,  3,  0i128                          => 0,                             truncated=false;

        // One and negative one
        slong_one:                              CDataType::SLong,    i32, 0,  10, 1i128                          => 1,                             truncated=false;
        slong_neg_one:                          CDataType::SLong,    i32, 0,  10, -1i128                         => -1,                            truncated=false;

        // Negative values
        slong_negative:                         CDataType::SLong,    i32, 0,  10, -42i128                        => -42,                           truncated=false;
        sbigint_negative:                       CDataType::SBigInt,  i64, 0,  10, -123456789i128                 => -123456789,                    truncated=false;

        // Boundary values — no truncation
        slong_i32_max:                          CDataType::SLong,    i32, 0,  10, 2_147_483_647i128              => 2_147_483_647,                 truncated=false;
        slong_i32_min:                          CDataType::SLong,    i32, 0,  10, -2_147_483_648i128             => -2_147_483_648,                truncated=false;
        sbigint_i64_max:                        CDataType::SBigInt,  i64, 0,  19, 9_223_372_036_854_775_807i128  => 9_223_372_036_854_775_807i64,  truncated=false;
        sbigint_i64_min:                        CDataType::SBigInt,  i64, 0,  19, -9_223_372_036_854_775_808i128 => -9_223_372_036_854_775_808i64, truncated=false;
        short_i16_max:                          CDataType::Short,    i16, 0,  5,  32767i128                      => 32767,                         truncated=false;
        short_i16_min:                          CDataType::Short,    i16, 0,  5,  -32768i128                     => -32768,                        truncated=false;
        ushort_u16_max:                         CDataType::UShort,   u16, 0,  5,  65535i128                      => 65535,                         truncated=false;
        tinyint_i8_max:                         CDataType::TinyInt,  i8,  0,  3,  127i128                        => 127,                           truncated=false;
        tinyint_i8_min:                         CDataType::TinyInt,  i8,  0,  3,  -128i128                       => -128,                          truncated=false;
        utinyint_u8_max:                        CDataType::UTinyInt, u8,  0,  3,  255i128                        => 255,                           truncated=false;

        // Fractional truncation — should produce 01S07 warning
        slong_truncates_positive_frac:          CDataType::SLong,    i32, 2,  10, 999i128                        => 9,                             truncated=true;
        slong_truncates_negative_frac:          CDataType::SLong,    i32, 2,  10, -999i128                       => -9,                            truncated=true;
        slong_frac_below_one_to_zero:           CDataType::SLong,    i32, 1,  10, 9i128                          => 0,                             truncated=true;
        slong_neg_frac_below_one_to_zero:       CDataType::SLong,    i32, 1,  10, -9i128                         => 0,                             truncated=true;
        sbigint_truncates_frac:                 CDataType::SBigInt,  i64, 3,  10, 12345i128                      => 12,                            truncated=true;
        short_truncates_frac:                   CDataType::Short,    i16, 1,  5,  255i128                        => 25,                            truncated=true;
        tinyint_truncates_frac:                 CDataType::TinyInt,  i8,  1,  3,  99i128                         => 9,                             truncated=true;

        // High scale
        slong_zero_scale_10:                    CDataType::SLong,    i32, 10, 38, 0i128                          => 0,                             truncated=false;
        slong_zero_scale_37:                    CDataType::SLong,    i32, 37, 38, 0i128                          => 0,                             truncated=false;
        slong_positive_scale_10:                CDataType::SLong,    i32, 10, 38, 50_000_000_000i128             => 5,                             truncated=false;
        slong_negative_scale_10:                CDataType::SLong,    i32, 10, 38, -30_000_000_000i128            => -3,                            truncated=false;
        long_zero_scale_15:                     CDataType::Long,     i32, 15, 20, 0i128                          => 0,                             truncated=false;
        ulong_zero_scale_10:                    CDataType::ULong,    u32, 10, 38, 0i128                          => 0,                             truncated=false;
        sbigint_zero_scale_20:                  CDataType::SBigInt,  i64, 20, 38, 0i128                          => 0,                             truncated=false;
        short_zero_scale_10:                    CDataType::Short,    i16, 10, 38, 0i128                          => 0,                             truncated=false;
        tinyint_zero_scale_10:                  CDataType::TinyInt,  i8,  10, 38, 0i128                          => 0,                             truncated=false;

        // Type aliases
        sshort_integer:                         CDataType::SShort,   i16, 0,  5,  300i128                        => 300,                           truncated=false;
        ushort_integer:                         CDataType::UShort,   u16, 0,  5,  300i128                        => 300,                           truncated=false;
        stinyint_integer:                       CDataType::STinyInt, i8,  0,  3,  100i128                        => 100,                           truncated=false;
        utinyint_integer:                       CDataType::UTinyInt, u8,  0,  3,  200i128                        => 200,                           truncated=false;
        ubigint_integer:                        CDataType::UBigInt,  u64, 0,  10, 999i128                        => 999,                           truncated=false;
        ubigint_u64_max:                        CDataType::UBigInt,  u64, 0,  20, 18_446_744_073_709_551_615i128 => 18_446_744_073_709_551_615u64, truncated=false;
    }

    // ========================================================================
    // Integer conversions — overflow error cases (SQLSTATE 22003)
    // ========================================================================

    macro_rules! integer_overflow_tests {
        ($($name:ident: $c_type:expr, $rust_type:ty, $scale:expr, $precision:expr, $input:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    let sn = make_decimal($scale, $precision);
                    let mut value: $rust_type = 0 as $rust_type;
                    let mut str_len: sql::Len = 0;
                    let binding = binding_for_value($c_type, &mut value, &mut str_len);
                    assert!(sn.write_odbc_type($input, &binding).is_err());
                }
            )*
        };
    }

    integer_overflow_tests! {
        // i32 overflow
        slong_overflow_above:               CDataType::SLong,    i32, 0, 10, 2_147_483_648i128;
        slong_overflow_below:               CDataType::SLong,    i32, 0, 10, -2_147_483_649i128;

        // u32 overflow (ULong)
        ulong_overflow_above:               CDataType::ULong,    u32, 0, 10, 4_294_967_296i128;
        ulong_overflow_negative:            CDataType::ULong,    u32, 0, 10, -1i128;

        // i16 overflow
        short_overflow_above:               CDataType::Short,    i16, 0, 5,  32768i128;
        short_overflow_below:               CDataType::Short,    i16, 0, 5,  -32769i128;

        // u16 overflow (UShort)
        ushort_overflow_above:              CDataType::UShort,   u16, 0, 5,  65536i128;
        ushort_overflow_negative:           CDataType::UShort,   u16, 0, 5,  -1i128;

        // i8 overflow
        tinyint_overflow_above:             CDataType::TinyInt,  i8,  0, 3,  128i128;
        tinyint_overflow_below:             CDataType::TinyInt,  i8,  0, 3,  -129i128;

        // u8 overflow (UTinyInt)
        utinyint_overflow_above:            CDataType::UTinyInt, u8,  0, 3,  256i128;
        utinyint_overflow_negative:         CDataType::UTinyInt, u8,  0, 3,  -1i128;

        // i64 overflow
        sbigint_overflow_above:             CDataType::SBigInt,  i64, 0, 20, 9_223_372_036_854_775_808i128;
        sbigint_overflow_below:             CDataType::SBigInt,  i64, 0, 20, -9_223_372_036_854_775_809i128;

        // u64 overflow (UBigInt)
        ubigint_overflow_above:             CDataType::UBigInt,  u64, 0, 20, 18_446_744_073_709_551_616i128;
        ubigint_overflow_negative:          CDataType::UBigInt,  u64, 0, 20, -1i128;

        // Overflow after scale division (value fits in i128 but not in target after division)
        slong_overflow_after_scale:         CDataType::SLong,    i32, 1, 10, 21_474_836_480i128;
    }

    // ========================================================================
    // Char / Default string conversions
    // ========================================================================

    macro_rules! char_conversion_tests {
        ($($name:ident: $c_type:expr, $scale:expr, $precision:expr, $input:expr => $expected:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    let sn = make_decimal($scale, $precision);
                    let mut buffer = vec![0u8; 128];
                    let mut str_len: sql::Len = 0;
                    let binding = binding_for_char_buffer($c_type, &mut buffer, &mut str_len);
                    sn.write_odbc_type($input, &binding).unwrap();
                    let expected: &str = $expected;
                    assert_eq!(str_len, expected.len() as sql::Len);
                    assert_eq!(&buffer[..expected.len()], expected.as_bytes());
                    assert_eq!(buffer[expected.len()], 0);
                }
            )*
        };
    }

    char_conversion_tests! {
        // Default type (maps to Char)
        default_integer_as_char:                CDataType::Default, 0, 10, 42i128               => "42";
        default_scaled_as_char:                 CDataType::Default, 2, 10, 12345i128            => "123.45";
        default_negative_scaled_as_char:        CDataType::Default, 3, 10, -50i128              => "-0.050";
        default_zero_as_char:                   CDataType::Default, 0, 10, 0i128                => "0";

        // Explicit Char type
        char_integer:                           CDataType::Char,    0, 10, 42i128               => "42";
        char_negative_integer:                  CDataType::Char,    0, 10, -42i128              => "-42";
        char_one:                               CDataType::Char,    0, 10, 1i128                => "1";
        char_negative_one:                      CDataType::Char,    0, 10, -1i128               => "-1";
        char_single_digit:                      CDataType::Char,    0, 1,  5i128                => "5";

        // Leading zeros in fractional part
        char_leading_zeros_3:                   CDataType::Char,    3, 10, 1i128                => "0.001";
        char_leading_zeros_3_neg:               CDataType::Char,    3, 10, -1i128               => "-0.001";
        char_leading_zeros_5:                   CDataType::Char,    5, 10, 1i128                => "0.00001";
        char_leading_zeros_5_neg:               CDataType::Char,    5, 10, -1i128               => "-0.00001";

        // Zero with various scales
        char_zero_scale_1:                      CDataType::Char,    1, 10, 0i128                => "0.0";
        char_zero_scale_3:                      CDataType::Char,    3, 10, 0i128                => "0.000";
        char_zero_scale_5:                      CDataType::Char,    5, 10, 0i128                => "0.00000";

        // Scale boundary: value digits == scale (entire value is fractional)
        char_scale_equals_digits:               CDataType::Char,    2, 10, 99i128               => "0.99";
        char_scale_equals_digits_neg:           CDataType::Char,    2, 10, -99i128              => "-0.99";
        char_scale_exactly_at_boundary:         CDataType::Char,    2, 10, 100i128              => "1.00";
        char_trailing_zeros_preserved:          CDataType::Char,    3, 10, 1000i128             => "1.000";

        // Large numbers
        char_large_integer:                     CDataType::Char,    0, 38, 99999999999999i128   => "99999999999999";
        char_large_negative:                    CDataType::Char,    0, 38, -99999999999999i128  => "-99999999999999";
        char_large_with_scale:                  CDataType::Char,    2, 38, 9999999999999900i128 => "99999999999999.00";
    }

    // ========================================================================
    // WChar conversions
    // ========================================================================

    macro_rules! wchar_conversion_tests {
        ($($name:ident: $scale:expr, $precision:expr, $input:expr => $expected:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    let sn = make_decimal($scale, $precision);
                    let mut buffer = vec![0u16; 128];
                    let mut str_len: sql::Len = 0;
                    let binding = Binding {
                        target_type: CDataType::WChar,
                        target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
                        buffer_length: (buffer.len() * 2) as sql::Len,
                        str_len_or_ind_ptr: &mut str_len as *mut sql::Len,
                    };
                    sn.write_odbc_type($input, &binding).unwrap();
                    let expected_str: &str = $expected;
                    let expected: Vec<u16> = expected_str.encode_utf16().collect();
                    assert_eq!(str_len, (expected.len() * 2) as sql::Len);
                    assert_eq!(&buffer[..expected.len()], &expected[..]);
                    assert_eq!(buffer[expected.len()], 0);
                }
            )*
        };
    }

    wchar_conversion_tests! {
        wchar_integer:              0, 10, 42i128     => "42";
        wchar_scaled:               2, 10, 12345i128  => "123.45";
        wchar_zero:                 0, 10, 0i128      => "0";
        wchar_negative:             0, 10, -42i128    => "-42";
        wchar_negative_scaled:      2, 10, -12345i128 => "-123.45";
        wchar_leading_zeros:        3, 10, 1i128      => "0.001";
        wchar_zero_with_scale:      2, 10, 0i128      => "0.00";
        wchar_large:                0, 38, 999999i128 => "999999";
    }

    // ========================================================================
    // Float / Double conversions (approximate comparison)
    // ========================================================================

    macro_rules! float_conversion_tests {
        ($($name:ident: $c_type:expr, $rust_type:ty, $scale:expr, $precision:expr, $input:expr => approx $expected:expr, tol $tol:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    let sn = make_decimal($scale, $precision);
                    let mut value: $rust_type = 0.0;
                    let mut str_len: sql::Len = 0;
                    let binding = binding_for_value($c_type, &mut value, &mut str_len);
                    sn.write_odbc_type($input, &binding).unwrap();
                    assert!(
                        (value - ($expected) as $rust_type).abs() < ($tol) as $rust_type,
                        "expected approximately {}, got {}", $expected, value
                    );
                }
            )*
        };
    }

    float_conversion_tests! {
        // f64
        double_integer:             CDataType::Double, f64, 0, 10, 42i128            => approx 42.0,    tol f64::EPSILON;
        double_zero:                CDataType::Double, f64, 0, 10, 0i128             => approx 0.0,     tol f64::EPSILON;
        double_negative:            CDataType::Double, f64, 0, 10, -42i128           => approx -42.0,   tol f64::EPSILON;
        double_one:                 CDataType::Double, f64, 0, 10, 1i128             => approx 1.0,     tol f64::EPSILON;
        double_neg_one:             CDataType::Double, f64, 0, 10, -1i128            => approx -1.0,    tol f64::EPSILON;
        double_scaled:              CDataType::Double, f64, 2, 10, 12345i128         => approx 123.45,  tol 0.001;
        double_negative_scaled:     CDataType::Double, f64, 3, 10, -50i128           => approx -0.05,   tol 0.001;
        double_large:               CDataType::Double, f64, 0, 15, 1_000_000_000i128 => approx 1e9,     tol 1.0;
        double_small_fraction:      CDataType::Double, f64, 5, 10, 1i128             => approx 0.00001, tol 1e-8;

        // f32
        float_scaled:               CDataType::Float,  f32, 3, 10, 123789i128        => approx 123.789, tol 0.01;
        float_zero:                 CDataType::Float,  f32, 0, 10, 0i128             => approx 0.0,     tol f32::EPSILON;
        float_negative:             CDataType::Float,  f32, 0, 10, -100i128          => approx -100.0,  tol 0.01;
        float_small_fraction:       CDataType::Float,  f32, 2, 10, 1i128             => approx 0.01,    tol 0.001;
        float_one:                  CDataType::Float,  f32, 0, 10, 1i128             => approx 1.0,     tol f32::EPSILON;
    }

    // ========================================================================
    // SQL_C_NUMERIC struct conversions
    // Per ODBC spec: fractional truncation → 01S07
    // ========================================================================

    macro_rules! numeric_struct_tests {
        ($($name:ident: $scale:expr, $precision:expr, $input:expr => sign=$sign:expr, val=$val:expr, truncated=$trunc:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    let sn = make_decimal($scale, $precision);
                    let mut value = sql::Numeric {
                        precision: 0,
                        scale: 0,
                        sign: 0,
                        val: [0u8; 16],
                    };
                    let mut str_len: sql::Len = 0;
                    let binding = binding_for_value(CDataType::Numeric, &mut value, &mut str_len);
                    let warnings = sn.write_odbc_type($input, &binding).unwrap();
                    assert_eq!(value.precision, $precision as u8);
                    assert_eq!(value.scale, 0);
                    assert_eq!(value.sign, $sign);
                    assert_eq!(u128::from_le_bytes(value.val), $val as u128);
                    assert_eq!(!warnings.is_empty(), $trunc,
                        "truncation warning mismatch: expected truncated={}, got warnings={:?}",
                        $trunc, warnings);
                }
            )*
        };
    }

    numeric_struct_tests! {
        numeric_positive_with_scale:    2,  10, 12345i128           => sign=1, val=123,        truncated=true;
        numeric_negative:               0,  10, -42i128             => sign=0, val=42,         truncated=false;
        numeric_zero:                   0,  10, 0i128               => sign=1, val=0,          truncated=false;
        numeric_one:                    0,  10, 1i128               => sign=1, val=1,          truncated=false;
        numeric_negative_one:           0,  10, -1i128              => sign=0, val=1,          truncated=false;

        // LE byte boundary values
        numeric_255:                    0,  10, 255i128             => sign=1, val=255,        truncated=false;
        numeric_256:                    0,  10, 256i128             => sign=1, val=256,        truncated=false;
        numeric_65535:                  0,  10, 65535i128           => sign=1, val=65535,      truncated=false;
        numeric_65536:                  0,  10, 65536i128           => sign=1, val=65536,      truncated=false;
        numeric_1_000_000:              0,  10, 1_000_000i128       => sign=1, val=1_000_000,  truncated=false;

        // Scale truncation
        numeric_scale_truncates_frac:   2,  10, 999i128             => sign=1, val=9,          truncated=true;
        numeric_scale_neg_truncates:    2,  10, -999i128            => sign=0, val=9,          truncated=true;
        numeric_zero_with_scale:        5,  10, 0i128               => sign=1, val=0,          truncated=false;

        // High scale
        numeric_high_scale_zero:        10, 38, 0i128               => sign=1, val=0,          truncated=false;
        numeric_high_scale_positive:    10, 38, 50_000_000_000i128  => sign=1, val=5,          truncated=false;
        numeric_high_scale_negative:    10, 38, -30_000_000_000i128 => sign=0, val=3,          truncated=false;
        numeric_scale_37_zero:          37, 38, 0i128               => sign=1, val=0,          truncated=false;
    }

    // ========================================================================
    // SQL_C_BINARY conversions (raw SQL_NUMERIC_STRUCT bytes)
    // ========================================================================

    macro_rules! binary_struct_tests {
        ($($name:ident: $scale:expr, $precision:expr, $input:expr => sign=$sign:expr, first_val_byte=$byte:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    let sn = make_decimal($scale, $precision);
                    let mut buffer = vec![0u8; 64];
                    let mut str_len: sql::Len = 0;
                    let binding = binding_for_char_buffer(CDataType::Binary, &mut buffer, &mut str_len);
                    sn.write_odbc_type($input, &binding).unwrap();
                    let numeric_size = std::mem::size_of::<sql::Numeric>() as sql::Len;
                    assert_eq!(str_len, numeric_size);
                    assert_eq!(buffer[2], $sign);   // sign at offset 2 (after precision + scale)
                    assert_eq!(buffer[3], $byte);   // val[0] at offset 3
                }
            )*
        };
    }

    binary_struct_tests! {
        binary_integer:             0,  10, 42i128             => sign=1, first_val_byte=42;
        binary_with_scale:          2,  10, 12345i128          => sign=1, first_val_byte=123;
        binary_zero:                0,  10, 0i128              => sign=1, first_val_byte=0;
        binary_one:                 0,  10, 1i128              => sign=1, first_val_byte=1;
        binary_negative:            0,  10, -42i128            => sign=0, first_val_byte=42;
        binary_255:                 0,  10, 255i128            => sign=1, first_val_byte=255;
        binary_256_le_low_byte:     0,  10, 256i128            => sign=1, first_val_byte=0;
        binary_high_scale_zero:     10, 38, 0i128              => sign=1, first_val_byte=0;
        binary_high_scale_positive: 10, 38, 50_000_000_000i128 => sign=1, first_val_byte=5;
    }

    // ========================================================================
    // SQL_C_BIT conversions (per ODBC spec)
    //   Exact 0 or 1       → ok, no warning
    //   0 < value < 2, ≠ 1 → truncate, 01S07
    //   value < 0 or ≥ 2   → 22003 error
    // ========================================================================

    macro_rules! bit_ok_tests {
        ($($name:ident: $scale:expr, $precision:expr, $input:expr => $expected:expr, truncated=$trunc:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    let sn = make_decimal($scale, $precision);
                    let mut value: u8 = 0xFF;
                    let mut str_len: sql::Len = 0;
                    let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);
                    let warnings = sn.write_odbc_type($input, &binding).unwrap();
                    assert_eq!(value, $expected);
                    assert_eq!(!warnings.is_empty(), $trunc,
                        "truncation warning mismatch: expected truncated={}, got warnings={:?}",
                        $trunc, warnings);
                }
            )*
        };
    }

    bit_ok_tests! {
        // Exact values — no warning
        bit_exact_zero:                     0,  10, 0i128   => 0, truncated=false;
        bit_exact_one:                      0,  10, 1i128   => 1, truncated=false;
        bit_exact_one_via_scale:            2,  10, 100i128 => 1, truncated=false;
        bit_exact_zero_high_scale:          10, 38, 0i128   => 0, truncated=false;

        // Fractional truncation → 01S07
        bit_frac_truncates_to_zero:         1,  10, 9i128   => 0, truncated=true;
        bit_frac_truncates_to_one:          1,  10, 15i128  => 1, truncated=true;
        bit_frac_099_truncates_to_zero:     2,  10, 99i128  => 0, truncated=true;
        bit_frac_150_truncates_to_one:      2,  10, 150i128 => 1, truncated=true;
    }

    macro_rules! bit_error_tests {
        ($($name:ident: $scale:expr, $precision:expr, $input:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    let sn = make_decimal($scale, $precision);
                    let mut value: u8 = 0;
                    let mut str_len: sql::Len = 0;
                    let binding = binding_for_value(CDataType::Bit, &mut value, &mut str_len);
                    assert!(sn.write_odbc_type($input, &binding).is_err());
                }
            )*
        };
    }

    bit_error_tests! {
        // value >= 2
        bit_rejects_two:                 0, 10, 2i128;
        bit_rejects_large_positive:      0, 10, 100i128;
        bit_rejects_frac_truncates_to_2: 1, 10, 25i128;

        // value < 0 (checked on original snowflake_value, not truncated)
        bit_rejects_negative_one:        0, 10, -1i128;
        bit_rejects_large_negative:      0, 10, -100i128;
        bit_rejects_neg_frac:            1, 10, -5i128;
    }
}
