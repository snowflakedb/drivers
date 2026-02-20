use crate::api::error::{
    ConversionSnafu, DataNotFetchedSnafu, ExecutionDoneSnafu, FetchDataSnafu,
    InvalidBufferLengthSnafu, InvalidCursorPositionSnafu, InvalidCursorStateSnafu,
    InvalidDescriptorIndexSnafu, NoMoreDataSnafu, NullPointerSnafu, StatementErrorStateSnafu,
    StatementNotExecutedSnafu, UnsupportedFeatureSnafu,
};
use crate::api::{
    GetDataState, OdbcResult, Statement, StatementState, WithState, stmt_from_handle,
};
use crate::cdata_types::CDataType;
use crate::conversion::warning::Warnings;
use crate::conversion::{Binding, ConversionError, make_converter};
use arrow::array::Array;
use arrow::datatypes::Field;
use odbc_sys as sql;
use snafu::ResultExt;
use tracing;

fn read_arrow_value(
    binding: &Binding,
    array_ref: &dyn Array,
    field: &Field,
    batch_idx: usize,
    get_data_offset: &mut Option<usize>,
) -> Result<Warnings, ConversionError> {
    let converter = make_converter(field, array_ref)?;
    let warnings = converter.convert_arrow_value(batch_idx, binding, get_data_offset)?;
    Ok(warnings)
}

const SQL_FETCH_NEXT: sql::SmallInt = 1;

#[repr(u16)]
#[derive(Debug, Clone, Copy)]
enum RowStatus {
    Success = 0,
    NoRow = 3,
    Error = 5,
    SuccessWithInfo = 6,
}

/// Advance the cursor by one row. Handles state transitions from
/// `Executed` → `Fetching` and from one batch to the next.
fn advance_cursor(state: &mut crate::api::State<StatementState>) -> OdbcResult<()> {
    state.transition_or_err(|s| match s {
        StatementState::NoResultSet => InvalidCursorStateSnafu
            .fail()
            .with_state(StatementState::NoResultSet),
        StatementState::Executed { mut reader, .. } => match reader.next() {
            Some(rb) => {
                let record_batch = rb
                    .context(FetchDataSnafu)
                    .with_state(StatementState::Error)?;
                Ok((
                    StatementState::Fetching {
                        reader,
                        record_batch,
                        batch_idx: 0,
                    },
                    (),
                ))
            }
            None => NoMoreDataSnafu.fail().with_state(StatementState::Done),
        },
        StatementState::Fetching {
            mut reader,
            record_batch,
            batch_idx,
        } => {
            let new_idx = batch_idx + 1;
            if new_idx < record_batch.num_rows() {
                Ok((
                    StatementState::Fetching {
                        reader,
                        record_batch,
                        batch_idx: new_idx,
                    },
                    (),
                ))
            } else {
                match reader.next() {
                    Some(rb) => {
                        let new_batch = rb
                            .context(FetchDataSnafu)
                            .with_state(StatementState::Error)?;
                        Ok((
                            StatementState::Fetching {
                                reader,
                                record_batch: new_batch,
                                batch_idx: 0,
                            },
                            (),
                        ))
                    }
                    None => NoMoreDataSnafu.fail().with_state(StatementState::Done),
                }
            }
        }
        state @ StatementState::Error => StatementErrorStateSnafu.fail().with_state(state),
        state @ StatementState::Done => ExecutionDoneSnafu.fail().with_state(state),
        state @ StatementState::Created => StatementNotExecutedSnafu.fail().with_state(state),
    })
}

