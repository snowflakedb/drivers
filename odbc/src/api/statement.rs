use crate::api::CDataType;
use crate::api::TimestampSubtype;
use crate::api::encoding::OdbcEncoding;
use crate::api::error::{
    ArrowArrayStreamReaderCreationSnafu, ConcatNullValueSnafu, CursorAlreadyOpenSnafu,
    DaeRequiredSnafu, DisconnectedSnafu, InvalidAttributeValueSnafu, InvalidBufferLengthSnafu,
    InvalidCursorStateSnafu, InvalidDuringDaeSnafu, InvalidHandleSnafu,
    InvalidParameterNumberSnafu, InvalidPrecisionOrScaleSnafu, JsonBindingSnafu, NoMoreDataSnafu,
    NonCharBinarySentInPiecesSnafu, NullPointerSnafu, OdbcRuntimeSnafu, OperationCanceledSnafu,
    ReadOnlyAttributeSnafu, Required, StatementNotExecutedSnafu, UnsupportedAttributeSnafu,
    UnsupportedFeatureSnafu,
};
use crate::api::query_type::{QueryType, ResultKind};
use crate::api::runtime::global;
use crate::api::{
    ApdRecord, Connection, ConnectionState, DaeContext, ExecutionOrigin, FreeStmtOption, IpdRecord,
    OdbcError, OdbcResult, ParamDirection, ParamValue, SQL_CONCUR_LOCK, SQL_CONCUR_READ_ONLY,
    SQL_CONCUR_VALUES, SQL_INSENSITIVE, SQL_NONSCROLLABLE, SQL_NOSCAN_OFF, SQL_NOSCAN_ON,
    SQL_RD_OFF, SQL_RD_ON, SQL_SCROLLABLE, SQL_SENSITIVE, SQL_UNSPECIFIED, SqlType, StatementInner,
    StatementState, stmt_from_handle,
};
use crate::conversion::Binding;
use crate::conversion::param_binding::odbc_bindings_to_json;
use arrow::array::RecordBatchReader;
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use odbc_sys as sql;
use sf_core::protobuf::generated::database_driver_v1::{
    ArrowArrayStreamPtr, BinaryDataPtr, ConfigSetting, ConnectionGetParameterRequest,
    ConnectionGetResultSetRequest, ConnectionHandle, ExecuteQueryResponse, QueryBindings,
    ResultSetGetStreamRequest, ResultSetHandle, ResultSetReleaseRequest, ResultSetResponse,
    StatementExecuteQueryRequest, StatementHandle, StatementPrepareRequest,
    StatementSetOptionsRequest, StatementSetSqlQueryRequest, config_setting,
    execute_query_response, query_bindings,
};
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;
use tracing;

/// Scan the APD for parameters marked as data-at-execution.
fn find_dae_params(apd: &crate::api::ApdDescriptor, param_limit: Option<u16>) -> Vec<u16> {
    let mut dae_params = Vec::new();
    for (&param_num, record) in &apd.records {
        if let Some(limit) = param_limit
            && param_num > limit
        {
            continue;
        }
        if !record.str_len_or_ind_ptr.is_null() {
            let ind = unsafe { *record.str_len_or_ind_ptr };
            // SQL_DATA_AT_EXEC (-2): simple DAE flag.
            // SQL_LEN_DATA_AT_EXEC(len) = (-len - 100): DAE with size hint, always <= -100.
            if ind == sql::DATA_AT_EXEC || ind <= -100 {
                dae_params.push(param_num);
            }
        }
    }
    dae_params.sort();
    dae_params
}

/// Execute a SQL statement directly (SQLExecDirect / SQLExecDirectW).
pub fn exec_direct<E: OdbcEncoding>(
    statement_handle: sql::Handle,
    statement_text: *const E::Char,
    text_length: sql::Integer,
    warnings: &mut crate::conversion::warning::Warnings,
) -> OdbcResult<()> {
    let query = E::read_string(statement_text, text_length)?;
    exec_direct_impl(statement_handle, &query, warnings)
}

fn exec_direct_impl(
    statement_handle: sql::Handle,
    statement_text: &str,
    warnings: &mut crate::conversion::warning::Warnings,
) -> OdbcResult<()> {
    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let conn_handle = {
        let conn = dbc.connection.lock();
        match &conn.state {
            ConnectionState::Connected { conn_handle, .. } => *conn_handle,
            ConnectionState::Disconnected => {
                tracing::error!("exec_direct: connection is disconnected");
                return DisconnectedSnafu.fail();
            }
        }
    };
    let mut inner = guard.inner.lock();
    tracing::debug!("exec_direct: statement_handle={:?}", statement_handle);

    if inner.state.as_ref().is_need_data() {
        return InvalidDuringDaeSnafu.fail();
    }

    if inner.state.as_ref().has_open_cursor() {
        tracing::error!("exec_direct: cursor is already open");
        return CursorAlreadyOpenSnafu.fail();
    }

    inner.prepared_param_count = None;

    let dae_params = find_dae_params(&inner.apd, None);
    if !dae_params.is_empty() {
        let pushed_data = dae_params
            .iter()
            .map(|&p| (p, ParamValue::Pending))
            .collect();
        let dae_context = DaeContext {
            dae_params,
            current_index: 0,
            pushed_data,
            deferred_query: Some(statement_text.to_string()),
        };
        inner.state.set(StatementState::AwaitingParamData {
            dae_context: Box::new(dae_context),
            origin: ExecutionOrigin::Direct,
        });
        return DaeRequiredSnafu.fail();
    }

    let stmt_handle = guard.stmt_handle;
    let query_timeout = inner.query_timeout;
    let effective_query = statement_text.to_string();
    let multi_statement_count = inner.multi_statement_count;

    let array_size = inner.apd.array_size.max(1);
    let bind_offset = if inner.apd.bind_offset_ptr.is_null() {
        0
    } else {
        unsafe { *inner.apd.bind_offset_ptr as isize }
    };
    let param_status_ptr = inner.ipd.array_status_ptr;
    let rows_processed_ptr = inner.ipd.rows_processed_ptr;
    let param_operation_ptr = inner.apd.array_status_ptr;

    if array_size == 1 {
        // Honour SQL_PARAM_IGNORE for the single set (mirrors array-loop behaviour).
        if !param_operation_ptr.is_null()
            && unsafe { param_operation_ptr.read() } == SQL_PARAM_IGNORE
        {
            unsafe { write_param_status(param_status_ptr, 0, SQL_PARAM_UNUSED) };
            if !rows_processed_ptr.is_null() {
                unsafe { rows_processed_ptr.write(0) };
            }
            set_state(
                &mut inner,
                StatementState::DmlExecuted {
                    rows_affected: 0,
                    schema: arrow::datatypes::Schema::empty().into(),
                    origin: ExecutionOrigin::Direct,
                },
            );
            inner.rows_returned = 0;
            return Ok(());
        }

        let (bindings, _json_owner) =
            apply_parameter_bindings(&inner.apd, &inner.ipd, false, None, 0, bind_offset)?;

        let token = CancellationToken::new();
        *guard.active_cancel.lock() = Some(token.clone());

        let response = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
            tokio::select! {
                biased;
                _ = token.cancelled() => Err(OperationCanceledSnafu.build()),
                result = async {
                    if multi_statement_count >= 0 {
                        let mut options = std::collections::HashMap::new();
                        options.insert(
                            "multi_statement_count".to_string(),
                            ConfigSetting {
                                value: Some(config_setting::Value::IntValue(
                                    multi_statement_count as i64,
                                )),
                            },
                        );
                        c.statement_set_options(StatementSetOptionsRequest {
                            stmt_handle: Some(stmt_handle),
                            options,
                        })
                        .await?;
                    }

                    c.statement_set_sql_query(StatementSetSqlQueryRequest {
                        stmt_handle: Some(stmt_handle),
                        query: effective_query,
                    })
                    .await?;

                    c.statement_execute_query(StatementExecuteQueryRequest {
                        stmt_handle: Some(stmt_handle),
                        bindings,
                        timeout_seconds: if query_timeout > 0 {
                            Some(query_timeout.min(u32::MAX as sql::ULen) as u32)
                        } else {
                            None
                        },
                    })
                    .await
                } => result.map_err(Into::into),
            }
        });

        *guard.active_cancel.lock() = None;

        tracing::info!("exec_direct: response={:?}", response);
        let response = match response {
            Ok(r) => r,
            Err(e) => {
                if let Some(qid) = e.query_id() {
                    inner.last_query_id = Some(qid.to_owned());
                }
                // write_param_status is null-safe; if the app set PARAM_STATUS_PTR
                // it guaranteed an array of at least array_size (1) elements.
                unsafe { write_param_status(param_status_ptr, 0, SQL_PARAM_ERROR) };
                if !rows_processed_ptr.is_null() {
                    unsafe { rows_processed_ptr.write(1) };
                }
                return Err(e);
            }
        };

        update_numeric_settings(&conn_handle, &mut dbc.connection.lock().numeric_settings)?;
        let apply_result =
            apply_execute_response(&mut inner, conn_handle, response, ExecutionOrigin::Direct);
        // write_param_status is null-safe; if the app set PARAM_STATUS_PTR
        // it guaranteed an array of at least array_size (1) elements.
        unsafe { write_param_status(param_status_ptr, 0, SQL_PARAM_SUCCESS) };
        if !rows_processed_ptr.is_null() {
            unsafe { rows_processed_ptr.write(1) };
        }
        // Re-propagate any error that is NOT NoMoreData (which just means a DML
        // statement affected 0 rows — a valid outcome, not a failure).
        match apply_result {
            Ok(()) | Err(OdbcError::NoMoreData { .. }) => {}
            Err(e) => return Err(e),
        }
    } else {
        // Parameter array execution: send the query text once, then run one
        // RPC per set via the shared helper.
        let token = CancellationToken::new();
        *guard.active_cancel.lock() = Some(token.clone());
        let set_query_result = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
            tokio::select! {
                biased;
                _ = token.cancelled() => Err(OperationCanceledSnafu.build()),
                result = c.statement_set_sql_query(StatementSetSqlQueryRequest {
                    stmt_handle: Some(stmt_handle),
                    query: effective_query,
                }) => result.map_err(Into::into),
            }
        });
        *guard.active_cancel.lock() = None;
        set_query_result?;

        let ArrayLoopResult {
            last_response,
            any_error,
            total_rows_affected,
        } = execute_param_array_loop(
            &mut inner,
            &guard.active_cancel,
            stmt_handle,
            array_size,
            bind_offset,
            param_status_ptr,
            rows_processed_ptr,
            param_operation_ptr,
            false,
            query_timeout,
            multi_statement_count,
        )?;

        if let Some(response) = last_response {
            update_numeric_settings(&conn_handle, &mut dbc.connection.lock().numeric_settings)?;
            // NoMoreData from apply_execute_response means the last set was a
            // DML statement that affected 0 rows — that is a valid outcome and
            // must not override SQL_SUCCESS/SQL_SUCCESS_WITH_INFO for the batch.
            match apply_execute_response(&mut inner, conn_handle, response, ExecutionOrigin::Direct)
            {
                Ok(()) | Err(OdbcError::NoMoreData { .. }) => {
                    // Overwrite the single-set rows_affected with the batch total.
                    inner
                        .state
                        .transition_or_err::<(), ()>(|s| {
                            Ok((
                                match s {
                                    StatementState::DmlExecuted { schema, origin, .. } => {
                                        StatementState::DmlExecuted {
                                            rows_affected: total_rows_affected,
                                            schema,
                                            origin,
                                        }
                                    }
                                    other => other,
                                },
                                (),
                            ))
                        })
                        .ok();
                }
                Err(e) => return Err(e),
            }
            if any_error {
                warnings.push(crate::conversion::warning::Warning::RowError);
            }
        } else if !any_error {
            // All sets were SQL_PARAM_IGNORE — no RPC was ever sent. Transition
            // the statement to DmlExecuted so it is not left in Created/Prepared.
            set_state(
                &mut inner,
                StatementState::DmlExecuted {
                    rows_affected: 0,
                    schema: arrow::datatypes::Schema::empty().into(),
                    origin: ExecutionOrigin::Direct,
                },
            );
            if !rows_processed_ptr.is_null() {
                unsafe { rows_processed_ptr.write(0) };
            }
        }
    }

    inner.rows_returned = 0;
    inner.current_row_number = 0;
    Ok(())
}

