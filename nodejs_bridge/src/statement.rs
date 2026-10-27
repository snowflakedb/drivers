mod column;
mod result;

pub use column::Column;
use column::column_names;

use crate::BRIDGE;
use crate::error::{BridgeError, ToJsError, async_to_js};
use crate::session::Session;
use crate::session_params::KnownSessionParameters;
use crate::session_state::SessionState;
use crate::validation_utils::{require_valid_query_id, require_valid_request_id};
use napi::bindgen_prelude::{spawn, *};
use napi::threadsafe_function::ThreadsafeFunction;
use napi::tokio::sync::MutexGuard;
use napi_derive::napi;
use result::{QueryResult, ResultValue, StatementResult};
use sf_core::apis::database_driver_v1::{BindingType, DataPtr, ErrorKind};
use sf_core::apis::operation_ctx::OperationCtx;
use sf_core::config::settings::Setting;
use sf_core::query_types::QueryType;
use std::collections::HashMap;
use std::sync::Arc;

const NUM_UPDATED_ROWS_NOT_APPLICABLE: i64 = -1;

#[napi(string_enum = "snake_case")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatementStatus {
    Fetching,
    Complete,
}

#[napi]
pub enum QueryBindingFormat {
    Json,
    Csv,
}

/// Bind parameters handed down from the wrapper. `data` is either a JSON object
/// of `{ "1": { type, value }, ... }` or CSV text, selected by `format`.
#[napi(object)]
pub struct QueryBindings {
    pub format: QueryBindingFormat,
    pub data: String,
}

impl QueryBindings {
    fn as_binding_type(&self) -> BindingType<'_> {
        let ptr = DataPtr::new(self.data.as_ptr(), self.data.len() as i64);
        match self.format {
            QueryBindingFormat::Csv => BindingType::Csv(ptr),
            QueryBindingFormat::Json => BindingType::Json(ptr),
        }
    }
}

pub(crate) type BuildFallbackInlineBindings =
    ThreadsafeFunction<(), String, (), Status, false, true>;

#[napi(object)]
pub struct ExecuteParams {
    pub query: String,
    pub bindings: Option<QueryBindings>,
    pub parameters: Option<HashMap<String, String>>,
    pub async_exec: Option<bool>,
    pub request_id: Option<String>,
    pub describe_only: Option<bool>,
}

#[napi]
pub struct Statement {
    result: StatementResult,
    operation_ctx: Arc<OperationCtx>,
    request_id: Option<uuid::Uuid>,
    session: Session,
}

#[napi]
impl Statement {
    pub(crate) fn execute(
        session: Session,
        mut params: ExecuteParams,
        build_fallback_inline_bindings: Option<BuildFallbackInlineBindings>,
    ) -> std::result::Result<Self, BridgeError> {
        let request_id = resolve_request_id(params.request_id.take(), uuid::Uuid::new_v4)?;
        let operation_ctx = Arc::new(OperationCtx::with_own_token());
        let task_ctx = Arc::clone(&operation_ctx);
        let task_session = session.clone();
        Ok(Self {
            result: StatementResult::from_future(async move {
                run_execute_with_inline_fallback(
                    task_session,
                    params,
                    build_fallback_inline_bindings,
                    request_id,
                    &task_ctx,
                )
                .await
            }),
            operation_ctx,
            request_id: Some(request_id),
            session,
        })
    }

    pub(crate) fn from_query_id(session: Session, query_id: String) -> Self {
        let operation_ctx = Arc::new(OperationCtx::with_own_token());
        let task_ctx = Arc::clone(&operation_ctx);
        let task_session = session.clone();
        Self {
            result: StatementResult::from_future(async move {
                require_valid_query_id(&query_id)?;
                let ready = task_session.ready().await?;
                let result = BRIDGE
                    .driver
                    .connection_get_query_result(Some(&task_ctx), ready.connection(), query_id)
                    .await?;
                ResultValue::from_execute_result(result, ready.connection()).await
            }),
            operation_ctx,
            request_id: None,
            session,
        }
    }

