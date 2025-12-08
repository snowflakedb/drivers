use crate::api::error::{
    ArrowReadSnafu, DataNotFetchedSnafu, ExecutionDoneSnafu, FetchDataSnafu,
    InvalidColumnNumberSnafu, NoMoreDataSnafu, StatementErrorStateSnafu, StatementNotExecutedSnafu,
};
use crate::api::{
    OdbcError, OdbcResult, ParameterBinding, Statement, StatementState, WithState, stmt_from_handle,
};
use crate::cdata_types::{CDataType, Double, Real, SBigInt, UBigInt};
use crate::read_arrow::{
    Buffer, ExtractError, ReadArrowValue, Value, set_read_session_timezone,
    set_read_timestamp_ltz_format, set_read_timestamp_ntz_format, set_read_timestamp_tz_format,
};
use arrow::{array::Array, datatypes::Field};
use odbc_sys as sql;
use snafu::ResultExt;
use std::io::Write;
use std::ptr;
use tracing;

const SQL_ROW_SUCCESS: sql::USmallInt = 0;
const SQL_ROW_NOROW: sql::USmallInt = 3;
const SQL_BIND_BY_COLUMN_VALUE: usize = 0;

fn read_arrow_value(
    target_type: CDataType,
    target_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
    array_ref: &dyn Array,
    field: &Field,
    batch_idx: usize,
) -> Result<(), ExtractError> {
    match target_type {
        CDataType::Char => {
            let buffer = Buffer::new(
                target_value_ptr as *mut sql::Char,
                buffer_length as usize,
                str_len_or_ind_ptr,
            );
            ReadArrowValue::read(buffer, array_ref, field, batch_idx)
        }
        CDataType::WChar => {
            let wchar_size = std::mem::size_of::<sql::WChar>();
            let len_bytes = if buffer_length <= 0 {
                0
            } else {
                buffer_length as usize
            };
            let char_capacity = if wchar_size == 0 {
                0
            } else {
                len_bytes / wchar_size
            };
            let buffer = Buffer::new(
                target_value_ptr as *mut sql::WChar,
                char_capacity,
                str_len_or_ind_ptr,
            );
            ReadArrowValue::read(buffer, array_ref, field, batch_idx)
        }
        CDataType::UBigInt => ReadArrowValue::read(
            Value::new(target_value_ptr as *mut UBigInt),
            array_ref,
            field,
            batch_idx,
        ),
        CDataType::SBigInt => ReadArrowValue::read(
            Value::new(target_value_ptr as *mut SBigInt),
            array_ref,
            field,
            batch_idx,
        ),
        CDataType::Long | CDataType::SLong => {
            let sink = Value::new(target_value_ptr as *mut sql::Integer);
            let sink = sink.contramap::<SBigInt>(|v| v as sql::Integer);
            ReadArrowValue::read(sink, array_ref, field, batch_idx)
        }
        CDataType::ULong => {
            let sink = Value::new(target_value_ptr as *mut sql::UInteger);
            let sink = sink.contramap::<UBigInt>(|v| v as sql::UInteger);
            ReadArrowValue::read(sink, array_ref, field, batch_idx)
        }
        CDataType::SShort | CDataType::Short => {
            let sink = Value::new(target_value_ptr as *mut sql::SmallInt);
            let sink = sink.contramap::<SBigInt>(|v| v as sql::SmallInt);
            ReadArrowValue::read(sink, array_ref, field, batch_idx)
        }
        CDataType::UShort => {
            let sink = Value::new(target_value_ptr as *mut sql::USmallInt);
            let sink = sink.contramap::<UBigInt>(|v| v as sql::USmallInt);
            ReadArrowValue::read(sink, array_ref, field, batch_idx)
        }
        CDataType::STinyInt | CDataType::TinyInt => {
            let sink = Value::new(target_value_ptr as *mut sql::SChar);
            let sink = sink.contramap::<SBigInt>(|v| v as sql::SChar);
            ReadArrowValue::read(sink, array_ref, field, batch_idx)
        }
        CDataType::UTinyInt => {
            let sink = Value::new(target_value_ptr as *mut sql::Char);
            let sink = sink.contramap::<UBigInt>(|v| v as sql::Char);
            ReadArrowValue::read(sink, array_ref, field, batch_idx)
        }
        CDataType::Float => {
            let sink = Value::new(target_value_ptr as *mut Real);
            let sink = sink.contramap::<Double>(|v| v as Real);
            ReadArrowValue::read(sink, array_ref, field, batch_idx)
        }
        CDataType::Double => {
            let sink = Value::new(target_value_ptr as *mut Double);
            ReadArrowValue::read(sink, array_ref, field, batch_idx)
        }
        CDataType::TypeTimestamp | CDataType::TimeStamp => {
            let sink = Value::new(target_value_ptr as *mut sql::Timestamp);
            ReadArrowValue::read(sink, array_ref, field, batch_idx)
        }
        CDataType::TypeDate | CDataType::Date => {
            let sink = Value::new(target_value_ptr as *mut sql::Date);
            ReadArrowValue::read(sink, array_ref, field, batch_idx)
        }
        CDataType::TypeTime | CDataType::Time => {
            // Check for null and set indicator before reading
            if array_ref.is_null(batch_idx) {
                if !str_len_or_ind_ptr.is_null() {
                    unsafe { std::ptr::write(str_len_or_ind_ptr, sql::NULL_DATA) };
                }
                return Ok(());
            }
            let sink = Value::new(target_value_ptr as *mut sql::Time);
            let result = ReadArrowValue::read(sink, array_ref, field, batch_idx);
            // Set indicator to size of struct on success
            if result.is_ok() && !str_len_or_ind_ptr.is_null() {
                unsafe {
                    std::ptr::write(
                        str_len_or_ind_ptr,
                        std::mem::size_of::<sql::Time>() as sql::Len,
                    )
                };
            }
            result
        }
        CDataType::Bit => {
            let sink = Value::new(target_value_ptr as *mut sql::SChar);
            let sink = sink.contramap::<SBigInt>(|v| v as sql::SChar);
            ReadArrowValue::read(sink, array_ref, field, batch_idx)
        }
        _ => Err(ExtractError::UnsupportedTargetType(target_type)),
    }
}

