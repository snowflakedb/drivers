//! `connection_upload_stream` — execute a PUT SQL statement using caller-
//! supplied in-memory bytes instead of reading from the filesystem path.
//!
//! This is the Rust backend for the Python `file_stream` kwarg on
//! `cursor.execute()` (PR-4) and JDBC's `uploadStream` (PR-3).
//!
//! # Protocol
//!
//! 1. Execute the PUT SQL synchronously against GS (same as a normal PUT) to
//!    obtain stage credentials, encryption material, and transfer parameters.
//! 2. Substitute `ByteSource::Bytes(data)` for the filesystem source that
//!    `upload_files` would normally open.
//! 3. Build and register a result set from the upload results, exactly as
//!    `statement_execute_query` does for a normal PUT.
//!
//! The virtual filename for the stage object is taken from the `src_locations[0]`
//! entry returned by GS (the basename of the `file://` path in the SQL),
//! matching the reference Python connector's behavior when `file_stream` is used.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use snafu::{OptionExt, ResultExt};
use tokio::sync::Mutex;
use tracing::Instrument;

use super::connection::{Connection, FinalSessionNames, RefreshContext};
use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::query::{StageCredsRefreshContext, perform_upload_with_bytes_source};
use super::result_set::{ResultSet, ResultSetInfo, resolve_reader_ctx, response_to_descriptor};
use super::statement::skip_leading_whitespace_and_comments_pub;
use crate::config::rest_parameters::QueryParameters;
use crate::config::retry::RetryPolicy;
use crate::handle_manager::Handle;
use crate::rest::snowflake::{
    QueryExecutionMode, QueryInput, RestError, snowflake_query_with_client,
};

impl DatabaseDriverV1 {
    /// Execute a PUT SQL and upload `data` bytes to the stage instead of
    /// reading from the filesystem path referenced in the SQL.
    ///
    /// Returns a `ResultSetInfo` whose handle/descriptor have the same shape
    /// as a normal `statement_execute_query` on a PUT statement.
    pub async fn connection_upload_stream(
        &self,
        conn_handle: Handle,
        sql: String,
        data: Vec<u8>,
    ) -> Result<ResultSetInfo, ApiError> {
        let session_id = self.session_id_for_conn(conn_handle).await;
        async {
            let conn_ptr = self.connections.get_obj(conn_handle).ok_or_else(|| {
                InvalidArgumentSnafu {
                    argument: "Connection handle not found".to_string(),
                }
                .build()
            })?;

            // Validate that the SQL is a PUT statement.
            if !is_put_sql(&sql) {
                return InvalidArgumentSnafu {
                    argument: concat!(
                        "file_stream requires a PUT SQL statement ",
                        "(SQL does not begin with PUT)"
                    )
                    .to_string(),
                }
                .fail();
            }

            // Obtain transport context (same as query_context in statement.rs).
            let (query_parameters, http_client, retry_policy) =
                connection_query_context(&conn_ptr).await?;

            let query_input = QueryInput {
                sql: sql.clone(),
                bindings: None,
                describe_only: None,
                query_parameters: None,
            };

            // Execute the PUT SQL against GS — blocking, like all file transfers.
            let response = {
                let mut ctx = RefreshContext::from_arc(&conn_ptr).await?;
                let mut last_error: Option<RestError> = None;
                loop {
                    let session_token = ctx.refresh_token(last_error).await?;
                    match snowflake_query_with_client(
                        &http_client,
                        query_parameters.clone(),
                        session_token.reveal(),
                        query_input.clone(),
                        &retry_policy,
                        QueryExecutionMode::Blocking,
                    )
                    .await
                    {
                        Ok(result) => break Ok(result),
                        Err(e) => last_error = Some(e),
                    }
                }
            }?;

            // Update session parameter cache (mirrors the normal PUT path).
            if response.success {
                let conn = conn_ptr.lock().await;
                conn.update_session_params_cache(
                    &sql,
                    response.data.parameters.as_ref(),
                    &FinalSessionNames {
                        database: response.data.final_database_name.clone(),
                        schema: response.data.final_schema_name.clone(),
                        warehouse: response.data.final_warehouse_name.clone(),
                        role: response.data.final_role_name.clone(),
                    },
                )
                .await;
            }

            let gs_data = response.data;
            let stage_creds_refresh_context = StageCredsRefreshContext {
                sql: sql.clone(),
                query_parameters: query_parameters.clone(),
                conn: conn_ptr.clone(),
            };
            let use_s3_regional_url = conn_ptr
                .lock()
                .await
                .use_s3_regional_url_session_param()
                .await;

            // Perform the upload substituting ByteSource::Bytes for the path.
            let rowset_data = perform_upload_with_bytes_source(
                &gs_data,
                &self.wrapper_presets,
                Some(stage_creds_refresh_context),
                use_s3_regional_url,
                data,
            )
            .await
            .context(QueryResponseProcessingSnafu)?;

            let descriptor = response_to_descriptor(&gs_data, &self.wrapper_presets);
            let reader_ctx = resolve_reader_ctx(&conn_ptr).await?;
            let handle = {
                let result_set = ResultSet {
                    descriptor: descriptor.clone(),
                    data: rowset_data,
                    reader_ctx,
                };
                self.results.add_handle(Mutex::new(result_set))
            };
            Ok(ResultSetInfo { handle, descriptor })
        }
        .instrument(crate::snowflake_op_span!(
            "connection_upload_stream",
            session_id
        ))
        .await
    }
}

/// Extract connection transport parameters (mirrors `query_context` in statement.rs).
async fn connection_query_context(
    conn: &Arc<Mutex<Connection>>,
) -> Result<(QueryParameters, reqwest::Client, RetryPolicy), ApiError> {
    let conn = conn.lock().await;
    if conn.is_closed.load(Ordering::SeqCst) {
        return Err(ConnectionClosedSnafu {}.build());
    }
    Ok((
        conn.query_transport_parameters()?,
        conn.http_client
            .clone()
            .context(ConnectionNotInitializedSnafu)?,
        conn.retry_policy.clone(),
    ))
}

/// Returns `true` when `sql` (after stripping leading whitespace/comments)
/// starts with `PUT` followed by whitespace or a comment marker.
fn is_put_sql(sql: &str) -> bool {
    let s = skip_leading_whitespace_and_comments_pub(sql);
    if s.len() < 4 {
        return false;
    }
    let prefix = &s[..3];
    let next_char = s.as_bytes()[3];
    prefix.eq_ignore_ascii_case("PUT")
        && (next_char.is_ascii_whitespace() || next_char == b'/' || next_char == b'-')
}