/// Fetch the next rowset of data (block cursor aware).
pub fn fetch(statement_handle: sql::Handle, warnings: &mut Warnings) -> OdbcResult<()> {
    tracing::debug!("fetch called");
    let stmt = stmt_from_handle(statement_handle);
    stmt.get_data_state = None;

    let array_size = stmt.ard.array_size.max(1);
    let bind_type = stmt.ard.bind_type;
    let bind_offset_ptr = stmt.ard.bind_offset_ptr;
    let row_status_ptr = stmt.ird.array_status_ptr;
    let rows_fetched_ptr = stmt.ird.rows_processed_ptr;

    if array_size == 1 && bind_offset_ptr.is_null() {
        advance_cursor(&mut stmt.state)?;
        if !rows_fetched_ptr.is_null() {
            unsafe { *rows_fetched_ptr = 1 };
        }
        match execute_bindings_for_row(stmt, 0, 0, 0) {
            Ok(row_warnings) => {
                let status = if row_warnings.is_empty() {
                    RowStatus::Success
                } else {
                    RowStatus::SuccessWithInfo
                };
                write_row_status(row_status_ptr, 0, status);
                warnings.extend(row_warnings);
            }
            Err(e) => {
                write_row_status(row_status_ptr, 0, RowStatus::Error);
                return Err(e);
            }
        }
        return Ok(());
    }

    let bind_offset = if !bind_offset_ptr.is_null() {
        unsafe { *bind_offset_ptr }
    } else {
        0
    };

    let mut rows_fetched: usize = 0;
    let mut has_error = false;

    for row_idx in 0..array_size {
        match advance_cursor(&mut stmt.state) {
            Ok(()) => {
                rows_fetched += 1;
                match execute_bindings_for_row(stmt, row_idx, bind_type, bind_offset) {
                    Ok(w) => {
                        let status = if w.is_empty() {
                            RowStatus::Success
                        } else {
                            RowStatus::SuccessWithInfo
                        };
                        write_row_status(row_status_ptr, row_idx, status);
                        warnings.extend(w);
                    }
                    Err(_) => {
                        write_row_status(row_status_ptr, row_idx, RowStatus::Error);
                        has_error = true;
                    }
                }
            }
            Err(crate::api::OdbcError::NoMoreData { .. })
            | Err(crate::api::OdbcError::ExecutionDone { .. }) => {
                for remaining in row_idx..array_size {
                    write_row_status(row_status_ptr, remaining, RowStatus::NoRow);
                }
                break;
            }
            Err(e) => {
                if rows_fetched == 0 {
                    return Err(e);
                }
                write_row_status(row_status_ptr, row_idx, RowStatus::Error);
                has_error = true;
                for remaining in (row_idx + 1)..array_size {
                    write_row_status(row_status_ptr, remaining, RowStatus::NoRow);
                }
                break;
            }
        }
    }

    if !rows_fetched_ptr.is_null() {
        unsafe { *rows_fetched_ptr = rows_fetched as sql::ULen };
    }

    if rows_fetched == 0 {
        return NoMoreDataSnafu.fail();
    }

    if has_error {
        warnings.push(crate::conversion::warning::Warning::RowError);
    }

    Ok(())
}

/// `SQLFetchScroll` — currently only `SQL_FETCH_NEXT` is supported.
pub fn fetch_scroll(
    statement_handle: sql::Handle,
    fetch_orientation: sql::SmallInt,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    if fetch_orientation != SQL_FETCH_NEXT {
        tracing::warn!(
            "fetch_scroll: unsupported orientation {}",
            fetch_orientation
        );
        return UnsupportedFeatureSnafu.fail();
    }
    fetch(statement_handle, warnings)
}

fn write_row_status(row_status_ptr: *mut u16, row_idx: usize, status: RowStatus) {
    if !row_status_ptr.is_null() {
        unsafe { *row_status_ptr.add(row_idx) = status as u16 };
    }
}

/// Compute the data stride (in bytes) between successive rows for a column
/// when using column-wise binding.
fn column_wise_data_stride(binding: &Binding) -> usize {
    if binding.buffer_length > 0 {
        binding.buffer_length as usize
    } else {
        binding.target_type.fixed_size().unwrap_or(8)
    }
}

