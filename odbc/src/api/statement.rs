use crate::api::CDataType;
use crate::api::encoding::{OdbcEncoding, write_string_bytes_i32};
use crate::api::error::{
    ArrowArrayStreamReaderCreationSnafu, CursorAlreadyOpenSnafu, DisconnectedSnafu,
    InvalidAttributeValueSnafu, InvalidBufferLengthSnafu, InvalidCursorStateSnafu,
    InvalidHandleSnafu, InvalidParameterNumberSnafu, InvalidPrecisionOrScaleSnafu,
    JsonBindingSnafu, NoMoreDataSnafu, NullPointerSnafu, OdbcRuntimeSnafu,
    ReadOnlyAttributeSnafu, Required, StatementNotExecutedSnafu, UnsupportedFeatureSnafu,
};
use crate::api::runtime::global;
use crate::api::{
    ApdRecord, ConnectionState, FreeStmtOption, IpdRecord, OdbcResult, ParamDirection,
    ParameterBinding, SqlType, Statement, StatementState, stmt_from_handle,
};
use crate::conversion::Binding;
use crate::conversion::param_binding::{odbc_bindings_to_json, odbc_bindings_to_json_array};
use arrow::array::RecordBatchReader;
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use odbc_sys as sql;
use sf_core::protobuf::generated::database_driver_v1::{
    ArrowArrayStreamPtr, BinaryDataPtr, ConnectionGetParameterRequest, ConnectionHandle,
    QueryBindings, StatementExecuteQueryRequest, StatementExecuteQueryResponse,
    StatementNewRequest, StatementPrepareRequest, StatementReleaseRequest,
    StatementSetOptionIntRequest, StatementSetSqlQueryRequest, query_bindings,
};
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;
use tracing;

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
    let stmt = stmt_from_handle(statement_handle);
    tracing::debug!("exec_direct: statement_handle={:?}", statement_handle);

    if matches!(
        stmt.state.as_ref(),
        StatementState::QueryExecuted { .. }
            | StatementState::Fetching { .. }
            | StatementState::Done { .. }
    ) {
        tracing::error!("exec_direct: cursor is already open");
        return CursorAlreadyOpenSnafu.fail();
    }

    // Obtain an independent &mut Connection without tying up a borrow on stmt,
    // so stmt.apd / stmt.ipd / stmt.stmt_handle remain accessible below.
    let conn = unsafe { &mut *stmt.conn_ptr() };
    match &mut conn.state {
        ConnectionState::Connected {
            db_handle: _,
            conn_handle,
        } => {
            let conn_h = *conn_handle;
            let array_size = stmt.apd.array_size;
            let rows_processed_ptr = stmt.ipd.rows_processed_ptr;
            let param_status_ptr = stmt.ipd.array_status_ptr;
            let operation_ptr = stmt.apd.array_status_ptr as *const u16;
            let (bindings, _json_owner) = apply_parameter_bindings(&stmt.apd, &stmt.ipd, false)?;
            let stmt_handle = stmt.stmt_handle;
            let query_timeout = stmt.query_timeout;
            let multi_statement_count = stmt.multi_statement_count;

            stmt.cancel_token = CancellationToken::new();
            let _cancel_token = stmt.cancel_token.clone();
            // TODO(SNOW-3258922): Wrap RPC in tokio::select! with
            // _cancel_token.cancelled() to support cross-thread SQLCancel.
            let response = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
                // Set session-level timeout using a temporary statement handle so
                // it can be unset after the main query without touching stmt_handle.
                let tmp_stmt_opt = if query_timeout > 0 {
                    let tmp_resp = c
                        .statement_new(StatementNewRequest {
                            conn_handle: Some(conn_h),
                        })
                        .await?;
                    let h = tmp_resp.stmt_handle.ok_or_else(|| {
                        proto_utils::ProtoError::Transport(
                            "Temporary statement handle is required".to_string(),
                        )
                    })?;
                    let set_result = async {
                        c.statement_set_sql_query(StatementSetSqlQueryRequest {
                            stmt_handle: Some(h),
                            query: format!(
                                "ALTER SESSION SET STATEMENT_TIMEOUT_IN_SECONDS = {query_timeout}"
                            ),
                        })
                        .await?;
                        c.statement_execute_query(StatementExecuteQueryRequest {
                            stmt_handle: Some(h),
                            bindings: None,
                        })
                        .await
                    }
                    .await;
                    if let Err(e) = set_result {
                        let _ = c
                            .statement_release(StatementReleaseRequest {
                                stmt_handle: Some(h),
                            })
                            .await;
                        return Err(e);
                    }
                    Some(h)
                } else {
                    None
                };

                if multi_statement_count >= 0 {
                    c.statement_set_option_int(StatementSetOptionIntRequest {
                        stmt_handle: Some(stmt_handle),
                        key: "MULTI_STATEMENT_COUNT".to_string(),
                        value: multi_statement_count as i64,
                    })
                    .await?;
                }

                c.statement_set_sql_query(StatementSetSqlQueryRequest {
                    stmt_handle: Some(stmt_handle),
                    query: statement_text.to_string(),
                })
                .await?;

                let main_result = c
                    .statement_execute_query(StatementExecuteQueryRequest {
                        stmt_handle: Some(stmt_handle),
                        bindings,
                    })
                    .await;

                // Reset session timeout regardless of main query outcome.
                if let Some(h) = tmp_stmt_opt {
                    let set_result = c
                        .statement_set_sql_query(StatementSetSqlQueryRequest {
                            stmt_handle: Some(h),
                            query: "ALTER SESSION UNSET STATEMENT_TIMEOUT_IN_SECONDS".to_string(),
                        })
                        .await;
                    if set_result.is_ok() {
                        let _ = c
                            .statement_execute_query(StatementExecuteQueryRequest {
                                stmt_handle: Some(h),
                                bindings: None,
                            })
                            .await;
                    }
                    if let Err(e) = c
                        .statement_release(StatementReleaseRequest {
                            stmt_handle: Some(h),
                        })
                        .await
                    {
                        tracing::warn!(
                            "exec_direct: failed to release timeout statement handle: {:?}",
                            e
                        );
                    }
                }

                main_result
            });

            tracing::info!("exec_direct: response={:?}", response);
            let response = response?;

            let query_id = response.result.as_ref().map(|r| r.query_id.clone());
            write_param_array_status(rows_processed_ptr, param_status_ptr, array_size, operation_ptr);
            update_numeric_settings(conn_handle, &mut conn.numeric_settings)?;
            let execute_state = create_execute_state(response, false)?;
            let is_zero_dml = matches!(
                &execute_state,
                StatementState::DmlExecuted {
                    rows_affected: 0,
                    ..
                }
            );
            set_state(stmt, execute_state);
            stmt.rows_returned = 0;
            stmt.last_query_id = query_id.filter(|s| !s.is_empty());
            if is_zero_dml {
                return NoMoreDataSnafu.fail();
            }
            Ok(())
        }
        ConnectionState::Disconnected => {
            tracing::error!("exec_direct: connection is disconnected");
            DisconnectedSnafu.fail()
        }
    }
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
    });
    Ok(())
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
    let stmt = stmt_from_handle(statement_handle);

    if matches!(
        stmt.state.as_ref(),
        StatementState::QueryExecuted { .. }
            | StatementState::Fetching { .. }
            | StatementState::Done { .. }
    ) {
        tracing::error!("prepare: cursor is already open");
        return CursorAlreadyOpenSnafu.fail();
    }

    let conn = unsafe { &mut *stmt.conn_ptr() };
    match &mut conn.state {
        ConnectionState::Connected {
            db_handle: _,
            conn_handle: _,
        } => {
            tracing::debug!("prepare: query = {query}");

            let stmt_handle = stmt.stmt_handle;
            stmt.cancel_token = CancellationToken::new();
            let _cancel_token = stmt.cancel_token.clone();
            // TODO(SNOW-3258922): Wire _cancel_token into tokio::select!
            // alongside the RPC future to support cancellation.
            let prepare_result = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
                c.statement_set_sql_query(StatementSetSqlQueryRequest {
                    stmt_handle: Some(stmt_handle),
                    query: query.to_string(),
                })
                .await?;

                c.statement_prepare(StatementPrepareRequest {
                    stmt_handle: Some(stmt_handle),
                })
                .await
            })?;

            let result = prepare_result.result.required("Result is required")?;
            let stream_ptr = result.stream.required("Stream is required")?;
            let reader = reader_from_protobuf_stream(stream_ptr)?;
            let schema = reader.schema();
            stmt.ird.desc_count = schema.fields().len() as sql::SmallInt;

            let param_count = result.number_of_binds.max(0) as usize;
            let max_varchar = conn.numeric_settings.max_varchar_size;
            stmt.ipd.records.retain(|&k, _| (k as usize) <= param_count);
            for i in 1..=param_count {
                stmt.ipd
                    .records
                    .entry(i as u16)
                    .or_insert_with(|| IpdRecord::with_varchar_size(max_varchar));
            }
            tracing::info!(
                "prepare: auto-IPD populated {param_count} parameter markers (from server)"
            );

            stmt.state.set(StatementState::Prepared { schema });
            tracing::info!("prepare: Successfully prepared statement");
            Ok(())
        }
        ConnectionState::Disconnected => {
            tracing::error!("prepare: connection is disconnected");
            DisconnectedSnafu.fail()
        }
    }
}