use crate::conversion::NumericSettings;

fn update_numeric_settings(
    conn_handle: &ConnectionHandle,
    settings: &mut NumericSettings,
) -> OdbcResult<()> {
    let g = global().context(OdbcRuntimeSnafu)?;
    g.block_on(async |c| {
        if let Ok(resp) = c
            .connection_get_parameter(ConnectionGetParameterRequest {
                conn_handle: Some(*conn_handle),
                key: "ODBC_TREAT_DECIMAL_AS_INT".to_string(),
            })
            .await
            && let Some(value) = resp.value
        {
            let bool_value = value.eq_ignore_ascii_case("true");
            settings.treat_decimal_as_int = bool_value;
            tracing::info!("Server parameter ODBC_TREAT_DECIMAL_AS_INT = {bool_value}");
        }

        if let Ok(resp) = c
            .connection_get_parameter(ConnectionGetParameterRequest {
                conn_handle: Some(*conn_handle),
                key: "ODBC_TREAT_BIG_NUMBER_AS_STRING".to_string(),
            })
            .await
            && let Some(value) = resp.value
        {
            let bool_value = value.eq_ignore_ascii_case("true");
            settings.treat_big_number_as_string = bool_value;
            tracing::info!("Server parameter ODBC_TREAT_BIG_NUMBER_AS_STRING = {bool_value}");
        }

        if let Ok(resp) = c
            .connection_get_parameter(ConnectionGetParameterRequest {
                conn_handle: Some(*conn_handle),
                key: "VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT".to_string(),
            })
            .await
            && let Some(value) = resp.value
            && let Ok(size) = value.parse::<u64>()
        {
            settings.max_varchar_size = size;
            tracing::info!("Server parameter VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT = {size}");
        }

        // TIMESTAMP_TZ_OUTPUT_FORMAT: read on every execute so an in-flight
        // `ALTER SESSION SET TIMESTAMP_TZ_OUTPUT_FORMAT = ...` takes effect
        // for the next statement. Empty / unset / no-TZ-token formats keep
        // the legacy UTC-only fetch behaviour (see
        // `crate::conversion::timestamp::parse_tz_offset_format`).
        //
        // Update semantics differ from the other settings in this
        // function: those have meaningful server-side defaults so
        // resetting to default on a transient RPC failure is harmless.
        // `tz_offset_format` does NOT -- the customer set it
        // deliberately via `ALTER SESSION` and a transient blip silently
        // flipping the next fetch from `+HH:MM` rendering back to bare
        // UTC is a wire-format regression with no diagnostic the
        // application can correlate. So:
        //   - On `Ok(resp)` with a non-empty value -> overwrite cache
        //     (parse_tz_offset_format collapses unrecognised values to
        //     None, which is the spec-correct fall-through to bare UTC).
        //   - On `Ok(resp)` with `None` or empty value -> the user
        //     explicitly UNSET the parameter, so clear the cache.
        //   - On `Err(_)` -> leave the cache untouched and warn.
        // See PR #1068 review on `statement.rs:209`.
        let rpc_result = c
            .connection_get_parameter(ConnectionGetParameterRequest {
                conn_handle: Some(*conn_handle),
                key: "TIMESTAMP_TZ_OUTPUT_FORMAT".to_string(),
            })
            .await
            .map(|resp| resp.value)
            .map_err(|e| format!("{e:?}"));
        apply_tz_offset_format_update(&mut settings.tz_offset_format, rpc_result);
    });
    Ok(())
}

/// Cache-update decision logic for `TIMESTAMP_TZ_OUTPUT_FORMAT`. Pure
/// function so the four-way state table (Ok+set / Ok+empty / Ok+None /
/// Err) can be unit-tested without standing up an RPC mock.
///
/// Semantics (see PR #1068 review on `statement.rs:209`):
/// - `Ok(Some(non_empty))` -> overwrite cache with parsed token (which
///   may itself be `None` if the format string carries no recognised
///   TZ token, the spec-correct fall-through to bare UTC).
/// - `Ok(Some(""))` / `Ok(None)` -> the user explicitly UNSET the
///   parameter, clear the cache.
/// - `Err(_)` -> a transient RPC blip; leave the cache untouched and
///   warn so a customer-configured wire format isn't silently lost.
pub(crate) fn apply_tz_offset_format_update(
    cached: &mut Option<crate::conversion::timestamp::TzOffsetFormat>,
    rpc_result: Result<Option<String>, String>,
) {
    match rpc_result {
        Ok(value) => {
            let new_format = match value.as_deref() {
                Some(v) if !v.is_empty() => crate::conversion::timestamp::parse_tz_offset_format(v),
                _ => None,
            };
            if *cached != new_format {
                tracing::info!(
                    "Server parameter TIMESTAMP_TZ_OUTPUT_FORMAT offset token = {new_format:?}"
                );
            }
            *cached = new_format;
        }
        Err(err) => {
            tracing::warn!(
                error = %err,
                "failed to refresh TIMESTAMP_TZ_OUTPUT_FORMAT; keeping cached value {:?}",
                cached
            );
        }
    }
}

#[cfg(test)]
mod apply_tz_offset_format_update_tests {
    use super::apply_tz_offset_format_update;
    use crate::conversion::timestamp::TzOffsetFormat;

    #[test]
    fn ok_with_recognised_format_overwrites_cache() {
        let mut cached = None;
        apply_tz_offset_format_update(
            &mut cached,
            Ok(Some("YYYY-MM-DD HH24:MI:SS.FF TZH:TZM".to_string())),
        );
        assert_eq!(cached, Some(TzOffsetFormat::Colon));
    }

    #[test]
    fn ok_with_unrecognised_format_clears_cache() {
        // A non-empty format string with no recognised TZ token is the
        // spec-correct fall-through to bare UTC -- the user is asking
        // for a custom format the driver doesn't render an offset for,
        // so we mustn't keep an old offset rendering active.
        let mut cached = Some(TzOffsetFormat::Colon);
        apply_tz_offset_format_update(&mut cached, Ok(Some("YYYY-MM-DD HH24:MI:SS".to_string())));
        assert_eq!(cached, None);
    }

    #[test]
    fn ok_with_empty_string_clears_cache() {
        // Server returns an explicit empty string for an unset parameter
        // on some configurations; treat it as UNSET and revert to bare
        // UTC.
        let mut cached = Some(TzOffsetFormat::NoColon);
        apply_tz_offset_format_update(&mut cached, Ok(Some(String::new())));
        assert_eq!(cached, None);
    }

    #[test]
    fn ok_with_none_clears_cache() {
        let mut cached = Some(TzOffsetFormat::HourOnly);
        apply_tz_offset_format_update(&mut cached, Ok(None));
        assert_eq!(cached, None);
    }

    /// The load-bearing assertion: a transient RPC failure must NOT
    /// silently flip a customer-configured `+HH:MM` rendering back to
    /// bare UTC. Pre-fix, the closure overwrote the cache with `None`
    /// on `Err(_)`, breaking the next fetch with no diagnostic. See PR
    /// #1068 review on `statement.rs:209`.
    #[test]
    fn err_keeps_existing_cached_value() {
        let mut cached = Some(TzOffsetFormat::Colon);
        apply_tz_offset_format_update(&mut cached, Err("transient transport error".to_string()));
        assert_eq!(cached, Some(TzOffsetFormat::Colon));
    }

    /// Symmetric: an `Err` against an already-empty cache must remain
    /// empty (i.e. we don't accidentally synthesise a value).
    #[test]
    fn err_leaves_empty_cache_empty() {
        let mut cached: Option<TzOffsetFormat> = None;
        apply_tz_offset_format_update(&mut cached, Err("transient transport error".to_string()));
        assert_eq!(cached, None);
    }
}

/// Prepare a SQL statement (SQLPrepare / SQLPrepareW).
pub fn prepare<E: OdbcEncoding>(
    statement_handle: sql::Handle,
    statement_text: *const E::Char,
    text_length: sql::Integer,
) -> OdbcResult<()> {
    let query = E::read_string(statement_text, text_length)?;
    prepare_impl(statement_handle, &query)
}

fn reader_from_protobuf_stream(stream: ArrowArrayStreamPtr) -> OdbcResult<ArrowArrayStreamReader> {
    let stream_ptr: *mut FFI_ArrowArrayStream = stream.into();
    let stream = unsafe { FFI_ArrowArrayStream::from_raw(stream_ptr) };
    let reader =
        ArrowArrayStreamReader::try_new(stream).context(ArrowArrayStreamReaderCreationSnafu {})?;
    Ok(reader)
}

fn prepare_impl(statement_handle: sql::Handle, query: &str) -> OdbcResult<()> {
    if statement_handle.is_null() {
        return InvalidHandleSnafu.fail();
    }
    if query.is_empty() {
        return InvalidBufferLengthSnafu { length: 0i64 }.fail();
    }
    tracing::debug!("prepare: statement_handle={:?}", statement_handle);
    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let conn = dbc.connection.lock();
    let _conn_handle = match &conn.state {
        ConnectionState::Connected { conn_handle, .. } => *conn_handle,
        ConnectionState::Disconnected => {
            tracing::error!("prepare: connection is disconnected");
            return DisconnectedSnafu.fail();
        }
    };
    let mut inner = guard.inner.lock();

    if inner.state.as_ref().is_need_data() {
        return InvalidDuringDaeSnafu.fail();
    }

    if inner.state.as_ref().has_open_cursor() {
        tracing::error!("prepare: cursor is already open");
        return CursorAlreadyOpenSnafu.fail();
    }

    tracing::debug!("prepare: query = {query}");

    let stmt_handle = guard.stmt_handle;

    let token = CancellationToken::new();
    *guard.active_cancel.lock() = Some(token.clone());

    let prepare_result = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
        tokio::select! {
            biased;
            _ = token.cancelled() => Err(OperationCanceledSnafu.build()),
            result = async {
                c.statement_set_sql_query(StatementSetSqlQueryRequest {
                    stmt_handle: Some(stmt_handle),
                    query: query.to_string(),
                })
                .await?;

                c.statement_prepare(StatementPrepareRequest {
                    stmt_handle: Some(stmt_handle),
                })
                .await
            } => result.map_err(Into::into),
        }
    });

    *guard.active_cancel.lock() = None;
    let prepare_result = prepare_result?;
    let result = prepare_result.result.required("Result is required")?;
    let stream_ptr = result.stream.required("Stream is required")?;
    let reader = reader_from_protobuf_stream(stream_ptr)?;
    let schema = reader.schema();
    inner.ird.desc_count = schema.fields().len() as sql::SmallInt;

    if result.number_of_binds < 0 {
        tracing::warn!(
            "prepare: server reported negative bind count ({}), treating as 0",
            result.number_of_binds
        );
    }
    let raw_bind_count = result.number_of_binds.max(0);
    let param_count = u16::try_from(raw_bind_count).map_err(|_| {
        crate::api::error::CountFieldIncorrectSnafu {
            reason: format!(
                "server reported {raw_bind_count} parameter markers, exceeds maximum {}",
                u16::MAX
            ),
        }
        .build()
    })?;
    inner.prepared_param_count = Some(param_count);
    let max_varchar = conn.numeric_settings.max_varchar_size;
    inner.ipd.records.retain(|&k, _| k <= param_count);
    for i in 1..=param_count {
        inner
            .ipd
            .records
            .entry(i)
            .or_insert_with(|| IpdRecord::with_varchar_size(max_varchar));
    }
    tracing::info!("prepare: auto-IPD populated {param_count} parameter markers (from server)");

    inner.state.set(StatementState::Prepared { schema });
    tracing::info!("prepare: Successfully prepared statement");
    Ok(())
}

