#[cfg(test)]
mod tests {
    use crate::conversion::error::ConversionError;
    use crate::conversion::{Binding, NumericSettings, make_converter};
    use arrow::array::{ArrayRef, Int64Array};
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
}
