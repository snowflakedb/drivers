use crate::api::CDataType;
use crate::api::TimestampSubtype;
use crate::api::encoding::OdbcEncoding;
use crate::api::error::{
    ArrowArrayStreamReaderCreationSnafu, ArrowBatchConcatSnafu, ArrowBatchReadSnafu,
    AttributeCannotBeSetNowSnafu, ConcatNullValueSnafu, CsvBindingSnafu, CursorAlreadyOpenSnafu,
    DaeRequiredSnafu, DisconnectedSnafu, InternalSnafu, InvalidAttributeValueSnafu,
    InvalidBufferLengthSnafu, InvalidCursorStateSnafu, InvalidDuringDaeSnafu, InvalidHandleSnafu,
    InvalidParameterNumberSnafu, InvalidPrecisionOrScaleSnafu, InvalidUseOfImplicitDescriptorSnafu,
    JsonBindingSnafu, NoMoreDataSnafu, NonCharBinarySentInPiecesSnafu, NullPointerSnafu,
    OdbcRuntimeSnafu, OperationCanceledSnafu, ReadOnlyAttributeSnafu, Required,
    StatementNotExecutedSnafu, StillExecutingSnafu, UnsupportedAttributeSnafu,
    UnsupportedFeatureSnafu,
};
use crate::api::handle_registry::HandleId;
use crate::api::query_type::{QueryType, ResultKind};
use crate::api::runtime::global;
use crate::api::{
    ApdRecord, Connection, ConnectionState, DaeContext, ExecutionOrigin, ExplicitDesc,
    FreeStmtOption, IpdRecord, OdbcResult, ParamDirection, ParamValue, SQL_CONCUR_LOCK,
    SQL_CONCUR_READ_ONLY, SQL_CONCUR_VALUES, SQL_INSENSITIVE, SQL_NONSCROLLABLE, SQL_NOSCAN_OFF,
    SQL_NOSCAN_ON, SQL_RD_OFF, SQL_RD_ON, SQL_SCROLLABLE, SQL_SENSITIVE, SQL_UNSPECIFIED, SqlType,
    StatementInner, StatementState, stmt_from_handle,
};
use crate::conversion::Binding;
use crate::conversion::param_binding::{odbc_bindings_to_csv, odbc_bindings_to_json};
use arrow::array::RecordBatch;
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
use snafu::{OptionExt, ResultExt};
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
    use crate::api::ExecDirectOutcome;

    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let mut conn = dbc.connection.lock();
    let mut inner = guard.inner.lock();

    let outcome = if let StatementState::AsyncExecDirect { .. } = inner.state.as_ref() {
        // === ASYNC POLL PATH ===
        let state = inner.state.take();
        let StatementState::AsyncExecDirect { join_handle } = state else {
            unreachable!()
        };
        if !join_handle.is_finished() {
            inner
                .state
                .set(StatementState::AsyncExecDirect { join_handle });
            return StillExecutingSnafu.fail();
        }
        match complete_async_poll(&guard.cancel_token, &mut inner, join_handle) {
            Ok(outcome) => outcome,
            Err(e) => {
                tracing::error!("exec_direct: async poll failed: {e}");
                if let Some(qid) = e.query_id() {
                    inner.last_query_id = Some(qid.to_owned());
                }
                return Err(e);
            }
        }
    } else {
        // === SYNC / SPAWN PATH ===
        tracing::debug!("exec_direct: statement_handle={:?}", statement_handle);

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
        inner.prepared_array_bind_supported = None;

        let dae_params = inner.with_effective_apd(|apd| find_dae_params(apd, None));
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

        let (param_count, param_array_size) = inner.with_effective_apd(|apd| {
            (
                effective_param_count(apd, &inner.ipd, false, None),
                apd.array_size,
            )
        });
        let effective_cells = param_array_size as u64 * u64::from(param_count);
        let binding_mode = select_binding_mode(
            &conn_handle,
            effective_cells,
            // exec-direct: no prepare describe ran — None means "unknown",
            // which falls through to the threshold check.
            None,
        )?;
        let (bindings, bindings_owner) = inner.with_effective_apd(|apd| {
            apply_parameter_bindings(apd, &inner.ipd, false, None, binding_mode)
        })?;
        let stmt_handle = guard.stmt_handle;
        let query_timeout = inner.query_timeout;
        let effective_query = statement_text.to_string();
        let multi_statement_count = inner.multi_statement_count;
        let async_enabled = inner.async_enabled;

        match run_cancellable(&guard, async_enabled, |client| async move {
            let _bindings_owner = bindings_owner;
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
                client
                    .statement_set_options(StatementSetOptionsRequest {
                        stmt_handle: Some(stmt_handle),
                        options,
                    })
                    .await?;
            }

            client
                .statement_set_sql_query(StatementSetSqlQueryRequest {
                    stmt_handle: Some(stmt_handle),
                    query: effective_query,
                })
                .await?;

            let response = client
                .statement_execute_query(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt_handle),
                    bindings,
                    timeout_seconds: if query_timeout > 0 {
                        Some(query_timeout.min(u32::MAX as sql::ULen) as u32)
                    } else {
                        None
                    },
                })
                .await?;
            Ok(ExecDirectOutcome {
                response,
                conn_handle,
            })
        }) {
            Ok(Execution::Completed(outcome)) => outcome,
            Ok(Execution::Spawned(join_handle)) => {
                inner
                    .state
                    .set(StatementState::AsyncExecDirect { join_handle });
                return StillExecutingSnafu.fail();
            }
            Err(e) => {
                tracing::error!("exec_direct: execution failed: {e}");
                if let Some(qid) = e.query_id() {
                    inner.last_query_id = Some(qid.to_owned());
                }
                return Err(e);
            }
        }
    };

    // === POST-PROCESSING (shared by poll and sync paths) ===
    finalize_execute_response(
        &mut conn,
        &mut inner,
        outcome.conn_handle,
        outcome.response,
        ExecutionOrigin::Direct,
        Some(statement_text),
    )?;
    Ok(())
}

use crate::conversion::NumericSettings;