/// Execute a prepared statement
pub fn execute(
    statement_handle: sql::Handle,
    warnings: &mut crate::conversion::warning::Warnings,
) -> OdbcResult<()> {
    tracing::debug!("execute: statement_handle={:?}", statement_handle);
    let guard = stmt_from_handle(statement_handle)?;
    let mut inner = guard.inner.lock();

    if inner.state.as_ref().is_need_data() {
        return InvalidDuringDaeSnafu.fail();
    }

    if inner.state.as_ref().has_open_cursor() {
        tracing::error!("execute: cursor is already open");
        return CursorAlreadyOpenSnafu.fail();
    }

    let origin = match inner.state.as_ref() {
        StatementState::Prepared { schema } => ExecutionOrigin::Prepared {
            schema: schema.clone(),
        },
        StatementState::DdlExecuted { origin, .. } | StatementState::DmlExecuted { origin, .. } => {
            origin.clone()
        }
        _ => ExecutionOrigin::Direct,
    };
    let is_prepared = origin.is_prepared();

    let dbc = guard.conn()?;
    if matches!(dbc.connection.lock().state, ConnectionState::Disconnected) {
        tracing::error!("execute: connection is disconnected");
        return DisconnectedSnafu.fail();
    }

    let dae_params = find_dae_params(&inner.apd, inner.prepared_param_count);
    if !dae_params.is_empty() {
        let pushed_data = dae_params
            .iter()
            .map(|&p| (p, ParamValue::Pending))
            .collect();
        let dae_context = DaeContext {
            dae_params,
            current_index: 0,
            pushed_data,
            deferred_query: None,
        };
        inner.state.set(StatementState::AwaitingParamData {
            dae_context: Box::new(dae_context),
            origin,
        });
        return DaeRequiredSnafu.fail();
    }

    let conn_handle = {
        let connection = dbc.connection.lock();
        match &connection.state {
            ConnectionState::Connected { conn_handle, .. } => *conn_handle,
            ConnectionState::Disconnected => {
                tracing::error!("execute: connection is disconnected");
                return DisconnectedSnafu.fail();
            }
        }
    };

    let stmt_handle = guard.stmt_handle;
    let query_timeout = inner.query_timeout;
    let multi_statement_count = inner.multi_statement_count;

    let array_size = inner.apd.array_size.max(1);
    let bind_offset = if inner.apd.bind_offset_ptr.is_null() {
        0
    } else {
        unsafe { *inner.apd.bind_offset_ptr as isize }
    };
    let param_status_ptr = inner.ipd.array_status_ptr;
    let rows_processed_ptr = inner.ipd.rows_processed_ptr;
    let param_operation_ptr = inner.apd.array_status_ptr;

    if array_size == 1 {
        // Honour SQL_PARAM_IGNORE for the single set (mirrors array-loop behaviour).
        if !param_operation_ptr.is_null()
            && unsafe { param_operation_ptr.read() } == SQL_PARAM_IGNORE
        {
            unsafe { write_param_status(param_status_ptr, 0, SQL_PARAM_UNUSED) };
            if !rows_processed_ptr.is_null() {
                unsafe { rows_processed_ptr.write(0) };
            }
            set_state(
                &mut inner,
                StatementState::DmlExecuted {
                    rows_affected: 0,
                    schema: arrow::datatypes::Schema::empty().into(),
                    origin,
                },
            );
            inner.rows_returned = 0;
            return Ok(());
        }

        let (bindings, _json_owner) = apply_parameter_bindings(
            &inner.apd,
            &inner.ipd,
            is_prepared,
            inner.prepared_param_count,
            0,
            bind_offset,
        )?;

        let token = CancellationToken::new();
        let _cancel_guard = ActiveCancelGuard::arm(&guard.active_cancel, token.clone());

        let response = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
            tokio::select! {
                biased;
                _ = token.cancelled() => Err(OperationCanceledSnafu.build()),
                result = async {
                    if multi_statement_count >= 0 {
                        let mut options = std::collections::HashMap::new();
                        options.insert(
                            "multi_statement_count".to_string(),
                            ConfigSetting {
                                value: Some(config_setting::Value::IntValue(
                                    multi_statement_count as i64,
                                )),
                            },
                        );
                        c.statement_set_options(StatementSetOptionsRequest {
                            stmt_handle: Some(stmt_handle),
                            options,
                        })
                        .await?;
                    }
                    c.statement_execute_query(StatementExecuteQueryRequest {
                        stmt_handle: Some(stmt_handle),
                        bindings,
                        timeout_seconds: if query_timeout > 0 {
                            Some(query_timeout.min(u32::MAX as sql::ULen) as u32)
                        } else {
                            None
                        },
                    })
                    .await
                } => result.map_err(Into::into),
            }
        });

        let response = match response {
            Ok(r) => r,
            Err(e) => {
                if let Some(qid) = e.query_id() {
                    inner.last_query_id = Some(qid.to_owned());
                }
                // write_param_status is null-safe; if the app set PARAM_STATUS_PTR
                // it guaranteed an array of at least array_size (1) elements.
                unsafe { write_param_status(param_status_ptr, 0, SQL_PARAM_ERROR) };
                if !rows_processed_ptr.is_null() {
                    unsafe { rows_processed_ptr.write(1) };
                }
                return Err(e);
            }
        };

        tracing::info!("execute: Successfully executed statement");
        update_numeric_settings(&conn_handle, &mut dbc.connection.lock().numeric_settings)?;
        let apply_result = apply_execute_response(&mut inner, conn_handle, response, origin);
        // write_param_status is null-safe; if the app set PARAM_STATUS_PTR
        // it guaranteed an array of at least array_size (1) elements.
        unsafe { write_param_status(param_status_ptr, 0, SQL_PARAM_SUCCESS) };
        if !rows_processed_ptr.is_null() {
            unsafe { rows_processed_ptr.write(1) };
        }
        // Re-propagate any error that is NOT NoMoreData (which just means a DML
        // statement affected 0 rows — a valid outcome, not a failure).
        match apply_result {
            Ok(()) | Err(OdbcError::NoMoreData { .. }) => {}
            Err(e) => return Err(e),
        }
    } else {
        let loop_result = execute_param_array_loop(
            &mut inner,
            &guard.active_cancel,
            stmt_handle,
            array_size,
            bind_offset,
            param_status_ptr,
            rows_processed_ptr,
            param_operation_ptr,
            is_prepared,
            query_timeout,
            multi_statement_count,
        );
        inner.rows_returned = 0;
        let ArrayLoopResult {
            last_response,
            any_error,
            total_rows_affected,
        } = loop_result?;

        if let Some(response) = last_response {
            tracing::info!("execute: Successfully executed statement (array mode)");
            update_numeric_settings(&conn_handle, &mut dbc.connection.lock().numeric_settings)?;
            // NoMoreData from apply_execute_response means the last set was a
            // DML statement that affected 0 rows — that is a valid outcome and
            // must not override SQL_SUCCESS/SQL_SUCCESS_WITH_INFO for the batch.
            match apply_execute_response(&mut inner, conn_handle, response, origin) {
                Ok(()) | Err(OdbcError::NoMoreData { .. }) => {
                    // Overwrite the single-set rows_affected with the batch total.
                    inner
                        .state
                        .transition_or_err::<(), ()>(|s| {
                            Ok((
                                match s {
                                    StatementState::DmlExecuted { schema, origin, .. } => {
                                        StatementState::DmlExecuted {
                                            rows_affected: total_rows_affected,
                                            schema,
                                            origin,
                                        }
                                    }
                                    other => other,
                                },
                                (),
                            ))
                        })
                        .ok();
                }
                Err(e) => return Err(e),
            }
            if any_error {
                warnings.push(crate::conversion::warning::Warning::RowError);
            }
        } else if !any_error {
            // All sets were SQL_PARAM_IGNORE — no RPC was ever sent. Transition
            // the statement to DmlExecuted so it is not left in Created/Prepared.
            set_state(
                &mut inner,
                StatementState::DmlExecuted {
                    rows_affected: 0,
                    schema: arrow::datatypes::Schema::empty().into(),
                    origin,
                },
            );
            if !rows_processed_ptr.is_null() {
                unsafe { rows_processed_ptr.write(0) };
            }
        }
    }

    inner.rows_returned = 0;
    inner.current_row_number = 0;
    Ok(())
}

fn set_state(stmt: &mut StatementInner, state: StatementState) {
    stmt.ird.desc_count = match &state {
        StatementState::QueryExecuted { reader, .. } => {
            reader.schema().fields().len() as sql::SmallInt
        }
        StatementState::DdlExecuted { .. }
        | StatementState::DmlExecuted { .. }
        | StatementState::Done { .. } => 0,
        _ => stmt.ird.desc_count,
    };
    stmt.state = state.into();
}

/// Process an `ExecuteQueryResponse` and apply the resulting state to the statement.
///
/// For Single results: uses the returned ResultSetHandle to fetch the Arrow stream,
/// then creates the appropriate state (DDL/DML/Query).
/// For Multi results: stores child query IDs, fetches the first child result set,
/// and sets up state for `SQLMoreResults` iteration.
fn apply_execute_response(
    stmt: &mut StatementInner,
    conn_handle: ConnectionHandle,
    response: ExecuteQueryResponse,
    origin: ExecutionOrigin,
) -> OdbcResult<()> {
    let result = response.result.required("Execute result is required")?;

    // Clear previous multi-statement state.
    stmt.multi_query_ids.clear();
    stmt.multi_current_idx = 0;

    match result {
        execute_query_response::Result::Single(rs_response) => {
            let descriptor = rs_response
                .result_descriptor
                .required("Descriptor is required")?;
            let rs_handle = rs_response
                .result_set_handle
                .required("ResultSet handle is required")?;
            let query_id = descriptor.query_id.clone();
            let stream = fetch_stream_and_release(rs_handle)?;
            let execute_state = create_execute_state_from_stream(
                stream,
                descriptor.statement_type_id,
                descriptor.rows_affected,
                origin,
            )?;
            let is_zero_dml = matches!(
                &execute_state,
                StatementState::DmlExecuted {
                    rows_affected: 0,
                    ..
                }
            );
            set_state(stmt, execute_state);
            stmt.last_query_id = Some(query_id).filter(|s| !s.is_empty());
            if is_zero_dml {
                return NoMoreDataSnafu.fail();
            }
            Ok(())
        }
        execute_query_response::Result::Multi(multi) => {
            let parent_query_id = multi
                .parent
                .as_ref()
                .map(|p| p.query_id.clone())
                .unwrap_or_default();
            stmt.last_query_id = Some(parent_query_id).filter(|s| !s.is_empty());
            stmt.multi_query_ids = multi.query_ids;

            if stmt.multi_query_ids.is_empty() {
                // No child statements — treat as DDL with no cursor.
                set_state(
                    stmt,
                    StatementState::DdlExecuted {
                        schema: arrow::datatypes::Schema::empty().into(),
                        origin,
                    },
                );
                return NoMoreDataSnafu.fail();
            }

            // Fetch and apply the first child result set.
            let first_id = &stmt.multi_query_ids[0];
            let rs = fetch_result_set_by_query_id(conn_handle, first_id)?;
            let descriptor = rs.result_descriptor.as_ref();
            let statement_type_id = descriptor.and_then(|d| d.statement_type_id);
            let rows_affected = descriptor.and_then(|d| d.rows_affected);
            let rs_handle = rs
                .result_set_handle
                .required("ResultSet handle is required")?;
            let stream = fetch_stream_and_release(rs_handle)?;
            let execute_state =
                create_execute_state_from_stream(stream, statement_type_id, rows_affected, origin)?;
            stmt.multi_current_idx = 1;
            set_state(stmt, execute_state);
            Ok(())
        }
    }
}

/// Fetch a ResultSetResponse (handle + descriptor) for a given query ID via the connection.
fn fetch_result_set_by_query_id(
    conn_handle: ConnectionHandle,
    query_id: &str,
) -> OdbcResult<ResultSetResponse> {
    let response = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
        c.connection_get_result_set(ConnectionGetResultSetRequest {
            conn_handle: Some(conn_handle),
            query_id: query_id.to_string(),
        })
        .await
    })?;
    Ok(response)
}

/// Fetch the Arrow stream from a ResultSet handle and release the handle.
///
/// `result_set_get_stream` takes ownership of the prebuilt stream (one-shot),
/// so the handle is no longer useful after this call.
fn fetch_stream_and_release(rs_handle: ResultSetHandle) -> OdbcResult<ArrowArrayStreamPtr> {
    let stream = {
        let response = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
            c.result_set_get_stream(ResultSetGetStreamRequest {
                result_set_handle: Some(rs_handle),
            })
            .await
        })?;
        response.stream.required("Stream is required")?
    };
    release_result_set(rs_handle);
    Ok(stream)
}

