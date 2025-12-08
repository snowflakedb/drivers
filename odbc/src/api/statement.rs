use crate::api::api_utils::cstr_to_string;
use crate::api::error::{
    ArrowArrayStreamReaderCreationSnafu, ArrowBindingSnafu, DisconnectedSnafu,
    InvalidParameterNumberSnafu, OdbcError, ParameterBindingSnafu, Required,
    StatementNotExecutedSnafu, UnsupportedParameterDirectionSnafu,
};
use crate::api::{
    ConnectionState, DataAtExecMode, DataAtExecState, OdbcResult, ParamBindType, ParameterBinding,
    Statement, StatementState, TimestampLtzFormat, TimestampType,
    connection::refresh_current_catalog, stmt_from_handle,
};
use crate::cdata_types::CDataType;
use crate::read_arrow::set_read_session_timezone;
use crate::write_arrow::{ArrowBindingError, odbc_bindings_to_arrow_bindings};
use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use odbc_sys as sql;
use sf_core::apis::database_driver_v1;
use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::{
    ArrowArrayPtr, ArrowSchemaPtr, StatementBindRequest, StatementExecuteQueryRequest,
    StatementExecuteQueryResponse, StatementNewRequest, StatementPrepareRequest,
    StatementSetSqlQueryRequest,
};
use snafu::{OptionExt, ResultExt};
use std::collections::VecDeque;
use std::io::Write;
use tracing;

const SQL_PARAM_SUCCESS: sql::USmallInt = 0;
const SQL_PARAM_ERROR: sql::USmallInt = 5;
const SQL_DATA_AT_EXEC: sql::Len = -2;
const SQL_LEN_DATA_AT_EXEC_OFFSET: sql::Len = -100;
const SQL_NULL_DATA: sql::Len = -1;

pub enum ParamDataStatus {
    NeedData(sql::Pointer),
    Success,
}

fn binding_requires_data_at_exec(binding: &ParameterBinding) -> bool {
    if binding.str_len_or_ind_ptr.is_null() {
        return false;
    }
    let indicator = unsafe { *binding.str_len_or_ind_ptr };
    indicator == SQL_DATA_AT_EXEC || indicator <= SQL_LEN_DATA_AT_EXEC_OFFSET
}

fn has_data_at_exec_bindings(stmt: &Statement) -> bool {
    stmt.parameter_bindings
        .values()
        .any(binding_requires_data_at_exec)
}

fn start_data_at_exec(stmt: &mut Statement, mode: DataAtExecMode) -> OdbcResult<()> {
    let mut keys: Vec<u16> = stmt.parameter_bindings.keys().copied().collect();
    keys.sort_unstable();
    let mut pending = VecDeque::new();
    for key in keys {
        if let Some(binding) = stmt.parameter_bindings.get(&key) {
            if binding_requires_data_at_exec(binding) {
                pending.push_back(key);
            }
        }
    }
    if pending.is_empty() {
        return Ok(());
    }
    stmt.data_at_exec_state = Some(DataAtExecState {
        mode,
        pending_params: pending,
        current_param: None,
        awaiting_data: false,
        buffers: std::collections::HashMap::new(),
        null_params: std::collections::HashSet::new(),
    });
    Ok(())
}

fn protobuf_from_ffi_arrow_array(raw: *mut FFI_ArrowArray) -> ArrowArrayPtr {
    let len = std::mem::size_of::<*mut FFI_ArrowArray>();
    let buf_ptr = std::ptr::addr_of!(raw) as *const u8;
    let slice = unsafe { std::slice::from_raw_parts(buf_ptr, len) };
    let vec = slice.to_vec();
    ArrowArrayPtr { value: vec }
}

fn protobuf_from_ffi_arrow_schema(raw: *mut FFI_ArrowSchema) -> ArrowSchemaPtr {
    let len = std::mem::size_of::<*mut FFI_ArrowSchema>();
    let buf_ptr = std::ptr::addr_of!(raw) as *const u8;
    let slice = unsafe { std::slice::from_raw_parts(buf_ptr, len) };
    let vec = slice.to_vec();
    ArrowSchemaPtr { value: vec }
}

/// Execute a SQL statement directly
pub fn exec_direct(
    statement_handle: sql::Handle,
    statement_text: *const sql::Char,
    text_length: sql::Integer,
) -> OdbcResult<()> {
    let stmt = stmt_from_handle(statement_handle);
    tracing::debug!("exec_direct: statement_handle={:?}", statement_handle);

    match &mut stmt.conn.state {
        ConnectionState::Connected {
            db_handle: _,
            conn_handle: _,
        } => {
            stmt.is_prepared = false;
            let query = cstr_to_string(statement_text, text_length)?;

            DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
                stmt_handle: Some(stmt.stmt_handle),
                query: query.clone(),
            })?;

            if has_data_at_exec_bindings(stmt) {
                start_data_at_exec(
                    stmt,
                    DataAtExecMode::ExecDirect {
                        query_text: query.clone(),
                    },
                )?;
                return Err(OdbcError::NeedData {
                    location: snafu::location!(),
                });
            }

            let response =
                DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt.stmt_handle),
                    describe_only: false,
                })?;
            apply_execution_response(stmt, response, Some(&query))
        }
        ConnectionState::Disconnected => {
            tracing::error!("exec_direct: connection is disconnected");
            DisconnectedSnafu.fail()
        }
    }
}