    #[napi]
    pub fn wait_for_completion(&self, env: &Env) -> Result<AsyncBlock<()>> {
        let result = self.result.clone();
        async_to_js(env, async move { result.ready().await.map(|_| ()) })
    }

    /// Loads the next batch of rows. Returns `false` when the result set is
    /// exhausted. Drain the loaded batch with
    /// [`get_next_row`](Self::get_next_row) before calling this again.
    #[napi]
    pub fn fetch_next_batch(&self, env: &Env) -> Result<AsyncBlock<bool>> {
        let result = self.result.clone();
        async_to_js(env, async move {
            let value = result.ready().await?;
            let ResultValue::QueryResult(query_result) = value else {
                return Ok(false);
            };
            let query_result = query_result.lock().await;
            let stream_state = Arc::clone(&query_result.stream_state);
            let session_params = Arc::clone(&query_result.session_params);
            drop(query_result);
            stream_state.fetch_next_batch(&session_params).await
        })
    }

    /// Returns the next row of the current batch, or `null` once that batch
    /// is drained. Call [`fetch_next_batch`](Self::fetch_next_batch) to load
    /// another.
    #[napi]
    pub fn get_next_row<'env>(&self, env: &'env Env) -> Result<Option<Array<'env>>> {
        match self.result.get() {
            None => Ok(None),
            Some(Err(error)) => Err(error.to_js_error(*env)),
            Some(Ok(value)) => match value.lock_query_result() {
                Some(query_result) => query_result.stream_state.next_row(env),
                None => Ok(None),
            },
        }
    }

    #[napi]
    pub fn get_request_id(&self) -> Option<String> {
        self.request_id.map(|id| id.to_string())
    }

    #[napi]
    pub fn get_status(&self) -> StatementStatus {
        if self.result.get().is_some() {
            StatementStatus::Complete
        } else {
            StatementStatus::Fetching
        }
    }

    // TODO:
    // - reusable error handling
    // - maybe an util to get field value so we don't repeat the match
    #[napi]
    pub fn get_query_id(&self, _env: &Env) -> Result<Option<String>> {
        match self.result.get() {
            None => Ok(None),
            Some(Ok(value)) => Ok(Some(value.query_id())),
            Some(Err(BridgeError::Core(api_error))) => Ok(api_error.query_id()),
            Some(Err(_)) => Ok(None),
        }
    }

    /// `None` while pending, on failure, or for async-submitted queries.
    fn successful_query_result(&self) -> Option<MutexGuard<'_, Box<QueryResult>>> {
        match self.result.get() {
            Some(Ok(value)) => value.lock_query_result(),
            None | Some(Err(_)) => None,
        }
    }

    #[napi]
    pub fn get_num_rows(&self) -> Option<i64> {
        self.successful_query_result()
            .and_then(|query_result| query_result.result_set_descriptor.row_count)
    }

    #[napi]
    pub fn get_num_updated_rows(&self) -> Option<i64> {
        let query_result = self.successful_query_result()?;
        let query_type = QueryType::from_raw(query_result.result_set_descriptor.statement_type_id);
        Some(
            if matches!(
                query_type,
                QueryType::INSERT
                    | QueryType::UPDATE
                    | QueryType::DELETE
                    | QueryType::MERGE
                    | QueryType::MULTI_TABLE_INSERT
                    | QueryType::COPY
            ) {
                query_result
                    .result_set_descriptor
                    .rows_affected
                    .unwrap_or(0)
            } else {
                NUM_UPDATED_ROWS_NOT_APPLICABLE
            },
        )
    }

    #[napi]
    pub fn get_session_state(&self) -> Option<SessionState> {
        self.successful_query_result()
            .map(|query_result| SessionState::from_descriptor(&query_result.result_set_descriptor))
    }

    /// Not part of the driver's public API. Callers are suposed to invoke this only after the
    /// statement has finished, so a result that is not yet ready is a programming error
    #[napi]
    pub fn get_session_parameters_snapshot(&self, env: &Env) -> Result<KnownSessionParameters> {
        match self.result.get() {
            None => Err(BridgeError::Message(
                "session parameters snapshot requested before the statement finished".to_string(),
            )
            .to_js_error(*env)),
            Some(Ok(value)) => match value.lock_query_result() {
                Some(query_result) => Ok((*query_result.session_params).clone()),
                None => Err(BridgeError::Message(
                    "session parameters snapshot requested on a statement with no result set"
                        .to_string(),
                )
                .to_js_error(*env)),
            },
            Some(Err(error)) => Err(error.to_js_error(*env)),
        }
    }

    #[napi]
    pub fn get_columns(&self, rename_duplicates: bool) -> Option<Vec<Column>> {
        self.successful_query_result().map(|query_result| {
            let columns = &query_result.result_set_descriptor.columns;
            columns
                .iter()
                .zip(column_names(columns, rename_duplicates))
                .enumerate()
                .map(|(i, (meta, name))| Column::from_metadata(i as u32, meta, name))
                .collect()
        })
    }

    #[napi]
    pub fn get_column(
        &self,
        identifier: Either<String, u32>,
        rename_duplicates: bool,
    ) -> Option<Column> {
        let query_result = self.successful_query_result()?;
        let columns = &query_result.result_set_descriptor.columns;
        let mut names = column_names(columns, rename_duplicates);
        let index = match identifier {
            Either::A(name) => names.iter().position(|candidate| *candidate == name)?,
            Either::B(index) => index as usize,
        };
        let meta = columns.get(index)?;
        Some(Column::from_metadata(
            index as u32,
            meta,
            names.swap_remove(index),
        ))
    }

    // TODO: instead of Node calling close, maybe we should call it when the result set is
    // processed from the bridge
    #[napi]
    pub fn close(&self) -> Result<()> {
        if let Some(mut query_result) = self.successful_query_result() {
            query_result.close();
        }
        Ok(())
    }

    #[napi]
    pub fn has_next(&self) -> bool {
        self.successful_query_result()
            .is_some_and(|query_result| query_result.has_next())
    }

    #[napi]
    pub fn is_multi_statement(&self) -> bool {
        self.successful_query_result()
            .is_some_and(|query_result| query_result.is_multi_statement())
    }

    #[napi]
    pub fn next_result(&self, env: &Env) -> Result<AsyncBlock<bool>> {
        let result = self.result.clone();
        async_to_js(env, async move {
            let value = result.ready().await?;
            let ResultValue::QueryResult(query_result) = value else {
                return Ok(false);
            };
            let mut query_result = query_result.lock().await;
            query_result.next_result().await
        })
    }

    #[napi]
    pub fn cancel(&self) {
        let Some(Ok(value)) = self.result.get() else {
            tracing::debug!("cancel before query id is known, cancelling the running execute");
            self.operation_ctx.cancel();
            return;
        };
        let query_id = value.query_id();
        let session = self.session.clone();
        spawn(async move {
            let Ok(ready) = session.ready().await else {
                return;
            };
            if let Err(error) = BRIDGE
                .driver
                .connection_abort_query(ready.connection(), query_id.clone())
                .await
            {
                tracing::warn!(%query_id, ?error, "abort request failed, ignoring");
            }
        });
    }
}

