use super::connection::RefreshContext;
use super::error::*;
use super::global_state::DatabaseDriverV1;
use crate::handle_manager::Handle;
use crate::rest::snowflake::{QueryExecutionMode, QueryInput, snowflake_query_with_client};

impl DatabaseDriverV1 {
    pub async fn connection_commit(&self, conn_handle: Handle) -> Result<(), ApiError> {
        self.execute_transaction_internal(conn_handle, "commit")
            .await
    }

    pub async fn connection_rollback(&self, conn_handle: Handle) -> Result<(), ApiError> {
        self.execute_transaction_internal(conn_handle, "rollback")
            .await
    }

    async fn execute_transaction_internal(
        &self,
        conn_handle: Handle,
        sql: &str,
    ) -> Result<(), ApiError> {
        let conn_ptr = self.connections.get_obj(conn_handle).ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .build()
        })?;

        let (query_parameters, http_client, retry_policy) = {
            let conn = conn_ptr.lock().await;
            conn.query_transport()?
        };

        let query_input = QueryInput {
            sql: sql.to_string(),
            bindings: None,
            describe_only: None,
        };

        let response = {
            let mut ctx = RefreshContext::from_arc(&conn_ptr).await?;
            let mut last_error = None;
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

        if response.success {
            let conn = conn_ptr.lock().await;
            conn.update_session_params_cache(
                sql,
                response.data.parameters.as_ref(),
                &super::connection::FinalSessionNames {
                    database: response.data.final_database_name.clone(),
                    schema: response.data.final_schema_name.clone(),
                    warehouse: response.data.final_warehouse_name.clone(),
                    role: response.data.final_role_name.clone(),
                },
            )
            .await;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bogus_handle() -> Handle {
        Handle { id: 999, magic: 0 }
    }

    #[tokio::test]
    async fn commit_with_invalid_handle_returns_error() {
        let driver = DatabaseDriverV1::new();
        let err = driver.connection_commit(bogus_handle()).await;
        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("Connection handle not found"),
        );
    }

    #[tokio::test]
    async fn rollback_with_invalid_handle_returns_error() {
        let driver = DatabaseDriverV1::new();
        let err = driver.connection_rollback(bogus_handle()).await;
        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("Connection handle not found"),
        );
    }

    #[tokio::test]
    async fn commit_without_initialized_connection_returns_error() {
        let driver = DatabaseDriverV1::new();
        let conn_handle = driver.connection_new();
        let err = driver.connection_commit(conn_handle).await;
        assert!(
            err.is_err(),
            "commit on uninitialized connection should fail"
        );
    }

    #[tokio::test]
    async fn rollback_without_initialized_connection_returns_error() {
        let driver = DatabaseDriverV1::new();
        let conn_handle = driver.connection_new();
        let err = driver.connection_rollback(conn_handle).await;
        assert!(
            err.is_err(),
            "rollback on uninitialized connection should fail"
        );
    }
}
