#[cfg(test)]
mod tests {
    use crate::api::CDataType;
    use crate::conversion::WriteODBCType;
    use crate::conversion::binary::SnowflakeBinary;
    use crate::conversion::test_utils::helpers::{
        binding_for_char_buffer, binding_for_wchar_buffer,
    };
    use crate::conversion::warning::Warning;
    use odbc_sys as sql;

    fn sn() -> SnowflakeBinary {
        SnowflakeBinary { len: 8_388_608 }
    }

    // ========================================================================
    // ReadArrowType — reading from GenericByteArray<GenericBinaryType<i32>>
    // ========================================================================

    #[test]
    fn read_arrow_binary_value() {
        use crate::conversion::ReadArrowType;
        use arrow::array::BinaryArray;
        let sn = sn();
        let array = BinaryArray::from(vec![Some(&[0x48, 0x65, 0x6C][..])]);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert_eq!(value, &[0x48, 0x65, 0x6C]);
    }

    #[test]
    fn read_arrow_empty_binary() {
        use crate::conversion::ReadArrowType;
        use arrow::array::BinaryArray;
        let sn = sn();
        let array = BinaryArray::from(vec![Some(&[][..])]);
        let value = sn.read_arrow_type(&array, 0).unwrap();
        assert!(value.is_empty());
    }

    #[test]
    fn read_arrow_null_returns_error() {
        use crate::conversion::ReadArrowType;
        use crate::conversion::error::ReadArrowError;
        use arrow::array::BinaryArray;
        let sn = sn();
        let array = BinaryArray::from(vec![None::<&[u8]>]);
        let result = sn.read_arrow_type(&array, 0);
        assert!(matches!(result, Err(ReadArrowError::NullValue { .. })));
    }

    // ========================================================================
    // CDataType::Default / CDataType::Binary — raw bytes
    // ========================================================================

    #[test]
    fn binary_raw_bytes() {
        let sn = sn();
        let input: &[u8] = &[0x48, 0x65, 0x6C, 0x6C, 0x6F];
        let mut buffer = vec![0u8; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Binary, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 5);
        assert_eq!(&buffer[..5], input);
    }