/// Prepare a SQL statement
pub fn prepare(
    statement_handle: sql::Handle,
    statement_text: *const sql::Char,
    text_length: sql::Integer,
) -> OdbcResult<()> {
    tracing::debug!("prepare: statement_handle={:?}", statement_handle);
    let stmt = stmt_from_handle(statement_handle);

    match &mut stmt.conn.state {
        ConnectionState::Connected {
            db_handle: _,
            conn_handle: _,
        } => {
            let query = cstr_to_string(statement_text, text_length)?;
            tracing::debug!("prepare: query = {}", query);

            // Store the query for SQLNumParams
            stmt.prepared_query = Some(query.clone());
            stmt.is_prepared = true;

            // Set the SQL query for the statement
            DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
                stmt_handle: Some(stmt.stmt_handle),
                query,
            })?;

            // Call the prepare method on the statement
            DatabaseDriverClient::statement_prepare(StatementPrepareRequest {
                stmt_handle: Some(stmt.stmt_handle),
            })?;

            // Validate the statement without executing it by running a describe-only request
            let describe_response =
                DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt.stmt_handle),
                    describe_only: true,
                })?;

            if let Some(result) = describe_response.result {
                if let Some(stream_ptr) = result.stream {
                    let raw_stream: *mut FFI_ArrowArrayStream = stream_ptr.into();
                    let stream = unsafe { FFI_ArrowArrayStream::from_raw(raw_stream) };
                    match ArrowArrayStreamReader::try_new(stream) {
                        Ok(mut reader) => {
                            if let Some(batch_result) = reader.next() {
                                match batch_result {
                                    Ok(batch) => {
                                        let schema = batch.schema();
                                        tracing::debug!(
                                            "prepare: cached describe-only schema with {} columns",
                                            schema.fields().len()
                                        );
                                        stmt.cached_schema = Some(schema);
                                    }
                                    Err(err) => {
                                        tracing::warn!(
                                            "prepare: failed to read describe-only batch: {err:?}"
                                        );
                                    }
                                }
                            } else {
                                tracing::warn!("prepare: describe-only stream returned no batches");
                            }
                        }
                        Err(err) => {
                            tracing::warn!(
                                "prepare: failed to create reader for describe-only stream: {err:?}"
                            );
                        }
                    }
                } else {
                    tracing::warn!("prepare: describe-only response missing Arrow stream");
                }
            } else {
                tracing::warn!("prepare: describe-only response missing ExecuteResult");
            }

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
    let prepared_query_text = stmt.prepared_query.clone();

    match &mut stmt.conn.state {
        ConnectionState::Connected {
            db_handle: _,
            conn_handle: _,
        } => {
            let mut rows_processed = 0usize;

            if has_data_at_exec_bindings(stmt) {
                if stmt.paramset_size > 1 {
                    return ParameterBindingSnafu {
                        parameters: "Data-at-exec is not supported with array bindings".to_string(),
                    }
                    .fail();
                }
                start_data_at_exec(stmt, DataAtExecMode::ExecutePrepared)?;
                return Err(OdbcError::NeedData {
                    location: snafu::location!(),
                });
            }

            let response = if stmt.parameter_bindings.is_empty() {
                tracing::info!("execute: no bound parameters; executing once");
                let response =
                    DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
                        stmt_handle: Some(stmt.stmt_handle),
                        describe_only: false,
                    })?;
                rows_processed = 1;
                response
            } else if stmt.paramset_size <= 1 {
                if bindings_require_timestamp_tz(&stmt.parameter_bindings) {
                    ensure_client_timestamp_type_mapping(stmt, TimestampType::Tz)?;
                }
                crate::write_arrow::set_timestamp_type_mapping(stmt.conn.timestamp_type_mapping);
                let mut filtered_bindings_storage: Option<
                    std::collections::HashMap<u16, ParameterBinding>,
                > = None;
                let bindings_to_use = if let Some(query) = prepared_query_text.as_deref() {
                    let expected_params = query.matches('?').count();
                    if expected_params > 0 {
                        let needs_filter = stmt
                            .parameter_bindings
                            .keys()
                            .any(|&param_num| (param_num as usize) > expected_params);
                        if needs_filter {
                            let filtered: std::collections::HashMap<u16, ParameterBinding> = stmt
                                .parameter_bindings
                                .iter()
                                .filter(|(param_num, _)| (**param_num as usize) <= expected_params)
                                .map(|(&param_num, binding)| (param_num, binding.clone()))
                                .collect();
                            filtered_bindings_storage = Some(filtered);
                            filtered_bindings_storage.as_ref().unwrap()
                        } else {
                            &stmt.parameter_bindings
                        }
                    } else {
                        &stmt.parameter_bindings
                    }
                } else {
                    &stmt.parameter_bindings
                };
                let response = bind_and_execute(stmt, bindings_to_use)?;
                rows_processed = 1;
                set_param_status(&stmt, 0, SQL_PARAM_SUCCESS);
                response
            } else {
                // Array binding: send all rows in a single batch
                tracing::info!(
                    "execute: binding {} parameter sets in batch",
                    stmt.paramset_size
                );

                if bindings_require_timestamp_tz(&stmt.parameter_bindings) {
                    ensure_client_timestamp_type_mapping(stmt, TimestampType::Tz)?;
                }

                // Set the session timezone for parameter binding
                crate::write_arrow::set_session_timezone(stmt.session_timezone.clone());
                crate::write_arrow::set_timestamp_type_mapping(stmt.conn.timestamp_type_mapping);

                // Build Arrow arrays with all rows
                let (schema, array) = crate::write_arrow::odbc_bindings_to_arrow_bindings_batch(
                    &stmt.parameter_bindings,
                    stmt.paramset_size,
                )
                .context(ArrowBindingSnafu {})?;

                eprintln!(
                    "DEBUG execute: Built Arrow array with paramset_size={}",
                    stmt.paramset_size
                );

                // Bind all parameters in one call
                DatabaseDriverClient::statement_bind(StatementBindRequest {
                    stmt_handle: Some(stmt.stmt_handle),
                    schema: Some(protobuf_from_ffi_arrow_schema(Box::into_raw(schema))),
                    array: Some(protobuf_from_ffi_arrow_array(Box::into_raw(array))),
                })?;

                tracing::info!("Successfully bound {} parameter sets", stmt.paramset_size);

                // Execute the statement
                let response =
                    DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
                        stmt_handle: Some(stmt.stmt_handle),
                        describe_only: false,
                    })?;

                // Mark all rows as successful
                rows_processed = stmt.paramset_size;
                for row_idx in 0..stmt.paramset_size {
                    set_param_status(&stmt, row_idx, SQL_PARAM_SUCCESS);
                }

                response
            };

            if let Some(ptr) = stmt.params_processed_ptr {
                unsafe { *ptr = rows_processed as sql::ULen }
            }
            tracing::info!("execute: Successfully executed statement");
            apply_execution_response(stmt, response, prepared_query_text.as_deref())
        }
        ConnectionState::Disconnected => {
            tracing::error!("execute: connection is disconnected");
            DisconnectedSnafu.fail()
        }
    }
}