fn release_result_set(rs_handle: ResultSetHandle) {
    if let Ok(rt) = global() {
        let _ = rt.block_on(async |c| {
            c.result_set_release(ResultSetReleaseRequest {
                result_set_handle: Some(rs_handle),
            })
            .await
        });
    }
}

/// Release any server-side result-set resources held by a response that will
/// not be consumed (i.e. all but the last response in an array-parameter batch).
fn discard_response(response: ExecuteQueryResponse) {
    if let Some(execute_query_response::Result::Single(rs)) = response.result
        && let Some(handle) = rs.result_set_handle
    {
        release_result_set(handle);
    }
    // Multi responses carry only query IDs (no live ResultSetHandle), so there
    // is nothing to release for them.
}

/// Result of running the parameter-array execution loop.
struct ArrayLoopResult {
    /// Last successful response (apply as the statement's active result).
    last_response: Option<ExecuteQueryResponse>,
    /// True when at least one set failed.
    any_error: bool,
    /// Accumulated rows affected across all successful DML sets.
    total_rows_affected: i64,
}

/// Execute the parameter-array loop (PARAMSET_SIZE > 1).
///
/// Iterates over `array_size` sets, skipping those marked `SQL_PARAM_IGNORE`,
/// and calls `statement_execute_query` once per set.  Uses a single batch-wide
/// `CancellationToken` so that `SQLCancel` reliably aborts the in-flight RPC
/// regardless of which set is executing.
///
/// Intermediate responses (all but the last) are released immediately to avoid
/// server-side resource leaks.  The `total_rows_affected` field accumulates DML
/// row counts so `SQLRowCount` reflects the whole batch, not just the last set.
#[allow(clippy::too_many_arguments)]
fn execute_param_array_loop(
    inner: &mut StatementInner,
    active_cancel: &parking_lot::Mutex<Option<CancellationToken>>,
    stmt_handle: StatementHandle,
    array_size: usize,
    bind_offset: isize,
    param_status_ptr: *mut u16,
    rows_processed_ptr: *mut sql::ULen,
    param_operation_ptr: *const u16,
    is_prepared: bool,
    query_timeout: sql::ULen,
    multi_statement_count: sql::SmallInt,
) -> OdbcResult<ArrayLoopResult> {
    let mut sets_processed: usize = 0;
    let mut last_response: Option<ExecuteQueryResponse> = None;
    let mut last_error: Option<OdbcError> = None;
    let mut any_error = false;
    let mut total_rows_affected: i64 = 0;

    // A single token covers the entire batch so SQLCancel stops the in-flight
    // RPC no matter which iteration is currently executing.
    let batch_token = CancellationToken::new();
    *active_cancel.lock() = Some(batch_token.clone());

    for set_idx in 0..array_size {
        // Honour SQL_PARAM_IGNORE on APD.array_status_ptr.
        if !param_operation_ptr.is_null()
            // SAFETY: null-checked above; app guarantees the array has array_size elements.
            && unsafe { param_operation_ptr.add(set_idx).read() } == SQL_PARAM_IGNORE
        {
            // write_param_status is null-safe; app guarantees PARAM_STATUS_PTR
            // array has at least array_size elements when non-null.
            unsafe { write_param_status(param_status_ptr, set_idx, SQL_PARAM_UNUSED) };
            continue;
        }

        let bindings_result = apply_parameter_bindings(
            &inner.apd,
            &inner.ipd,
            is_prepared,
            inner.prepared_param_count,
            set_idx,
            bind_offset,
        );
        let (bindings, _json_owner) = match bindings_result {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("array loop: set_idx={set_idx} parameter binding failed: {e}");
                // write_param_status is null-safe; see comment above.
                unsafe { write_param_status(param_status_ptr, set_idx, SQL_PARAM_ERROR) };
                add_param_set_error_diag(inner, set_idx, &e);
                last_error = Some(e);
                sets_processed += 1;
                if !rows_processed_ptr.is_null() {
                    unsafe { rows_processed_ptr.write(sets_processed as sql::ULen) };
                }
                any_error = true;
                continue;
            }
        };

        let exec_result = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
            // Forward multi_statement_count when set (matches single-set path).
            if multi_statement_count >= 0 {
                let mut options = std::collections::HashMap::new();
                options.insert(
                    "multi_statement_count".to_string(),
                    ConfigSetting {
                        value: Some(config_setting::Value::IntValue(
                            multi_statement_count as i64,
                        )),
                    },
                );
                c.statement_set_options(StatementSetOptionsRequest {
                    stmt_handle: Some(stmt_handle),
                    options,
                })
                .await
                .map_err(OdbcError::from)?;
            }
            tokio::select! {
                biased;
                _ = batch_token.cancelled() => Err(OperationCanceledSnafu.build()),
                result = c.statement_execute_query(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt_handle),
                    bindings,
                    timeout_seconds: if query_timeout > 0 {
                        Some(query_timeout.min(u32::MAX as sql::ULen) as u32)
                    } else {
                        None
                    },
                }) => result.map_err(Into::into),
            }
        });

        sets_processed += 1;
        // Write the running count after each set so a cancellation handler
        // or diagnostic inspector can observe progress mid-batch.
        if !rows_processed_ptr.is_null() {
            unsafe { rows_processed_ptr.write(sets_processed as sql::ULen) };
        }
        match exec_result {
            Ok(resp) => {
                // Accumulate row counts so SQLRowCount reflects the whole batch.
                if let Some(execute_query_response::Result::Single(ref rs)) = resp.result
                    && let Some(ref d) = rs.result_descriptor
                {
                    total_rows_affected += d.rows_affected.unwrap_or(0);
                }
                // write_param_status is null-safe; see comment above.
                unsafe { write_param_status(param_status_ptr, set_idx, SQL_PARAM_SUCCESS) };
                // Release the previous response before replacing it to avoid leaking
                // server-side result-set resources (e.g. SELECT in array mode).
                if let Some(prev) = last_response.take() {
                    discard_response(prev);
                }
                last_response = Some(resp);
            }
            Err(e) => {
                tracing::error!("array loop: set_idx={set_idx} execution failed: {e}");
                if let Some(qid) = e.query_id() {
                    inner.last_query_id = Some(qid.to_owned());
                }
                // write_param_status is null-safe; see comment above.
                unsafe { write_param_status(param_status_ptr, set_idx, SQL_PARAM_ERROR) };
                add_param_set_error_diag(inner, set_idx, &e);
                last_error = Some(e);
                any_error = true;
            }
        }
    }

    *active_cancel.lock() = None;

    if any_error && last_response.is_none() {
        return Err(last_error.ok_or_else(|| {
            crate::api::error::InternalSnafu {
                message: "array execution: any_error flag set but no error stored".to_string(),
            }
            .build()
        })?);
    }

    Ok(ArrayLoopResult {
        last_response,
        any_error,
        total_rows_affected,
    })
}

fn create_execute_state_from_stream(
    stream: ArrowArrayStreamPtr,
    statement_type_id: Option<i64>,
    rows_affected: Option<i64>,
    origin: ExecutionOrigin,
) -> OdbcResult<StatementState> {
    let reader = reader_from_protobuf_stream(stream)?;
    let schema = reader.schema();

    let state = match QueryType::from_raw(statement_type_id).result_kind() {
        ResultKind::UpdateCount => StatementState::DmlExecuted {
            rows_affected: rows_affected.unwrap_or(0),
            schema,
            origin,
        },
        ResultKind::Cursor => StatementState::QueryExecuted {
            reader,
            rows_affected,
            origin,
        },
        ResultKind::NoResult => StatementState::DdlExecuted { schema, origin },
    };
    Ok(state)
}

/// Build JSON query bindings from ODBC parameter bindings for a specific set index.
///
/// When `prepared` is true (SQLPrepare+SQLExecute flow), the IPD has server-
/// provided parameter count and we validate that the APD covers every marker.
/// When `prepared` is false (SQLExecDirect), the IPD only has records from
/// SQLBindParameter — we send whatever the APD has and let the server validate.
///
/// `prepared_param_count` caps how many parameters are serialized for prepared
/// statements, preventing phantom bindings beyond the server-reported marker
/// count from being dereferenced.
///
/// `set_idx` is 0-based (pass 0 for the single-set / non-array path).
/// `bind_offset` is the dereferenced value of `APD.bind_offset_ptr` (pass 0 when null).
fn apply_parameter_bindings(
    apd: &crate::api::ApdDescriptor,
    ipd: &crate::api::IpdDescriptor,
    prepared: bool,
    prepared_param_count: Option<u16>,
    set_idx: usize,
    bind_offset: isize,
) -> OdbcResult<(Option<QueryBindings>, Option<String>)> {
    let effective_count: u16 = if prepared {
        prepared_param_count.ok_or_else(|| {
            crate::api::error::CountFieldIncorrectSnafu {
                reason: "prepared statement is missing prepared_param_count".to_string(),
            }
            .build()
        })?
    } else {
        apd.desc_count().max(ipd.desc_count())
    };

    if effective_count == 0 {
        return Ok((None, None));
    }

    if apd.records.is_empty() {
        if prepared {
            return crate::api::error::CountFieldIncorrectSnafu {
                reason: format!(
                    "parameter 1 is not bound (statement has {effective_count} parameter markers)"
                ),
            }
            .fail();
        }
        return Ok((None, None));
    }

    if prepared {
        for i in 1..=effective_count {
            if !apd.records.contains_key(&i) {
                return crate::api::error::CountFieldIncorrectSnafu {
                    reason: format!(
                        "parameter {i} is not bound (statement has {effective_count} parameter markers)"
                    ),
                }
                .fail();
            }
        }
    }
    tracing::info!(
        "apply_parameter_bindings: Found {} bound parameters (effective_count={}, set_idx={})",
        apd.records.len(),
        effective_count,
        set_idx,
    );

    let json_string = odbc_bindings_to_json(apd, ipd, effective_count, set_idx, bind_offset)
        .context(JsonBindingSnafu {})?;

    let json_data_ptr = json_string.as_bytes().as_ptr() as u64;
    let json_data_len = json_string.len();

    let binary_data_ptr = BinaryDataPtr {
        value: json_data_ptr.to_le_bytes().to_vec(),
        length: json_data_len as i64,
    };

    let bindings = QueryBindings {
        binding_type: Some(query_bindings::BindingType::Json(binary_data_ptr)),
    };

    tracing::info!("apply_parameter_bindings: Successfully bound parameters");

    Ok((Some(bindings), Some(json_string)))
}

/// Write a per-set status code to `IPD.array_status_ptr`.
///
/// Does nothing when `ptr` is null (the app did not set `PARAM_STATUS_PTR`).
///
/// # Safety
/// When `ptr` is non-null the caller must ensure it points to an array of at
/// least `set_idx + 1` elements (guaranteed by the ODBC application which must
/// size the array to at least `SQL_ATTR_PARAMSET_SIZE`).
unsafe fn write_param_status(ptr: *mut u16, set_idx: usize, status: u16) {
    if !ptr.is_null() {
        unsafe { ptr.add(set_idx).write(status) };
    }
}

/// Add a diagnostic record identifying the 1-based parameter-set row that
/// failed during array execution (SQLSTATE 01S01 per ODBC §13.2).
fn add_param_set_error_diag(stmt: &mut StatementInner, set_idx: usize, err: &OdbcError) {
    use crate::api::diagnostic::{ClassOrigin, DiagnosticRecord};
    stmt.diagnostic_info.add_record(DiagnosticRecord {
        sql_state: err.to_sql_state(),
        class_origin: ClassOrigin::Odbc3_0,
        native_error: err.to_native_error(),
        row_number: Some((set_idx + 1) as sql::Integer),
        column_number: None,
        connection_name: String::new(),
        message_text: format!("Error in parameter set {}: {err}", set_idx + 1),
    });
}