/// Execute a prepared statement
pub fn execute(statement_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!("execute: statement_handle={:?}", statement_handle);
    let stmt = stmt_from_handle(statement_handle);

    if matches!(
        stmt.state.as_ref(),
        StatementState::QueryExecuted { .. }
            | StatementState::Fetching { .. }
            | StatementState::Done { .. }
    ) {
        tracing::error!("execute: cursor is already open");
        return CursorAlreadyOpenSnafu.fail();
    }

    let prepared = match stmt.state.as_ref() {
        StatementState::Prepared { .. } => true,
        StatementState::DdlExecuted { prepared, .. }
        | StatementState::DmlExecuted { prepared, .. } => *prepared,
        _ => false,
    };

    let query_timeout = stmt.query_timeout;
    let multi_statement_count = stmt.multi_statement_count;
    let array_size = stmt.apd.array_size;
    let rows_processed_ptr = stmt.ipd.rows_processed_ptr;
    let param_status_ptr = stmt.ipd.array_status_ptr;
    let operation_ptr = stmt.apd.array_status_ptr as *const u16;
    let conn = unsafe { &mut *stmt.conn_ptr() };
    match &mut conn.state {
        ConnectionState::Connected {
            db_handle: _,
            conn_handle,
        } => {
            let conn_h = *conn_handle;
            let stmt_handle = stmt.stmt_handle;
            let (bindings, _json_owner) = apply_parameter_bindings(&stmt.apd, &stmt.ipd, prepared)?;

            stmt.cancel_token = CancellationToken::new();
            let _cancel_token = stmt.cancel_token.clone();
            // TODO(SNOW-3258922): Wrap RPC in tokio::select! with
            // _cancel_token.cancelled() to support cross-thread SQLCancel.
            let response = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
                let tmp_stmt_opt = if query_timeout > 0 {
                    let tmp_resp = c
                        .statement_new(StatementNewRequest {
                            conn_handle: Some(conn_h),
                        })
                        .await?;
                    let h = tmp_resp.stmt_handle.ok_or_else(|| {
                        proto_utils::ProtoError::Transport(
                            "Temporary statement handle is required".to_string(),
                        )
                    })?;
                    let set_result = async {
                        c.statement_set_sql_query(StatementSetSqlQueryRequest {
                            stmt_handle: Some(h),
                            query: format!(
                                "ALTER SESSION SET STATEMENT_TIMEOUT_IN_SECONDS = {query_timeout}"
                            ),
                        })
                        .await?;
                        c.statement_execute_query(StatementExecuteQueryRequest {
                            stmt_handle: Some(h),
                            bindings: None,
                        })
                        .await
                    }
                    .await;
                    if let Err(e) = set_result {
                        let _ = c
                            .statement_release(StatementReleaseRequest {
                                stmt_handle: Some(h),
                            })
                            .await;
                        return Err(e);
                    }
                    Some(h)
                } else {
                    None
                };

                if multi_statement_count >= 0 {
                    c.statement_set_option_int(StatementSetOptionIntRequest {
                        stmt_handle: Some(stmt_handle),
                        key: "MULTI_STATEMENT_COUNT".to_string(),
                        value: multi_statement_count as i64,
                    })
                    .await?;
                }

                let main_result = c
                    .statement_execute_query(StatementExecuteQueryRequest {
                        stmt_handle: Some(stmt_handle),
                        bindings,
                    })
                    .await;

                // Reset session timeout regardless of main query outcome.
                if let Some(h) = tmp_stmt_opt {
                    let set_result = c
                        .statement_set_sql_query(StatementSetSqlQueryRequest {
                            stmt_handle: Some(h),
                            query: "ALTER SESSION UNSET STATEMENT_TIMEOUT_IN_SECONDS".to_string(),
                        })
                        .await;
                    if set_result.is_ok() {
                        let _ = c
                            .statement_execute_query(StatementExecuteQueryRequest {
                                stmt_handle: Some(h),
                                bindings: None,
                            })
                            .await;
                    }
                    if let Err(e) = c
                        .statement_release(StatementReleaseRequest {
                            stmt_handle: Some(h),
                        })
                        .await
                    {
                        tracing::warn!(
                            "execute: failed to release timeout statement handle: {:?}",
                            e
                        );
                    }
                }

                main_result
            })?;

            tracing::info!("execute: Successfully executed statement");
            write_param_array_status(rows_processed_ptr, param_status_ptr, array_size, operation_ptr);
            update_numeric_settings(conn_handle, &mut conn.numeric_settings)?;

            let query_id = response.result.as_ref().map(|r| r.query_id.clone());

            let execute_state = create_execute_state(response, prepared)?;
            let is_zero_dml = matches!(
                &execute_state,
                StatementState::DmlExecuted {
                    rows_affected: 0,
                    ..
                }
            );
            set_state(stmt, execute_state);
            stmt.last_query_id = query_id.filter(|s| !s.is_empty());
            if is_zero_dml {
                return NoMoreDataSnafu.fail();
            }
            stmt.rows_returned = 0;
            Ok(())
        }
        ConnectionState::Disconnected => {
            tracing::error!("execute: connection is disconnected");
            DisconnectedSnafu.fail()
        }
    }
}