/// Returns (state, has_cursor) where has_cursor is true for SELECT queries, false for DDL
fn create_execute_state(
    response: StatementExecuteQueryResponse,
) -> OdbcResult<(StatementState, bool, Option<arrow::datatypes::SchemaRef>)> {
    let result = response.result.required("Execute result is required")?;
    let stream_ptr: *mut FFI_ArrowArrayStream =
        result.stream.required("Stream is required")?.into();
    let stream = unsafe { FFI_ArrowArrayStream::from_raw(stream_ptr) };
    let mut reader =
        ArrowArrayStreamReader::try_new(stream).context(ArrowArrayStreamReaderCreationSnafu {})?;

    let rows_affected = result.rows_affected;

    // Peek at the first batch to get the schema
    // We need to do this because ArrowArrayStreamReader doesn't expose schema() publicly
    match reader.next() {
        Some(batch_result) => {
            let batch = batch_result.context(ArrowArrayStreamReaderCreationSnafu {})?;
            let schema = batch.schema();

            let field_names: Vec<String> = schema
                .fields()
                .iter()
                .map(|f| f.name().to_string())
                .collect();
            tracing::debug!(
                "create_execute_state: first batch field_names={:?}",
                field_names
            );

            // Check if this is a DDL statement (typically has a single "status" column)
            // DDL statements shouldn't have a cursor, so we go to Done state
            let is_ddl = schema.fields().len() == 1
                && schema
                    .fields()
                    .first()
                    .map(|f| f.name() == "status")
                    .unwrap_or(false);

            if is_ddl {
                tracing::debug!("create_execute_state: DDL detected, going to Done state");
                return Ok((StatementState::Done, false, Some(schema)));
            }

            // Put the batch back by transitioning to Fetching state immediately
            // Note: rows_affected is stored in last_rows_affected by the caller
            return Ok((
                StatementState::Fetching {
                    reader,
                    record_batch: batch,
                    batch_idx: 0,
                },
                true,
                Some(schema),
            )); // SELECT has cursor
        }
        None => {
            // Empty result set - we still need a schema
            // Create a minimal schema
            let schema = std::sync::Arc::new(arrow::datatypes::Schema::empty());
            Ok((
                StatementState::Executed {
                    reader,
                    schema: schema.clone(),
                    rows_affected,
                },
                true,
                Some(schema),
            ))
        }
    }
}

fn bind_and_execute(
    stmt: &Statement,
    bindings: &std::collections::HashMap<u16, ParameterBinding>,
) -> OdbcResult<StatementExecuteQueryResponse> {
    crate::write_arrow::set_timestamp_type_mapping(stmt.conn.timestamp_type_mapping);
    if !bindings.is_empty() {
        tracing::info!("execute: Found {} bound parameters", bindings.len());
        // Set the session timezone for parameter binding
        crate::write_arrow::set_session_timezone(stmt.session_timezone.clone());
        let (schema, array) =
            odbc_bindings_to_arrow_bindings(bindings).context(ArrowBindingSnafu {})?;

        DatabaseDriverClient::statement_bind(StatementBindRequest {
            stmt_handle: Some(stmt.stmt_handle),
            schema: Some(protobuf_from_ffi_arrow_schema(Box::into_raw(schema))),
            array: Some(protobuf_from_ffi_arrow_array(Box::into_raw(array))),
        })?;

        tracing::info!("Successfully bound parameters");
    }

    Ok(DatabaseDriverClient::statement_execute_query(
        StatementExecuteQueryRequest {
            stmt_handle: Some(stmt.stmt_handle),
            describe_only: false,
        },
    )?)
}