    #[test]
    fn default_maps_to_binary() {
        let sn = sn();
        let input: &[u8] = &[0xCA, 0xFE];
        let mut buffer = vec![0u8; 16];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Default, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 2);
        assert_eq!(&buffer[..2], &[0xCA, 0xFE]);
    }

    #[test]
    fn binary_empty_input() {
        let sn = sn();
        let input: &[u8] = &[];
        let mut buffer = vec![0xFFu8; 8];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Binary, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 0);
        assert_eq!(buffer[0], 0xFF);
    }

    #[test]
    fn binary_exact_fit_buffer() {
        let sn = sn();
        let input: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05];
        let mut buffer = vec![0u8; 5];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Binary, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 5);
        assert_eq!(&buffer[..5], input);
    }

    #[test]
    fn binary_truncation_small_buffer() {
        let sn = sn();
        let input: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05];
        let mut buffer = vec![0u8; 3];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Binary, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
        assert_eq!(&buffer[..3], &[0x01, 0x02, 0x03]);
        assert_eq!(str_len, 5);
    }

    #[test]
    fn binary_chunked_retrieval() {
        let sn = sn();
        let input: &[u8] = &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE];
        let mut buffer = vec![0u8; 3];
        let mut str_len: sql::Len = 0;
        let mut offset: Option<usize> = None;

        let binding = binding_for_char_buffer(CDataType::Binary, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut offset).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
        assert_eq!(&buffer[..3], &[0xAA, 0xBB, 0xCC]);
        assert_eq!(str_len, 5);
        assert_eq!(offset, Some(3));

        buffer.fill(0);
        str_len = 0;
        let binding = binding_for_char_buffer(CDataType::Binary, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut offset).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(&buffer[..2], &[0xDD, 0xEE]);
        assert_eq!(str_len, 2);
        assert_eq!(offset, None);
    }

    #[test]
    fn binary_three_chunk_retrieval() {
        let sn = sn();
        let input: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let mut buffer = vec![0u8; 3];
        let mut str_len: sql::Len = 0;
        let mut offset: Option<usize> = None;

        // Chunk 1: bytes 0..3
        let binding = binding_for_char_buffer(CDataType::Binary, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut offset).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
        assert_eq!(&buffer[..3], &[0x01, 0x02, 0x03]);
        assert_eq!(str_len, 8);
        assert_eq!(offset, Some(3));

        // Chunk 2: bytes 3..6
        buffer.fill(0);
        str_len = 0;
        let binding = binding_for_char_buffer(CDataType::Binary, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut offset).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
        assert_eq!(&buffer[..3], &[0x04, 0x05, 0x06]);
        assert_eq!(str_len, 5);
        assert_eq!(offset, Some(6));

        // Chunk 3: bytes 6..8
        buffer.fill(0);
        str_len = 0;
        let binding = binding_for_char_buffer(CDataType::Binary, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut offset).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(&buffer[..2], &[0x07, 0x08]);
        assert_eq!(str_len, 2);
        assert_eq!(offset, None);
    }

    // ========================================================================
    // CDataType::Char — uppercase hex encoding
    // ========================================================================

    #[test]
    fn char_hex_encoding() {
        let sn = sn();
        let input: &[u8] = &[0x48, 0x65, 0x6C];
        let mut buffer = vec![0u8; 16];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 6);
        assert_eq!(&buffer[..6], b"48656C");
        assert_eq!(buffer[6], 0);
    }

    #[test]
    fn char_hex_empty_binary() {
        let sn = sn();
        let input: &[u8] = &[];
        let mut buffer = vec![0xFFu8; 8];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 0);
        assert_eq!(buffer[0], 0);
    }

    #[test]
    fn char_hex_exact_fit_buffer() {
        let sn = sn();
        let input: &[u8] = &[0xAB, 0xCD, 0xEF];
        let mut buffer = vec![0u8; 7];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 6);
        assert_eq!(&buffer[..6], b"ABCDEF");
        assert_eq!(buffer[6], 0);
    }

    #[test]
    fn char_hex_truncation() {
        let sn = sn();
        let input: &[u8] = &[0xAB, 0xCD, 0xEF];
        let mut buffer = vec![0u8; 4];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
        assert_eq!(&buffer[..3], b"ABC");
        assert_eq!(buffer[3], 0);
        assert_eq!(str_len, 6);
    }

    #[test]
    fn char_hex_chunked_retrieval() {
        let sn = sn();
        let input: &[u8] = &[0xAB, 0xCD, 0xEF];
        let mut buffer = vec![0u8; 4];
        let mut str_len: sql::Len = 0;
        let mut offset: Option<usize> = None;

        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut offset).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
        assert_eq!(&buffer[..3], b"ABC");
        assert_eq!(str_len, 6);
        assert_eq!(offset, Some(3));

        buffer.fill(0);
        str_len = 0;
        let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut offset).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(&buffer[..3], b"DEF");
        assert_eq!(buffer[3], 0);
        assert_eq!(str_len, 3);
        assert_eq!(offset, None);
    }

    #[test]
    fn char_hex_single_byte_values() {
        let sn = sn();
        for (byte, expected) in [(0x00u8, "00"), (0xFF, "FF"), (0x0A, "0A")] {
            let input: &[u8] = &[byte];
            let mut buffer = vec![0u8; 8];
            let mut str_len: sql::Len = 0;
            let binding = binding_for_char_buffer(CDataType::Char, &mut buffer, &mut str_len);
            let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
            assert!(warnings.is_empty());
            assert_eq!(str_len, 2);
            assert_eq!(&buffer[..2], expected.as_bytes());
        }
    }

    // ========================================================================
    // CDataType::WChar — uppercase hex encoding (wide chars)
    // ========================================================================

    #[test]
    fn wchar_hex_encoding() {
        let sn = sn();
        let input: &[u8] = &[0xAB, 0xCD];
        let mut buffer = vec![0u16; 16];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 8);
        assert_eq!(buffer[0], 'A' as u16);
        assert_eq!(buffer[1], 'B' as u16);
        assert_eq!(buffer[2], 'C' as u16);
        assert_eq!(buffer[3], 'D' as u16);
        assert_eq!(buffer[4], 0);
    }

    #[test]
    fn wchar_hex_empty_binary() {
        let sn = sn();
        let input: &[u8] = &[];
        let mut buffer = vec![0xFFFFu16; 8];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 0);
        assert_eq!(buffer[0], 0);
    }

    #[test]
    fn wchar_hex_single_byte_values() {
        let sn = sn();
        for (byte, expected_u16) in [
            (0x00u8, ['0' as u16, '0' as u16]),
            (0xFF, ['F' as u16, 'F' as u16]),
            (0x0A, ['0' as u16, 'A' as u16]),
        ] {
            let input: &[u8] = &[byte];
            let mut buffer = vec![0u16; 8];
            let mut str_len: sql::Len = 0;
            let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
            let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
            assert!(warnings.is_empty());
            assert_eq!(str_len, 4);
            assert_eq!(buffer[0], expected_u16[0]);
            assert_eq!(buffer[1], expected_u16[1]);
            assert_eq!(buffer[2], 0);
        }
    }

    #[test]
    fn wchar_hex_exact_fit_buffer() {
        let sn = sn();
        let input: &[u8] = &[0x01, 0xFF];
        let mut buffer = vec![0u16; 5];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 8);
        assert_eq!(buffer[0], '0' as u16);
        assert_eq!(buffer[1], '1' as u16);
        assert_eq!(buffer[2], 'F' as u16);
        assert_eq!(buffer[3], 'F' as u16);
        assert_eq!(buffer[4], 0);
    }

    #[test]
    fn wchar_hex_truncation() {
        let sn = sn();
        let input: &[u8] = &[0xAB, 0xCD, 0xEF];
        let mut buffer = vec![0u16; 3];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut None).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
        assert_eq!(buffer[0], 'A' as u16);
        assert_eq!(buffer[1], 'B' as u16);
        assert_eq!(buffer[2], 0);
        assert_eq!(str_len, 12);
    }

    #[test]
    fn wchar_hex_chunked_retrieval() {
        let sn = sn();
        let input: &[u8] = &[0xAB, 0xCD];
        let mut buffer = vec![0u16; 3];
        let mut str_len: sql::Len = 0;
        let mut offset: Option<usize> = None;

        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut offset).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
        assert_eq!(buffer[0], 'A' as u16);
        assert_eq!(buffer[1], 'B' as u16);
        assert_eq!(str_len, 8);
        assert_eq!(offset, Some(2));

        buffer.fill(0);
        str_len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn.write_odbc_type(input, &binding, &mut offset).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(buffer[0], 'C' as u16);
        assert_eq!(buffer[1], 'D' as u16);
        assert_eq!(buffer[2], 0);
        assert_eq!(str_len, 4);
        assert_eq!(offset, None);
    }

    // ========================================================================
    // Unsupported target type returns error
    // ========================================================================

    #[test]
    fn unsupported_type_returns_error() {
        use crate::conversion::error::WriteOdbcError;
        use crate::conversion::test_utils::helpers::binding_for_value;
        let sn = sn();
        let input: &[u8] = &[0x01];
        let mut value: u8 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::TypeDate, &mut value, &mut str_len);
        let err = sn.write_odbc_type(input, &binding, &mut None).unwrap_err();
        assert!(
            matches!(err, WriteOdbcError::UnsupportedOdbcType { target_type, .. } if target_type == CDataType::TypeDate),
            "expected UnsupportedOdbcType for TypeDate, got: {err}"
        );
    }

    // ========================================================================
    // Metadata
    // ========================================================================

    #[test]
    fn sql_type_is_ext_var_binary() {
        let sn = sn();
        assert_eq!(sn.sql_type(), sql::SqlDataType::EXT_VAR_BINARY);
    }

    #[test]
    fn column_size_returns_len() {
        let sn = SnowflakeBinary { len: 1024 };
        assert_eq!(sn.column_size(), 1024);
    }

    #[test]
    fn decimal_digits_is_0() {
        let sn = sn();
        assert_eq!(sn.decimal_digits(), 0);
    }

    // ========================================================================
    // from_field metadata parsing (via column_size_from_field)
    // ========================================================================

    fn binary_field(metadata: Vec<(&str, &str)>) -> arrow::datatypes::Field {
        let md: std::collections::HashMap<String, String> = metadata
            .into_iter()
            .chain(std::iter::once(("logicalType", "BINARY")))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        arrow::datatypes::Field::new("col", arrow::datatypes::DataType::Binary, true)
            .with_metadata(md)
    }

    #[test]
    fn from_field_uses_byte_length_metadata() {
        use crate::conversion::{NumericSettings, column_size_from_field};
        let field = binary_field(vec![("byteLength", "1024")]);
        let size = column_size_from_field(&field, &NumericSettings::default()).unwrap();
        assert_eq!(size, 1024);
    }

    #[test]
    fn from_field_defaults_when_byte_length_missing() {
        use crate::conversion::{NumericSettings, column_size_from_field};
        let field = binary_field(vec![]);
        let size = column_size_from_field(&field, &NumericSettings::default()).unwrap();
        assert_eq!(size, 8_388_608);
    }

    #[test]
    fn from_field_errors_on_unparseable_byte_length() {
        use crate::conversion::error::ConversionError;
        use crate::conversion::{NumericSettings, column_size_from_field};
        let field = binary_field(vec![("byteLength", "not_a_number")]);
        let err = column_size_from_field(&field, &NumericSettings::default()).unwrap_err();
        assert!(
            matches!(err, ConversionError::FieldMetadataParsing { ref key, .. } if key == "byteLength"),
            "expected FieldMetadataParsing for byteLength, got: {err}"
        );
    }
}