/// Common finalization after a successful execution (ExecDirect, Execute).
/// Refreshes connection numeric settings from the server, applies the
/// execution response to statement state, resets the row counter, and writes
/// the parameter-array output fields (`PARAMS_PROCESSED_PTR` / `PARAM_STATUS_PTR`).
fn finalize_execute_response(
    conn: &mut crate::api::Connection,
    inner: &mut StatementInner,
    conn_handle: ConnectionHandle,
    response: ExecuteQueryResponse,
    origin: ExecutionOrigin,
    last_sql: Option<&str>,
) -> OdbcResult<()> {
    update_numeric_settings(&conn_handle, &mut conn.numeric_settings, last_sql)?;

    // Snapshot output pointers before passing `inner` into apply_execute_response.
    let param_set_size = inner.with_effective_apd(|apd| apd.array_size);
    let rows_processed_ptr = inner.ipd.rows_processed_ptr;
    let param_status_ptr = inner.ipd.array_status_ptr;

    let result = apply_execute_response(inner, conn_handle, response, origin);
    inner.rows_returned = 0;

    // Write PARAMS_PROCESSED and PARAM_STATUS regardless of result so the
    // application always gets feedback for the rows that were sent.
    unsafe {
        if !rows_processed_ptr.is_null() {
            *rows_processed_ptr = param_set_size as sql::ULen;
        }
        if !param_status_ptr.is_null() {
            for i in 0..param_set_size {
                *param_status_ptr.add(i) = 0u16; // SQL_PARAM_SUCCESS
            }
        }
    }

    result
}

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
        // harmless. The TZ offset format does NOT -- the customer set it
        // deliberately via `ALTER SESSION` and a transient blip silently
        // flipping the next fetch from `+HH:MM` rendering back to bare
        // UTC is a wire-format regression with no diagnostic the
        // application can correlate. So `apply_tz_offset_format_update`:
        //   - On `Ok(resp)` with a non-empty value -> `Loaded(token)`
        //     (parse_tz_offset_format collapses unrecognised values to
        //     None, which is the spec-correct fall-through to bare UTC).
        //   - On `Ok(resp)` with `None` or empty value -> the user
        //     explicitly UNSET the parameter, so `Loaded(None)`.
        //   - On `Err(_)` -> leave the cache state untouched and warn. A
        //     failure on the *first* execute therefore leaves the cache
        //     `Unloaded`, so the next execute retries instead of locking
        //     in a wrong bare-UTC value. A failure after a successful
        //     load keeps the customer's `Loaded(_)` value.
        // See PR #1068 review on `statement.rs:209`.
        if tz_format_needs_refresh(settings.tz_offset_format_cache, last_sql) {
            let rpc_result = c
                .connection_get_parameter(ConnectionGetParameterRequest {
                    conn_handle: Some(*conn_handle),
                    key: "TIMESTAMP_TZ_OUTPUT_FORMAT".to_string(),
                })
                .await
                .map(|resp| resp.value)
                .map_err(|e| format!("{e:?}"));
            apply_tz_offset_format_update(&mut settings.tz_offset_format_cache, rpc_result);
        }
    });
    Ok(())
}

