#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::api::CDataType;
    use crate::api::encoding::{WIDE_CHAR_SIZE, WideChar};
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
        assert_eq!(value.as_ref(), &[0x48, 0x65, 0x6C]);
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
            .unwrap();
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
            .unwrap();
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
            .unwrap();
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
            .unwrap();
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
            .unwrap();
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut offset)
            .unwrap();
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut offset)
            .unwrap();
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut offset)
            .unwrap();
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut offset)
            .unwrap();
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut offset)
            .unwrap();
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
            .unwrap();
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
            .unwrap();
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
            .unwrap();
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
            .unwrap();
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut offset)
            .unwrap();
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
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut offset)
            .unwrap();
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
            let warnings = sn
                .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
                .unwrap();
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
        let mut buffer = vec![0 as WideChar; 16];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
            .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, (4 * WIDE_CHAR_SIZE) as sql::Len);
        assert_eq!(buffer[0], 'A' as WideChar);
        assert_eq!(buffer[1], 'B' as WideChar);
        assert_eq!(buffer[2], 'C' as WideChar);
        assert_eq!(buffer[3], 'D' as WideChar);
        assert_eq!(buffer[4], 0);
    }

    #[test]
    fn wchar_hex_empty_binary() {
        let sn = sn();
        let input: &[u8] = &[];
        let mut buffer = vec![0xFFFF as WideChar; 8];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
            .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, 0);
        assert_eq!(buffer[0], 0);
    }

    #[test]
    fn wchar_hex_single_byte_values() {
        let sn = sn();
        for (byte, expected) in [
            (0x00u8, ['0' as WideChar, '0' as WideChar]),
            (0xFF, ['F' as WideChar, 'F' as WideChar]),
            (0x0A, ['0' as WideChar, 'A' as WideChar]),
        ] {
            let input: &[u8] = &[byte];
            let mut buffer = vec![0 as WideChar; 8];
            let mut str_len: sql::Len = 0;
            let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
            let warnings = sn
                .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
                .unwrap();
            assert!(warnings.is_empty());
            assert_eq!(str_len, (2 * WIDE_CHAR_SIZE) as sql::Len);
            assert_eq!(buffer[0], expected[0]);
            assert_eq!(buffer[1], expected[1]);
            assert_eq!(buffer[2], 0);
        }
    }

    #[test]
    fn wchar_hex_exact_fit_buffer() {
        let sn = sn();
        let input: &[u8] = &[0x01, 0xFF];
        let mut buffer = vec![0 as WideChar; 5];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
            .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(str_len, (4 * WIDE_CHAR_SIZE) as sql::Len);
        assert_eq!(buffer[0], '0' as WideChar);
        assert_eq!(buffer[1], '1' as WideChar);
        assert_eq!(buffer[2], 'F' as WideChar);
        assert_eq!(buffer[3], 'F' as WideChar);
        assert_eq!(buffer[4], 0);
    }

    #[test]
    fn wchar_hex_truncation() {
        let sn = sn();
        let input: &[u8] = &[0xAB, 0xCD, 0xEF];
        let mut buffer = vec![0 as WideChar; 3];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
            .unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
        assert_eq!(buffer[0], 'A' as WideChar);
        assert_eq!(buffer[1], 'B' as WideChar);
        assert_eq!(buffer[2], 0);
        assert_eq!(str_len, (6 * WIDE_CHAR_SIZE) as sql::Len);
    }

    #[test]
    fn wchar_hex_chunked_retrieval() {
        let sn = sn();
        let input: &[u8] = &[0xAB, 0xCD];
        let mut buffer = vec![0 as WideChar; 3];
        let mut str_len: sql::Len = 0;
        let mut offset: Option<usize> = None;

        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut offset)
            .unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::StringDataTruncated))
        );
        assert_eq!(buffer[0], 'A' as WideChar);
        assert_eq!(buffer[1], 'B' as WideChar);
        assert_eq!(str_len, (4 * WIDE_CHAR_SIZE) as sql::Len);
        assert_eq!(offset, Some(2));

        buffer.fill(0);
        str_len = 0;
        let binding = binding_for_wchar_buffer(&mut buffer, &mut str_len);
        let warnings = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut offset)
            .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(buffer[0], 'C' as WideChar);
        assert_eq!(buffer[1], 'D' as WideChar);
        assert_eq!(buffer[2], 0);
        assert_eq!(str_len, (2 * WIDE_CHAR_SIZE) as sql::Len);
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
        let err = sn
            .write_odbc_type(Cow::Borrowed(input), &binding, &mut None)
            .unwrap_err();
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

    // ========================================================================
    // ReadODBC — bind path (SQLBindParameter source → BINARY SQL target)
    //
    // Per ODBC Appendix D the only legal C source types for SQL_BINARY /
    // SQL_VARBINARY / SQL_LONGVARBINARY are SQL_C_BINARY, SQL_C_CHAR,
    // SQL_C_WCHAR, and SQL_C_DEFAULT. Char/WChar inputs are ASCII hex
    // literals that the driver decodes into raw bytes. Everything else
    // must be rejected with SQLSTATE 07006 ("restricted data type
    // attribute violation").
    // ========================================================================

    use crate::api::ParameterBinding;
    use crate::conversion::error::JsonBindingError;
    use crate::conversion::traits::ReadODBC;

    fn binding(value_type: CDataType, ptr: sql::Pointer, buffer_len: sql::Len) -> ParameterBinding {
        ParameterBinding {
            sql_data_type: sql::SqlDataType::EXT_VAR_BINARY,
            value_type,
            parameter_value_ptr: ptr,
            buffer_length: buffer_len,
            str_len_or_ind_ptr: std::ptr::null_mut(),
            sf_subtype: None,
        }
    }

    fn binding_with_indicator(
        value_type: CDataType,
        ptr: sql::Pointer,
        buffer_len: sql::Len,
        ind: &mut sql::Len,
    ) -> ParameterBinding {
        ParameterBinding {
            sql_data_type: sql::SqlDataType::EXT_VAR_BINARY,
            value_type,
            parameter_value_ptr: ptr,
            buffer_length: buffer_len,
            str_len_or_ind_ptr: ind as *mut sql::Len,
            sf_subtype: None,
        }
    }

    #[test]
    fn read_odbc_binary_returns_buffer_verbatim() {
        let sn = sn();
        let buf: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
        let mut ind: sql::Len = 4;
        let b =
            binding_with_indicator(CDataType::Binary, buf.as_ptr() as sql::Pointer, 4, &mut ind);
        let result = sn.read_odbc(&b).unwrap();
        assert_eq!(result.as_ref(), &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn read_odbc_default_routes_to_binary() {
        // SQL_C_DEFAULT against a SQL_BINARY parameter must behave as
        // SQL_C_BINARY per ODBC Appendix D ("Default C Data Types").
        let sn = sn();
        let buf: [u8; 3] = [0xCA, 0xFE, 0x00];
        let mut ind: sql::Len = 3;
        let b = binding_with_indicator(
            CDataType::Default,
            buf.as_ptr() as sql::Pointer,
            3,
            &mut ind,
        );
        let result = sn.read_odbc(&b).unwrap();
        assert_eq!(result.as_ref(), &[0xCA, 0xFE, 0x00]);
    }

    #[test]
    fn read_odbc_char_decodes_uppercase_hex() {
        // Application sends "DEADBEEF" as 8 ASCII bytes; driver must
        // hex-decode into 4 bytes per ODBC Appendix D ("Converting Data
        // from C to SQL: Binary"). Without decoding, the legacy 3.16.0
        // driver and the universal driver would round-trip the same
        // 4 bytes round-trip; with the previous (broken) verbatim path
        // 8 bytes ended up in the BINARY column.
        let sn = sn();
        let buf = b"DEADBEEF";
        let mut ind: sql::Len = 8;
        let b = binding_with_indicator(
            CDataType::Char,
            buf.as_ptr() as sql::Pointer,
            buf.len() as sql::Len,
            &mut ind,
        );
        let result = sn.read_odbc(&b).unwrap();
        assert_eq!(result.as_ref(), &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn read_odbc_char_accepts_lowercase_and_mixed_case_hex() {
        // The ODBC spec doesn't pin the case of the hex literal; the
        // legacy driver accepts both. We mirror that and lowercase
        // produces the same bytes.
        let sn = sn();
        for input in [
            &b"deadbeef"[..],
            &b"DeAdBeEf"[..],
            &b"DEADBEEF"[..],
            &b"deadBEEF"[..],
        ] {
            let mut ind: sql::Len = input.len() as sql::Len;
            let b = binding_with_indicator(
                CDataType::Char,
                input.as_ptr() as sql::Pointer,
                input.len() as sql::Len,
                &mut ind,
            );
            let result = sn.read_odbc(&b).unwrap();
            assert_eq!(
                result.as_ref(),
                &[0xDE, 0xAD, 0xBE, 0xEF],
                "input={input:?}"
            );
        }
    }

    #[test]
    fn read_odbc_char_with_sql_nts_indicator() {
        // An SQL_NTS (-3) indicator means "null-terminated"; driver
        // must scan for the NUL terminator instead of using
        // buffer_length.
        let sn = sn();
        let buf = b"ABCD\0extra";
        let mut ind: sql::Len = sql::NTS;
        let b = binding_with_indicator(
            CDataType::Char,
            buf.as_ptr() as sql::Pointer,
            buf.len() as sql::Len,
            &mut ind,
        );
        let result = sn.read_odbc(&b).unwrap();
        assert_eq!(result.as_ref(), &[0xAB, 0xCD]);
    }

    #[test]
    fn read_odbc_char_empty_string_decodes_to_empty_bytes() {
        let sn = sn();
        let buf = b"";
        let mut ind: sql::Len = 0;
        let b = binding_with_indicator(CDataType::Char, buf.as_ptr() as sql::Pointer, 0, &mut ind);
        let result = sn.read_odbc(&b).unwrap();
        let empty: &[u8] = &[];
        assert_eq!(result.as_ref(), empty);
    }

    #[test]
    fn read_odbc_char_rejects_odd_length_hex() {
        // SQLSTATE 22018 — ODBC Appendix D requires an even number of
        // hex digits since each pair encodes one byte.
        let sn = sn();
        let buf = b"ABC"; // 3 chars
        let mut ind: sql::Len = 3;
        let b = binding_with_indicator(CDataType::Char, buf.as_ptr() as sql::Pointer, 3, &mut ind);
        let err = sn.read_odbc(&b).unwrap_err();
        assert!(
            matches!(err, JsonBindingError::InvalidHexLiteral { .. }),
            "expected InvalidHexLiteral, got: {err}"
        );
    }

    #[test]
    fn read_odbc_char_rejects_non_hex_chars() {
        // SQLSTATE 22018 — only [0-9A-Fa-f] are admissible.
        let sn = sn();
        let buf = b"ZZZZ";
        let mut ind: sql::Len = 4;
        let b = binding_with_indicator(CDataType::Char, buf.as_ptr() as sql::Pointer, 4, &mut ind);
        let err = sn.read_odbc(&b).unwrap_err();
        assert!(matches!(err, JsonBindingError::InvalidHexLiteral { .. }));
    }

    #[test]
    fn read_odbc_wchar_decodes_hex_after_utf16_transcode() {
        // SQL_C_WCHAR source is UTF-16; driver transcodes to UTF-8 first
        // (read_wchar_str) and then hex-decodes. End result is the same
        // 4 bytes as SQL_C_CHAR.
        let sn = sn();
        let units: [u16; 8] = [
            'D' as u16, 'E' as u16, 'A' as u16, 'D' as u16, 'B' as u16, 'E' as u16, 'E' as u16,
            'F' as u16,
        ];
        let mut ind: sql::Len = (units.len() * 2) as sql::Len;
        let b = binding_with_indicator(
            CDataType::WChar,
            units.as_ptr() as sql::Pointer,
            (units.len() * 2) as sql::Len,
            &mut ind,
        );
        let result = sn.read_odbc(&b).unwrap();
        assert_eq!(result.as_ref(), &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn read_odbc_wchar_rejects_invalid_hex() {
        let sn = sn();
        let units: [u16; 3] = ['A' as u16, 'B' as u16, 'X' as u16];
        let mut ind: sql::Len = 6;
        let b = binding_with_indicator(
            CDataType::WChar,
            units.as_ptr() as sql::Pointer,
            6,
            &mut ind,
        );
        let err = sn.read_odbc(&b).unwrap_err();
        assert!(matches!(err, JsonBindingError::InvalidHexLiteral { .. }));
    }

    #[test]
    fn read_odbc_rejects_every_disallowed_c_type_with_07006() {
        // ODBC Appendix D: only SQL_C_BINARY / SQL_C_CHAR / SQL_C_WCHAR /
        // SQL_C_DEFAULT may be bound to a binary SQL target. Every other
        // C type must surface as UnsupportedCDataType (→ SQLSTATE 07006
        // in `OdbcError::sql_state`). The previous implementation
        // silently hex-encoded whatever raw bytes were at the parameter
        // pointer, producing inflated `bindparam_to_binary` matrix
        // coverage and silently mangling source values.
        let sn = sn();
        let dummy: [u8; 32] = [0; 32];
        for &c_type in &[
            CDataType::Bit,
            CDataType::TinyInt,
            CDataType::STinyInt,
            CDataType::UTinyInt,
            CDataType::Short,
            CDataType::SShort,
            CDataType::UShort,
            CDataType::Long,
            CDataType::SLong,
            CDataType::ULong,
            CDataType::SBigInt,
            CDataType::UBigInt,
            CDataType::Float,
            CDataType::Double,
            CDataType::Numeric,
            CDataType::TypeDate,
            CDataType::TypeTime,
            CDataType::TypeTimestamp,
            CDataType::IntervalYear,
            CDataType::IntervalMonth,
            CDataType::IntervalDay,
            CDataType::IntervalHour,
            CDataType::IntervalMinute,
            CDataType::IntervalSecond,
            CDataType::IntervalYearToMonth,
            CDataType::IntervalDayToHour,
            CDataType::IntervalDayToMinute,
            CDataType::IntervalDayToSecond,
            CDataType::IntervalHourToMinute,
            CDataType::IntervalHourToSecond,
            CDataType::IntervalMinuteToSecond,
            CDataType::Guid,
        ] {
            let b = binding(
                c_type,
                dummy.as_ptr() as sql::Pointer,
                dummy.len() as sql::Len,
            );
            let err = sn.read_odbc(&b).unwrap_err();
            assert!(
                matches!(err, JsonBindingError::UnsupportedCDataType { c_type: c, .. } if c == c_type),
                "expected UnsupportedCDataType for {c_type:?}, got: {err}"
            );
        }
    }

    // ========================================================================
    // Bind-side end-to-end (ReadODBC -> WriteJson) — the legacy driver bug
    //
    // Pre-fix: SQL_C_CHAR "DEADBEEF" produced JSON value
    // "4445414442454546" (16 hex chars = ASCII codes of "DEADBEEF"),
    // storing 8 bytes in Snowflake instead of the intended 4. Post-fix:
    // the same input yields "deadbeef" (4 bytes server-side), matching
    // the legacy 3.16.0 driver's behaviour and the ODBC spec.
    // ========================================================================

    #[test]
    fn read_odbc_then_write_json_round_trips_char_hex_to_lowercase_string() {
        use crate::conversion::traits::WriteJson;

        let sn = sn();
        let buf = b"DEADBEEF";
        let mut ind: sql::Len = 8;
        let b = binding_with_indicator(
            CDataType::Char,
            buf.as_ptr() as sql::Pointer,
            buf.len() as sql::Len,
            &mut ind,
        );
        let bytes = sn.read_odbc(&b).unwrap();
        let json = sn.write_json(bytes).unwrap();
        assert_eq!(json, serde_json::Value::String("deadbeef".to_string()));
    }

    #[test]
    fn read_odbc_then_write_json_round_trips_binary_to_lowercase_hex() {
        use crate::conversion::traits::WriteJson;

        let sn = sn();
        let buf: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
        let mut ind: sql::Len = 4;
        let b =
            binding_with_indicator(CDataType::Binary, buf.as_ptr() as sql::Pointer, 4, &mut ind);
        let bytes = sn.read_odbc(&b).unwrap();
        let json = sn.write_json(bytes).unwrap();
        assert_eq!(json, serde_json::Value::String("deadbeef".to_string()));
    }
}
