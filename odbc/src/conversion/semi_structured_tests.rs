#[cfg(test)]
mod tests {
    use crate::conversion::error::ConversionError;
    use crate::conversion::{
        NumericSettings, SF_DEFAULT_VARCHAR_MAX_LEN, column_size_from_field,
        decimal_digits_from_field, make_converter, sql_type_from_field,
    };
    use arrow::array::GenericByteArray;
    use arrow::datatypes::{Field, Utf8Type};
    use odbc_sys as sql;
    use std::collections::HashMap;

    fn semi_structured_field(logical_type: &str, extra_metadata: Vec<(&str, &str)>) -> Field {
        let md: HashMap<String, String> = extra_metadata
            .into_iter()
            .chain(std::iter::once(("logicalType", logical_type)))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Field::new("col", arrow::datatypes::DataType::Utf8, true).with_metadata(md)
    }

    #[test]
    fn object_maps_to_sql_varchar() {
        let field = semi_structured_field("OBJECT", vec![]);
        let ns = NumericSettings::default();
        assert_eq!(
            sql_type_from_field(&field, &ns).unwrap(),
            sql::SqlDataType::VARCHAR
        );
    }

    #[test]
    fn array_maps_to_sql_varchar() {
        let field = semi_structured_field("ARRAY", vec![]);
        let ns = NumericSettings::default();
        assert_eq!(
            sql_type_from_field(&field, &ns).unwrap(),
            sql::SqlDataType::VARCHAR
        );
    }

    #[test]
    fn variant_maps_to_sql_varchar() {
        let field = semi_structured_field("VARIANT", vec![]);
        let ns = NumericSettings::default();
        assert_eq!(
            sql_type_from_field(&field, &ns).unwrap(),
            sql::SqlDataType::VARCHAR
        );
    }

    #[test]
    fn defaults_column_size_to_16mb() {
        for lt in &["OBJECT", "ARRAY", "VARIANT"] {
            let field = semi_structured_field(lt, vec![]);
            let ns = NumericSettings::default();
            assert_eq!(
                column_size_from_field(&field, &ns).unwrap(),
                SF_DEFAULT_VARCHAR_MAX_LEN as sql::ULen,
                "column_size mismatch for {lt}"
            );
        }
    }

    #[test]
    fn uses_configured_varchar_max_when_char_length_missing() {
        for lt in &["OBJECT", "ARRAY", "VARIANT"] {
            let field = semi_structured_field(lt, vec![]);
            let ns = NumericSettings {
                max_varchar_size: 134_217_728,
                ..NumericSettings::default()
            };
            assert_eq!(
                column_size_from_field(&field, &ns).unwrap(),
                134_217_728 as sql::ULen,
                "column_size mismatch for {lt} with configured max_varchar_size"
            );
        }
    }

    #[test]
    fn uses_char_length_when_present() {
        for lt in &["OBJECT", "ARRAY", "VARIANT"] {
            let field = semi_structured_field(lt, vec![("charLength", "4096")]);
            let ns = NumericSettings::default();
            assert_eq!(
                column_size_from_field(&field, &ns).unwrap(),
                4096 as sql::ULen,
                "column_size mismatch for {lt} with charLength=4096"
            );
        }
    }

    #[test]
    fn decimal_digits_is_zero() {
        for lt in &["OBJECT", "ARRAY", "VARIANT"] {
            let field = semi_structured_field(lt, vec![]);
            let ns = NumericSettings::default();
            assert_eq!(
                decimal_digits_from_field(&field, &ns).unwrap(),
                0,
                "decimal_digits mismatch for {lt}"
            );
        }
    }

    #[test]
    fn errors_on_unparseable_char_length() {
        let field = semi_structured_field("VARIANT", vec![("charLength", "not_a_number")]);
        let ns = NumericSettings::default();
        let err = sql_type_from_field(&field, &ns).unwrap_err();
        assert!(
            matches!(err, ConversionError::FieldMetadataParsing { ref key, .. } if key == "charLength"),
            "expected FieldMetadataParsing for charLength, got: {err}"
        );
    }

    #[test]
    fn make_converter_succeeds_on_utf8_array() {
        for lt in &["OBJECT", "ARRAY", "VARIANT"] {
            let field = semi_structured_field(lt, vec![]);
            let array: GenericByteArray<Utf8Type> = vec![Some(r#"{"key":"value"}"#)].into();
            let ns = NumericSettings::default();
            let result = make_converter(&field, &array, &ns);
            assert!(
                result.is_ok(),
                "make_converter failed for {lt}: {:?}",
                result.err()
            );
        }
    }
}
