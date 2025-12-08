#[cfg(test)]
mod tests {
    use crate::read_arrow::{Buffer, FieldMeta, ReadArrowValue};
    use arrow::array::{Array, Int32Array, Int64Array, StructArray};
    use arrow::datatypes::{DataType, Field, Fields};
    use odbc_sys as sql;
    use std::sync::Arc;

    #[test]
    fn test_timestamp_ltz_struct_parsing() {
        // Test case from BindCatchTest: epoch=1, fraction=512037025
        // Should represent 1512037025 seconds since Unix epoch
        // Which is 2017-11-30 10:17:05 UTC

        let epoch_array = Int64Array::from(vec![1]);
        let fraction_array = Int32Array::from(vec![512037025]);

        let fields = Fields::from(vec![
            Field::new("epoch", DataType::Int64, true),
            Field::new("fraction", DataType::Int32, true),
        ]);

        let struct_array = StructArray::new(
            fields.clone(),
            vec![
                Arc::new(epoch_array) as Arc<dyn Array>,
                Arc::new(fraction_array) as Arc<dyn Array>,
            ],
            None,
        );

        // Create a buffer to write the result
        let mut output = vec![0u8; 100];
        let mut str_len: sql::Len = 0;
        let buffer = Buffer::new(
            output.as_mut_ptr() as *mut sql::Char,
            output.len(),
            &mut str_len as *mut sql::Len,
        );

        // Read the struct
        let field_meta = FieldMeta::Other {
            logical_type: None,
            scale: None,
        };
        let field = Field::new("test_timestamp_ltz", DataType::Struct(fields), true);

        buffer.read_struct(&field_meta, &struct_array, 0).unwrap();

        // Check the result
        let result_str = unsafe {
            let len = str_len as usize;
            std::str::from_utf8_unchecked(&output[..len])
        };

        println!("Result: {}", result_str);

        // The timestamp should be 2017-11-30 10:17:05 UTC
        // (1512037025 seconds since epoch)
        assert_eq!(result_str, "2017-11-30T10:17:05Z");
    }

    #[test]
    fn test_timestamp_ltz_struct_with_large_epoch() {
        // Test case with normal epoch value (not split)
        // 1512055025 seconds = 2017-11-30 15:17:05 UTC

        let epoch_array = Int64Array::from(vec![1512055025]);
        let fraction_array = Int32Array::from(vec![0]);

        let fields = Fields::from(vec![
            Field::new("epoch", DataType::Int64, true),
            Field::new("fraction", DataType::Int32, true),
        ]);

        let struct_array = StructArray::new(
            fields.clone(),
            vec![
                Arc::new(epoch_array) as Arc<dyn Array>,
                Arc::new(fraction_array) as Arc<dyn Array>,
            ],
            None,
        );

        let mut output = vec![0u8; 100];
        let mut str_len: sql::Len = 0;
        let buffer = Buffer::new(
            output.as_mut_ptr() as *mut sql::Char,
            output.len(),
            &mut str_len as *mut sql::Len,
        );

        let field_meta = FieldMeta::Other {
            logical_type: None,
            scale: None,
        };
        let field = Field::new("test_timestamp_ltz", DataType::Struct(fields), true);

        buffer.read_struct(&field_meta, &struct_array, 0).unwrap();

        let result_str = unsafe {
            let len = str_len as usize;
            std::str::from_utf8_unchecked(&output[..len])
        };

        println!("Result: {}", result_str);

        // Should be 2017-11-30 15:17:05 UTC
        assert_eq!(result_str, "2017-11-30T15:17:05Z");
    }

