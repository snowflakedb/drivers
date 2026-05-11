use crate::api::CDataType;
use crate::api::TimestampSubtype;
use crate::api::encoding::{OdbcEncoding, write_string_bytes_i32};
use crate::api::error::{
    ArrowArrayStreamReaderCreationSnafu, CursorAlreadyOpenSnafu, DaeRequiredSnafu,
    DisconnectedSnafu, InvalidBufferLengthSnafu, InvalidCursorStateSnafu, InvalidDuringDaeSnafu,
    InvalidHandleSnafu, InvalidParameterNumberSnafu, InvalidPrecisionOrScaleSnafu,
    JsonBindingSnafu, NoMoreDataSnafu, NullPointerSnafu, OdbcRuntimeSnafu, OperationCanceledSnafu,
    ReadOnlyAttributeSnafu, Required, StatementNotExecutedSnafu, UnsupportedAttributeSnafu,
};
use crate::api::query_type::{QueryType, ResultKind};
use crate::api::runtime::global;
use crate::api::{
    ApdRecord, ConnectionState, DaeContext, ExecutionOrigin, FreeStmtOption, IpdRecord, OdbcResult,
    ParamDirection, ParamValue, SqlType, StatementInner, StatementState, stmt_from_handle,
};
use crate::conversion::Binding;
use crate::conversion::param_binding::odbc_bindings_to_json;
use arrow::array::RecordBatchReader;
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use odbc_sys as sql;
use sf_core::protobuf::generated::database_driver_v1::{
    ArrowArrayStreamPtr, BinaryDataPtr, ConfigSetting, ConnectionGetParameterRequest,
    ConnectionHandle, ExecuteQueryResponse, QueryBindings, ResultSetResponse,
    StatementExecuteQueryRequest, StatementGetResultSetRequest,
    StatementHandle as StatementHandleProto, StatementPrepareRequest, StatementSetOptionsRequest,
    StatementSetSqlQueryRequest, config_setting, execute_query_response, query_bindings,
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
) -> OdbcResult<()> {
    let query = E::read_string(statement_text, text_length)?;
    exec_direct_impl(statement_handle, &query)
}

fn exec_direct_impl(statement_handle: sql::Handle, statement_text: &str) -> OdbcResult<()> {
    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let mut conn = dbc.connection.lock();
    let mut inner = guard.inner.lock();
    tracing::debug!("exec_direct: statement_handle={:?}", statement_handle);

    // Validate connection before committing to NeedData state.
    let conn_handle = match &conn.state {
        ConnectionState::Connected { conn_handle, .. } => *conn_handle,
        ConnectionState::Disconnected => {
            tracing::error!("exec_direct: connection is disconnected");
            return DisconnectedSnafu.fail();
        }
    };

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

    let (bindings, _json_owner) = apply_parameter_bindings(&inner.apd, &inner.ipd, false, None)?;
    let stmt_handle = guard.stmt_handle;

    let token = CancellationToken::new();
    *guard.active_cancel.lock() = Some(token.clone());

    let response = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
        tokio::select! {
            biased;
            _ = token.cancelled() => Err(OperationCanceledSnafu.build()),
            result = async {
                c.statement_set_sql_query(StatementSetSqlQueryRequest {
                    stmt_handle: Some(stmt_handle),
                    query: statement_text.to_string(),
                })
                .await?;

                c.statement_execute_query(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt_handle),
                    bindings,
                })
                .await
            } => result.map_err(Into::into),
        }
    });

    *guard.active_cancel.lock() = None;

    tracing::info!("exec_direct: response={:?}", response);
    let response = response?;

    update_numeric_settings(
        &conn_handle,
        &mut conn.numeric_settings,
        Some(statement_text),
    )?;
    apply_execute_response(&mut inner, stmt_handle, response, ExecutionOrigin::Direct)?;
    Ok(())
}

use crate::conversion::NumericSettings;