/// Decide whether to re-read `TIMESTAMP_TZ_OUTPUT_FORMAT` from the
/// server after the just-executed statement.
///
/// Pure function so the truth table can be unit-tested without an RPC
/// mock. Returns `true` when:
/// - the cache has never been populated (`Unloaded`), OR
/// - the SQL we just ran starts with `ALTER SESSION` (case-insensitive,
///   leading whitespace and SQL comments stripped).
///
/// `last_sql == None` is the prepared `SQLExecute` path -- a prepared
/// statement that holds an `ALTER SESSION` is pathological enough that
/// we trade exactness for the perf win.
///
/// Known blind spots (all accepted -- they only ever cause a *stale*
/// cache, never a crash, and none are reachable by the documented
/// `ALTER SESSION SET TIMESTAMP_TZ_OUTPUT_FORMAT = ...` flow):
/// - Multi-statement submissions (`SELECT 1; ALTER SESSION SET ...`):
///   only the leading keyword is inspected, so a trailing `ALTER
///   SESSION` in the same batch is missed.
/// - A stored procedure / `EXECUTE IMMEDIATE` body that runs `ALTER
///   SESSION` internally: the outer `CALL` / `EXECUTE IMMEDIATE` text
///   doesn't start with `ALTER SESSION`, so the mutation is invisible
///   here.
///
/// In both cases the customer can still force a refresh by issuing a
/// top-level `ALTER SESSION` (or opening a new connection); we
/// deliberately don't pay the per-execute RPC to cover them.
pub(crate) fn tz_format_needs_refresh(
    cache: crate::conversion::TzOffsetFormatCache,
    last_sql: Option<&str>,
) -> bool {
    if matches!(cache, crate::conversion::TzOffsetFormatCache::Unloaded) {
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
///
/// Whitespace *and* comments are tolerated both before `ALTER` and
/// between the `ALTER` and `SESSION` keywords, so
/// `ALTER /* x */ SESSION ...` and `ALTER SESSION/* x */ SET ...` are
/// both detected.
fn sql_starts_with_alter_session(sql: &str) -> bool {
    let head = strip_leading_whitespace_and_comments(sql);
    let Some(after_alter) = strip_keyword(head, "ALTER") else {
        return false;
    };
    // `ALTER` and `SESSION` must be separated by at least one whitespace
    // char or a comment, otherwise we'd match `ALTERSESSION`. The strip
    // must consume something.
    let after_sep = strip_leading_whitespace_and_comments(after_alter);
    if after_sep.len() == after_alter.len() {
        return false;
    }
    let Some(after_session) = strip_keyword(after_sep, "SESSION") else {
        return false;
    };
    // `SESSION` must end at a token boundary: end of statement, or
    // followed by whitespace or a comment. (`ALTER SESSION` is never
    // followed by `(` in the grammar, so there's no paren case to
    // special-case.)
    after_session.is_empty()
        || after_session.starts_with(|c: char| c.is_whitespace())
        || after_session.starts_with("--")
        || after_session.starts_with("/*")
}

/// Case-insensitively strip a leading ASCII `keyword` from `s`, returning
/// the remainder, or `None` if `s` doesn't start with it. `keyword` is
/// assumed ASCII, so its byte length is also its char length.
fn strip_keyword<'a>(s: &'a str, keyword: &str) -> Option<&'a str> {
    let prefix = s.get(..keyword.len())?;
    prefix
        .eq_ignore_ascii_case(keyword)
        .then(|| &s[keyword.len()..])
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

/// Cache-update decision logic for `TIMESTAMP_TZ_OUTPUT_FORMAT`. Pure
/// function so the four-way state table (Ok+set / Ok+empty / Ok+None /
/// Err) can be unit-tested without standing up an RPC mock.
///
/// Semantics (see PR #1068 review on `statement.rs:209`):
/// - `Ok(Some(non_empty))` -> `Loaded(token)` with the parsed token
///   (which may itself be `None` if the format string carries no
///   recognised TZ token, the spec-correct fall-through to bare UTC).
/// - `Ok(Some(""))` / `Ok(None)` -> the user explicitly UNSET the
///   parameter, so `Loaded(None)` (bare UTC).
/// - `Err(_)` -> a transient RPC blip; leave the cache *state* untouched
///   and warn. Because the `Unloaded` -> `Loaded` transition only ever
///   happens in the `Ok(_)` arm, a failure on the first execute leaves
///   the cache `Unloaded` so the next execute retries, and a failure
///   after a successful load keeps the customer-configured wire format.
pub(crate) fn apply_tz_offset_format_update(
    cache: &mut crate::conversion::TzOffsetFormatCache,
    rpc_result: Result<Option<String>, String>,
) {
    use crate::conversion::TzOffsetFormatCache;
    match rpc_result {
        Ok(value) => {
            let new_format = match value.as_deref() {
                Some(v) if !v.is_empty() => crate::conversion::timestamp::parse_tz_offset_format(v),
                _ => None,
            };
            let changed = !matches!(*cache, TzOffsetFormatCache::Loaded(f) if f == new_format);
            if changed {
                tracing::info!(
                    "Server parameter TIMESTAMP_TZ_OUTPUT_FORMAT offset token = {new_format:?}"
                );
            }
            *cache = TzOffsetFormatCache::Loaded(new_format);
        }
        Err(err) => {
            tracing::warn!(
                error = %err,
                "failed to refresh TIMESTAMP_TZ_OUTPUT_FORMAT; keeping cached state {:?}",
                cache
            );
        }
    }
}

#[cfg(test)]
mod apply_tz_offset_format_update_tests {
    use super::apply_tz_offset_format_update;
    use crate::conversion::TzOffsetFormatCache;
    use crate::conversion::timestamp::TzOffsetFormat;

    #[test]
    fn ok_with_recognised_format_loads_token() {
        let mut cache = TzOffsetFormatCache::Unloaded;
        apply_tz_offset_format_update(
            &mut cache,
            Ok(Some("YYYY-MM-DD HH24:MI:SS.FF TZH:TZM".to_string())),
        );
        assert_eq!(
            cache,
            TzOffsetFormatCache::Loaded(Some(TzOffsetFormat::Colon))
        );
    }

    #[test]
    fn ok_with_unrecognised_format_loads_none() {
        // A non-empty format string with no recognised TZ token is the
        // spec-correct fall-through to bare UTC -- the user is asking
        // for a custom format the driver doesn't render an offset for,
        // so we mustn't keep an old offset rendering active.
        let mut cache = TzOffsetFormatCache::Loaded(Some(TzOffsetFormat::Colon));
        apply_tz_offset_format_update(&mut cache, Ok(Some("YYYY-MM-DD HH24:MI:SS".to_string())));
        assert_eq!(cache, TzOffsetFormatCache::Loaded(None));
    }

    #[test]
    fn ok_with_empty_string_loads_none() {
        // Server returns an explicit empty string for an unset parameter
        // on some configurations; treat it as UNSET and revert to bare
        // UTC.
        let mut cache = TzOffsetFormatCache::Loaded(Some(TzOffsetFormat::NoColon));
        apply_tz_offset_format_update(&mut cache, Ok(Some(String::new())));
        assert_eq!(cache, TzOffsetFormatCache::Loaded(None));
    }

    #[test]
    fn ok_with_none_loads_none() {
        let mut cache = TzOffsetFormatCache::Loaded(Some(TzOffsetFormat::HourOnly));
        apply_tz_offset_format_update(&mut cache, Ok(None));
        assert_eq!(cache, TzOffsetFormatCache::Loaded(None));
    }

    /// rkowalski #1 (the load-bearing regression): a transient RPC
    /// failure on the *first* execute must leave the cache `Unloaded`,
    /// NOT flip it to `Loaded(None)`. Pre-fix, `tz_format_loaded` was
    /// set unconditionally, locking the connection into bare-UTC
    /// rendering until an `ALTER SESSION` happened by. Leaving it
    /// `Unloaded` makes the next execute retry the load. See PR #1068
    /// review on `statement.rs:209`.
    #[test]
    fn err_on_first_execute_stays_unloaded() {
        let mut cache = TzOffsetFormatCache::Unloaded;
        apply_tz_offset_format_update(&mut cache, Err("transient transport error".to_string()));
        assert_eq!(cache, TzOffsetFormatCache::Unloaded);
    }

    /// A transient failure *after* a successful load must NOT silently
    /// flip a customer-configured `+HH:MM` rendering back to bare UTC --
    /// the loaded value is kept.
    #[test]
    fn err_keeps_existing_loaded_value() {
        let mut cache = TzOffsetFormatCache::Loaded(Some(TzOffsetFormat::Colon));
        apply_tz_offset_format_update(&mut cache, Err("transient transport error".to_string()));
        assert_eq!(
            cache,
            TzOffsetFormatCache::Loaded(Some(TzOffsetFormat::Colon))
        );
    }

    /// Symmetric: an `Err` against an already-`Loaded(None)` cache must
    /// remain `Loaded(None)` (we don't accidentally synthesise a value
    /// or regress to `Unloaded`).
    #[test]
    fn err_keeps_existing_loaded_none() {
        let mut cache = TzOffsetFormatCache::Loaded(None);
        apply_tz_offset_format_update(&mut cache, Err("transient transport error".to_string()));
        assert_eq!(cache, TzOffsetFormatCache::Loaded(None));
    }
}

#[cfg(test)]
mod tz_format_needs_refresh_tests {
    use super::tz_format_needs_refresh;
    use crate::conversion::TzOffsetFormatCache;

    const UNLOADED: TzOffsetFormatCache = TzOffsetFormatCache::Unloaded;
    const LOADED: TzOffsetFormatCache = TzOffsetFormatCache::Loaded(None);

    #[test]
    fn first_load_always_refreshes() {
        // `Unloaded` -> refresh regardless of SQL (or absence).
        assert!(tz_format_needs_refresh(UNLOADED, None));
        assert!(tz_format_needs_refresh(UNLOADED, Some("SELECT 1")));
    }

    #[test]
    fn loaded_with_no_sql_skips_refresh() {
        // Prepared `SQLExecute` path -- we never refresh just from a
        // bare execute. A prepared `ALTER SESSION` is the documented
        // edge case (see call site comment in `execute`).
        assert!(!tz_format_needs_refresh(LOADED, None));
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
                !tz_format_needs_refresh(LOADED, Some(sql)),
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
                tz_format_needs_refresh(LOADED, Some(sql)),
                "should refresh for: {sql:?}"
            );
        }
    }

    #[test]
    fn loaded_with_inter_keyword_comment_refreshes() {
        // rkowalski #3: comments *between* the ALTER and SESSION keywords
        // (and immediately after SESSION) must still be detected, not just
        // leading comments.
        for sql in [
            "ALTER /* x */ SESSION SET FOO = 1",
            "ALTER/* x */SESSION SET FOO = 1",
            "ALTER -- inline\n SESSION SET FOO = 1",
            "ALTER SESSION/* x */ SET FOO = 1",
            "ALTER SESSION-- c\nSET FOO = 1",
            "/* a */ ALTER /* b */ SESSION /* c */ SET FOO = 1",
        ] {
            assert!(
                tz_format_needs_refresh(LOADED, Some(sql)),
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
            LOADED,
            Some("SELECT 'ALTER SESSION SET ...' AS sql")
        ));
        assert!(!tz_format_needs_refresh(LOADED, Some("ALTERED")));
        assert!(!tz_format_needs_refresh(
            LOADED,
            Some("ALTER USER me UNSET FOO")
        ));
        assert!(!tz_format_needs_refresh(LOADED, Some("ALTER")));
        assert!(!tz_format_needs_refresh(
            LOADED,
            Some("ALTERSESSION SET FOO=1")
        ));
        // `SESSION` immediately followed by a non-boundary char is not a
        // keyword match.
        assert!(!tz_format_needs_refresh(
            LOADED,
            Some("ALTER SESSIONX SET FOO=1")
        ));
    }

    #[test]
    fn empty_or_whitespace_sql_skips_refresh() {
        assert!(!tz_format_needs_refresh(LOADED, Some("")));
        assert!(!tz_format_needs_refresh(LOADED, Some("   \n\t  ")));
        assert!(!tz_format_needs_refresh(LOADED, Some("-- comment only\n")));
        assert!(!tz_format_needs_refresh(LOADED, Some("/* comment only */")));
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
    use crate::api::PrepareOutcome;

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
    let mut inner = guard.inner.lock();

    let outcome = if let StatementState::AsyncPrepare { .. } = inner.state.as_ref() {
        // === ASYNC POLL PATH ===
        let state = inner.state.take();
        let StatementState::AsyncPrepare { join_handle } = state else {
            unreachable!()
        };
        if !join_handle.is_finished() {
            inner
                .state
                .set(StatementState::AsyncPrepare { join_handle });
            return StillExecutingSnafu.fail();
        }
        complete_async_poll(&guard.cancel_token, &mut inner, join_handle)?
    } else {
        // === SYNC / SPAWN PATH ===
        let _conn_handle = match &conn.state {
            ConnectionState::Connected { conn_handle, .. } => *conn_handle,
            ConnectionState::Disconnected => {
                tracing::error!("prepare: connection is disconnected");
                return DisconnectedSnafu.fail();
            }
        };

        if inner.state.as_ref().is_need_data() {
            return InvalidDuringDaeSnafu.fail();
        }

        if inner.state.as_ref().has_open_cursor() {
            tracing::error!("prepare: cursor is already open");
            return CursorAlreadyOpenSnafu.fail();
        }

        tracing::debug!("prepare: query = {query}");

        let stmt_handle = guard.stmt_handle;
        let async_enabled = inner.async_enabled;

        let query_owned = query.to_string();

        let execution_outcome = run_cancellable(&guard, async_enabled, |client| async move {
            client
                .statement_set_sql_query(StatementSetSqlQueryRequest {
                    stmt_handle: Some(stmt_handle),
                    query: query_owned,
                })
                .await?;

            let prepare_response = client
                .statement_prepare(StatementPrepareRequest {
                    stmt_handle: Some(stmt_handle),
                })
                .await?;

            let result = prepare_response.result.required("Result is required")?;
            let stream_ptr = result.stream.required("Stream is required")?;
            let reader = reader_from_protobuf_stream(stream_ptr)?;
            let schema = reader.schema();

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

            Ok(PrepareOutcome {
                number_of_binds: param_count,
                schema,
                array_bind_supported: result.array_bind_supported,
            })
        })?;
        match execution_outcome {
            Execution::Completed(outcome) => outcome,
            Execution::Spawned(join_handle) => {
                inner
                    .state
                    .set(StatementState::AsyncPrepare { join_handle });
                return StillExecutingSnafu.fail();
            }
        }
    };

    apply_prepare_outcome(&mut inner, &conn, outcome);
    Ok(())
}

