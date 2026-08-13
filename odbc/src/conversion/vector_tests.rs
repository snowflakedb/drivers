#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use arrow::array::{Array, FixedSizeListArray, Float32Array, Int32Array};
    use arrow::buffer::NullBuffer;
    use arrow::datatypes::{DataType, Field};
    use odbc_sys as sql;

    use crate::conversion::error::ConversionError;
    use crate::conversion::{
        NumericSettings, SF_DEFAULT_VARCHAR_MAX_LEN, column_size_from_field,
        decimal_digits_from_field, make_converter, sql_type_from_field,
    };

    fn vector_field(child_data_type: DataType, dimension: i32) -> Field {
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), "VECTOR".to_string());
        let child_field = Arc::new(Field::new("item", child_data_type, false));
        Field::new("col", DataType::FixedSizeList(child_field, dimension), true).with_metadata(meta)
    }

    fn vector_field_with_char_length(
        child_data_type: DataType,
        dimension: i32,
        char_len: u32,
    ) -> Field {
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), "VECTOR".to_string());
        meta.insert("charLength".to_string(), char_len.to_string());
        let child_field = Arc::new(Field::new("item", child_data_type, false));
        Field::new("col", DataType::FixedSizeList(child_field, dimension), true).with_metadata(meta)
    }

    // -------------------------------------------------------------------------
    // Metadata: sql_type
    // -------------------------------------------------------------------------

    #[test]
    fn should_map_int_vector_to_sql_varchar() {
        let field = vector_field(DataType::Int32, 3);
        let ns = NumericSettings::default();
        assert_eq!(
            sql_type_from_field(&field, &ns).unwrap(),
            sql::SqlDataType::VARCHAR
        );
    }

    #[test]
    fn should_map_float_vector_to_sql_varchar() {
        let field = vector_field(DataType::Float32, 5);
        let ns = NumericSettings::default();
        assert_eq!(
            sql_type_from_field(&field, &ns).unwrap(),
            sql::SqlDataType::VARCHAR
        );
    }

    // -------------------------------------------------------------------------
    // Metadata: column_size
    // -------------------------------------------------------------------------

    #[test]
    fn should_default_vector_column_size_to_max_varchar() {
        let field = vector_field(DataType::Int32, 3);
        let ns = NumericSettings::default();
        assert_eq!(
            column_size_from_field(&field, &ns).unwrap(),
            SF_DEFAULT_VARCHAR_MAX_LEN as sql::ULen
        );
    }

    #[test]
    fn should_use_configured_max_varchar_size_for_vector_when_char_length_missing() {
        let field = vector_field(DataType::Float32, 3);
        let ns = NumericSettings {
            max_varchar_size: 134_217_728,
            ..NumericSettings::default()
        };
        assert_eq!(
            column_size_from_field(&field, &ns).unwrap(),
            134_217_728 as sql::ULen
        );
    }

    #[test]
    fn should_use_char_length_for_vector_when_present() {
        let field = vector_field_with_char_length(DataType::Int32, 4, 4096);
        let ns = NumericSettings::default();
        assert_eq!(
            column_size_from_field(&field, &ns).unwrap(),
            4096 as sql::ULen
        );
    }

    // -------------------------------------------------------------------------
    // Metadata: decimal_digits
    // -------------------------------------------------------------------------

    #[test]
    fn should_return_zero_decimal_digits_for_vector() {
        let field = vector_field(DataType::Int32, 3);
        let ns = NumericSettings::default();
        assert_eq!(decimal_digits_from_field(&field, &ns).unwrap(), 0);
    }

    // -------------------------------------------------------------------------
    // make_converter: success cases
    // -------------------------------------------------------------------------

    #[test]
    fn should_make_converter_for_int_vector_field() {
        let field = vector_field(DataType::Int32, 3);
        let ns = NumericSettings::default();
        let result = make_converter(&field, &ns);
        assert!(
            result.is_ok(),
            "make_converter failed for VECTOR(INT): {:?}",
            result.err()
        );
    }

    #[test]
    fn should_make_converter_for_float_vector_field() {
        let field = vector_field(DataType::Float32, 5);
        let ns = NumericSettings::default();
        let result = make_converter(&field, &ns);
        assert!(
            result.is_ok(),
            "make_converter failed for VECTOR(FLOAT): {:?}",
            result.err()
        );
    }

    // -------------------------------------------------------------------------
    // make_converter: rejection of wrong Arrow layouts
    // -------------------------------------------------------------------------

    #[test]
    fn should_reject_vector_field_with_utf8_arrow_type() {
        // VECTOR logicalType must be backed by FixedSizeList; plain Utf8 must fail.
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), "VECTOR".to_string());
        let field = Field::new("col", DataType::Utf8, true).with_metadata(meta);
        let ns = NumericSettings::default();
        let err = make_converter(&field, &ns)
            .err()
            .expect("expected error for VECTOR+Utf8 field");
        assert!(
            matches!(err, ConversionError::IncompatibleFieldMetadata { ref logical_type, .. }
                if logical_type.contains("VECTOR")),
            "expected IncompatibleFieldMetadata for VECTOR+Utf8, got: {err}"
        );
    }

    #[test]
    fn should_reject_vector_field_with_unsupported_child_type() {
        // VECTOR child must be Int32 or Float32; Int64 is not supported.
        let mut meta = HashMap::new();
        meta.insert("logicalType".to_string(), "VECTOR".to_string());
        let child_field = Arc::new(Field::new("item", DataType::Int64, false));
        let field =
            Field::new("col", DataType::FixedSizeList(child_field, 3), true).with_metadata(meta);
        let ns = NumericSettings::default();
        let err = make_converter(&field, &ns)
            .err()
            .expect("expected error for VECTOR+Int64 child field");
        assert!(
            matches!(err, ConversionError::IncompatibleFieldMetadata { .. }),
            "expected IncompatibleFieldMetadata for VECTOR+Int64 child, got: {err}"
        );
    }

    // -------------------------------------------------------------------------
    // Serialization: INT values
    // -------------------------------------------------------------------------

    fn make_int_vector_array(rows: &[Option<Vec<i32>>], dimension: i32) -> FixedSizeListArray {
        let child_field = Arc::new(Field::new("item", DataType::Int32, false));
        let _nulls_count = rows.len();
        let flat: Vec<i32> = rows
            .iter()
            .flat_map(|opt| match opt {
                Some(v) => v.clone(),
                None => vec![0i32; dimension as usize],
            })
            .collect();
        let values = Arc::new(Int32Array::from(flat)) as Arc<dyn Array>;
        let null_buf = NullBuffer::from(rows.iter().map(|o| o.is_some()).collect::<Vec<bool>>());
        FixedSizeListArray::try_new(child_field, dimension, values, Some(null_buf))
            .expect("valid FixedSizeListArray")
    }

    fn make_float_vector_array(rows: &[Option<Vec<f32>>], dimension: i32) -> FixedSizeListArray {
        let child_field = Arc::new(Field::new("item", DataType::Float32, false));
        let flat: Vec<f32> = rows
            .iter()
            .flat_map(|opt| match opt {
                Some(v) => v.clone(),
                None => vec![0.0f32; dimension as usize],
            })
            .collect();
        let values = Arc::new(Float32Array::from(flat)) as Arc<dyn Array>;
        let null_buf = NullBuffer::from(rows.iter().map(|o| o.is_some()).collect::<Vec<bool>>());
        FixedSizeListArray::try_new(child_field, dimension, values, Some(null_buf))
            .expect("valid FixedSizeListArray")
    }

    use crate::conversion::ReadArrowType;
    use crate::conversion::vector::{SnowflakeVector, VectorElementType};

    #[test]
    fn should_serialize_int_vector_to_json_string() {
        let arr = make_int_vector_array(&[Some(vec![1, 3, -5])], 3);
        let sv = SnowflakeVector {
            element_type: VectorElementType::Int32,
            column_size: 134_217_728,
        };
        let result = sv.read_arrow_type(&arr, 0).unwrap();
        assert_eq!(result, "[1,3,-5]");
    }

    #[test]
    fn should_serialize_float_vector_to_json_string() {
        let arr = make_float_vector_array(&[Some(vec![1.5f32, -3.5f32, 0.0f32])], 3);
        let sv = SnowflakeVector {
            element_type: VectorElementType::Float32,
            column_size: 134_217_728,
        };
        let result = sv.read_arrow_type(&arr, 0).unwrap();
        // Each element must be parseable as a number; exact spacing may vary.
        assert!(result.starts_with('[') && result.ends_with(']'));
        let inner = &result[1..result.len() - 1];
        let parts: Vec<&str> = inner.split(',').collect();
        assert_eq!(parts.len(), 3);
        let v0: f64 = parts[0].parse().expect("parseable float");
        let v1: f64 = parts[1].parse().expect("parseable float");
        let v2: f64 = parts[2].parse().expect("parseable float");
        assert!((v0 - 1.5).abs() < 1e-6);
        assert!((v1 - (-3.5)).abs() < 1e-6);
        assert!(v2.abs() < 1e-6);
    }

    #[test]
    fn should_return_null_value_error_for_null_row() {
        let arr = make_int_vector_array(&[None], 3);
        let sv = SnowflakeVector {
            element_type: VectorElementType::Int32,
            column_size: 134_217_728,
        };
        let result = sv.read_arrow_type(&arr, 0);
        assert!(
            matches!(
                result,
                Err(crate::conversion::error::ReadArrowError::NullValue { .. })
            ),
            "expected NullValue error, got: {result:?}"
        );
    }

    #[test]
    fn should_preserve_float_smallest_normal() {
        // FLOAT32_SMALLEST_NORMAL = 2^-126 ≈ 1.1754944e-38
        let smallest = f32::MIN_POSITIVE;
        let arr = make_float_vector_array(&[Some(vec![smallest])], 1);
        let sv = SnowflakeVector {
            element_type: VectorElementType::Float32,
            column_size: 134_217_728,
        };
        let result = sv.read_arrow_type(&arr, 0).unwrap();
        let inner = &result[1..result.len() - 1];
        let parsed: f64 = inner.parse().expect("parseable f64");
        assert!(
            parsed > 0.0,
            "FLOAT32_SMALLEST_NORMAL must not round to zero; got: {result}"
        );
    }

    #[test]
    fn should_serialize_non_finite_floats_with_ecosystem_tokens() {
        let arr =
            make_float_vector_array(&[Some(vec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY])], 3);
        let sv = SnowflakeVector {
            element_type: VectorElementType::Float32,
            column_size: 134_217_728,
        };
        // Non-finite floats use the Snowflake ecosystem spellings (old ODBC picojson,
        // JSON bind parser, JDBC List.toString()) rather than Rust's `inf` / `-inf`.
        assert_eq!(
            sv.read_arrow_type(&arr, 0).unwrap(),
            "[NaN,Infinity,-Infinity]"
        );
    }

    #[test]
    fn should_serialize_int_boundary_values() {
        let arr = make_int_vector_array(&[Some(vec![i32::MIN, i32::MAX, 0])], 3);
        let sv = SnowflakeVector {
            element_type: VectorElementType::Int32,
            column_size: 134_217_728,
        };
        let result = sv.read_arrow_type(&arr, 0).unwrap();
        assert_eq!(result, format!("[{},{},0]", i32::MIN, i32::MAX));
    }

    #[test]
    fn should_serialize_non_null_rows_from_mixed_batch() {
        let arr = make_int_vector_array(&[Some(vec![1, 2, 3]), None, Some(vec![4, 5, 6])], 3);
        let sv = SnowflakeVector {
            element_type: VectorElementType::Int32,
            column_size: 134_217_728,
        };
        assert_eq!(sv.read_arrow_type(&arr, 0).unwrap(), "[1,2,3]");
        assert!(sv.read_arrow_type(&arr, 1).is_err());
        assert_eq!(sv.read_arrow_type(&arr, 2).unwrap(), "[4,5,6]");
    }
}
