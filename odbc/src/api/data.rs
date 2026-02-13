use crate::api::error::{
    ConversionSnafu, DataNotFetchedSnafu, ExecutionDoneSnafu, FetchDataSnafu, NoMoreDataSnafu,
    StatementErrorStateSnafu, StatementNotExecutedSnafu,
};
use crate::api::{OdbcResult, Statement, StatementState, WithState, stmt_from_handle};
use crate::cdata_types::CDataType;
use crate::conversion::warning::Warnings;
use crate::conversion::{Binding, ConversionError, make_converter};
use arrow::array::Array;
use arrow::datatypes::Field;
use odbc_sys as sql;
use snafu::ResultExt;
use tracing;

fn read_arrow_value(
    target_type: CDataType,
    target_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
    array_ref: &dyn Array,
    field: &Field,
    batch_idx: usize,
) -> Result<Warnings, ConversionError> {
    let converter = make_converter(field, array_ref)?;
    let warnings = converter.convert_arrow_value(
        batch_idx,
        &Binding {
            target_type,
            target_value_ptr,
            buffer_length,
            str_len_or_ind_ptr,
        },
    )?;
    Ok(warnings)
}

/// Fetch the next row of data
pub fn fetch(statement_handle: sql::Handle, warnings: &mut Warnings) -> OdbcResult<()> {
    tracing::debug!("fetch called");
    let stmt = stmt_from_handle(statement_handle);
    stmt.state.transition_or_err(|state| match state {
        StatementState::Executed { mut reader, .. } => match reader.next() {
            Some(record_batch_result) => {
                let record_batch = record_batch_result
                    .context(FetchDataSnafu)
                    .with_state(StatementState::Error)?;
                tracing::debug!(
                    "fetch: fetched record_batch with {} rows",
                    record_batch.num_rows()
                );
                let next_state = StatementState::Fetching {
                    reader,
                    record_batch,
                    batch_idx: 0,
                };
                Ok((next_state, ()))
            }
            None => {
                tracing::debug!("fetch: no more data available");
                NoMoreDataSnafu.fail().with_state(StatementState::Done)
            }
        },
        StatementState::Fetching {
            mut reader,
            record_batch,
            batch_idx,
        } => {
            let new_batch_idx = batch_idx + 1;
            if new_batch_idx < record_batch.num_rows() {
                Ok((
                    StatementState::Fetching {
                        reader,
                        record_batch,
                        batch_idx: new_batch_idx,
                    },
                    (),
                ))
            } else {
                match reader.next() {
                    Some(new_record_batch_result) => {
                        let new_record_batch = new_record_batch_result
                            .context(FetchDataSnafu)
                            .with_state(StatementState::Error)?;
                        let next_state = StatementState::Fetching {
                            reader,
                            record_batch: new_record_batch,
                            batch_idx: 0,
                        };
                        Ok((next_state, ()))
                    }
                    None => {
                        tracing::debug!("fetch: no more data available");
                        NoMoreDataSnafu.fail().with_state(StatementState::Done)
                    }
                }
            }
        }
        state @ StatementState::Error => {
            tracing::error!("fetch: statement error");
            StatementErrorStateSnafu.fail().with_state(state)
        }
        state @ StatementState::Done => {
            tracing::debug!("fetch: statement execution is done");
            ExecutionDoneSnafu.fail().with_state(state)
        }
        state @ StatementState::Created => {
            tracing::error!("fetch: statement not executed");
            StatementNotExecutedSnafu.fail().with_state(state)
        }
    })?;
    warnings.extend(execute_bindings(stmt)?);
    Ok(())
}

fn execute_bindings(stmt: &mut Statement) -> OdbcResult<Warnings> {
    let mut warnings = vec![];
    if let StatementState::Fetching {
        reader: _,
        record_batch,
        batch_idx,
    } = &stmt.state.as_ref()
    {
        for (column_number, binding) in &stmt.column_bindings {
            let array_ref = record_batch.column((column_number - 1) as usize);
            let schema = record_batch.schema();
            let field = schema.field((column_number - 1) as usize);
            tracing::debug!(
                "execute_bindings: column_number={}, binding={:?}",
                column_number,
                binding
            );
            let conversion_warnings = read_arrow_value(
                binding.target_type,
                binding.target_value_ptr,
                binding.buffer_length,
                binding.str_len_or_ind_ptr,
                array_ref,
                field,
                *batch_idx,
            )
            .context(ConversionSnafu)?;
            warnings.extend(conversion_warnings);
        }
    }
    Ok(vec![])
}