// SQL_PARAM_* status codes written to IPD.array_status_ptr.
const SQL_PARAM_SUCCESS: u16 = 0;
const SQL_PARAM_ERROR: u16 = 5;
const SQL_PARAM_UNUSED: u16 = 7;
// SQL_PARAM_IGNORE is the value placed in APD.array_status_ptr to skip a set.
const SQL_PARAM_IGNORE: u16 = 1;

/// Bind a parameter to a prepared statement
#[allow(clippy::too_many_arguments)]
pub fn bind_parameter(
    statement_handle: sql::Handle,
    parameter_number: sql::USmallInt,
    raw_input_output_type: sql::SmallInt,
    raw_value_type: sql::SmallInt,
    raw_parameter_type: sql::SmallInt,
    column_size: sql::ULen,
    decimal_digits: sql::SmallInt,
    parameter_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
) -> OdbcResult<()> {
    tracing::debug!(
        "bind_parameter: parameter_number={}, input_output_type={}, value_type={}, parameter_type={}",
        parameter_number,
        raw_input_output_type,
        raw_value_type,
        raw_parameter_type
    );

    if statement_handle.is_null() {
        return InvalidHandleSnafu.fail();
    }

    let guard = stmt_from_handle(statement_handle)?;
    let inner = guard.inner.lock();
    if inner.state.as_ref().is_need_data() {
        return InvalidDuringDaeSnafu.fail();
    }

    if parameter_number == 0 {
        tracing::error!("bind_parameter: parameter_number cannot be 0");
        return InvalidParameterNumberSnafu.fail();
    }

    let direction = ParamDirection::try_from(raw_input_output_type)?;

    let value_type = CDataType::try_from(raw_value_type)?;

    let sql_type = SqlType::try_from(raw_parameter_type)?;
    let parameter_type: sql::SqlDataType = sql_type.into();

    // Normalise Snowflake vendor timestamp codes (2000/2001/2002) to the
    // standard SQL_TYPE_TIMESTAMP (93) on the IPD, while remembering the
    // chosen subtype on `sf_subtype`. Keeps `SQLDescribeParam` and
    // `SQLGetDescField(IPD, SQL_DESC_TYPE)` returning spec-mandated codes
    // while still letting the bind pipeline route to the right Snowflake
    // logical type.
    let sf_subtype = TimestampSubtype::from_parameter_type(parameter_type);
    let stored_sql_data_type = if sf_subtype.is_some() {
        sql::SqlDataType::TIMESTAMP
    } else {
        parameter_type
    };

    if direction == ParamDirection::Input
        && parameter_value_ptr.is_null()
        && str_len_or_ind_ptr.is_null()
    {
        tracing::error!(
            "bind_parameter: both parameter_value_ptr and str_len_or_ind_ptr are null for input parameter"
        );
        return NullPointerSnafu.fail();
    }

    if buffer_length < 0 {
        return InvalidBufferLengthSnafu {
            length: buffer_length as i64,
        }
        .fail();
    }

    if decimal_digits < 0 {
        return InvalidPrecisionOrScaleSnafu {
            reason: format!("decimal_digits ({decimal_digits}) must not be negative"),
        }
        .fail();
    }

    // TODO: validate that (value_type, sql_type) is a supported conversion,
    // returning UnsupportedFeatureSnafu (HYC00) if not.

    // Re-lock inner (was dropped after DAE check above so we could do validation
    // without holding the lock, but in practice this is fine to hold throughout).
    drop(inner);
    let mut inner = guard.inner.lock();

    inner.apd.records.insert(
        parameter_number,
        ApdRecord {
            value_type,
            data_ptr: parameter_value_ptr,
            buffer_length,
            str_len_or_ind_ptr,
        },
    );

    inner.ipd.records.insert(
        parameter_number,
        IpdRecord {
            sql_data_type: stored_sql_data_type,
            column_size,
            decimal_digits,
            direction: raw_input_output_type,
            sf_subtype,
            ..IpdRecord::default()
        },
    );

    tracing::info!(
        "bind_parameter: Successfully bound parameter {}",
        parameter_number
    );
    Ok(())
}

/// Free statement resources based on the option
pub fn free_stmt(statement_handle: sql::Handle, option: FreeStmtOption) -> OdbcResult<()> {
    tracing::debug!("free_stmt: statement_handle={statement_handle:?}, option={option:?}");

    if statement_handle.is_null() {
        return InvalidHandleSnafu.fail();
    }
    let guard = stmt_from_handle(statement_handle)?;
    let mut inner = guard.inner.lock();

    if inner.state.as_ref().is_need_data() {
        return InvalidDuringDaeSnafu.fail();
    }

    match option {
        FreeStmtOption::Close => {
            tracing::info!("free_stmt: Closing cursor");
            let transition = match inner.state.as_ref() {
                StatementState::Created | StatementState::Prepared { .. } => None,
                StatementState::QueryExecuted { origin, .. }
                | StatementState::Fetching { origin, .. }
                | StatementState::DdlExecuted { origin, .. }
                | StatementState::DmlExecuted { origin, .. }
                | StatementState::Done { origin, .. } => {
                    let next = origin.restore_state();
                    let desc_count = match &next {
                        StatementState::Prepared { schema } => {
                            schema.fields().len() as sql::SmallInt
                        }
                        _ => 0,
                    };
                    Some((next, desc_count))
                }
                _ => Some((StatementState::Created, 0)),
            };
            if let Some((state, desc_count)) = transition {
                inner.state.set(state);
                inner.ird.desc_count = desc_count;
                inner.get_data_state = None;
                inner.used_extended_fetch = false;
                inner.current_row_number = 0;
            }
        }
        FreeStmtOption::Unbind => {
            tracing::info!("free_stmt: Unbinding all columns");
            inner.ard.unbind_all();
        }
        FreeStmtOption::ResetParams => {
            tracing::info!("free_stmt: Resetting all parameter bindings");
            inner.apd.clear();
            if let Some(count) = inner.prepared_param_count {
                inner.ipd.records.retain(|&k, _| k <= count);
            }
        }
    }

    Ok(())
}

/// Close the cursor on a statement, returning SQLSTATE 24000 if no cursor is open.
/// Unlike `free_stmt(SQL_CLOSE)`, which silently no-ops when no cursor is open,
/// this function errors per the ODBC spec for `SQLCloseCursor`.
pub fn close_cursor(statement_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!("close_cursor: statement_handle={statement_handle:?}");

    {
        let guard = stmt_from_handle(statement_handle)?;
        let inner = guard.inner.lock();

        if inner.state.as_ref().is_need_data() {
            return InvalidDuringDaeSnafu.fail();
        }

        if !inner.state.as_ref().has_open_cursor() {
            return InvalidCursorStateSnafu.fail();
        }
    }

    free_stmt(statement_handle, FreeStmtOption::Close)
}

/// Return the number of parameters in the statement via the IPD descriptor.
///
/// After `SQLPrepare`, auto-IPD populates the IPD with one record per `?`
/// marker, so this works even without prior `SQLBindParameter` calls.
pub fn num_params(
    statement_handle: sql::Handle,
    param_count_ptr: *mut sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("num_params: statement_handle={:?}", statement_handle);

    let guard = stmt_from_handle(statement_handle)?;
    let inner = guard.inner.lock();

    if inner.state.as_ref().is_need_data() {
        return InvalidDuringDaeSnafu.fail();
    }

    if matches!(inner.state.as_ref(), StatementState::Created) {
        return StatementNotExecutedSnafu.fail();
    }

    let count = inner.ipd.desc_count();

    if !param_count_ptr.is_null() {
        unsafe {
            *param_count_ptr = count as sql::SmallInt;
        }
    }

    tracing::info!("num_params: {} parameters", count);
    Ok(())
}

/// Describe a parameter via the IPD descriptor.
///
/// Works for both explicitly bound parameters and auto-IPD markers
/// populated during `SQLPrepare`.
pub fn describe_param(
    statement_handle: sql::Handle,
    parameter_number: sql::USmallInt,
    data_type_ptr: *mut sql::SmallInt,
    parameter_size_ptr: *mut sql::ULen,
    decimal_digits_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!(
        "describe_param: statement_handle={:?}, parameter_number={}",
        statement_handle,
        parameter_number
    );

    if parameter_number == 0 {
        return InvalidParameterNumberSnafu.fail();
    }

    let guard = stmt_from_handle(statement_handle)?;
    let inner = guard.inner.lock();

    if inner.state.as_ref().is_need_data() {
        return InvalidDuringDaeSnafu.fail();
    }

    let allowed = match inner.state.as_ref() {
        StatementState::Prepared { .. } => true,
        StatementState::DdlExecuted { origin, .. }
        | StatementState::DmlExecuted { origin, .. }
        | StatementState::Done { origin, .. } => origin.is_prepared(),
        _ => false,
    };
    if !allowed {
        return StatementNotExecutedSnafu.fail();
    }
    let ipd_rec = inner.ipd.records.get(&parameter_number).ok_or_else(|| {
        tracing::error!(
            "describe_param: parameter #{} not found in IPD",
            parameter_number
        );
        InvalidParameterNumberSnafu.build()
    })?;

    if !data_type_ptr.is_null() {
        unsafe {
            *data_type_ptr = ipd_rec.sql_data_type.0;
        }
    }
    if !parameter_size_ptr.is_null() {
        unsafe {
            *parameter_size_ptr = ipd_rec.column_size;
        }
    }
    if !decimal_digits_ptr.is_null() {
        unsafe {
            *decimal_digits_ptr = ipd_rec.decimal_digits;
        }
    }
    if !nullable_ptr.is_null() {
        unsafe {
            *nullable_ptr = ipd_rec.nullable;
        }
    }

    tracing::info!(
        "describe_param: parameter {} type={:?} size={} digits={} nullable={}",
        parameter_number,
        ipd_rec.sql_data_type,
        ipd_rec.column_size,
        ipd_rec.decimal_digits,
        ipd_rec.nullable,
    );
    Ok(())
}

/// Bind a column to a statement
pub fn bind_col(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    target_type: CDataType,
    target_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
) -> OdbcResult<()> {
    tracing::debug!(
        "bind_col: statement_handle={:?}, column_number={}, target_type={:?}",
        statement_handle,
        column_number,
        target_type
    );

    let guard = stmt_from_handle(statement_handle)?;
    let mut inner = guard.inner.lock();

    if inner.state.as_ref().is_need_data() {
        return InvalidDuringDaeSnafu.fail();
    }

    // Per ODBC specification, if target_value_ptr is null, unbind the column
    if target_value_ptr.is_null() {
        tracing::debug!("bind_col: unbinding column {}", column_number);
        inner.ard.bindings.remove(&column_number);
    } else {
        if buffer_length < 0 {
            return InvalidBufferLengthSnafu {
                length: buffer_length as i64,
            }
            .fail();
        }
        inner.ard.bindings.insert(
            column_number,
            Binding {
                target_type,
                target_value_ptr,
                buffer_length,
                octet_length_ptr: str_len_or_ind_ptr,
                indicator_ptr: str_len_or_ind_ptr,
                precision: None,
                scale: None,
                datetime_interval_precision: None,
            },
        );
    }
    Ok(())
}