/// Fetch the next row of data
pub fn fetch(statement_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!("fetch called");
    let stmt = stmt_from_handle(statement_handle);
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_debug.log")
    {
        let _ = writeln!(
            file,
            "fetch: stmt_handle={:?} session_timezone={:?}",
            statement_handle, stmt.session_timezone
        );
    }
    tracing::debug!("fetch: using session_timezone={:?}", stmt.session_timezone);
    set_read_session_timezone(stmt.session_timezone.clone());
    set_read_timestamp_ltz_format(stmt.conn.timestamp_ltz_format);
    set_read_timestamp_ntz_format(stmt.conn.timestamp_ntz_format);
    set_read_timestamp_tz_format(stmt.conn.timestamp_tz_format);

    if stmt.max_rows > 0 && stmt.current_row >= stmt.max_rows {
        stmt.state = StatementState::Done.into();
        return NoMoreDataSnafu.fail();
    }

    let row_array_size = stmt.row_array_size.max(1);
    let mut rows_fetched_this_call = 0usize;
    let mut end_of_data = false;

    while rows_fetched_this_call < row_array_size {
        if stmt.max_rows > 0 && stmt.current_row >= stmt.max_rows {
            end_of_data = true;
            break;
        }

        match advance_fetch_state(stmt) {
            Ok(()) => {
                if let Some(row_index) = current_batch_row_index(stmt) {
                    populate_bound_columns_for_row(stmt, row_index, rows_fetched_this_call);
                }
                rows_fetched_this_call += 1;
                stmt.current_row += 1;
            }
            Err(OdbcError::NoMoreData { .. }) => {
                end_of_data = true;
                break;
            }
            Err(err) => return Err(err),
        }
    }

    if rows_fetched_this_call == 0 {
        return NoMoreDataSnafu.fail();
    }

    update_rowset_metadata(stmt, rows_fetched_this_call);

    if end_of_data || (stmt.max_rows > 0 && stmt.current_row >= stmt.max_rows) {
        stmt.state = StatementState::Done.into();
    }

    Ok(())
}

fn advance_fetch_state(stmt: &mut Statement) -> OdbcResult<()> {
    stmt.state.transition_or_err(|state| match state {
        StatementState::Executed { reader, .. } => load_next_batch(reader),
        StatementState::Fetching {
            mut reader,
            record_batch,
            batch_idx,
        } => {
            if batch_idx < record_batch.num_rows() {
                let next_batch_idx = batch_idx + 1;
                Ok((
                    StatementState::Fetching {
                        reader,
                        record_batch,
                        batch_idx: next_batch_idx,
                    },
                    (),
                ))
            } else {
                load_next_batch(reader)
            }
        }
        state @ StatementState::Error => {
            tracing::error!("fetch: statement error");
            StatementErrorStateSnafu.fail().with_state(state)
        }
        state @ StatementState::Done => NoMoreDataSnafu.fail().with_state(state),
        state @ StatementState::Created => {
            tracing::error!("fetch: statement not executed");
            StatementNotExecutedSnafu.fail().with_state(state)
        }
    })
}

fn current_batch_row_index(stmt: &Statement) -> Option<usize> {
    match stmt.state.as_ref() {
        StatementState::Fetching { batch_idx, .. } => {
            if *batch_idx == 0 {
                None
            } else {
                Some(*batch_idx - 1)
            }
        }
        _ => None,
    }
}

