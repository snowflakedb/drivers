#[cfg(test)]
mod tests {
    use crate::api::CDataType;
    use crate::conversion::WriteODBCType;
    use crate::conversion::number::{NumericSettings, NumericSqlType, SnowflakeNumber};
    use crate::conversion::test_utils::helpers::{
        binding_for_char_buffer, binding_for_interval, binding_for_value, binding_for_wchar_buffer,
        zero_interval,
    };
    use crate::conversion::traits::Binding;
    use odbc_sys as sql;

    const SETTINGS_DEFAULT: NumericSettings = NumericSettings {
        treat_decimal_as_int: false,
        treat_big_number_as_string: false,
        max_varchar_size: 16_777_216,
    };

    const SETTINGS_DECIMAL_AS_INT: NumericSettings = NumericSettings {
        treat_decimal_as_int: true,
        treat_big_number_as_string: false,
        max_varchar_size: 16_777_216,
    };

    const SETTINGS_BOTH: NumericSettings = NumericSettings {
        treat_decimal_as_int: true,
        treat_big_number_as_string: true,
        max_varchar_size: 16_777_216,
    };

    const SETTINGS_BIG_NUMBER_AS_STRING: NumericSettings = NumericSettings {
        treat_decimal_as_int: false,
        treat_big_number_as_string: true,
        max_varchar_size: 16_777_216,
    };

    fn make_decimal(scale: u32, precision: u32) -> SnowflakeNumber {
        SnowflakeNumber {
            scale,
            precision,
            sql_type: NumericSqlType::Decimal,
        }
    }

    fn make_number(scale: u32, precision: u32, settings: &NumericSettings) -> SnowflakeNumber {
        SnowflakeNumber {
            scale,
            precision,
            sql_type: NumericSqlType::from_scale_and_precision(scale, precision, settings),
        }
    }

    #[test]
    fn decimal_default_c_type_is_char() {
        assert_eq!(NumericSqlType::Decimal.default_c_type(), CDataType::Char);
    }

    #[test]
    fn bigint_default_c_type_is_sbigint() {
        assert_eq!(NumericSqlType::BigInt.default_c_type(), CDataType::SBigInt);
    }

    #[test]
    fn varchar_default_c_type_is_char() {
        assert_eq!(NumericSqlType::VarChar.default_c_type(), CDataType::Char);
    }

    // BD#11: treat_decimal_as_int=true, scale=0 → BigInt for any precision
    #[test]
    fn treat_decimal_as_int_scale_zero_resolves_to_bigint() {
        assert_eq!(
            NumericSqlType::from_scale_and_precision(0, 1, &SETTINGS_DECIMAL_AS_INT),
            NumericSqlType::BigInt
        );
        assert_eq!(
            NumericSqlType::from_scale_and_precision(0, 10, &SETTINGS_DECIMAL_AS_INT),
            NumericSqlType::BigInt
        );
        assert_eq!(
            NumericSqlType::from_scale_and_precision(0, 18, &SETTINGS_DECIMAL_AS_INT),
            NumericSqlType::BigInt
        );
        // precision > 18 still BigInt when treat_big_number_as_string=false
        assert_eq!(
            NumericSqlType::from_scale_and_precision(0, 19, &SETTINGS_DECIMAL_AS_INT),
            NumericSqlType::BigInt
        );
        assert_eq!(
            NumericSqlType::from_scale_and_precision(0, 38, &SETTINGS_DECIMAL_AS_INT),
            NumericSqlType::BigInt
        );
    }

    // BD#11: treat_decimal_as_int=false → Decimal regardless of scale
    #[test]
    fn no_treat_decimal_as_int_stays_decimal() {
        assert_eq!(
            NumericSqlType::from_scale_and_precision(0, 10, &SETTINGS_DEFAULT),
            NumericSqlType::Decimal
        );
        assert_eq!(
            NumericSqlType::from_scale_and_precision(0, 38, &SETTINGS_DEFAULT),
            NumericSqlType::Decimal
        );
    }

    // BD#11 + BD#12: treat_big_number_as_string overrides BigInt for precision > 18
    #[test]
    fn big_number_as_string_overrides_bigint() {
        // precision > 18: BigInt from step 2 is overridden to VarChar by step 3
        assert_eq!(
            NumericSqlType::from_scale_and_precision(0, 19, &SETTINGS_BOTH),
            NumericSqlType::VarChar
        );
        assert_eq!(
            NumericSqlType::from_scale_and_precision(0, 38, &SETTINGS_BOTH),
            NumericSqlType::VarChar
        );
        // precision <= 18: BigInt is NOT overridden
        assert_eq!(
            NumericSqlType::from_scale_and_precision(0, 18, &SETTINGS_BOTH),
            NumericSqlType::BigInt
        );
        assert_eq!(
            NumericSqlType::from_scale_and_precision(0, 10, &SETTINGS_BOTH),
            NumericSqlType::BigInt
        );
    }