fn apply_execution_response(
    stmt: &mut Statement,
    response: StatementExecuteQueryResponse,
    query_text: Option<&str>,
) -> OdbcResult<()> {
    if let Some(result) = &response.result {
        stmt.last_query_id = if result.query_id.is_empty() {
            None
        } else {
            Some(result.query_id.clone())
        };
        stmt.child_result_ids = result.child_result_ids.clone();
        stmt.current_result_index = 0;

        if !stmt.child_result_ids.is_empty() {
            let first_child_id = stmt.child_result_ids[0].clone();
            let handle = sf_core::handle_manager::Handle {
                id: stmt.stmt_handle.id as u64,
                magic: stmt.stmt_handle.magic as u64,
            };
            match sf_core::apis::database_driver_v1::fetch_child_result(handle, &first_child_id) {
                Ok(child_result) => {
                    stmt.last_rows_affected = child_result.rows_affected;
                }
                Err(e) => {
                    tracing::warn!(
                        "apply_execution_response: failed to fetch first child result: {e}"
                    );
                    stmt.last_rows_affected = result.rows_affected;
                }
            }
        } else {
            stmt.last_rows_affected = result.rows_affected;
        }
    }

    let (new_state, has_cursor, cached_schema) = create_execute_state(response)?;
    let tz_override = query_text.and_then(|query| update_session_settings_from_query(stmt, query));
    refresh_session_timezone(stmt);
    if let Some(tz) = tz_override {
        stmt.conn.session_timezone = Some(tz.clone());
        stmt.session_timezone = Some(tz.clone());
        crate::write_arrow::set_session_timezone(stmt.session_timezone.clone());
        set_read_session_timezone(stmt.session_timezone.clone());
    }
    if let StatementState::Executed { rows_affected, .. } = &new_state {
        stmt.last_rows_affected = *rows_affected;
    }
    stmt.state = new_state.into();
    stmt.cached_schema = cached_schema;
    stmt.current_row = 0;
    stmt.has_cursor = has_cursor;

    if stmt.conn.use_current_catalog {
        let handle = match &stmt.conn.state {
            ConnectionState::Connected { conn_handle, .. } => Some(*conn_handle),
            _ => None,
        };
        if let Some(conn_handle) = handle {
            if let Err(err) = refresh_current_catalog(&mut stmt.conn, &conn_handle) {
                tracing::warn!(
                    "apply_execution_response: failed to refresh current catalog: {err}"
                );
            }
        }
    }

    Ok(())
}

fn finalize_current_data_param(
    stmt: &mut Statement,
    state: &mut DataAtExecState,
) -> OdbcResult<()> {
    let param_num = state
        .current_param
        .ok_or_else(|| StatementNotExecutedSnafu.build())?;
    let binding = stmt
        .parameter_bindings
        .get_mut(&param_num)
        .context(ParameterBindingSnafu {
            parameters: format!("Parameter {param_num} not found"),
        })?;

    if state.null_params.remove(&param_num) {
        binding.owned_buffer = None;
        binding.parameter_value_ptr = std::ptr::null_mut();
        binding.buffer_length = 0;
        if !binding.str_len_or_ind_ptr.is_null() {
            unsafe { *binding.str_len_or_ind_ptr = SQL_NULL_DATA };
        }
        return Ok(());
    }

    let buffer = state.buffers.remove(&param_num).unwrap_or_default();
    binding.owned_buffer = Some(buffer);
    if let Some(data) = binding.owned_buffer.as_ref() {
        binding.parameter_value_ptr = data.as_ptr() as *const u8 as *mut u8 as sql::Pointer;
        binding.buffer_length = data.len() as sql::Len;
        if !binding.str_len_or_ind_ptr.is_null() {
            unsafe { *binding.str_len_or_ind_ptr = data.len() as sql::Len };
        }
    } else {
        binding.parameter_value_ptr = std::ptr::null_mut();
        binding.buffer_length = 0;
    }
    Ok(())
}

fn complete_data_at_exec_execution(
    stmt: &mut Statement,
    state: &DataAtExecState,
) -> OdbcResult<()> {
    let response = if stmt.parameter_bindings.is_empty() {
        DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
            stmt_handle: Some(stmt.stmt_handle),
            describe_only: false,
        })?
    } else {
        bind_and_execute(stmt, &stmt.parameter_bindings)?
    };

    if matches!(state.mode, DataAtExecMode::ExecutePrepared) {
        set_param_status(stmt, 0, SQL_PARAM_SUCCESS);
        if let Some(ptr) = stmt.params_processed_ptr {
            unsafe { *ptr = 1 }
        }
    }

    let prepared_query_text = stmt.prepared_query.clone();
    let query_text = match &state.mode {
        DataAtExecMode::ExecDirect { query_text } => Some(query_text.as_str()),
        DataAtExecMode::ExecutePrepared => prepared_query_text.as_deref(),
    };
    apply_execution_response(stmt, response, query_text)
}

fn is_sql_nts(value: sql::Len) -> bool {
    value == sql::NTS as sql::Len
}

unsafe fn read_c_string_len(ptr: *const u8) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let mut len = 0usize;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    len
}

pub fn param_data(statement_handle: sql::Handle) -> OdbcResult<ParamDataStatus> {
    let stmt = stmt_from_handle(statement_handle);

    loop {
        let mut state = stmt
            .data_at_exec_state
            .take()
            .ok_or_else(|| StatementNotExecutedSnafu.build())?;

        if state.awaiting_data {
            finalize_current_data_param(stmt, &mut state)?;
            state.awaiting_data = false;
            state.current_param = None;
            stmt.data_at_exec_state = Some(state);
            continue;
        }

        if let Some(next_param) = state.pending_params.pop_front() {
            state.current_param = Some(next_param);
            state.awaiting_data = true;
            state
                .buffers
                .entry(next_param)
                .or_insert_with(Vec::new)
                .clear();
            state.null_params.remove(&next_param);
            let binding =
                stmt.parameter_bindings
                    .get(&next_param)
                    .context(ParameterBindingSnafu {
                        parameters: format!("Parameter {next_param} not found"),
                    })?;
            stmt.data_at_exec_state = Some(state);
            return Ok(ParamDataStatus::NeedData(binding.parameter_value_ptr));
        } else {
            complete_data_at_exec_execution(stmt, &state)?;
            stmt.data_at_exec_state = None;
            return Ok(ParamDataStatus::Success);
        }
    }
}