fn resolve_request_id(
    request_id: Option<String>,
    uuid_supplier: impl FnOnce() -> uuid::Uuid,
) -> std::result::Result<uuid::Uuid, BridgeError> {
    match request_id.filter(|id| !id.is_empty()) {
        None => Ok(uuid_supplier()),
        // Uuid::parse_str also accepts 32-char, braced, and urn forms.
        // require_valid_request_id keeps requestId on the same hyphenated check as queryId.
        Some(id) => require_valid_request_id(&id).and_then(|()| {
            uuid::Uuid::parse_str(&id).map_err(|_| BridgeError::InvalidRequestId(id))
        }),
    }
}

async fn run_execute_with_inline_fallback(
    session: Session,
    params: ExecuteParams,
    build_fallback_inline_bindings: Option<BuildFallbackInlineBindings>,
    request_id: uuid::Uuid,
    operation_ctx: &OperationCtx,
) -> std::result::Result<ResultValue, BridgeError> {
    let result = run_execute(
        &session,
        &params,
        params.bindings.as_ref(),
        request_id,
        operation_ctx,
    )
    .await;
    let Some(build_fallback_inline_bindings) = build_fallback_inline_bindings else {
        return result;
    };
    match &result {
        Err(BridgeError::Core(error)) if matches!(error.kind(), ErrorKind::StageBinding) => {
            tracing::warn!(%error, "stage binding failed, falling back to inline binds");
        }
        _ => return result,
    }
    let data = build_fallback_inline_bindings
        .call_async_catch(())
        .await
        .map_err(|thrown| BridgeError::Message(thrown.reason))?;
    let json = QueryBindings {
        format: QueryBindingFormat::Json,
        data,
    };
    run_execute(&session, &params, Some(&json), request_id, operation_ctx).await
}

