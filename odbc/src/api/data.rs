use std::collections::HashMap;
use std::sync::Arc;

use crate::api::CDataType;
use crate::api::ConnectionState;
use crate::api::error::DisconnectedSnafu;
use crate::api::error::{
    ConversionSnafu, DataNotFetchedSnafu, FetchDataSnafu, FetchTypeOutOfRangeSnafu, InternalSnafu,
    InvalidBufferLengthSnafu, InvalidCursorPositionSnafu, InvalidCursorStateSnafu,
    InvalidDescriptorIndexSnafu, InvalidDuringDaeSnafu, MixedCursorFunctionsSnafu, NoMoreDataSnafu,
    NullPointerSnafu, OdbcError, StatementErrorStateSnafu, StatementNotExecutedSnafu,
    UnsupportedFeatureSnafu,
};
use crate::api::{
    GetDataState, OdbcResult, StatementInner, StatementState, WithState, stmt_from_handle,
};
use crate::conversion::warning::Warnings;
use crate::conversion::{
    Binding, BindingStrides, ColumnConverter, ConversionError, NumericSettings,
    make_fetch_converter,
};
use arrow::array::RecordBatchReader;
use arrow::datatypes::{Field, SchemaRef};
use odbc_sys as sql;
use snafu::ResultExt;
use tracing;

/// Builds the converter for `field` and returns `binding` with `SQL_C_DEFAULT`
/// resolved. Rejects a `SQL_C_DEFAULT` interval binding whose buffer is
/// shorter than the struct.
fn fetch_converter(
    binding: &Binding,
    field: &Field,
    numeric_settings: &NumericSettings,
) -> OdbcResult<(Box<dyn ColumnConverter>, Binding)> {
    let (converter, target_type) =
        make_fetch_converter(field, binding.target_type, numeric_settings)
            .context(ConversionSnafu)?;
    let struct_len = std::mem::size_of::<sql::IntervalStruct>() as sql::Len;
    if binding.target_type == CDataType::Default
        && target_type.is_interval()
        && (1..struct_len).contains(&binding.buffer_length)
    {
        return InvalidBufferLengthSnafu {
            length: binding.buffer_length as i64,
        }
        .fail();
    }
    Ok((
        converter,
        Binding {
            target_type,
            ..*binding
        },
    ))
}

/// Per-statement cache of converters and `Binding` snapshots for the bound
/// columns. Kept on the statement across `SQLFetch` calls and batches, and
/// rebuilt only when the result schema, the ARD bindings or the numeric
/// settings change.
#[derive(Default)]
pub(crate) struct FetchConverterCache {
    schema: Option<SchemaRef>,
    bindings: HashMap<u16, Binding>,
    numeric_settings: Option<NumericSettings>,
    entries: Vec<CachedColumn>,
}

struct CachedColumn {
    /// `usize::MAX` sentinels an out-of-range column (logged + skipped).
    arrow_col: usize,
    /// Snapshot of the ARD `Binding` so the inner loop never touches the
    /// `HashMap`. Safe because bindings cannot mutate during `SQLFetch`.
    binding: Binding,
    converter: Option<Box<dyn ColumnConverter>>,
}

impl FetchConverterCache {
    /// Rebuild the cache if the current `RecordBatch` schema, the active ARD
    /// bindings or the numeric settings differ from the cached ones.
    fn refresh_if_needed(
        &mut self,
        inner: &StatementInner,
        numeric_settings: &NumericSettings,
    ) -> OdbcResult<()> {
        let StatementState::Fetching { record_batch, .. } = inner.state.as_ref() else {
            return Ok(());
        };
        let schema = record_batch.schema_ref();

        let up_to_date = self
            .schema
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(cached, schema) || cached == schema)
            && self.numeric_settings.as_ref() == Some(numeric_settings)
            && inner.with_active_ard(|ard| ard.bindings == self.bindings);
        if up_to_date {
            return Ok(());
        }

        self.schema = None;
        self.entries.clear();
        // Resolve active ARD bindings (they may be from an explicit descriptor).
        self.bindings = inner.with_active_ard(|ard| ard.bindings.clone());
        self.entries.reserve(self.bindings.len());

        for (&column_number, binding) in &self.bindings {
            // ODBC reserves column 0 for bookmarks; `checked_sub` also guards
            // debug-mode panics if `SQLBindCol` accepted a 0.
            let arrow_col = (column_number as usize)
                .checked_sub(1)
                .filter(|&idx| idx < schema.fields().len());
            let Some(arrow_col) = arrow_col else {
                // Log once per cache rebuild, not once per fetched row.
                tracing::error!(
                    "fetch converter cache: column_number {} is out of range \
                     (schema has {} columns)",
                    column_number,
                    schema.fields().len(),
                );
                self.entries.push(CachedColumn {
                    arrow_col: usize::MAX,
                    binding: *binding,
                    converter: None,
                });
                continue;
            };
            let field = schema.field(arrow_col);
            let (converter, binding) = fetch_converter(binding, field, numeric_settings)?;
            self.entries.push(CachedColumn {
                arrow_col,
                binding,
                converter: Some(converter),
            });
        }

        self.schema = Some(Arc::clone(schema));
        self.numeric_settings = Some(*numeric_settings);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum FetchOrientation {
    Next,
    First,
    Last,
    Prior,
    Absolute,
    Relative,
}

impl TryFrom<sql::SmallInt> for FetchOrientation {
    type Error = OdbcError;

    fn try_from(value: sql::SmallInt) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(FetchOrientation::Next),
            2 => Ok(FetchOrientation::First),
            3 => Ok(FetchOrientation::Last),
            4 => Ok(FetchOrientation::Prior),
            5 => Ok(FetchOrientation::Absolute),
            6 => Ok(FetchOrientation::Relative),
            _ => FetchTypeOutOfRangeSnafu.fail(),
        }
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy)]
enum RowStatus {
    Success = 0,
    NoRow = 3,
    Error = 5,
    SuccessWithInfo = 6,
}

/// Read batches from the Arrow reader until a non-empty one is found.
/// Empty batches (0 rows) are skipped. Returns `NoMoreData` when the
/// reader is exhausted.
#[allow(clippy::result_large_err)]
fn next_non_empty_batch(
    mut reader: crate::api::ResultReader,
    rows_affected: Option<i64>,
    origin: crate::api::ExecutionOrigin,
) -> Result<(StatementState, ()), (StatementState, crate::api::error::OdbcError)> {
    loop {
        match reader.next() {
            Some(rb) => {
                let record_batch = rb
                    .context(FetchDataSnafu)
                    .with_state(StatementState::Error)?;
                if record_batch.num_rows() == 0 {
                    continue;
                }
                break Ok((
                    StatementState::Fetching {
                        reader,
                        record_batch,
                        batch_idx: 0,
                        rows_affected,
                        origin,
                    },
                    (),
                ));
            }
            None => {
                let schema = reader.schema();
                break NoMoreDataSnafu
                    .fail()
                    .with_state(StatementState::Done { schema, origin });
            }
        }
    }
}