/// Get data from a specific column
pub fn get_data(
    statement_handle: sql::Handle,
    col_or_param_num: sql::USmallInt,
    target_type: CDataType,
    target_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    tracing::debug!("get_data: statement_handle={:?}", statement_handle);
    let stmt = stmt_from_handle(statement_handle);
    match stmt.state.as_ref() {
        StatementState::Fetching {
            reader: _,
            record_batch,
            batch_idx,
        } => {
            let array_ref = record_batch.column((col_or_param_num - 1) as usize);
            let schema = record_batch.schema();
            let field = schema.field((col_or_param_num - 1) as usize);

            let conversion_warnings = read_arrow_value(
                target_type,
                target_value_ptr,
                buffer_length,
                str_len_or_ind_ptr,
                array_ref,
                field,
                *batch_idx,
            )
            .context(ConversionSnafu)?;

            warnings.extend(conversion_warnings);
            Ok(())
        }
        StatementState::Done => {
            tracing::debug!("get_data: statement execution is done");
            ExecutionDoneSnafu.fail()
        }
        StatementState::Created => {
            tracing::error!("get_data: data not fetched yet");
            DataNotFetchedSnafu.fail()
        }
        StatementState::Error => {
            tracing::error!("get_data: statement error");
            StatementErrorStateSnafu.fail()
        }
        StatementState::Executed { .. } => {
            tracing::error!("get_data: statement not executed");
            StatementNotExecutedSnafu.fail()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdata_types::{Double, Real, SBigInt, UBigInt};
    use arrow::array::{
        Decimal128Array, Int8Array, Int16Array, Int32Array, Int64Array, StringArray,
    };
    use arrow::datatypes::{DataType, Field};
    use std::collections::HashMap;

    fn field_with_fixed_meta(data_type: DataType, scale: u32, precision: u32) -> Field {
        let mut metadata = HashMap::new();
        metadata.insert("logicalType".to_string(), "FIXED".to_string());
        metadata.insert("scale".to_string(), scale.to_string());
        metadata.insert("precision".to_string(), precision.to_string());
        Field::new("test", data_type, false).with_metadata(metadata)
    }

    fn field_with_text_meta() -> Field {
        let mut metadata = HashMap::new();
        metadata.insert("logicalType".to_string(), "TEXT".to_string());
        metadata.insert("charLength".to_string(), "4096".to_string());
        Field::new("test", DataType::Utf8, false).with_metadata(metadata)
    }

    fn decimal128_field(precision: u8, scale: i8) -> Field {
        let mut metadata = HashMap::new();
        metadata.insert("logicalType".to_string(), "FIXED".to_string());
        metadata.insert("scale".to_string(), scale.to_string());
        metadata.insert("precision".to_string(), precision.to_string());
        Field::new("test", DataType::Decimal128(precision, scale), false).with_metadata(metadata)
    }

    // Tests for CDataType::Char
    mod read_to_char {
        use super::*;

        #[test]
        fn reads_utf8_to_char_buffer() {
            let array = StringArray::from(vec!["hello"]);
            let field = field_with_text_meta();
            let mut buffer = vec![0u8; 32];
            let mut str_len: sql::Len = 0;

            let result = read_arrow_value(
                CDataType::Char,
                buffer.as_mut_ptr() as sql::Pointer,
                buffer.len() as sql::Len,
                &mut str_len,
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(str_len, 5);
            assert_eq!(&buffer[..5], b"hello");
        }

        #[test]
        fn reads_int64_with_scale_to_char_buffer() {
            let array = Int64Array::from(vec![12345i64]); // 123.45 with scale 2
            let field = field_with_fixed_meta(DataType::Int64, 2, 10);
            let mut buffer = vec![0u8; 32];
            let mut str_len: sql::Len = 0;

            let result = read_arrow_value(
                CDataType::Char,
                buffer.as_mut_ptr() as sql::Pointer,
                buffer.len() as sql::Len,
                &mut str_len,
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(str_len, 6);
            assert_eq!(&buffer[..6], b"123.45");
        }

        #[test]
        fn truncates_when_buffer_too_small() {
            let array = StringArray::from(vec!["hello world"]);
            let field = field_with_text_meta();
            let mut buffer = vec![0u8; 5];
            let mut str_len: sql::Len = 0;

            let result = read_arrow_value(
                CDataType::Char,
                buffer.as_mut_ptr() as sql::Pointer,
                buffer.len() as sql::Len,
                &mut str_len,
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(str_len, 11); // Full length reported
            assert_eq!(&buffer[..5], b"hello"); // Truncated
        }
    }

    // Tests for CDataType::UBigInt
    mod read_to_ubigint {
        use super::*;

        #[test]
        fn reads_int64_as_ubigint() {
            let array = Int64Array::from(vec![9876543210i64]);
            let field = field_with_fixed_meta(DataType::Int64, 0, 20);
            let mut value: UBigInt = 0;

            let result = read_arrow_value(
                CDataType::UBigInt,
                &mut value as *mut UBigInt as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(value, 9876543210u64);
        }

        #[test]
        fn reads_int64_with_scale_drops_decimals() {
            let array = Int64Array::from(vec![12345i64]); // 123.45 with scale 2
            let field = field_with_fixed_meta(DataType::Int64, 2, 10);
            let mut value: UBigInt = 0;

            let result = read_arrow_value(
                CDataType::UBigInt,
                &mut value as *mut UBigInt as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(value, 123u64);
        }

        #[test]
        fn reads_decimal128_as_ubigint() {
            let array = Decimal128Array::from(vec![12345i128])
                .with_precision_and_scale(10, 2)
                .unwrap();
            let field = decimal128_field(10, 2);
            let mut value: UBigInt = 0;

            let result = read_arrow_value(
                CDataType::UBigInt,
                &mut value as *mut UBigInt as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(value, 123u64);
        }
    }

    // Tests for CDataType::SBigInt
    mod read_to_sbigint {
        use super::*;

        #[test]
        fn reads_negative_int64_as_sbigint() {
            let array = Int64Array::from(vec![-9876543210i64]);
            let field = field_with_fixed_meta(DataType::Int64, 0, 20);
            let mut value: SBigInt = 0;

            let result = read_arrow_value(
                CDataType::SBigInt,
                &mut value as *mut SBigInt as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(value, -9876543210i64);
        }

        #[test]
        fn reads_int64_with_scale_drops_decimals() {
            let array = Int64Array::from(vec![-12345i64]); // -123.45 with scale 2
            let field = field_with_fixed_meta(DataType::Int64, 2, 10);
            let mut value: SBigInt = 0;

            let result = read_arrow_value(
                CDataType::SBigInt,
                &mut value as *mut SBigInt as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(value, -123i64);
        }
    }

    // Tests for CDataType::Long and CDataType::SLong
    mod read_to_long {
        use super::*;

        #[test]
        fn reads_int32_as_long() {
            let array = Int32Array::from(vec![123456i32]);
            let field = field_with_fixed_meta(DataType::Int32, 0, 10);
            let mut value: sql::Integer = 0;

            let result = read_arrow_value(
                CDataType::Long,
                &mut value as *mut sql::Integer as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(value, 123456);
        }

        #[test]
        fn reads_negative_int32_as_slong() {
            let array = Int32Array::from(vec![-123456i32]);
            let field = field_with_fixed_meta(DataType::Int32, 0, 10);
            let mut value: sql::Integer = 0;

            let result = read_arrow_value(
                CDataType::SLong,
                &mut value as *mut sql::Integer as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(value, -123456);
        }
    }

    // Tests for CDataType::ULong
    mod read_to_ulong {
        use super::*;

        #[test]
        fn reads_int32_as_ulong() {
            let array = Int32Array::from(vec![123456i32]);
            let field = field_with_fixed_meta(DataType::Int32, 0, 10);
            let mut value: sql::UInteger = 0;

            let result = read_arrow_value(
                CDataType::ULong,
                &mut value as *mut sql::UInteger as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(value, 123456);
        }
    }

    // Tests for CDataType::Short and CDataType::SShort
    mod read_to_short {
        use super::*;

        #[test]
        fn reads_int16_as_short() {
            let array = Int16Array::from(vec![1234i16]);
            let field = field_with_fixed_meta(DataType::Int16, 0, 5);
            let mut value: sql::SmallInt = 0;

            let result = read_arrow_value(
                CDataType::Short,
                &mut value as *mut sql::SmallInt as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(value, 1234);
        }

        #[test]
        fn reads_negative_int16_as_sshort() {
            let array = Int16Array::from(vec![-1234i16]);
            let field = field_with_fixed_meta(DataType::Int16, 0, 5);
            let mut value: sql::SmallInt = 0;

            let result = read_arrow_value(
                CDataType::SShort,
                &mut value as *mut sql::SmallInt as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(value, -1234);
        }
    }

    // Tests for CDataType::UShort
    mod read_to_ushort {
        use super::*;

        #[test]
        fn reads_int16_as_ushort() {
            let array = Int16Array::from(vec![1234i16]);
            let field = field_with_fixed_meta(DataType::Int16, 0, 5);
            let mut value: sql::USmallInt = 0;

            let result = read_arrow_value(
                CDataType::UShort,
                &mut value as *mut sql::USmallInt as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(value, 1234);
        }
    }

    // Tests for CDataType::TinyInt and CDataType::STinyInt
    mod read_to_tinyint {
        use super::*;

        #[test]
        fn reads_int8_as_tinyint() {
            let array = Int8Array::from(vec![42i8]);
            let field = field_with_fixed_meta(DataType::Int8, 0, 3);
            let mut value: sql::SChar = 0;

            let result = read_arrow_value(
                CDataType::TinyInt,
                &mut value as *mut sql::SChar as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(value, 42);
        }

        #[test]
        fn reads_negative_int8_as_stinyint() {
            let array = Int8Array::from(vec![-42i8]);
            let field = field_with_fixed_meta(DataType::Int8, 0, 3);
            let mut value: sql::SChar = 0;

            let result = read_arrow_value(
                CDataType::STinyInt,
                &mut value as *mut sql::SChar as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(value, -42);
        }
    }

    // Tests for CDataType::UTinyInt
    mod read_to_utinyint {
        use super::*;

        #[test]
        fn reads_int8_as_utinyint() {
            let array = Int8Array::from(vec![42i8]);
            let field = field_with_fixed_meta(DataType::Int8, 0, 3);
            let mut value: sql::Char = 0;

            let result = read_arrow_value(
                CDataType::UTinyInt,
                &mut value as *mut sql::Char as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(value, 42);
        }
    }

    // Tests for CDataType::Float
    mod read_to_float {
        use super::*;

        #[test]
        fn reads_int64_with_scale_as_float() {
            let array = Int64Array::from(vec![12345i64]); // 123.45 with scale 2
            let field = field_with_fixed_meta(DataType::Int64, 2, 10);
            let mut value: Real = 0.0;

            let result = read_arrow_value(
                CDataType::Float,
                &mut value as *mut Real as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert!((value - 123.45f32).abs() < 0.01);
        }
    }

    // Tests for CDataType::Double
    mod read_to_double {
        use super::*;

        #[test]
        fn reads_int64_with_scale_as_double() {
            let array = Int64Array::from(vec![12345i64]); // 123.45 with scale 2
            let field = field_with_fixed_meta(DataType::Int64, 2, 10);
            let mut value: Double = 0.0;

            let result = read_arrow_value(
                CDataType::Double,
                &mut value as *mut Double as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert!((value - 123.45f64).abs() < 0.001);
        }

        #[test]
        fn reads_decimal128_as_double() {
            let array = Decimal128Array::from(vec![12345i128])
                .with_precision_and_scale(10, 2)
                .unwrap();
            let field = decimal128_field(10, 2);
            let mut value: Double = 0.0;

            let result = read_arrow_value(
                CDataType::Double,
                &mut value as *mut Double as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert!((value - 123.45f64).abs() < 0.001);
        }
    }

    // Tests for unsupported types
    mod unsupported_types {
        use super::*;

        #[test]
        fn returns_error_for_unsupported_target_type() {
            let array = Int64Array::from(vec![42i64]);
            let field = field_with_fixed_meta(DataType::Int64, 0, 10);
            let mut value: i64 = 0;

            let result = read_arrow_value(
                CDataType::Binary, // Unsupported
                &mut value as *mut i64 as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(matches!(
                result,
                Err(ConversionError::WriteOdbcValue { .. })
            ));
        }

        #[test]
        fn returns_error_for_wchar_target_type() {
            let array = StringArray::from(vec!["hello"]);
            let field = field_with_text_meta();
            let mut buffer = vec![0u16; 32];

            let result = read_arrow_value(
                CDataType::WChar, // Unsupported
                buffer.as_mut_ptr() as sql::Pointer,
                buffer.len() as sql::Len,
                std::ptr::null_mut(),
                &array,
                &field,
                0,
            );

            assert!(matches!(
                result,
                Err(ConversionError::WriteOdbcValue { .. })
            ));
        }
    }

    // Tests for reading from different row indices
    mod row_index_tests {
        use super::*;

        #[test]
        fn reads_from_different_batch_indices() {
            let array = Int64Array::from(vec![100i64, 200i64, 300i64]);
            let field = field_with_fixed_meta(DataType::Int64, 0, 10);

            for (idx, expected) in [(0, 100u64), (1, 200u64), (2, 300u64)] {
                let mut value: UBigInt = 0;

                let result = read_arrow_value(
                    CDataType::UBigInt,
                    &mut value as *mut UBigInt as sql::Pointer,
                    0,
                    std::ptr::null_mut(),
                    &array,
                    &field,
                    idx,
                );

                assert!(result.is_ok());
                assert_eq!(value, expected);
            }
        }

        #[test]
        fn reads_strings_from_different_batch_indices() {
            let array = StringArray::from(vec!["first", "second", "third"]);
            let field = field_with_text_meta();

            for (idx, expected) in [(0, "first"), (1, "second"), (2, "third")] {
                let mut buffer = vec![0u8; 32];
                let mut str_len: sql::Len = 0;

                let result = read_arrow_value(
                    CDataType::Char,
                    buffer.as_mut_ptr() as sql::Pointer,
                    buffer.len() as sql::Len,
                    &mut str_len,
                    &array,
                    &field,
                    idx,
                );

                assert!(result.is_ok());
                assert_eq!(str_len, expected.len() as sql::Len);
                assert_eq!(&buffer[..expected.len()], expected.as_bytes());
            }
        }
    }

    // Tests for null str_len_or_ind_ptr
    mod null_indicator_tests {
        use super::*;

        #[test]
        fn works_with_null_str_len_ptr_for_numeric() {
            let array = Int64Array::from(vec![42i64]);
            let field = field_with_fixed_meta(DataType::Int64, 0, 10);
            let mut value: UBigInt = 0;

            let result = read_arrow_value(
                CDataType::UBigInt,
                &mut value as *mut UBigInt as sql::Pointer,
                0,
                std::ptr::null_mut(), // null indicator
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(value, 42u64);
        }

        #[test]
        fn works_with_null_str_len_ptr_for_char() {
            let array = StringArray::from(vec!["hello"]);
            let field = field_with_text_meta();
            let mut buffer = vec![0u8; 32];

            let result = read_arrow_value(
                CDataType::Char,
                buffer.as_mut_ptr() as sql::Pointer,
                buffer.len() as sql::Len,
                std::ptr::null_mut(), // null indicator
                &array,
                &field,
                0,
            );

            assert!(result.is_ok());
            assert_eq!(&buffer[..5], b"hello");
        }
    }
}