/// Refresh the cached connection-level numeric settings after an execute.
///
/// `last_sql` is the SQL text that was just executed, when available
/// (it always is for `SQLExecDirect`; it's `None` for prepared
/// `SQLExecute` since the prepared statement's text isn't kept on the
/// client). It is consulted only by [`tz_format_needs_refresh`] to
/// decide whether to issue the `TIMESTAMP_TZ_OUTPUT_FORMAT` RPC for
/// this call -- see that function and PR #1068 follow-up for the
/// rationale.
fn update_numeric_settings(
    conn_handle: &ConnectionHandle,
    settings: &mut NumericSettings,
    last_sql: Option<&str>,
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

        // TIMESTAMP_TZ_OUTPUT_FORMAT: lazy + invalidate-on-DDL.
        //
        // Snowflake session parameters can only be mutated by SQL run on
        // the same connection (`ALTER SESSION SET/UNSET ...`), so we
        // don't have to re-read the server cache on every execute. We
        // refresh only when:
        //   - the cache has never been populated (first execute), OR
        //   - the SQL we just ran could have mutated it (`ALTER SESSION`
        //     prefix, see `tz_format_needs_refresh`).
        //
        // Skipping this RPC removes the per-execute regression measured
        // in `select_timestamp_tz_1M_arrow_recorded_http` (#1068) for
        // every workload that doesn't issue ALTER SESSION mid-stream.
        //
        // Update semantics for an actual refresh differ from the other
        // settings in this function: those have meaningful server-side
        // defaults so resetting to default on a transient RPC failure is
        // harmless. `tz_offset_format` does NOT -- the customer set it
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
        if tz_format_needs_refresh(settings.tz_format_loaded, last_sql) {
            let rpc_result = c
                .connection_get_parameter(ConnectionGetParameterRequest {
                    conn_handle: Some(*conn_handle),
                    key: "TIMESTAMP_TZ_OUTPUT_FORMAT".to_string(),
                })
                .await
                .map(|resp| resp.value)
                .map_err(|e| format!("{e:?}"));
            apply_tz_offset_format_update(&mut settings.tz_offset_format, rpc_result);
            settings.tz_format_loaded = true;
        }
    });
    Ok(())
}

/// Decide whether to re-read `TIMESTAMP_TZ_OUTPUT_FORMAT` from the
/// server after the just-executed statement.
///
/// Pure function so the truth table can be unit-tested without an RPC
/// mock. Returns `true` when:
/// - the cache has never been populated (`!loaded`), OR
/// - the SQL we just ran starts with `ALTER SESSION` (case-insensitive,
///   leading whitespace and SQL comments stripped).
///
/// `last_sql == None` is the prepared `SQLExecute` path -- a prepared
/// statement that holds an `ALTER SESSION` is pathological enough that
/// we trade exactness for the perf win.
pub(crate) fn tz_format_needs_refresh(loaded: bool, last_sql: Option<&str>) -> bool {
    if !loaded {
        return true;
    }
    let Some(sql) = last_sql else {
        return false;
    };
    sql_starts_with_alter_session(sql)
}

/// `true` if `sql`, after stripping leading whitespace and `--` /
/// `/* ... */` comments, begins with `ALTER SESSION` (case-insensitive).
///
/// Conservative on purpose: we'd rather over-refresh than miss a real
/// `ALTER SESSION SET TIMESTAMP_TZ_OUTPUT_FORMAT = ...`. Anything else
/// (including `ALTER USER`, `USE ROLE`, plain `SELECT`, etc.) skips the
/// refresh -- those statements cannot change session parameters.
fn sql_starts_with_alter_session(sql: &str) -> bool {
    let trimmed = strip_leading_whitespace_and_comments(sql);
    let mut head = trimmed.chars();
    let alter = "ALTER";
    let session = "SESSION";
    if !consume_keyword(&mut head, alter) {
        return false;
    }
    if !consume_at_least_one_space(&mut head) {
        return false;
    }
    consume_keyword(&mut head, session) && head.next().is_none_or(|c| c.is_whitespace() || c == '(')
}