/// Advance the cursor by one row. Handles state transitions from
/// `QueryExecuted` → `Fetching` and from one batch to the next.
#[allow(clippy::result_large_err)]
fn advance_cursor(state: &mut crate::api::State<StatementState>) -> OdbcResult<()> {
    state.transition_or_err(|s| match s {
        StatementState::DdlExecuted { schema, origin } => InvalidCursorStateSnafu
            .fail()
            .with_state(StatementState::DdlExecuted { schema, origin }),
        StatementState::DmlExecuted {
            rows_affected,
            schema,
            origin,
        } => InvalidCursorStateSnafu
            .fail()
            .with_state(StatementState::DmlExecuted {
                rows_affected,
                schema,
                origin,
            }),
        StatementState::QueryExecuted {
            reader,
            rows_affected,
            origin,
        } => next_non_empty_batch(reader, rows_affected, origin),
        StatementState::Fetching {
            reader,
            record_batch,
            batch_idx,
            rows_affected,
            origin,
        } => {
            let new_idx = batch_idx + 1;
            if new_idx < record_batch.num_rows() {
                Ok((
                    StatementState::Fetching {
                        reader,
                        record_batch,
                        batch_idx: new_idx,
                        rows_affected,
                        origin,
                    },
                    (),
                ))
            } else {
                next_non_empty_batch(reader, rows_affected, origin)
            }
        }
        state @ StatementState::Error => StatementErrorStateSnafu.fail().with_state(state),
        state @ StatementState::Done { .. } => NoMoreDataSnafu.fail().with_state(state),
        state @ StatementState::Created
        | state @ StatementState::Prepared { .. }
        // NeedData variants are unreachable (callers guard with is_need_data())
        // but required for exhaustive match.
        | state @ StatementState::AwaitingParamData { .. }
        | state @ StatementState::AwaitingPutData { .. }
        | state @ StatementState::PutDataCalled { .. }
        | state @ StatementState::AsyncExecDirect { .. }
        | state @ StatementState::AsyncPrepare { .. }
        | state @ StatementState::AsyncExecute { .. } => {
            StatementNotExecutedSnafu.fail().with_state(state)
        }
    })
}

/// Fetch the next rowset of data (block cursor aware).
pub fn fetch(statement_handle: sql::Handle, warnings: &mut Warnings) -> OdbcResult<()> {
    tracing::debug!("fetch called");
    let guard = stmt_from_handle(statement_handle)?;
    {
        let inner = guard.inner.lock();
        if inner.state.as_ref().is_need_data() {
            return InvalidDuringDaeSnafu.fail();
        }
        if inner.used_extended_fetch {
            return MixedCursorFunctionsSnafu.fail();
        }
    }
    let dbc = guard.conn()?;
    let conn = dbc.connection.lock();
    if matches!(conn.state, ConnectionState::Disconnected { .. }) {
        tracing::error!("fetch: connection is disconnected");
        return DisconnectedSnafu.fail();
    }
    let numeric_settings = conn.numeric_settings;
    fetch_impl(&guard, &numeric_settings, warnings, None)
}

fn fetch_impl(
    guard: &crate::api::handle_registry::HandleGuard<crate::api::Statement>,
    numeric_settings: &NumericSettings,
    warnings: &mut Warnings,
    // `SQLExtendedFetch` passes `Some(SQL_ROWSET_SIZE)`; `SQLFetch`/`SQLFetchScroll`
    // pass `None` and use the ARD `SQL_DESC_ARRAY_SIZE` (`SQL_ATTR_ROW_ARRAY_SIZE`).
    // The two rowset counts are kept separate per the ODBC spec — extended fetch
    // never reads or writes the ARD array size.
    rowset_override: Option<sql::ULen>,
) -> OdbcResult<()> {
    let mut inner = guard.inner.lock();
    let mut cache = std::mem::take(&mut inner.fetch_cache);
    let result = fetch_rows(
        &mut inner,
        &mut cache,
        numeric_settings,
        warnings,
        rowset_override,
    );
    inner.fetch_cache = cache;
    result
}

