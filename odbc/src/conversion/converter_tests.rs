#[cfg(test)]
mod tests {
    use crate::api::CDataType;
    use crate::conversion::error::ConversionError;
    use crate::conversion::traits::BindingStrides;
    use crate::conversion::warning::Warnings;
    use crate::conversion::{Binding, NumericSettings, make_converter};
    use arrow::array::{ArrayRef, Date32Array, Decimal128Array, Float64Array, Int64Array};
    use arrow::datatypes::{DataType, Field};
    use odbc_sys as sql;
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

    /// `convert_arrow_range` and `convert_arrow_value` must produce the same
    /// bytes / warnings / errors per row. This guards against the batched
    /// hot paths in `BatchedWrite` drifting from the per-cell
    /// `WriteODBCType` implementation.
    fn assert_range_matches_per_cell(
        field: &Field,
        array: ArrayRef,
        target_type: CDataType,
        buffer_length: sql::Len,
    ) {
        let ns = NumericSettings::default();
        let converter = make_converter(field, &ns).expect("converter");

        let row_count = array.len();
        let value_stride = target_type.fixed_size().unwrap_or(buffer_length as usize);
        let mut value_buf_range = vec![0u8; row_count * value_stride];
        let mut len_buf_range: Vec<sql::Len> = vec![0; row_count];
        let base = Binding {
            target_type,
            target_value_ptr: value_buf_range.as_mut_ptr() as sql::Pointer,
            buffer_length,
            octet_length_ptr: len_buf_range.as_mut_ptr(),
            indicator_ptr: len_buf_range.as_mut_ptr(),
            ..Default::default()
        };
        let strides = BindingStrides::default();
        let mut outputs: Vec<Result<Warnings, ConversionError>> =
            (0..row_count).map(|_| Ok(Warnings::new())).collect();
        converter.convert_arrow_range(
            array.as_ref(),
            0..row_count,
            &base,
            0,
            strides,
            &mut outputs,
        );

        let mut value_buf_per_cell = vec![0u8; row_count * value_stride];
        let mut len_buf_per_cell: Vec<sql::Len> = vec![0; row_count];
        for (i, slot) in outputs.iter().enumerate() {
            let binding = Binding {
                target_type,
                target_value_ptr: unsafe {
                    value_buf_per_cell.as_mut_ptr().add(i * value_stride) as sql::Pointer
                },
                buffer_length,
                octet_length_ptr: unsafe { len_buf_per_cell.as_mut_ptr().add(i) },
                indicator_ptr: unsafe { len_buf_per_cell.as_mut_ptr().add(i) },
                ..Default::default()
            };
            let cell_result = converter.convert_arrow_value(array.as_ref(), i, &binding, &mut None);
            match (slot, cell_result) {
                (Ok(w_range), Ok(w_cell)) => assert_eq!(w_range, &w_cell, "row {i} warnings"),
                // `Debug` includes call-site `Location`s that differ between
                // the batched and per-cell entrypoints; compare the
                // user-visible Display string instead.
                (Err(e_range), Err(e_cell)) => {
                    assert_eq!(e_range.to_string(), e_cell.to_string(), "row {i} error",)
                }
                (a, b) => panic!("row {i} status mismatch: range={a:?}, cell={b:?}"),
            }
        }
        assert_eq!(value_buf_range, value_buf_per_cell, "value bytes diverged");
        assert_eq!(
            len_buf_range, len_buf_per_cell,
            "length indicators diverged"
        );
    }

    fn fixed_field(scale: u32, precision: u32, dt: DataType) -> Field {
        let md: HashMap<String, String> = [
            ("logicalType".to_string(), "FIXED".to_string()),
            ("scale".to_string(), scale.to_string()),
            ("precision".to_string(), precision.to_string()),
        ]
        .into_iter()
        .collect();
        Field::new("col", dt, true).with_metadata(md)
    }

    fn date_field() -> Field {
        let md: HashMap<String, String> = [("logicalType", "DATE")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Field::new("col", DataType::Date32, true).with_metadata(md)
    }

    fn real_field() -> Field {
        let md: HashMap<String, String> = [("logicalType", "REAL")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Field::new("col", DataType::Float64, true).with_metadata(md)
    }

    #[test]
    fn batched_number_to_char_matches_per_cell() {
        let field = fixed_field(0, 18, DataType::Int64);
        let array: ArrayRef = Arc::new(Int64Array::from(vec![
            Some(0i64),
            Some(1),
            Some(-1),
            Some(i64::MAX),
            Some(i64::MIN),
            None,
            Some(42),
        ]));
        assert_range_matches_per_cell(&field, array, CDataType::Char, 64);
    }

    #[test]
    fn batched_decimal128_to_char_matches_per_cell() {
        let field = fixed_field(2, 38, DataType::Decimal128(38, 2));
        let array: ArrayRef = Arc::new(
            Decimal128Array::from(vec![
                Some(0i128),
                Some(123_456_i128),
                Some(-987_654_i128),
                None,
            ])
            .with_precision_and_scale(38, 2)
            .unwrap(),
        );
        assert_range_matches_per_cell(&field, array, CDataType::Char, 64);
    }

    #[test]
    fn batched_number_falls_back_to_per_row_for_sbigint_target() {
        // Non-Char target must go through write_odbc_segment_per_row so the
        // existing CDataType::SBigInt path is exercised unchanged.
        let field = fixed_field(0, 18, DataType::Int64);
        let array: ArrayRef = Arc::new(Int64Array::from(vec![Some(7i64), None, Some(-3)]));
        assert_range_matches_per_cell(&field, array, CDataType::SBigInt, 0);
    }

    #[test]
    fn batched_date_to_char_matches_per_cell() {
        let field = date_field();
        // Date32 = days since 1970-01-01: 0, +1, -1, large positive, null.
        let array: ArrayRef = Arc::new(Date32Array::from(vec![
            Some(0),
            Some(1),
            Some(-1),
            Some(20_000),
            None,
        ]));
        assert_range_matches_per_cell(&field, array, CDataType::Char, 32);
    }

    #[test]
    fn batched_date_to_char_short_buffer_errors_every_row() {
        let field = date_field();
        let array: ArrayRef = Arc::new(Date32Array::from(vec![Some(0), Some(1), Some(2)]));
        // Buffer < 11 bytes means no row can fit "YYYY-MM-DD\0".
        assert_range_matches_per_cell(&field, array, CDataType::Char, 8);
    }

    #[test]
    fn batched_real_uses_default_per_row_dispatch() {
        // SnowflakeReal has the default BatchedWrite impl — this guards
        // that the dispatcher still produces parity output.
        let field = real_field();
        let array: ArrayRef = Arc::new(Float64Array::from(vec![
            Some(0.0),
            Some(-0.5),
            Some(123.456),
            None,
        ]));
        assert_range_matches_per_cell(&field, array, CDataType::Char, 32);
    }
}