pub fn put_data(
    statement_handle: sql::Handle,
    data_ptr: sql::Pointer,
    str_len_or_ind_ptr: sql::Len,
) -> OdbcResult<()> {
    let stmt = stmt_from_handle(statement_handle);
    let mut state = stmt
        .data_at_exec_state
        .take()
        .ok_or_else(|| StatementNotExecutedSnafu.build())?;

    if !state.awaiting_data {
        stmt.data_at_exec_state = Some(state);
        return StatementNotExecutedSnafu.fail();
    }

    let current_param = state
        .current_param
        .ok_or_else(|| StatementNotExecutedSnafu.build())?;

    if str_len_or_ind_ptr == SQL_NULL_DATA {
        state.null_params.insert(current_param);
        state.buffers.remove(&current_param);
        stmt.data_at_exec_state = Some(state);
        return Ok(());
    }

    if data_ptr.is_null() {
        let result = ParameterBindingSnafu {
            parameters: "SQLPutData received null data pointer".to_string(),
        }
        .fail();
        stmt.data_at_exec_state = Some(state);
        return result;
    }

    let binding = stmt
        .parameter_bindings
        .get(&current_param)
        .context(ParameterBindingSnafu {
            parameters: format!("Parameter {current_param} not found"),
        })?;

    let len = if is_sql_nts(str_len_or_ind_ptr) {
        match binding.value_type {
            CDataType::Char => unsafe { read_c_string_len(data_ptr as *const u8) },
            _ => {
                return ParameterBindingSnafu {
                    parameters:
                        "SQL_NTS indicator for SQLPutData is only supported for SQL_C_CHAR data"
                            .to_string(),
                }
                .fail();
            }
        }
    } else if str_len_or_ind_ptr >= 0 {
        str_len_or_ind_ptr as usize
    } else {
        return ParameterBindingSnafu {
            parameters: format!(
                "Unsupported StrLen_or_IndPtr value {str_len_or_ind_ptr} for SQLPutData"
            ),
        }
        .fail();
    };

    if len == 0 {
        stmt.data_at_exec_state = Some(state);
        return Ok(());
    }

    let slice = unsafe { std::slice::from_raw_parts(data_ptr as *const u8, len) };
    let buffer = state.buffers.entry(current_param).or_insert_with(Vec::new);
    buffer.extend_from_slice(slice);
    stmt.data_at_exec_state = Some(state);
    Ok(())
}

fn build_row_parameter_bindings(
    bindings: &std::collections::HashMap<u16, ParameterBinding>,
    row_idx: usize,
    bind_type: ParamBindType,
) -> Result<std::collections::HashMap<u16, ParameterBinding>, ArrowBindingError> {
    match bind_type {
        ParamBindType::Column => {
            let mut row_bindings = std::collections::HashMap::new();
            for (&param_num, binding) in bindings {
                let mut row_binding = binding.clone();
                row_binding.parameter_value_ptr = column_value_ptr(binding, row_idx)?;
                if binding.str_len_or_ind_ptr.is_null() {
                    row_binding.str_len_or_ind_ptr = std::ptr::null_mut();
                } else {
                    unsafe {
                        row_binding.str_len_or_ind_ptr = binding.str_len_or_ind_ptr.add(row_idx);
                    }
                }
                row_bindings.insert(param_num, row_binding);
            }
            Ok(row_bindings)
        }
        ParamBindType::Row(_) => Err(ArrowBindingError::UnsupportedBindingMode),
    }
}

fn column_value_ptr(
    binding: &ParameterBinding,
    row_idx: usize,
) -> Result<sql::Pointer, ArrowBindingError> {
    if binding.parameter_value_ptr.is_null() {
        return Err(ArrowBindingError::NullParameterValue);
    }
    let stride = column_value_stride(binding)?;
    let offset = stride
        .checked_mul(row_idx)
        .ok_or(ArrowBindingError::InvalidParameterIndices)?;
    let ptr = unsafe { (binding.parameter_value_ptr as *mut u8).add(offset) };
    Ok(ptr as sql::Pointer)
}

fn column_value_stride(binding: &ParameterBinding) -> Result<usize, ArrowBindingError> {
    #[repr(C)]
    struct SqlTimestampStruct {
        year: i16,
        month: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        fraction: u32,
    }

    #[repr(C)]
    struct SqlDateStruct {
        year: i16,
        month: u16,
        day: u16,
    }

    #[repr(C)]
    struct SqlTimeStruct {
        hour: u16,
        minute: u16,
        second: u16,
    }

    match binding.value_type {
        CDataType::Char | CDataType::Binary | CDataType::WChar => {
            if binding.buffer_length <= 0 {
                Err(ArrowBindingError::InvalidColumnBufferLength)
            } else {
                Ok(binding.buffer_length as usize)
            }
        }
        CDataType::Double => Ok(std::mem::size_of::<f64>()),
        CDataType::Bit => Ok(std::mem::size_of::<sql::Char>()),
        CDataType::TypeTimestamp | CDataType::TimeStamp => {
            Ok(std::mem::size_of::<SqlTimestampStruct>())
        }
        CDataType::TypeDate | CDataType::Date => Ok(std::mem::size_of::<SqlDateStruct>()),
        CDataType::TypeTime | CDataType::Time => Ok(std::mem::size_of::<SqlTimeStruct>()),
        CDataType::Long => Ok(std::mem::size_of::<i32>()),
        CDataType::ULong => Ok(std::mem::size_of::<u32>()),
        _ => Err(ArrowBindingError::UnsupportedCDataType(binding.value_type)),
    }
}

