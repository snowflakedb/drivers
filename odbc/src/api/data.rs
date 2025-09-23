use crate::api::error::{
    ArrowReadSnafu, DataNotFetchedSnafu, ExecutionDoneSnafu, FetchDataSnafu, NoMoreDataSnafu,
    StatementErrorStateSnafu, StatementNotExecutedSnafu,
};
use crate::api::{OdbcResult, StatementState, WithState, stmt_from_handle};
use crate::cdata_types::{CDataType, Double, Real, SBigInt, UBigInt};
use crate::read_arrow::{Buffer, ExtractError, ReadArrowValue, Value};
use arrow::{array::Array, datatypes::Field};
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
        _ => Err(ExtractError::UnsupportedTargetType(target_type)),
    }
}

/// Fetch the next row of data
pub fn fetch(statement_handle: sql::Handle) -> OdbcResult<()> {
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
    })
}

/// Get data from a specific column
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
    match stmt.state.as_ref() {
        StatementState::Fetching {
            reader: _,
            record_batch,
            batch_idx,
        } => {
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
                *batch_idx,
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