fn fetch_rows(
    inner: &mut StatementInner,
    cache: &mut FetchConverterCache,
    numeric_settings: &NumericSettings,
    warnings: &mut Warnings,
    rowset_override: Option<sql::ULen>,
) -> OdbcResult<()> {
    inner.get_data_state = None;

    let (ard_array_size, bind_type, bind_offset_ptr) =
        inner.with_active_ard(|ard| (ard.array_size, ard.bind_type, ard.bind_offset_ptr));
    let array_size = rowset_override.unwrap_or(ard_array_size).max(1);
    let row_status_ptr = inner.ird.array_status_ptr;
    let rows_fetched_ptr = inner.ird.rows_processed_ptr;
    let bind_offset = if !bind_offset_ptr.is_null() {
        unsafe { *bind_offset_ptr }
    } else {
        0
    };
    let max_rows = inner.max_rows;

    // Enforce SQL_ATTR_MAX_ROWS: if the limit has already been reached, stop.
    if max_rows > 0 && inner.rows_returned >= max_rows {
        if !rows_fetched_ptr.is_null() {
            unsafe { *rows_fetched_ptr = 0 };
        }
        for row_index in 0..array_size {
            write_row_status(row_status_ptr, row_index, RowStatus::NoRow);
        }
        return NoMoreDataSnafu.fail();
    }

    // Fast path: a single-row fetch propagates the conversion error to
    // the caller instead of just marking the row as Error.
    if array_size == 1 && bind_offset_ptr.is_null() {
        advance_cursor(&mut inner.state)?;
        cache.refresh_if_needed(inner, numeric_settings)?;
        if !rows_fetched_ptr.is_null() {
            unsafe { *rows_fetched_ptr = 1 };
        }
        let arrow_start = match inner.state.as_ref() {
            StatementState::Fetching { batch_idx, .. } => *batch_idx,
            _ => {
                return InternalSnafu {
                    message: "advance_cursor returned Ok but state is not Fetching in fast path"
                        .to_string(),
                }
                .fail();
            }
        };
        let outputs = execute_bindings_for_segment(inner, cache, arrow_start, 1, 0, 0, 0);
        match outputs.into_iter().next().unwrap_or_else(|| Ok(Vec::new())) {
            Ok(row_warnings) => {
                inner.rows_returned += 1;
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
                return Err(e).context(ConversionSnafu);
            }
        }
        return Ok(());
    }

    let mut rows_fetched: usize = 0;
    let mut has_error = false;

    while rows_fetched < array_size {
        // Stop if max_rows limit reached mid-rowset.
        if max_rows > 0 && inner.rows_returned + rows_fetched as sql::ULen >= max_rows {
            for remaining in rows_fetched..array_size {
                write_row_status(row_status_ptr, remaining, RowStatus::NoRow);
            }
            break;
        }
        match advance_cursor(&mut inner.state) {
            Ok(()) => {}
            Err(crate::api::OdbcError::NoMoreData { .. }) => {
                for remaining in rows_fetched..array_size {
                    write_row_status(row_status_ptr, remaining, RowStatus::NoRow);
                }
                break;
            }
            Err(e) => {
                if rows_fetched == 0 {
                    return Err(e);
                }
                write_row_status(row_status_ptr, rows_fetched, RowStatus::Error);
                has_error = true;
                // Errored rows count toward `SQL_ATTR_ROWS_FETCHED_PTR`
                // per the ODBC spec, so the row-status array and the
                // reported count stay consistent.
                rows_fetched += 1;
                for remaining in rows_fetched..array_size {
                    write_row_status(row_status_ptr, remaining, RowStatus::NoRow);
                }
                break;
            }
        }

        if let Err(e) = cache.refresh_if_needed(inner, numeric_settings) {
            if rows_fetched == 0 {
                return Err(e);
            }
            write_row_status(row_status_ptr, rows_fetched, RowStatus::Error);
            has_error = true;
            rows_fetched += 1;
            for remaining in rows_fetched..array_size {
                write_row_status(row_status_ptr, remaining, RowStatus::NoRow);
            }
            break;
        }

        let (arrow_start, segment_len) = {
            let StatementState::Fetching {
                record_batch,
                batch_idx,
                ..
            } = inner.state.as_ref()
            else {
                return InternalSnafu {
                    message: "advance_cursor returned Ok but state is not Fetching".to_string(),
                }
                .fail();
            };
            let num_rows = record_batch.num_rows();
            let in_batch_remaining = num_rows.saturating_sub(*batch_idx);
            if in_batch_remaining == 0 {
                // Invariant: after a successful `advance_cursor`,
                // `batch_idx` must point at a valid row.
                // `next_non_empty_batch` guarantees `num_rows >= 1` and
                // `batch_idx == 0` for fresh batches; the in-batch branch
                // only stays in `Fetching` when `new_idx < num_rows`. If we
                // ever observe `batch_idx >= num_rows` here it's a driver
                // bug — fail loudly with `InternalError` rather than
                // silently producing an out-of-bounds Arrow index.
                return InternalSnafu {
                    message: format!(
                        "advance_cursor left batch_idx ({}) >= num_rows ({}) in fetch_impl",
                        *batch_idx, num_rows,
                    ),
                }
                .fail();
            }
            let mut wanted = array_size - rows_fetched;
            if max_rows > 0 {
                let remaining_quota = max_rows
                    .saturating_sub(inner.rows_returned)
                    .saturating_sub(rows_fetched as sql::ULen);
                if (wanted as sql::ULen) > remaining_quota {
                    wanted = remaining_quota;
                }
            }
            (*batch_idx, in_batch_remaining.min(wanted).max(1))
        };

        let outputs = execute_bindings_for_segment(
            inner,
            cache,
            arrow_start,
            segment_len,
            rows_fetched,
            bind_type,
            bind_offset,
        );

        for (i, outcome) in outputs.into_iter().enumerate() {
            let out_row = rows_fetched + i;
            match outcome {
                Ok(w) => {
                    let status = if w.is_empty() {
                        RowStatus::Success
                    } else {
                        RowStatus::SuccessWithInfo
                    };
                    write_row_status(row_status_ptr, out_row, status);
                    warnings.extend(w);
                }
                Err(_) => {
                    write_row_status(row_status_ptr, out_row, RowStatus::Error);
                    has_error = true;
                }
            }
        }
        rows_fetched += segment_len;

        // `advance_cursor` already landed on the first row of the segment;
        // skip the rest so the next call lands on the row after it.
        if segment_len > 1 {
            bump_batch_idx(&mut inner.state, segment_len - 1)?;
        }
    }

    inner.rows_returned += rows_fetched as sql::ULen;

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

/// Advance `batch_idx` by `delta` rows within the current `RecordBatch`.
/// Returns `OdbcError::InternalError` if advancing would cross the batch
/// boundary or the statement is not in the `Fetching` state.
#[allow(clippy::result_large_err)]
fn bump_batch_idx(state: &mut crate::api::State<StatementState>, delta: usize) -> OdbcResult<()> {
    if delta == 0 {
        return Ok(());
    }
    state.transition_or_err(|s| match s {
        StatementState::Fetching {
            reader,
            record_batch,
            batch_idx,
            rows_affected,
            origin,
        } => {
            let num_rows = record_batch.num_rows();
            if batch_idx + delta >= num_rows {
                let err = InternalSnafu {
                    message: format!(
                        "bump_batch_idx crossed batch boundary (delta={delta}, \
                         batch_idx={batch_idx}, num_rows={num_rows})"
                    ),
                }
                .build();
                let restored = StatementState::Fetching {
                    reader,
                    record_batch,
                    batch_idx,
                    rows_affected,
                    origin,
                };
                return Err((restored, err));
            }
            Ok((
                StatementState::Fetching {
                    reader,
                    record_batch,
                    batch_idx: batch_idx + delta,
                    rows_affected,
                    origin,
                },
                (),
            ))
        }
        other => {
            let err = InternalSnafu {
                message: "bump_batch_idx called outside Fetching state".to_string(),
            }
            .build();
            Err((other, err))
        }
    })
}

/// `SQLFetchScroll` — currently only `SQL_FETCH_NEXT` is supported.
pub fn fetch_scroll(
    statement_handle: sql::Handle,
    fetch_orientation: sql::SmallInt,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    let guard = stmt_from_handle(statement_handle)?;
    {
        let inner = guard.inner.lock();
        if inner.state.as_ref().is_need_data() {
            return InvalidDuringDaeSnafu.fail();
        }
    }

    let orientation = FetchOrientation::try_from(fetch_orientation)?;
    match orientation {
        FetchOrientation::Next => fetch(statement_handle, warnings),
        other => {
            tracing::warn!("fetch_scroll: unsupported orientation {other:?}");
            FetchTypeOutOfRangeSnafu.fail()
        }
    }
}

/// `SQLExtendedFetch` — ODBC 2.x block-fetch function.
///
/// Sets the `used_extended_fetch` flag so that subsequent `SQLFetch` calls
/// are rejected (per ODBC spec) until the cursor is closed.
pub fn extended_fetch(
    statement_handle: sql::Handle,
    fetch_orientation: sql::SmallInt,
    _fetch_offset: sql::Len,
    row_count_ptr: *mut sql::ULen,
    row_status_ptr: *mut sql::USmallInt,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    tracing::debug!("extended_fetch called");

    let guard = stmt_from_handle(statement_handle)?;
    {
        let inner = guard.inner.lock();
        if inner.state.as_ref().is_need_data() {
            return InvalidDuringDaeSnafu.fail();
        }
    }
    let dbc = guard.conn()?;
    let conn = dbc.connection.lock();
    if matches!(conn.state, ConnectionState::Disconnected { .. }) {
        tracing::error!("extended_fetch: connection is disconnected");
        return DisconnectedSnafu.fail();
    }
    let numeric_settings = conn.numeric_settings;

    let orientation = FetchOrientation::try_from(fetch_orientation)?;
    match orientation {
        FetchOrientation::Next => {}
        other => {
            tracing::warn!("extended_fetch: unsupported orientation {other:?}");
            return UnsupportedFeatureSnafu.fail();
        }
    }

    // SQL_ROWSET_SIZE (9) drives the SQLExtendedFetch rowset. It is passed to
    // fetch_impl directly rather than written into the ARD SQL_DESC_ARRAY_SIZE,
    // so SQL_ATTR_ROW_ARRAY_SIZE (27) — which drives SQLFetch/SQLFetchScroll —
    // is never clobbered and survives SQLFreeStmt(SQL_CLOSE). Read it in the
    // same lock scope that stamps the IRD pointers to avoid re-locking.
    let rowset_size = {
        let mut inner = guard.inner.lock();
        inner.used_extended_fetch = true;
        inner.ird.rows_processed_ptr = row_count_ptr;
        inner.ird.array_status_ptr = row_status_ptr;
        inner.rowset_size
    };
    fetch_impl(&guard, &numeric_settings, warnings, Some(rowset_size))
}

fn write_row_status(row_status_ptr: *mut u16, row_idx: usize, status: RowStatus) {
    if !row_status_ptr.is_null() {
        unsafe { *row_status_ptr.add(row_idx) = status as u16 };
    }
}

/// Column-major hot path for `SQLFetch`/`SQLExtendedFetch`. Iterates over
/// the cached columns and dispatches each one to
/// [`ColumnConverter::convert_arrow_range`] once for the whole segment, so the
/// Arrow downcast and vtable call are paid per `(column, segment)` rather
/// than per cell. Returns one `Result` per row, preserving the row-major
/// "first error aborts the row" semantics.
fn execute_bindings_for_segment(
    inner: &StatementInner,
    cache: &FetchConverterCache,
    arrow_start: usize,
    segment_len: usize,
    out_row_start: usize,
    bind_type: usize,
    bind_offset: isize,
) -> Vec<Result<Warnings, ConversionError>> {
    let mut outputs: Vec<Result<Warnings, ConversionError>> =
        (0..segment_len).map(|_| Ok(Vec::new())).collect();

    let StatementState::Fetching { record_batch, .. } = inner.state.as_ref() else {
        return outputs;
    };

    let arrow_row_range = arrow_start..(arrow_start + segment_len);
    let strides = BindingStrides {
        bind_type,
        bind_offset,
    };

    for cached in &cache.entries {
        // Out-of-range columns were reported once in `refresh_if_needed`.
        let Some(converter) = cached.converter.as_deref() else {
            continue;
        };
        let array_ref = record_batch.column(cached.arrow_col);
        converter.convert_arrow_range(
            array_ref.as_ref(),
            arrow_row_range.clone(),
            &cached.binding,
            out_row_start,
            strides,
            &mut outputs,
        );
    }

    outputs
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

    let guard = stmt_from_handle(statement_handle)?;
    let mut inner = guard.inner.lock();

    if inner.state.as_ref().is_need_data() {
        return InvalidDuringDaeSnafu.fail();
    }

    if target_value_ptr.is_null() {
        return NullPointerSnafu.fail();
    }

    if buffer_length < 0 {
        return InvalidBufferLengthSnafu {
            length: buffer_length as i64,
        }
        .fail();
    }

    let ard_array_size = inner.with_active_ard(|ard| ard.array_size);
    if ard_array_size > 1 {
        tracing::warn!("get_data: cannot use SQLGetData with row_array_size > 1");
        return InvalidCursorPositionSnafu.fail();
    }

    // Column 0 is reserved for bookmarks; reject it when bookmarks are off
    if col_or_param_num == 0 {
        return InvalidDescriptorIndexSnafu { number: 0i16 }.fail();
    }

    // SQL_ARD_TYPE: resolve from the ARD descriptor's concise type for the column
    let target_type = if target_type == CDataType::Ard {
        let resolved =
            inner.with_active_ard(|ard| ard.bindings.get(&col_or_param_num).map(|b| b.target_type));
        match resolved {
            Some(t) => t,
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
    if let Some(ref state) = inner.get_data_state
        && state.col() == col_or_param_num
    {
        match state {
            GetDataState::Completed { .. } => {
                inner.get_data_state = None;
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
    inner.get_data_state = None;

    match inner.state.as_ref() {
        StatementState::Fetching {
            record_batch,
            batch_idx,
            ..
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

            let ard_binding = inner.with_active_ard(|ard| {
                ard.bindings.get(&col_or_param_num).copied()
            });
            let settings = guard.conn()?.connection.lock().numeric_settings;
            let (converter, binding) = fetch_converter(
                &Binding {
                    target_type,
                    target_value_ptr,
                    buffer_length,
                    octet_length_ptr: str_len_or_ind_ptr,
                    indicator_ptr: str_len_or_ind_ptr,
                    precision: ard_binding.and_then(|b| b.precision),
                    scale: ard_binding.and_then(|b| b.scale),
                    datetime_interval_precision: ard_binding
                        .and_then(|b| b.datetime_interval_precision),
                    length: 0,
                },
                field,
                &settings,
            )?;
            let conversion_warnings = converter
                .convert_arrow_value(array_ref, *batch_idx, &binding, &mut offset)
                .context(ConversionSnafu)?;
            warnings.extend(conversion_warnings);

            // The write method sets offset to Some(n) on truncation, None when complete.
            match offset {
                Some(new_offset) => {
                    inner.get_data_state = Some(GetDataState::Partial {
                        col: col_or_param_num,
                        offset: new_offset,
                    });
                }
                None => {
                    inner.get_data_state = Some(GetDataState::Completed {
                        col: col_or_param_num,
                    });
                }
            }

            Ok(())
        }
        StatementState::DdlExecuted { .. } | StatementState::DmlExecuted { .. } => {
            tracing::error!("get_data: no result set associated with the statement");
            NoMoreDataSnafu.fail()
        }
        StatementState::Done { .. } => {
            tracing::debug!("get_data: statement execution is done");
            NoMoreDataSnafu.fail()
        }
        StatementState::Created
        | StatementState::Prepared { .. }
        | StatementState::QueryExecuted { .. }
        // NeedData variants are unreachable (guarded by is_need_data() above)
        // but required for exhaustive match.
        | StatementState::AwaitingParamData { .. }
        | StatementState::AwaitingPutData { .. }
        | StatementState::PutDataCalled { .. }
        | StatementState::AsyncExecDirect { .. }
        | StatementState::AsyncPrepare { .. }
        | StatementState::AsyncExecute { .. } => {
            tracing::error!("get_data: data not fetched yet");
            DataNotFetchedSnafu.fail()
        }
        StatementState::Error => {
            tracing::error!("get_data: statement error");
            StatementErrorStateSnafu.fail()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Double, Real, SBigInt, UBigInt};
    use crate::conversion::resolve_fetch_c_type;
    use arrow::array::Array;
    use arrow::array::{
        Decimal128Array, Float64Array, Int8Array, Int16Array, Int32Array, Int64Array, StringArray,
    };
    use arrow::datatypes::{DataType, Field};
    use std::collections::HashMap;

    // Resolves SQL_C_DEFAULT the way SQLGetData does.
    fn read_arrow_value_test(
        binding: &Binding,
        array_ref: &dyn Array,
        field: &Field,
        batch_idx: usize,
    ) -> Result<Warnings, ConversionError> {
        let (converter, target_type) =
            make_fetch_converter(field, binding.target_type, &NumericSettings::default())?;
        let binding = Binding {
            target_type,
            ..*binding
        };
        converter.convert_arrow_value(array_ref, batch_idx, &binding, &mut None)
    }

    fn resolved_sql_c_default(field: &Field) -> CDataType {
        resolve_fetch_c_type(CDataType::Default, field, &NumericSettings::default()).unwrap()
    }

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

    fn field_with_real_meta() -> Field {
        let mut metadata = HashMap::new();
        metadata.insert("logicalType".to_string(), "REAL".to_string());
        Field::new("test", DataType::Float64, false).with_metadata(metadata)
    }

    impl Binding {
        fn from_buffer(target_type: CDataType, buffer: &mut [u8], str_len: *mut sql::Len) -> Self {
            Self {
                target_type,
                target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
                buffer_length: buffer.len() as sql::Len,
                octet_length_ptr: str_len,
                ..Default::default()
            }
        }

        fn from_value<T>(target_type: CDataType, value: &mut T) -> Self {
            Self {
                target_type,
                target_value_ptr: value as *mut T as sql::Pointer,
                ..Default::default()
            }
        }
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

            let binding = Binding::from_buffer(CDataType::Char, &mut buffer, &mut str_len);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(str_len, 5);
            assert_eq!(&buffer[..5], b"hello");
            assert_eq!(buffer[5], 0); // Null terminator
        }

        #[test]
        fn reads_int64_with_scale_to_char_buffer() {
            let array = Int64Array::from(vec![12345i64]); // 123.45 with scale 2
            let field = field_with_fixed_meta(DataType::Int64, 2, 10);
            let mut buffer = vec![0u8; 32];
            let mut str_len: sql::Len = 0;

            let binding = Binding::from_buffer(CDataType::Char, &mut buffer, &mut str_len);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(str_len, 6);
            assert_eq!(&buffer[..6], b"123.45");
            assert_eq!(buffer[6], 0); // Null terminator
        }

        #[test]
        fn truncates_when_buffer_too_small() {
            let array = StringArray::from(vec!["hello world"]);
            let field = field_with_text_meta();
            let mut buffer = vec![0u8; 5];
            let mut str_len: sql::Len = 0;

            let binding = Binding::from_buffer(CDataType::Char, &mut buffer, &mut str_len);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(str_len, 11); // total character count of "hello world"
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

            let binding = Binding::from_value(CDataType::UBigInt, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(value, 9876543210u64);
        }

        #[test]
        fn reads_int64_with_scale_drops_decimals() {
            let array = Int64Array::from(vec![12345i64]); // 123.45 with scale 2
            let field = field_with_fixed_meta(DataType::Int64, 2, 10);
            let mut value: UBigInt = 0;

            let binding = Binding::from_value(CDataType::UBigInt, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

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

            let binding = Binding::from_value(CDataType::UBigInt, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

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

            let binding = Binding::from_value(CDataType::SBigInt, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(value, -9876543210i64);
        }

        #[test]
        fn reads_int64_with_scale_drops_decimals() {
            let array = Int64Array::from(vec![-12345i64]); // -123.45 with scale 2
            let field = field_with_fixed_meta(DataType::Int64, 2, 10);
            let mut value: SBigInt = 0;

            let binding = Binding::from_value(CDataType::SBigInt, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

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

            let binding = Binding::from_value(CDataType::Long, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(value, 123456);
        }

        #[test]
        fn reads_negative_int32_as_slong() {
            let array = Int32Array::from(vec![-123456i32]);
            let field = field_with_fixed_meta(DataType::Int32, 0, 10);
            let mut value: sql::Integer = 0;

            let binding = Binding::from_value(CDataType::SLong, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

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

            let binding = Binding::from_value(CDataType::ULong, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

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

            let binding = Binding::from_value(CDataType::Short, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(value, 1234);
        }

        #[test]
        fn reads_negative_int16_as_sshort() {
            let array = Int16Array::from(vec![-1234i16]);
            let field = field_with_fixed_meta(DataType::Int16, 0, 5);
            let mut value: sql::SmallInt = 0;

            let binding = Binding::from_value(CDataType::SShort, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

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

            let binding = Binding::from_value(CDataType::UShort, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

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

            let binding = Binding::from_value(CDataType::TinyInt, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(value, 42);
        }

        #[test]
        fn reads_negative_int8_as_stinyint() {
            let array = Int8Array::from(vec![-42i8]);
            let field = field_with_fixed_meta(DataType::Int8, 0, 3);
            let mut value: sql::SChar = 0;

            let binding = Binding::from_value(CDataType::STinyInt, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

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

            let binding = Binding::from_value(CDataType::UTinyInt, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

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

            let binding = Binding::from_value(CDataType::Float, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

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

            let binding = Binding::from_value(CDataType::Double, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

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

            let binding = Binding::from_value(CDataType::Double, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

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

            let binding = Binding::from_value(CDataType::TypeDate, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(matches!(
                result,
                Err(ConversionError::WriteOdbcValue { .. })
            ));
        }

        #[test]
        fn successfully_reads_wchar_target_type() {
            use crate::api::encoding::{WIDE_CHAR_SIZE, WideChar};
            let array = StringArray::from(vec!["hello"]);
            let field = field_with_text_meta();
            let mut buffer = vec![0 as WideChar; 32];
            let mut str_len: sql::Len = 0;

            let binding = Binding {
                target_type: CDataType::WChar,
                target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
                buffer_length: (buffer.len() * WIDE_CHAR_SIZE) as sql::Len,
                octet_length_ptr: &mut str_len,
                ..Default::default()
            };
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            // "hello" is 5 ASCII code points → 5 wide-char code units → bytes.
            assert_eq!(str_len, (5 * WIDE_CHAR_SIZE) as sql::Len);
            assert_eq!(
                &buffer[..5],
                &[
                    b'h' as WideChar,
                    b'e' as WideChar,
                    b'l' as WideChar,
                    b'l' as WideChar,
                    b'o' as WideChar,
                ]
            );
            assert_eq!(buffer[5], 0);
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

                let binding = Binding::from_value(CDataType::UBigInt, &mut value);
                let result = read_arrow_value_test(&binding, &array, &field, idx);

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

                let binding = Binding::from_buffer(CDataType::Char, &mut buffer, &mut str_len);
                let result = read_arrow_value_test(&binding, &array, &field, idx);

                assert!(result.is_ok());
                assert_eq!(str_len, expected.len() as sql::Len);
                assert_eq!(&buffer[..expected.len()], expected.as_bytes());
            }
        }
    }

    // Tests for CDataType::Default (SQL_C_DEFAULT)
    // Per ODBC spec, SQL_DECIMAL/SQL_NUMERIC default C type is SQL_C_CHAR.
    mod read_to_default {
        use super::*;

        #[test]
        fn default_reads_int64_as_char_for_fixed_type() {
            let array = Int64Array::from(vec![123i64]);
            let field = field_with_fixed_meta(DataType::Int64, 0, 10);
            let mut buffer = vec![0u8; 32];
            let mut str_len: sql::Len = 0;

            let binding =
                Binding::from_buffer(resolved_sql_c_default(&field), &mut buffer, &mut str_len);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(str_len, 3);
            assert_eq!(&buffer[..3], b"123");
        }

        #[test]
        fn default_reads_int64_with_scale_as_char() {
            let array = Int64Array::from(vec![12345i64]); // 123.45 with scale 2
            let field = field_with_fixed_meta(DataType::Int64, 2, 10);
            let mut buffer = vec![0u8; 32];
            let mut str_len: sql::Len = 0;

            let binding =
                Binding::from_buffer(resolved_sql_c_default(&field), &mut buffer, &mut str_len);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(str_len, 6);
            assert_eq!(&buffer[..6], b"123.45");
        }

        #[test]
        fn default_reads_negative_int64_with_scale_as_char() {
            let array = Int64Array::from(vec![-12345i64]); // -123.45 with scale 2
            let field = field_with_fixed_meta(DataType::Int64, 2, 10);
            let mut buffer = vec![0u8; 32];
            let mut str_len: sql::Len = 0;

            let binding =
                Binding::from_buffer(resolved_sql_c_default(&field), &mut buffer, &mut str_len);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(str_len, 7);
            assert_eq!(&buffer[..7], b"-123.45");
        }

        #[test]
        fn default_reads_decimal128_as_char() {
            let array = Decimal128Array::from(vec![12345i128])
                .with_precision_and_scale(10, 2)
                .unwrap();
            let field = decimal128_field(10, 2);
            let mut buffer = vec![0u8; 32];
            let mut str_len: sql::Len = 0;

            let binding =
                Binding::from_buffer(resolved_sql_c_default(&field), &mut buffer, &mut str_len);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(str_len, 6);
            assert_eq!(&buffer[..6], b"123.45");
        }

        #[test]
        fn default_reads_small_value_with_large_scale() {
            let array = Int64Array::from(vec![5i64]); // 0.05 with scale 2
            let field = field_with_fixed_meta(DataType::Int64, 2, 10);
            let mut buffer = vec![0u8; 32];
            let mut str_len: sql::Len = 0;

            let binding =
                Binding::from_buffer(resolved_sql_c_default(&field), &mut buffer, &mut str_len);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(str_len, 4);
            assert_eq!(&buffer[..4], b"0.05");
        }

        #[test]
        fn default_reads_zero_as_char() {
            let array = Int64Array::from(vec![0i64]);
            let field = field_with_fixed_meta(DataType::Int64, 0, 10);
            let mut buffer = vec![0u8; 32];
            let mut str_len: sql::Len = 0;

            let binding =
                Binding::from_buffer(resolved_sql_c_default(&field), &mut buffer, &mut str_len);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(str_len, 1);
            assert_eq!(&buffer[..1], b"0");
        }

        #[test]
        fn default_reads_zero_with_scale_as_char() {
            let array = Int64Array::from(vec![0i64]);
            let field = field_with_fixed_meta(DataType::Int64, 3, 10);
            let mut buffer = vec![0u8; 32];
            let mut str_len: sql::Len = 0;

            let binding =
                Binding::from_buffer(resolved_sql_c_default(&field), &mut buffer, &mut str_len);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(str_len, 5);
            assert_eq!(&buffer[..5], b"0.000");
        }

        #[test]
        fn default_reads_large_decimal128_as_char() {
            let large_value: i128 = 99999999999999999999999999999999999999;
            let array = Decimal128Array::from(vec![large_value])
                .with_precision_and_scale(38, 0)
                .unwrap();
            let field = decimal128_field(38, 0);
            let mut buffer = vec![0u8; 64];
            let mut str_len: sql::Len = 0;

            let binding =
                Binding::from_buffer(resolved_sql_c_default(&field), &mut buffer, &mut str_len);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            let expected = b"99999999999999999999999999999999999999";
            assert_eq!(str_len, expected.len() as sql::Len);
            assert_eq!(&buffer[..expected.len()], expected);
        }

        #[test]
        fn default_reads_large_negative_decimal128_as_char() {
            let large_value: i128 = -99999999999999999999999999999999999999;
            let array = Decimal128Array::from(vec![large_value])
                .with_precision_and_scale(38, 0)
                .unwrap();
            let field = decimal128_field(38, 0);
            let mut buffer = vec![0u8; 64];
            let mut str_len: sql::Len = 0;

            let binding =
                Binding::from_buffer(resolved_sql_c_default(&field), &mut buffer, &mut str_len);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            let expected = b"-99999999999999999999999999999999999999";
            assert_eq!(str_len, expected.len() as sql::Len);
            assert_eq!(&buffer[..expected.len()], expected);
        }

        #[test]
        fn default_reads_decimal128_with_high_scale_as_char() {
            let value: i128 = 12345678901234567890123456789012345678;
            let array = Decimal128Array::from(vec![value])
                .with_precision_and_scale(38, 37)
                .unwrap();
            let field = decimal128_field(38, 37);
            let mut buffer = vec![0u8; 64];
            let mut str_len: sql::Len = 0;

            let binding =
                Binding::from_buffer(resolved_sql_c_default(&field), &mut buffer, &mut str_len);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            let expected = b"1.2345678901234567890123456789012345678";
            assert_eq!(str_len, expected.len() as sql::Len);
            assert_eq!(&buffer[..expected.len()], expected);
        }
    }

    mod read_real_to_default {
        use super::*;

        #[test]
        fn default_reads_float64_as_double() {
            let array = Float64Array::from(vec![3.125]);
            let field = field_with_real_meta();
            let mut value: Double = 0.0;
            let mut str_len: sql::Len = 0;

            let binding = Binding {
                octet_length_ptr: &mut str_len,
                ..Binding::from_value(resolved_sql_c_default(&field), &mut value)
            };
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert!((value - 3.125).abs() < f64::EPSILON);
        }

        #[test]
        fn default_reads_negative_float64() {
            let array = Float64Array::from(vec![-99.5]);
            let field = field_with_real_meta();
            let mut value: Double = 0.0;
            let mut str_len: sql::Len = 0;

            let binding = Binding {
                octet_length_ptr: &mut str_len,
                ..Binding::from_value(resolved_sql_c_default(&field), &mut value)
            };
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert!((value - (-99.5)).abs() < f64::EPSILON);
        }

        #[test]
        fn default_reads_zero_float64() {
            let array = Float64Array::from(vec![0.0]);
            let field = field_with_real_meta();
            let mut value: Double = 1.0;
            let mut str_len: sql::Len = 0;

            let binding = Binding {
                octet_length_ptr: &mut str_len,
                ..Binding::from_value(resolved_sql_c_default(&field), &mut value)
            };
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert!((value - 0.0).abs() < f64::EPSILON);
        }
    }

    mod read_real_explicit {
        use super::*;

        #[test]
        fn reads_float64_as_float() {
            let array = Float64Array::from(vec![3.125]);
            let field = field_with_real_meta();
            let mut value: Real = 0.0;

            let binding = Binding::from_value(CDataType::Float, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert!((value - 3.125f32).abs() < f32::EPSILON);
        }

        #[test]
        fn reads_float64_as_slong() {
            let array = Float64Array::from(vec![42.7]);
            let field = field_with_real_meta();
            let mut value: sql::Integer = 0;

            let binding = Binding::from_value(CDataType::SLong, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(value, 42);
        }

        #[test]
        fn reads_float64_as_sbigint() {
            let array = Float64Array::from(vec![123456789.9]);
            let field = field_with_real_meta();
            let mut value: SBigInt = 0;

            let binding = Binding::from_value(CDataType::SBigInt, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(value, 123456789i64);
        }

        #[test]
        fn reads_float64_as_char() {
            let array = Float64Array::from(vec![3.125]);
            let field = field_with_real_meta();
            let mut buffer = vec![0u8; 32];
            let mut str_len: sql::Len = 0;

            let binding = Binding::from_buffer(CDataType::Char, &mut buffer, &mut str_len);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            let expected = b"3.125";
            assert_eq!(str_len, expected.len() as sql::Len);
            assert_eq!(&buffer[..expected.len()], expected);
            assert_eq!(buffer[expected.len()], 0);
        }

        #[test]
        fn reads_float64_as_bit() {
            let array = Float64Array::from(vec![1.0]);
            let field = field_with_real_meta();
            let mut value: u8 = 0;

            let binding = Binding::from_value(CDataType::Bit, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(value, 1);
        }

        #[test]
        fn reads_float64_as_bit_out_of_range() {
            let array = Float64Array::from(vec![5.5]);
            let field = field_with_real_meta();
            let mut value: u8 = 0;

            let binding = Binding::from_value(CDataType::Bit, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_err());
        }

        #[test]
        fn reads_float64_from_different_indices() {
            let array = Float64Array::from(vec![1.1, 2.2, 3.3]);
            let field = field_with_real_meta();

            for (idx, expected) in [(0, 1.1), (1, 2.2), (2, 3.3)] {
                let mut value: Double = 0.0;

                let binding = Binding::from_value(CDataType::Double, &mut value);
                let result = read_arrow_value_test(&binding, &array, &field, idx);

                assert!(result.is_ok());
                assert!((value - expected).abs() < f64::EPSILON);
            }
        }
    }

    // Tests for null octet_length_ptr
    mod null_indicator_tests {
        use super::*;

        #[test]
        fn works_with_null_str_len_ptr_for_numeric() {
            let array = Int64Array::from(vec![42i64]);
            let field = field_with_fixed_meta(DataType::Int64, 0, 10);
            let mut value: UBigInt = 0;

            let binding = Binding::from_value(CDataType::UBigInt, &mut value);
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(value, 42u64);
        }

        #[test]
        fn works_with_null_str_len_ptr_for_char() {
            let array = StringArray::from(vec!["hello"]);
            let field = field_with_text_meta();
            let mut buffer = vec![0u8; 32];

            let binding = Binding::from_buffer(CDataType::Char, &mut buffer, std::ptr::null_mut());
            let result = read_arrow_value_test(&binding, &array, &field, 0);

            assert!(result.is_ok());
            assert_eq!(&buffer[..5], b"hello");
            assert_eq!(buffer[5], 0); // Null terminator
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
                buffer_length: size_of::<sql::Numeric>() as sql::Len,
                octet_length_ptr: &mut indicator,
                precision: Some(10),
                scale: Some(2),
                ..Default::default()
            };

            let result = read_arrow_value_test(&binding, &array, &field, 0);

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
                buffer_length: size_of::<sql::Numeric>() as sql::Len,
                octet_length_ptr: std::ptr::null_mut(),
                precision: Some(5),
                scale: Some(0),
                ..Default::default()
            };

            let result = read_arrow_value_test(&binding, &array, &field, 0);

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
                buffer_length: size_of::<sql::Numeric>() as sql::Len,
                octet_length_ptr: std::ptr::null_mut(),
                precision: Some(10),
                scale: Some(4),
                ..Default::default()
            };

            let result = read_arrow_value_test(&binding, &array, &field, 0);

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
        fn defaults_precision_and_scale_when_not_set() {
            let array = Int64Array::from(vec![42i64]);
            let field = field_with_fixed_meta(DataType::Int64, 0, 10);
            let mut numeric = sql::Numeric::default();

            let binding = Binding {
                target_type: CDataType::Numeric,
                target_value_ptr: &mut numeric as *mut sql::Numeric as sql::Pointer,
                buffer_length: size_of::<sql::Numeric>() as sql::Len,
                octet_length_ptr: std::ptr::null_mut(),
                precision: None,
                scale: None,
                ..Default::default()
            };

            let result = read_arrow_value_test(&binding, &array, &field, 0);
            assert!(result.is_ok());
            assert_eq!(numeric.precision, 10);
            assert_eq!(numeric.scale, 0);
            assert_eq!(numeric.sign, 1);
            assert_eq!(u128::from_le_bytes(numeric.val), 42);
        }
    }

    mod fetch_converter_buffer_length {
        use super::*;

        const STRUCT_LEN: sql::Len = std::mem::size_of::<sql::IntervalStruct>() as sql::Len;

        fn interval_year_month_field() -> Field {
            let metadata =
                HashMap::from([("logicalType".to_string(), "INTERVAL_YEAR_MONTH".to_string())]);
            Field::new("test", DataType::Int64, true).with_metadata(metadata)
        }

        fn resolve(
            target_type: CDataType,
            buffer_length: sql::Len,
            field: &Field,
        ) -> OdbcResult<Binding> {
            let binding = Binding {
                target_type,
                buffer_length,
                ..Default::default()
            };
            fetch_converter(&binding, field, &NumericSettings::default()).map(|(_, b)| b)
        }

        #[test]
        fn default_on_interval_rejects_buffer_shorter_than_struct() {
            let err = resolve(
                CDataType::Default,
                STRUCT_LEN - 1,
                &interval_year_month_field(),
            )
            .unwrap_err();
            assert!(
                matches!(err, OdbcError::InvalidBufferLength { length, .. } if length == (STRUCT_LEN - 1) as i64),
                "{err:?}"
            );
        }

        #[test]
        fn default_on_interval_accepts_zero_or_struct_sized_buffer() {
            for buffer_length in [0, STRUCT_LEN, 64] {
                let resolved = resolve(
                    CDataType::Default,
                    buffer_length,
                    &interval_year_month_field(),
                )
                .unwrap();
                assert_eq!(
                    resolved.target_type,
                    CDataType::IntervalYearToMonth,
                    "BufferLength {buffer_length}"
                );
            }
        }

        #[test]
        fn explicit_char_on_interval_accepts_short_buffer() {
            let resolved = resolve(CDataType::Char, 4, &interval_year_month_field()).unwrap();
            assert_eq!(resolved.target_type, CDataType::Char);
        }

        #[test]
        fn default_on_text_accepts_short_buffer() {
            let resolved = resolve(CDataType::Default, 4, &field_with_text_meta()).unwrap();
            assert_eq!(resolved.target_type, CDataType::Char);
        }
    }

    #[test]
    fn fetch_converter_cache_rebuilds_only_when_bindings_or_schema_change() {
        use crate::api::handle_registry::HandleId;
        use crate::api::{ExecutionOrigin, Statement};
        use arrow::record_batch::{RecordBatch, RecordBatchIterator};
        use sf_core::protobuf::generated::database_driver_v1::StatementHandle;

        let field = field_with_fixed_meta(DataType::Int64, 0, 18);
        let schema = Arc::new(arrow::datatypes::Schema::new(vec![field]));
        let fetching = |schema: SchemaRef| {
            let batch =
                RecordBatch::try_new(schema.clone(), vec![Arc::new(Int64Array::from(vec![1]))])
                    .unwrap();
            StatementState::Fetching {
                reader: Box::new(RecordBatchIterator::new(vec![], schema)),
                record_batch: batch,
                batch_idx: 0,
                rows_affected: None,
                origin: ExecutionOrigin::Direct,
            }
            .into()
        };
        let stmt = Statement::new(HandleId::default(), StatementHandle::default(), false);
        let mut inner = stmt.inner.lock();
        inner.state = fetching(schema.clone());
        let bind = |inner: &mut StatementInner, buffer_length| {
            inner.with_active_ard_mut(|ard| {
                ard.bindings.insert(
                    1,
                    Binding {
                        target_type: CDataType::Default,
                        buffer_length,
                        ..Default::default()
                    },
                )
            });
        };
        let converter_ptr = |cache: &FetchConverterCache| {
            cache.entries[0].converter.as_deref().unwrap() as *const dyn ColumnConverter
                as *const ()
        };
        let settings = NumericSettings::default();
        let mut cache = FetchConverterCache::default();

        bind(&mut inner, 8);
        cache.refresh_if_needed(&inner, &settings).unwrap();
        assert_eq!(
            cache.entries[0].binding.target_type,
            resolved_sql_c_default(schema.field(0))
        );
        let first = converter_ptr(&cache);

        inner.state = fetching(Arc::new((*schema).clone()));
        cache.refresh_if_needed(&inner, &settings).unwrap();
        assert_eq!(converter_ptr(&cache), first);

        bind(&mut inner, 16);
        cache.refresh_if_needed(&inner, &settings).unwrap();
        assert_eq!(cache.entries[0].binding.buffer_length, 16);

        let decimal_settings = NumericSettings {
            treat_decimal_as_int: !settings.treat_decimal_as_int,
            ..settings
        };
        cache.refresh_if_needed(&inner, &decimal_settings).unwrap();
        assert_eq!(cache.numeric_settings, Some(decimal_settings));

        let text_schema = Arc::new(arrow::datatypes::Schema::new(vec![field_with_text_meta()]));
        let batch = RecordBatch::try_new(
            Arc::clone(&text_schema),
            vec![Arc::new(arrow::array::StringArray::from(vec!["a"]))],
        )
        .unwrap();
        inner.state = StatementState::Fetching {
            reader: Box::new(RecordBatchIterator::new(vec![], Arc::clone(&text_schema))),
            record_batch: batch,
            batch_idx: 0,
            rows_affected: None,
            origin: ExecutionOrigin::Direct,
        }
        .into();
        cache.refresh_if_needed(&inner, &decimal_settings).unwrap();
        assert!(Arc::ptr_eq(cache.schema.as_ref().unwrap(), &text_schema));
        assert_eq!(cache.entries[0].binding.target_type, CDataType::Char);
    }
}