fn set_param_status(stmt: &Statement, row_idx: usize, status: sql::USmallInt) {
    if let Some(ptr) = stmt.param_status_ptr {
        unsafe {
            *ptr.add(row_idx) = status;
        }
    }
}

fn update_session_settings_from_query(stmt: &mut Statement, query: &str) -> Option<String> {
    let mut timezone_override: Option<String> = None;
    if let Some(format) = parse_timestamp_ltz_format(query) {
        tracing::debug!(
            "update_session_settings_from_query: setting TIMESTAMP_LTZ format to {:?}",
            format
        );
        stmt.conn.timestamp_ltz_format = format;
    }
    if let Some(format) = parse_timestamp_ntz_format(query) {
        tracing::debug!(
            "update_session_settings_from_query: setting TIMESTAMP_NTZ format to {:?}",
            format
        );
        stmt.conn.timestamp_ntz_format = format;
    }
    if let Some(format) = parse_timestamp_tz_format(query) {
        tracing::debug!(
            "update_session_settings_from_query: setting TIMESTAMP_TZ format to {:?}",
            format
        );
        stmt.conn.timestamp_tz_format = format;
    }
    if let Some(mapping) = parse_timestamp_type_mapping(query) {
        tracing::debug!(
            "update_session_settings_from_query: setting TIMESTAMP type mapping to {:?}",
            mapping
        );
        stmt.conn.timestamp_type_mapping = mapping;
    }
    if let Some(tz_value) = parse_timezone_setting(query) {
        let normalized = crate::timezone::normalize_timezone_name(&tz_value);
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rust_debug.log")
        {
            let _ = writeln!(
                file,
                "update_session_settings_from_query: TIMEZONE {} -> {}",
                tz_value, normalized
            );
        }
        stmt.conn.session_timezone = Some(normalized.clone());
        stmt.session_timezone = Some(normalized.clone());
        crate::write_arrow::set_session_timezone(stmt.session_timezone.clone());
        set_read_session_timezone(stmt.session_timezone.clone());
        timezone_override = Some(normalized);
    }
    if let Some(enabled) = parse_custom_sql_types_setting(query) {
        tracing::debug!(
            "update_session_settings_from_query: ODBC_USE_CUSTOM_SQL_DATA_TYPES={enabled}"
        );
        stmt.conn.use_custom_sql_types = enabled;
    }

    timezone_override
}

fn refresh_session_timezone(stmt: &mut Statement) {
    let needs_fetch = stmt.conn.session_timezone.is_none();

    if needs_fetch {
        if let ConnectionState::Connected { conn_handle, .. } = &stmt.conn.state {
            let core_handle = database_driver_v1::Handle {
                id: conn_handle.id as u64,
                magic: conn_handle.magic as u64,
            };
            match database_driver_v1::connection_get_timezone(core_handle) {
                Ok(tz_opt) => {
                    let normalized = tz_opt.map(|tz| crate::timezone::normalize_timezone_name(&tz));
                    tracing::debug!(
                        "refresh_session_timezone: fetched timezone from core = {:?}",
                        normalized
                    );
                    stmt.conn.session_timezone = normalized;
                }
                Err(err) => {
                    tracing::warn!(
                        "refresh_session_timezone: failed to fetch timezone from core: {err}"
                    );
                }
            }
        }
    }

    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_debug.log")
    {
        let _ = writeln!(
            file,
            "refresh_session_timezone: stmt_handle={:?} conn_timezone={:?}",
            stmt.stmt_handle.id, stmt.conn.session_timezone
        );
    }
    stmt.session_timezone = stmt.conn.session_timezone.clone();
}

fn parse_timestamp_ltz_format(query: &str) -> Option<TimestampLtzFormat> {
    let format_value = extract_timestamp_format_value(query, "TIMESTAMP_LTZ_OUTPUT_FORMAT")?;
    Some(build_timestamp_format(&format_value, true, false, false))
}

fn parse_timestamp_ntz_format(query: &str) -> Option<TimestampLtzFormat> {
    let format_value = extract_timestamp_format_value(query, "TIMESTAMP_NTZ_OUTPUT_FORMAT")?;
    Some(build_timestamp_format(&format_value, false, false, false))
}

fn parse_timestamp_tz_format(query: &str) -> Option<TimestampLtzFormat> {
    let format_value = extract_timestamp_format_value(query, "TIMESTAMP_TZ_OUTPUT_FORMAT")?;
    Some(build_timestamp_format(&format_value, true, true, true))
}