/// Set a statement attribute value
pub fn set_stmt_attr(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    _string_length: sql::Integer,
    warnings: &mut crate::conversion::warning::Warnings,
) -> OdbcResult<()> {
    use crate::api::{CursorType, StmtAttr};
    use crate::conversion::warning::Warning;

    tracing::debug!(
        "set_stmt_attr: statement_handle={:?}, attribute={}, value_ptr={:?}",
        statement_handle,
        attribute,
        value_ptr
    );

    let attr = StmtAttr::try_from(attribute)?;
    let guard = stmt_from_handle(statement_handle)?;
    let mut inner = guard.inner.lock();

    if inner.state.as_ref().is_need_data() {
        return InvalidDuringDaeSnafu.fail();
    }

    match attr {
        StmtAttr::CursorType => {
            let raw = value_ptr as sql::ULen;
            let requested = CursorType::try_from(raw)?;
            tracing::debug!("set_stmt_attr: CursorType requested = {requested:?}");
            if requested != CursorType::ForwardOnly {
                inner.cursor_type = CursorType::ForwardOnly;
                warnings.push(Warning::OptionValueChanged);
            } else {
                inner.cursor_type = CursorType::ForwardOnly;
            }
            Ok(())
        }
        StmtAttr::MaxLength => {
            let length = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: MaxLength = {}", length);
            inner.max_length = length;
            Ok(())
        }
        StmtAttr::UseBookmarks => {
            tracing::debug!("set_stmt_attr: UseBookmarks (ignored, bookmarks not supported)");
            Ok(())
        }
        StmtAttr::RowArraySize => {
            let size = value_ptr as usize;
            tracing::debug!("set_stmt_attr: RowArraySize = {}", size);
            let effective_size = if size == 0 {
                tracing::warn!("set_stmt_attr: RowArraySize value 0 is invalid; coercing to 1");
                1
            } else {
                size
            };
            inner.ard.array_size = effective_size;
            Ok(())
        }
        StmtAttr::RowStatusPtr => {
            let ptr = value_ptr as *mut u16;
            tracing::debug!("set_stmt_attr: RowStatusPtr = {:?}", ptr);
            inner.ird.array_status_ptr = ptr;
            Ok(())
        }
        StmtAttr::RowsFetchedPtr => {
            let ptr = value_ptr as *mut sql::ULen;
            tracing::debug!("set_stmt_attr: RowsFetchedPtr = {:?}", ptr);
            inner.ird.rows_processed_ptr = ptr;
            Ok(())
        }
        StmtAttr::RowBindType => {
            let raw_bind_type = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: RowBindType (raw) = {}", raw_bind_type);
            inner.ard.bind_type = raw_bind_type;
            Ok(())
        }
        StmtAttr::RowBindOffsetPtr => {
            let ptr = value_ptr as *mut sql::Len;
            tracing::debug!("set_stmt_attr: RowBindOffsetPtr = {:?}", ptr);
            inner.ard.bind_offset_ptr = ptr;
            Ok(())
        }
        StmtAttr::RowOperationPtr => {
            let ptr = value_ptr as *mut u16;
            tracing::debug!("set_stmt_attr: RowOperationPtr = {:?}", ptr);
            inner.ard.array_status_ptr = ptr;
            Ok(())
        }
        StmtAttr::ParamsetSize => {
            let size = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: ParamsetSize = {}", size);
            // Reject values that would overflow a signed isize used in loop
            // bounds and pointer arithmetic (e.g. (SQLULEN)-1 = 2^64-1).
            if size > (isize::MAX as sql::ULen) {
                return InvalidAttributeValueSnafu {
                    attribute: 22i32, // SQL_ATTR_PARAMSET_SIZE
                    value: size as i64,
                }
                .fail();
            }
            if size == 0 {
                tracing::warn!("set_stmt_attr: ParamsetSize value 0 is invalid; coercing to 1");
                inner.apd.array_size = 1;
                warnings.push(Warning::OptionValueChanged);
            } else {
                inner.apd.array_size = size as usize;
            }
            Ok(())
        }
        StmtAttr::ParamBindType => {
            let val = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: ParamBindType = {}", val);
            inner.apd.bind_type = val;
            Ok(())
        }
        StmtAttr::ParamBindOffsetPtr => {
            let ptr = value_ptr as *mut sql::Len;
            tracing::debug!("set_stmt_attr: ParamBindOffsetPtr = {:?}", ptr);
            inner.apd.bind_offset_ptr = ptr;
            Ok(())
        }
        StmtAttr::ParamOperationPtr => {
            let ptr = value_ptr as *mut u16;
            tracing::debug!("set_stmt_attr: ParamOperationPtr = {:?}", ptr);
            inner.apd.array_status_ptr = ptr;
            Ok(())
        }
        StmtAttr::ParamStatusPtr => {
            let ptr = value_ptr as *mut u16;
            tracing::debug!("set_stmt_attr: ParamStatusPtr = {:?}", ptr);
            inner.ipd.array_status_ptr = ptr;
            Ok(())
        }
        StmtAttr::ParamsProcessedPtr => {
            let ptr = value_ptr as *mut sql::ULen;
            tracing::debug!("set_stmt_attr: ParamsProcessedPtr = {:?}", ptr);
            inner.ipd.rows_processed_ptr = ptr;
            Ok(())
        }
        StmtAttr::MetadataId => {
            let val = value_ptr as sql::ULen;
            inner.metadata_id = val != 0;
            Ok(())
        }
        StmtAttr::SnowflakeLastQueryId
        | StmtAttr::ImpRowDesc
        | StmtAttr::ImpParamDesc
        | StmtAttr::RowNumber => {
            tracing::warn!("set_stmt_attr: {:?} is read-only", attr);
            ReadOnlyAttributeSnafu { attribute }.fail()
        }
        StmtAttr::QueryTimeout => {
            let val = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: QueryTimeout = {}", val);
            if val > u32::MAX as sql::ULen {
                return InvalidAttributeValueSnafu {
                    attribute,
                    value: val as i64,
                }
                .fail();
            }
            inner.query_timeout = val;
            Ok(())
        }
        StmtAttr::MaxRows => {
            let val = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: MaxRows = {}", val);
            inner.max_rows = val;
            Ok(())
        }
        StmtAttr::Noscan => {
            let val = value_ptr as sql::ULen;
            match val {
                SQL_NOSCAN_OFF | SQL_NOSCAN_ON => {
                    inner.noscan = val;
                    Ok(())
                }
                _ => InvalidAttributeValueSnafu {
                    attribute,
                    value: val as i64,
                }
                .fail(),
            }
        }
        StmtAttr::Concurrency => {
            // 24000 if a cursor is open (includes Done — all rows fetched but not yet closed)
            if inner.state.as_ref().has_open_cursor() {
                tracing::error!("set_stmt_attr: Concurrency cannot be set while cursor is open");
                return InvalidCursorStateSnafu.fail();
            }
            let val = value_ptr as sql::ULen;
            match val {
                SQL_CONCUR_READ_ONLY => {
                    inner.concurrency = val;
                    Ok(())
                }
                SQL_CONCUR_LOCK..=SQL_CONCUR_VALUES => {
                    // SQL_CONCUR_LOCK / SQL_CONCUR_ROWVER / SQL_CONCUR_VALUES
                    // Snowflake cursors are always read-only; substitute and warn
                    inner.concurrency = SQL_CONCUR_READ_ONLY;
                    warnings.push(Warning::OptionValueChanged);
                    Ok(())
                }
                _ => InvalidAttributeValueSnafu {
                    attribute,
                    value: val as i64,
                }
                .fail(),
            }
        }
        StmtAttr::CursorScrollable => {
            if inner.state.as_ref().has_open_cursor() {
                return InvalidCursorStateSnafu.fail();
            }
            let val = value_ptr as sql::ULen;
            match val {
                SQL_NONSCROLLABLE => {
                    inner.cursor_scrollable = val;
                    Ok(())
                }
                SQL_SCROLLABLE => {
                    // Substitute with SQL_NONSCROLLABLE + 01S02
                    inner.cursor_scrollable = SQL_NONSCROLLABLE;
                    warnings.push(Warning::OptionValueChanged);
                    Ok(())
                }
                _ => InvalidAttributeValueSnafu {
                    attribute,
                    value: val as i64,
                }
                .fail(),
            }
        }
        StmtAttr::CursorSensitivity => {
            if inner.state.as_ref().has_open_cursor() {
                return InvalidCursorStateSnafu.fail();
            }
            let val = value_ptr as sql::ULen;
            match val {
                SQL_UNSPECIFIED => {
                    inner.cursor_sensitivity = val;
                    Ok(())
                }
                SQL_INSENSITIVE | SQL_SENSITIVE => {
                    // Substitute with SQL_UNSPECIFIED + 01S02
                    inner.cursor_sensitivity = SQL_UNSPECIFIED;
                    warnings.push(Warning::OptionValueChanged);
                    Ok(())
                }
                _ => InvalidAttributeValueSnafu {
                    attribute,
                    value: val as i64,
                }
                .fail(),
            }
        }
        StmtAttr::EnableAutoIpd => {
            let val = value_ptr as sql::ULen;
            match val {
                0 => {
                    // SQL_FALSE — accepted (no-op)
                    tracing::debug!("set_stmt_attr: EnableAutoIpd = SQL_FALSE (no-op)");
                    Ok(())
                }
                1 => {
                    // SQL_TRUE — valid value, but optional feature not implemented
                    tracing::debug!("set_stmt_attr: EnableAutoIpd = SQL_TRUE is not supported");
                    UnsupportedFeatureSnafu.fail()
                }
                _ => InvalidAttributeValueSnafu {
                    attribute,
                    value: val as i64,
                }
                .fail(),
            }
        }
        StmtAttr::KeysetSize => {
            let val = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: KeysetSize = {}", val);
            inner.keyset_size = val;
            Ok(())
        }
        StmtAttr::SimulateCursor => {
            if inner.state.as_ref().has_open_cursor() {
                return InvalidCursorStateSnafu.fail();
            }
            let val = value_ptr as sql::ULen;
            match val {
                0 => {
                    // SQL_SC_NON_UNIQUE — accepted
                    inner.simulate_cursor = val;
                    Ok(())
                }
                1 | 2 => {
                    // SQL_SC_TRY_UNIQUE / SQL_SC_UNIQUE — substitute with SQL_SC_NON_UNIQUE + 01S02
                    inner.simulate_cursor = 0;
                    warnings.push(Warning::OptionValueChanged);
                    Ok(())
                }
                _ => InvalidAttributeValueSnafu {
                    attribute,
                    value: val as i64,
                }
                .fail(),
            }
        }
        StmtAttr::RetrieveData => {
            let val = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: RetrieveData = {}", val);
            match val {
                SQL_RD_OFF | SQL_RD_ON => {
                    inner.retrieve_data = val;
                    Ok(())
                }
                _ => InvalidAttributeValueSnafu {
                    attribute,
                    value: val as i64,
                }
                .fail(),
            }
        }
        StmtAttr::SnowflakeMultiStatementCount => {
            let val = value_ptr as isize as i64;
            if val < -1 || val > i16::MAX as i64 {
                return InvalidAttributeValueSnafu {
                    attribute,
                    value: val,
                }
                .fail();
            }
            inner.multi_statement_count = val as i16;
            Ok(())
        }
        _ => {
            tracing::warn!("set_stmt_attr: unsupported attribute {:?}", attr);
            UnsupportedAttributeSnafu { attribute }.fail()
        }
    }
}