/// Create an adjusted `Binding` whose pointers target `row_idx` within the
/// bound arrays, taking into account column-wise vs row-wise binding and an
/// optional bind offset.
fn adjust_binding_for_row(
    binding: &Binding,
    row_idx: usize,
    bind_type: usize,
    bind_offset: isize,
) -> Binding {
    if row_idx == 0 && bind_offset == 0 {
        return Binding {
            target_type: binding.target_type,
            target_value_ptr: binding.target_value_ptr,
            buffer_length: binding.buffer_length,
            str_len_or_ind_ptr: binding.str_len_or_ind_ptr,
            precision: binding.precision,
            scale: binding.scale,
        };
    }

    let (data_ptr, ind_ptr) = if bind_type == 0 {
        let stride = column_wise_data_stride(binding);
        let row_offset = row_idx
            .checked_mul(stride)
            .expect("row index and stride multiplication overflowed");
        let data_ptr = unsafe {
            (binding.target_value_ptr as *mut u8)
                .offset(bind_offset)
                .add(row_offset) as sql::Pointer
        };
        let ind_ptr = if !binding.str_len_or_ind_ptr.is_null() {
            let ind_stride = std::mem::size_of::<sql::Len>();
            let ind_offset = row_idx
                .checked_mul(ind_stride)
                .expect("row index and indicator stride multiplication overflowed");
            unsafe {
                (binding.str_len_or_ind_ptr as *mut u8)
                    .offset(bind_offset)
                    .add(ind_offset) as *mut sql::Len
            }
        } else {
            std::ptr::null_mut()
        };
        (data_ptr, ind_ptr)
    } else {
        let row_byte_offset = row_idx
            .checked_mul(bind_type)
            .expect("row index and bind type multiplication overflowed");
        let data_ptr = unsafe {
            (binding.target_value_ptr as *mut u8)
                .offset(bind_offset)
                .add(row_byte_offset) as sql::Pointer
        };
        let ind_ptr = if !binding.str_len_or_ind_ptr.is_null() {
            unsafe {
                (binding.str_len_or_ind_ptr as *mut u8)
                    .offset(bind_offset)
                    .add(row_byte_offset) as *mut sql::Len
            }
        } else {
            std::ptr::null_mut()
        };
        (data_ptr, ind_ptr)
    };

    Binding {
        target_type: binding.target_type,
        target_value_ptr: data_ptr,
        buffer_length: binding.buffer_length,
        str_len_or_ind_ptr: ind_ptr,
        precision: binding.precision,
        scale: binding.scale,
    }
}