async fn run_execute(
    session: &Session,
    params: &ExecuteParams,
    bindings: Option<&QueryBindings>,
    request_id: uuid::Uuid,
    operation_ctx: &OperationCtx,
) -> std::result::Result<ResultValue, BridgeError> {
    let driver = &BRIDGE.driver;
    let conn_handle = session.ready().await?.connection();
    let stmt = driver.statement_new(conn_handle)?;

    let result = async {
        driver
            .statement_set_sql_query(stmt, params.query.clone())
            .await?;
        if let Some(parameters) = &params.parameters {
            let options = parameters
                .iter()
                .map(|(k, v)| (k.clone(), Setting::String(v.clone())))
                .collect();
            driver.statement_set_options(stmt, options).await?;
        }
        let binds = bindings.map(QueryBindings::as_binding_type);
        if params.async_exec.unwrap_or(false) {
            let submitted = driver
                .statement_execute_async(
                    Some(operation_ctx),
                    stmt,
                    binds,
                    Some(request_id),
                    params.describe_only,
                )
                .await?;
            return Ok(ResultValue::AsyncSubmitted {
                query_id: submitted.query_id,
            });
        }
        let result = driver
            .statement_execute_query(
                Some(operation_ctx),
                stmt,
                binds,
                None,
                Some(request_id),
                params.describe_only,
            )
            .await?;
        ResultValue::from_execute_result(result, conn_handle).await
    }
    .await;

    let _ = driver.statement_release(stmt);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_request_id_creates_uuid_when_omitted() {
        let supplied = uuid::Uuid::from_u128(0x1234_5678_1234_4123_a123_1234_5678_9012);
        let Ok(parsed) = resolve_request_id(None, || supplied) else {
            panic!("minted request id parses");
        };
        assert_eq!(parsed, supplied);
    }

    #[test]
    fn resolve_request_id_creates_uuid_when_empty() {
        let supplied = uuid::Uuid::from_u128(0x1234_5678_1234_4123_a123_1234_5678_9012);
        let Ok(parsed) = resolve_request_id(Some(String::new()), || supplied) else {
            panic!("empty request id mints");
        };
        assert_eq!(parsed, supplied);
    }

    #[test]
    fn resolve_request_id_keeps_a_valid_caller_id() {
        let request_id = "12345678-1234-4123-A123-123456789012";
        let Ok(parsed) = resolve_request_id(Some(request_id.to_string()), uuid::Uuid::new_v4)
        else {
            panic!("valid request id parses");
        };
        assert_eq!(
            parsed.to_string().to_ascii_lowercase(),
            request_id.to_ascii_lowercase()
        );
    }

    #[test]
    fn resolve_request_id_rejects_malformed_values() {
        let request_id = "foobar";
        assert!(matches!(
            resolve_request_id(Some(request_id.to_string()), uuid::Uuid::new_v4),
            Err(BridgeError::InvalidRequestId(id)) if id == request_id
        ));
    }
}