/// Get a statement attribute value
pub fn get_stmt_attr<E: OdbcEncoding>(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
    warnings: &mut crate::conversion::warning::Warnings,
) -> OdbcResult<()> {
    use crate::api::StmtAttr;
    use crate::api::encoding::write_string_bytes_i32;

    tracing::debug!("get_stmt_attr: attribute={}", attribute);

    let attr = StmtAttr::try_from(attribute)?;
    let guard = stmt_from_handle(statement_handle)?;
    let mut inner = guard.inner.lock();

    if inner.state.as_ref().is_need_data() {
        return InvalidDuringDaeSnafu.fail();
    }

    match attr {
        StmtAttr::CursorType => {
            unsafe {
                std::ptr::write_unaligned(
                    value_ptr as *mut sql::ULen,
                    inner.cursor_type as sql::ULen,
                );
                if !string_length_ptr.is_null() {
                    std::ptr::write_unaligned(
                        string_length_ptr,
                        size_of::<sql::ULen>() as sql::Integer,
                    );
                }
            }
            Ok(())
        }
        StmtAttr::MaxLength => {
            unsafe {
                *(value_ptr as *mut sql::ULen) = inner.max_length;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::AppRowDesc => {
            let ard_ptr = &mut inner.ard as *mut crate::api::ArdDescriptor as sql::Handle;
            unsafe {
                *(value_ptr as *mut sql::Handle) = ard_ptr;
            }
            Ok(())
        }
        StmtAttr::ImpRowDesc => {
            let ird_ptr = &mut inner.ird as *mut crate::api::IrdDescriptor as sql::Handle;
            unsafe {
                *(value_ptr as *mut sql::Handle) = ird_ptr;
            }
            Ok(())
        }
        StmtAttr::AppParamDesc => {
            let apd_ptr = &mut inner.apd as *mut crate::api::ApdDescriptor as sql::Handle;
            unsafe {
                *(value_ptr as *mut sql::Handle) = apd_ptr;
            }
            Ok(())
        }
        StmtAttr::ImpParamDesc => {
            let ipd_ptr = &mut inner.ipd as *mut crate::api::IpdDescriptor as sql::Handle;
            unsafe {
                *(value_ptr as *mut sql::Handle) = ipd_ptr;
            }
            Ok(())
        }
        StmtAttr::RowNumber => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::ULen) = inner.current_row_number;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::RowArraySize => {
            unsafe {
                *(value_ptr as *mut sql::ULen) = inner.ard.array_size as sql::ULen;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::RowStatusPtr => {
            unsafe {
                *(value_ptr as *mut *mut u16) = inner.ird.array_status_ptr;
            }
            Ok(())
        }
        StmtAttr::RowsFetchedPtr => {
            unsafe {
                *(value_ptr as *mut *mut sql::ULen) = inner.ird.rows_processed_ptr;
            }
            Ok(())
        }
        StmtAttr::RowBindType => {
            unsafe {
                *(value_ptr as *mut sql::ULen) = inner.ard.bind_type;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::RowBindOffsetPtr => {
            unsafe {
                *(value_ptr as *mut *mut sql::Len) = inner.ard.bind_offset_ptr;
            }
            Ok(())
        }
        StmtAttr::RowOperationPtr => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut *mut u16) = inner.ard.array_status_ptr;
                }
            }
            Ok(())
        }
        StmtAttr::ParamsetSize => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::ULen) = inner.apd.array_size as sql::ULen;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::ParamBindType => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::ULen) = inner.apd.bind_type;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::ParamBindOffsetPtr => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut *mut sql::Len) = inner.apd.bind_offset_ptr;
                }
            }
            Ok(())
        }
        StmtAttr::ParamOperationPtr => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut *mut u16) = inner.apd.array_status_ptr;
                }
            }
            Ok(())
        }
        StmtAttr::ParamStatusPtr => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut *mut u16) = inner.ipd.array_status_ptr;
                }
            }
            Ok(())
        }
        StmtAttr::ParamsProcessedPtr => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut *mut sql::ULen) = inner.ipd.rows_processed_ptr;
                }
            }
            Ok(())
        }
        StmtAttr::MetadataId => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::ULen) = inner.metadata_id as sql::ULen;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::SnowflakeLastQueryId => {
            if buffer_length < 0 {
                return InvalidBufferLengthSnafu {
                    length: buffer_length as i64,
                }
                .fail();
            }
            let query_id = inner.last_query_id.as_deref().unwrap_or("");
            write_string_bytes_i32::<E>(
                query_id,
                value_ptr as *mut E::Char,
                buffer_length,
                string_length_ptr,
                Some(warnings),
            );
            Ok(())
        }
        StmtAttr::QueryTimeout => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = inner.query_timeout };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::MaxRows => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = inner.max_rows };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::Noscan => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = inner.noscan };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::Concurrency => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = inner.concurrency };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::CursorScrollable => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = inner.cursor_scrollable };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::CursorSensitivity => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = inner.cursor_sensitivity };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::EnableAutoIpd => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = 0 }; // Always SQL_FALSE
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::KeysetSize => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = inner.keyset_size };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::SimulateCursor => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = inner.simulate_cursor };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::RetrieveData => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = inner.retrieve_data };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::SnowflakeMultiStatementCount => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::Integer) = inner.multi_statement_count as sql::Integer;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = size_of::<sql::Integer>() as sql::Integer;
                }
            }
            Ok(())
        }
        _ => {
            tracing::warn!("get_stmt_attr: unsupported attribute {:?}", attr);
            crate::api::error::UnknownAttributeSnafu { attribute }.fail()
        }
    }
}

/// SQLParamData — advance the DAE state machine.
///
/// State transitions:
/// - S8 (AwaitingParamData) → S9 (AwaitingPutData): writes the current
///   parameter's token to `*value_ptr_ptr` and returns `SQL_NEED_DATA`.
/// - S9 (AwaitingPutData) → HY010: consecutive `SQLParamData` without an
///   intervening `SQLPutData` is a function-sequence error.
/// - S10 (PutDataCalled) → S9 (AwaitingPutData) if more params remain,
///   returning `SQL_NEED_DATA`. If all params are supplied, executes the
///   deferred query and transitions to the appropriate executed state.
pub fn param_data(
    statement_handle: sql::Handle,
    value_ptr_ptr: *mut sql::Pointer,
) -> OdbcResult<()> {
    tracing::debug!("param_data: statement_handle={statement_handle:?}");

    if statement_handle.is_null() {
        return InvalidHandleSnafu.fail();
    }

    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    // Lock `Connection` before `inner` so the all-DAE-params-supplied branch can
    // hand a `&mut Connection` to `execute_dae` without re-locking. Acquiring it
    // unconditionally also closes the TOCTOU window against a concurrent
    // `SQLDisconnect`, matching `exec_direct_impl` / `prepare_impl`.
    let mut conn = dbc.connection.lock();
    let mut inner = guard.inner.lock();

    match inner.state.take() {
        // S8 → S9: first SQLParamData call after SQLExecute/SQLExecDirect
        // returned SQL_NEED_DATA. Expose the first DAE parameter's token.
        StatementState::AwaitingParamData {
            dae_context,
            origin,
        } => {
            let param_num = dae_context.dae_params[dae_context.current_index];
            if !value_ptr_ptr.is_null() {
                let token = get_param_token(&inner.apd, param_num);
                unsafe { *value_ptr_ptr = token };
            }
            inner.state.set(StatementState::AwaitingPutData {
                dae_context,
                origin,
            });
            DaeRequiredSnafu.fail()
        }

        // S9 → HY010: SQLParamData called again without SQLPutData.
        StatementState::AwaitingPutData {
            dae_context,
            origin,
        } => {
            inner.state.set(StatementState::AwaitingPutData {
                dae_context,
                origin,
            });
            InvalidDuringDaeSnafu.fail()
        }

        // S10 → S9 or execute: SQLPutData was called at least once.
        // Advance to the next parameter, or execute if all are provided.
        StatementState::PutDataCalled {
            mut dae_context,
            origin,
        } => {
            dae_context.current_index += 1;

            if dae_context.current_index < dae_context.dae_params.len() {
                let param_num = dae_context.dae_params[dae_context.current_index];
                if !value_ptr_ptr.is_null() {
                    let token = get_param_token(&inner.apd, param_num);
                    unsafe { *value_ptr_ptr = token };
                }
                inner.state.set(StatementState::AwaitingPutData {
                    dae_context,
                    origin,
                });
                DaeRequiredSnafu.fail()
            } else {
                let restored = origin.restore_state();
                execute_dae(
                    &mut inner,
                    &mut conn,
                    guard.stmt_handle,
                    &guard.active_cancel,
                    *dae_context,
                    origin,
                    restored,
                )
            }
        }

        other => {
            inner.state.set(other);
            InvalidDuringDaeSnafu.fail()
        }
    }
}

/// Return the application's `ParameterValuePtr` token for a DAE parameter.
/// This is the value the application passed to `SQLBindParameter` as the
/// `ParameterValuePtr` argument — the DM commonly uses a small integer
/// cast to pointer so the application can identify which parameter is being
/// requested.
fn get_param_token(apd: &crate::api::ApdDescriptor, param_num: u16) -> sql::Pointer {
    apd.records
        .get(&param_num)
        .map_or(std::ptr::null_mut(), |r| r.data_ptr)
}

/// SQLPutData — supply data for a DAE parameter.
///
/// Accumulates one chunk of data for the current parameter.
/// Transitions S9 (AwaitingPutData) → S10 (PutDataCalled).
/// Also accepts S10 → S10 for multi-chunk puts.
pub fn put_data(
    statement_handle: sql::Handle,
    data_ptr: sql::Pointer,
    str_len_or_ind: sql::Len,
) -> OdbcResult<()> {
    tracing::debug!("put_data: statement_handle={statement_handle:?}");

    if statement_handle.is_null() {
        return InvalidHandleSnafu.fail();
    }

    let guard = stmt_from_handle(statement_handle)?;
    let mut inner = guard.inner.lock();

    match inner.state.take() {
        // S9 → S10 on success, S9 → S9 on error
        StatementState::AwaitingPutData {
            mut dae_context,
            origin,
        } => {
            let result = put_data_inner(&inner.apd, &mut dae_context, data_ptr, str_len_or_ind);
            inner.state.set(if result.is_ok() {
                StatementState::PutDataCalled {
                    dae_context,
                    origin,
                }
            } else {
                StatementState::AwaitingPutData {
                    dae_context,
                    origin,
                }
            });
            result
        }
        // S10 → S10 regardless of success or error
        StatementState::PutDataCalled {
            mut dae_context,
            origin,
        } => {
            let result = put_data_inner(&inner.apd, &mut dae_context, data_ptr, str_len_or_ind);
            inner.state.set(StatementState::PutDataCalled {
                dae_context,
                origin,
            });
            result
        }

        other => {
            inner.state.set(other);
            InvalidDuringDaeSnafu.fail()
        }
    }
}

/// Validate inputs and accumulate one `SQLPutData` chunk.
///
/// Separated from `put_data()` so that each match arm can restore its own
/// state variant on error. This makes it structurally impossible to restore
/// the wrong ODBC state (S9 vs S10) after a validation failure -- the
/// compiler enforces correctness rather than relying on a manual boolean flag.
fn put_data_inner(
    apd: &crate::api::ApdDescriptor,
    dae_context: &mut DaeContext,
    data_ptr: sql::Pointer,
    str_len_or_ind: sql::Len,
) -> OdbcResult<()> {
    let param_num = dae_context.dae_params[dae_context.current_index];

    // HY009: null DataPtr with non-null-data, non-zero indicator.
    // Per spec, (null, 0) and (null, SQL_NULL_DATA) are both valid.
    if data_ptr.is_null() && str_len_or_ind != sql::NULL_DATA && str_len_or_ind != 0 {
        return NullPointerSnafu.fail();
    }

    // HY090: negative StrLen_or_Ind that isn't SQL_NTS or SQL_NULL_DATA
    if str_len_or_ind < 0 && str_len_or_ind != sql::NTS && str_len_or_ind != sql::NULL_DATA {
        return InvalidBufferLengthSnafu {
            length: str_len_or_ind as i64,
        }
        .fail();
    }

    let c_type = apd
        .records
        .get(&param_num)
        .map(|r| r.value_type)
        .unwrap_or(CDataType::Default);
    accumulate_put_data(dae_context, param_num, data_ptr, str_len_or_ind, c_type)
}

/// Accumulate a single `SQLPutData` chunk into the DAE context.
fn accumulate_put_data(
    ctx: &mut DaeContext,
    param_num: u16,
    data_ptr: sql::Pointer,
    str_len_or_ind: sql::Len,
    c_type: CDataType,
) -> OdbcResult<()> {
    let entry = ctx.pushed_data.get_mut(&param_num).ok_or_else(|| {
        crate::api::error::CountFieldIncorrectSnafu {
            reason: format!("DAE param {param_num} not found in pushed_data"),
        }
        .build()
    })?;

    // HY020: cannot mix SQL_NULL_DATA with previously sent data chunks
    if matches!(entry, ParamValue::Data(chunks) if !chunks.is_empty())
        && str_len_or_ind == sql::NULL_DATA
    {
        return ConcatNullValueSnafu.fail();
    }

    if str_len_or_ind == sql::NULL_DATA {
        *entry = ParamValue::Null;
        return Ok(());
    }

    // HY020: cannot send data after SQL_NULL_DATA was already set
    if matches!(entry, ParamValue::Null) {
        return ConcatNullValueSnafu.fail();
    }

    // HY019: only character (SQL_C_CHAR, SQL_C_WCHAR) and binary (SQL_C_BINARY)
    // types may be sent in multiple pieces. A second chunk for any other type
    // is a spec violation.
    if matches!(entry, ParamValue::Data(chunks) if !chunks.is_empty()) {
        let splittable = matches!(
            c_type,
            CDataType::Char | CDataType::WChar | CDataType::Binary
        );
        if !splittable {
            return NonCharBinarySentInPiecesSnafu.fail();
        }
    }

    let len = if str_len_or_ind == sql::NTS {
        unsafe {
            let cstr = std::ffi::CStr::from_ptr(data_ptr as *const std::os::raw::c_char);
            cstr.to_bytes().len()
        }
    } else if str_len_or_ind < 0 {
        return InvalidBufferLengthSnafu {
            length: str_len_or_ind as i64,
        }
        .fail();
    } else {
        str_len_or_ind as usize
    };

    if len == 0 {
        return Ok(());
    }

    let chunk = unsafe { std::slice::from_raw_parts(data_ptr as *const u8, len) }.to_vec();

    match entry {
        ParamValue::Pending => *entry = ParamValue::Data(vec![chunk]),
        ParamValue::Data(chunks) => chunks.push(chunk),
        ParamValue::Null => unreachable!("NULL case handled above"),
    }
    Ok(())
}

