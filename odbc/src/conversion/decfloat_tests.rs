#[cfg(test)]
mod tests {
    use crate::api::CDataType;
    use crate::conversion::WriteODBCType;
    use crate::conversion::decfloat::{
        SnowflakeDecfloat, format_decfloat, i128_from_big_endian_signed,
    };
    use crate::conversion::error::ReadArrowError;
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
    // WriteODBCType — unsupported types
    // ======================================================================

    #[test]
    fn write_unsupported_type_returns_error() {
        let df = decfloat();
        let mut value: f64 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = Binding {
            target_type: CDataType::Double,
            target_value_ptr: &mut value as *mut f64 as sql::Pointer,
            buffer_length: 0,
            octet_length_ptr: &mut str_len as *mut sql::Len,
            indicator_ptr: &mut str_len as *mut sql::Len,
            ..Default::default()
        };

        assert!(df.write_odbc_type((123, 0), &binding, &mut None).is_err());
    }

    #[test]
    fn write_unsupported_numeric_type_returns_error() {
        let df = decfloat();
        let mut value = sql::Numeric {
            precision: 0,
            scale: 0,
            sign: 0,
            val: [0; 16],
        };
        let mut str_len: sql::Len = 0;
        let binding = Binding {
            target_type: CDataType::Numeric,
            target_value_ptr: &mut value as *mut sql::Numeric as sql::Pointer,
            buffer_length: 0,
            octet_length_ptr: &mut str_len as *mut sql::Len,
            indicator_ptr: &mut str_len as *mut sql::Len,
            ..Default::default()
        };

        assert!(df.write_odbc_type((123, 0), &binding, &mut None).is_err());
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
}
