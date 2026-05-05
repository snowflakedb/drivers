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
    Binding, BindingStrides, ColumnConverter, ConversionError, NumericSettings, make_converter,
};
use arrow::array::{Array, RecordBatchReader};
use arrow::datatypes::Field;
use arrow::ffi_stream::ArrowArrayStreamReader;
use odbc_sys as sql;
use snafu::ResultExt;
use tracing;

/// One-shot conversion used on the `SQLGetData` path; caching is not
/// worthwhile because only a single cell is read per call.
fn read_arrow_value(
    binding: &Binding,
    array_ref: &dyn Array,
    field: &Field,
    batch_idx: usize,
    numeric_settings: &NumericSettings,
    get_data_offset: &mut Option<usize>,
) -> Result<Warnings, ConversionError> {
    let converter = make_converter(field, numeric_settings)?;
    let warnings = converter.convert_arrow_value(array_ref, batch_idx, binding, get_data_offset)?;
    Ok(warnings)
}

/// Per-batch cache of converters and `Binding` snapshots for the bound
/// columns. Built once per `RecordBatch` and reused across the column-major
/// segments processed by `ColumnConverter::convert_arrow_range`, keeping both the
/// ARD `HashMap` and the per-cell Arrow downcast out of the inner loop.
struct FetchConverterCache {
    /// `Arc::as_ptr(record_batch.column(0))` when the batch has columns,
    /// otherwise the schema `Arc` pointer. Null means "no cache yet".
    batch_identity: *const (),
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
    fn new() -> Self {
        Self {
            batch_identity: std::ptr::null(),
            entries: Vec::new(),
        }
    }

