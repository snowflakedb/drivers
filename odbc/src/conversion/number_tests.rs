#[cfg(test)]
mod tests {
    use crate::cdata_types::CDataType;
    use crate::conversion::WriteODBCType;
    use crate::conversion::number::{NumericSqlType, SnowflakeNumber};
    use crate::conversion::traits::Binding;
    use odbc_sys as sql;

    // ======================================================================
    // Test helpers
    // ======================================================================

    /// Helper: create a Binding for a fixed-size C type (no string buffer).
    fn binding_for_value<T>(
        target_type: CDataType,
        value: &mut T,
        str_len: &mut sql::Len,
    ) -> Binding {
        Binding {
            target_type,
            value: value as *mut T as sql::Pointer,
            buffer_length: 0,
            str_len_or_ind_ptr: str_len as *mut sql::Len,
        }
    }

    /// Helper: create a Binding for a character buffer.
    fn binding_for_char_buffer(
        target_type: CDataType,
        buffer: &mut [u8],
        str_len: &mut sql::Len,
    ) -> Binding {
        Binding {
            target_type,
            value: buffer.as_mut_ptr() as sql::Pointer,
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

    // ======================================================================
    // NumericSqlType::default_c_type
    // ======================================================================

    #[test]
    fn decimal_default_c_type_is_char() {
        assert_eq!(NumericSqlType::Decimal.default_c_type(), CDataType::Char);
    }

    // ======================================================================
    // SQL_DECIMAL + CDataType::Default  →  SQL_C_CHAR
    // ======================================================================

    #[test]
    fn decimal_default_writes_integer_as_char() {
        let sn = make_decimal(0, 10);
        let mut buffer = vec![0u8; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Default, &mut buffer, &mut str_len);

        sn.write_odbc_type(42, &binding).unwrap();

        assert_eq!(str_len, 2);
        assert_eq!(&buffer[..2], b"42");
        assert_eq!(buffer[2], 0);
    }

    #[test]
    fn decimal_default_writes_scaled_value_as_char() {
        let sn = make_decimal(2, 10);
        let mut buffer = vec![0u8; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Default, &mut buffer, &mut str_len);

        sn.write_odbc_type(12345, &binding).unwrap();

        assert_eq!(str_len, 6);
        assert_eq!(&buffer[..6], b"123.45");
        assert_eq!(buffer[6], 0);
    }

    #[test]
    fn decimal_default_writes_negative_scaled_value_as_char() {
        let sn = make_decimal(3, 10);
        let mut buffer = vec![0u8; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Default, &mut buffer, &mut str_len);

        sn.write_odbc_type(-50, &binding).unwrap();

        assert_eq!(str_len, 6);
        assert_eq!(&buffer[..6], b"-0.050");
        assert_eq!(buffer[6], 0);
    }

    #[test]
    fn decimal_default_writes_zero_as_char() {
        let sn = make_decimal(0, 10);
        let mut buffer = vec![0u8; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Default, &mut buffer, &mut str_len);

        sn.write_odbc_type(0, &binding).unwrap();

        assert_eq!(str_len, 1);
        assert_eq!(&buffer[..1], b"0");
        assert_eq!(buffer[1], 0);
    }

    // ======================================================================
    // Explicit C types work regardless of NumericSqlType
    // ======================================================================

    #[test]
    fn decimal_explicit_slong_writes_i32() {
        let sn = make_decimal(0, 10);
        let mut value: i32 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::SLong, &mut value, &mut str_len);

        sn.write_odbc_type(42, &binding).unwrap();

        assert_eq!(value, 42);
    }

    #[test]
    fn decimal_explicit_sbigint_writes_i64() {
        let sn = make_decimal(0, 10);
        let mut value: i64 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::SBigInt, &mut value, &mut str_len);

        sn.write_odbc_type(123456789, &binding).unwrap();

        assert_eq!(value, 123456789i64);
    }

    #[test]
    fn decimal_explicit_double_writes_f64() {
        let sn = make_decimal(0, 10);
        let mut value: f64 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Double, &mut value, &mut str_len);

        sn.write_odbc_type(42, &binding).unwrap();

        assert!((value - 42.0).abs() < f64::EPSILON);
    }
}