/// Execute column bindings for a single row within a block-cursor fetch.
fn execute_bindings_for_row(
    stmt: &mut Statement,
    row_idx: usize,
    bind_type: usize,
    bind_offset: isize,
) -> OdbcResult<Warnings> {
    let mut warnings = vec![];
    if let StatementState::Fetching {
        record_batch,
        batch_idx,
        ..
    } = stmt.state.as_ref()
    {
        let batch_idx = *batch_idx;
        let schema = record_batch.schema();
        let bindings: Vec<(u16, Binding)> = stmt
            .ard
            .bindings
            .iter()
            .map(|(&col, b)| {
                (
                    col,
                    adjust_binding_for_row(b, row_idx, bind_type, bind_offset),
                )
            })
            .collect();

        for (column_number, adjusted) in &bindings {
            let arrow_col = *column_number as usize - 1;
            if arrow_col >= schema.fields().len() {
                tracing::error!(
                    "execute_bindings_for_row: column_number {} is out of range",
                    column_number
                );
                continue;
            }
            let array_ref = record_batch.column(arrow_col);
            let field = schema.field(arrow_col);
            let w = read_arrow_value(adjusted, array_ref, field, batch_idx, &mut None)
                .context(ConversionSnafu)?;
            warnings.extend(w);
        }
    }
    Ok(warnings)
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

    if target_value_ptr.is_null() {
        return NullPointerSnafu.fail();
    }

    if buffer_length < 0 {
        return InvalidBufferLengthSnafu {
            length: buffer_length as i64,
        }
        .fail();
    }

    let stmt = stmt_from_handle(statement_handle);

    if stmt.ard.array_size > 1 {
        tracing::warn!("get_data: cannot use SQLGetData with row_array_size > 1");
        return InvalidCursorPositionSnafu.fail();
    }

    // Column 0 is reserved for bookmarks; reject it when bookmarks are off
    if col_or_param_num == 0 {
        return InvalidDescriptorIndexSnafu { number: 0i16 }.fail();
    }

    // SQL_ARD_TYPE: resolve from the ARD descriptor's concise type for the column
    let target_type = if target_type == CDataType::Ard {
        match stmt.ard.bindings.get(&col_or_param_num) {
            Some(b) => b.target_type,
            None => {
                return InvalidDescriptorIndexSnafu {
                    number: col_or_param_num as sql::SmallInt,
                }
                .fail();
            }
        }
    } else {
        target_type
    };

    // Handle state from previous SQLGetData calls
    let mut offset: Option<usize> = None;
    if let Some(ref state) = stmt.get_data_state
        && state.col() == col_or_param_num
    {
        match state {
            GetDataState::Completed { .. } => {
                stmt.get_data_state = None;
                return NoMoreDataSnafu.fail();
            }
            GetDataState::Partial {
                offset: prev_offset,
                ..
            } => {
                offset = Some(*prev_offset);
            }
        }
    }
    // Clear state before proceeding (will be re-set below based on result)
    stmt.get_data_state = None;

    match stmt.state.as_ref() {
        StatementState::Fetching {
            reader: _,
            record_batch,
            batch_idx,
        } => {
            let col_idx = (col_or_param_num - 1) as usize;
            if col_idx >= record_batch.num_columns() {
                return InvalidDescriptorIndexSnafu {
                    number: col_or_param_num as sql::SmallInt,
                }
                .fail();
            }
            let array_ref = record_batch.column(col_idx);
            let schema = record_batch.schema();
            let field = schema.field(col_idx);

            let binding = Binding {
                target_type,
                target_value_ptr,
                buffer_length,
                str_len_or_ind_ptr,
                precision: None,
                scale: None,
            };
            let conversion_warnings =
                read_arrow_value(&binding, array_ref, field, *batch_idx, &mut offset)
                    .context(ConversionSnafu)?;
            warnings.extend(conversion_warnings);

            // The write method sets offset to Some(n) on truncation, None when complete.
            match offset {
                Some(new_offset) => {
                    stmt.get_data_state = Some(GetDataState::Partial {
                        col: col_or_param_num,
                        offset: new_offset,
                    });
                }
                None => {
                    stmt.get_data_state = Some(GetDataState::Completed {
                        col: col_or_param_num,
                    });
                }
            }

            Ok(())
        }
        StatementState::NoResultSet => {
            tracing::error!("get_data: no result set associated with the statement");
            NoMoreDataSnafu.fail()
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

            let binding = Binding {
                target_type: CDataType::Char,
                target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
                buffer_length: buffer.len() as sql::Len,
                str_len_or_ind_ptr: &mut str_len,
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

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

            let binding = Binding {
                target_type: CDataType::Char,
                target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
                buffer_length: buffer.len() as sql::Len,
                str_len_or_ind_ptr: &mut str_len,
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

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

            let binding = Binding {
                target_type: CDataType::Char,
                target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
                buffer_length: buffer.len() as sql::Len,
                str_len_or_ind_ptr: &mut str_len,
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

            assert!(result.is_ok());
            assert_eq!(str_len, sql::NO_TOTAL); // SQL_NO_TOTAL when truncated
            assert_eq!(&buffer[..4], b"hell"); // Truncated with null terminator at position 4
            assert_eq!(buffer[4], 0); // Null terminator
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

            let binding = Binding {
                target_type: CDataType::UBigInt,
                target_value_ptr: &mut value as *mut UBigInt as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

            assert!(result.is_ok());
            assert_eq!(value, 9876543210u64);
        }

        #[test]
        fn reads_int64_with_scale_drops_decimals() {
            let array = Int64Array::from(vec![12345i64]); // 123.45 with scale 2
            let field = field_with_fixed_meta(DataType::Int64, 2, 10);
            let mut value: UBigInt = 0;

            let binding = Binding {
                target_type: CDataType::UBigInt,
                target_value_ptr: &mut value as *mut UBigInt as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

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

            let binding = Binding {
                target_type: CDataType::UBigInt,
                target_value_ptr: &mut value as *mut UBigInt as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

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

            let binding = Binding {
                target_type: CDataType::SBigInt,
                target_value_ptr: &mut value as *mut SBigInt as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

            assert!(result.is_ok());
            assert_eq!(value, -9876543210i64);
        }

        #[test]
        fn reads_int64_with_scale_drops_decimals() {
            let array = Int64Array::from(vec![-12345i64]); // -123.45 with scale 2
            let field = field_with_fixed_meta(DataType::Int64, 2, 10);
            let mut value: SBigInt = 0;

            let binding = Binding {
                target_type: CDataType::SBigInt,
                target_value_ptr: &mut value as *mut SBigInt as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

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

            let binding = Binding {
                target_type: CDataType::Long,
                target_value_ptr: &mut value as *mut sql::Integer as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

            assert!(result.is_ok());
            assert_eq!(value, 123456);
        }

        #[test]
        fn reads_negative_int32_as_slong() {
            let array = Int32Array::from(vec![-123456i32]);
            let field = field_with_fixed_meta(DataType::Int32, 0, 10);
            let mut value: sql::Integer = 0;

            let binding = Binding {
                target_type: CDataType::SLong,
                target_value_ptr: &mut value as *mut sql::Integer as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

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

            let binding = Binding {
                target_type: CDataType::ULong,
                target_value_ptr: &mut value as *mut sql::UInteger as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

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

            let binding = Binding {
                target_type: CDataType::Short,
                target_value_ptr: &mut value as *mut sql::SmallInt as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

            assert!(result.is_ok());
            assert_eq!(value, 1234);
        }

        #[test]
        fn reads_negative_int16_as_sshort() {
            let array = Int16Array::from(vec![-1234i16]);
            let field = field_with_fixed_meta(DataType::Int16, 0, 5);
            let mut value: sql::SmallInt = 0;

            let binding = Binding {
                target_type: CDataType::SShort,
                target_value_ptr: &mut value as *mut sql::SmallInt as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

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

            let binding = Binding {
                target_type: CDataType::UShort,
                target_value_ptr: &mut value as *mut sql::USmallInt as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

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

            let binding = Binding {
                target_type: CDataType::TinyInt,
                target_value_ptr: &mut value as *mut sql::SChar as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

            assert!(result.is_ok());
            assert_eq!(value, 42);
        }

        #[test]
        fn reads_negative_int8_as_stinyint() {
            let array = Int8Array::from(vec![-42i8]);
            let field = field_with_fixed_meta(DataType::Int8, 0, 3);
            let mut value: sql::SChar = 0;

            let binding = Binding {
                target_type: CDataType::STinyInt,
                target_value_ptr: &mut value as *mut sql::SChar as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

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

            let binding = Binding {
                target_type: CDataType::UTinyInt,
                target_value_ptr: &mut value as *mut sql::Char as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

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

            let binding = Binding {
                target_type: CDataType::Float,
                target_value_ptr: &mut value as *mut Real as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

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

            let binding = Binding {
                target_type: CDataType::Double,
                target_value_ptr: &mut value as *mut Double as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

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

            let binding = Binding {
                target_type: CDataType::Double,
                target_value_ptr: &mut value as *mut Double as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

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

            let binding = Binding {
                target_type: CDataType::Binary, // Unsupported
                target_value_ptr: &mut value as *mut i64 as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

            assert!(matches!(
                result,
                Err(ConversionError::WriteOdbcValue { .. })
            ));
        }

        #[test]
        fn successfully_reads_wchar_target_type() {
            let array = StringArray::from(vec!["hello"]);
            let field = field_with_text_meta();
            let mut buffer = vec![0u16; 32];
            let mut str_len: sql::Len = 0;

            let binding = Binding {
                target_type: CDataType::WChar,
                target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
                buffer_length: (buffer.len() * 2) as sql::Len, // buffer_length is in bytes for WChar
                str_len_or_ind_ptr: &mut str_len,
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

            assert!(result.is_ok());
            assert_eq!(str_len, 10); // "hello" is 5 UTF-16 code units = 10 bytes
            assert_eq!(
                &buffer[..5],
                &[
                    b'h' as u16,
                    b'e' as u16,
                    b'l' as u16,
                    b'l' as u16,
                    b'o' as u16
                ]
            );
            assert_eq!(buffer[5], 0); // Null terminator
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

                let binding = Binding {
                    target_type: CDataType::UBigInt,
                    target_value_ptr: &mut value as *mut UBigInt as sql::Pointer,
                    buffer_length: 0,
                    str_len_or_ind_ptr: std::ptr::null_mut(),
                    precision: None,
                    scale: None,
                };
                let result = read_arrow_value(&binding, &array, &field, idx, &mut None);

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

                let binding = Binding {
                    target_type: CDataType::Char,
                    target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
                    buffer_length: buffer.len() as sql::Len,
                    str_len_or_ind_ptr: &mut str_len,
                    precision: None,
                    scale: None,
                };
                let result = read_arrow_value(&binding, &array, &field, idx, &mut None);

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

            let binding = Binding {
                target_type: CDataType::UBigInt,
                target_value_ptr: &mut value as *mut UBigInt as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(), // null indicator
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

            assert!(result.is_ok());
            assert_eq!(value, 42u64);
        }

        #[test]
        fn works_with_null_str_len_ptr_for_char() {
            let array = StringArray::from(vec!["hello"]);
            let field = field_with_text_meta();
            let mut buffer = vec![0u8; 32];

            let binding = Binding {
                target_type: CDataType::Char,
                target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
                buffer_length: buffer.len() as sql::Len,
                str_len_or_ind_ptr: std::ptr::null_mut(), // null indicator
                precision: None,
                scale: None,
            };
            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

            assert!(result.is_ok());
            assert_eq!(&buffer[..5], b"hello");
        }
    }

    // Tests for CDataType::Numeric (SQL_C_NUMERIC)
    mod read_to_numeric {
        use super::*;

        #[test]
        fn reads_int64_as_numeric_with_rescale() {
            // SELECT 12345 AS value -> Arrow stores 12345 with scale=0
            // Application requests SQL_C_NUMERIC with precision=10, scale=2
            // Expected: unscaled value = 12345 * 10^2 = 1234500, sign=1
            let array = Int64Array::from(vec![12345i64]);
            let field = field_with_fixed_meta(DataType::Int64, 0, 10);
            let mut numeric = sql::Numeric::default();
            let mut indicator: sql::Len = 0;

            let binding = Binding {
                target_type: CDataType::Numeric,
                target_value_ptr: &mut numeric as *mut sql::Numeric as sql::Pointer,
                buffer_length: std::mem::size_of::<sql::Numeric>() as sql::Len,
                str_len_or_ind_ptr: &mut indicator,
                precision: Some(10),
                scale: Some(2),
            };

            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

            assert!(result.is_ok());
            assert_eq!(numeric.sign, 1); // positive
            assert_eq!(numeric.precision, 10);
            assert_eq!(numeric.scale, 2);

            // Reconstruct value from little-endian val[]
            let mut reconstructed: u128 = 0;
            for i in (0..16).rev() {
                reconstructed = (reconstructed << 8) | numeric.val[i] as u128;
            }
            assert_eq!(reconstructed, 1234500);
        }

        #[test]
        fn reads_negative_int64_as_numeric() {
            let array = Int64Array::from(vec![-500i64]);
            let field = field_with_fixed_meta(DataType::Int64, 0, 5);
            let mut numeric = sql::Numeric::default();

            let binding = Binding {
                target_type: CDataType::Numeric,
                target_value_ptr: &mut numeric as *mut sql::Numeric as sql::Pointer,
                buffer_length: std::mem::size_of::<sql::Numeric>() as sql::Len,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: Some(5),
                scale: Some(0),
            };

            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

            assert!(result.is_ok());
            assert_eq!(numeric.sign, 0); // negative
            assert_eq!(numeric.precision, 5);
            assert_eq!(numeric.scale, 0);

            let mut reconstructed: u128 = 0;
            for i in (0..16).rev() {
                reconstructed = (reconstructed << 8) | numeric.val[i] as u128;
            }
            assert_eq!(reconstructed, 500);
        }

        #[test]
        fn reads_decimal128_as_numeric() {
            // Decimal128 value 12345 with arrow scale=2 => actual value 123.45
            // Target: precision=10, scale=4 => unscaled = 1234500
            let array = Decimal128Array::from(vec![12345i128])
                .with_precision_and_scale(10, 2)
                .unwrap();
            let field = decimal128_field(10, 2);
            let mut numeric = sql::Numeric::default();

            let binding = Binding {
                target_type: CDataType::Numeric,
                target_value_ptr: &mut numeric as *mut sql::Numeric as sql::Pointer,
                buffer_length: std::mem::size_of::<sql::Numeric>() as sql::Len,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: Some(10),
                scale: Some(4),
            };

            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);

            assert!(result.is_ok());
            assert_eq!(numeric.sign, 1);
            assert_eq!(numeric.precision, 10);
            assert_eq!(numeric.scale, 4);

            let mut reconstructed: u128 = 0;
            for i in (0..16).rev() {
                reconstructed = (reconstructed << 8) | numeric.val[i] as u128;
            }
            // 12345 (arrow) * 10^(4-2) = 12345 * 100 = 1234500
            assert_eq!(reconstructed, 1234500);
        }

        #[test]
        fn returns_error_without_precision() {
            let array = Int64Array::from(vec![42i64]);
            let field = field_with_fixed_meta(DataType::Int64, 0, 10);
            let mut numeric = sql::Numeric::default();

            let binding = Binding {
                target_type: CDataType::Numeric,
                target_value_ptr: &mut numeric as *mut sql::Numeric as sql::Pointer,
                buffer_length: std::mem::size_of::<sql::Numeric>() as sql::Len,
                str_len_or_ind_ptr: std::ptr::null_mut(),
                precision: None, // not set
                scale: Some(0),
            };

            let result = read_arrow_value(&binding, &array, &field, 0, &mut None);
            assert!(result.is_err());
        }
    }
}