fn populate_bound_columns_for_row(stmt: &Statement, batch_row_index: usize, rowset_row: usize) {
    if stmt.column_bindings.is_empty() {
        return;
    }

    let (record_batch, schema) = match stmt.state.as_ref() {
        StatementState::Fetching { record_batch, .. } => (record_batch, record_batch.schema()),
        _ => return,
    };

    for (col_num, binding) in &stmt.column_bindings {
        let col_idx = (*col_num - 1) as usize;
        if col_idx >= record_batch.num_columns() {
            continue;
        }
        let value_ptr = if let Some(ptr) =
            compute_value_ptr(binding, stmt.row_bind_type, rowset_row, stmt.row_array_size)
        {
            ptr
        } else {
            tracing::warn!("fetch: column {col_num} has no valid buffer for row {rowset_row}");
            continue;
        };
        let indicator_ptr = compute_indicator_ptr(binding, stmt.row_bind_type, rowset_row);
        let array_ref = record_batch.column(col_idx);
        let field = schema.field(col_idx);
        if let Err(e) = read_arrow_value(
            binding.value_type,
            value_ptr,
            binding.buffer_length,
            indicator_ptr,
            array_ref,
            field,
            batch_row_index,
        ) {
            tracing::error!("fetch: failed to populate bound column {col_num}: {e:?}");
        }
    }
}

fn compute_value_ptr(
    binding: &ParameterBinding,
    row_bind_type: usize,
    rowset_row: usize,
    row_array_size: usize,
) -> Option<sql::Pointer> {
    if binding.parameter_value_ptr.is_null() {
        return None;
    }

    let offset = if row_bind_type == SQL_BIND_BY_COLUMN_VALUE {
        let stride = column_stride(binding, row_array_size)?;
        stride.checked_mul(rowset_row)?
    } else {
        row_bind_type.checked_mul(rowset_row)?
    };

    unsafe { Some((binding.parameter_value_ptr as *mut u8).add(offset) as sql::Pointer) }
}

fn column_stride(binding: &ParameterBinding, row_array_size: usize) -> Option<usize> {
    if binding.buffer_length > 0 {
        let base = binding.buffer_length as usize;
        if row_array_size > 1 {
            if let Some(elem_size) = fixed_size_of_c_type(binding.value_type) {
                let required = elem_size.saturating_mul(row_array_size);
                if base >= required && elem_size > 0 {
                    return Some(elem_size);
                }
            }
        }
        Some(base)
    } else {
        default_stride_for_type(binding.value_type)
    }
}

fn default_stride_for_type(value_type: CDataType) -> Option<usize> {
    use std::mem::size_of;
    Some(match value_type {
        CDataType::Double => size_of::<Double>(),
        CDataType::Float => size_of::<Real>(),
        CDataType::SBigInt | CDataType::UBigInt => size_of::<SBigInt>(),
        CDataType::SLong | CDataType::Long | CDataType::ULong => size_of::<sql::Integer>(),
        CDataType::SShort | CDataType::Short | CDataType::UShort => size_of::<sql::SmallInt>(),
        CDataType::STinyInt | CDataType::TinyInt | CDataType::UTinyInt | CDataType::Bit => {
            size_of::<sql::SChar>()
        }
        CDataType::TypeTimestamp | CDataType::TimeStamp => size_of::<sql::Timestamp>(),
        CDataType::TypeDate | CDataType::Date => size_of::<sql::Date>(),
        CDataType::TypeTime | CDataType::Time => size_of::<sql::Time>(),
        CDataType::Numeric => return None,
        CDataType::Char | CDataType::WChar | CDataType::Binary => return None,
        _ => return None,
    })
}

fn fixed_size_of_c_type(value_type: CDataType) -> Option<usize> {
    use std::mem::size_of;
    Some(match value_type {
        CDataType::Double => size_of::<Double>(),
        CDataType::Float => size_of::<Real>(),
        CDataType::SBigInt | CDataType::UBigInt => size_of::<SBigInt>(),
        CDataType::SLong | CDataType::Long | CDataType::ULong => size_of::<sql::Integer>(),
        CDataType::SShort | CDataType::Short | CDataType::UShort => size_of::<sql::SmallInt>(),
        CDataType::STinyInt | CDataType::TinyInt | CDataType::UTinyInt | CDataType::Bit => {
            size_of::<sql::SChar>()
        }
        CDataType::TypeTimestamp | CDataType::TimeStamp => size_of::<sql::Timestamp>(),
        CDataType::TypeDate | CDataType::Date => size_of::<sql::Date>(),
        CDataType::TypeTime | CDataType::Time => size_of::<sql::Time>(),
        CDataType::Numeric => return None,
        CDataType::Char | CDataType::WChar | CDataType::Binary => return None,
        _ => return None,
    })
}