fn strip_leading_whitespace_and_comments(mut s: &str) -> &str {
    loop {
        let before = s;
        s = s.trim_start();
        if let Some(rest) = s.strip_prefix("--") {
            s = match rest.find('\n') {
                Some(idx) => &rest[idx + 1..],
                None => "",
            };
        } else if let Some(rest) = s.strip_prefix("/*") {
            s = match rest.find("*/") {
                Some(idx) => &rest[idx + 2..],
                None => "",
            };
        }
        if s.len() == before.len() {
            return s;
        }
    }
}

fn consume_keyword(iter: &mut std::str::Chars<'_>, keyword: &str) -> bool {
    for kc in keyword.chars() {
        match iter.next() {
            Some(c) if c.eq_ignore_ascii_case(&kc) => continue,
            _ => return false,
        }
    }
    true
}

fn consume_at_least_one_space(iter: &mut std::str::Chars<'_>) -> bool {
    let mut consumed = false;
    let peek = iter.clone();
    for c in peek {
        if c.is_whitespace() {
            consumed = true;
            iter.next();
        } else {
            break;
        }
    }
    consumed
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

#[cfg(test)]
mod tz_format_needs_refresh_tests {
    use super::tz_format_needs_refresh;

    #[test]
    fn first_load_always_refreshes() {
        // `loaded == false` -> refresh regardless of SQL (or absence).
        assert!(tz_format_needs_refresh(false, None));
        assert!(tz_format_needs_refresh(false, Some("SELECT 1")));
    }

    #[test]
    fn loaded_with_no_sql_skips_refresh() {
        // Prepared `SQLExecute` path -- we never refresh just from a
        // bare execute. A prepared `ALTER SESSION` is the documented
        // edge case (see call site comment in `execute`).
        assert!(!tz_format_needs_refresh(true, None));
    }

    #[test]
    fn loaded_with_select_skips_refresh() {
        for sql in [
            "SELECT 1",
            "SELECT * FROM tbl WHERE TS::TIMESTAMP_TZ > CURRENT_TIMESTAMP()",
            "  \t\n SELECT 1",
            "/* hi */ SELECT 1",
            "-- comment\nSELECT 1",
            "INSERT INTO t VALUES (1)",
            "USE ROLE analyst",
            "USE WAREHOUSE wh",
            "ALTER USER me SET RSA_PUBLIC_KEY = '...'",
            "ALTER WAREHOUSE wh SUSPEND",
            "SET myvar = 1",
        ] {
            assert!(
                !tz_format_needs_refresh(true, Some(sql)),
                "should NOT refresh for: {sql:?}"
            );
        }
    }

    #[test]
    fn loaded_with_alter_session_refreshes() {
        for sql in [
            "ALTER SESSION SET TIMESTAMP_TZ_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS TZH:TZM'",
            "alter session set timestamp_tz_output_format = 'YYYY-MM-DD'",
            "ALTER SESSION UNSET TIMESTAMP_TZ_OUTPUT_FORMAT",
            "AlTeR  SeSsIoN  SET FOO = 1",
            "  \t\nALTER SESSION SET FOO = 1",
            "/* warm-up */ ALTER SESSION SET FOO = 1",
            "-- toggle\nALTER SESSION SET FOO = 1",
            "/* a */ -- b\n /* c */ ALTER SESSION SET FOO = 1",
            "ALTER\tSESSION\tSET FOO = 1",
        ] {
            assert!(
                tz_format_needs_refresh(true, Some(sql)),
                "should refresh for: {sql:?}"
            );
        }
    }

    #[test]
    fn alter_session_substring_does_not_refresh() {
        // We anchor at the start of the (stripped) statement text.
        // A SELECT that mentions ALTER SESSION in a literal must not
        // trigger the refresh -- otherwise an app fetching audit logs
        // pays the per-execute cost on every row.
        assert!(!tz_format_needs_refresh(
            true,
            Some("SELECT 'ALTER SESSION SET ...' AS sql")
        ));
        assert!(!tz_format_needs_refresh(true, Some("ALTERED")));
        assert!(!tz_format_needs_refresh(
            true,
            Some("ALTER USER me UNSET FOO")
        ));
        assert!(!tz_format_needs_refresh(true, Some("ALTER")));
        assert!(!tz_format_needs_refresh(
            true,
            Some("ALTERSESSION SET FOO=1")
        ));
    }

    #[test]
    fn empty_or_whitespace_sql_skips_refresh() {
        assert!(!tz_format_needs_refresh(true, Some("")));
        assert!(!tz_format_needs_refresh(true, Some("   \n\t  ")));
        assert!(!tz_format_needs_refresh(true, Some("-- comment only\n")));
        assert!(!tz_format_needs_refresh(true, Some("/* comment only */")));
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
pub fn execute(statement_handle: sql::Handle) -> OdbcResult<()> {
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
    let (bindings, _json_owner) = apply_parameter_bindings(
        &inner.apd,
        &inner.ipd,
        is_prepared,
        inner.prepared_param_count,
    )?;

    let stmt_handle = guard.stmt_handle;

    let token = CancellationToken::new();
    *guard.active_cancel.lock() = Some(token.clone());

    let response = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
        tokio::select! {
            biased;
            _ = token.cancelled() => Err(OperationCanceledSnafu.build()),
            result = async {
                c.statement_execute_query(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt_handle),
                    bindings,
                })
                .await
            } => result.map_err(Into::into),
        }
    });

    *guard.active_cancel.lock() = None;
    let response = response?;

    tracing::info!("execute: Successfully executed statement");
    let mut settings = dbc.connection.lock().numeric_settings;
    // Prepared statement: text isn't kept on the client. We pass `None`
    // and rely on the first-execute path to populate the
    // `tz_offset_format` cache; subsequent prepared executes skip the
    // RPC. An `ALTER SESSION` issued via prepare/execute (very rare) is
    // the documented edge case where the new format won't take effect
    // until the next `SQLExecDirect` runs.
    update_numeric_settings(&conn_handle, &mut settings, None)?;
    dbc.connection.lock().numeric_settings = settings;
    apply_execute_response(&mut inner, stmt_handle, response, origin)?;
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
/// For Single results: fetches the Arrow stream via `StatementGetResultSet`, then
/// creates the appropriate state (DDL/DML/Query).
/// For Multi results: stores child query IDs, fetches the first child result set,
/// and sets up state for `SQLMoreResults` iteration.
fn apply_execute_response(
    stmt: &mut StatementInner,
    stmt_handle: sf_core::protobuf::generated::database_driver_v1::StatementHandle,
    response: ExecuteQueryResponse,
    origin: ExecutionOrigin,
) -> OdbcResult<()> {
    let result = response.result.required("Execute result is required")?;

    // Clear previous multi-statement state.
    stmt.multi_query_ids.clear();
    stmt.multi_current_idx = 0;

    match result {
        execute_query_response::Result::Single(descriptor) => {
            let query_id = descriptor.query_id.clone();
            let rs = fetch_result_set(stmt_handle, &query_id)?;
            let execute_state = create_execute_state_from_result_set(
                rs,
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
            let rs = fetch_result_set(stmt_handle, first_id)?;
            let statement_type_id = rs
                .result_descriptor
                .as_ref()
                .and_then(|d| d.statement_type_id);
            let rows_affected = rs.result_descriptor.as_ref().and_then(|d| d.rows_affected);
            let execute_state =
                create_execute_state_from_result_set(rs, statement_type_id, rows_affected, origin)?;
            stmt.multi_current_idx = 1;
            set_state(stmt, execute_state);
            Ok(())
        }
    }
}

/// Fetch a result set (descriptor + Arrow stream) for a given query ID.
fn fetch_result_set(
    stmt_handle: StatementHandleProto,
    query_id: &str,
) -> OdbcResult<ResultSetResponse> {
    let response = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
        c.statement_get_result_set(StatementGetResultSetRequest {
            stmt_handle: Some(stmt_handle),
            query_id: query_id.to_string(),
        })
        .await
    })?;
    Ok(response)
}