fn parse_timestamp_type_mapping(query: &str) -> Option<TimestampType> {
    let upper = query.to_ascii_uppercase();
    if !upper.contains("ALTER SESSION") || !upper.contains("CLIENT_TIMESTAMP_TYPE_MAPPING") {
        return None;
    }

    let pos = upper.find("CLIENT_TIMESTAMP_TYPE_MAPPING")?;
    let slice = &query[pos..];
    let eq_idx = slice.find('=')?;
    let after_eq = slice[eq_idx + 1..].trim_start();
    let (value, _) = if let Some(rest) = after_eq.strip_prefix('\'') {
        let end = rest.find('\'')?;
        (&rest[..end], &rest[end + 1..])
    } else if let Some(rest) = after_eq.strip_prefix('"') {
        let end = rest.find('"')?;
        (&rest[..end], &rest[end + 1..])
    } else {
        let end = after_eq
            .find(|c: char| c == ' ' || c == ';' || c == '\n')
            .unwrap_or(after_eq.len());
        (&after_eq[..end], &after_eq[end..])
    };

    match value.trim().to_ascii_uppercase().as_str() {
        "TIMESTAMP_NTZ" => Some(TimestampType::Ntz),
        "TIMESTAMP_TZ" => Some(TimestampType::Tz),
        "TIMESTAMP_LTZ" => Some(TimestampType::Ltz),
        _ => None,
    }
}

fn parse_timezone_setting(query: &str) -> Option<String> {
    let upper = query.to_ascii_uppercase();
    if !upper.contains("ALTER SESSION") || !upper.contains("TIMEZONE") {
        return None;
    }

    let pos = upper.find("TIMEZONE")?;
    let slice = &query[pos..];
    let eq_idx = slice.find('=')?;
    let after_eq = slice[eq_idx + 1..].trim_start();

    let (value, _) = if let Some(rest) = after_eq.strip_prefix('\'') {
        let end = rest.find('\'')?;
        (&rest[..end], &rest[end + 1..])
    } else if let Some(rest) = after_eq.strip_prefix('"') {
        let end = rest.find('"')?;
        (&rest[..end], &rest[end + 1..])
    } else {
        let end = after_eq
            .find(|c: char| c == ' ' || c == ';' || c == '\n')
            .unwrap_or(after_eq.len());
        (&after_eq[..end], &after_eq[end..])
    };

    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_custom_sql_types_setting(query: &str) -> Option<bool> {
    let upper = query.to_ascii_uppercase();
    if !upper.contains("ALTER SESSION") || !upper.contains("ODBC_USE_CUSTOM_SQL_DATA_TYPES") {
        return None;
    }

    let pos = upper.find("ODBC_USE_CUSTOM_SQL_DATA_TYPES")?;
    let slice = &query[pos..];
    let eq_idx = slice.find('=')?;
    let after_eq = slice[eq_idx + 1..].trim_start();

    let value = if let Some(rest) = after_eq.strip_prefix('\'') {
        let end = rest.find('\'')?;
        &rest[..end]
    } else if let Some(rest) = after_eq.strip_prefix('"') {
        let end = rest.find('"')?;
        &rest[..end]
    } else {
        let end = after_eq
            .find(|c: char| c == ' ' || c == ';' || c == '\n')
            .unwrap_or(after_eq.len());
        &after_eq[..end]
    }
    .trim()
    .to_ascii_uppercase();

    match value.as_str() {
        "TRUE" | "ON" | "1" => Some(true),
        "FALSE" | "OFF" | "0" => Some(false),
        _ => None,
    }
}

fn extract_timestamp_format_value(query: &str, keyword: &str) -> Option<String> {
    let upper = query.to_ascii_uppercase();
    if !upper.contains("ALTER SESSION") || !upper.contains(keyword) {
        return None;
    }
    let pos = upper.find(keyword)?;
    let slice = &query[pos..];
    let eq_idx = slice.find('=')?;
    let after_eq = slice[eq_idx + 1..].trim_start();
    if let Some(rest) = after_eq.strip_prefix('\'') {
        let end = rest.find('\'')?;
        return Some(rest[..end].to_string());
    }
    if let Some(rest) = after_eq.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }
    let end = after_eq
        .find(|c: char| c == ' ' || c == ';' || c == '\n')
        .unwrap_or(after_eq.len());
    Some(after_eq[..end].trim().to_string())
}

fn build_timestamp_format(
    format_value: &str,
    allow_timezone_marker: bool,
    default_include_timezone: bool,
    enforce_timezone: bool,
) -> TimestampLtzFormat {
    let format_upper = format_value.to_ascii_uppercase();
    let include_fraction = format_upper.contains(".FF");
    let fractional_digits = if include_fraction {
        extract_fractional_digits(&format_upper)
    } else {
        None
    };
    let include_timezone = if enforce_timezone {
        true
    } else if allow_timezone_marker {
        if format_upper.contains("TZ") {
            true
        } else {
            default_include_timezone
        }
    } else {
        false
    };
    let mut format = TimestampLtzFormat::new(include_fraction, include_timezone);
    if include_fraction {
        format = format
            .with_digits(fractional_digits)
            .with_force_fractional(true);
    }
    format
}

fn extract_fractional_digits(format_upper: &str) -> Option<u8> {
    if let Some(idx) = format_upper.find(".FF") {
        let mut digits = String::new();
        for ch in format_upper[idx + 3..].chars() {
            if ch.is_ascii_digit() {
                digits.push(ch);
            } else {
                break;
            }
        }
        if digits.is_empty() {
            None
        } else {
            let parsed = digits.parse::<u16>().ok()?;
            let clamped = parsed.min(9) as u8;
            Some(clamped)
        }
    } else {
        None
    }
}

fn bindings_require_timestamp_tz(
    bindings: &std::collections::HashMap<u16, ParameterBinding>,
) -> bool {
    bindings
        .values()
        .any(|binding| binding.parameter_type.0 == 2002)
}