fn compute_indicator_ptr(
    binding: &ParameterBinding,
    row_bind_type: usize,
    rowset_row: usize,
) -> *mut sql::Len {
    if binding.str_len_or_ind_ptr.is_null() {
        return std::ptr::null_mut();
    }
    if row_bind_type == SQL_BIND_BY_COLUMN_VALUE {
        unsafe { binding.str_len_or_ind_ptr.add(rowset_row) }
    } else {
        let offset = row_bind_type.checked_mul(rowset_row).unwrap_or(0);
        unsafe { (binding.str_len_or_ind_ptr as *mut u8).add(offset) as *mut sql::Len }
    }
}

fn update_rowset_metadata(stmt: &mut Statement, rows_fetched: usize) {
    if let Some(ptr) = stmt.rows_fetched_ptr {
        unsafe {
            ptr::write_unaligned(ptr, rows_fetched as sql::ULen);
        }
    }
    if let Some(ptr) = stmt.row_status_ptr {
        let row_array_size = stmt.row_array_size.max(1);
        for idx in 0..rows_fetched {
            unsafe {
                ptr::write_unaligned(ptr.add(idx), SQL_ROW_SUCCESS);
            }
        }
        for idx in rows_fetched..row_array_size {
            unsafe {
                ptr::write_unaligned(ptr.add(idx), SQL_ROW_NOROW);
            }
        }
    }
}

fn load_next_batch(
    mut reader: arrow::ffi_stream::ArrowArrayStreamReader,
) -> Result<(StatementState, ()), (StatementState, OdbcError)> {
    while let Some(new_record_batch_result) = reader.next() {
        let new_record_batch = new_record_batch_result
            .context(FetchDataSnafu)
            .with_state(StatementState::Error)?;
        tracing::debug!(
            "fetch: got next batch with {} rows",
            new_record_batch.num_rows()
        );
        if new_record_batch.num_rows() == 0 {
            tracing::debug!("fetch: batch contains zero rows, continuing");
            continue;
        }
        let next_state = StatementState::Fetching {
            reader,
            record_batch: new_record_batch,
            batch_idx: 1,
        };
        return Ok((next_state, ()));
    }
    tracing::debug!("fetch: no more batches available from reader");
    NoMoreDataSnafu.fail().with_state(StatementState::Done)
}

/// Bind a column to an application buffer
pub fn bind_col(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    target_type: CDataType,
    target_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
) -> OdbcResult<()> {
    tracing::debug!(
        "bind_col: column_number={}, target_type={:?}",
        column_number,
        target_type
    );

    if column_number == 0 {
        tracing::error!("bind_col: column_number cannot be 0");
        return InvalidColumnNumberSnafu.fail();
    }

    let stmt = stmt_from_handle(statement_handle);

    // Store the column binding (reuse ParameterBinding struct)
    let binding = crate::api::ParameterBinding {
        parameter_type: sql::SqlDataType(0), // Not used for column bindings
        value_type: target_type,
        parameter_value_ptr: target_value_ptr,
        buffer_length,
        str_len_or_ind_ptr,
        owned_buffer: None,
    };

    stmt.column_bindings.insert(column_number, binding);

    tracing::info!("bind_col: Successfully bound column {}", column_number);
    Ok(())
}

/// Get data from a specific column (or use bound column if available)
pub fn get_data(
    statement_handle: sql::Handle,
    col_or_param_num: sql::USmallInt,
    target_type: CDataType,
    target_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
) -> OdbcResult<()> {
    tracing::debug!("get_data: statement_handle={:?}", statement_handle);
    let stmt = stmt_from_handle(statement_handle);
    set_read_session_timezone(stmt.session_timezone.clone());
    set_read_timestamp_ltz_format(stmt.conn.timestamp_ltz_format);
    set_read_timestamp_tz_format(stmt.conn.timestamp_tz_format);
    match stmt.state.as_ref() {
        StatementState::Fetching {
            reader: _,
            record_batch,
            batch_idx,
        } => {
            if *batch_idx == 0 {
                tracing::error!("get_data: data not fetched yet for requested column");
                return DataNotFetchedSnafu.fail();
            }
            let row_index = batch_idx - 1;
            let array_ref = record_batch.column((col_or_param_num - 1) as usize);
            let schema = record_batch.schema();
            let field = schema.field((col_or_param_num - 1) as usize);

            read_arrow_value(
                target_type,
                target_value_ptr,
                buffer_length,
                str_len_or_ind_ptr,
                array_ref,
                field,
                row_index,
            )
            .context(ArrowReadSnafu)?;

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
