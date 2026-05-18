#[cfg(test)]
mod tests {
    use crate::conversion::error::ConversionError;
    use crate::conversion::{Binding, NumericSettings, make_converter};
    use arrow::array::{ArrayRef, Int32Array, Int64Array};
    use arrow::datatypes::{DataType, Field};
    use std::collections::HashMap;
    use std::sync::Arc;

    fn boolean_field() -> Field {
        let md: HashMap<String, String> = [("logicalType", "BOOLEAN")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Field::new("col", DataType::Boolean, true).with_metadata(md)
    }

    fn time_field(scale: u32, data_type: DataType) -> Field {
        let md: HashMap<String, String> = [
            ("logicalType".to_string(), "TIME".to_string()),
            ("scale".to_string(), scale.to_string()),
        ]
        .into_iter()
        .collect();
        Field::new("col", data_type, true).with_metadata(md)
    }

    /// Cached converters now downcast at `convert_arrow_value` time, so a
    /// mismatched array must surface as `ArrowArrayDowncast` rather than UB.
    #[test]
    fn convert_arrow_value_returns_downcast_error_on_type_mismatch() {
        let field = boolean_field();
        let ns = NumericSettings::default();
        let converter = make_converter(&field, &ns).expect("converter for BOOLEAN");

        let wrong_array: ArrayRef = Arc::new(Int64Array::from(vec![Some(1i64)]));
        let binding = Binding::default();

        let err = converter
            .convert_arrow_value(wrong_array.as_ref(), 0, &binding, &mut None)
            .expect_err("expected ArrowArrayDowncast for Int64 array given BOOLEAN converter");

        match err {
            ConversionError::ArrowArrayDowncast { expected_type, .. } => {
                assert!(
                    expected_type.contains("BooleanArray"),
                    "expected_type should identify BooleanArray, got: {expected_type}"
                );
            }
            other => panic!("expected ArrowArrayDowncast, got: {other:?}"),
        }
    }

    /// TIME columns with scale ≤ 4 arrive as Int32-backed arrays; scale ≥ 5
    /// arrive as Int64. The converter must select its downcast target from
    /// the Arrow field's data type rather than hardcoding one width.
    #[test]
    fn time_int32_field_accepts_int32_array() {
        let field = time_field(0, DataType::Int32);
        let ns = NumericSettings::default();
        let converter = make_converter(&field, &ns).expect("converter for TIME(0) Int32");

        let array: ArrayRef = Arc::new(Int32Array::from(vec![Some(45_296)])); // 12:34:56
        let mut buffer = vec![0u8; 16];
        let mut str_len: odbc_sys::Len = 0;
        let binding = crate::conversion::test_utils::helpers::binding_for_char_buffer(
            crate::api::CDataType::Char,
            &mut buffer,
            &mut str_len,
        );

        converter
            .convert_arrow_value(array.as_ref(), 0, &binding, &mut None)
            .expect("Int32-backed TIME column must convert without downcast errors");
        assert_eq!(&buffer[..8], b"12:34:56");
    }

    #[test]
    fn time_int64_field_accepts_int64_array() {
        let field = time_field(9, DataType::Int64);
        let ns = NumericSettings::default();
        let converter = make_converter(&field, &ns).expect("converter for TIME(9) Int64");

        let array: ArrayRef = Arc::new(Int64Array::from(vec![Some(45_296_123_456_789i64)])); // 12:34:56.123456789
        let mut buffer = vec![0u8; 32];
        let mut str_len: odbc_sys::Len = 0;
        let binding = crate::conversion::test_utils::helpers::binding_for_char_buffer(
            crate::api::CDataType::Char,
            &mut buffer,
            &mut str_len,
        );

        converter
            .convert_arrow_value(array.as_ref(), 0, &binding, &mut None)
            .expect("Int64-backed TIME column must convert");
        assert_eq!(&buffer[..18], b"12:34:56.123456789");
    }

    #[test]
    fn time_unsupported_data_type_fails_converter_construction() {
        let field = time_field(0, DataType::Float32);
        let ns = NumericSettings::default();
        let err = match make_converter(&field, &ns) {
            Ok(_) => panic!("Float32 must not be accepted as a TIME backing array"),
            Err(e) => e,
        };
        assert!(matches!(
            err,
            ConversionError::UnsupportedArrowDataType { .. }
        ));
    }
}