fn ensure_client_timestamp_type_mapping(
    stmt: &mut Statement,
    desired: TimestampType,
) -> OdbcResult<()> {
    if stmt.conn.timestamp_type_mapping == desired {
        return Ok(());
    }

    if let ConnectionState::Connected { conn_handle, .. } = &stmt.conn.state {
        apply_timestamp_type_mapping(conn_handle.clone(), desired)?;
        stmt.conn.timestamp_type_mapping = desired;
        crate::write_arrow::set_timestamp_type_mapping(desired);
    } else {
        tracing::warn!(
            "ensure_client_timestamp_type_mapping: connection not established; skipping ALTER SESSION"
        );
    }

    Ok(())
}

fn apply_timestamp_type_mapping(
    conn_handle: sf_core::protobuf_gen::database_driver_v1::ConnectionHandle,
    mapping: TimestampType,
) -> OdbcResult<()> {
    let keyword = timestamp_mapping_keyword(mapping);
    let sql = format!("ALTER SESSION SET CLIENT_TIMESTAMP_TYPE_MAPPING = '{keyword}'");
    execute_session_statement(conn_handle, &sql)
}

fn timestamp_mapping_keyword(mapping: TimestampType) -> &'static str {
    match mapping {
        TimestampType::Ltz => "TIMESTAMP_LTZ",
        TimestampType::Ntz => "TIMESTAMP_NTZ",
        TimestampType::Tz => "TIMESTAMP_TZ",
    }
}

fn execute_session_statement(
    conn_handle: sf_core::protobuf_gen::database_driver_v1::ConnectionHandle,
    sql: &str,
) -> OdbcResult<()> {
    tracing::info!("execute_session_statement: {sql}");
    let stmt_handle = DatabaseDriverClient::statement_new(StatementNewRequest {
        conn_handle: Some(conn_handle),
    })?
    .stmt_handle
    .required("Statement handle is required")?;

    DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
        stmt_handle: Some(stmt_handle.clone()),
        query: sql.to_string(),
    })?;

    DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
        stmt_handle: Some(stmt_handle),
        describe_only: false,
    })?;

    Ok(())
}

/// Bind a parameter to a prepared statement
#[allow(clippy::too_many_arguments)]
pub fn bind_parameter(
    statement_handle: sql::Handle,
    parameter_number: sql::USmallInt,
    input_output_type: sql::ParamType,
    value_type: CDataType,
    parameter_type: sql::SqlDataType,
    _column_size: sql::ULen,
    _decimal_digits: sql::SmallInt,
    parameter_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
) -> OdbcResult<()> {
    tracing::debug!(
        "bind_parameter: parameter_number={}, input_output_type={:?}, value_type={:?}, parameter_type={:?}",
        parameter_number,
        input_output_type,
        value_type,
        parameter_type
    );

    if parameter_number == 0 {
        tracing::error!("bind_parameter: parameter_number cannot be 0");
        return InvalidParameterNumberSnafu.fail();
    }

    match input_output_type {
        sql::ParamType::Input => {}
        other => {
            tracing::error!(
                "bind_parameter: unsupported parameter direction {:?}",
                other
            );
            return UnsupportedParameterDirectionSnafu { direction: other }.fail();
        }
    }

    let stmt = stmt_from_handle(statement_handle);

    let binding = ParameterBinding {
        parameter_type,
        value_type,
        parameter_value_ptr,
        buffer_length,
        str_len_or_ind_ptr,
        owned_buffer: None,
    };

    // Store the binding
    stmt.parameter_bindings.insert(parameter_number, binding);

    tracing::info!(
        "bind_parameter: Successfully bound parameter {}",
        parameter_number
    );
    Ok(())
}

/// Get the number of parameters in a prepared statement
pub fn num_params(
    statement_handle: sql::Handle,
    param_count_ptr: *mut sql::SmallInt,
) -> OdbcResult<()> {
    let stmt = stmt_from_handle(statement_handle);

    // Count the number of ? placeholders in the prepared query
    let count = if let Some(ref query) = stmt.prepared_query {
        query.matches('?').count() as sql::SmallInt
    } else {
        0
    };

    if !param_count_ptr.is_null() {
        unsafe { *param_count_ptr = count };
    }

    tracing::debug!("num_params: count={}", count);
    Ok(())
}

/// Describe a parameter in a prepared statement
pub fn describe_param(
    statement_handle: sql::Handle,
    parameter_number: sql::USmallInt,
    data_type_ptr: *mut sql::SmallInt,
    parameter_size_ptr: *mut sql::ULen,
    decimal_digits_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
) -> OdbcResult<()> {
    let stmt = stmt_from_handle(statement_handle);
    if !stmt.is_prepared {
        return StatementNotExecutedSnafu.fail();
    }

    // For now, return generic VARCHAR type for all parameters
    // Snowflake doesn't provide parameter metadata before execution
    if !data_type_ptr.is_null() {
        unsafe { *data_type_ptr = sql::SqlDataType::VARCHAR.0 };
    }

    if !parameter_size_ptr.is_null() {
        // Default to max varchar size (Snowflake uses 134217728 = 128MB)
        unsafe { *parameter_size_ptr = 134217728 };
    }

    if !decimal_digits_ptr.is_null() {
        unsafe { *decimal_digits_ptr = 0 };
    }

    if !nullable_ptr.is_null() {
        // SQL_NULLABLE = 1
        unsafe { *nullable_ptr = 1 };
    }

    tracing::debug!("describe_param: parameter_number={}", parameter_number);
    Ok(())
}