    #[test]
    fn test_timestamp_ltz_struct_with_nanoseconds() {
        // Test with fractional seconds
        let epoch_array = Int64Array::from(vec![1512055025]);
        let fraction_array = Int32Array::from(vec![123456789]); // 123.456789 ms

        let fields = Fields::from(vec![
            Field::new("epoch", DataType::Int64, true),
            Field::new("fraction", DataType::Int32, true),
        ]);

        let struct_array = StructArray::new(
            fields.clone(),
            vec![
                Arc::new(epoch_array) as Arc<dyn Array>,
                Arc::new(fraction_array) as Arc<dyn Array>,
            ],
            None,
        );

        let mut output = vec![0u8; 100];
        let mut str_len: sql::Len = 0;
        let buffer = Buffer::new(
            output.as_mut_ptr() as *mut sql::Char,
            output.len(),
            &mut str_len as *mut sql::Len,
        );

        let field_meta = FieldMeta::Other {
            logical_type: None,
            scale: None,
        };
        let field = Field::new("test_timestamp_ltz", DataType::Struct(fields), true);

        buffer.read_struct(&field_meta, &struct_array, 0).unwrap();

        let result_str = unsafe {
            let len = str_len as usize;
            std::str::from_utf8_unchecked(&output[..len])
        };

        println!("Result with nanos: {}", result_str);

        // Should include fractional seconds
        assert_eq!(result_str, "2017-11-30T15:17:05.123456789Z");
    }

    #[test]
    fn test_timestamp_ltz_null() {
        // Test NULL timestamp
        let epoch_array = Int64Array::from(vec![None]);
        let fraction_array = Int32Array::from(vec![None]);

        let fields = Fields::from(vec![
            Field::new("epoch", DataType::Int64, true),
            Field::new("fraction", DataType::Int32, true),
        ]);

        let struct_array = StructArray::new(
            fields.clone(),
            vec![
                Arc::new(epoch_array) as Arc<dyn Array>,
                Arc::new(fraction_array) as Arc<dyn Array>,
            ],
            None,
        );

        let mut output = vec![0u8; 100];
        let mut str_len: sql::Len = 0;
        let buffer = Buffer::new(
            output.as_mut_ptr() as *mut sql::Char,
            output.len(),
            &mut str_len as *mut sql::Len,
        );

        let field_meta = FieldMeta::Other {
            logical_type: None,
            scale: None,
        };
        let field = Field::new("test_timestamp_ltz", DataType::Struct(fields), true);

        buffer.read_struct(&field_meta, &struct_array, 0).unwrap();

        // Should write SQL_NULL_DATA
        assert_eq!(str_len, sql::NULL_DATA);
    }

    #[test]
    fn test_timestamp_epoch_zero() {
        // Test Unix epoch (1970-01-01 00:00:00)
        let epoch_array = Int64Array::from(vec![0]);
        let fraction_array = Int32Array::from(vec![0]);

        let fields = Fields::from(vec![
            Field::new("epoch", DataType::Int64, true),
            Field::new("fraction", DataType::Int32, true),
        ]);

        let struct_array = StructArray::new(
            fields.clone(),
            vec![
                Arc::new(epoch_array) as Arc<dyn Array>,
                Arc::new(fraction_array) as Arc<dyn Array>,
            ],
            None,
        );

        let mut output = vec![0u8; 100];
        let mut str_len: sql::Len = 0;
        let buffer = Buffer::new(
            output.as_mut_ptr() as *mut sql::Char,
            output.len(),
            &mut str_len as *mut sql::Len,
        );

        let field_meta = FieldMeta::Other {
            logical_type: None,
            scale: None,
        };
        let field = Field::new("test_timestamp_ltz", DataType::Struct(fields), true);

        buffer.read_struct(&field_meta, &struct_array, 0).unwrap();

        let result_str = unsafe {
            let len = str_len as usize;
            std::str::from_utf8_unchecked(&output[..len])
        };

        println!("Epoch zero result: {}", result_str);
        assert_eq!(result_str, "1970-01-01T00:00:00Z");
    }