const STATEMENT_TYPE_ID_MANAGE_PATS: i64 = 0x6244;

fn is_ddl_statement(statement_type_id: i64) -> bool {
    tracing::debug!("is_ddl_statement: statement_type_id={}", statement_type_id);
    if statement_type_id == STATEMENT_TYPE_ID_MANAGE_PATS {
        return false;
    }
    (0x6000..0x7000).contains(&statement_type_id)
}

fn is_dml_statement_type(statement_type_id: Option<i64>) -> bool {
    statement_type_id.is_some_and(|id| (0x3000..0x4000).contains(&id))
}

fn set_state(stmt: &mut Statement, state: StatementState) {
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

fn create_execute_state(
    response: StatementExecuteQueryResponse,
    prepared: bool,
) -> OdbcResult<StatementState> {
    tracing::debug!("create_execute_state: response={:?}", response);
    let result = response.result.required("Execute result is required")?;
    let stream = result.stream.required("Stream is required")?;
    let reader = reader_from_protobuf_stream(stream)?;
    let rows_affected = result.rows_affected;
    if let Some(id) = result.statement_type_id {
        if is_ddl_statement(id) {
            return Ok(StatementState::DdlExecuted {
                schema: reader.schema(),
                prepared,
            });
        }
        if is_dml_statement_type(Some(id))
            && let Some(affected) = rows_affected
        {
            return Ok(StatementState::DmlExecuted {
                rows_affected: affected,
                schema: reader.schema(),
                prepared,
            });
        }
    }
    Ok(StatementState::QueryExecuted {
        reader,
        rows_affected,
        prepared,
    })
}

/// Per-parameter-set status constants written to `SQL_ATTR_PARAM_STATUS_PTR`.
const SQL_PARAM_SUCCESS: u16 = 0;
/// Written for rows skipped via `SQL_ATTR_PARAM_OPERATION_PTR`.
const SQL_PARAM_UNUSED: u16 = 7;
/// Value in the operation array that marks a row as ignored.
const SQL_PARAM_IGNORE_OP: u16 = 1;

/// After a successful execution with parameter arrays, write status values to the
/// IPD pointers set via `SQL_ATTR_PARAMS_PROCESSED_PTR` and `SQL_ATTR_PARAM_STATUS_PTR`.
///
/// `rows_processed_ptr` receives the count of parameter sets actually sent (excluding
/// ignored rows). `array_status_ptr` receives `SQL_PARAM_SUCCESS` for each processed
/// row and `SQL_PARAM_UNUSED` for rows skipped via `operation_ptr`.
fn write_param_array_status(
    rows_processed_ptr: *mut sql::ULen,
    array_status_ptr: *mut u16,
    array_size: usize,
    operation_ptr: *const u16,
) {
    let ignored = if operation_ptr.is_null() {
        0
    } else {
        (0..array_size)
            .filter(|&i| unsafe { *operation_ptr.add(i) } == SQL_PARAM_IGNORE_OP)
            .count()
    };

    if !rows_processed_ptr.is_null() {
        unsafe { *rows_processed_ptr = (array_size - ignored) as sql::ULen };
    }

    if !array_status_ptr.is_null() {
        for i in 0..array_size {
            let status = if !operation_ptr.is_null()
                && unsafe { *operation_ptr.add(i) } == SQL_PARAM_IGNORE_OP
            {
                SQL_PARAM_UNUSED
            } else {
                SQL_PARAM_SUCCESS
            };
            unsafe { *array_status_ptr.add(i) = status };
        }
    }
}

/// Build JSON query bindings from ODBC parameter bindings.
///
/// When `prepared` is true (SQLPrepare+SQLExecute flow), the IPD has server-
/// provided parameter count and we validate that the APD covers every marker.
/// When `prepared` is false (SQLExecDirect), the IPD only has records from
/// SQLBindParameter — we send whatever the APD has and let the server validate.
///
/// When `apd.array_size > 1`, emits an array binding JSON where each parameter's
/// `"value"` is a JSON array of values (one per row).
///
/// Returns `(bindings, json_owner)`. The caller **must** keep `json_owner` alive
/// until after the bindings have been consumed by `statement_execute_query`,
/// because `BinaryDataPtr` holds a raw pointer into the owned `String`.
fn apply_parameter_bindings(
    apd: &crate::api::ApdDescriptor,
    ipd: &crate::api::IpdDescriptor,
    prepared: bool,
) -> OdbcResult<(Option<QueryBindings>, Option<String>)> {
    if apd.records.is_empty() {
        if prepared {
            let ipd_count = ipd.desc_count() as usize;
            if ipd_count > 0 {
                return crate::api::error::CountFieldIncorrectSnafu {
                    reason: format!(
                        "parameter 1 is not bound (statement has {ipd_count} parameter markers)"
                    ),
                }
                .fail();
            }
        }
        return Ok((None, None));
    }

    let ipd_count = ipd.desc_count() as usize;
    if ipd_count == 0 && !prepared {
        return Ok((None, None));
    }

    if prepared {
        for i in 1..=ipd_count {
            if !apd.records.contains_key(&(i as u16)) {
                return crate::api::error::CountFieldIncorrectSnafu {
                    reason: format!(
                        "parameter {i} is not bound (statement has {ipd_count} parameter markers)"
                    ),
                }
                .fail();
            }
        }
    }

    let array_size = apd.array_size;
    tracing::info!(
        "apply_parameter_bindings: Found {} bound parameters, array_size={}",
        apd.records.len(),
        array_size,
    );

    let json_string = if array_size > 1 {
        // Build ParameterBinding map for array execution.
        let max_key = apd.desc_count().max(ipd.desc_count());
        let mut parameter_bindings = std::collections::HashMap::new();
        for param_num in 1..=max_key {
            if let (Some(apd_rec), Some(ipd_rec)) =
                (apd.records.get(&param_num), ipd.records.get(&param_num))
            {
                parameter_bindings.insert(param_num, ParameterBinding::from_apd_ipd(apd_rec, ipd_rec));
            }
        }
        let bind_type = apd.bind_type as usize;
        let bind_offset = if apd.bind_offset_ptr.is_null() {
            0
        } else {
            unsafe { *apd.bind_offset_ptr }
        };
        let operation_ptr = apd.array_status_ptr as *const u16;
        odbc_bindings_to_json_array(&parameter_bindings, array_size, bind_type, bind_offset, operation_ptr)
            .context(JsonBindingSnafu {})?
    } else {
        odbc_bindings_to_json(apd, ipd).context(JsonBindingSnafu {})?
    };

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

    if parameter_number == 0 {
        tracing::error!("bind_parameter: parameter_number cannot be 0");
        return InvalidParameterNumberSnafu.fail();
    }

    let direction = ParamDirection::try_from(raw_input_output_type)?;

    let value_type = CDataType::try_from(raw_value_type)?;

    let sql_type = SqlType::try_from(raw_parameter_type)?;
    let parameter_type: sql::SqlDataType = sql_type.into();

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

    let stmt = stmt_from_handle(statement_handle);

    stmt.apd.records.insert(
        parameter_number,
        ApdRecord {
            value_type,
            data_ptr: parameter_value_ptr,
            buffer_length,
            str_len_or_ind_ptr,
        },
    );

    stmt.ipd.records.insert(
        parameter_number,
        IpdRecord {
            sql_data_type: parameter_type,
            column_size,
            decimal_digits,
            direction: raw_input_output_type,
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
    let stmt = stmt_from_handle(statement_handle);

    match option {
        FreeStmtOption::Close => {
            tracing::info!("free_stmt: Closing cursor");
            let transition = match stmt.state.as_ref() {
                StatementState::Created | StatementState::Prepared { .. } => None,
                StatementState::QueryExecuted {
                    reader,
                    prepared: true,
                    ..
                }
                | StatementState::Fetching {
                    reader,
                    prepared: true,
                    ..
                } => {
                    let schema = reader.schema();
                    let desc_count = schema.fields().len() as sql::SmallInt;
                    Some((StatementState::Prepared { schema }, desc_count))
                }
                StatementState::DdlExecuted {
                    schema,
                    prepared: true,
                }
                | StatementState::DmlExecuted {
                    schema,
                    prepared: true,
                    ..
                }
                | StatementState::Done {
                    schema,
                    prepared: true,
                } => {
                    let desc_count = schema.fields().len() as sql::SmallInt;
                    Some((
                        StatementState::Prepared {
                            schema: schema.clone(),
                        },
                        desc_count,
                    ))
                }
                _ => Some((StatementState::Created, 0)),
            };
            if let Some((state, desc_count)) = transition {
                stmt.state.set(state);
                stmt.ird.desc_count = desc_count;
                stmt.get_data_state = None;
                stmt.used_extended_fetch = false;
            }
        }
        FreeStmtOption::Unbind => {
            tracing::info!("free_stmt: Unbinding all columns");
            stmt.ard.unbind_all();
        }
        FreeStmtOption::ResetParams => {
            tracing::info!("free_stmt: Resetting all parameter bindings (APD)");
            stmt.apd.clear();
        }
    }

    Ok(())
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

    let stmt = stmt_from_handle(statement_handle);

    if matches!(stmt.state.as_ref(), StatementState::Created) {
        return StatementNotExecutedSnafu.fail();
    }

    let count = stmt.ipd.desc_count();

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

    let stmt = stmt_from_handle(statement_handle);

    let allowed = matches!(
        stmt.state.as_ref(),
        StatementState::Prepared { .. }
            | StatementState::DdlExecuted { prepared: true, .. }
            | StatementState::DmlExecuted { prepared: true, .. }
            | StatementState::Done { prepared: true, .. }
    );
    if !allowed {
        return StatementNotExecutedSnafu.fail();
    }
    let ipd_rec = stmt.ipd.records.get(&parameter_number).ok_or_else(|| {
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

    let stmt = stmt_from_handle(statement_handle);

    // Per ODBC specification, if target_value_ptr is null, unbind the column
    if target_value_ptr.is_null() {
        tracing::debug!("bind_col: unbinding column {}", column_number);
        stmt.ard.bindings.remove(&column_number);
    } else {
        stmt.ard.bindings.insert(
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
    let stmt = stmt_from_handle(statement_handle);

    match attr {
        StmtAttr::CursorType => {
            let raw = value_ptr as sql::ULen;
            let requested = CursorType::try_from(raw)?;
            tracing::debug!("set_stmt_attr: CursorType requested = {requested:?}");
            if requested != CursorType::ForwardOnly {
                stmt.cursor_type = CursorType::ForwardOnly;
                warnings.push(Warning::OptionValueChanged);
            } else {
                stmt.cursor_type = CursorType::ForwardOnly;
            }
            Ok(())
        }
        StmtAttr::MaxLength => {
            let length = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: MaxLength = {}", length);
            stmt.max_length = length;
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
            stmt.ard.array_size = effective_size;
            Ok(())
        }
        StmtAttr::RowStatusPtr => {
            let ptr = value_ptr as *mut u16;
            tracing::debug!("set_stmt_attr: RowStatusPtr = {:?}", ptr);
            stmt.ird.array_status_ptr = ptr;
            Ok(())
        }
        StmtAttr::RowsFetchedPtr => {
            let ptr = value_ptr as *mut sql::ULen;
            tracing::debug!("set_stmt_attr: RowsFetchedPtr = {:?}", ptr);
            stmt.ird.rows_processed_ptr = ptr;
            Ok(())
        }
        StmtAttr::RowBindType => {
            let raw_bind_type = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: RowBindType (raw) = {}", raw_bind_type);
            stmt.ard.bind_type = raw_bind_type;
            Ok(())
        }
        StmtAttr::RowBindOffsetPtr => {
            let ptr = value_ptr as *mut sql::Len;
            tracing::debug!("set_stmt_attr: RowBindOffsetPtr = {:?}", ptr);
            stmt.ard.bind_offset_ptr = ptr;
            Ok(())
        }
        StmtAttr::MetadataId => {
            let val = value_ptr as sql::ULen;
            match val {
                0 => {
                    stmt.metadata_id = false;
                    Ok(())
                }
                1 => {
                    stmt.metadata_id = true;
                    Ok(())
                }
                _ => InvalidAttributeValueSnafu {
                    attribute,
                    value: val as i64,
                }
                .fail(),
            }
        }
        StmtAttr::SnowflakeLastQueryId => {
            crate::api::error::ReadOnlyAttributeSnafu { attribute }.fail()
        }
        StmtAttr::ImpRowDesc | StmtAttr::ImpParamDesc => {
            crate::api::error::ReadOnlyAttributeSnafu { attribute }.fail()
        }
        StmtAttr::QueryTimeout => {
            let val = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: QueryTimeout = {}", val);
            stmt.query_timeout = val;
            Ok(())
        }
        StmtAttr::MaxRows => {
            let val = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: MaxRows = {}", val);
            stmt.max_rows = val;
            Ok(())
        }
        StmtAttr::Noscan => {
            let val = value_ptr as sql::ULen;
            match val {
                0 | 1 => {
                    stmt.noscan = val;
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
            // 24000 if a cursor is open
            if matches!(
                stmt.state.as_ref(),
                StatementState::QueryExecuted { .. } | StatementState::Fetching { .. }
            ) {
                tracing::error!("set_stmt_attr: Concurrency cannot be set while cursor is open");
                return InvalidCursorStateSnafu.fail();
            }
            let val = value_ptr as sql::ULen;
            match val {
                1 => {
                    // SQL_CONCUR_READ_ONLY — accepted directly
                    stmt.concurrency = val;
                    Ok(())
                }
                2..=4 => {
                    // SQL_CONCUR_LOCK / SQL_CONCUR_ROWVER / SQL_CONCUR_VALUES
                    // Snowflake cursors are always read-only; substitute and warn
                    stmt.concurrency = 1; // SQL_CONCUR_READ_ONLY
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
            let val = value_ptr as sql::ULen;
            match val {
                0 => {
                    // SQL_NONSCROLLABLE — accepted
                    stmt.cursor_scrollable = val;
                    Ok(())
                }
                1 => {
                    // SQL_SCROLLABLE — substitute with SQL_NONSCROLLABLE + 01S02
                    stmt.cursor_scrollable = 0;
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
            let val = value_ptr as sql::ULen;
            match val {
                0 => {
                    // SQL_UNSPECIFIED — accepted
                    stmt.cursor_sensitivity = val;
                    Ok(())
                }
                1 | 2 => {
                    // SQL_INSENSITIVE / SQL_SENSITIVE — substitute with SQL_UNSPECIFIED + 01S02
                    stmt.cursor_sensitivity = 0;
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
                _ => {
                    // SQL_TRUE and other values — HYC00 (optional feature not implemented)
                    tracing::debug!("set_stmt_attr: EnableAutoIpd = SQL_TRUE is not supported");
                    UnsupportedFeatureSnafu.fail()
                }
            }
        }
        StmtAttr::KeysetSize => {
            let val = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: KeysetSize = {}", val);
            stmt.keyset_size = val;
            Ok(())
        }
        StmtAttr::SimulateCursor => {
            let val = value_ptr as sql::ULen;
            match val {
                0 => {
                    // SQL_SC_NON_UNIQUE — accepted
                    stmt.simulate_cursor = val;
                    Ok(())
                }
                _ => {
                    // Other values — substitute with SQL_SC_NON_UNIQUE + 01S02
                    stmt.simulate_cursor = 0;
                    warnings.push(Warning::OptionValueChanged);
                    Ok(())
                }
            }
        }
        StmtAttr::RetrieveData => {
            let val = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: RetrieveData = {}", val);
            stmt.retrieve_data = val;
            Ok(())
        }
        StmtAttr::ParamsetSize => {
            let size = value_ptr as usize;
            let effective = if size == 0 {
                tracing::warn!("set_stmt_attr: ParamsetSize value 0 is invalid; coercing to 1");
                1
            } else {
                size
            };
            tracing::debug!("set_stmt_attr: ParamsetSize = {}", effective);
            stmt.apd.array_size = effective;
            Ok(())
        }
        StmtAttr::ParamBindType => {
            let val = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: ParamBindType = {}", val);
            stmt.apd.bind_type = val;
            Ok(())
        }
        StmtAttr::ParamBindOffsetPtr => {
            let ptr = value_ptr as *mut sql::Len;
            tracing::debug!("set_stmt_attr: ParamBindOffsetPtr = {:?}", ptr);
            stmt.apd.bind_offset_ptr = ptr;
            Ok(())
        }
        StmtAttr::ParamStatusPtr => {
            let ptr = value_ptr as *mut u16;
            tracing::debug!("set_stmt_attr: ParamStatusPtr = {:?}", ptr);
            stmt.ipd.array_status_ptr = ptr;
            Ok(())
        }
        StmtAttr::ParamsProcessedPtr => {
            let ptr = value_ptr as *mut sql::ULen;
            tracing::debug!("set_stmt_attr: ParamsProcessedPtr = {:?}", ptr);
            stmt.ipd.rows_processed_ptr = ptr;
            Ok(())
        }
        StmtAttr::ParamOperationPtr => {
            let ptr = value_ptr as *mut u16;
            tracing::debug!("set_stmt_attr: ParamOperationPtr = {:?}", ptr);
            stmt.apd.array_status_ptr = ptr;
            Ok(())
        }
        StmtAttr::SnowflakeLastQueryId => {
            // Read-only attribute — cannot be set
            crate::api::error::ReadOnlyAttributeSnafu {
                attribute: attr as i32,
            }
            .fail()
        }
        StmtAttr::SnowflakeMultiStatementCount => {
            let val = value_ptr as i64;
            if val < -1 {
                return InvalidAttributeValueSnafu {
                    attribute: attr as i32,
                    value: val,
                }
                .fail();
            }
            stmt.multi_statement_count = val as i16;
            Ok(())
        }
        _ => {
            tracing::warn!("set_stmt_attr: unsupported attribute {:?}", attr);
            crate::api::error::UnsupportedAttributeSnafu { attribute }.fail()
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
    let stmt = stmt_from_handle(statement_handle);

    match attr {
        StmtAttr::CursorType => {
            unsafe {
                std::ptr::write_unaligned(
                    value_ptr as *mut sql::ULen,
                    stmt.cursor_type as sql::ULen,
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
                *(value_ptr as *mut sql::ULen) = stmt.max_length;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::AppRowDesc => {
            let ard_ptr = &mut stmt.ard as *mut crate::api::ArdDescriptor as sql::Handle;
            unsafe {
                *(value_ptr as *mut sql::Handle) = ard_ptr;
            }
            Ok(())
        }
        StmtAttr::ImpRowDesc => {
            let ird_ptr = &mut stmt.ird as *mut crate::api::IrdDescriptor as sql::Handle;
            unsafe {
                *(value_ptr as *mut sql::Handle) = ird_ptr;
            }
            Ok(())
        }
        StmtAttr::AppParamDesc => {
            let apd_ptr = &mut stmt.apd as *mut crate::api::ApdDescriptor as sql::Handle;
            unsafe {
                *(value_ptr as *mut sql::Handle) = apd_ptr;
            }
            Ok(())
        }
        StmtAttr::ImpParamDesc => {
            let ipd_ptr = &mut stmt.ipd as *mut crate::api::IpdDescriptor as sql::Handle;
            unsafe {
                *(value_ptr as *mut sql::Handle) = ipd_ptr;
            }
            Ok(())
        }
        StmtAttr::RowArraySize => {
            unsafe {
                *(value_ptr as *mut sql::ULen) = stmt.ard.array_size as sql::ULen;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::RowStatusPtr => {
            unsafe {
                *(value_ptr as *mut *mut u16) = stmt.ird.array_status_ptr;
            }
            Ok(())
        }
        StmtAttr::RowsFetchedPtr => {
            unsafe {
                *(value_ptr as *mut *mut sql::ULen) = stmt.ird.rows_processed_ptr;
            }
            Ok(())
        }
        StmtAttr::RowBindType => {
            unsafe {
                *(value_ptr as *mut sql::ULen) = stmt.ard.bind_type;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::RowBindOffsetPtr => {
            unsafe {
                *(value_ptr as *mut *mut sql::Len) = stmt.ard.bind_offset_ptr;
            }
            Ok(())
        }
        StmtAttr::MetadataId => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::ULen) = stmt.metadata_id as sql::ULen;
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
            let query_id = stmt.last_query_id.as_deref().unwrap_or("");
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
                unsafe { *(value_ptr as *mut sql::ULen) = stmt.query_timeout };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::MaxRows => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = stmt.max_rows };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::Noscan => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = stmt.noscan };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::Concurrency => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = stmt.concurrency };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::CursorScrollable => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = stmt.cursor_scrollable };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::CursorSensitivity => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = stmt.cursor_sensitivity };
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
                unsafe { *(value_ptr as *mut sql::ULen) = stmt.keyset_size };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::SimulateCursor => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = stmt.simulate_cursor };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::RetrieveData => {
            if !value_ptr.is_null() {
                unsafe { *(value_ptr as *mut sql::ULen) = stmt.retrieve_data };
            }
            if !string_length_ptr.is_null() {
                unsafe { *string_length_ptr = size_of::<sql::ULen>() as sql::Integer };
            }
            Ok(())
        }
        StmtAttr::ParamsetSize => {
            unsafe {
                *(value_ptr as *mut sql::ULen) = stmt.apd.array_size as sql::ULen;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::ParamBindType => {
            unsafe {
                *(value_ptr as *mut sql::ULen) = stmt.apd.bind_type;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::ParamBindOffsetPtr => {
            unsafe {
                *(value_ptr as *mut *mut sql::Len) = stmt.apd.bind_offset_ptr;
            }
            Ok(())
        }
        StmtAttr::ParamStatusPtr => {
            unsafe {
                *(value_ptr as *mut *mut u16) = stmt.ipd.array_status_ptr;
            }
            Ok(())
        }
        StmtAttr::ParamsProcessedPtr => {
            unsafe {
                *(value_ptr as *mut *mut sql::ULen) = stmt.ipd.rows_processed_ptr;
            }
            Ok(())
        }
        StmtAttr::ParamOperationPtr => {
            unsafe {
                *(value_ptr as *mut *mut u16) = stmt.apd.array_status_ptr;
            }
            Ok(())
        }
        StmtAttr::SnowflakeLastQueryId => {
            let id = stmt.last_query_id.as_deref().unwrap_or("");
            write_string_bytes_i32::<E>(
                id,
                value_ptr as *mut E::Char,
                buffer_length,
                string_length_ptr,
                None,
            );
            Ok(())
        }
        StmtAttr::SnowflakeMultiStatementCount => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::Integer) = stmt.multi_statement_count as sql::Integer;
                    if !string_length_ptr.is_null() {
                        *string_length_ptr = size_of::<sql::Integer>() as sql::Integer;
                    }
                }
            }
            Ok(())
        }
        _ => {
            tracing::warn!("get_stmt_attr: unsupported attribute {:?}", attr);
            crate::api::error::UnsupportedAttributeSnafu { attribute }.fail()
        }
    }
}

/// Cancel processing on a statement (SQLCancel).
///
/// Cancels the `CancellationToken` stored on the `Statement` struct.
/// Called from `SQLCancel` in `c_api.rs`, which may be invoked from a
/// different thread. Per ODBC 3.5 spec, cross-thread `SQLCancel` does
/// not clear or post diagnostic records.
///
/// NOTE: Cross-thread calls create `&mut Statement` via `stmt_from_handle`
/// concurrently with the executing thread — the same pre-existing aliasing
/// pattern used by every C API entry point. A future handle manager will
/// introduce proper interior mutability to eliminate this UB.
pub fn cancel(statement_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!("cancel: statement_handle={:?}", statement_handle);

    // TODO(SNOW-3258918): Cancel async execution.
    // Blocked by: SQLSetStmtAttr does not support SQL_ATTR_ASYNC_ENABLE.

    // TODO(SNOW-3258919): Cancel data-at-execution (SQL_NEED_DATA).
    // Blocked by: SQLParamData and SQLPutData are not implemented/exported.

    // TODO(SNOW-3258922): Cancel execution on another thread.
    // Blocked by: no server-side cancel RPC. When implemented,
    // cancelling the token resolves the cancelled() future observed
    // by the executing thread's tokio::select!, aborting the in-flight RPC.

    let stmt = stmt_from_handle(statement_handle);
    stmt.cancel_token.cancel();
    Ok(())
}