fn create_execute_state_from_result_set(
    rs: ResultSetResponse,
    statement_type_id: Option<i64>,
    rows_affected: Option<i64>,
    origin: ExecutionOrigin,
) -> OdbcResult<StatementState> {
    let stream = rs.stream.required("Stream is required")?;
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

/// Build JSON query bindings from ODBC parameter bindings.
///
/// When `prepared` is true (SQLPrepare+SQLExecute flow), the IPD has server-
/// provided parameter count and we validate that the APD covers every marker.
/// When `prepared` is false (SQLExecDirect), the IPD only has records from
/// SQLBindParameter — we send whatever the APD has and let the server validate.
///
/// `prepared_param_count` caps how many parameters are serialized for prepared
/// statements, preventing phantom bindings beyond the server-reported marker
/// count from being dereferenced.
fn apply_parameter_bindings(
    apd: &crate::api::ApdDescriptor,
    ipd: &crate::api::IpdDescriptor,
    prepared: bool,
    prepared_param_count: Option<u16>,
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
        "apply_parameter_bindings: Found {} bound parameters (effective_count={})",
        apd.records.len(),
        effective_count,
    );

    let json_string =
        odbc_bindings_to_json(apd, ipd, effective_count).context(JsonBindingSnafu {})?;

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
        StmtAttr::MetadataId => {
            let val = value_ptr as sql::ULen;
            inner.metadata_id = val != 0;
            Ok(())
        }
        StmtAttr::SnowflakeLastQueryId | StmtAttr::ImpRowDesc | StmtAttr::ImpParamDesc => {
            tracing::warn!("set_stmt_attr: {:?} is read-only", attr);
            ReadOnlyAttributeSnafu { attribute }.fail()
        }
        StmtAttr::MultiStatementCount => {
            let count = value_ptr as i64;
            tracing::debug!("set_stmt_attr: MultiStatementCount = {}", count);
            let stmt_handle = guard.stmt_handle;
            let mut options = std::collections::HashMap::new();
            options.insert(
                "multi_statement_count".to_string(),
                ConfigSetting {
                    value: Some(config_setting::Value::IntValue(count)),
                },
            );
            global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
                c.statement_set_options(StatementSetOptionsRequest {
                    stmt_handle: Some(stmt_handle),
                    options,
                })
                .await
            })?;
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
        StmtAttr::MetadataId => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::ULen) = inner.metadata_id as sql::ULen;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = std::mem::size_of::<sql::ULen>() as sql::Integer;
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
        StmtAttr::MultiStatementCount => {
            tracing::warn!("get_stmt_attr: MultiStatementCount is write-only");
            crate::api::error::UnsupportedAttributeSnafu { attribute }.fail()
        }
        _ => {
            tracing::warn!("get_stmt_attr: unsupported attribute {:?}", attr);
            crate::api::error::UnknownAttributeSnafu { attribute }.fail()
        }
    }
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

    let stmt_handle = guard.stmt_handle;
    let rs = fetch_result_set(stmt_handle, &query_id)?;
    let statement_type_id = rs
        .result_descriptor
        .as_ref()
        .and_then(|d| d.statement_type_id);
    let rows_affected = rs.result_descriptor.as_ref().and_then(|d| d.rows_affected);
    let execute_state =
        create_execute_state_from_result_set(rs, statement_type_id, rows_affected, origin)?;
    set_state(&mut inner, execute_state);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ApdDescriptor, IpdDescriptor, SqlState};

    #[test]
    fn apply_bindings_prepared_without_param_count_errors() {
        let apd = ApdDescriptor::new();
        let ipd = IpdDescriptor::new();
        let result = apply_parameter_bindings(&apd, &ipd, true, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.to_sql_state(), SqlState::CountFieldIncorrect);
    }
}