    // BD#12: treat_big_number_as_string alone (no treat_decimal_as_int)
    #[test]
    fn big_number_as_string_without_decimal_as_int() {
        // precision > 18 → VarChar
        assert_eq!(
            NumericSqlType::from_scale_and_precision(0, 19, &SETTINGS_BIG_NUMBER_AS_STRING),
            NumericSqlType::VarChar
        );
        assert_eq!(
            NumericSqlType::from_scale_and_precision(5, 20, &SETTINGS_BIG_NUMBER_AS_STRING),
            NumericSqlType::VarChar
        );
        // precision <= 18 → still Decimal
        assert_eq!(
            NumericSqlType::from_scale_and_precision(0, 18, &SETTINGS_BIG_NUMBER_AS_STRING),
            NumericSqlType::Decimal
        );
    }

    // Non-zero scale → Decimal (treat_decimal_as_int only applies to scale=0)
    #[test]
    fn nonzero_scale_resolves_to_decimal() {
        assert_eq!(
            NumericSqlType::from_scale_and_precision(2, 10, &SETTINGS_DECIMAL_AS_INT),
            NumericSqlType::Decimal
        );
        assert_eq!(
            NumericSqlType::from_scale_and_precision(1, 18, &SETTINGS_DECIMAL_AS_INT),
            NumericSqlType::Decimal
        );
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
                    let warnings = sn.write_odbc_type($input, &binding, &mut None).unwrap();
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
                    assert!(sn.write_odbc_type($input, &binding, &mut None).is_err());
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
                    sn.write_odbc_type($input, &binding, &mut None).unwrap();
                    let expected: &str = $expected;
                    assert_eq!(str_len, expected.len() as sql::Len);
                    assert_eq!(&buffer[..expected.len()], expected.as_bytes());
                    assert_eq!(buffer[expected.len()], 0);
                }
            )*
        };
    }

    // Treat_decimal_as_int=true, scale=0, precision<=18 → Default resolves to SBigInt
    macro_rules! default_bigint_tests {
        ($($name:ident: $scale:expr, $precision:expr, $input:expr => $expected:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    let sn = make_number($scale, $precision, &SETTINGS_DECIMAL_AS_INT);
                    let mut value: i64 = 0;
                    let mut str_len: sql::Len = 0;
                    let binding = binding_for_value(CDataType::Default, &mut value, &mut str_len);
                    let warnings = sn.write_odbc_type($input, &binding, &mut None).unwrap();
                    assert!(warnings.is_empty());
                    assert_eq!(value, $expected);
                }
            )*
        };
    }

    default_bigint_tests! {
        default_decimal_as_int_positive:  0, 10, 42i128  => 42i64;
        default_decimal_as_int_zero:      0, 10, 0i128   => 0i64;
        default_decimal_as_int_negative:  0, 10, -42i128 => -42i64;
        default_decimal_as_int_max_prec:  0, 18, 999999999999999999i128 => 999999999999999999i64;
    }

    // BD#11: treat_decimal_as_int=false → Default still resolves to Char
    #[test]
    fn default_without_treat_decimal_as_int_is_char() {
        let sn = make_number(0, 10, &SETTINGS_DEFAULT);
        let mut buffer = vec![0u8; 128];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Default, &mut buffer, &mut str_len);
        sn.write_odbc_type(42i128, &binding, &mut None).unwrap();
        assert_eq!(str_len, 2);
        assert_eq!(&buffer[..2], b"42");
    }

    // precision > 18 with treat_decimal_as_int=true (no big_number_as_string)
    // → BigInt, so Default resolves to SBigInt
    #[test]
    fn default_high_precision_decimal_as_int_is_bigint() {
        let sn = make_number(0, 38, &SETTINGS_DECIMAL_AS_INT);
        let mut value: i64 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Default, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(42i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value, 42);
    }

    // BD#12: precision > 18 with both settings → VarChar, Default resolves to Char
    #[test]
    fn default_high_precision_both_settings_is_char() {
        let sn = make_number(0, 38, &SETTINGS_BOTH);
        let mut buffer = vec![0u8; 128];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Default, &mut buffer, &mut str_len);
        sn.write_odbc_type(42i128, &binding, &mut None).unwrap();
        assert_eq!(str_len, 2);
        assert_eq!(&buffer[..2], b"42");
    }

    // BD#12: precision > 18 with only big_number_as_string → VarChar, Default resolves to Char
    #[test]
    fn default_high_precision_big_number_as_string_is_char() {
        let sn = make_number(0, 38, &SETTINGS_BIG_NUMBER_AS_STRING);
        let mut buffer = vec![0u8; 128];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Default, &mut buffer, &mut str_len);
        sn.write_odbc_type(42i128, &binding, &mut None).unwrap();
        assert_eq!(str_len, 2);
        assert_eq!(&buffer[..2], b"42");
    }

    char_conversion_tests! {
        // Default type with scale > 0 (maps to Char via Decimal)
        default_scaled_as_char:                 CDataType::Default, 2, 10, 12345i128            => "123.45";
        default_negative_scaled_as_char:        CDataType::Default, 3, 10, -50i128              => "-0.050";

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
                    let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
                    sn.write_odbc_type($input, &binding, &mut None).unwrap();
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
                    sn.write_odbc_type($input, &binding, &mut None).unwrap();
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
                    let mut binding = binding_for_value(CDataType::Numeric, &mut value, &mut str_len);
                    binding.precision = Some($precision as i16);
                    binding.scale = Some(0);
                    let warnings = sn.write_odbc_type($input, &binding, &mut None).unwrap();
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
    // SQL_C_NUMERIC with explicit target precision and scale
    // Tests the behavior when SQL_DESC_PRECISION / SQL_DESC_SCALE differ
    // from the source column's precision/scale.
    // ========================================================================

    macro_rules! numeric_target_scale_tests {
        ($($name:ident: src_scale=$src_scale:expr, src_precision=$src_prec:expr, target_precision=$tgt_prec:expr, target_scale=$tgt_scale:expr, $input:expr
            => sign=$sign:expr, val=$val:expr, out_precision=$out_prec:expr, out_scale=$out_scale:expr, truncated=$trunc:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    let sn = make_decimal($src_scale, $src_prec);
                    let mut value = sql::Numeric {
                        precision: 0,
                        scale: 0,
                        sign: 0,
                        val: [0u8; 16],
                    };
                    let mut str_len: sql::Len = 0;
                    let mut binding = binding_for_value(CDataType::Numeric, &mut value, &mut str_len);
                    binding.precision = Some($tgt_prec as i16);
                    binding.scale = Some($tgt_scale as i16);
                    let warnings = sn.write_odbc_type($input, &binding, &mut None).unwrap();
                    assert_eq!(value.precision, $out_prec as u8, "precision mismatch");
                    assert_eq!(value.scale, $out_scale as i8, "scale mismatch");
                    assert_eq!(value.sign, $sign, "sign mismatch");
                    assert_eq!(u128::from_le_bytes(value.val), $val as u128, "val mismatch");
                    assert_eq!(!warnings.is_empty(), $trunc,
                        "truncation warning mismatch: expected truncated={}, got warnings={:?}",
                        $trunc, warnings);
                }
            )*
        };
    }

    numeric_target_scale_tests! {
        // Target scale == source scale → exact, no truncation
        numeric_exact_scale_match:
            src_scale=2, src_precision=10, target_precision=10, target_scale=2, 12345i128
            => sign=1, val=12345, out_precision=10, out_scale=2, truncated=false;

        // Target scale > source scale → upscale (multiply), no truncation
        numeric_upscale_integer:
            src_scale=0, src_precision=10, target_precision=10, target_scale=3, 42i128
            => sign=1, val=42000, out_precision=10, out_scale=3, truncated=false;

        numeric_upscale_from_scaled:
            src_scale=2, src_precision=10, target_precision=10, target_scale=5, 12345i128
            => sign=1, val=12345000, out_precision=10, out_scale=5, truncated=false;

        // Target scale < source scale → downscale (divide), fractional truncation
        numeric_downscale_truncates:
            src_scale=3, src_precision=10, target_precision=10, target_scale=1, 12345i128
            => sign=1, val=123, out_precision=10, out_scale=1, truncated=true;

        numeric_downscale_exact:
            src_scale=3, src_precision=10, target_precision=10, target_scale=1, 12300i128
            => sign=1, val=123, out_precision=10, out_scale=1, truncated=false;

        // Target precision differs from source precision → output uses target precision
        numeric_custom_precision:
            src_scale=0, src_precision=38, target_precision=5, target_scale=0, 42i128
            => sign=1, val=42, out_precision=5, out_scale=0, truncated=false;

        // Negative value with upscale
        numeric_negative_upscale:
            src_scale=0, src_precision=10, target_precision=10, target_scale=2, -7i128
            => sign=0, val=700, out_precision=10, out_scale=2, truncated=false;

        // Negative value with downscale truncation
        numeric_negative_downscale_truncation:
            src_scale=2, src_precision=10, target_precision=10, target_scale=0, -12345i128
            => sign=0, val=123, out_precision=10, out_scale=0, truncated=true;

        // Zero with various target scales
        numeric_zero_target_scale_3:
            src_scale=0, src_precision=10, target_precision=10, target_scale=3, 0i128
            => sign=1, val=0, out_precision=10, out_scale=3, truncated=false;

        numeric_zero_downscale:
            src_scale=5, src_precision=10, target_precision=10, target_scale=0, 0i128
            => sign=1, val=0, out_precision=10, out_scale=0, truncated=false;
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
                    sn.write_odbc_type($input, &binding, &mut None).unwrap();
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
    // SQL_C_CHAR truncation: 01004 vs 22003
    // Per ODBC spec: fractional-only truncation → 01004 (StringDataTruncated),
    //                whole digits don't fit → 22003 (NumericValueOutOfRange).
    // ========================================================================

    #[test]
    fn char_fractional_truncation_returns_01004() {
        use crate::conversion::warning::Warning;
        let sn = make_decimal(3, 10);
        // "12.345" → whole part "12" (2 chars), total 6 chars
        // Buffer of 4 can hold "12." (3 chars + null). Whole digits fit → 01004.
        let mut buffer = vec![0u8; 4];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(12345i128, &binding, &mut None).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
    }

    #[test]
    fn char_whole_digit_truncation_returns_22003() {
        let sn = make_decimal(0, 10);
        // "123456" → whole part "123456" (6 chars)
        // Buffer of 4 can only hold "123" + null. Whole digits don't fit → 22003.
        let mut buffer = vec![0u8; 4];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        assert!(sn.write_odbc_type(123456i128, &binding, &mut None).is_err());
    }

    #[test]
    fn char_negative_whole_digit_truncation_returns_22003() {
        let sn = make_decimal(0, 10);
        // "-123" → whole part "-123" (4 chars)
        // Buffer of 4 can hold "−12" + null. Whole digits don't fit → 22003.
        let mut buffer = vec![0u8; 4];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        assert!(sn.write_odbc_type(-123i128, &binding, &mut None).is_err());
    }

    #[test]
    fn char_negative_whole_digits_just_fit_returns_01004() {
        use crate::conversion::warning::Warning;
        let sn = make_decimal(2, 10);
        // Value 12345 with scale 2 → "-123.45"
        // Whole part is "-123" (4 chars). Buffer of 5 can hold "-123" + null.
        // Total doesn't fit → 01004 (fractional truncation only).
        let mut buffer = vec![0u8; 5];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(-12345i128, &binding, &mut None).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
    }

    #[test]
    fn char_exact_fit_no_truncation() {
        let sn = make_decimal(0, 10);
        // "42" → 2 chars + null = 3 bytes
        let mut buffer = vec![0u8; 3];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(42i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 2);
        assert_eq!(&buffer[..2], b"42");
    }

    // ========================================================================
    // SQL_C_WCHAR truncation: 01004 vs 22003
    // ========================================================================

    #[test]
    fn wchar_whole_digit_truncation_returns_22003() {
        let sn = make_decimal(0, 10);
        // "123456" → 6 wide chars + null = 7 * 2 = 14 bytes
        // Buffer of 8 bytes = 4 wide chars. Whole digits (6) >= capacity (4) → 22003.
        let mut buffer = vec![0u16; 4];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        assert!(sn.write_odbc_type(123456i128, &binding, &mut None).is_err());
    }

    #[test]
    fn wchar_fractional_truncation_returns_01004() {
        use crate::conversion::warning::Warning;
        let sn = make_decimal(3, 10);
        // "12.345" → whole part "12" (2 chars)
        // Buffer of 4 wide chars. Whole digits (2) < capacity (4) → 01004.
        let mut buffer = vec![0u16; 4];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(12345i128, &binding, &mut None).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
    }

    // ========================================================================
    // SQL_C_BINARY buffer too small → 22003
    // ========================================================================

    #[test]
    fn binary_buffer_too_small_returns_22003() {
        let sn = make_decimal(0, 10);
        let mut buffer = vec![0u8; 4]; // Too small for SQL_NUMERIC_STRUCT
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Binary, &mut buffer, &mut str_len);
        assert!(sn.write_odbc_type(42i128, &binding, &mut None).is_err());
    }

    #[test]
    fn binary_buffer_exact_size_succeeds() {
        let sn = make_decimal(0, 10);
        let numeric_size = std::mem::size_of::<sql::Numeric>();
        let mut buffer = vec![0u8; numeric_size];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Binary, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(42i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, numeric_size as sql::Len);
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
                    let warnings = sn.write_odbc_type($input, &binding, &mut None).unwrap();
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
                    assert!(sn.write_odbc_type($input, &binding, &mut None).is_err());
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

    // ========================================================================
    // Interval conversions -- single-field (SQLSTATE 01S07 for fractional)
    // ========================================================================

    #[test]
    fn interval_year_positive_integer() {
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalYear, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(5i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.interval_type, sql::Interval::Year as i32);
        assert_eq!(value.interval_sign, 0);
        assert_eq!(unsafe { value.interval_value.year_month.year }, 5);
    }

    #[test]
    fn interval_year_negative() {
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalYear, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(-3i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.interval_sign, 1);
        assert_eq!(unsafe { value.interval_value.year_month.year }, 3);
    }

    #[test]
    fn interval_year_zero() {
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalYear, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(0i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(unsafe { value.interval_value.year_month.year }, 0);
    }

    #[test]
    fn interval_year_fractional_truncates() {
        use crate::conversion::warning::Warning;
        let sn = make_decimal(1, 10); // scale=1, so 57 → 5.7
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalYear, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(57i128, &binding, &mut None).unwrap();
        assert!(warnings.contains(&Warning::NumericValueTruncated));
        assert_eq!(unsafe { value.interval_value.year_month.year }, 5);
    }

    #[test]
    fn interval_month_positive() {
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalMonth, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(10i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.interval_type, sql::Interval::Month as i32);
        assert_eq!(unsafe { value.interval_value.year_month.month }, 10);
    }

    #[test]
    fn interval_day_positive() {
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalDay, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(15i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.interval_type, sql::Interval::Day as i32);
        assert_eq!(unsafe { value.interval_value.day_second.day }, 15);
    }

    #[test]
    fn interval_hour_positive() {
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalHour, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(8i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.interval_type, sql::Interval::Hour as i32);
        assert_eq!(unsafe { value.interval_value.day_second.hour }, 8);
    }

    #[test]
    fn interval_minute_positive() {
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalMinute, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(30i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.interval_type, sql::Interval::Minute as i32);
        assert_eq!(unsafe { value.interval_value.day_second.minute }, 30);
    }

    #[test]
    fn interval_second_integer() {
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalSecond, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(45i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.interval_type, sql::Interval::Second as i32);
        assert_eq!(unsafe { value.interval_value.day_second.second }, 45);
        assert_eq!(unsafe { value.interval_value.day_second.fraction }, 0);
    }

    #[test]
    fn interval_second_with_fractional_part() {
        let sn = make_decimal(3, 10); // scale=3, so 45500 → 45.500
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalSecond, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(45_500i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(unsafe { value.interval_value.day_second.second }, 45);
        assert_eq!(unsafe { value.interval_value.day_second.fraction }, 500_000);
    }

    #[test]
    fn interval_second_negative_with_fraction() {
        let sn = make_decimal(2, 10); // scale=2, so -1025 → -10.25
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalSecond, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(-1025i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.interval_sign, 1);
        assert_eq!(unsafe { value.interval_value.day_second.second }, 10);
        assert_eq!(unsafe { value.interval_value.day_second.fraction }, 250_000);
    }

    #[test]
    fn interval_second_negative_integer() {
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalSecond, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(-45i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.interval_type, sql::Interval::Second as i32);
        assert_eq!(value.interval_sign, 1);
        assert_eq!(unsafe { value.interval_value.day_second.second }, 45);
        assert_eq!(unsafe { value.interval_value.day_second.fraction }, 0);
    }

    #[test]
    fn interval_year_no_negative_zero() {
        let sn = make_decimal(1, 10); // scale=1, so -5 represents -0.5
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalYear, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(-5i128, &binding, &mut None).unwrap();
        assert!(warnings.contains(&crate::conversion::warning::Warning::NumericValueTruncated));
        assert_eq!(value.interval_sign, 0);
        assert_eq!(unsafe { value.interval_value.year_month.year }, 0);
    }

    #[test]
    fn interval_month_no_negative_zero() {
        let sn = make_decimal(1, 10); // scale=1, so -3 represents -0.3
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalMonth, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(-3i128, &binding, &mut None).unwrap();
        assert!(warnings.contains(&crate::conversion::warning::Warning::NumericValueTruncated));
        assert_eq!(value.interval_sign, 0);
        assert_eq!(unsafe { value.interval_value.year_month.month }, 0);
    }

    #[test]
    fn interval_day_no_negative_zero() {
        let sn = make_decimal(1, 10); // scale=1, so -9 represents -0.9
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalDay, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(-9i128, &binding, &mut None).unwrap();
        assert!(warnings.contains(&crate::conversion::warning::Warning::NumericValueTruncated));
        assert_eq!(value.interval_sign, 0);
        assert_eq!(unsafe { value.interval_value.day_second.day }, 0);
    }

    #[test]
    fn interval_second_no_negative_zero() {
        let sn = make_decimal(4, 10); // scale=4, so -1 represents -0.0001
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalSecond, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(-1i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        let second = unsafe { value.interval_value.day_second.second };
        let fraction = unsafe { value.interval_value.day_second.fraction };
        if fraction > 0 {
            assert_eq!(value.interval_sign, 1);
        } else {
            assert_eq!(value.interval_sign, 0);
        }
        assert_eq!(second, 0);
    }

    #[test]
    fn interval_second_negative_fraction_keeps_sign() {
        let sn = make_decimal(1, 10); // scale=1, so -5 represents -0.5
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalSecond, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(-5i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(value.interval_sign, 1);
        assert_eq!(unsafe { value.interval_value.day_second.second }, 0);
        assert_eq!(unsafe { value.interval_value.day_second.fraction }, 500_000);
    }

    #[test]
    fn interval_second_sub_microsecond_truncation_warns() {
        use crate::conversion::warning::Warning;
        let sn = make_decimal(9, 15); // scale=9, so 45_123456789 → 45.123456789
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalSecond, &mut value, &mut str_len);
        let warnings = sn
            .write_odbc_type(45_123_456_789i128, &binding, &mut None)
            .unwrap();
        assert!(warnings.contains(&Warning::NumericValueTruncated));
        assert_eq!(unsafe { value.interval_value.day_second.second }, 45);
        assert_eq!(unsafe { value.interval_value.day_second.fraction }, 123_456);
    }

    #[test]
    fn interval_second_exact_microsecond_no_warning() {
        let sn = make_decimal(9, 15); // scale=9, so 45_123456000 → 45.123456000
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalSecond, &mut value, &mut str_len);
        let warnings = sn
            .write_odbc_type(45_123_456_000i128, &binding, &mut None)
            .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(unsafe { value.interval_value.day_second.second }, 45);
        assert_eq!(unsafe { value.interval_value.day_second.fraction }, 123_456);
    }

    #[test]
    fn interval_second_high_scale_no_overflow() {
        use crate::conversion::warning::Warning;
        let sn = make_decimal(35, 38); // scale=35: remainder * 1_000_000 would overflow u128
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalSecond, &mut value, &mut str_len);
        let scale_factor = 10i128.pow(35);
        let raw = scale_factor + 1; // represents 1.00...001 (sub-microsecond trailing digit)
        let warnings = sn.write_odbc_type(raw, &binding, &mut None).unwrap();
        assert!(warnings.contains(&Warning::NumericValueTruncated));
        assert_eq!(unsafe { value.interval_value.day_second.second }, 1);
        assert_eq!(unsafe { value.interval_value.day_second.fraction }, 0);
    }

    #[test]
    fn interval_year_at_max_precision_9() {
        let sn = make_decimal(0, 38);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let mut binding = binding_for_interval(CDataType::IntervalYear, &mut value, &mut str_len);
        binding.datetime_interval_precision = Some(9);
        let warnings = sn
            .write_odbc_type(999_999_999i128, &binding, &mut None)
            .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(unsafe { value.interval_value.year_month.year }, 999_999_999);
    }

    #[test]
    fn interval_year_overflows_at_precision_9() {
        use crate::conversion::error::WriteOdbcError;
        let sn = make_decimal(0, 38);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let mut binding = binding_for_interval(CDataType::IntervalYear, &mut value, &mut str_len);
        binding.datetime_interval_precision = Some(9);
        let result = sn.write_odbc_type(1_000_000_000i128, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::IntervalFieldOverflow { .. }
        ));
    }

    // ========================================================================
    // Interval overflow -- single-field (SQLSTATE 22015)
    // ========================================================================

    #[test]
    fn interval_year_overflow_default_precision() {
        use crate::conversion::error::WriteOdbcError;
        let sn = make_decimal(0, 38);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalYear, &mut value, &mut str_len);
        let result = sn.write_odbc_type(100i128, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::IntervalFieldOverflow { .. }
        ));
    }

    #[test]
    fn interval_second_overflow_default_precision() {
        use crate::conversion::error::WriteOdbcError;
        let sn = make_decimal(0, 38);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalSecond, &mut value, &mut str_len);
        let result = sn.write_odbc_type(100i128, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::IntervalFieldOverflow { .. }
        ));
    }

    // ========================================================================
    // Interval leading precision -- SQL_DESC_DATETIME_INTERVAL_PRECISION
    // ========================================================================

    #[test]
    fn interval_year_default_precision_allows_99() {
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalYear, &mut value, &mut str_len);
        let warnings = sn.write_odbc_type(99i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(unsafe { value.interval_value.year_month.year }, 99);
    }

    #[test]
    fn interval_year_default_precision_rejects_100() {
        use crate::conversion::error::WriteOdbcError;
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let binding = binding_for_interval(CDataType::IntervalYear, &mut value, &mut str_len);
        let result = sn.write_odbc_type(100i128, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::IntervalFieldOverflow { .. }
        ));
    }

    #[test]
    fn interval_day_precision_5_allows_99999() {
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let mut binding = binding_for_interval(CDataType::IntervalDay, &mut value, &mut str_len);
        binding.datetime_interval_precision = Some(5);
        let warnings = sn.write_odbc_type(99_999i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(unsafe { value.interval_value.day_second.day }, 99_999);
    }

    #[test]
    fn interval_day_precision_5_rejects_100000() {
        use crate::conversion::error::WriteOdbcError;
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let mut binding = binding_for_interval(CDataType::IntervalDay, &mut value, &mut str_len);
        binding.datetime_interval_precision = Some(5);
        let result = sn.write_odbc_type(100_000i128, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::IntervalFieldOverflow { .. }
        ));
    }

    #[test]
    fn interval_hour_precision_1_allows_9() {
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let mut binding = binding_for_interval(CDataType::IntervalHour, &mut value, &mut str_len);
        binding.datetime_interval_precision = Some(1);
        let warnings = sn.write_odbc_type(9i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(unsafe { value.interval_value.day_second.hour }, 9);
    }

    #[test]
    fn interval_hour_precision_1_rejects_10() {
        use crate::conversion::error::WriteOdbcError;
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let mut binding = binding_for_interval(CDataType::IntervalHour, &mut value, &mut str_len);
        binding.datetime_interval_precision = Some(1);
        let result = sn.write_odbc_type(10i128, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::IntervalFieldOverflow { .. }
        ));
    }

    #[test]
    fn interval_second_precision_3_allows_999() {
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let mut binding = binding_for_interval(CDataType::IntervalSecond, &mut value, &mut str_len);
        binding.datetime_interval_precision = Some(3);
        let warnings = sn.write_odbc_type(999i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(unsafe { value.interval_value.day_second.second }, 999);
    }

    #[test]
    fn interval_second_precision_3_rejects_1000() {
        use crate::conversion::error::WriteOdbcError;
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let mut binding = binding_for_interval(CDataType::IntervalSecond, &mut value, &mut str_len);
        binding.datetime_interval_precision = Some(3);
        let result = sn.write_odbc_type(1000i128, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::IntervalFieldOverflow { .. }
        ));
    }

    #[test]
    fn interval_minute_precision_0_allows_zero_only() {
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let mut binding = binding_for_interval(CDataType::IntervalMinute, &mut value, &mut str_len);
        binding.datetime_interval_precision = Some(0);
        let warnings = sn.write_odbc_type(0i128, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(unsafe { value.interval_value.day_second.minute }, 0);
    }

    #[test]
    fn interval_minute_precision_0_rejects_1() {
        use crate::conversion::error::WriteOdbcError;
        let sn = make_decimal(0, 10);
        let mut value = zero_interval();
        let mut str_len: sql::Len = 0;
        let mut binding = binding_for_interval(CDataType::IntervalMinute, &mut value, &mut str_len);
        binding.datetime_interval_precision = Some(0);
        let result = sn.write_odbc_type(1i128, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::IntervalFieldOverflow { .. }
        ));
    }

    // ========================================================================
    // Multi-field intervals -- always 22015
    // ========================================================================

    macro_rules! interval_multi_field_error_tests {
        ($($name:ident: $c_type:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    use crate::conversion::error::WriteOdbcError;
                    let sn = make_decimal(0, 10);
                    let mut value = zero_interval();
                    let mut str_len: sql::Len = 0;
                    let binding = binding_for_interval($c_type, &mut value, &mut str_len);
                    let result = sn.write_odbc_type(42i128, &binding, &mut None);
                    assert!(matches!(
                        result.unwrap_err(),
                        WriteOdbcError::IntervalFieldOverflow { .. }
                    ));
                }
            )*
        };
    }

    interval_multi_field_error_tests! {
        interval_year_to_month_errors:    CDataType::IntervalYearToMonth;
        interval_day_to_hour_errors:      CDataType::IntervalDayToHour;
        interval_day_to_minute_errors:    CDataType::IntervalDayToMinute;
        interval_day_to_second_errors:    CDataType::IntervalDayToSecond;
        interval_hour_to_minute_errors:   CDataType::IntervalHourToMinute;
        interval_hour_to_second_errors:   CDataType::IntervalHourToSecond;
        interval_minute_to_second_errors: CDataType::IntervalMinuteToSecond;
    }

    // ========================================================================
    // Nullable NULL handling (SQLSTATE 22002)
    // When the value is SQL NULL and no indicator pointer is provided,
    // the driver must return an IndicatorVariableRequired error.
    // ========================================================================

    mod nullable_null_tests {
        use super::*;
        use crate::api::SQL_NULL_DATA;
        use crate::conversion::WriteODBCType;
        use crate::conversion::error::WriteOdbcError;
        use crate::conversion::nullable::Nullable;

        #[test]
        fn null_with_indicator_writes_sql_null_data() {
            let nullable = Nullable {
                value: make_decimal(0, 10),
            };
            let mut value: i32 = 42;
            let mut str_len: sql::Len = 0;
            let binding = binding_for_value(CDataType::SLong, &mut value, &mut str_len);

            let warnings = nullable
                .write_odbc_type(None::<i128>, &binding, &mut None)
                .unwrap();

            assert!(warnings.is_empty());
            assert_eq!(str_len, SQL_NULL_DATA);
        }

        #[test]
        fn null_without_indicator_returns_error() {
            let nullable = Nullable {
                value: make_decimal(0, 10),
            };
            let mut value: i32 = 42;
            let binding = Binding {
                target_type: CDataType::SLong,
                target_value_ptr: &mut value as *mut i32 as sql::Pointer,
                ..Default::default()
            };

            let result = nullable.write_odbc_type(None::<i128>, &binding, &mut None);

            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                WriteOdbcError::IndicatorRequired { .. }
            ));
        }

        #[test]
        fn non_null_with_null_indicator_still_writes_value() {
            let nullable = Nullable {
                value: make_decimal(0, 10),
            };
            let mut value: i32 = 0;
            let binding = Binding {
                target_type: CDataType::SLong,
                target_value_ptr: &mut value as *mut i32 as sql::Pointer,
                ..Default::default()
            };

            let warnings = nullable
                .write_odbc_type(Some(42i128), &binding, &mut None)
                .unwrap();

            assert!(warnings.is_empty());
            assert_eq!(value, 42);
        }
    }

    // ========================================================================
    // Unsupported target type returns UnsupportedOdbcType error
    // ========================================================================

    #[test]
    fn unsupported_target_type_returns_error() {
        use crate::conversion::error::WriteOdbcError;
        let sn = make_decimal(0, 10);
        let mut value = [0u8; 16];
        let mut str_len: sql::Len = 0;
        let binding = Binding {
            target_type: CDataType::Guid,
            target_value_ptr: value.as_mut_ptr() as sql::Pointer,
            buffer_length: value.len() as sql::Len,
            octet_length_ptr: &mut str_len as *mut sql::Len,
            indicator_ptr: &mut str_len as *mut sql::Len,
            ..Default::default()
        };
        let result = sn.write_odbc_type(42i128, &binding, &mut None);
        assert!(matches!(
            result.unwrap_err(),
            WriteOdbcError::UnsupportedOdbcType { .. }
        ));
    }
}
