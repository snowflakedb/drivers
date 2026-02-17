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