    #[test]
    fn test_timestamp_negative_epoch() {
        // Test timestamp before Unix epoch (1969-12-31 23:59:59)
        let epoch_array = Int64Array::from(vec![-1]);
        let fraction_array = Int32Array::from(vec![0]);

        let fields = Fields::from(vec![
            Field::new("epoch", DataType::Int64, true),
            Field::new("fraction", DataType::Int32, true),
        ]);

        let struct_array = StructArray::new(
            fields.clone(),
            vec![
                Arc::new(epoch_array) as Arc<dyn Array>,
                Arc::new(fraction_array) as Arc<dyn Array>,
            ],
            None,
        );

        let mut output = vec![0u8; 100];
        let mut str_len: sql::Len = 0;
        let buffer = Buffer::new(
            output.as_mut_ptr() as *mut sql::Char,
            output.len(),
            &mut str_len as *mut sql::Len,
        );

        let field_meta = FieldMeta::Other {
            logical_type: None,
            scale: None,
        };
        let field = Field::new("test_timestamp_ltz", DataType::Struct(fields), true);

        buffer.read_struct(&field_meta, &struct_array, 0).unwrap();

        let result_str = unsafe {
            let len = str_len as usize;
            std::str::from_utf8_unchecked(&output[..len])
        };

        println!("Negative epoch result: {}", result_str);
        assert_eq!(result_str, "1969-12-31T23:59:59Z");
    }

    #[test]
    fn test_timestamp_year_2038_problem() {
        // Test timestamp beyond 32-bit signed int max (2038-01-19 03:14:08)
        let epoch_array = Int64Array::from(vec![2147483648]); // 2^31
        let fraction_array = Int32Array::from(vec![0]);

        let fields = Fields::from(vec![
            Field::new("epoch", DataType::Int64, true),
            Field::new("fraction", DataType::Int32, true),
        ]);

        let struct_array = StructArray::new(
            fields.clone(),
            vec![
                Arc::new(epoch_array) as Arc<dyn Array>,
                Arc::new(fraction_array) as Arc<dyn Array>,
            ],
            None,
        );

        let mut output = vec![0u8; 100];
        let mut str_len: sql::Len = 0;
        let buffer = Buffer::new(
            output.as_mut_ptr() as *mut sql::Char,
            output.len(),
            &mut str_len as *mut sql::Len,
        );

        let field_meta = FieldMeta::Other {
            logical_type: None,
            scale: None,
        };
        let field = Field::new("test_timestamp_ltz", DataType::Struct(fields), true);

        buffer.read_struct(&field_meta, &struct_array, 0).unwrap();

        let result_str = unsafe {
            let len = str_len as usize;
            std::str::from_utf8_unchecked(&output[..len])
        };

        println!("Year 2038 result: {}", result_str);
        assert_eq!(result_str, "2038-01-19T03:14:08Z");
    }

    #[test]
    fn test_timestamp_max_nanoseconds() {
        // Test maximum nanosecond precision (999999999 ns)
        let epoch_array = Int64Array::from(vec![1512055025]);
        let fraction_array = Int32Array::from(vec![999999999]);

        let fields = Fields::from(vec![
            Field::new("epoch", DataType::Int64, true),
            Field::new("fraction", DataType::Int32, true),
        ]);

        let struct_array = StructArray::new(
            fields.clone(),
            vec![
                Arc::new(epoch_array) as Arc<dyn Array>,
                Arc::new(fraction_array) as Arc<dyn Array>,
            ],
            None,
        );

        let mut output = vec![0u8; 100];
        let mut str_len: sql::Len = 0;
        let buffer = Buffer::new(
            output.as_mut_ptr() as *mut sql::Char,
            output.len(),
            &mut str_len as *mut sql::Len,
        );

        let field_meta = FieldMeta::Other {
            logical_type: None,
            scale: None,
        };
        let field = Field::new("test_timestamp_ltz", DataType::Struct(fields), true);

        buffer.read_struct(&field_meta, &struct_array, 0).unwrap();

        let result_str = unsafe {
            let len = str_len as usize;
            std::str::from_utf8_unchecked(&output[..len])
        };

        println!("Max nanos result: {}", result_str);
        assert_eq!(result_str, "2017-11-30T15:17:05.999999999Z");
    }
}