/// Fixed VARCHAR size `SQLDescribeParam` reports for an untyped `?` marker,
/// matching the reference driver's `maxVarcharSize`. ODBC `ColumnSize` for a
/// character type is a length **in characters**, so this is 134,217,728
/// characters (128 Mi), not bytes. Intentionally distinct from
/// `SF_DEFAULT_VARCHAR_MAX_LEN` (16,777,216 characters), which is Snowflake's
/// default VARCHAR *column* length used for result-set column sizing.
const PARAM_DESCRIBE_VARCHAR_SIZE_IN_CHARS: u64 = 134_217_728;

fn apply_prepare_outcome(
    inner: &mut StatementInner,
    _conn: &crate::api::Connection,
    outcome: crate::api::PrepareOutcome,
) {
    let crate::api::PrepareOutcome {
        number_of_binds,
        schema,
        array_bind_supported,
    } = outcome;
    inner.ird.desc_count = schema.fields().len() as sql::SmallInt;
    inner.prepared_param_count = Some(number_of_binds);
    inner.prepared_array_bind_supported = Some(array_bind_supported);
    inner.ipd.records.retain(|&k, _| k <= number_of_binds);
    for i in 1..=number_of_binds {
        inner
            .ipd
            .records
            .entry(i)
            .or_insert_with(|| IpdRecord::with_varchar_size(PARAM_DESCRIBE_VARCHAR_SIZE_IN_CHARS));
    }
    tracing::info!("prepare: auto-IPD populated {number_of_binds} parameter markers (from server)");
    inner.state.set(StatementState::Prepared { schema });
}

/// Execute a prepared statement
pub fn execute(statement_handle: sql::Handle) -> OdbcResult<()> {
    use crate::api::ExecuteOutcome;

    tracing::debug!("execute: statement_handle={:?}", statement_handle);
    let guard = stmt_from_handle(statement_handle)?;
    let dbc = guard.conn()?;
    let mut conn = dbc.connection.lock();
    let mut inner = guard.inner.lock();

    let (outcome, origin) = if let StatementState::AsyncExecute { .. } = inner.state.as_ref() {
        // === ASYNC POLL PATH ===
        let state = inner.state.take();
        let StatementState::AsyncExecute {
            join_handle,
            origin,
        } = state
        else {
            unreachable!()
        };
        if !join_handle.is_finished() {
            inner.state.set(StatementState::AsyncExecute {
                join_handle,
                origin,
            });
            return StillExecutingSnafu.fail();
        }
        let outcome = match complete_async_poll(&guard.cancel_token, &mut inner, join_handle) {
            Ok(outcome) => outcome,
            Err(e) => {
                tracing::error!("execute: async poll failed: {e}");
                if let Some(qid) = e.query_id() {
                    inner.last_query_id = Some(qid.to_owned());
                }
                return Err(e);
            }
        };
        (outcome, origin)
    } else {
        // === SYNC / SPAWN PATH ===
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
            StatementState::DdlExecuted { origin, .. }
            | StatementState::DmlExecuted { origin, .. } => origin.clone(),
            _ => ExecutionOrigin::Direct,
        };
        let is_prepared = origin.is_prepared();

        if matches!(conn.state, ConnectionState::Disconnected) {
            tracing::error!("execute: connection is disconnected");
            return DisconnectedSnafu.fail();
        }

        let dae_params =
            inner.with_effective_apd(|apd| find_dae_params(apd, inner.prepared_param_count));
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

        let conn_handle = match &conn.state {
            ConnectionState::Connected { conn_handle, .. } => *conn_handle,
            ConnectionState::Disconnected => {
                tracing::error!("execute: connection is disconnected");
                return DisconnectedSnafu.fail();
            }
        };
        let (param_count, param_array_size) = inner.with_effective_apd(|apd| {
            (
                effective_param_count(apd, &inner.ipd, is_prepared, inner.prepared_param_count),
                apd.array_size,
            )
        });
        let effective_cells = param_array_size as u64 * u64::from(param_count);
        let binding_mode = select_binding_mode(
            &conn_handle,
            effective_cells,
            inner.prepared_array_bind_supported,
        )?;
        let (bindings, bindings_owner) = inner.with_effective_apd(|apd| {
            apply_parameter_bindings(
                apd,
                &inner.ipd,
                is_prepared,
                inner.prepared_param_count,
                binding_mode,
            )
        })?;

        let stmt_handle = guard.stmt_handle;
        let query_timeout = inner.query_timeout;
        let multi_statement_count = inner.multi_statement_count;
        let async_enabled = inner.async_enabled;

        let outcome = match run_cancellable(&guard, async_enabled, |client| async move {
            let _bindings_owner = bindings_owner;
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
                client
                    .statement_set_options(StatementSetOptionsRequest {
                        stmt_handle: Some(stmt_handle),
                        options,
                    })
                    .await?;
            }
            let response = client
                .statement_execute_query(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt_handle),
                    bindings,
                    timeout_seconds: if query_timeout > 0 {
                        Some(query_timeout.min(u32::MAX as sql::ULen) as u32)
                    } else {
                        None
                    },
                })
                .await?;
            Ok(ExecuteOutcome {
                response,
                conn_handle,
            })
        }) {
            Ok(Execution::Completed(outcome)) => outcome,
            Ok(Execution::Spawned(join_handle)) => {
                inner.state.set(StatementState::AsyncExecute {
                    join_handle,
                    origin,
                });
                return StillExecutingSnafu.fail();
            }
            Err(e) => {
                tracing::error!("execute: execution failed: {e}");
                if let Some(qid) = e.query_id() {
                    inner.last_query_id = Some(qid.to_owned());
                }
                return Err(e);
            }
        };
        (outcome, origin)
    };

    // === POST-PROCESSING (shared by poll and sync paths) ===
    tracing::info!("execute: Successfully executed statement");
    // Prepared statement: text isn't kept on the client. We pass `None`
    // and rely on the first-execute path to populate the
    // `tz_offset_format` cache; subsequent prepared executes skip the
    // RPC. An `ALTER SESSION` issued via prepare/execute (very rare) is
    // the documented edge case where the new format won't take effect
    // until the next `SQLExecDirect` runs.
    finalize_execute_response(
        &mut conn,
        &mut inner,
        outcome.conn_handle,
        outcome.response,
        origin,
        None,
    )?;
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

