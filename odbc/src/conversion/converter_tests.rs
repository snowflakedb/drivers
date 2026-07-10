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

    // Multi-row block-cursor striding: `convert_arrow_range` must write each
    // row into its own slot of the bound buffer (and its own indicator). This
    // is the path the incremental-striding fast path optimizes, and it was
    // previously only covered end-to-end; assert it directly so a striding
    // regression (every row landing in slot 0, or a wrong offset) is caught by
    // `cargo test` — across fixed-size (DATE), variable-size (VARCHAR), and
    // wider (TIMESTAMP_NTZ) converters, not just one.
    fn date_converter() -> Box<dyn crate::conversion::ColumnConverter> {
        let md = HashMap::from([("logicalType".to_string(), "DATE".to_string())]);
        let field = Field::new("col", DataType::Date32, true).with_metadata(md);
        make_converter(&field, &NumericSettings::default()).expect("date converter")
    }

    fn varchar_converter() -> Box<dyn crate::conversion::ColumnConverter> {
        let md = HashMap::from([
            ("logicalType".to_string(), "TEXT".to_string()),
            ("charLength".to_string(), "256".to_string()),
        ]);
        let field = Field::new("col", DataType::Utf8, true).with_metadata(md);
        make_converter(&field, &NumericSettings::default()).expect("varchar converter")
    }

    fn timestamp_ntz_converter() -> Box<dyn crate::conversion::ColumnConverter> {
        let md = HashMap::from([
            ("logicalType".to_string(), "TIMESTAMP_NTZ".to_string()),
            ("scale".to_string(), "9".to_string()),
        ]);
        let field = Field::new("col", DataType::Int64, true).with_metadata(md);
        make_converter(&field, &NumericSettings::default()).expect("timestamp_ntz converter")
    }

    /// Drive `convert_arrow_range` over `array` and assert every row lands in
    /// its own strided slot with the right value + indicator, and that slots
    /// before `out_row_start` are untouched. Generic over the converter so
    /// fixed-size, variable-size, and wide types all exercise the same path.
    fn assert_strided(
        conv: &dyn crate::conversion::ColumnConverter,
        array: &dyn arrow::array::Array,
        expected: &[&str],
        out_row_start: usize,
    ) {
        use crate::api::CDataType;
        use crate::conversion::BindingStrides;
        use crate::conversion::traits::Binding;
        use odbc_sys as sql;

        const CELL: usize = 48;
        let n = expected.len();
        let total = n + out_row_start;
        let mut buf = vec![0u8; total * CELL];
        let mut inds = vec![0 as sql::Len; total];
        let base = Binding {
            target_type: CDataType::Char,
            target_value_ptr: buf.as_mut_ptr() as sql::Pointer,
            buffer_length: CELL as sql::Len,
            octet_length_ptr: inds.as_mut_ptr(),
            indicator_ptr: inds.as_mut_ptr(),
            ..Default::default()
        };
        let mut outputs: Vec<Result<crate::conversion::warning::Warnings, ConversionError>> =
            (0..n).map(|_| Ok(Vec::new())).collect();

        conv.convert_arrow_range(
            array,
            0..n,
            &base,
            out_row_start,
            BindingStrides {
                bind_type: 0,
                bind_offset: 0,
            },
            &mut outputs,
        );

        for r in 0..n {
            assert!(outputs[r].is_ok(), "row {r} errored: {:?}", outputs[r]);
            let slot = out_row_start + r;
            let cell = &buf[slot * CELL..slot * CELL + CELL];
            let s = std::ffi::CStr::from_bytes_until_nul(cell)
                .unwrap()
                .to_str()
                .unwrap();
            assert_eq!(s, expected[r], "value at slot {slot} (row {r})");
            assert_eq!(
                inds[slot],
                expected[r].len() as sql::Len,
                "indicator slot {slot}"
            );
        }
        for slot in 0..out_row_start {
            assert!(
                buf[slot * CELL..slot * CELL + CELL].iter().all(|&b| b == 0),
                "slot {slot} before out_row_start should be untouched"
            );
        }
    }

    fn assert_strided_dates(out_row_start: usize) {
        use arrow::array::Date32Array;
        // Include pre-Unix (negative) days to cover the year<1970 / negative
        // epoch path, not just forward offsets.
        let array = Date32Array::from(vec![-366, -365, -1, 0, 1, 31, 365]);
        let expected = [
            "1968-12-31",
            "1969-01-01",
            "1969-12-31",
            "1970-01-01",
            "1970-01-02",
            "1970-02-01",
            "1971-01-01",
        ];
        assert_strided(date_converter().as_ref(), &array, &expected, out_row_start);
    }

    #[test]
    fn convert_arrow_range_strides_each_row_to_its_own_slot() {
        assert_strided_dates(0);
    }

    #[test]
    fn convert_arrow_range_honors_out_row_start_offset() {
        assert_strided_dates(2);
    }

    #[test]
    fn convert_arrow_range_strides_varchar_variable_size() {
        use arrow::array::StringArray;
        let array = StringArray::from(vec!["a", "bb", "ccc", "dddd"]);
        let expected = ["a", "bb", "ccc", "dddd"];
        assert_strided(varchar_converter().as_ref(), &array, &expected, 0);
        assert_strided(varchar_converter().as_ref(), &array, &expected, 2);
    }

    #[test]
    fn convert_arrow_range_strides_timestamp_ntz() {
        // scale 9 -> raw is epoch nanoseconds; whole seconds render without a
        // fractional part.
        let array = Int64Array::from(vec![0i64, 1_000_000_000, 61_000_000_000]);
        let expected = [
            "1970-01-01 00:00:00",
            "1970-01-01 00:00:01",
            "1970-01-01 00:01:01",
        ];
        assert_strided(timestamp_ntz_converter().as_ref(), &array, &expected, 0);
        assert_strided(timestamp_ntz_converter().as_ref(), &array, &expected, 2);
    }

    // The "pathological first-row overflow" fallback the incremental-striding
    // path documents: a row-wise stride large enough to overflow `for_row` when
    // it materializes the first row's binding must fall back to the per-cell
    // path, which reports the overflow per row — no panic, no silent mis-write.
    #[test]
    fn convert_arrow_range_falls_back_to_per_cell_on_stride_overflow() {
        use crate::api::CDataType;
        use crate::conversion::BindingStrides;
        use crate::conversion::traits::Binding;
        use arrow::array::Date32Array;
        use odbc_sys as sql;

        let array = Date32Array::from(vec![0, 1, 31]);
        let n = 3;
        let mut buf = vec![0u8; 64];
        let mut inds = vec![0 as sql::Len; 8];
        let base = Binding {
            target_type: CDataType::Char,
            target_value_ptr: buf.as_mut_ptr() as sql::Pointer,
            buffer_length: 16,
            octet_length_ptr: inds.as_mut_ptr(),
            indicator_ptr: inds.as_mut_ptr(),
            ..Default::default()
        };
        let strides = BindingStrides {
            bind_type: usize::MAX / 2,
            bind_offset: 0,
        };
        let mut outputs: Vec<Result<crate::conversion::warning::Warnings, ConversionError>> =
            (0..n).map(|_| Ok(Vec::new())).collect();

        date_converter().convert_arrow_range(&array, 0..n, &base, 4, strides, &mut outputs);

        for (r, out) in outputs.iter().enumerate() {
            assert!(
                matches!(out, Err(ConversionError::BindingStrideOverflow { .. })),
                "row {r} expected BindingStrideOverflow, got {out:?}"
            );
        }
    }
}