/// Clears `active_cancel` when dropped so every exit path releases the
/// in-flight cancellation slot.
struct ActiveCancelGuard<'a> {
    slot: &'a parking_lot::Mutex<Option<CancellationToken>>,
}

impl<'a> ActiveCancelGuard<'a> {
    fn arm(
        slot: &'a parking_lot::Mutex<Option<CancellationToken>>,
        token: CancellationToken,
    ) -> Self {
        *slot.lock() = Some(token);
        Self { slot }
    }
}

impl Drop for ActiveCancelGuard<'_> {
    fn drop(&mut self) {
        *self.slot.lock() = None;
    }
}

/// Overwrite a DAE parameter's APD record to represent SQL NULL.
fn mark_apd_record_null(
    apd: &mut crate::api::ApdDescriptor,
    param_num: u16,
    null_indicators: &mut Vec<sql::Len>,
) {
    null_indicators.push(sql::NULL_DATA);
    if let Some(rec) = apd.records.get_mut(&param_num) {
        rec.data_ptr = std::ptr::null_mut();
        rec.str_len_or_ind_ptr = null_indicators.last_mut().unwrap();
    }
}

/// Execute the deferred query after all DAE parameters have been supplied.
///
/// Builds temporary `ApdRecord`s from the accumulated `ParamValue` data,
/// merges them with the existing APD/IPD bindings, serializes to JSON,
/// and sends the query to sf_core.
fn execute_dae(
    inner: &mut StatementInner,
    conn: &mut Connection,
    stmt_handle: StatementHandle,
    active_cancel: &parking_lot::Mutex<Option<CancellationToken>>,
    dae_context: DaeContext,
    origin: ExecutionOrigin,
    restored: StatementState,
) -> OdbcResult<()> {
    let is_prepared = origin.is_prepared();

    let conn_handle = match &conn.state {
        ConnectionState::Connected { conn_handle, .. } => *conn_handle,
        ConnectionState::Disconnected => {
            tracing::error!("execute_dae: connection is disconnected");
            inner.state.set(restored);
            return DisconnectedSnafu.fail();
        }
    };

    // Build a temporary APD with DAE parameters replaced by their
    // accumulated data, keeping non-DAE records as-is.
    let mut temp_apd = crate::api::ApdDescriptor::new();
    for (&param_num, rec) in &inner.apd.records {
        temp_apd.records.insert(
            param_num,
            ApdRecord {
                value_type: rec.value_type,
                data_ptr: rec.data_ptr,
                buffer_length: rec.buffer_length,
                str_len_or_ind_ptr: rec.str_len_or_ind_ptr,
            },
        );
    }

    let mut dae_buffers: Vec<Vec<u8>> = Vec::new();
    let param_count = dae_context.pushed_data.len();
    let mut null_indicators: Vec<sql::Len> = Vec::with_capacity(param_count);
    let mut len_indicators: Vec<sql::Len> = Vec::with_capacity(param_count);
    for (&param_num, value) in &dae_context.pushed_data {
        match value {
            ParamValue::Null | ParamValue::Pending => {
                if matches!(value, ParamValue::Pending) {
                    tracing::warn!(
                        "execute_dae: param {param_num} still pending, treating as null"
                    );
                }
                mark_apd_record_null(&mut temp_apd, param_num, &mut null_indicators);
            }
            ParamValue::Data(chunks) => {
                let concatenated: Vec<u8> = chunks.iter().flat_map(|c| c.iter().copied()).collect();
                len_indicators.push(concatenated.len() as sql::Len);
                dae_buffers.push(concatenated);
                let buf = dae_buffers.last().unwrap();
                if let Some(rec) = temp_apd.records.get_mut(&param_num) {
                    rec.data_ptr = buf.as_ptr() as sql::Pointer;
                    rec.buffer_length = buf.len() as sql::Len;
                    rec.str_len_or_ind_ptr = len_indicators.last_mut().unwrap();
                }
            }
        }
    }

    let (bindings, _json_owner) = match apply_parameter_bindings(
        &temp_apd,
        &inner.ipd,
        is_prepared,
        inner.prepared_param_count,
        0,
        0,
    ) {
        Ok(b) => b,
        Err(e) => {
            inner.state.set(restored);
            return Err(e);
        }
    };

    let query_timeout = inner.query_timeout;
    let deferred_query = dae_context.deferred_query;

    let token = CancellationToken::new();
    let _cancel_guard = ActiveCancelGuard::arm(active_cancel, token.clone());

    let globals = match global().context(OdbcRuntimeSnafu) {
        Err(e) => {
            inner.state.set(restored);
            return Err(e);
        }
        Ok(globals) => globals,
    };
    let response = globals.block_on(async |c| {
        tokio::select! {
            biased;
            _ = token.cancelled() => Err(OperationCanceledSnafu.build()),
            result = async {
                if let Some(query) = deferred_query {
                    c.statement_set_sql_query(StatementSetSqlQueryRequest {
                        stmt_handle: Some(stmt_handle),
                        query,
                    })
                    .await?;
                }
                c.statement_execute_query(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt_handle),
                    bindings,
                    timeout_seconds: if query_timeout > 0 {
                        Some(query_timeout.min(u32::MAX as sql::ULen) as u32)
                    } else {
                        None
                    },
                })
                .await
            } => result.map_err(Into::into),
        }
    });

    let response = match response {
        Ok(r) => r,
        Err(e) => {
            inner.state.set(restored);
            if let Some(qid) = e.query_id() {
                inner.last_query_id = Some(qid.to_owned());
            }
            return Err(e);
        }
    };

    tracing::info!("execute_dae: Successfully executed deferred statement");
    if let Err(e) = update_numeric_settings(&conn_handle, &mut conn.numeric_settings) {
        inner.state.set(restored);
        return Err(e);
    }
    apply_execute_response(inner, conn_handle, response, origin)?;
    inner.rows_returned = 0;
    Ok(())
}

/// Cancel processing on a statement (SQLCancel).
///
/// Two-path design for safe cross-thread cancellation:
/// - Path 1: If an RPC is in flight (`active_cancel` is `Some`), cancel the
///   token. The executing thread observes this via `tokio::select!`. Never
///   touches the inner Mutex.
/// - Path 2: If no RPC is in flight (`active_cancel` is `None`), check for
///   NeedData state and restore it. This is a single-threaded scenario.
///
/// Per ODBC 3.5 spec, cross-thread `SQLCancel` does not clear or post
/// diagnostic records.
pub fn cancel(statement_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!("cancel: statement_handle={:?}", statement_handle);

    // TODO(SNOW-3258918): Cancel async execution.
    // Blocked by: SQLSetStmtAttr does not support SQL_ATTR_ASYNC_ENABLE.

    // TODO(SNOW-3258922): Cancel execution on another thread.
    // Blocked by: no server-side cancel RPC. When implemented,
    // cancelling the token resolves the cancelled() future observed
    // by the executing thread's tokio::select!, aborting the in-flight RPC.

    let guard = stmt_from_handle(statement_handle)?;

    // Path 1: cancel in-flight RPC without touching inner.
    {
        let active = guard.active_cancel.lock();
        if let Some(token) = active.as_ref() {
            token.cancel();
            return Ok(());
        }
    }

    // Path 2: no RPC in flight — check NeedData state (single-threaded).
    let mut inner = guard.inner.lock();
    match inner.state.as_ref() {
        StatementState::AwaitingParamData { origin, .. }
        | StatementState::AwaitingPutData { origin, .. }
        | StatementState::PutDataCalled { origin, .. } => {
            // TODO(SNOW-3258919): Full cancel testing during NeedData.
            let restored = origin.restore_state();
            inner.state.set(restored);
        }
        _ => {}
    }

    Ok(())
}

/// Advance to the next result set in a multi-statement execution (SQLMoreResults).
///
/// Returns `Ok(())` when a new result set is available, or `NoMoreDataSnafu`
/// when all result sets have been consumed (the cursor is closed).
pub fn more_results(statement_handle: sql::Handle) -> OdbcResult<()> {
    let guard = stmt_from_handle(statement_handle)?;
    let mut inner = guard.inner.lock();
    tracing::debug!(
        "more_results: multi_current_idx={}, multi_query_ids.len()={}",
        inner.multi_current_idx,
        inner.multi_query_ids.len()
    );

    let origin = match inner.state.as_ref() {
        StatementState::QueryExecuted { origin, .. }
        | StatementState::Fetching { origin, .. }
        | StatementState::DdlExecuted { origin, .. }
        | StatementState::DmlExecuted { origin, .. }
        | StatementState::Done { origin, .. } => origin.clone(),
        _ => ExecutionOrigin::Direct,
    };

    if inner.multi_current_idx >= inner.multi_query_ids.len() {
        // No more result sets — close cursor per ODBC spec.
        // Drop inner lock before calling free_stmt which will re-acquire it.
        drop(inner);
        free_stmt(statement_handle, FreeStmtOption::Close)?;
        let mut inner = guard.inner.lock();
        inner.multi_query_ids.clear();
        inner.multi_current_idx = 0;
        return NoMoreDataSnafu.fail();
    }

    let query_id = inner.multi_query_ids[inner.multi_current_idx].clone();
    inner.multi_current_idx += 1;

    let dbc = guard.conn()?;
    let conn = dbc.connection.lock();
    let conn_handle = match &conn.state {
        ConnectionState::Connected { conn_handle, .. } => *conn_handle,
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
    };
    drop(conn);

    let rs = fetch_result_set_by_query_id(conn_handle, &query_id)?;
    let descriptor = rs.result_descriptor.as_ref();
    let statement_type_id = descriptor.and_then(|d| d.statement_type_id);
    let rows_affected = descriptor.and_then(|d| d.rows_affected);
    let rs_handle = rs
        .result_set_handle
        .required("ResultSet handle is required")?;
    let stream = fetch_stream_and_release(rs_handle)?;
    let execute_state =
        create_execute_state_from_stream(stream, statement_type_id, rows_affected, origin)?;
    set_state(&mut inner, execute_state);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::runtime::global;
    use crate::api::{ApdDescriptor, IpdDescriptor, SqlState};

    #[test]
    fn active_cancel_guard_clears_slot_on_drop() {
        let slot = parking_lot::Mutex::new(None);
        {
            let token = CancellationToken::new();
            let _guard = ActiveCancelGuard::arm(&slot, token);
            assert!(slot.lock().is_some());
        }
        assert!(slot.lock().is_none());
    }

    /// Mirrors `execute` / `execute_dae`: arm `active_cancel`, fail before `block_on`, slot must clear.
    #[test]
    fn active_cancel_cleared_after_runtime_unavailable() {
        let slot = parking_lot::Mutex::new(None);
        {
            let token = CancellationToken::new();
            let _guard = ActiveCancelGuard::arm(&slot, token);
            assert!(global().context(OdbcRuntimeSnafu).is_err());
        }
        assert!(slot.lock().is_none());
    }

    #[test]
    fn apply_bindings_prepared_without_param_count_errors() {
        let apd = ApdDescriptor::new();
        let ipd = IpdDescriptor::new();
        let result = apply_parameter_bindings(&apd, &ipd, true, None, 0, 0);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.to_sql_state(), SqlState::CountFieldIncorrect);
    }
}