/// Set statement state for catalog functions (SQLTables, etc.).
/// Does NOT call `finalize_execute_response` — catalog results don't need
/// parameter metadata refresh or multi-statement tracking.
pub(crate) fn set_state_for_catalog(inner: &mut StatementInner, state: StatementState) {
    set_state(inner, state);
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

pub(crate) fn collect_nested_batch(
    mut reader: Box<dyn RecordBatchReader + Send>,
) -> OdbcResult<RecordBatch> {
    let schema = reader.schema();
    let mut batches = vec![];
    for b in &mut *reader {
        let batch = b.context(ArrowBatchReadSnafu)?;
        batches.push(batch);
    }
    if batches.is_empty() {
        Ok(RecordBatch::new_empty(schema))
    } else if batches.len() == 1 {
        Ok(batches.remove(0))
    } else {
        use arrow::compute::concat_batches;
        concat_batches(&schema, &batches).context(ArrowBatchConcatSnafu)
    }
}

pub(crate) fn execute_show_query_collect_batch(
    stmt_handle: StatementHandle,
    sql: &str,
) -> OdbcResult<RecordBatch> {
    let rt = global().context(OdbcRuntimeSnafu)?;
    let response = rt.block_on(async |c| {
        c.statement_set_sql_query(StatementSetSqlQueryRequest {
            stmt_handle: Some(stmt_handle),
            query: sql.to_string(),
        })
        .await?;
        c.statement_execute_query(StatementExecuteQueryRequest {
            stmt_handle: Some(stmt_handle),
            bindings: None,
            timeout_seconds: None,
        })
        .await
    })?;

    let rs_handle = match response.result.context(InternalSnafu {
        message: "execute_show_query: missing execute result".to_string(),
    })? {
        execute_query_response::Result::Single(rs) => {
            rs.result_set_handle.context(InternalSnafu {
                message: "execute_show_query: missing result_set_handle".to_string(),
            })?
        }
        execute_query_response::Result::Multi(_) => {
            return InternalSnafu {
                message: "execute_show_query: unexpected multi-statement result".to_string(),
            }
            .fail();
        }
    };

    let stream = fetch_stream_and_release(rs_handle)?;
    let reader = reader_from_protobuf_stream(stream)?;
    collect_nested_batch(Box::new(reader))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BindingMode {
    Json,
    Csv,
}

/// Build query bindings from ODBC parameter bindings.
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
/// `mode` selects the wire format. The second tuple element is the owner
/// of the serialized buffer that the returned `BinaryDataPtr` points into;
/// callers must hold onto it for the lifetime of the async execute call.
fn apply_parameter_bindings(
    apd: &crate::api::ApdDescriptor,
    ipd: &crate::api::IpdDescriptor,
    prepared: bool,
    prepared_param_count: Option<u16>,
    mode: BindingMode,
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
        "apply_parameter_bindings: Found {} bound parameters (effective_count={}, mode={:?})",
        apd.records.len(),
        effective_count,
        mode,
    );

    let (owner, binding_type) = match mode {
        BindingMode::Json => {
            let s = odbc_bindings_to_json(apd, ipd, effective_count).context(JsonBindingSnafu)?;
            let ptr = BinaryDataPtr {
                value: (s.as_bytes().as_ptr() as u64).to_le_bytes().to_vec(),
                length: s.len() as i64,
            };
            (s, query_bindings::BindingType::Json(ptr))
        }
        BindingMode::Csv => {
            let s = odbc_bindings_to_csv(apd, ipd, effective_count).context(CsvBindingSnafu)?;
            let ptr = BinaryDataPtr {
                value: (s.as_bytes().as_ptr() as u64).to_le_bytes().to_vec(),
                length: s.len() as i64,
            };
            (s, query_bindings::BindingType::Csv(ptr))
        }
    };

    let bindings = QueryBindings {
        binding_type: Some(binding_type),
    };

    tracing::info!("apply_parameter_bindings: Successfully bound parameters");

    Ok((Some(bindings), Some(owner)))
}

/// Decide whether to use JSON or CSV (stage) binding.
///
/// `effective_cells` is `array_size × param_count` — the total number of
/// cells across all parameter sets in this execute.  The server threshold
/// `CLIENT_STAGE_ARRAY_BINDING_THRESHOLD` is expressed in cells.
///
/// * `array_bind_supported == Some(false)` — server explicitly said no (e.g.
///   bare SELECT, EXECUTE IMMEDIATE).  Always JSON to avoid server rejection.
/// * `array_bind_supported == None` — no prepare ran (SQLExecDirect) or no
///   hint returned.  Treated as "unknown / assume yes"; fall through to
///   threshold check.
/// * `array_bind_supported == Some(true)` — server confirmed support.
fn select_binding_mode(
    conn_handle: &ConnectionHandle,
    effective_cells: u64,
    array_bind_supported: Option<bool>,
) -> OdbcResult<BindingMode> {
    if effective_cells == 0 {
        return Ok(BindingMode::Json);
    }
    // Hard block only when server explicitly said no.
    if array_bind_supported == Some(false) {
        return Ok(BindingMode::Json);
    }
    let threshold = stage_binding_threshold(conn_handle)?;
    if effective_cells >= u64::from(threshold) {
        Ok(BindingMode::Csv)
    } else {
        Ok(BindingMode::Json)
    }
}

fn stage_binding_threshold(conn_handle: &ConnectionHandle) -> OdbcResult<u32> {
    let raw = get_session_parameter(conn_handle, "CLIENT_STAGE_ARRAY_BINDING_THRESHOLD")?;
    // Default Snowflake value is 65280 (255 * 256).  0 would mean "never
    // stage-bind" when the parameter is absent, which is wrong.
    Ok(raw.and_then(|s| s.parse::<u32>().ok()).unwrap_or(65280))
}

fn effective_param_count(
    apd: &crate::api::ApdDescriptor,
    ipd: &crate::api::IpdDescriptor,
    prepared: bool,
    prepared_param_count: Option<u16>,
) -> u16 {
    if prepared {
        prepared_param_count.unwrap_or(0)
    } else {
        apd.desc_count().max(ipd.desc_count())
    }
}

fn get_session_parameter(conn_handle: &ConnectionHandle, key: &str) -> OdbcResult<Option<String>> {
    crate::api::runtime::global()
        .context(OdbcRuntimeSnafu)?
        .block_on(async |c| {
            let resp = c
                .connection_get_parameter(ConnectionGetParameterRequest {
                    conn_handle: Some(*conn_handle),
                    key: key.to_string(),
                })
                .await?;
            Ok(resp.value)
        })
}

/// Default ODBC column size (precision) for fixed-size SQL parameter types,
/// per the ODBC spec (Appendix D). Returns `None` for variable-length / unsized
/// types, whose size must come from the caller's `ColumnSize` argument.
fn default_param_column_size(sql_data_type: sql::SqlDataType) -> Option<sql::ULen> {
    Some(match sql_data_type {
        sql::SqlDataType::EXT_BIT => 1,
        sql::SqlDataType::EXT_TINY_INT => 3,
        sql::SqlDataType::SMALLINT => 5,
        sql::SqlDataType::INTEGER => 10,
        sql::SqlDataType::EXT_BIG_INT => 19,
        sql::SqlDataType::REAL => 7,
        sql::SqlDataType::FLOAT | sql::SqlDataType::DOUBLE => 15,
        _ => return None,
    })
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

    // For fixed-size SQL types the application may legitimately pass
    // `ColumnSize` 0 (it is ignored for those types); `SQLDescribeParam` must
    // still report the type's natural precision rather than 0. Variable-length
    // types keep the caller-supplied size.
    let column_size = if column_size == 0 {
        default_param_column_size(stored_sql_data_type).unwrap_or(column_size)
    } else {
        column_size
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

    inner.insert_active_apd_record(
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
            inner.with_active_ard_mut(|ard| ard.unbind_all());
        }
        FreeStmtOption::ResetParams => {
            tracing::info!("free_stmt: Resetting all parameter bindings");
            inner.clear_active_apd_records();
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
        inner.with_active_ard_mut(|ard| {
            ard.bindings.remove(&column_number);
        });
    } else {
        if buffer_length < 0 {
            return InvalidBufferLengthSnafu {
                length: buffer_length as i64,
            }
            .fail();
        }
        inner.with_active_ard_mut(|ard| {
            ard.bindings.insert(
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
                    length: 0,
                },
            );
        });
    }
    Ok(())
}

/// Look up the `ExplicitDesc` Arc for `desc_handle` on connection `conn_id`.
/// Returns HY017 if the handle is an implicitly-allocated descriptor (foreign,
/// since the caller already handled the statement's own implicit handles).
/// Returns HY024 if the handle belongs to a different connection.
/// Does NOT hold the statement `inner` lock — call this after dropping it to
/// preserve the Connection-before-inner lock ordering.
fn lookup_explicit_desc(
    desc_handle: sql::Handle,
    conn_id: HandleId,
    attribute: i32,
) -> OdbcResult<ExplicitDesc> {
    let desc_id = HandleId::from(desc_handle);
    let g = global().context(OdbcRuntimeSnafu)?;
    let desc_guard = g.desc_manager.get(desc_id)?;
    match *desc_guard {
        crate::api::handle_registry::DescLookup::Implicit { .. } => {
            InvalidUseOfImplicitDescriptorSnafu.fail()
        }
        crate::api::handle_registry::DescLookup::Explicit { conn_id: owner }
            if owner == conn_id =>
        {
            drop(desc_guard);
            let dbc = g.dbc_registry.get(conn_id)?;
            let conn = dbc.connection.lock();
            conn.child_descriptors
                .iter()
                .find(|(id, _)| *id == desc_id)
                .map(|(_, a)| a.clone())
                .ok_or_else(|| InvalidHandleSnafu.build())
        }
        _ => InvalidAttributeValueSnafu {
            attribute,
            value: desc_handle as i64,
        }
        .fail(),
    }
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
            inner.with_active_ard_mut(|ard| {
                ard.array_size = effective_size;
            });
            Ok(())
        }
        StmtAttr::RowStatusPtr => {
            let ptr = value_ptr as *mut u16;
            tracing::debug!("set_stmt_attr: RowStatusPtr = {:?}", ptr);
            inner.ird.array_status_ptr = ptr;
            Ok(())
        }
        StmtAttr::RowOperationPtr => {
            // SQL_DESC_ARRAY_STATUS_PTR on the ARD. Stored for retrieval; not
            // consulted because Snowflake cursors are forward-only (no SQLSetPos).
            let ptr = value_ptr as *mut u16;
            tracing::debug!("set_stmt_attr: RowOperationPtr = {:?}", ptr);
            inner.with_active_ard_mut(|ard| {
                ard.array_status_ptr = ptr;
            });
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
            inner.with_active_ard_mut(|ard| {
                ard.bind_type = raw_bind_type;
            });
            Ok(())
        }
        StmtAttr::RowBindOffsetPtr => {
            let ptr = value_ptr as *mut sql::Len;
            tracing::debug!("set_stmt_attr: RowBindOffsetPtr = {:?}", ptr);
            inner.with_active_ard_mut(|ard| {
                ard.bind_offset_ptr = ptr;
            });
            Ok(())
        }
        StmtAttr::ParamBindType => {
            let raw = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: ParamBindType (raw) = {}", raw);
            inner.with_effective_apd_header_mut(|_, bind_type, _| *bind_type = raw);
            Ok(())
        }
        StmtAttr::ParamBindOffsetPtr => {
            let ptr = value_ptr as *mut sql::Len;
            tracing::debug!("set_stmt_attr: ParamBindOffsetPtr = {:?}", ptr);
            inner.with_effective_apd_header_mut(|_, _, bind_offset_ptr| *bind_offset_ptr = ptr);
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
        StmtAttr::ParamsetSize => {
            let size = value_ptr as usize;
            tracing::debug!("set_stmt_attr: ParamsetSize = {}", size);
            let coerced = if size == 0 {
                tracing::warn!("set_stmt_attr: ParamsetSize 0 is invalid, coercing to 1");
                1
            } else {
                size
            };
            inner.with_effective_apd_header_mut(|array_size, _, _| *array_size = coerced);
            Ok(())
        }
        StmtAttr::MetadataId => {
            let val = value_ptr as sql::ULen;
            inner.metadata_id = val != 0;
            Ok(())
        }
        StmtAttr::AppRowDesc => {
            let handle = value_ptr as sql::Handle;
            if handle.is_null() || HandleId::from(handle) == inner.ard_handle {
                // NULL means "revert to implicit". The Windows DM also sends
                // the statement's own implicit ARD handle to mean the same thing.
                inner.active_ard = None;
            } else {
                let conn_id = guard.conn_id;
                drop(inner);
                let arc = lookup_explicit_desc(handle, conn_id, attribute)?;
                let mut inner = guard.inner.lock();
                inner.active_ard = Some((HandleId::from(handle), arc));
            }
            Ok(())
        }
        StmtAttr::AppParamDesc => {
            let handle = value_ptr as sql::Handle;
            if handle.is_null() || HandleId::from(handle) == inner.apd_handle {
                inner.active_apd = None;
            } else {
                let conn_id = guard.conn_id;
                drop(inner);
                let arc = lookup_explicit_desc(handle, conn_id, attribute)?;
                let mut inner = guard.inner.lock();
                inner.active_apd = Some((HandleId::from(handle), arc));
            }
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
        StmtAttr::AsyncEnable => {
            let val = value_ptr as sql::ULen;
            if inner.state.as_ref().is_async_executing() {
                return AttributeCannotBeSetNowSnafu { attribute }.fail();
            }
            match val {
                0 => {
                    inner.async_enabled = false;
                    Ok(())
                }
                1 => {
                    inner.async_enabled = true;
                    Ok(())
                }
                _ => InvalidAttributeValueSnafu {
                    attribute,
                    value: val as i64,
                }
                .fail(),
            }
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

    let attr = match StmtAttr::try_from(attribute) {
        Ok(a) => a,
        // A get of a valid-but-unsupported ODBC attribute returns HYC00; an
        // identifier outside the ODBC-defined range returns HY092 — the error
        // `try_from` already produced (SNOW-3235557).
        Err(e) => {
            return if StmtAttr::is_known_odbc(attribute) {
                UnsupportedAttributeSnafu { attribute }.fail()
            } else {
                Err(e)
            };
        }
    };
    let guard = stmt_from_handle(statement_handle)?;
    let inner = guard.inner.lock();

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
            unsafe {
                *(value_ptr as *mut sql::Handle) = inner.active_ard_handle().into();
            }
            Ok(())
        }
        StmtAttr::ImpRowDesc => {
            unsafe {
                *(value_ptr as *mut sql::Handle) = inner.ird_handle.into();
            }
            Ok(())
        }
        StmtAttr::AppParamDesc => {
            unsafe {
                *(value_ptr as *mut sql::Handle) = inner.active_apd_handle().into();
            }
            Ok(())
        }
        StmtAttr::ImpParamDesc => {
            unsafe {
                *(value_ptr as *mut sql::Handle) = inner.ipd_handle.into();
            }
            Ok(())
        }
        StmtAttr::RowArraySize => {
            inner.with_active_ard(|ard| unsafe {
                *(value_ptr as *mut sql::ULen) = ard.array_size as sql::ULen;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            });
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
            inner.with_active_ard(|ard| unsafe {
                *(value_ptr as *mut sql::ULen) = ard.bind_type;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            });
            Ok(())
        }
        StmtAttr::RowBindOffsetPtr => {
            inner.with_active_ard(|ard| unsafe {
                *(value_ptr as *mut *mut sql::Len) = ard.bind_offset_ptr;
            });
            Ok(())
        }
        StmtAttr::RowNumber => {
            // Read-only: 1-based position of the current row while the cursor is
            // positioned (Fetching); 0 before the first fetch, after end-of-data,
            // and in error states. `rows_returned` is the running fetch count —
            // exact for the common array_size == 1 case; for block cursors it
            // reports rows returned so far (Snowflake serves forward-only).
            let row_number = if matches!(inner.state.as_ref(), StatementState::Fetching { .. }) {
                inner.rows_returned
            } else {
                0
            };
            unsafe {
                *(value_ptr as *mut sql::ULen) = row_number;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::RowOperationPtr => {
            inner.with_active_ard(|ard| unsafe {
                *(value_ptr as *mut *mut u16) = ard.array_status_ptr;
            });
            Ok(())
        }
        StmtAttr::ParamBindType => {
            let bind_type = inner.with_effective_apd(|apd| apd.bind_type);
            unsafe {
                *(value_ptr as *mut sql::ULen) = bind_type;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::ParamBindOffsetPtr => {
            let bind_offset_ptr = inner.with_effective_apd(|apd| apd.bind_offset_ptr);
            unsafe {
                *(value_ptr as *mut *mut sql::Len) = bind_offset_ptr;
            }
            Ok(())
        }
        StmtAttr::ParamStatusPtr => {
            unsafe {
                *(value_ptr as *mut *mut u16) = inner.ipd.array_status_ptr;
            }
            Ok(())
        }
        StmtAttr::ParamsProcessedPtr => {
            unsafe {
                *(value_ptr as *mut *mut sql::ULen) = inner.ipd.rows_processed_ptr;
            }
            Ok(())
        }
        StmtAttr::ParamsetSize => {
            let array_size = inner.with_effective_apd(|apd| apd.array_size);
            unsafe {
                *(value_ptr as *mut sql::ULen) = array_size as sql::ULen;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
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
        StmtAttr::AsyncEnable => {
            let val: sql::ULen = if inner.async_enabled { 1 } else { 0 };
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = val };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
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
                let token = inner.with_effective_apd(|apd| get_param_token(apd, param_num));
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
                    let token = inner.with_effective_apd(|apd| get_param_token(apd, param_num));
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
                    &guard.cancel_token,
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
            let result = inner.with_effective_apd(|apd| {
                put_data_inner(apd, &mut dae_context, data_ptr, str_len_or_ind)
            });
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
            let result = inner.with_effective_apd(|apd| {
                put_data_inner(apd, &mut dae_context, data_ptr, str_len_or_ind)
            });
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

/// Sets `cancel_token` on creation and clears it on drop, so every exit
/// path (including early returns and panics) releases the token.
struct CancelTokenGuard<'a> {
    slot: &'a parking_lot::Mutex<Option<CancellationToken>>,
}

impl<'a> CancelTokenGuard<'a> {
    fn arm(
        slot: &'a parking_lot::Mutex<Option<CancellationToken>>,
        token: CancellationToken,
    ) -> Self {
        *slot.lock() = Some(token);
        Self { slot }
    }
}

impl Drop for CancelTokenGuard<'_> {
    fn drop(&mut self) {
        *self.slot.lock() = None;
    }
}

/// Awaits a finished `JoinHandle` and translates panics into internal errors.
fn poll_join_handle<T>(join_handle: tokio::task::JoinHandle<OdbcResult<T>>) -> OdbcResult<T> {
    let outcome = global()
        .context(OdbcRuntimeSnafu)?
        .block_on(async |_c| join_handle.await);
    match outcome {
        Ok(Ok(o)) => Ok(o),
        Ok(Err(e)) => Err(e),
        Err(_join_err) => crate::api::error::InternalSnafu {
            message: "async task panicked".to_string(),
        }
        .fail(),
    }
}

/// Completes an in-flight async operation: clears the cancel token, awaits
/// the join handle, and sets state to `Error` on failure.
fn complete_async_poll<T>(
    cancel_token: &parking_lot::Mutex<Option<CancellationToken>>,
    inner: &mut StatementInner,
    join_handle: tokio::task::JoinHandle<OdbcResult<T>>,
) -> OdbcResult<T> {
    *cancel_token.lock() = None;
    match poll_join_handle(join_handle) {
        Ok(result) => Ok(result),
        Err(e) => {
            inner.state.set(StatementState::Error);
            Err(e)
        }
    }
}

/// Result of `run_cancellable`: either the operation completed synchronously,
/// or a task was spawned and the caller must store the join handle.
enum Execution<T> {
    Completed(T),
    Spawned(tokio::task::JoinHandle<OdbcResult<T>>),
}

/// Executes an async operation either synchronously (blocking) or by spawning
/// it as a background task.
///
/// - **Sync** (`async_enabled = false`): arms a `CancelTokenGuard`, blocks on
///   the future, and returns `Execution::Completed(result)`.
/// - **Async** (`async_enabled = true`): spawns the future, sets
///   `cancel_token`, and returns `Execution::Spawned(join_handle)`. The
///   caller is responsible for storing the handle in the appropriate
///   `StatementState` variant and returning `StillExecuting`.
///
/// In both modes, a `CancellationToken` is wired into `tokio::select!` so
/// that `SQLCancel` can abort the operation.
fn run_cancellable<T, F>(
    stmt: &crate::api::Statement,
    async_enabled: bool,
    f: impl FnOnce(
        std::sync::Arc<sf_core::protobuf::apis::database_driver_v1::DatabaseDriverClient>,
    ) -> F,
) -> OdbcResult<Execution<T>>
where
    T: Send + 'static,
    F: std::future::Future<Output = OdbcResult<T>> + Send + 'static,
{
    let token = CancellationToken::new();
    let g = global().context(OdbcRuntimeSnafu)?;
    let client = g.client();

    if async_enabled {
        let token_clone = token.clone();
        let future = f(client);
        let join_handle = g.spawn(async move {
            tokio::select! {
                biased;
                _ = token_clone.cancelled() => Err(OperationCanceledSnafu.build()),
                result = future => result,
            }
        });
        *stmt.cancel_token.lock() = Some(token);
        return Ok(Execution::Spawned(join_handle));
    }

    let _cancel_guard = CancelTokenGuard::arm(&stmt.cancel_token, token.clone());
    let future = f(client);
    let result = g.block_on(async move |_c| {
        tokio::select! {
            biased;
            _ = token.cancelled() => Err(OperationCanceledSnafu.build()),
            result = future => result,
        }
    })?;
    Ok(Execution::Completed(result))
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
    cancel_token_slot: &parking_lot::Mutex<Option<CancellationToken>>,
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
    // accumulated data, keeping non-DAE records as-is. Seed from the *active*
    // APD (implicit or explicit) including its header fields.
    let mut temp_apd = crate::api::ApdDescriptor::new();
    inner.with_effective_apd(|apd| {
        temp_apd.array_size = apd.array_size;
        temp_apd.bind_type = apd.bind_type;
        temp_apd.bind_offset_ptr = apd.bind_offset_ptr;
        for (&param_num, rec) in &apd.records {
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
    });

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

    let param_count = effective_param_count(
        &temp_apd,
        &inner.ipd,
        is_prepared,
        inner.prepared_param_count,
    );
    let effective_cells = temp_apd.array_size as u64 * u64::from(param_count);
    let binding_mode = match select_binding_mode(
        &conn_handle,
        effective_cells,
        inner.prepared_array_bind_supported,
    ) {
        Ok(m) => m,
        Err(e) => {
            inner.state.set(restored);
            return Err(e);
        }
    };
    let (bindings, _bindings_owner) = match apply_parameter_bindings(
        &temp_apd,
        &inner.ipd,
        is_prepared,
        inner.prepared_param_count,
        binding_mode,
    ) {
        Ok(b) => b,
        Err(e) => {
            inner.state.set(restored);
            return Err(e);
        }
    };

    let query_timeout = inner.query_timeout;
    let deferred_query = dae_context.deferred_query;
    // Capture the SQL (if any) before `deferred_query` is moved into the
    // async block, so the post-execute refresh can detect `ALTER SESSION`.
    // `None` for prepared DAE executes, matching the `execute` path.
    let last_sql = deferred_query.clone();

    let token = CancellationToken::new();
    let _cancel_guard = CancelTokenGuard::arm(cancel_token_slot, token.clone());

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
    if let Err(e) = update_numeric_settings(
        &conn_handle,
        &mut conn.numeric_settings,
        last_sql.as_deref(),
    ) {
        inner.state.set(restored);
        return Err(e);
    }
    apply_execute_response(inner, conn_handle, response, origin)?;
    inner.rows_returned = 0;
    Ok(())
}

/// Cancel processing on a statement (SQLCancel).
///
/// Checks `Statement::cancel_token` first to signal any in-flight
/// sync or async operation without touching `inner`. Falls back to
/// restoring NeedData state (single-threaded DAE scenarios): from S8/S9/S10
/// the statement is restored to its pre-execute state (`Prepared` for
/// `SQLExecute` origin, `Created` for `SQLExecDirect`). Column and parameter
/// bindings are preserved; accumulated SQLPutData is discarded.
///
/// Per ODBC 3.5 spec, cross-thread `SQLCancel` does not clear or post
/// diagnostic records.
pub fn cancel(statement_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!("cancel: statement_handle={statement_handle:?}");
    let guard = stmt_from_handle(statement_handle)?;

    // Fast path: cancel any in-flight execution via the token.
    {
        let token = guard.cancel_token.lock();
        if let Some(ref t) = *token {
            t.cancel();
            return Ok(());
        }
    }

    // No execution in flight — check NeedData state (single-threaded).
    let mut inner = guard.inner.lock();
    match inner.state.as_ref() {
        StatementState::AwaitingParamData { origin, .. }
        | StatementState::AwaitingPutData { origin, .. }
        | StatementState::PutDataCalled { origin, .. } => {
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
    fn cancel_token_guard_clears_slot_on_drop() {
        let slot = parking_lot::Mutex::new(None);
        {
            let token = CancellationToken::new();
            let _guard = CancelTokenGuard::arm(&slot, token);
            assert!(slot.lock().is_some());
        }
        assert!(slot.lock().is_none());
    }

    #[test]
    fn cancel_token_guard_cleared_after_runtime_unavailable() {
        let slot = parking_lot::Mutex::new(None);
        {
            let token = CancellationToken::new();
            let _guard = CancelTokenGuard::arm(&slot, token);
            assert!(global().context(OdbcRuntimeSnafu).is_err());
        }
        assert!(slot.lock().is_none());
    }

    #[test]
    fn apply_bindings_prepared_without_param_count_errors() {
        let apd = ApdDescriptor::new();
        let ipd = IpdDescriptor::new();
        let result = apply_parameter_bindings(&apd, &ipd, true, None, BindingMode::Json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.to_sql_state(), SqlState::CountFieldIncorrect);
    }

    #[test]
    fn select_binding_mode_returns_json_when_no_bindings() {
        // Zero cells short-circuits to Json without any threshold lookup.
        let handle = ConnectionHandle { id: 0, magic: 0 };
        let mode = select_binding_mode(&handle, 0, None).unwrap();
        assert_eq!(mode, BindingMode::Json);
        let mode = select_binding_mode(&handle, 0, Some(true)).unwrap();
        assert_eq!(mode, BindingMode::Json);
    }

    #[test]
    fn select_binding_mode_returns_json_when_array_bind_unsupported() {
        // `arrayBindSupported=false` must hard-force JSON — server would reject
        // stage-binding (SQLSTATE 42601). Fires before threshold lookup.
        let handle = ConnectionHandle { id: 0, magic: 0 };
        let mode = select_binding_mode(&handle, 1, Some(false)).unwrap();
        assert_eq!(mode, BindingMode::Json);
    }

    #[test]
    fn select_binding_mode_none_hint_falls_through_to_threshold() {
        // `None` means no prepare hint (SQLExecDirect path) — treated as
        // "unknown / assume supported".  With 0 cells it still short-circuits
        // to Json; only the cell count controls the outcome here.
        let handle = ConnectionHandle { id: 0, magic: 0 };
        // 0 cells → Json regardless of hint
        let mode = select_binding_mode(&handle, 0, None).unwrap();
        assert_eq!(mode, BindingMode::Json);
        // Non-zero cells → falls through to threshold check (no live
        // connection here, so the RPC will error; the test only asserts the
        // short-circuit paths above don't fire for `None`).
    }
}