    /// Rebuild the cache if the current `RecordBatch` differs from the
    /// cached one (or if the cache is empty).
    fn refresh_if_needed(
        &mut self,
        inner: &StatementInner,
        numeric_settings: &NumericSettings,
    ) -> OdbcResult<()> {
        let StatementState::Fetching { record_batch, .. } = inner.state.as_ref() else {
            return Ok(());
        };

        let identity: *const () = if record_batch.num_columns() > 0 {
            std::sync::Arc::as_ptr(record_batch.column(0)) as *const ()
        } else {
            std::sync::Arc::as_ptr(&record_batch.schema()) as *const ()
        };

        // `batch_identity` is null after `new()` and only set to a real
        // `Arc::as_ptr` after a successful rebuild, so it doubles as the
        // "have we ever populated this cache?" sentinel — no extra check
        // needed for the empty-bindings case.
        if identity == self.batch_identity {
            return Ok(());
        }

        let schema = record_batch.schema();
        self.entries.clear();
        self.entries.reserve(inner.ard.bindings.len());

        for (&column_number, binding) in &inner.ard.bindings {
            // ODBC reserves column 0 for bookmarks; `checked_sub` also guards
            // debug-mode panics if `SQLBindCol` accepted a 0.
            let arrow_col = (column_number as usize).checked_sub(1);
            let in_range = arrow_col.is_some_and(|idx| idx < schema.fields().len());
            if !in_range {
                // Log once per batch rebuild, not once per fetched row.
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
            }
            let arrow_col = arrow_col.unwrap();
            let field = schema.field(arrow_col);
            let converter = make_converter(field, numeric_settings).context(ConversionSnafu)?;
            self.entries.push(CachedColumn {
                arrow_col,
                binding: *binding,
                converter: Some(converter),
            });
        }

        self.batch_identity = identity;
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
    mut reader: ArrowArrayStreamReader,
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
        | state @ StatementState::PutDataCalled { .. } => {
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
    if matches!(conn.state, ConnectionState::Disconnected) {
        tracing::error!("fetch: connection is disconnected");
        return DisconnectedSnafu.fail();
    }
    let numeric_settings = conn.numeric_settings;
    fetch_impl(&guard, &numeric_settings, warnings)
}

fn fetch_impl(
    guard: &crate::api::handle_registry::HandleGuard<crate::api::Statement>,
    numeric_settings: &NumericSettings,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    let mut inner = guard.inner.lock();
    inner.get_data_state = None;

    let array_size = inner.ard.array_size.max(1);
    let bind_type = inner.ard.bind_type;
    let bind_offset_ptr = inner.ard.bind_offset_ptr;
    let row_status_ptr = inner.ird.array_status_ptr;
    let rows_fetched_ptr = inner.ird.rows_processed_ptr;
    let bind_offset = if !bind_offset_ptr.is_null() {
        unsafe { *bind_offset_ptr }
    } else {
        0
    };

    let mut cache = FetchConverterCache::new();
    // Fast path: a single-row fetch propagates the conversion error to
    // the caller instead of just marking the row as Error.
    if array_size == 1 && bind_offset_ptr.is_null() {
        advance_cursor(&mut inner.state)?;
        cache.refresh_if_needed(&inner, numeric_settings)?;
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
        let outputs = execute_bindings_for_segment(&inner, &cache, arrow_start, 1, 0, 0, 0);
        match outputs.into_iter().next().unwrap_or_else(|| Ok(Vec::new())) {
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
                return Err(e).context(ConversionSnafu);
            }
        }
        return Ok(());
    }

    let mut rows_fetched: usize = 0;
    let mut has_error = false;

    while rows_fetched < array_size {
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

        if let Err(e) = cache.refresh_if_needed(&inner, numeric_settings) {
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
            let wanted = array_size - rows_fetched;
            (*batch_idx, in_batch_remaining.min(wanted))
        };

        let outputs = execute_bindings_for_segment(
            &inner,
            &cache,
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
    if matches!(conn.state, ConnectionState::Disconnected) {
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

    {
        let mut inner = guard.inner.lock();
        inner.used_extended_fetch = true;
        inner.ird.rows_processed_ptr = row_count_ptr;
        inner.ird.array_status_ptr = row_status_ptr;
    }

    fetch_impl(&guard, &numeric_settings, warnings)
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

    if inner.ard.array_size > 1 {
        tracing::warn!("get_data: cannot use SQLGetData with row_array_size > 1");
        return InvalidCursorPositionSnafu.fail();
    }

    // Column 0 is reserved for bookmarks; reject it when bookmarks are off
    if col_or_param_num == 0 {
        return InvalidDescriptorIndexSnafu { number: 0i16 }.fail();
    }

    // SQL_ARD_TYPE: resolve from the ARD descriptor's concise type for the column
    let target_type = if target_type == CDataType::Ard {
        match inner.ard.bindings.get(&col_or_param_num) {
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

            let ard_binding = inner.ard.bindings.get(&col_or_param_num);
            let binding = Binding {
                target_type,
                target_value_ptr,
                buffer_length,
                octet_length_ptr: str_len_or_ind_ptr,
                indicator_ptr: str_len_or_ind_ptr,
                precision: ard_binding.and_then(|b| b.precision),
                scale: ard_binding.and_then(|b| b.scale),
                datetime_interval_precision: ard_binding
                    .and_then(|b| b.datetime_interval_precision),
            };
            let settings = guard.conn()?.connection.lock().numeric_settings;
            let conversion_warnings = read_arrow_value(
                &binding,
                array_ref,
                field,
                *batch_idx,
                &settings,
                &mut offset,
            )
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
        | StatementState::PutDataCalled { .. } => {
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
    use arrow::array::{
        Decimal128Array, Float64Array, Int8Array, Int16Array, Int32Array, Int64Array, StringArray,
    };
    use arrow::datatypes::{DataType, Field};
    use std::collections::HashMap;

    fn read_arrow_value_test(
        binding: &Binding,
        array_ref: &dyn Array,
        field: &Field,
        batch_idx: usize,
    ) -> Result<Warnings, ConversionError> {
        read_arrow_value(
            binding,
            array_ref,
            field,
            batch_idx,
            &NumericSettings::default(),
            &mut None,
        )
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
            let array = StringArray::from(vec!["hello"]);
            let field = field_with_text_meta();
            let mut buffer = vec![0u16; 32];
            let mut str_len: sql::Len = 0;

            let binding = Binding {
                target_type: CDataType::WChar,
                target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
                buffer_length: (buffer.len() * 2) as sql::Len, // buffer_length is in bytes for WChar
                octet_length_ptr: &mut str_len,
                ..Default::default()
            };
            let result = read_arrow_value_test(&binding, &array, &field, 0);

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

            let binding = Binding::from_buffer(CDataType::Default, &mut buffer, &mut str_len);
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

            let binding = Binding::from_buffer(CDataType::Default, &mut buffer, &mut str_len);
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

            let binding = Binding::from_buffer(CDataType::Default, &mut buffer, &mut str_len);
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

            let binding = Binding::from_buffer(CDataType::Default, &mut buffer, &mut str_len);
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

            let binding = Binding::from_buffer(CDataType::Default, &mut buffer, &mut str_len);
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

            let binding = Binding::from_buffer(CDataType::Default, &mut buffer, &mut str_len);
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

            let binding = Binding::from_buffer(CDataType::Default, &mut buffer, &mut str_len);
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

            let binding = Binding::from_buffer(CDataType::Default, &mut buffer, &mut str_len);
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

            let binding = Binding::from_buffer(CDataType::Default, &mut buffer, &mut str_len);
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

            let binding = Binding::from_buffer(CDataType::Default, &mut buffer, &mut str_len);
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
                ..Binding::from_value(CDataType::Default, &mut value)
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
                ..Binding::from_value(CDataType::Default, &mut value)
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
                ..Binding::from_value(CDataType::Default, &mut value)
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
}
